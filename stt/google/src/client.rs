use golem_stt::golem::stt::types::{SttError, TranscriptAlternative};
use log::{error, trace, warn};
use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug)]
pub struct GoogleSpeechClient {
    api_key: String,
    client: Client,
    base_url: String,
    timeout: Duration,
    max_retries: u32,
}

impl GoogleSpeechClient {
    pub fn new(api_key: String) -> Self {
        let timeout_str = std::env::var("STT_PROVIDER_TIMEOUT").unwrap_or_else(|_| "30".to_string());
        let timeout = Duration::from_secs(timeout_str.parse().unwrap_or(30));
        
        let max_retries_str = std::env::var("STT_PROVIDER_MAX_RETRIES").unwrap_or_else(|_| "3".to_string());
        let max_retries = max_retries_str.parse().unwrap_or(3);
        
        let base_url = std::env::var("STT_PROVIDER_ENDPOINT")
            .unwrap_or_else(|_| "https://speech.googleapis.com/v1".to_string());
        
        // Initialize logging level if specified
        if let Ok(log_level) = std::env::var("STT_PROVIDER_LOG_LEVEL") {
            match log_level.to_lowercase().as_str() {
                "trace" | "debug" | "info" | "warn" | "error" => {
                    trace!("STT provider log level set to: {}", log_level);
                }
                _ => {
                    trace!("Invalid STT_PROVIDER_LOG_LEVEL '{}', using default", log_level);
                }
            }
        }

        Self {
            api_key,
            client: Client::new(),
            base_url,
            timeout,
            max_retries,
        }
    }

    pub fn transcribe(&self, request: RecognizeRequest) -> Result<RecognizeResponse, SttError> {
        let url = format!("{}/speech:recognize?key={}", self.base_url, self.api_key);
        
        let mut attempts = 0;
        loop {
            attempts += 1;
            if attempts == 1 {
                trace!("Google Speech API request (initial attempt, max retries: {})", self.max_retries);
            } else {
                trace!("Google Speech API request (retry {}/{}, max retries: {})", attempts - 1, self.max_retries, self.max_retries);
            }
            
            match self.make_request(Method::POST, &url, Some(&request)) {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<RecognizeResponse>() {
                            Ok(result) => return Ok(result),
                            Err(e) => {
                                error!("Failed to parse Google Speech response: {}", e);
                                return Err(SttError::InternalError(format!("Failed to parse response: {}", e)));
                            }
                        }
                    } else {
                        let error = self.handle_error_response(response);
                        if self.should_retry(&error) && attempts <= self.max_retries {
                            trace!("Will retry Google Speech request (retry {}/{})", attempts, self.max_retries);
                            std::thread::sleep(std::time::Duration::from_secs(1));
                            continue;
                        } else {
                            return Err(error);
                        }
                    }
                }
                Err(e) => {
                    if attempts <= self.max_retries {
                        trace!("Will retry Google Speech request due to network error (retry {}/{})", attempts, self.max_retries);
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        continue;
                    } else {
                        return Err(SttError::NetworkError(format!("Request failed after {} attempts: {}", self.max_retries + 1, e)));
                    }
                }
            }
        }
    }

    fn make_request<T: Serialize>(
        &self,
        method: Method,
        url: &str,
        body: Option<&T>,
    ) -> Result<Response, reqwest::Error> {
        let mut req = self
            .client
            .request(method, url)
            .header("Content-Type", "application/json")
            .timeout(self.timeout);

        if let Some(body) = body {
            req = req.json(body);
        }

        req.send()
    }

    fn should_retry(&self, error: &SttError) -> bool {
        match error {
            // Retry on rate limits and server errors
            SttError::RateLimited(_) | SttError::ServiceUnavailable(_) => true,
            // Don't retry on client errors (auth, invalid input, etc.)
            _ => false,
        }
    }

    fn handle_error_response(&self, response: Response) -> SttError {
        let status = response.status();
        let error_text = response.text().unwrap_or_else(|_| "Unknown error".to_string());
        
        trace!("Google Speech API error response: {} - {}", status, error_text);

        match status.as_u16() {
            400 => SttError::InvalidAudio(error_text),
            401 => SttError::Unauthorized(error_text),
            403 => SttError::AccessDenied(error_text),
            429 => SttError::RateLimited(60), // Default retry after 60 seconds
            500..=599 => SttError::ServiceUnavailable(error_text),
            _ => SttError::InternalError(format!("HTTP {}: {}", status, error_text)),
        }
    }

    pub fn start_streaming_session(&self, config: RecognitionConfig) -> Result<GoogleStreamingSession, SttError> {
        trace!("Starting Google Speech streaming session");
        Ok(GoogleStreamingSession::new(self.clone(), config))
    }
}

impl Clone for GoogleSpeechClient {
    fn clone(&self) -> Self {
        Self {
            api_key: self.api_key.clone(),
            client: Client::new(),
            base_url: self.base_url.clone(),
            timeout: self.timeout,
            max_retries: self.max_retries,
        }
    }
}

#[derive(Debug)]
pub struct GoogleStreamingSession {
    client: GoogleSpeechClient,
    config: RecognitionConfig,
    audio_buffer: Arc<Mutex<Vec<u8>>>,
    is_active: Arc<Mutex<bool>>,
}

impl GoogleStreamingSession {
    pub fn new(client: GoogleSpeechClient, config: RecognitionConfig) -> Self {
        Self {
            client,
            config,
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            is_active: Arc::new(Mutex::new(true)),
        }
    }

    pub fn send_audio(&self, chunk: Vec<u8>) -> Result<(), SttError> {
        let is_active = self.is_active.lock().map_err(|_| 
            SttError::InternalError("Failed to acquire lock".to_string()))?;
        
        if !*is_active {
            return Err(SttError::InternalError("Streaming session is not active".to_string()));
        }

        let mut buffer = self.audio_buffer.lock().map_err(|_| 
            SttError::InternalError("Failed to acquire buffer lock".to_string()))?;
        
        buffer.extend_from_slice(&chunk);
        trace!("Added {} bytes to streaming buffer, total: {}", chunk.len(), buffer.len());
        
        Ok(())
    }

    pub fn finish_and_get_result(&self) -> Result<RecognizeResponse, SttError> {
        let mut is_active = self.is_active.lock().map_err(|_| 
            SttError::InternalError("Failed to acquire lock".to_string()))?;
        
        if !*is_active {
            return Err(SttError::InternalError("Streaming session already finished".to_string()));
        }

        *is_active = false;

        let buffer = self.audio_buffer.lock().map_err(|_| 
            SttError::InternalError("Failed to acquire buffer lock".to_string()))?;
        
        if buffer.is_empty() {
            return Err(SttError::InvalidAudio("No audio data provided".to_string()));
        }

        trace!("Finishing Google streaming session with {} bytes of audio", buffer.len());

        // Convert accumulated audio to base64 for Google API
        let audio_base64 = base64::encode(&*buffer);
        
        let request = RecognizeRequest {
            config: self.config.clone(),
            audio: RecognitionAudio {
                content: Some(audio_base64),
                uri: None,
            },
            name: None,
        };

        self.client.transcribe(request)
    }

    pub fn close(&self) {
        if let Ok(mut is_active) = self.is_active.lock() {
            *is_active = false;
        }
        trace!("Google streaming session closed");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognizeRequest {
    pub config: RecognitionConfig,
    pub audio: RecognitionAudio,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognitionConfig {
    pub encoding: AudioEncoding,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate_hertz: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_channel_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternative_language_codes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_alternatives: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profanity_filter: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speech_contexts: Option<Vec<SpeechContext>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_word_time_offsets: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_word_confidence: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_automatic_punctuation: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phrases: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boost: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognitionAudio {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>, // Base64 encoded audio data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioEncoding {
    #[serde(rename = "ENCODING_UNSPECIFIED")]
    EncodingUnspecified,
    #[serde(rename = "LINEAR16")]
    Linear16,
    #[serde(rename = "FLAC")]
    Flac,
    #[serde(rename = "MULAW")]
    Mulaw,
    #[serde(rename = "AMR")]
    Amr,
    #[serde(rename = "AMR_WB")]
    AmrWb,
    #[serde(rename = "OGG_OPUS")]
    OggOpus,
    #[serde(rename = "SPEEX_WITH_HEADER_BYTE")]
    SpeexWithHeaderByte,
    #[serde(rename = "MP3")]
    Mp3,
    #[serde(rename = "WEBM_OPUS")]
    WebmOpus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognizeResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<SpeechRecognitionResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_billed_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speech_adaptation_info: Option<SpeechAdaptationInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechRecognitionResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternatives: Option<Vec<SpeechRecognitionAlternative>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_tag: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechRecognitionAlternative {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub words: Option<Vec<WordInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker_tag: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechAdaptationInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adaptation_timeout: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_message: Option<String>,
}
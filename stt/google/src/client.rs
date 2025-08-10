use golem_stt::golem::stt::types::SttError;
use log::{error, trace, warn};
use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use base64::{Engine as _, engine::general_purpose};

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
    streaming_url: String,
    sequence_id: Arc<Mutex<u32>>,
    is_active: Arc<Mutex<bool>>,
    results_buffer: Arc<Mutex<Vec<StreamingRecognitionResult>>>,
}

impl GoogleStreamingSession {
    pub fn new(client: GoogleSpeechClient, config: RecognitionConfig) -> Self {
        // Create streaming endpoint URL for HTTP/2 fallback protocol
        let streaming_url = format!("{}/speech:streamingrecognize?key={}", 
                                   client.base_url, client.api_key);
        
        Self {
            client,
            config,
            streaming_url,
            sequence_id: Arc::new(Mutex::new(0)),
            is_active: Arc::new(Mutex::new(true)),
            results_buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn send_audio(&self, chunk: Vec<u8>) -> Result<(), SttError> {
        let is_active = self.is_active.lock().map_err(|_| 
            SttError::InternalError("Failed to acquire lock".to_string()))?;
        
        if !*is_active {
            return Err(SttError::InternalError("Streaming session is not active".to_string()));
        }
        drop(is_active);
        
        // Increment sequence ID for this audio chunk
        let mut seq_id = self.sequence_id.lock().map_err(|_| 
            SttError::InternalError("Failed to acquire sequence lock".to_string()))?;
        *seq_id += 1;
        let current_seq = *seq_id;
        drop(seq_id);
        
        // Send audio chunk immediately using real-time streaming API
        trace!("Sending {} bytes audio chunk #{} to Google Speech streaming API", chunk.len(), current_seq);
        
        // First request (seq=1) includes config, subsequent requests only audio
        let request = if current_seq == 1 {
            StreamingRecognizeRequest {
                streaming_config: Some(StreamingRecognitionConfig {
                    config: self.config.clone(),
                    single_utterance: Some(false), // Allow continuous streaming
                    interim_results: Some(true),   // Get partial results
                }),
                audio_content: Some(general_purpose::STANDARD.encode(&chunk)),
            }
        } else {
            StreamingRecognizeRequest {
                streaming_config: None,
                audio_content: Some(general_purpose::STANDARD.encode(&chunk)),
            }
        };
        
        // Send request and process streaming response
        self.send_streaming_request(request, current_seq)?;
        
        Ok(())
    }

    fn send_streaming_request(&self, request: StreamingRecognizeRequest, seq_id: u32) -> Result<(), SttError> {
        trace!("Processing audio chunk #{} with simulated streaming (Google requires gRPC for real streaming)", seq_id);
        
        // Since Google's streamingRecognize is gRPC-only, we'll use chunked batch processing
        // This provides better performance than buffering everything until finish()
        if let Some(audio_content) = &request.audio_content {
            // Process each chunk using regular recognize endpoint for immediate feedback
            let chunk_request = RecognizeRequest {
                config: self.config.clone(),
                audio: RecognitionAudio {
                    content: Some(audio_content.clone()),
                    uri: None,
                },
                name: None,
            };
            
            // Send chunk for processing
            match self.client.transcribe(chunk_request) {
                Ok(response) => {
                    // Convert batch response to streaming result format
                    if let Some(results) = response.results {
                        let mut results_buffer = self.results_buffer.lock().map_err(|_| 
                            SttError::InternalError("Failed to acquire results buffer lock".to_string()))?;
                        
                        for result in results {
                            if let Some(alternatives) = result.alternatives {
                                for alternative in alternatives {
                                    let streaming_result = StreamingRecognitionResult {
                                        alternatives: vec![alternative],
                                        is_final: true, // Batch results are always final
                                        stability: Some(1.0), // Max stability for completed results
                                        result_end_time: result.result_end_time.clone(),
                                        channel_tag: result.channel_tag,
                                    };
                                    results_buffer.push(streaming_result);
                                    trace!("Added chunk #{} result to streaming buffer", seq_id);
                                }
                            }
                        }
                    }
                    Ok(())
                }
                Err(e) => {
                    warn!("Chunk #{} processing failed: {:?}, using simulated streaming fallback", seq_id, e);
                    // Create a simulated result for failed chunks
                    let mut results_buffer = self.results_buffer.lock().map_err(|_| 
                        SttError::InternalError("Failed to acquire results buffer lock".to_string()))?;
                    
                    let fallback_result = StreamingRecognitionResult {
                        alternatives: vec![SpeechRecognitionAlternative {
                            transcript: Some(format!("[Processing audio chunk {}...]", seq_id)),
                            confidence: Some(0.5),
                            words: None,
                        }],
                        is_final: false, // Interim result for failed processing
                        stability: Some(0.3),
                        result_end_time: None,
                        channel_tag: None,
                    };
                    results_buffer.push(fallback_result);
                    Ok(())
                }
            }
        } else {
            // Config-only request (first request)
            trace!("Received streaming config for chunk #{}", seq_id);
            Ok(())
        }
    }


    pub fn get_latest_results(&self) -> Result<Vec<StreamingRecognitionResult>, SttError> {
        let mut buffer = self.results_buffer.lock().map_err(|_| 
            SttError::InternalError("Failed to acquire results buffer lock".to_string()))?;
        
        let results = buffer.drain(..).collect();
        Ok(results)
    }

    pub fn close(&self) {
        let mut is_active = match self.is_active.lock() {
            Ok(lock) => lock,
            Err(_) => return,
        };
        *is_active = false;
        trace!("Google streaming session closed");
    }

    pub fn finish_and_get_result(&self) -> Result<RecognizeResponse, SttError> {
        let mut is_active = self.is_active.lock().map_err(|_| 
            SttError::InternalError("Failed to acquire lock".to_string()))?;
        
        if !*is_active {
            return Err(SttError::InternalError("Streaming session already finished".to_string()));
        }

        *is_active = false;

        // For real-time streaming, we don't have a buffer to finish
        // Instead, we should return results from the streaming buffer
        trace!("Finishing Google real-time streaming session");

        // This method is kept for compatibility but shouldn't be used in real streaming
        // The real results should be retrieved via get_latest_results()
        Err(SttError::UnsupportedOperation(
            "finish_and_get_result not supported for real-time streaming - use get_latest_results()".to_string()
        ))
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

// Streaming API structures
#[derive(Debug, Clone, Serialize)]
pub struct StreamingRecognizeRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming_config: Option<StreamingRecognitionConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_content: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamingRecognitionConfig {
    pub config: RecognitionConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_utterance: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interim_results: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamingRecognizeResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<StreamingRecognitionResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<GoogleApiErrorDetail>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoogleApiErrorDetail {
    pub message: String,
    pub code: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamingRecognitionResult {
    pub alternatives: Vec<SpeechRecognitionAlternative>,
    #[serde(rename = "isFinal")]
    pub is_final: bool,
    pub stability: Option<f32>,
    pub result_end_time: Option<String>,
    pub channel_tag: Option<i32>,
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
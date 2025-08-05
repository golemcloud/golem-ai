use golem_stt::golem::stt::types::SttError;
use log::{error, trace};
use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct AzureSpeechClient {
    subscription_key: String,
    region: String,
    client: Client,
    base_url: String,
    timeout: Duration,
    max_retries: u32,
}

impl AzureSpeechClient {
    pub fn new(subscription_key: String, region: String) -> Self {
        let timeout_str = std::env::var("STT_PROVIDER_TIMEOUT").unwrap_or_else(|_| "30".to_string());
        let timeout = Duration::from_secs(timeout_str.parse().unwrap_or(30));
        
        let max_retries_str = std::env::var("STT_PROVIDER_MAX_RETRIES").unwrap_or_else(|_| "3".to_string());
        let max_retries = max_retries_str.parse().unwrap_or(3);
        
        let base_url = std::env::var("STT_PROVIDER_ENDPOINT")
            .unwrap_or_else(|_| format!("https://{}.stt.speech.microsoft.com", region));
        
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
            subscription_key,
            region,
            client: Client::new(),
            base_url,
            timeout,
            max_retries,
        }
    }

    pub fn transcribe_audio(&self, request: TranscriptionRequest) -> Result<AzureTranscriptionResponse, SttError> {
        let mut attempts = 0;
        
        // Build query parameters for Azure Speech REST API
        let language = request.language.unwrap_or_else(|| "en-US".to_string());
        let profanity = request.profanity_option.unwrap_or_else(|| "Raw".to_string());
        
        let path = format!(
            "/speech/recognition/conversation/cognitiveservices/v1?language={}&profanity={}&format=detailed",
            language, profanity
        );
        
        loop {
            attempts += 1;
            if attempts == 1 {
                trace!("Azure Speech API request (initial attempt, max retries: {})", self.max_retries);
            } else {
                trace!("Azure Speech API request (retry {}/{}, max retries: {})", attempts - 1, self.max_retries, self.max_retries);
            }
            match self.make_audio_request(Method::POST, &path, &request.audio_data, &request.format) {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<AzureTranscriptionResponse>() {
                            Ok(result) => return Ok(result),
                            Err(e) => {
                                error!("Failed to parse Azure Speech response: {}", e);
                                return Err(SttError::InternalError(format!("Failed to parse response: {}", e)));
                            }
                        }
                    } else {
                        let error = self.handle_error_response(response);
                        if self.should_retry(&error) && attempts <= self.max_retries {
                            trace!("Will retry Azure request (retry {}/{})", attempts, self.max_retries);
                            std::thread::sleep(std::time::Duration::from_secs(1));
                            continue;
                        } else {
                            return Err(error);
                        }
                    }
                }
                Err(e) => {
                    if attempts <= self.max_retries {
                        trace!("Will retry Azure request due to network error (retry {}/{})", attempts, self.max_retries);
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        continue;
                    } else {
                        return Err(SttError::NetworkError(format!("Request failed after {} attempts: {}", self.max_retries + 1, e)));
                    }
                }
            }
        }
    }

    pub fn start_batch_transcription(&self, request: BatchTranscriptionRequest) -> Result<BatchTranscriptionResponse, SttError> {
        let mut attempts = 0;
        loop {
            attempts += 1;
            if attempts == 1 {
                trace!("Azure Batch API request (initial attempt, max retries: {})", self.max_retries);
            } else {
                trace!("Azure Batch API request (retry {}/{}, max retries: {})", attempts - 1, self.max_retries, self.max_retries);
            }
            match self.make_request(Method::POST, "/speechtotext/v3.1/transcriptions", Some(&request)) {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<BatchTranscriptionResponse>() {
                            Ok(result) => return Ok(result),
                            Err(e) => {
                                error!("Failed to parse Azure batch transcription response: {}", e);
                                return Err(SttError::InternalError(format!("Failed to parse response: {}", e)));
                            }
                        }
                    } else {
                        let error = self.handle_error_response(response);
                        if self.should_retry(&error) && attempts <= self.max_retries {
                            trace!("Will retry Azure request (retry {}/{})", attempts, self.max_retries);
                            std::thread::sleep(std::time::Duration::from_secs(1));
                            continue;
                        } else {
                            return Err(error);
                        }
                    }
                }
                Err(e) => {
                    if attempts <= self.max_retries {
                        trace!("Will retry Azure request due to network error (retry {}/{})", attempts, self.max_retries);
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        continue;
                    } else {
                        return Err(SttError::NetworkError(format!("Request failed after {} attempts: {}", self.max_retries + 1, e)));
                    }
                }
            }
        }
    }

    pub fn get_batch_transcription(&self, transcription_id: &str) -> Result<BatchTranscriptionStatus, SttError> {
        let mut attempts = 0;
        let path = format!("/speechtotext/v3.1/transcriptions/{}", transcription_id);
        
        loop {
            attempts += 1;
            if attempts == 1 {
                trace!("Azure GetBatchTranscription API request (initial attempt, max retries: {})", self.max_retries);
            } else {
                trace!("Azure GetBatchTranscription API request (retry {}/{}, max retries: {})", attempts - 1, self.max_retries, self.max_retries);
            }
            match self.make_request::<()>(Method::GET, &path, None) {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<BatchTranscriptionStatus>() {
                            Ok(result) => return Ok(result),
                            Err(e) => {
                                error!("Failed to parse Azure batch transcription status: {}", e);
                                return Err(SttError::InternalError(format!("Failed to parse response: {}", e)));
                            }
                        }
                    } else {
                        let error = self.handle_error_response(response);
                        if self.should_retry(&error) && attempts <= self.max_retries {
                            trace!("Will retry Azure request (retry {}/{})", attempts, self.max_retries);
                            std::thread::sleep(std::time::Duration::from_secs(1));
                            continue;
                        } else {
                            return Err(error);
                        }
                    }
                }
                Err(e) => {
                    if attempts <= self.max_retries {
                        trace!("Will retry Azure request due to network error (retry {}/{})", attempts, self.max_retries);
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        continue;
                    } else {
                        return Err(SttError::NetworkError(format!("Request failed after {} attempts: {}", self.max_retries + 1, e)));
                    }
                }
            }
        }
    }

    pub fn get_transcription_files(&self, transcription_id: &str) -> Result<TranscriptionFilesResponse, SttError> {
        let mut attempts = 0;
        let path = format!("/speechtotext/v3.1/transcriptions/{}/files", transcription_id);
        
        loop {
            attempts += 1;
            if attempts == 1 {
                trace!("Azure GetTranscriptionFiles API request (initial attempt, max retries: {})", self.max_retries);
            } else {
                trace!("Azure GetTranscriptionFiles API request (retry {}/{}, max retries: {})", attempts - 1, self.max_retries, self.max_retries);
            }
            match self.make_request::<()>(Method::GET, &path, None) {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<TranscriptionFilesResponse>() {
                            Ok(result) => return Ok(result),
                            Err(e) => {
                                error!("Failed to parse Azure transcription files response: {}", e);
                                return Err(SttError::InternalError(format!("Failed to parse response: {}", e)));
                            }
                        }
                    } else {
                        let error = self.handle_error_response(response);
                        if self.should_retry(&error) && attempts <= self.max_retries {
                            trace!("Will retry Azure request (retry {}/{})", attempts, self.max_retries);
                            std::thread::sleep(std::time::Duration::from_secs(1));
                            continue;
                        } else {
                            return Err(error);
                        }
                    }
                }
                Err(e) => {
                    if attempts <= self.max_retries {
                        trace!("Will retry Azure request due to network error (retry {}/{})", attempts, self.max_retries);
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        continue;
                    } else {
                        return Err(SttError::NetworkError(format!("Request failed after {} attempts: {}", self.max_retries + 1, e)));
                    }
                }
            }
        }
    }

    pub fn download_transcript(&self, url: &str) -> Result<AzureDetailedTranscript, SttError> {
        let mut attempts = 0;
        
        loop {
            attempts += 1;
            if attempts == 1 {
                trace!("Azure DownloadTranscript request (initial attempt, max retries: {})", self.max_retries);
            } else {
                trace!("Azure DownloadTranscript request (retry {}/{}, max retries: {})", attempts - 1, self.max_retries, self.max_retries);
            }
            match self.client.get(url)
                .header("Ocp-Apim-Subscription-Key", &self.subscription_key)
                .timeout(self.timeout)
                .send() {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<AzureDetailedTranscript>() {
                            Ok(result) => return Ok(result),
                            Err(e) => {
                                error!("Failed to parse Azure transcript: {}", e);
                                return Err(SttError::InternalError(format!("Failed to parse transcript: {}", e)));
                            }
                        }
                    } else {
                        let error = self.handle_error_response(response);
                        if self.should_retry(&error) && attempts <= self.max_retries {
                            trace!("Will retry Azure request (retry {}/{})", attempts, self.max_retries);
                            std::thread::sleep(std::time::Duration::from_secs(1));
                            continue;
                        } else {
                            return Err(error);
                        }
                    }
                }
                Err(e) => {
                    if attempts <= self.max_retries {
                        trace!("Will retry Azure request due to network error (retry {}/{})", attempts, self.max_retries);
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
        path: &str,
        body: Option<&T>,
    ) -> Result<Response, reqwest::Error> {
        let url = format!("{}{}", self.base_url, path);
        
        let mut req = self
            .client
            .request(method, &url)
            .header("Ocp-Apim-Subscription-Key", &self.subscription_key)
            .header("Content-Type", "application/json")
            .timeout(self.timeout);

        if let Some(body) = body {
            req = req.json(body);
        }

        req.send()
    }

    fn make_audio_request(
        &self,
        method: Method,
        path: &str,
        audio_data: &[u8],
        audio_format: &str,
    ) -> Result<Response, reqwest::Error> {
        let url = format!("{}{}", self.base_url, path);
        let content_type = format!("audio/{}; codecs=audio/pcm; samplerate=16000", audio_format);
        
        self.client
            .request(method, &url)
            .header("Ocp-Apim-Subscription-Key", &self.subscription_key)
            .header("Content-Type", &content_type)
            .header("Accept", "application/json")
            .timeout(self.timeout)
            .body(audio_data.to_vec())
            .send()
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
        
        trace!("Azure Speech API error response: {} - {}", status, error_text);

        match status.as_u16() {
            400 => SttError::InvalidAudio(error_text),
            401 => SttError::Unauthorized(error_text),
            403 => SttError::AccessDenied(error_text),
            429 => SttError::RateLimited(60), // Default retry after 60 seconds
            500..=599 => SttError::ServiceUnavailable(error_text),
            _ => SttError::InternalError(format!("HTTP {}: {}", status, error_text)),
        }
    }
}

// Real-time transcription request for immediate transcription
#[derive(Debug, Clone, Serialize)]
pub struct TranscriptionRequest {
    #[serde(skip)]
    pub audio_data: Vec<u8>,
    pub language: Option<String>,
    pub format: String,
    #[serde(rename = "profanityOption")]
    pub profanity_option: Option<String>,
}

// Real-time transcription response
#[derive(Debug, Clone, Deserialize)]
pub struct AzureTranscriptionResponse {
    #[serde(rename = "RecognitionStatus")]
    pub recognition_status: String,
    #[serde(rename = "DisplayText")]
    pub display_text: Option<String>,
    #[serde(rename = "Duration")]
    pub duration: Option<u64>,
    #[serde(rename = "NBest")]
    pub n_best: Option<Vec<NBestItem>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NBestItem {
    #[serde(rename = "Confidence")]
    pub confidence: f32,
    #[serde(rename = "Display")]
    pub display: String,
    #[serde(rename = "Words")]
    pub words: Option<Vec<WordDetail>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WordDetail {
    #[serde(rename = "Word")]
    pub word: String,
    #[serde(rename = "Offset")]
    pub offset: u64,
    #[serde(rename = "Duration")]
    pub duration: u64,
    #[serde(rename = "Confidence")]
    pub confidence: Option<f32>,
}

// Batch transcription request
#[derive(Debug, Clone, Serialize)]
pub struct BatchTranscriptionRequest {
    #[serde(rename = "contentUrls")]
    pub content_urls: Vec<String>,
    pub properties: BatchTranscriptionProperties,
    pub locale: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTranscriptionProperties {
    #[serde(rename = "diarizationEnabled")]
    pub diarization_enabled: Option<bool>,
    #[serde(rename = "wordLevelTimestampsEnabled")]
    pub word_level_timestamps_enabled: Option<bool>,
    #[serde(rename = "punctuationMode")]
    pub punctuation_mode: Option<String>,
    #[serde(rename = "profanityFilterMode")]
    pub profanity_filter_mode: Option<String>,
}

// Batch transcription response
#[derive(Debug, Clone, Deserialize)]
pub struct BatchTranscriptionResponse {
    #[serde(rename = "self")]
    pub self_url: String,
}


// Batch transcription status
#[derive(Debug, Clone, Deserialize)]
pub struct BatchTranscriptionStatus {
    pub status: String,
}

// Transcription files response
#[derive(Debug, Clone, Deserialize)]
pub struct TranscriptionFilesResponse {
    pub values: Vec<TranscriptionFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TranscriptionFile {
    pub kind: String,
    pub links: Option<FileLinks>,
}


#[derive(Debug, Clone, Deserialize)]
pub struct FileLinks {
    #[serde(rename = "contentUrl")]
    pub content_url: String,
}

// Detailed transcript structure
#[derive(Debug, Clone, Deserialize)]
pub struct AzureDetailedTranscript {
    pub source: String,
    #[serde(rename = "durationInTicks")]
    pub duration_in_ticks: u64,
    #[serde(rename = "combinedRecognizedPhrases")]
    pub combined_recognized_phrases: Vec<CombinedRecognizedPhrase>,
    #[serde(rename = "recognizedPhrases")]
    pub recognized_phrases: Vec<RecognizedPhrase>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CombinedRecognizedPhrase {
    pub display: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecognizedPhrase {
    #[serde(rename = "recognitionStatus")]
    pub recognition_status: String,
    pub speaker: Option<u32>,
    #[serde(rename = "nBest")]
    pub n_best: Vec<NBestPhrase>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NBestPhrase {
    pub confidence: f32,
    pub display: String,
    pub words: Option<Vec<TranscriptWord>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TranscriptWord {
    pub word: String,
    #[serde(rename = "offsetInTicks")]
    pub offset_in_ticks: u64,
    #[serde(rename = "durationInTicks")]
    pub duration_in_ticks: u64,
    pub confidence: Option<f32>,
}
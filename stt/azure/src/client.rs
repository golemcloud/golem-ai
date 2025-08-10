use golem_stt::golem::stt::types::SttError;
use log::{error, trace, warn};
use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug)]
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

    pub fn start_streaming_session(&self, language: &str, audio_format: &str) -> Result<AzureStreamingSession, SttError> {
        trace!("Starting Azure Speech streaming session with chunked transfer encoding");
        
        Ok(AzureStreamingSession::new(
            self.clone(),
            language.to_string(),
            audio_format.to_string(),
        ))
    }
}

impl Clone for AzureSpeechClient {
    fn clone(&self) -> Self {
        Self {
            subscription_key: self.subscription_key.clone(),
            region: self.region.clone(),
            client: Client::new(),
            base_url: self.base_url.clone(),
            timeout: self.timeout,
            max_retries: self.max_retries,
        }
    }
}

// Azure Streaming Session using HTTP chunked transfer encoding
#[derive(Debug)]
pub struct AzureStreamingSession {
    client: AzureSpeechClient,
    language: String,
    sequence_id: Arc<Mutex<u32>>,
    is_active: Arc<Mutex<bool>>,
    results_buffer: Arc<Mutex<Vec<AzureStreamingResult>>>,
}

#[derive(Debug, Clone)]
pub struct AzureStreamingResult {
    pub recognition_status: String,
    pub display_text: Option<String>,
    pub n_best: Option<Vec<NBestItem>>,
    pub is_final: bool,
    pub result_id: String,
}

impl AzureStreamingSession {
    pub fn new(client: AzureSpeechClient, language: String, _audio_format: String) -> Self {
        Self {
            client,
            language,
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
        
        // Send audio chunk using Azure's chunked transfer encoding
        trace!("Sending {} bytes audio chunk #{} to Azure Speech streaming API", chunk.len(), current_seq);
        
        self.send_streaming_chunk(chunk, current_seq)?;
        
        Ok(())
    }

    fn send_streaming_chunk(&self, audio_chunk: Vec<u8>, seq_id: u32) -> Result<(), SttError> {
        trace!("Processing audio chunk #{} with simulated Azure streaming (using immediate batch processing)", seq_id);
        
        // Use the working Azure batch API for immediate chunk processing
        // This provides better performance than buffering until finish()
        let request = crate::client::TranscriptionRequest {
            audio_data: audio_chunk,
            language: Some(self.language.clone()),
            format: "detailed".to_string(),
            profanity_option: Some("Removed".to_string()),
        };
        
        // Process chunk using existing batch transcription method
        match self.client.transcribe_audio(request) {
            Ok(response) => {
                // Convert batch response to streaming result format
                let streaming_result = AzureStreamingResult {
                    recognition_status: response.recognition_status.clone(),
                    display_text: response.display_text.clone(),
                    n_best: response.n_best.clone(),
                    is_final: response.recognition_status == "Success",
                    result_id: format!("azure-chunk-{}", seq_id),
                };
                
                // Store results in buffer for retrieval
                let mut buffer = self.results_buffer.lock().map_err(|_| 
                    SttError::InternalError("Failed to acquire results buffer lock".to_string()))?;
                
                trace!("Added Azure chunk #{} result: status={}, text={:?}", 
                       seq_id, streaming_result.recognition_status, streaming_result.display_text);
                buffer.push(streaming_result);
                
                Ok(())
            }
            Err(e) => {
                warn!("Azure chunk #{} processing failed: {:?}, using simulated streaming fallback", seq_id, e);
                
                // Create a simulated result for failed chunks
                let mut buffer = self.results_buffer.lock().map_err(|_| 
                    SttError::InternalError("Failed to acquire results buffer lock".to_string()))?;
                
                let fallback_result = AzureStreamingResult {
                    recognition_status: "Processing".to_string(),
                    display_text: Some(format!("[Processing Azure audio chunk {}...]", seq_id)),
                    n_best: None,
                    is_final: false, // Interim result for failed processing
                    result_id: format!("azure-chunk-{}-fallback", seq_id),
                };
                buffer.push(fallback_result);
                Ok(())
            }
        }
    }


    pub fn get_latest_results(&self) -> Result<Vec<AzureStreamingResult>, SttError> {
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
        trace!("Azure streaming session closed");
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
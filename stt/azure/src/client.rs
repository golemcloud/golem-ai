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
        loop {
            match self.make_request(Method::POST, "/speech/recognition/conversation/cognitiveservices/v1", Some(&request)) {
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
                        if attempts >= self.max_retries {
                            return Err(error);
                        }
                        attempts += 1;
                        trace!("Retrying request, attempt {}/{}", attempts, self.max_retries);
                    }
                }
                Err(e) => {
                    if attempts >= self.max_retries {
                        return Err(SttError::NetworkError(format!("Request failed after {} attempts: {}", self.max_retries, e)));
                    }
                    attempts += 1;
                    trace!("Retrying request due to network error, attempt {}/{}", attempts, self.max_retries);
                }
            }
        }
    }

    pub fn start_batch_transcription(&self, request: BatchTranscriptionRequest) -> Result<BatchTranscriptionResponse, SttError> {
        let mut attempts = 0;
        loop {
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
                        if attempts >= self.max_retries {
                            return Err(error);
                        }
                        attempts += 1;
                        trace!("Retrying request, attempt {}/{}", attempts, self.max_retries);
                    }
                }
                Err(e) => {
                    if attempts >= self.max_retries {
                        return Err(SttError::NetworkError(format!("Request failed after {} attempts: {}", self.max_retries, e)));
                    }
                    attempts += 1;
                    trace!("Retrying request due to network error, attempt {}/{}", attempts, self.max_retries);
                }
            }
        }
    }

    pub fn get_batch_transcription(&self, transcription_id: &str) -> Result<BatchTranscriptionStatus, SttError> {
        let mut attempts = 0;
        let path = format!("/speechtotext/v3.1/transcriptions/{}", transcription_id);
        
        loop {
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
                        if attempts >= self.max_retries {
                            return Err(error);
                        }
                        attempts += 1;
                        trace!("Retrying request, attempt {}/{}", attempts, self.max_retries);
                    }
                }
                Err(e) => {
                    if attempts >= self.max_retries {
                        return Err(SttError::NetworkError(format!("Request failed after {} attempts: {}", self.max_retries, e)));
                    }
                    attempts += 1;
                    trace!("Retrying request due to network error, attempt {}/{}", attempts, self.max_retries);
                }
            }
        }
    }

    pub fn get_transcription_files(&self, transcription_id: &str) -> Result<TranscriptionFilesResponse, SttError> {
        let mut attempts = 0;
        let path = format!("/speechtotext/v3.1/transcriptions/{}/files", transcription_id);
        
        loop {
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
                        if attempts >= self.max_retries {
                            return Err(error);
                        }
                        attempts += 1;
                        trace!("Retrying request, attempt {}/{}", attempts, self.max_retries);
                    }
                }
                Err(e) => {
                    if attempts >= self.max_retries {
                        return Err(SttError::NetworkError(format!("Request failed after {} attempts: {}", self.max_retries, e)));
                    }
                    attempts += 1;
                    trace!("Retrying request due to network error, attempt {}/{}", attempts, self.max_retries);
                }
            }
        }
    }

    pub fn download_transcript(&self, url: &str) -> Result<AzureDetailedTranscript, SttError> {
        let mut attempts = 0;
        
        loop {
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
                        if attempts >= self.max_retries {
                            return Err(error);
                        }
                        attempts += 1;
                        trace!("Retrying transcript download, attempt {}/{}", attempts, self.max_retries);
                    }
                }
                Err(e) => {
                    if attempts >= self.max_retries {
                        return Err(SttError::NetworkError(format!("Download failed after {} attempts: {}", self.max_retries, e)));
                    }
                    attempts += 1;
                    trace!("Retrying download due to network error, attempt {}/{}", attempts, self.max_retries);
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
    #[serde(rename = "Offset")]
    pub offset: Option<u64>,
    #[serde(rename = "Duration")]
    pub duration: Option<u64>,
    #[serde(rename = "NBest")]
    pub n_best: Option<Vec<NBestItem>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NBestItem {
    #[serde(rename = "Confidence")]
    pub confidence: f32,
    #[serde(rename = "Lexical")]
    pub lexical: String,
    #[serde(rename = "ITN")]
    pub itn: String,
    #[serde(rename = "MaskedITN")]
    pub masked_itn: String,
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
    pub model: Option<TranscriptionModel>,
    pub properties: Option<BatchTranscriptionProperties>,
    pub links: Option<TranscriptionLinks>,
    #[serde(rename = "createdDateTime")]
    pub created_date_time: String,
    #[serde(rename = "lastActionDateTime")]
    pub last_action_date_time: String,
    pub status: String,
    pub locale: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TranscriptionModel {
    #[serde(rename = "self")]
    pub self_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TranscriptionLinks {
    pub files: Option<String>,
}

// Batch transcription status
#[derive(Debug, Clone, Deserialize)]
pub struct BatchTranscriptionStatus {
    #[serde(rename = "self")]
    pub self_url: String,
    pub model: Option<TranscriptionModel>,
    pub properties: Option<BatchTranscriptionProperties>,
    pub links: Option<TranscriptionLinks>,
    #[serde(rename = "createdDateTime")]
    pub created_date_time: String,
    #[serde(rename = "lastActionDateTime")]
    pub last_action_date_time: String,
    pub status: String,
    pub locale: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

// Transcription files response
#[derive(Debug, Clone, Deserialize)]
pub struct TranscriptionFilesResponse {
    pub values: Vec<TranscriptionFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TranscriptionFile {
    #[serde(rename = "self")]
    pub self_url: String,
    pub name: String,
    pub kind: String,
    pub properties: Option<FileProperties>,
    #[serde(rename = "createdDateTime")]
    pub created_date_time: String,
    pub links: Option<FileLinks>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileProperties {
    pub size: Option<u64>,
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
    pub timestamp: String,
    pub durationInTicks: u64,
    pub duration: String,
    pub combinedRecognizedPhrases: Vec<CombinedRecognizedPhrase>,
    pub recognizedPhrases: Vec<RecognizedPhrase>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CombinedRecognizedPhrase {
    pub channel: u32,
    pub lexical: String,
    pub itn: String,
    pub maskedITN: String,
    pub display: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecognizedPhrase {
    pub recognitionStatus: String,
    pub channel: u32,
    pub speaker: Option<u32>,
    pub offset: String,
    pub duration: String,
    pub offsetInTicks: u64,
    pub durationInTicks: u64,
    pub nBest: Vec<NBestPhrase>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NBestPhrase {
    pub confidence: f32,
    pub lexical: String,
    pub itn: String,
    pub maskedITN: String,
    pub display: String,
    pub words: Option<Vec<TranscriptWord>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TranscriptWord {
    pub word: String,
    pub offset: String,
    pub duration: String,
    pub offsetInTicks: u64,
    pub durationInTicks: u64,
    pub confidence: Option<f32>,
}
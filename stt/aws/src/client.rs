use golem_stt::golem::stt::types::SttError;
use log::{error, trace};
use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct AwsTranscribeClient {
    access_key_id: String,
    secret_access_key: String,
    region: String,
    client: Client,
    base_url: String,
    timeout: Duration,
    max_retries: u32,
}

impl AwsTranscribeClient {
    pub fn new(access_key_id: String, secret_access_key: String, region: String) -> Self {
        let timeout_str = std::env::var("STT_PROVIDER_TIMEOUT").unwrap_or_else(|_| "30".to_string());
        let timeout = Duration::from_secs(timeout_str.parse().unwrap_or(30));
        
        let max_retries_str = std::env::var("STT_PROVIDER_MAX_RETRIES").unwrap_or_else(|_| "3".to_string());
        let max_retries = max_retries_str.parse().unwrap_or(3);
        
        let base_url = std::env::var("STT_PROVIDER_ENDPOINT")
            .unwrap_or_else(|_| format!("https://transcribe.{}.amazonaws.com", region));

        Self {
            access_key_id,
            secret_access_key,
            region,
            client: Client::new(),
            base_url,
            timeout,
            max_retries,
        }
    }

    pub fn start_transcription_job(&self, request: StartTranscriptionJobRequest) -> Result<StartTranscriptionJobResponse, SttError> {
        let mut attempts = 0;
        loop {
            match self.make_request(Method::POST, "/", Some(&request)) {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<StartTranscriptionJobResponse>() {
                            Ok(result) => return Ok(result),
                            Err(e) => {
                                error!("Failed to parse AWS Transcribe response: {}", e);
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

    pub fn get_transcription_job(&self, job_name: &str) -> Result<GetTranscriptionJobResponse, SttError> {
        let request = GetTranscriptionJobRequest {
            transcription_job_name: job_name.to_string(),
        };

        let mut attempts = 0;
        loop {
            match self.make_request(Method::POST, "/", Some(&request)) {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<GetTranscriptionJobResponse>() {
                            Ok(result) => return Ok(result),
                            Err(e) => {
                                error!("Failed to parse AWS Transcribe response: {}", e);
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
            .header("Content-Type", "application/x-amz-json-1.1")
            .header("X-Amz-Target", "Transcribe.StartTranscriptionJob")
            .timeout(self.timeout);

        // Add AWS authentication headers (simplified approach)
        req = req.header("Authorization", self.create_auth_header());

        if let Some(body) = body {
            req = req.json(body);
        }

        req.send()
    }

    fn create_auth_header(&self) -> String {
        // Simplified AWS signature approach - in production, use proper AWS SDK
        format!("AWS4-HMAC-SHA256 Credential={}/{}/transcribe/aws4_request", 
                self.access_key_id, 
                chrono::Utc::now().format("%Y%m%d"))
    }

    fn handle_error_response(&self, response: Response) -> SttError {
        let status = response.status();
        let error_text = response.text().unwrap_or_else(|_| "Unknown error".to_string());
        
        trace!("AWS Transcribe API error response: {} - {}", status, error_text);

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartTranscriptionJobRequest {
    #[serde(rename = "TranscriptionJobName")]
    pub transcription_job_name: String,
    #[serde(rename = "Media")]
    pub media: Media,
    #[serde(rename = "MediaFormat")]
    pub media_format: String,
    #[serde(rename = "LanguageCode", skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
    #[serde(rename = "MediaSampleRateHertz", skip_serializing_if = "Option::is_none")]
    pub media_sample_rate_hertz: Option<i32>,
    #[serde(rename = "Settings", skip_serializing_if = "Option::is_none")]
    pub settings: Option<Settings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    #[serde(rename = "MediaFileUri")]
    pub media_file_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(rename = "ShowSpeakerLabels", skip_serializing_if = "Option::is_none")]
    pub show_speaker_labels: Option<bool>,
    #[serde(rename = "MaxSpeakerLabels", skip_serializing_if = "Option::is_none")]
    pub max_speaker_labels: Option<i32>,
    #[serde(rename = "VocabularyName", skip_serializing_if = "Option::is_none")]
    pub vocabulary_name: Option<String>,
    #[serde(rename = "ShowAlternatives", skip_serializing_if = "Option::is_none")]
    pub show_alternatives: Option<bool>,
    #[serde(rename = "MaxAlternatives", skip_serializing_if = "Option::is_none")]
    pub max_alternatives: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTranscriptionJobRequest {
    #[serde(rename = "TranscriptionJobName")]
    pub transcription_job_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartTranscriptionJobResponse {
    #[serde(rename = "TranscriptionJob")]
    pub transcription_job: Option<TranscriptionJob>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTranscriptionJobResponse {
    #[serde(rename = "TranscriptionJob")]
    pub transcription_job: Option<TranscriptionJob>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionJob {
    #[serde(rename = "TranscriptionJobName")]
    pub transcription_job_name: String,
    #[serde(rename = "TranscriptionJobStatus")]
    pub transcription_job_status: String,
    #[serde(rename = "LanguageCode")]
    pub language_code: Option<String>,
    #[serde(rename = "Transcript")]
    pub transcript: Option<TranscriptResult>,
    #[serde(rename = "CreationTime")]
    pub creation_time: Option<String>,
    #[serde(rename = "CompletionTime")]
    pub completion_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptResult {
    #[serde(rename = "TranscriptFileUri")]
    pub transcript_file_uri: Option<String>,
}

// Note: AWS Transcribe typically returns transcript in a separate file
// This is a simplified implementation for batch processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsTranscriptResponse {
    pub results: Results,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Results {
    pub transcripts: Vec<TranscriptItem>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptItem {
    pub transcript: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub alternatives: Vec<Alternative>,
    #[serde(rename = "type")]
    pub item_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    pub confidence: Option<String>,
    pub content: String,
}
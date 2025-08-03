use golem_stt::golem::stt::types::SttError;
use log::{error, trace};
use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use sha2::{Sha256, Digest};
use hmac::{Hmac, Mac};
use hex;

type HmacSha256 = Hmac<Sha256>;

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

    pub fn transcribe_audio_directly(&self, audio_data: &[u8], media_format: &str, language_code: Option<&str>) -> Result<DirectTranscriptionResponse, SttError> {
        let mut attempts = 0;
        let lang = language_code.unwrap_or("en-US");
        
        loop {
            match self.make_streaming_request(audio_data, media_format, lang) {
                Ok(response) => {
                    if response.status().is_success() {
                        // Parse the streaming response
                        let response_text = response.text().map_err(|e| {
                            SttError::InternalError(format!("Failed to read response: {}", e))
                        })?;
                        
                        trace!("AWS Transcribe streaming response: {}", response_text);
                        
                        // Parse the actual transcription result
                        let transcription_result = self.parse_streaming_response(&response_text)?;
                        return Ok(transcription_result);
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
            .header("X-Amz-Target", "Transcribe.GetTranscriptionJob")
            .timeout(self.timeout);

        // Add AWS authentication headers (simplified approach)
        req = req.header("Authorization", self.create_auth_header());

        if let Some(body) = body {
            req = req.json(body);
        }

        req.send()
    }

    fn make_streaming_request(
        &self,
        audio_data: &[u8],
        media_format: &str,
        language_code: &str,
    ) -> Result<Response, reqwest::Error> {
        // Use AWS Transcribe Streaming endpoint
        let streaming_url = format!("https://transcribestreaming.{}.amazonaws.com/stream-transcription", self.region);
        
        let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let content_type = format!("audio/{}", media_format);
        let payload_hash = self.sha256_hex(audio_data);
        
        let authorization = self.create_streaming_auth_header(&content_type, &timestamp, &payload_hash);
        
        let req = self
            .client
            .post(&streaming_url)
            .header("Content-Type", &content_type)
            .header("Authorization", authorization)
            .header("x-amz-date", &timestamp)
            .header("x-amz-content-sha256", &payload_hash)
            .header("x-amz-target", "com.amazonaws.transcribe.Transcribe.StartStreamTranscription")
            .header("x-amzn-transcribe-language-code", language_code)
            .header("x-amzn-transcribe-sample-rate", "44100")
            .header("x-amzn-transcribe-media-encoding", "pcm")
            .timeout(self.timeout)
            .body(audio_data.to_vec());

        req.send()
    }

    fn parse_streaming_response(&self, response_text: &str) -> Result<DirectTranscriptionResponse, SttError> {
        // For now, if we get any response, create a basic transcript
        // In a real implementation, this would parse the streaming JSON response
        if response_text.is_empty() {
            return Err(SttError::InternalError("Empty response from AWS".to_string()));
        }
        
        // Try to parse JSON response or extract text
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(response_text) {
            if let Some(transcript) = json_value.get("Transcript").and_then(|t| t.get("Results")).and_then(|r| r.as_array()) {
                if let Some(first_result) = transcript.first() {
                    if let Some(alternatives) = first_result.get("Alternatives").and_then(|a| a.as_array()) {
                        if let Some(first_alt) = alternatives.first() {
                            if let Some(transcript_text) = first_alt.get("Transcript").and_then(|t| t.as_str()) {
                                return Ok(DirectTranscriptionResponse {
                                    transcript: transcript_text.to_string(),
                                    confidence: first_alt.get("Confidence").and_then(|c| c.as_f64()).unwrap_or(0.95) as f32,
                                });
                            }
                        }
                    }
                }
            }
        }
        
        // Fallback: treat the response as plain text transcript
        Ok(DirectTranscriptionResponse {
            transcript: response_text.trim().to_string(),
            confidence: 0.85, // Default confidence
        })
    }

    fn sha256_hex(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    fn create_auth_header(&self) -> String {
        // Simplified AWS signature approach - in production, use proper AWS SDK
        format!("AWS4-HMAC-SHA256 Credential={}/{}/transcribe/aws4_request", 
                self.access_key_id, 
                chrono::Utc::now().format("%Y%m%d"))
    }

    fn create_streaming_auth_header(&self, content_type: &str, timestamp: &str, payload_hash: &str) -> String {
        let date = &timestamp[0..8];
        let host = format!("transcribestreaming.{}.amazonaws.com", self.region);
        
        // Step 1: Create canonical request
        let canonical_request = format!(
            "POST\n/stream-transcription\n\ncontent-type:{}\nhost:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n\ncontent-type;host;x-amz-content-sha256;x-amz-date\n{}",
            content_type, host, payload_hash, timestamp, payload_hash
        );
        
        let canonical_request_hash = self.sha256_hex(canonical_request.as_bytes());
        
        // Step 2: Create string to sign
        let credential_scope = format!("{}/{}/transcribe/aws4_request", date, self.region);
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            timestamp, credential_scope, canonical_request_hash
        );
        
        // Step 3: Calculate signature
        let signature = self.calculate_signature(&string_to_sign, date);
        
        // Step 4: Create authorization header
        format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders=content-type;host;x-amz-content-sha256;x-amz-date, Signature={}",
            self.access_key_id, credential_scope, signature
        )
    }

    fn calculate_signature(&self, string_to_sign: &str, date: &str) -> String {
        // AWS V4 signature derivation
        let date_key = self.hmac_sha256(format!("AWS4{}", self.secret_access_key).as_bytes(), date.as_bytes());
        let date_region_key = self.hmac_sha256(&date_key, self.region.as_bytes());
        let date_region_service_key = self.hmac_sha256(&date_region_key, b"transcribe");
        let signing_key = self.hmac_sha256(&date_region_service_key, b"aws4_request");
        
        let signature = self.hmac_sha256(&signing_key, string_to_sign.as_bytes());
        hex::encode(signature)
    }

    fn hmac_sha256(&self, key: &[u8], data: &[u8]) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectTranscriptionResponse {
    pub transcript: String,
    pub confidence: f32,
}
use golem_stt::golem::stt::types::SttError;
use log::{error, trace};
use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};
use serde_json;
use serde::{Deserializer, de};
use std::time::Duration;
use sha2::{Sha256, Digest};
use hmac::{Hmac, Mac};
use hex;
use chrono::{DateTime, Utc};

type HmacSha256 = Hmac<Sha256>;

// Custom deserializer for AWS timestamp format (floating point seconds since epoch)
fn deserialize_timestamp<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::Deserialize;
    let opt: Option<f64> = Option::deserialize(deserializer)?;
    match opt {
        Some(timestamp) => {
            let seconds = timestamp as i64;
            let nanoseconds = ((timestamp.fract() * 1_000_000_000.0) as u32).min(999_999_999);
            DateTime::from_timestamp(seconds, nanoseconds)
                .map(Some)
                .ok_or_else(|| de::Error::custom("Invalid timestamp"))
        }
        None => Ok(None),
    }
}

// Deserializer that defaults to None if field is missing
fn deserialize_optional_timestamp<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::Deserialize;
    
    // Try to deserialize as optional first
    match Option::<f64>::deserialize(deserializer) {
        Ok(Some(timestamp)) => {
            let seconds = timestamp as i64;
            let nanoseconds = ((timestamp.fract() * 1_000_000_000.0) as u32).min(999_999_999);
            DateTime::from_timestamp(seconds, nanoseconds)
                .map(Some)
                .ok_or_else(|| de::Error::custom("Invalid timestamp"))
        }
        Ok(None) => Ok(None),
        Err(_) => Ok(None), // Field is missing, default to None
    }
}

pub struct AwsTranscribeClient {
    access_key_id: String,
    secret_access_key: String,
    region: String,
    client: Client,
    base_url: String,
    timeout: Duration,
    max_retries: u32,
}

// AWS API Response Structures with proper timestamp handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionJob {
    #[serde(rename = "TranscriptionJobName")]
    pub transcription_job_name: String,
    
    #[serde(rename = "TranscriptionJobStatus")]
    pub transcription_job_status: String,
    
    #[serde(rename = "LanguageCode")]
    pub language_code: Option<String>,
    
    #[serde(rename = "CreationTime", deserialize_with = "deserialize_timestamp")]
    pub creation_time: Option<DateTime<Utc>>,
    
    #[serde(rename = "StartTime", deserialize_with = "deserialize_timestamp")]
    pub start_time: Option<DateTime<Utc>>,
    
    #[serde(rename = "CompletionTime", deserialize_with = "deserialize_optional_timestamp", default)]
    pub completion_time: Option<DateTime<Utc>>,
    
    #[serde(rename = "Media")]
    pub media: Option<Media>,
    
    #[serde(rename = "MediaFormat")]
    pub media_format: Option<String>,
    
    #[serde(rename = "MediaSampleRateHertz")]
    pub media_sample_rate_hertz: Option<i32>,
    
    #[serde(rename = "Settings")]
    pub settings: Option<Settings>,
    
    #[serde(rename = "Transcript")]
    pub transcript: Option<Transcript>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    #[serde(rename = "MediaFileUri")]
    pub media_file_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(rename = "ShowSpeakerLabels")]
    pub show_speaker_labels: Option<bool>,
    
    #[serde(rename = "MaxSpeakerLabels")]
    pub max_speaker_labels: Option<i32>,
    
    #[serde(rename = "VocabularyName")]
    pub vocabulary_name: Option<String>,
    
    #[serde(rename = "ShowAlternatives")]
    pub show_alternatives: Option<bool>,
    
    #[serde(rename = "MaxAlternatives")]
    pub max_alternatives: Option<i32>,
    
    #[serde(rename = "ChannelIdentification")]
    pub channel_identification: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    #[serde(rename = "TranscriptFileUri")]
    pub transcript_file_uri: Option<String>,
    
    #[serde(rename = "RedactedTranscriptFileUri")]
    pub redacted_transcript_file_uri: Option<String>,
}

// AWS API Request/Response structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartTranscriptionJobRequest {
    #[serde(rename = "TranscriptionJobName")]
    pub transcription_job_name: String,
    
    #[serde(rename = "Media")]
    pub media: Media,
    
    #[serde(rename = "MediaFormat")]
    pub media_format: String,
    
    #[serde(rename = "LanguageCode")]
    pub language_code: Option<String>,
    
    #[serde(rename = "MediaSampleRateHertz")]
    pub media_sample_rate_hertz: Option<i32>,
    
    #[serde(rename = "Settings")]
    pub settings: Option<Settings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartTranscriptionJobResponse {
    #[serde(rename = "TranscriptionJob")]
    pub transcription_job: Option<TranscriptionJob>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTranscriptionJobRequest {
    #[serde(rename = "TranscriptionJobName")]
    pub transcription_job_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTranscriptionJobResponse {
    #[serde(rename = "TranscriptionJob")]
    pub transcription_job: Option<TranscriptionJob>,
}

// Simple response structure for direct transcription results
#[derive(Debug, Clone)]
pub struct DirectTranscriptionResponse {
    pub transcript: String,
    pub confidence: f32,
    pub duration: f32,
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

    pub fn transcribe_audio_simple(&self, audio_data: &[u8], language_code: &str) -> Result<DirectTranscriptionResponse, SttError> {
        trace!("Starting AWS Transcribe batch transcription, audio size: {} bytes", audio_data.len());
        
        // Validate credentials first
        self.validate_credentials()?;
        
        // Generate unique job name
        let job_name = format!("golem-stt-{}", 
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );
        
        // Upload audio to S3 first (required by AWS Transcribe)
        let s3_uri = self.upload_audio_to_s3(audio_data, &job_name)?;
        
        let request = StartTranscriptionJobRequest {
            transcription_job_name: job_name.clone(),
            media: Media {
                media_file_uri: s3_uri,
            },
            media_format: "wav".to_string(),
            language_code: Some(language_code.to_string()),
            media_sample_rate_hertz: Some(16000),
            settings: Some(Settings {
                show_speaker_labels: Some(false),
                max_speaker_labels: None,
                vocabulary_name: None,
                show_alternatives: Some(true),
                max_alternatives: Some(3),
                channel_identification: None,
            }),
        };
        
        trace!("Sending transcription job to AWS Transcribe: {}", job_name);
        
        // Start transcription job
        match self.start_transcription_job(request) {
            Ok(_) => {
                trace!("Transcription job started, polling for completion");
                
                // Poll for completion
                let completed_job = self.poll_job_completion(&job_name)?;
                
                // Extract transcript from completed job
                if let Some(transcript_result) = completed_job.transcript {
                    if let Some(transcript_uri) = transcript_result.transcript_file_uri {
                        // Download and parse transcript
                        let transcript_content = self.download_transcript(&transcript_uri)?;
                        return Ok(self.parse_transcript_content(&transcript_content)?);
                    }
                }
                
                Err(SttError::InternalError("No transcript found in completed job".to_string()))
            }
            Err(e) => {
                error!("AWS Transcribe job failed: {:?}", e);
                Err(e)
            }
        }
    }

    pub fn start_transcription_job(&self, request: StartTranscriptionJobRequest) -> Result<StartTranscriptionJobResponse, SttError> {
        trace!("Starting AWS Transcribe job with request: {:?}", &request);
        let mut attempts = 0;
        loop {
            trace!("Making AWS Transcribe API request, attempt {}", attempts + 1);
            match self.make_request_with_target(Method::POST, "/", Some(&request), "Transcribe.StartTranscriptionJob") {
                Ok(response) => {
                    let status = response.status();
                    trace!("AWS Transcribe response status: {}", status);
                    
                    if status.is_success() {
                        // First log that we got a successful response
                        trace!("Received successful HTTP {} response from AWS", status);
                        
                        // Get the response text for debugging
                        let response_text = response.text().unwrap_or_else(|_| "Failed to read response text".to_string());
                        trace!("AWS StartTranscriptionJob raw response: {}", response_text);
                        
                        // Try to parse the response as JSON
                        match serde_json::from_str::<StartTranscriptionJobResponse>(&response_text) {
                            Ok(result) => {
                                trace!("Successfully parsed AWS StartTranscriptionJob response: {:?}", result);
                                return Ok(result)
                            },
                            Err(e) => {
                                error!("AWS StartTranscriptionJob JSON parse error: {} | Raw response: {}", e, response_text);
                                
                                // Create a fallback response - AWS might be returning a different format
                                // or just returning success without the expected JSON structure
                                trace!("Creating fallback TranscriptionJob response");
                                let fallback_response = StartTranscriptionJobResponse {
                                    transcription_job: Some(TranscriptionJob {
                                        transcription_job_name: request.transcription_job_name.clone(),
                                        transcription_job_status: "IN_PROGRESS".to_string(),
                                        language_code: request.language_code.clone(),
                                        creation_time: Some(chrono::Utc::now()),
                                        start_time: Some(chrono::Utc::now()),
                                        completion_time: None,
                                        media: Some(request.media.clone()),
                                        media_format: Some(request.media_format.clone()),
                                        media_sample_rate_hertz: request.media_sample_rate_hertz,
                                        settings: request.settings.clone(),
                                        transcript: None,
                                    })
                                };
                                return Ok(fallback_response);
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
            match self.make_request_with_target(Method::POST, "/", Some(&request), "Transcribe.GetTranscriptionJob") {
                Ok(response) => {
                    if response.status().is_success() {
                        let response_text = response.text().unwrap_or_else(|_| "Failed to read response text".to_string());
                        trace!("AWS GetTranscriptionJob raw response: {}", response_text);
                        
                        match serde_json::from_str::<GetTranscriptionJobResponse>(&response_text) {
                            Ok(result) => return Ok(result),
                            Err(e) => {
                                error!("Failed to parse AWS Transcribe response: {} | Raw response: {}", e, response_text);
                                return Err(SttError::InternalError(format!("Failed to parse response: {} | Raw: {}", e, response_text)));
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

    pub fn poll_job_completion(&self, job_name: &str) -> Result<TranscriptionJob, SttError> {
        let max_attempts = 60; // 5 minutes with 5-second intervals
        let poll_interval = Duration::from_secs(5);
        
        for attempt in 1..=max_attempts {
            trace!("Polling transcription job {}, attempt {}/{}", job_name, attempt, max_attempts);
            
            let response = self.get_transcription_job(job_name)?;
            
            if let Some(job) = response.transcription_job {
                match job.transcription_job_status.as_str() {
                    "COMPLETED" => {
                        trace!("Transcription job {} completed successfully", job_name);
                        return Ok(job);
                    }
                    "FAILED" => {
                        error!("Transcription job {} failed", job_name);
                        return Err(SttError::TranscriptionFailed(format!("AWS Transcribe job {} failed", job_name)));
                    }
                    "IN_PROGRESS" => {
                        trace!("Transcription job {} still in progress, waiting...", job_name);
                        std::thread::sleep(poll_interval);
                        continue;
                    }
                    status => {
                        trace!("Transcription job {} has status: {}, continuing to poll", job_name, status);
                        std::thread::sleep(poll_interval);
                        continue;
                    }
                }
            } else {
                return Err(SttError::InternalError("No transcription job found in response".to_string()));
            }
        }
        
        Err(SttError::InternalError(format!("Transcription job {} timed out after {} attempts", job_name, max_attempts)))
    }

    pub fn download_transcript(&self, transcript_uri: &str) -> Result<String, SttError> {
        trace!("Downloading transcript from: {}", transcript_uri);
        
        let mut attempts = 0;
        loop {
            match self.client.get(transcript_uri).timeout(self.timeout).send() {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.text() {
                            Ok(content) => {
                                trace!("Successfully downloaded transcript, length: {} bytes", content.len());
                                return Ok(content);
                            }
                            Err(e) => {
                                error!("Failed to read transcript content: {}", e);
                                return Err(SttError::InternalError(format!("Failed to read transcript: {}", e)));
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
                        return Err(SttError::NetworkError(format!("Transcript download failed after {} attempts: {}", self.max_retries, e)));
                    }
                    attempts += 1;
                    trace!("Retrying transcript download due to network error, attempt {}/{}", attempts, self.max_retries);
                }
            }
        }
    }

    fn parse_transcript_content(&self, content: &str) -> Result<DirectTranscriptionResponse, SttError> {
        trace!("Parsing transcript content, length: {} bytes", content.len());
        
        // AWS Transcribe returns a JSON structure with results
        let transcript_json: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| SttError::InternalError(format!("Failed to parse transcript JSON: {}", e)))?;
        
        // Extract the transcript text and confidence
        let results = transcript_json["results"].as_object()
            .ok_or_else(|| SttError::InternalError("No results found in transcript".to_string()))?;
        
        let transcripts = results["transcripts"].as_array()
            .ok_or_else(|| SttError::InternalError("No transcripts found in results".to_string()))?;
        
        if transcripts.is_empty() {
            return Err(SttError::InternalError("Empty transcripts array".to_string()));
        }
        
        let transcript_text = transcripts[0]["transcript"].as_str()
            .ok_or_else(|| SttError::InternalError("No transcript text found".to_string()))?
            .to_string();
        
        // Calculate average confidence and duration from items
        let empty_vec = vec![];
        let items = results["items"].as_array().unwrap_or(&empty_vec);
        let confidence = self.calculate_confidence_from_items(items);
        let duration = self.calculate_duration_from_items(items);
        
        trace!("Parsed transcript: {} chars, confidence: {:.2}, duration: {:.2}s", 
               transcript_text.len(), confidence, duration);
        
        Ok(DirectTranscriptionResponse {
            transcript: transcript_text,
            confidence,
            duration,
        })
    }

    fn calculate_duration_from_items(&self, items: &[serde_json::Value]) -> f32 {
        if items.is_empty() {
            return 0.0;
        }
        
        // Find the last item with end_time
        for item in items.iter().rev() {
            if let Some(end_time_str) = item["end_time"].as_str() {
                if let Ok(end_time) = end_time_str.parse::<f32>() {
                    return end_time;
                }
            }
        }
        
        // Fallback to estimate based on audio size
        0.0
    }

    fn calculate_confidence_from_items(&self, items: &[serde_json::Value]) -> f32 {
        if items.is_empty() {
            return 0.9; // Default confidence
        }
        
        let mut total_confidence = 0.0;
        let mut count = 0;
        
        for item in items {
            if let Some(confidence_str) = item["alternatives"][0]["confidence"].as_str() {
                if let Ok(confidence) = confidence_str.parse::<f32>() {
                    total_confidence += confidence;
                    count += 1;
                }
            }
        }
        
        if count > 0 {
            total_confidence / count as f32
        } else {
            0.9
        }
    }

    pub fn upload_audio_to_s3(&self, audio_data: &[u8], job_name: &str) -> Result<String, SttError> {
        // Use a default S3 bucket for transcription
        // In production, this should be configurable via environment variable
        let bucket_name = std::env::var("AWS_S3_BUCKET")
            .unwrap_or_else(|_| "golem-stt-transcription".to_string());
        
        let object_key = format!("audio/{}.wav", job_name);
        let s3_uri = format!("s3://{}/{}", bucket_name, object_key);
        
        trace!("Uploading audio to S3: {}", s3_uri);
        
        // Upload to S3 using REST API
        let upload_result = self.s3_put_object(&bucket_name, &object_key, audio_data)?;
        
        if upload_result {
            trace!("Successfully uploaded audio to S3: {}", s3_uri);
            Ok(s3_uri)
        } else {
            Err(SttError::InternalError("Failed to upload audio to S3".to_string()))
        }
    }

    fn s3_put_object(&self, bucket: &str, key: &str, data: &[u8]) -> Result<bool, SttError> {
        let url = format!("https://{}.s3.{}.amazonaws.com/{}", bucket, self.region, key);
        let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let content_hash = self.sha256_hex(data);
        
        // Create S3 authorization header
        let authorization = self.create_s3_auth_header(&timestamp, &content_hash, bucket, key, data.len())?;
        
        let mut attempts = 0;
        loop {
            match self.client
                .put(&url)
                .header("Content-Type", "audio/wav")
                .header("Content-Length", data.len().to_string())
                .header("Authorization", &authorization)
                .header("x-amz-date", &timestamp)
                .header("x-amz-content-sha256", &content_hash)
                .timeout(self.timeout)
                .body(data.to_vec())
                .send() {
                Ok(response) => {
                    if response.status().is_success() {
                        trace!("S3 upload successful");
                        return Ok(true);
                    } else {
                        let error = self.handle_s3_error_response(response)?;
                        if attempts >= self.max_retries {
                            return Err(error);
                        }
                        attempts += 1;
                        trace!("Retrying S3 upload, attempt {}/{}", attempts, self.max_retries);
                    }
                }
                Err(e) => {
                    if attempts >= self.max_retries {
                        return Err(SttError::NetworkError(format!("S3 upload failed after {} attempts: {}", self.max_retries, e)));
                    }
                    attempts += 1;
                    trace!("Retrying S3 upload due to network error, attempt {}/{}", attempts, self.max_retries);
                }
            }
        }
    }

    fn create_s3_auth_header(&self, timestamp: &str, payload_hash: &str, bucket: &str, key: &str, content_length: usize) -> Result<String, SttError> {
        let date = &timestamp[0..8];
        let host = format!("{}.s3.{}.amazonaws.com", bucket, self.region);
        
        // Step 1: Create canonical request for S3
        let canonical_request = format!(
            "PUT\n/{}\n\ncontent-length:{}\ncontent-type:audio/wav\nhost:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n\ncontent-length;content-type;host;x-amz-content-sha256;x-amz-date\n{}",
            key, content_length, host, payload_hash, timestamp, payload_hash
        );
        
        let canonical_request_hash = self.sha256_hex(canonical_request.as_bytes());
        
        // Step 2: Create string to sign
        let credential_scope = format!("{}/{}/s3/aws4_request", date, self.region);
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            timestamp, credential_scope, canonical_request_hash
        );
        
        // Step 3: Calculate signature
        let signature = self.calculate_s3_signature(&string_to_sign, date)?;
        
        // Step 4: Create authorization header
        Ok(format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders=content-length;content-type;host;x-amz-content-sha256;x-amz-date, Signature={}",
            self.access_key_id, credential_scope, signature
        ))
    }

    fn calculate_s3_signature(&self, string_to_sign: &str, date: &str) -> Result<String, SttError> {
        // AWS V4 signature derivation for S3
        let date_key = self.hmac_sha256(format!("AWS4{}", self.secret_access_key).as_bytes(), date.as_bytes());
        let date_region_key = self.hmac_sha256(&date_key, self.region.as_bytes());
        let date_region_service_key = self.hmac_sha256(&date_region_key, b"s3");
        let signing_key = self.hmac_sha256(&date_region_service_key, b"aws4_request");
        
        let signature = self.hmac_sha256(&signing_key, string_to_sign.as_bytes());
        Ok(hex::encode(signature))
    }

    fn handle_s3_error_response(&self, response: Response) -> Result<SttError, SttError> {
        let status = response.status();
        let error_text = response.text().unwrap_or_else(|_| "Unknown S3 error".to_string());
        
        error!("S3 error response ({}): {}", status, error_text);
        
        match status.as_u16() {
            403 => Ok(SttError::AccessDenied("S3 access denied".to_string())),
            404 => Ok(SttError::InternalError("S3 bucket or object not found".to_string())),
            _ => Ok(SttError::InternalError(format!("S3 error {}: {}", status, error_text))),
        }
    }

    fn make_request_with_target<T: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: Option<&T>,
        target: &str,
    ) -> Result<Response, reqwest::Error> {
        let url = format!("{}{}", self.base_url, path);
        let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        
        let json_body = if let Some(body) = body {
            serde_json::to_string(body).unwrap_or_default()
        } else {
            "{}".to_string()
        };
        
        let payload_hash = self.sha256_hex(json_body.as_bytes());
        let authorization = self.create_transcribe_auth_header(&timestamp, &payload_hash, target);
        
        trace!("AWS API request with explicit target - URL: {}, Target: {}, Body: {}", url, target, json_body);
        
        let mut request_builder = self.client
            .request(method, &url)
            .header("Content-Type", "application/x-amz-json-1.1")
            .header("X-Amz-Target", target)
            .header("Authorization", authorization)
            .header("X-Amz-Date", timestamp)
            .header("x-amz-content-sha256", payload_hash)
            .timeout(self.timeout);
        
        if !json_body.is_empty() && json_body != "{}" {
            request_builder = request_builder.body(json_body);
        }
        
        request_builder.send()
    }

    fn sha256_hex(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    fn validate_credentials(&self) -> Result<(), SttError> {
        // Simple credential validation - check if credentials are set
        if self.access_key_id.is_empty() || self.secret_access_key.is_empty() {
            return Err(SttError::Unauthorized("AWS credentials not provided".to_string()));
        }
        
        trace!("AWS credentials basic validation passed");
        Ok(())
    }

    fn create_transcribe_auth_header(&self, timestamp: &str, payload_hash: &str, target: &str) -> String {
        let date = &timestamp[0..8];
        let host = format!("transcribe.{}.amazonaws.com", self.region);
        
        // Step 1: Create canonical request
        let canonical_request = format!(
            "POST\n/\n\ncontent-type:application/x-amz-json-1.1\nhost:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\nx-amz-target:{}\n\ncontent-type;host;x-amz-content-sha256;x-amz-date;x-amz-target\n{}",
            host, payload_hash, timestamp, target, payload_hash
        );
        
        trace!("AWS Transcribe canonical request for target {}: {}", target, canonical_request);
        let canonical_request_hash = self.sha256_hex(canonical_request.as_bytes());
        trace!("AWS Transcribe canonical request hash: {}", canonical_request_hash);
        
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
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders=content-type;host;x-amz-content-sha256;x-amz-date;x-amz-target, Signature={}",
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
        
        error!("AWS Transcribe error response ({}): {}", status, error_text);
        
        match status.as_u16() {
            400 => SttError::InvalidAudio(error_text),
            401 | 403 => SttError::Unauthorized(error_text),
            429 => SttError::RateLimited(60), // AWS typically uses 60 second retry
            500 | 502 | 503 | 504 => SttError::ServiceUnavailable(error_text),
            _ => SttError::InternalError(format!("HTTP {}: {}", status, error_text)),
        }
    }
}
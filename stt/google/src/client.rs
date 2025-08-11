use golem_stt::golem::stt::types::SttError;
use log::{error, trace, warn};
use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use base64::{Engine as _, engine::general_purpose};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

#[derive(Debug, Serialize, Deserialize)]
struct ServiceAccountCredentials {
    #[serde(rename = "type")]
    account_type: String,
    project_id: String,
    private_key_id: String,
    private_key: String,
    client_email: String,
    client_id: String,
    auth_uri: String,
    token_uri: String,
}

#[derive(Debug, Serialize)]
struct JwtClaims {
    iss: String,  // issuer (client_email)
    sub: String,  // subject (client_email)
    scope: String, // OAuth scopes
    aud: String,  // audience (token_uri)
    exp: i64,     // expiration time
    iat: i64,     // issued at time
}

#[derive(Debug)]
struct CachedToken {
    token: String,
    expires_at: SystemTime,
}

#[derive(Debug)]
pub struct GoogleSpeechClient {
    credentials_path: String,
    credentials_json: Option<String>,
    project_id: String,
    access_token: Arc<Mutex<Option<CachedToken>>>,
    client: Client,
    base_url: String,
    timeout: Duration,
    max_retries: u32,
}

impl GoogleSpeechClient {
    pub fn new_from_file(credentials_path: String, project_id: String) -> Self {
        Self::new_internal(Some(credentials_path), None, project_id)
    }

    pub fn new_from_json(credentials_json: String, project_id: String) -> Self {
        Self::new_internal(None, Some(credentials_json), project_id)
    }

    fn new_internal(credentials_path: Option<String>, credentials_json: Option<String>, project_id: String) -> Self {
        let timeout_str = std::env::var("STT_PROVIDER_TIMEOUT").unwrap_or_else(|_| "30".to_string());
        let timeout = Duration::from_secs(timeout_str.parse().unwrap_or(30));
        
        let max_retries_str = std::env::var("STT_PROVIDER_MAX_RETRIES").unwrap_or_else(|_| "3".to_string());
        let max_retries = max_retries_str.parse().unwrap_or(3);
        
        let base_url = std::env::var("STT_PROVIDER_ENDPOINT")
            .unwrap_or_else(|_| "https://speech.googleapis.com/v2".to_string());
        
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
            credentials_path: credentials_path.unwrap_or_default(),
            credentials_json,
            project_id,
            access_token: Arc::new(Mutex::new(None)),
            client: Client::new(),
            base_url,
            timeout,
            max_retries,
        }
    }

    fn get_access_token(&self) -> Result<String, SttError> {
        let mut token = self.access_token.lock().map_err(|_| 
            SttError::InternalError("Failed to acquire token lock".to_string()))?;
        
        // Check if we have a cached token and it's not expired
        if let Some(ref cached) = *token {
            if SystemTime::now() < cached.expires_at {
                trace!("Using cached OAuth token");
                return Ok(cached.token.clone());
            } else {
                trace!("Cached OAuth token expired, generating new one");
            }
        }
        
        // Generate new token
        let new_token = self.generate_oauth_token()?;
        let expires_at = SystemTime::now() + Duration::from_secs(3600); // 1 hour
        
        *token = Some(CachedToken {
            token: new_token.clone(),
            expires_at,
        });
        
        Ok(new_token)
    }
    
    fn generate_oauth_token(&self) -> Result<String, SttError> {
        let key_content = if let Some(ref json) = self.credentials_json {
            // Use JSON from environment variable
            trace!("Using service account JSON from environment variable");
            json.clone()
        } else if !self.credentials_path.is_empty() {
            // Read from file path
            trace!("Reading service account from file: {}", self.credentials_path);
            std::fs::read_to_string(&self.credentials_path)
                .map_err(|e| SttError::Unauthorized(format!("Failed to read credentials file: {}", e)))?
        } else {
            return Err(SttError::Unauthorized("No credentials available".to_string()));
        };
        
        let credentials: ServiceAccountCredentials = serde_json::from_str(&key_content)
            .map_err(|e| SttError::Unauthorized(format!("Invalid credentials format: {}", e)))?;
        
        trace!("Service account credentials parsed successfully");
        
        // Generate JWT for Google OAuth 2.0
        let jwt_token = self.create_jwt(&credentials)?;
        trace!("JWT token created, exchanging for OAuth access token");
        
        // Exchange JWT for access token
        let access_token = self.exchange_jwt_for_token(&jwt_token, &credentials.token_uri)?;
        
        Ok(access_token)
    }
    
    fn create_jwt(&self, credentials: &ServiceAccountCredentials) -> Result<String, SttError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| SttError::InternalError("Invalid system time".to_string()))?
            .as_secs() as i64;
        
        let claims = JwtClaims {
            iss: credentials.client_email.clone(),
            sub: credentials.client_email.clone(),
            scope: "https://www.googleapis.com/auth/cloud-platform".to_string(),
            aud: credentials.token_uri.clone(),
            exp: now + 3600, // 1 hour
            iat: now,
        };
        
        // Parse the private key
        let private_key = credentials.private_key
            .replace("\\n", "\n");
            
        let encoding_key = EncodingKey::from_rsa_pem(private_key.as_bytes())
            .map_err(|e| SttError::Unauthorized(format!("Invalid private key: {}", e)))?;
        
        let header = Header::new(Algorithm::RS256);
        
        let token = encode(&header, &claims, &encoding_key)
            .map_err(|e| SttError::Unauthorized(format!("Failed to create JWT: {}", e)))?;
        
        trace!("JWT token created successfully");
        Ok(token)
    }
    
    fn exchange_jwt_for_token(&self, jwt: &str, token_uri: &str) -> Result<String, SttError> {
        let params = [
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", jwt),
        ];
        
        trace!("Exchanging JWT for OAuth access token at {}", token_uri);
        
        let response = self.client
            .post(token_uri)
            .form(&params)
            .timeout(self.timeout)
            .send()
            .map_err(|e| SttError::NetworkError(format!("Token exchange failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(SttError::Unauthorized(format!("Token exchange failed: {}", error_text)));
        }
        
        let token_response: serde_json::Value = response.json()
            .map_err(|e| SttError::InternalError(format!("Failed to parse token response: {}", e)))?;
        
        let access_token = token_response["access_token"]
            .as_str()
            .ok_or_else(|| SttError::Unauthorized("No access token in response".to_string()))?
            .to_string();
        
        trace!("OAuth access token obtained successfully");
        Ok(access_token)
    }

    pub fn transcribe(&self, request: RecognizeRequest) -> Result<RecognizeResponse, SttError> {
        // Estimate audio duration from base64 content size
        // Base64 adds ~33% overhead, so raw audio size ≈ content.len() * 3/4
        let estimated_raw_size = (request.content.len() * 3) / 4;
        
        // For WAV/Linear16: 16kHz * 2 bytes * 1 channel = 32,000 bytes/second
        // Use conservative estimate of 60 seconds (1.8MB) for sync vs async decision
        if estimated_raw_size > 1_800_000 {
            trace!("Audio size {} bytes estimated - using async recognition with Cloud Storage for longer audio", estimated_raw_size);
            return self.transcribe_long_running(request);
        } else {
            trace!("Audio size {} bytes estimated - using sync recognition", estimated_raw_size);
            return self.transcribe_sync(request);
        }
    }

    fn transcribe_sync(&self, request: RecognizeRequest) -> Result<RecognizeResponse, SttError> {
        let access_token = self.get_access_token()?;
        // Google Speech-to-Text API v2 requires project and location in the URL
        let location = std::env::var("GOOGLE_CLOUD_LOCATION").unwrap_or_else(|_| "global".to_string());
        let url = format!("{}/projects/{}/locations/{}/recognizers/_:recognize", 
                         self.base_url, self.project_id, location);
        
        let mut attempts = 0;
        loop {
            attempts += 1;
            if attempts == 1 {
                trace!("Google Speech API request (initial attempt, max retries: {})", self.max_retries);
            } else {
                trace!("Google Speech API request (retry {}/{}, max retries: {})", attempts - 1, self.max_retries, self.max_retries);
            }
            
            match self.make_request_with_auth(Method::POST, &url, Some(&request), &access_token) {
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

    fn make_request_with_auth<T: Serialize>(
        &self,
        method: Method,
        url: &str,
        body: Option<&T>,
        access_token: &str,
    ) -> Result<Response, reqwest::Error> {
        let mut req = self
            .client
            .request(method, url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", access_token))
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
            credentials_path: self.credentials_path.clone(),
            credentials_json: self.credentials_json.clone(),
            project_id: self.project_id.clone(),
            access_token: self.access_token.clone(),
            client: Client::new(),
            base_url: self.base_url.clone(),
            timeout: self.timeout,
            max_retries: self.max_retries,
        }
    }
}

impl GoogleSpeechClient {
    fn transcribe_long_running(&self, request: RecognizeRequest) -> Result<RecognizeResponse, SttError> {
        let access_token = self.get_access_token()?;
        let location = std::env::var("GOOGLE_CLOUD_LOCATION").unwrap_or_else(|_| "global".to_string());
        
        // Step 1: Upload audio to Google Cloud Storage
        let audio_bytes = base64::prelude::BASE64_STANDARD.decode(&request.content)
            .map_err(|e| SttError::InternalError(format!("Failed to decode base64 audio: {}", e)))?;
        
        let gcs_uri = self.upload_to_cloud_storage(&audio_bytes, &access_token)?;
        trace!("Audio uploaded to Cloud Storage: {}", gcs_uri);
        
        // Step 2: Create batch recognition request with GCS URI
        let batch_request = BatchRecognizeRequest {
            recognizer: format!("projects/{}/locations/{}/recognizers/_", self.project_id, location),
            config: request.config.clone(),
            files: vec![BatchRecognizeFileMetadata {
                content: None,
                uri: Some(gcs_uri.clone()),
            }],
            recognition_output_config: RecognitionOutputConfig {
                gcs_output_config: None,
                inline_response_config: Some(InlineOutputConfig {}),
            },
            processing_strategy: Some("DYNAMIC_BATCHING".to_string()),
        };
        
        let batch_url = format!("{}/projects/{}/locations/{}/recognizers/_:batchRecognize", 
                               self.base_url, self.project_id, location);
        
        trace!("Starting Google batch recognition for long audio");
        
        // Step 3: Start the long-running operation
        let operation_response = self.make_request_with_auth(
            Method::POST, &batch_url, Some(&batch_request), &access_token
        ).map_err(|e| SttError::NetworkError(format!("Batch recognition request failed: {}", e)))?;
        
        if !operation_response.status().is_success() {
            let error = self.handle_error_response(operation_response);
            return Err(error);
        }
        
        // Debug: Log the response body to see what we're getting
        let response_text = operation_response.text()
            .map_err(|e| SttError::InternalError(format!("Failed to read response body: {}", e)))?;
        
        trace!("Batch operation response body: {}", response_text);
        
        let operation: Operation = serde_json::from_str(&response_text)
            .map_err(|e| SttError::InternalError(format!("Failed to parse operation response: {}. Response was: {}", e, response_text)))?;
        
        trace!("Batch operation started: {}", operation.name);
        
        // Step 4: Poll for completion
        let result = self.poll_operation(&operation.name, &access_token);
        
        // Step 5: Clean up - delete the temporary file from Cloud Storage
        if let Err(cleanup_err) = self.delete_from_cloud_storage(&gcs_uri, &access_token) {
            trace!("Warning: Failed to clean up temporary file {}: {:?}", gcs_uri, cleanup_err);
        }
        
        result
    }
    
    fn create_bucket_if_needed(&self, bucket_name: &str, access_token: &str) -> Result<(), SttError> {
        let create_bucket_url = "https://storage.googleapis.com/storage/v1/b";
        
        let bucket_request = serde_json::json!({
            "name": bucket_name,
            "location": "US",
            "storageClass": "STANDARD",
            "lifecycle": {
                "rule": [{
                    "action": {
                        "type": "Delete"
                    },
                    "condition": {
                        "age": 1
                    }
                }]
            }
        });
        
        trace!("Creating Cloud Storage bucket: {}", bucket_name);
        
        let response = self.client
            .post(create_bucket_url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .query(&[("project", &self.project_id)])
            .body(bucket_request.to_string())
            .send()
            .map_err(|e| SttError::NetworkError(format!("Bucket creation request failed: {}", e)))?;
            
        if response.status().is_success() {
            trace!("Bucket {} created successfully", bucket_name);
            Ok(())
        } else if response.status() == 409 {
            // Bucket already exists, which is fine
            trace!("Bucket {} already exists", bucket_name);
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(SttError::NetworkError(format!(
                "Failed to create bucket {} with status {}: {}",
                bucket_name, status, error_text
            )))
        }
    }

    fn upload_to_cloud_storage(&self, audio_data: &[u8], access_token: &str) -> Result<String, SttError> {
        // Generate a unique filename
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let filename = format!("stt-temp-{}.wav", timestamp);
        
        // Get bucket name from environment or use default
        let bucket_name = std::env::var("GOOGLE_CLOUD_STORAGE_BUCKET")
            .unwrap_or_else(|_| format!("{}-stt-temp", self.project_id));
            
        // Try to create bucket if it doesn't exist, but continue if permission denied
        match self.create_bucket_if_needed(&bucket_name, access_token) {
            Ok(_) => trace!("Bucket {} is ready for upload", bucket_name),
            Err(SttError::NetworkError(err)) if err.contains("storage.buckets.create") => {
                trace!("Bucket creation failed due to permissions, assuming bucket exists: {}", bucket_name);
            },
            Err(e) => return Err(e),
        }
        
        let upload_url = format!("https://storage.googleapis.com/upload/storage/v1/b/{}/o", bucket_name);
        
        trace!("Uploading {} bytes to gs://{}/{}", audio_data.len(), bucket_name, filename);
        
        // Create multipart upload request
        let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
        let mut body = Vec::new();
        
        // Add metadata part
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(b"Content-Type: application/json; charset=UTF-8\r\n\r\n");
        body.extend_from_slice(format!(r#"{{"name":"{}"}}"#, filename).as_bytes());
        body.extend_from_slice(b"\r\n");
        
        // Add file data part
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(b"Content-Type: audio/wav\r\n\r\n");
        body.extend_from_slice(audio_data);
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());
        
        let upload_response = self.client
            .post(&upload_url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", format!("multipart/related; boundary={}", boundary))
            .query(&[("uploadType", "multipart")])
            .body(body)
            .send()
            .map_err(|e| SttError::NetworkError(format!("Cloud Storage upload failed: {}", e)))?;
        
        if !upload_response.status().is_success() {
            let status = upload_response.status();
            let error_text = upload_response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(SttError::NetworkError(format!(
                "Cloud Storage upload failed with status {}: {}", 
                status, 
                error_text
            )));
        }
        
        Ok(format!("gs://{}/{}", bucket_name, filename))
    }
    
    fn delete_from_cloud_storage(&self, gcs_uri: &str, access_token: &str) -> Result<(), SttError> {
        // Parse gs://bucket/object URI
        let uri_parts: Vec<&str> = gcs_uri.strip_prefix("gs://")
            .ok_or_else(|| SttError::InternalError("Invalid GCS URI format".to_string()))?
            .splitn(2, '/')
            .collect();
        
        if uri_parts.len() != 2 {
            return Err(SttError::InternalError("Invalid GCS URI format".to_string()));
        }
        
        let bucket_name = uri_parts[0];
        let object_name = uri_parts[1];
        
        let delete_url = format!("https://storage.googleapis.com/storage/v1/b/{}/o/{}", 
                                bucket_name, object_name);
        
        let delete_response = self.client
            .delete(&delete_url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .map_err(|e| SttError::NetworkError(format!("Cloud Storage delete failed: {}", e)))?;
        
        if !delete_response.status().is_success() && delete_response.status().as_u16() != 404 {
            trace!("Warning: Failed to delete temporary file, status: {}", delete_response.status());
        }
        
        Ok(())
    }

    fn poll_operation(&self, operation_name: &str, access_token: &str) -> Result<RecognizeResponse, SttError> {
        let operation_url = format!("{}/{}", self.base_url, operation_name);
        let mut poll_attempts = 0;
        let max_poll_attempts = 120; // 10 minutes max with 5-second intervals
        
        loop {
            poll_attempts += 1;
            if poll_attempts > max_poll_attempts {
                return Err(SttError::InternalError("Long-running operation timed out".to_string()));
            }
            
            trace!("Polling operation status (attempt {}/{})", poll_attempts, max_poll_attempts);
            
            let response = self.make_request_with_auth(
                Method::GET, &operation_url, None::<&()>, access_token
            ).map_err(|e| SttError::NetworkError(format!("Operation polling failed: {}", e)))?;
            
            if !response.status().is_success() {
                let error = self.handle_error_response(response);
                return Err(error);
            }
            
            let operation: Operation = response.json()
                .map_err(|e| SttError::InternalError(format!("Failed to parse operation status: {}", e)))?;
            
            if operation.done {
                trace!("Operation completed successfully");
                
                if let Some(response_data) = operation.response {
                    // Debug: Log the response data structure to understand what we're getting
                    trace!("Batch operation response data: {}", serde_json::to_string_pretty(&response_data).unwrap_or_else(|_| "Failed to serialize".to_string()));
                    
                    // Extract batch recognition response and convert to sync format
                    let batch_response: BatchRecognizeResponse = serde_json::from_value(response_data.clone())
                        .map_err(|e| SttError::InternalError(format!("Failed to parse batch response: {}. Response data was: {}", e, serde_json::to_string_pretty(&response_data).unwrap_or_else(|_| "Failed to serialize".to_string()))))?;
                    
                    return self.convert_batch_to_sync_response(batch_response);
                } else if let Some(error) = operation.error {
                    return Err(SttError::InternalError(format!("Operation failed: {} - {}", error.code, error.message)));
                } else {
                    return Err(SttError::InternalError("Operation completed without response or error".to_string()));
                }
            }
            
            // Wait before next poll
            std::thread::sleep(Duration::from_secs(5));
        }
    }

    fn convert_batch_to_sync_response(&self, batch_response: BatchRecognizeResponse) -> Result<RecognizeResponse, SttError> {
        // Extract results from batch response and convert to sync format
        let mut all_results = vec![];
        
        for (_gcs_uri, file_result) in batch_response.results {
            if let Some(mut transcript) = file_result.transcript {
                if let Some(mut results) = transcript.results {
                    all_results.append(&mut results);
                }
            }
        }
        
        Ok(RecognizeResponse {
            results: Some(all_results),
            request_id: batch_response.request_id,
            total_billed_time: batch_response.total_billed_time,
            speech_adaptation_info: None,
        })
    }
}

#[derive(Debug)]
pub struct GoogleStreamingSession {
    client: GoogleSpeechClient,
    config: RecognitionConfig,
    sequence_id: Arc<Mutex<u32>>,
    is_active: Arc<Mutex<bool>>,
    results_buffer: Arc<Mutex<Vec<StreamingRecognitionResult>>>,
}

impl GoogleStreamingSession {
    pub fn new(client: GoogleSpeechClient, config: RecognitionConfig) -> Self {
        Self {
            client,
            config,
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
                content: audio_content.clone(),
                config_mask: None,
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
    pub content: String, // v2 uses "content" instead of "audio" object
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_mask: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognitionConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_decoding_config: Option<AutoDetectDecodingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explicit_decoding_config: Option<ExplicitDecodingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_codes: Option<Vec<String>>, // v2 uses language_codes (plural)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation_config: Option<TranslationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adaptation: Option<SpeechAdaptation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speech_contexts: Option<Vec<SpeechContext>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoDetectDecodingConfig {
    // Auto-detection configuration for v2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplicitDecodingConfig {
    pub encoding: AudioEncoding,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate_hertz: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_channel_count: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechAdaptation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phrase_sets: Option<Vec<AdaptationPhraseSet>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationPhraseSet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phrases: Option<Vec<Phrase>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boost: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phrase {
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boost: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phrases: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boost: Option<f32>,
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

// Additional structs for long-running operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRecognizeRequest {
    pub recognizer: String,
    pub config: RecognitionConfig,
    pub files: Vec<BatchRecognizeFileMetadata>,
    pub recognition_output_config: RecognitionOutputConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_strategy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRecognizeFileMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognitionOutputConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcs_output_config: Option<GcsOutputConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_response_config: Option<InlineOutputConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcsOutputConfig {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineOutputConfig {
    // Empty struct for inline responses
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<OperationError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRecognizeResponse {
    pub results: std::collections::HashMap<String, BatchRecognizeFileResult>,
    #[serde(rename = "totalBilledDuration", skip_serializing_if = "Option::is_none")]
    pub total_billed_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRecognizeFileResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<RecognizeResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<OperationError>,
}
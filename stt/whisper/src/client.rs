use golem_stt::golem::stt::types::SttError;
use log::{error, trace, warn};
use reqwest::{Client, Response};
use serde::{Deserialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug)]
pub struct WhisperClient {
    api_key: String,
    client: Client,
    base_url: String,
    timeout: Duration,
    max_retries: u32,
}

impl WhisperClient {
    pub fn new(api_key: String) -> Self {
        let timeout_str = std::env::var("STT_PROVIDER_TIMEOUT").unwrap_or_else(|_| "60".to_string()); // Whisper can be slower
        let timeout = Duration::from_secs(timeout_str.parse().unwrap_or(60));
        
        let max_retries_str = std::env::var("STT_PROVIDER_MAX_RETRIES").unwrap_or_else(|_| "3".to_string());
        let max_retries = max_retries_str.parse().unwrap_or(3);
        
        let base_url = std::env::var("STT_PROVIDER_ENDPOINT")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

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

    fn should_retry(&self, error: &SttError) -> bool {
        match error {
            SttError::RateLimited(_) | SttError::ServiceUnavailable(_) => true,
            _ => false,
        }
    }

    pub fn transcribe_audio(&self, request: WhisperTranscriptionRequest) -> Result<WhisperTranscriptionResponse, SttError> {
        let url = format!("{}/audio/transcriptions", self.base_url);
        
        let mut attempts = 0;
        loop {
            attempts += 1;
            if attempts == 1 {
                trace!("OpenAI Whisper API request (initial attempt, max retries: {})", self.max_retries);
            } else {
                trace!("OpenAI Whisper API request (retry {}/{}, max retries: {})", attempts - 1, self.max_retries, self.max_retries);
            }
            // Manually construct multipart form data since reqwest multipart doesn't work in WASM
            let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
            let mut body = Vec::new();
            
            // Add file field
            body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
            body.extend_from_slice(b"Content-Disposition: form-data; name=\"file\"; filename=\"audio.wav\"\r\n");
            body.extend_from_slice(b"Content-Type: audio/wav\r\n\r\n");
            body.extend_from_slice(&request.audio);
            body.extend_from_slice(b"\r\n");
            
            // Add model field
            body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
            body.extend_from_slice(b"Content-Disposition: form-data; name=\"model\"\r\n\r\n");
            body.extend_from_slice(request.model.as_bytes());
            body.extend_from_slice(b"\r\n");
            
            // Add response_format field
            let default_format = "json".to_string();
            let response_format = request.response_format.as_ref().unwrap_or(&default_format);
            body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
            body.extend_from_slice(b"Content-Disposition: form-data; name=\"response_format\"\r\n\r\n");
            body.extend_from_slice(response_format.as_bytes());
            body.extend_from_slice(b"\r\n");
            
            // Add optional fields
            if let Some(language) = &request.language {
                body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
                body.extend_from_slice(b"Content-Disposition: form-data; name=\"language\"\r\n\r\n");
                body.extend_from_slice(language.as_bytes());
                body.extend_from_slice(b"\r\n");
            }
            
            if let Some(prompt) = &request.prompt {
                body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
                body.extend_from_slice(b"Content-Disposition: form-data; name=\"prompt\"\r\n\r\n");
                body.extend_from_slice(prompt.as_bytes());
                body.extend_from_slice(b"\r\n");
            }
            
            if let Some(temperature) = request.temperature {
                body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
                body.extend_from_slice(b"Content-Disposition: form-data; name=\"temperature\"\r\n\r\n");
                body.extend_from_slice(temperature.to_string().as_bytes());
                body.extend_from_slice(b"\r\n");
            }
            
            if let Some(timestamp_granularities) = &request.timestamp_granularities {
                for granularity in timestamp_granularities {
                    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
                    body.extend_from_slice(b"Content-Disposition: form-data; name=\"timestamp_granularities[]\"\r\n\r\n");
                    body.extend_from_slice(granularity.as_bytes());
                    body.extend_from_slice(b"\r\n");
                }
            }
            
            // Close the form
            body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

            let req = self.client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", format!("multipart/form-data; boundary={}", boundary))
                .body(body)
                .timeout(self.timeout);

            match req.send() {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<WhisperTranscriptionResponse>() {
                            Ok(result) => return Ok(result),
                            Err(e) => {
                                error!("Failed to parse OpenAI Whisper response: {}", e);
                                return Err(SttError::InternalError(format!("Failed to parse response: {}", e)));
                            }
                        }
                    } else {
                        let error = self.handle_error_response(response);
                        if self.should_retry(&error) && attempts <= self.max_retries {
                            trace!("Will retry OpenAI Whisper request (retry {}/{})", attempts, self.max_retries);
                            std::thread::sleep(std::time::Duration::from_secs(1));
                            continue;
                        } else {
                            return Err(error);
                        }
                    }
                }
                Err(e) => {
                    if attempts <= self.max_retries {
                        trace!("Will retry OpenAI Whisper request due to network error (retry {}/{})", attempts, self.max_retries);
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        continue;
                    } else {
                        return Err(SttError::NetworkError(format!("Request failed after {} attempts: {}", self.max_retries + 1, e)));
                    }
                }
            }
        }
    }

    fn handle_error_response(&self, response: Response) -> SttError {
        let status = response.status();
        let error_text = response.text().unwrap_or_else(|_| "Unknown error".to_string());
        
        trace!("OpenAI Whisper API error response: {} - {}", status, error_text);

        match status.as_u16() {
            400 => SttError::InvalidAudio(error_text),
            401 => SttError::Unauthorized(error_text),
            403 => SttError::AccessDenied(error_text),
            429 => SttError::RateLimited(60), // Default retry after 60 seconds
            500..=599 => SttError::ServiceUnavailable(error_text),
            _ => SttError::InternalError(format!("HTTP {}: {}", status, error_text)),
        }
    }

    pub fn start_streaming_session(&self, request_template: WhisperTranscriptionRequest) -> Result<WhisperStreamingSession, SttError> {
        trace!("Starting OpenAI Whisper pseudo-streaming session");
        warn!("OpenAI Whisper does not support real-time streaming. Using chunked buffering approach.");
        Ok(WhisperStreamingSession::new(self.clone(), request_template))
    }
}

impl Clone for WhisperClient {
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
pub struct WhisperStreamingSession {
    client: WhisperClient,
    request_template: WhisperTranscriptionRequest,
    audio_buffer: Arc<Mutex<Vec<u8>>>,
    is_active: Arc<Mutex<bool>>,
    chunk_size: usize,
}

impl WhisperStreamingSession {
    pub fn new(client: WhisperClient, request_template: WhisperTranscriptionRequest) -> Self {
        // Use a reasonable chunk size for pseudo-streaming (30 seconds worth of audio at 16kHz)
        let chunk_size = 16000 * 2 * 30; // 16kHz, 16-bit, 30 seconds
        
        Self {
            client,
            request_template,
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            is_active: Arc::new(Mutex::new(true)),
            chunk_size,
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
        trace!("Added {} bytes to Whisper streaming buffer, total: {}", chunk.len(), buffer.len());
        
        // Note: For true streaming, we'd process chunks here, but Whisper needs complete files
        Ok(())
    }

    pub fn finish_and_get_result(&self) -> Result<WhisperTranscriptionResponse, SttError> {
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

        trace!("Finishing Whisper streaming session with {} bytes of audio", buffer.len());

        // Create final request with accumulated audio
        let mut request = self.request_template.clone();
        request.audio = buffer.clone();

        self.client.transcribe_audio(request)
    }

    pub fn close(&self) {
        if let Ok(mut is_active) = self.is_active.lock() {
            *is_active = false;
        }
        trace!("Whisper streaming session closed");
    }
}

// OpenAI Whisper transcription request
#[derive(Debug, Clone)]
pub struct WhisperTranscriptionRequest {
    pub audio: Vec<u8>,
    pub model: String,
    pub language: Option<String>,
    pub prompt: Option<String>,
    pub response_format: Option<String>,
    pub temperature: Option<f32>,
    pub timestamp_granularities: Option<Vec<String>>,
}

// OpenAI Whisper transcription response
#[derive(Debug, Clone, Deserialize)]
pub struct WhisperTranscriptionResponse {
    pub text: String,
    pub language: Option<String>,
    pub duration: Option<f32>,
    pub words: Option<Vec<WhisperWord>>,
    pub segments: Option<Vec<WhisperSegment>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WhisperWord {
    pub word: String,
    pub start: f32,
    pub end: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WhisperSegment {
    pub text: String,
    pub no_speech_prob: f32,
    pub words: Option<Vec<WhisperWord>>,
}

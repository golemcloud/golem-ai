use golem_stt::golem::stt::types::SttError;
use log::{error, trace, warn};
use reqwest::{Client, Response};
use serde::{Deserialize};
use std::time::Duration;
use base64::prelude::*;

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

        Self {
            api_key,
            client: Client::new(),
            base_url,
            timeout,
            max_retries,
        }
    }

    pub fn transcribe_audio(&self, request: WhisperTranscriptionRequest) -> Result<WhisperTranscriptionResponse, SttError> {
        let url = format!("{}/audio/transcriptions", self.base_url);
        
        // Try alternative approach: use base64 encoding with JSON body
        // This avoids multipart form issues in WASM environments
        let audio_base64 = base64::prelude::BASE64_STANDARD.encode(&request.audio);
        
        let mut body = serde_json::json!({
            "file": audio_base64,
            "model": request.model,
            "response_format": request.response_format.unwrap_or("json".to_string())
        });
        
        if let Some(language) = &request.language {
            body["language"] = serde_json::Value::String(language.clone());
        }
        
        if let Some(prompt) = &request.prompt {
            body["prompt"] = serde_json::Value::String(prompt.clone());
        }
        
        if let Some(temperature) = request.temperature {
            body["temperature"] = serde_json::Value::Number(serde_json::Number::from_f64(temperature as f64).unwrap());
        }
        
        if let Some(timestamp_granularities) = &request.timestamp_granularities {
            body["timestamp_granularities"] = serde_json::Value::Array(
                timestamp_granularities.iter().map(|s| serde_json::Value::String(s.clone())).collect()
            );
        }

        let mut attempts = 0;
        loop {
            let req = self.client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
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
    pub task: Option<String>,
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
    pub id: u32,
    pub seek: u32,
    pub start: f32,
    pub end: f32,
    pub text: String,
    pub tokens: Vec<u32>,
    pub temperature: f32,
    pub avg_logprob: f32,
    pub compression_ratio: f32,
    pub no_speech_prob: f32,
    pub words: Option<Vec<WhisperWord>>,
}

// OpenAI Whisper error response
#[derive(Debug, Clone, Deserialize)]
pub struct WhisperErrorResponse {
    pub error: WhisperError,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WhisperError {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
    pub param: Option<String>,
    pub code: Option<String>,
}

// OpenAI Whisper translation request (optional feature)
#[derive(Debug, Clone)]
pub struct WhisperTranslationRequest {
    pub audio: Vec<u8>,
    pub model: String,
    pub prompt: Option<String>,
    pub response_format: Option<String>,
    pub temperature: Option<f32>,
}

// OpenAI Whisper translation response
#[derive(Debug, Clone, Deserialize)]
pub struct WhisperTranslationResponse {
    pub text: String,
}
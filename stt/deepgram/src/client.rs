use golem_stt::golem::stt::types::SttError;
use log::{error, trace};
use reqwest::{Client, Response};
use serde::Deserialize;
use std::time::Duration;

pub struct DeepgramClient {
    api_key: String,
    client: Client,
    base_url: String,
    timeout: Duration,
    max_retries: u32,
}

impl DeepgramClient {
    pub fn new(api_key: String) -> Self {
        let timeout_str = std::env::var("STT_PROVIDER_TIMEOUT").unwrap_or_else(|_| "30".to_string());
        let timeout = Duration::from_secs(timeout_str.parse().unwrap_or(30));
        
        let max_retries_str = std::env::var("STT_PROVIDER_MAX_RETRIES").unwrap_or_else(|_| "3".to_string());
        let max_retries = max_retries_str.parse().unwrap_or(3);
        
        let base_url = std::env::var("STT_PROVIDER_ENDPOINT")
            .unwrap_or_else(|_| "https://api.deepgram.com/v1".to_string());
        
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

    pub fn transcribe_prerecorded(&self, request: PrerecordedTranscriptionRequest) -> Result<DeepgramTranscriptionResponse, SttError> {
        let url = format!("{}/listen", self.base_url);
        
        let mut attempts = 0;
        loop {
            attempts += 1;
            if attempts == 1 {
                trace!("Deepgram API request (initial attempt, max retries: {})", self.max_retries);
            } else {
                trace!("Deepgram API request (retry {}/{}, max retries: {})", attempts - 1, self.max_retries, self.max_retries);
            }
            
            let mut req = self.client
                .post(&url)
                .header("Authorization", format!("Token {}", self.api_key))
                .header("Content-Type", "audio/wav")
                .timeout(self.timeout);

            // Add query parameters
            if let Some(language) = &request.language {
                req = req.query(&[("language", language)]);
            }
            if let Some(model) = &request.model {
                req = req.query(&[("model", model)]);
            }
            if request.punctuate {
                req = req.query(&[("punctuate", "true")]);
            }
            if request.diarize {
                req = req.query(&[("diarize", "true")]);
            }
            if request.smart_format {
                req = req.query(&[("smart_format", "true")]);
            }
            if request.utterances {
                req = req.query(&[("utterances", "true")]);
            }
            if let Some(keywords) = &request.keywords {
                for keyword in keywords {
                    req = req.query(&[("keywords", keyword)]);
                }
            }

            // Send the audio data in the request body
            req = req.body(request.audio.clone());

            match req.send() {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<DeepgramTranscriptionResponse>() {
                            Ok(result) => return Ok(result),
                            Err(e) => {
                                error!("Failed to parse Deepgram response: {}", e);
                                return Err(SttError::InternalError(format!("Failed to parse response: {}", e)));
                            }
                        }
                    } else {
                        let error = self.handle_error_response(response);
                        if self.should_retry(&error) && attempts <= self.max_retries {
                            trace!("Will retry Deepgram request (retry {}/{})", attempts, self.max_retries);
                            std::thread::sleep(std::time::Duration::from_secs(1));
                            continue;
                        } else {
                            return Err(error);
                        }
                    }
                }
                Err(e) => {
                    if attempts <= self.max_retries {
                        trace!("Will retry Deepgram request due to network error (retry {}/{})", attempts, self.max_retries);
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        continue;
                    } else {
                        return Err(SttError::NetworkError(format!("Request failed after {} attempts: {}", self.max_retries + 1, e)));
                    }
                }
            }
        }
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
        
        trace!("Deepgram API error response: {} - {}", status, error_text);

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

// Prerecorded transcription request
#[derive(Debug, Clone)]
pub struct PrerecordedTranscriptionRequest {
    pub audio: Vec<u8>,
    pub language: Option<String>,
    pub model: Option<String>,
    pub punctuate: bool,
    pub diarize: bool,
    pub smart_format: bool,
    pub utterances: bool,
    pub keywords: Option<Vec<String>>,
}

// Deepgram transcription response
#[derive(Debug, Clone, Deserialize)]
pub struct DeepgramTranscriptionResponse {
    pub metadata: DeepgramMetadata,
    pub results: DeepgramResults,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeepgramMetadata {
    pub request_id: String,
    pub duration: f32,
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeepgramResults {
    pub channels: Vec<DeepgramChannel>,
    pub utterances: Option<Vec<DeepgramUtterance>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeepgramChannel {
    pub alternatives: Vec<DeepgramAlternative>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeepgramAlternative {
    pub transcript: String,
    pub confidence: f32,
    pub words: Vec<DeepgramWord>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeepgramWord {
    pub word: String,
    pub start: f32,
    pub end: f32,
    pub confidence: f32,
    pub speaker: Option<u32>,
    pub punctuated_word: Option<String>,
}


#[derive(Debug, Clone, Deserialize)]
pub struct DeepgramUtterance {
    pub confidence: f32,
    pub transcript: String,
    pub words: Vec<DeepgramWord>,
    pub speaker: Option<u32>,
}


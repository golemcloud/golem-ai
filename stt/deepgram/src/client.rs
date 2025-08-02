use golem_stt::golem::stt::types::SttError;
use log::{error, trace};
use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};
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

        Self {
            api_key,
            client: Client::new(),
            base_url,
            timeout,
            max_retries,
        }
    }

    pub fn transcribe_prerecorded(&self, request: PrerecordedTranscriptionRequest) -> Result<DeepgramTranscriptionResponse, SttError> {
        let mut attempts = 0;
        let url = format!("{}/listen", self.base_url);
        
        loop {
            let mut req = self.client
                .post(&url)
                .header("Authorization", format!("Token {}", self.api_key))
                .header("Content-Type", "application/json")
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
    pub transaction_key: String,
    pub request_id: String,
    pub sha256: String,
    pub created: String,
    pub duration: f32,
    pub channels: u32,
    pub models: Vec<String>,
    pub model_info: Option<serde_json::Value>,
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
    pub paragraphs: Option<DeepgramParagraphs>,
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
pub struct DeepgramParagraphs {
    pub transcript: String,
    pub paragraphs: Vec<DeepgramParagraph>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeepgramParagraph {
    pub sentences: Vec<DeepgramSentence>,
    pub start: f32,
    pub end: f32,
    pub num_words: u32,
    pub speaker: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeepgramSentence {
    pub text: String,
    pub start: f32,
    pub end: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeepgramUtterance {
    pub start: f32,
    pub end: f32,
    pub confidence: f32,
    pub channel: u32,
    pub transcript: String,
    pub words: Vec<DeepgramWord>,
    pub speaker: Option<u32>,
    pub id: String,
}

// Live streaming structures (for future use)
#[derive(Debug, Clone, Serialize)]
pub struct LiveStreamingConfig {
    pub language: Option<String>,
    pub model: Option<String>,
    pub punctuate: bool,
    pub diarize: bool,
    pub smart_format: bool,
    pub interim_results: bool,
    pub utterance_end_ms: Option<u32>,
    pub vad_events: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LiveTranscriptionResponse {
    pub channel_index: Vec<u32>,
    pub duration: f32,
    pub start: f32,
    pub is_final: bool,
    pub speech_final: Option<bool>,
    pub channel: DeepgramChannel,
    pub metadata: Option<LiveMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LiveMetadata {
    pub request_id: String,
    pub model_info: Option<serde_json::Value>,
}

// Error response structure
#[derive(Debug, Clone, Deserialize)]
pub struct DeepgramErrorResponse {
    pub error: String,
    pub request_id: Option<String>,
}
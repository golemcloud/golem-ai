use golem_stt::golem::stt::types::SttError;
use log::{error, trace};
use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct GoogleSpeechClient {
    api_key: String,
    client: Client,
    base_url: String,
    timeout: Duration,
}

impl GoogleSpeechClient {
    pub fn new(api_key: String) -> Self {
        let timeout_str = std::env::var("STT_PROVIDER_TIMEOUT").unwrap_or_else(|_| "30".to_string());
        let timeout = Duration::from_secs(timeout_str.parse().unwrap_or(30));
        
        
        let base_url = std::env::var("STT_PROVIDER_ENDPOINT")
            .unwrap_or_else(|_| "https://speech.googleapis.com/v1".to_string());

        Self {
            api_key,
            client: Client::new(),
            base_url,
            timeout,
        }
    }

    pub fn transcribe(&self, request: RecognizeRequest) -> Result<RecognizeResponse, SttError> {
        let url = format!("{}/speech:recognize?key={}", self.base_url, self.api_key);
        
        match self.make_request(Method::POST, &url, Some(&request)) {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<RecognizeResponse>() {
                        Ok(result) => Ok(result),
                        Err(e) => {
                            error!("Failed to parse Google Speech response: {}", e);
                            Err(SttError::InternalError(format!("Failed to parse response: {}", e)))
                        }
                    }
                } else {
                    Err(self.handle_error_response(response))
                }
            }
            Err(e) => Err(SttError::NetworkError(format!("Request failed: {}", e)))
        }
    }

    fn make_request<T: Serialize>(
        &self,
        method: Method,
        url: &str,
        body: Option<&T>,
    ) -> Result<Response, reqwest::Error> {
        let mut req = self
            .client
            .request(method, url)
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognizeRequest {
    pub config: RecognitionConfig,
    pub audio: RecognitionAudio,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognitionConfig {
    pub encoding: AudioEncoding,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate_hertz: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_channel_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternative_language_codes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_alternatives: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profanity_filter: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speech_contexts: Option<Vec<SpeechContext>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_word_time_offsets: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_word_confidence: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_automatic_punctuation: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phrases: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boost: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognitionAudio {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>, // Base64 encoded audio data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
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
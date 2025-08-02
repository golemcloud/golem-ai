use serde::Deserialize;

/// WIT mirrors are re-declared minimally within provider components via generated bindings.
/// In the shared crate we define an internal error enum and provide mapping helpers that
/// components can convert to wit::types::SttError quickly.
#[derive(Debug)]
pub enum InternalSttError {
    InvalidAudio(String),
    UnsupportedFormat(String),
    UnsupportedLanguage(String),
    TranscriptionFailed(String),
    Unauthorized(String),
    AccessDenied(String),
    QuotaExceeded(QuotaInfo),
    RateLimited(u32),
    InsufficientCredits,
    UnsupportedOperation(String),
    ServiceUnavailable(String),
    NetworkError(String),
    InternalError(String),
}

#[derive(Debug, Clone)]
pub enum QuotaUnit {
    Seconds,
    Requests,
    Credits,
}

#[derive(Debug, Clone)]
pub struct QuotaInfo {
    pub used: u32,
    pub limit: u32,
    pub reset_time: u64,
    pub unit: QuotaUnit,
}

impl InternalSttError {
    pub fn network<S: Into<String>>(msg: S) -> Self {
        InternalSttError::NetworkError(msg.into())
    }
    pub fn internal<S: Into<String>>(msg: S) -> Self {
        InternalSttError::InternalError(msg.into())
    }
    pub fn unauthorized<S: Into<String>>(msg: S) -> Self {
        InternalSttError::Unauthorized(msg.into())
    }
    pub fn access_denied<S: Into<String>>(msg: S) -> Self {
        InternalSttError::AccessDenied(msg.into())
    }
    pub fn rate_limited(after_secs: u32) -> Self {
        InternalSttError::RateLimited(after_secs)
    }
    pub fn quota_exceeded(info: QuotaInfo) -> Self {
        InternalSttError::QuotaExceeded(info)
    }
    pub fn unsupported<S: Into<String>>(msg: S) -> Self {
        InternalSttError::UnsupportedOperation(msg.into())
    }
    pub fn invalid_audio<S: Into<String>>(msg: S) -> Self {
        InternalSttError::InvalidAudio(msg.into())
    }
    pub fn unsupported_format<S: Into<String>>(msg: S) -> Self {
        InternalSttError::UnsupportedFormat(msg.into())
    }
    pub fn unsupported_language<S: Into<String>>(msg: S) -> Self {
        InternalSttError::UnsupportedLanguage(msg.into())
    }
    pub fn failed<S: Into<String>>(msg: S) -> Self {
        InternalSttError::TranscriptionFailed(msg.into())
    }
    pub fn service_unavailable<S: Into<String>>(msg: S) -> Self {
        InternalSttError::ServiceUnavailable(msg.into())
    }
}

/// Minimal Deepgram error payloads we may encounter.
/// We keep it resilient and optional because API responses can vary.
#[derive(Debug, Deserialize)]
pub struct DeepgramErrorResponse {
    #[serde(default)]
    pub err: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

pub fn extract_deepgram_error_message(body: &str) -> String {
    if let Ok(parsed) = serde_json::from_str::<DeepgramErrorResponse>(body) {
        parsed
            .err
            .or(parsed.error)
            .or(parsed.message)
            .unwrap_or_else(|| body.to_string())
    } else {
        body.to_string()
    }
}

/// Minimal Google error payloads.
#[derive(Debug, Deserialize)]
pub struct GoogleErrorResponse {
    pub error: GoogleError,
}

#[derive(Debug, Deserialize)]
pub struct GoogleError {
    pub message: String,
    pub code: u16,
    pub status: String,
}

pub fn extract_google_error_message(body: &str) -> String {
    if let Ok(parsed) = serde_json::from_str::<GoogleErrorResponse>(body) {
        parsed.error.message
    } else {
        body.to_string()
    }
}

/// Minimal Azure error payloads.
#[derive(Debug, Deserialize)]
pub struct AzureErrorResponse {
    pub error: AzureError,
}

#[derive(Debug, Deserialize)]
pub struct AzureError {
    pub message: String,
    pub code: String,
}

pub fn extract_azure_error_message(body: &str) -> String {
    if let Ok(parsed) = serde_json::from_str::<AzureErrorResponse>(body) {
        parsed.error.message
    } else {
        body.to_string()
    }
}

/// Minimal AWS error payloads.
#[derive(Debug, Deserialize)]
pub struct AwsErrorResponse {
    #[serde(rename = "Message")]
    pub message: Option<String>,
}

pub fn extract_aws_error_message(body: &str) -> String {
    if let Ok(parsed) = serde_json::from_str::<AwsErrorResponse>(body) {
        parsed.message.unwrap_or_else(|| body.to_string())
    } else {
        body.to_string()
    }
}

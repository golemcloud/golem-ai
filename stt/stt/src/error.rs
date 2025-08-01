use crate::golem::stt::types::{SttError, QuotaInfo, QuotaUnit};
use reqwest::StatusCode;

pub fn from_reqwest_error(details: impl AsRef<str>, err: reqwest::Error) -> SttError {
    if err.is_timeout() {
        SttError::NetworkError(format!("{}: timeout", details.as_ref()))
    } else if err.is_request() {
        SttError::NetworkError(format!("{}: request failed", details.as_ref()))
    } else {
        SttError::InternalError(format!("{}: {err}", details.as_ref()))
    }
}

pub fn error_code_from_status(status: StatusCode) -> SttError {
    match status {
        StatusCode::TOO_MANY_REQUESTS => SttError::RateLimited(429),
        StatusCode::UNAUTHORIZED => SttError::Unauthorized("Invalid API key or authentication failed".to_string()),
        StatusCode::FORBIDDEN => SttError::AccessDenied("Access denied or insufficient permissions".to_string()),
        StatusCode::BAD_REQUEST => SttError::InvalidAudio("Invalid audio format or parameters".to_string()),
        StatusCode::UNSUPPORTED_MEDIA_TYPE => SttError::UnsupportedFormat("Unsupported audio format".to_string()),
        StatusCode::SERVICE_UNAVAILABLE => SttError::ServiceUnavailable("Service temporarily unavailable".to_string()),
        StatusCode::INSUFFICIENT_STORAGE => SttError::InsufficientCredits,
        status if status.is_client_error() => SttError::InvalidAudio(format!("Client error: {status}")),
        status if status.is_server_error() => SttError::ServiceUnavailable(format!("Server error: {status}")),
        _ => SttError::InternalError(format!("Unexpected status code: {status}")),
    }
}

pub fn create_quota_error(used: u32, limit: u32, reset_time: u64, unit: QuotaUnit) -> SttError {
    SttError::QuotaExceeded(QuotaInfo {
        used,
        limit,
        reset_time,
        unit,
    })
}

pub fn language_not_supported(language: &str) -> SttError {
    SttError::UnsupportedLanguage(format!("Language '{language}' is not supported by this provider"))
}

pub fn operation_not_supported(operation: &str) -> SttError {
    SttError::UnsupportedOperation(format!("Operation '{operation}' is not supported by this provider"))
}
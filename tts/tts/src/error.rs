use crate::exports::golem::tts::types::{QuotaInfo, QuotaUnit, TtsError};
use reqwest::StatusCode;

pub fn unsupported(_what: impl AsRef<str>) -> TtsError {
    TtsError::UnsupportedOperation(_what.as_ref().to_string())
}

pub fn invalid_text(message: impl AsRef<str>) -> TtsError {
    TtsError::InvalidText(message.as_ref().to_string())
}

pub fn internal_error(message: impl AsRef<str>) -> TtsError {
    TtsError::InternalError(message.as_ref().to_string())
}

pub fn voice_not_found(voice_id: impl AsRef<str>) -> TtsError {
    TtsError::VoiceNotFound(voice_id.as_ref().to_string())
}

pub fn network_error(message: impl AsRef<str>) -> TtsError {
    TtsError::NetworkError(message.as_ref().to_string())
}

pub fn rate_limited(retry_after_seconds: u32) -> TtsError {
    TtsError::RateLimited(retry_after_seconds)
}

pub fn quota_exceeded(used: u32, limit: u32, reset_time: u64, unit: QuotaUnit) -> TtsError {
    TtsError::QuotaExceeded(QuotaInfo {
        used,
        limit,
        reset_time,
        unit,
    })
}

pub fn synthesis_failed(message: impl AsRef<str>) -> TtsError {
    TtsError::SynthesisFailed(message.as_ref().to_string())
}

pub fn service_unavailable(message: impl AsRef<str>) -> TtsError {
    TtsError::ServiceUnavailable(message.as_ref().to_string())
}

pub fn unauthorized(message: impl AsRef<str>) -> TtsError {
    TtsError::Unauthorized(message.as_ref().to_string())
}

pub fn from_reqwest_error(details: impl AsRef<str>, err: reqwest::Error) -> TtsError {
    if err.is_timeout() {
        TtsError::NetworkError(format!("{}: timeout", details.as_ref()))
    } else if err.is_request() {
        TtsError::NetworkError(format!("{}: connection failed", details.as_ref()))
    } else {
        TtsError::InternalError(format!("{}: {err}", details.as_ref()))
    }
}

pub fn tts_error_from_status(status: StatusCode) -> TtsError {
    match status {
        StatusCode::TOO_MANY_REQUESTS => TtsError::RateLimited(60), // Default 60 seconds
        StatusCode::REQUEST_TIMEOUT | StatusCode::GATEWAY_TIMEOUT => {
            TtsError::NetworkError("Request timeout".to_string())
        }
        StatusCode::NOT_FOUND => TtsError::VoiceNotFound("Voice not found".to_string()),
        StatusCode::BAD_REQUEST => TtsError::InvalidText("Bad request".to_string()),
        StatusCode::UNAUTHORIZED => TtsError::Unauthorized("Authentication failed".to_string()),
        StatusCode::FORBIDDEN => TtsError::AccessDenied("Access denied".to_string()),
        StatusCode::PAYMENT_REQUIRED => TtsError::QuotaExceeded(QuotaInfo {
            used: 0,
            limit: 0,
            reset_time: 0,
            unit: QuotaUnit::Characters,
        }),
        StatusCode::UNPROCESSABLE_ENTITY => {
            TtsError::SynthesisFailed("Unable to process synthesis request".to_string())
        }
        StatusCode::SERVICE_UNAVAILABLE => {
            TtsError::ServiceUnavailable("Service temporarily unavailable".to_string())
        }
        _ if status.is_client_error() => TtsError::InvalidText(format!("Client error: {status}")),
        _ => TtsError::InternalError(format!("Server error: {status}")),
    }
}

use crate::golem::tts::types::{QuotaInfo as WitQuotaInfo, QuotaUnit as WitQuotaUnit};
use crate::golem::tts::types::TtsError as WitTtsError;

use derive_more::From;

#[allow(unused)]
#[derive(Debug, From)]
pub enum Error {
    EnvVariablesNotSet(String),
    AuthError(String),
    #[from]
    Http(String, crate::http::Error),
    ValidationError(String),
    Unsupported(String),
    VoiceNotFound(String),
    ModelNotFound(String),
    VoiceUnavailable(String),
    Unauthorized(String),
    AccessDenied(String),
    RateLimited(u32),
    QuotaExceeded {
        used: u32,
        limit: u32,
        reset_time: u64,
        unit: QuotaUnit,
    },
    InsufficientCredits,
    SynthesisFailed(String),
    InvalidConfiguration(String),
    ServiceUnavailable(String),
    NetworkError(String),
    Internal(String),
    InvalidStorageLocation(String),
    StorageAccessDenied(String),
}

#[derive(Debug, Clone)]
pub enum QuotaUnit {
    Characters,
    Requests,
    Seconds,
    Credits,
}

impl Error {
    pub fn request_id(&self) -> &str {
        match self {
            Error::Http(request_id, ..) => request_id,
            _ => "",
        }
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}

impl From<Error> for WitTtsError {
    fn from(error: Error) -> Self {
        match error {
            Error::ValidationError(message) => WitTtsError::InvalidText(message),
            Error::Unsupported(message) => WitTtsError::UnsupportedOperation(message),
            Error::VoiceNotFound(message) => WitTtsError::VoiceNotFound(message),
            Error::ModelNotFound(message) => WitTtsError::ModelNotFound(message),
            Error::VoiceUnavailable(message) => WitTtsError::VoiceUnavailable(message),
            Error::Unauthorized(message) => WitTtsError::Unauthorized(message),
            Error::AccessDenied(message) => WitTtsError::AccessDenied(message),
            Error::RateLimited(seconds) => WitTtsError::RateLimited(seconds),
            Error::QuotaExceeded {
                used,
                limit,
                reset_time,
                unit,
            } => WitTtsError::QuotaExceeded(WitQuotaInfo {
                used,
                limit,
                reset_time,
                unit: match unit {
                    QuotaUnit::Characters => WitQuotaUnit::Characters,
                    QuotaUnit::Requests => WitQuotaUnit::Requests,
                    QuotaUnit::Seconds => WitQuotaUnit::Seconds,
                    QuotaUnit::Credits => WitQuotaUnit::Credits,
                },
            }),
            Error::InsufficientCredits => WitTtsError::InsufficientCredits,
            Error::SynthesisFailed(message) => WitTtsError::SynthesisFailed(message),
            Error::InvalidConfiguration(message) => WitTtsError::InvalidConfiguration(message),
            Error::ServiceUnavailable(message) => WitTtsError::ServiceUnavailable(message),
            Error::NetworkError(message) => WitTtsError::NetworkError(message),
            Error::Internal(message) => WitTtsError::InternalError(message),
            Error::InvalidStorageLocation(message) => {
                WitTtsError::InvalidStorageLocation(message)
            }
            Error::StorageAccessDenied(message) => WitTtsError::StorageAccessDenied(message),
            Error::EnvVariablesNotSet(message) | Error::AuthError(message) => {
                WitTtsError::InternalError(message)
            }
            Error::Http(request_id, error) => {
                WitTtsError::InternalError(format!("{request_id}: {error}"))
            }
        }
    }
}

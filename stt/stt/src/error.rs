use crate::golem::stt::types::SttError as WitSttError;

use derive_more::From;

#[allow(unused)]
#[derive(Debug, From)]
pub enum Error {
    EnvVariablesNotSet(String),
    AuthError(String),
    #[from]
    Http(String, crate::http::Error),

    APIBadRequest {
        request_id: String,
        provider_error: String,
    },
    APIUnauthorized {
        request_id: String,
        provider_error: String,
    },
    APIForbidden {
        request_id: String,
        provider_error: String,
    },
    APIAccessDenied {
        request_id: String,
        provider_error: String,
    },
    APINotFound {
        request_id: String,
        provider_error: String,
    },
    APIConflict {
        request_id: String,
        provider_error: String,
    },
    APIUnprocessableEntity {
        request_id: String,
        provider_error: String,
    },
    APIRateLimit {
        request_id: String,
        provider_error: String,
    },
    #[allow(clippy::enum_variant_names)]
    APIInternalServerError {
        request_id: String,
        provider_error: String,
    },
    APIUnknown {
        request_id: String,
        provider_error: String,
    },
}

impl Error {
    pub fn request_id(&self) -> &str {
        match self {
            Error::APIBadRequest { request_id, .. } => request_id,
            Error::APIUnauthorized { request_id, .. } => request_id,
            Error::APIForbidden { request_id, .. } => request_id,
            Error::APIAccessDenied { request_id, .. } => request_id,
            Error::APIConflict { request_id, .. } => request_id,
            Error::APIUnprocessableEntity { request_id, .. } => request_id,
            Error::APIRateLimit { request_id, .. } => request_id,
            Error::APIInternalServerError { request_id, .. } => request_id,
            Error::APIUnknown { request_id, .. } => request_id,
            Error::Http(request_id, ..) => request_id,
            Error::APINotFound { request_id, .. } => request_id,
            Error::EnvVariablesNotSet(_) => "",
            Error::AuthError(_) => "",
        }
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}

impl From<Error> for WitSttError {
    fn from(error: Error) -> Self {
        match error {
            Error::APIBadRequest {
                request_id,
                provider_error,
            } => WitSttError::TranscriptionFailed(format!("{request_id}: {provider_error}")),
            Error::APIUnauthorized {
                request_id,
                provider_error,
            } => WitSttError::AccessDenied(format!("{request_id}: {provider_error}")),
            Error::APIForbidden {
                request_id,
                provider_error,
            } => WitSttError::Unauthorized(format!("{request_id}: {provider_error}")),
            Error::APIAccessDenied {
                request_id,
                provider_error,
            } => WitSttError::AccessDenied(format!("{request_id}: {provider_error}")),
            Error::APINotFound {
                request_id,
                provider_error,
            } => WitSttError::UnsupportedOperation(format!("{request_id}: {provider_error}")),
            Error::APIConflict {
                request_id,
                provider_error,
            } => WitSttError::ServiceUnavailable(format!("{request_id}: {provider_error}")),
            Error::APIUnprocessableEntity {
                request_id,
                provider_error,
            } => WitSttError::ServiceUnavailable(format!("{request_id}: {provider_error}")),
            Error::APIRateLimit {
                request_id,
                provider_error,
            } => WitSttError::RateLimited(format!("{request_id}: {provider_error}")),
            Error::APIInternalServerError {
                request_id,
                provider_error,
            } => WitSttError::ServiceUnavailable(format!("{request_id}: {provider_error}")),
            Error::APIUnknown {
                request_id,
                provider_error,
            } => WitSttError::InternalError(format!("{request_id}: {provider_error}")),
            Error::Http(request_id, error) => {
                WitSttError::InternalError(format!("{request_id}: Internal error: {error}"))
            }
            Error::EnvVariablesNotSet(reason) => {
                WitSttError::InternalError(format!("Internal error: {reason}"))
            }
            Error::AuthError(reason) => {
                WitSttError::InternalError(format!("Internal error: {reason}"))
            }
        }
    }
}

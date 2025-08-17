use crate::exports::golem::vector::types::VectorError;
use reqwest::StatusCode;

/// Creates a `VectorError` value representing a missing or invalid parameter.
pub fn invalid_params(details: impl AsRef<str>) -> VectorError {
    VectorError::InvalidParams(details.as_ref().to_string())
}

/// Creates a `VectorError` value representing resource not found.
pub fn not_found(details: impl AsRef<str>) -> VectorError {
    VectorError::NotFound(details.as_ref().to_string())
}

/// Creates a `VectorError` value representing resource already exists.
pub fn already_exists(details: impl AsRef<str>) -> VectorError {
    VectorError::AlreadyExists(details.as_ref().to_string())
}

/// Creates a `VectorError` value for unsupported feature.
pub fn unsupported_feature(details: impl AsRef<str>) -> VectorError {
    VectorError::UnsupportedFeature(details.as_ref().to_string())
}

/// Creates a `VectorError` value for provider-specific internal error.
pub fn provider_error(details: impl AsRef<str>) -> VectorError {
    VectorError::ProviderError(details.as_ref().to_string())
}

/// Creates a `VectorError` value for connection/network problems.
pub fn connection_error(details: impl AsRef<str>) -> VectorError {
    VectorError::ConnectionError(details.as_ref().to_string())
}

/// Creates a `VectorError` value for unauthorized access.
pub fn unauthorized(details: impl AsRef<str>) -> VectorError {
    VectorError::Unauthorized(details.as_ref().to_string())
}

/// Creates a `VectorError` value for rate-limiting situations.
pub fn rate_limited() -> VectorError {
    VectorError::RateLimited("Rate limit exceeded".into())
}

/// Creates a `VectorError` value for dimension mismatch.
pub fn dimension_mismatch(details: impl AsRef<str>) -> VectorError {
    VectorError::DimensionMismatch(details.as_ref().to_string())
}

/// Creates a `VectorError` value for invalid vector data.
pub fn invalid_vector(details: impl AsRef<str>) -> VectorError {
    VectorError::InvalidVector(details.as_ref().to_string())
}

/// Converts a `reqwest::Error` into an internal provider error.
pub fn from_reqwest_error(details: impl AsRef<str>, _err: reqwest::Error) -> VectorError {
    provider_error(details)
}

/// Maps HTTP status codes to appropriate vector error types.
pub fn vector_error_from_status(status: StatusCode, message: impl AsRef<str>) -> VectorError {
    let msg = message.as_ref().to_string();
    match status {
        StatusCode::TOO_MANY_REQUESTS => VectorError::RateLimited(msg),
        StatusCode::BAD_REQUEST => VectorError::InvalidParams(msg),
        StatusCode::NOT_IMPLEMENTED | StatusCode::METHOD_NOT_ALLOWED => {
            VectorError::UnsupportedFeature(msg)
        }
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => VectorError::Unauthorized(msg),
        StatusCode::NOT_FOUND => VectorError::NotFound(msg),
        _ => VectorError::ProviderError(format!("HTTP {status}: {msg}")),
    }
}

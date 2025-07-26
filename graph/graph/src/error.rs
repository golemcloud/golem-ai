use crate::golem::graph::errors::GraphError;
use reqwest::StatusCode;
use std::error::Error;

pub fn unsupported(what: impl AsRef<str>) -> GraphError {
    GraphError::UnsupportedOperation(format!("Unsupported: {}", what.as_ref()))
}

pub fn from_reqwest_error(context: impl AsRef<str>, err: reqwest::Error) -> GraphError {
    GraphError::InternalError(format!("{}: {}", context.as_ref(), err))
}

pub fn from_generic_error<T: Error>(context: impl AsRef<str>, err: T) -> GraphError {
    GraphError::InternalError(format!("{}: {}", context.as_ref(), err))
}

pub fn error_from_status(status: StatusCode, body: Option<String>) -> GraphError {
    match status {
        StatusCode::TOO_MANY_REQUESTS => {
            let message = body.unwrap_or_else(|| "Rate limit exceeded".to_string());
            GraphError::ResourceExhausted(message)
        }
        StatusCode::UNAUTHORIZED =>
            GraphError::AuthenticationFailed(
                body.unwrap_or_else(|| "Authentication failed".to_string())
            ),
        StatusCode::FORBIDDEN =>
            GraphError::AuthorizationFailed(
                body.unwrap_or_else(|| "Authorization failed".to_string())
            ),
        StatusCode::REQUEST_TIMEOUT | StatusCode::GATEWAY_TIMEOUT => GraphError::Timeout,
        StatusCode::SERVICE_UNAVAILABLE =>
            GraphError::ServiceUnavailable(
                body.unwrap_or_else(|| "Service unavailable".to_string())
            ),
        s if s.is_client_error() =>
            GraphError::InvalidQuery(body.unwrap_or_else(|| "Invalid query".to_string())),
        _ => {
            let message = match body {
                Some(b) => format!("HTTP {status}: {b}"),
                None => format!("HTTP {status}"),
            };
            GraphError::InternalError(message)
        }
    }
}

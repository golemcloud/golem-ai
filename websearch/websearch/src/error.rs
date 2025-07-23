use crate::golem::web_search::web_search::SearchError;
use reqwest::StatusCode;
use std::error::Error;

pub fn unsupported(what: impl AsRef<str>) -> SearchError {
    SearchError::UnsupportedFeature(format!("Unsupported: {}", what.as_ref()))
}

pub fn from_reqwest_error(context: impl AsRef<str>, err: reqwest::Error) -> SearchError {
    SearchError::BackendError(format!("{}: {}", context.as_ref(), err))
}

pub fn from_generic_error<T: Error>(context: impl AsRef<str>, err: T) -> SearchError {
    SearchError::BackendError(format!("{}: {}", context.as_ref(), err))
}

pub fn error_from_status(status: StatusCode, body: Option<String>) -> SearchError {
    match status {
        StatusCode::TOO_MANY_REQUESTS => {
            let retry_after = body.and_then(|b| b.parse::<u32>().ok()).unwrap_or(60);
            SearchError::RateLimited(retry_after)
        }
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::PAYMENT_REQUIRED => {
            SearchError::BackendError("Authentication failed".to_string())
        }
        s if s.is_client_error() => SearchError::InvalidQuery,
        _ => {
            let message = match body {
                Some(b) => format!("HTTP {status}: {b}"),
                None => format!("HTTP {status}"),
            };
            SearchError::BackendError(message)
        }
    }
}


use reqwest::{Response, StatusCode};

use crate::golem::search::types::SearchError;

pub fn unsupported_error() -> SearchError {
    SearchError::Unsupported
}

pub fn timeout_error(message: String) -> SearchError {
    SearchError::Timeout(message)
}

pub fn from_reqwest_error(
    details: impl AsRef<str>,
    err: reqwest::Error,
) -> SearchError {
    SearchError::Internal(format!("{}: {err}", details.as_ref()))
}




pub fn from_status_code(response: Response) -> SearchError {
    let status_code = response.status();
    let body = response.text().map_err(|err| from_reqwest_error("Failed to decode response body", err)).unwrap_or_default();
    match status_code {
        StatusCode::TOO_MANY_REQUESTS => SearchError::RateLimited(body),
        StatusCode::NOT_FOUND => SearchError::IndexNotFound(body),
        StatusCode::REQUEST_TIMEOUT => SearchError::Timeout(body),
        StatusCode::BAD_REQUEST => SearchError::InvalidQuery(body),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::PAYMENT_REQUIRED => {
            // Instead of message, we could use the status code to
            // allow consumers to handle the error
            // in a more specific way.
            SearchError::AuthenticationFailed(status_code.as_u16())
        }
        _ => SearchError::Internal(format!("StatusCode : {}  {}", status_code.as_u16(), body)),
    }
}
use golem_tts::golem::tts::types::TtsError;
use reqwest::Response;

/// Convert HTTP response to appropriate TtsError based on status code
pub fn from_http_error(response: Response) -> TtsError {
    let status_code = response.status().as_u16();
    let error_text = response.text().unwrap_or_default();
    match status_code {
        401 => TtsError::Unauthorized(format!("ERROR: {error_text}")),
        403 => TtsError::AccessDenied(format!("ERROR: {error_text}")),
        429 => TtsError::RateLimited(status_code.into()),
        500..=599 => TtsError::ServiceUnavailable(format!("ERROR: {error_text}")),
        _ => TtsError::RequestError(format!("ERROR: {error_text}")),
    }
}

pub fn unsupported<T>(msg: &str) -> Result<T, TtsError> {
    Err(TtsError::UnsupportedOperation(msg.to_string()))
}

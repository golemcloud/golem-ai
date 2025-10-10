use golem_tts::golem::tts::types::TtsError;
use reqwest::Response;

pub fn from_http_error(response: Response) -> TtsError {
    let status = response.status();

    match status.as_u16() {
        400 => TtsError::RequestError("Bad Request - Invalid request parameters".to_string()),
        401 => TtsError::Unauthorized("Invalid or expired Google Cloud credentials".to_string()),
        403 => TtsError::AccessDenied(
            "Google Cloud API access denied - check service account permissions".to_string(),
        ),
        404 => TtsError::VoiceNotFound("Voice or resource not found".to_string()),
        413 => TtsError::TextTooLong(5000), // Google TTS has a ~5000 character limit
        429 => TtsError::RateLimited(429),
        500 => {
            TtsError::ServiceUnavailable("Google Cloud Text-to-Speech service error".to_string())
        }
        503 => TtsError::ServiceUnavailable(
            "Google Cloud Text-to-Speech service temporarily unavailable".to_string(),
        ),
        501..=599 => TtsError::ServiceUnavailable(format!("Google Cloud server error: {}", status)),
        _ => TtsError::RequestError(format!("HTTP error: {}", status)),
    }
}

pub fn unsupported<T>(msg: &str) -> Result<T, TtsError> {
    Err(TtsError::UnsupportedOperation(msg.to_string()))
}

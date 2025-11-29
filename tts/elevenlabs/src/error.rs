use golem_tts::golem::tts::types::TtsError;
use reqwest::Response;

pub fn from_http_error(response: Response) -> TtsError {
    let status_code = response.status().as_u16();
    let error_text = response.text().unwrap_or_default();

    // Check for specific ElevenLabs error messages in the response body
    if error_text.contains("max_character_limit_exceeded") {
        return TtsError::RequestError(
            "You are sending too many characters in a single request".to_string(),
        );
    }
    if error_text.contains("invalid_api_key") {
        return TtsError::Unauthorized("You have not set your API key correctly".to_string());
    }
    if error_text.contains("quota_exceeded") {
        return TtsError::AccessDenied(
            "You have insufficient quota to complete the request".to_string(),
        );
    }
    if error_text.contains("voice_not_found") {
        return TtsError::RequestError("You have entered the incorrect voice_id".to_string());
    }
    if error_text.contains("only_for_creator+") {
        return TtsError::AccessDenied(
            "You are trying to use professional voices on a free or basic subscription".to_string(),
        );
    }
    if error_text.contains("too_many_concurrent_requests") {
        return TtsError::RateLimited(status_code.into());
    }
    if error_text.contains("system_busy") {
        return TtsError::ServiceUnavailable("Our services are experiencing high levels of traffic and your request could not be processed".to_string());
    }

    // Fallback to status code based mapping
    match status_code {
        400 => TtsError::RequestError(format!("Bad Request: {error_text}")),
        401 => TtsError::Unauthorized(format!("Unauthorized: {error_text}")),
        403 => TtsError::AccessDenied(format!("Forbidden: {error_text}")),
        429 => TtsError::RateLimited(status_code.into()),
        500..=599 => TtsError::ServiceUnavailable(format!("Server Error: {error_text}")),
        _ => TtsError::RequestError(format!("HTTP {status_code}: {error_text}")),
    }
}

pub fn unsupported<T>(msg: &str) -> Result<T, TtsError> {
    Err(TtsError::UnsupportedOperation(msg.to_string()))
}

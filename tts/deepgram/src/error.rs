use golem_tts::golem::tts::types::TtsError;
use regex::Regex;
use reqwest::Response;

pub fn from_http_error(response: Response) -> TtsError {
    let status_code = response.status().as_u16();
    let error_text = response.text().unwrap_or_default();

    match status_code {
        400 => TtsError::RequestError(format!("ERROR: {error_text}")),
        401 => TtsError::Unauthorized(format!("ERROR: {error_text}")),
        402 => TtsError::InsufficientCredits,
        403 => TtsError::AccessDenied(format!("ERROR: {error_text}")),
        404 => TtsError::ModelNotFound(format!("ERROR: {error_text}")),
        413 => {
            if let Some(limit) = extract_character_limit(&error_text) {
                TtsError::TextTooLong(limit)
            } else {
                TtsError::RequestError(format!("ERROR: {error_text}"))
            }
        }
        429 => TtsError::RateLimited(1000), // Deepgram recommended backoff
        500..=599 => TtsError::ServiceUnavailable(format!("ERROR: {error_text}")),
        _ => TtsError::RequestError(format!("ERROR: {error_text}")),
    }
}

pub fn extract_character_limit(error_text: &str) -> Option<u32> {
    let patterns = [
        r"maximum character limit of (\d+)",
        r"(?i)(?:limit|maximum)[^\d]{0,10}(\d+)",
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(error_text) {
                if let Some(m) = caps.get(1) {
                    return m.as_str().parse::<u32>().ok();
                }
            }
        }
    }
    None
}

pub fn unsupported<T>(msg: &str) -> Result<T, TtsError> {
    Err(TtsError::UnsupportedOperation(msg.to_string()))
}

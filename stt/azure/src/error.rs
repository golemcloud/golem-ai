use golem_stt::golem::stt::types::SttError;

pub fn map_http_status(status: u16) -> SttError {
    match status {
        400 => SttError::InvalidAudio("invalid audio".into()),
        401 => SttError::Unauthorized("unauthorized".into()),
        403 => SttError::AccessDenied("access denied".into()),
        404 => SttError::UnsupportedLanguage("not found".into()),
        429 => SttError::RateLimited(0),
        500 | 502 | 503 => SttError::ServiceUnavailable("service unavailable".into()),
        _ => SttError::InternalError(format!("http {status}")),
    }
}

use golem_stt::golem::stt::types::SttError;

pub fn map_http_status(status: u16, msg: String) -> SttError {
    match status {
        400 => SttError::InvalidAudio(msg),
        401 => SttError::Unauthorized(msg),
        402 => SttError::InsufficientCredits,
        403 => SttError::AccessDenied(msg),
        404 => SttError::UnsupportedOperation(msg),
        409 => SttError::TranscriptionFailed(msg),
        413 => SttError::UnsupportedFormat(msg),
        415 => SttError::UnsupportedFormat(msg),
        429 => SttError::RateLimited(0),
        500..=599 => SttError::ServiceUnavailable(msg),
        _ => SttError::InternalError(msg),
    }
}

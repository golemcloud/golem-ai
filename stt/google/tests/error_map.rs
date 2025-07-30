use golem_stt_google::error::map_http_status;
use golem_stt::golem::stt::types::SttError;

#[test]
fn error_mapping_variants() {
    let cases: Vec<(u16, fn(SttError) -> bool)> = vec![
        (400, |e| matches!(e, SttError::InvalidAudio(_))),
        (401, |e| matches!(e, SttError::Unauthorized(_))),
        (403, |e| matches!(e, SttError::AccessDenied(_))),
        (404, |e| matches!(e, SttError::UnsupportedLanguage(_))),
        (429, |e| matches!(e, SttError::RateLimited(_))),
        (500, |e| matches!(e, SttError::ServiceUnavailable(_))),
        (999, |e| matches!(e, SttError::InternalError(_))),
    ];

    for (status, checker) in cases {
        assert!(checker(map_http_status(status)), "status {status} did not map as expected");
    }
} 
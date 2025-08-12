use golem_stt::golem::stt::types::SttError;

#[test]
fn test_error_mapping() {
    assert!(matches!(
        golem_stt_azure::error::map_http_status(400),
        SttError::InvalidAudio(_)
    ));
    
    assert!(matches!(
        golem_stt_azure::error::map_http_status(401),
        SttError::Unauthorized(_)
    ));
    
    assert!(matches!(
        golem_stt_azure::error::map_http_status(403),
        SttError::AccessDenied(_)
    ));
    
    assert!(matches!(
        golem_stt_azure::error::map_http_status(404),
        SttError::UnsupportedLanguage(_)
    ));
    
    assert!(matches!(
        golem_stt_azure::error::map_http_status(429),
        SttError::RateLimited(_)
    ));
    
    assert!(matches!(
        golem_stt_azure::error::map_http_status(500),
        SttError::ServiceUnavailable(_)
    ));
    
    assert!(matches!(
        golem_stt_azure::error::map_http_status(502),
        SttError::ServiceUnavailable(_)
    ));
    
    assert!(matches!(
        golem_stt_azure::error::map_http_status(503),
        SttError::ServiceUnavailable(_)
    ));
    
    assert!(matches!(
        golem_stt_azure::error::map_http_status(999),
        SttError::InternalError(_)
    ));
}

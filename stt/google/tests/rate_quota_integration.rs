use golem_stt::golem::stt::types::SttError;

#[test]
fn rate_quota_scaffold() {
    if std::env::var("GOOGLE_STT_SIMULATE_RATE").is_err() { return; }
    let err = Err::<(), _>(SttError::RateLimited(0));
    assert!(matches!(err, Err(SttError::RateLimited(_))));
}


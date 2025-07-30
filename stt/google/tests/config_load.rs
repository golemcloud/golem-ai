use serial_test::serial;
use golem_stt_google::config::GoogleConfig;

#[test]
#[serial]
fn config_env_parsing_and_defaults() {
    // Prepare fake credentials JSON inline
    static CREDS: &str = include_str!("data/fake_creds.json");

    // Set required env vars
    std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", CREDS);
    std::env::set_var("STT_PROVIDER_TIMEOUT", "42");
    std::env::remove_var("STT_PROVIDER_MAX_RETRIES"); // ensure default kicks in
    std::env::remove_var("STT_PROVIDER_ENDPOINT");

    let cfg = GoogleConfig::load().expect("config should load");

    assert_eq!(cfg.timeout_secs, 42);
    assert_eq!(cfg.max_retries, 3); // default
    assert!(cfg.endpoint.is_none());
    assert!(cfg.credentials_json.contains("client_email"));
    assert_eq!(cfg.max_buffer_bytes, 5_000_000);
} 
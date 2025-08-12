use serial_test::serial;
use std::env;

#[test]
#[serial]
fn test_config_load_success() {
    env::set_var("AZURE_SPEECH_KEY", "test-key");
    env::set_var("AZURE_SPEECH_REGION", "eastus");
    
    let config = golem_stt_azure::config::AzureConfig::load();
    assert!(config.is_ok());
    
    let config = config.unwrap();
    assert_eq!(config.subscription_key, "test-key");
    assert_eq!(config.region, "eastus");
    assert_eq!(config.timeout_secs, 30);
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.max_buffer_bytes, 5_000_000);
    
    env::remove_var("AZURE_SPEECH_KEY");
    env::remove_var("AZURE_SPEECH_REGION");
}

#[test]
#[serial]
fn test_config_load_missing_key() {
    env::remove_var("AZURE_SPEECH_KEY");
    env::set_var("AZURE_SPEECH_REGION", "eastus");
    
    let config = golem_stt_azure::config::AzureConfig::load();
    assert!(config.is_err());
    
    env::remove_var("AZURE_SPEECH_REGION");
}

#[test]
#[serial]
fn test_config_load_missing_region() {
    env::set_var("AZURE_SPEECH_KEY", "test-key");
    env::remove_var("AZURE_SPEECH_REGION");
    
    let config = golem_stt_azure::config::AzureConfig::load();
    assert!(config.is_err());
    
    env::remove_var("AZURE_SPEECH_KEY");
}

#[test]
#[serial]
fn test_config_load_custom_values() {
    env::set_var("AZURE_SPEECH_KEY", "custom-key");
    env::set_var("AZURE_SPEECH_REGION", "westus");
    env::set_var("STT_PROVIDER_TIMEOUT", "60");
    env::set_var("STT_PROVIDER_MAX_RETRIES", "5");
    env::set_var("STT_BUFFER_LIMIT_BYTES", "10000000");
    
    let config = golem_stt_azure::config::AzureConfig::load();
    assert!(config.is_ok());
    
    let config = config.unwrap();
    assert_eq!(config.timeout_secs, 60);
    assert_eq!(config.max_retries, 5);
    assert_eq!(config.max_buffer_bytes, 10_000_000);
    
    env::remove_var("AZURE_SPEECH_KEY");
    env::remove_var("AZURE_SPEECH_REGION");
    env::remove_var("STT_PROVIDER_TIMEOUT");
    env::remove_var("STT_PROVIDER_MAX_RETRIES");
    env::remove_var("STT_BUFFER_LIMIT_BYTES");
}

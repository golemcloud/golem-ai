use crate::exports::golem::tts::types::TtsError;
use std::ffi::OsStr;

pub fn with_config_key<R>(
    key: impl AsRef<OsStr>,
    fail: impl FnOnce(TtsError) -> R,
    succeed: impl FnOnce(String) -> R,
) -> R {
    let key_str = key.as_ref().to_string_lossy().to_string();
    match std::env::var(key) {
        Ok(value) => succeed(value),
        Err(_) => {
            let error = TtsError::InternalError(format!("Missing config key: {key_str}"));
            fail(error)
        }
    }
}

pub fn get_optional_config(key: impl AsRef<OsStr>) -> Option<String> {
    std::env::var(key).ok()
}

pub fn get_config_with_default(key: impl AsRef<OsStr>, default: impl Into<String>) -> String {
    std::env::var(key).unwrap_or_else(|_| default.into())
}

pub fn validate_config_key(key: impl AsRef<OsStr>) -> Result<String, TtsError> {
    let key_str = key.as_ref().to_string_lossy().to_string();
    std::env::var(key)
        .map_err(|_| TtsError::InternalError(format!("Missing config key: {key_str}")))
}

pub fn with_config_keys<R>(keys: &[&str], callback: impl FnOnce(Vec<String>) -> R) -> R {
    let mut values = Vec::new();
    for key in keys {
        match std::env::var(key) {
            Ok(value) => values.push(value),
            Err(_) => {
                return callback(Vec::new());
            }
        }
    }
    callback(values)
}

pub fn get_timeout_config() -> u64 {
    get_config_with_default("TTS_PROVIDER_TIMEOUT", "30")
        .parse()
        .unwrap_or(30)
}

pub fn get_max_retries_config() -> u32 {
    get_config_with_default("TTS_PROVIDER_MAX_RETRIES", "3")
        .parse()
        .unwrap_or(3)
}

pub fn get_endpoint_config(default_endpoint: impl Into<String>) -> String {
    get_config_with_default("TTS_PROVIDER_ENDPOINT", default_endpoint)
}

use crate::golem::search::types::SearchError;
use std::ffi::OsStr;

pub fn with_config_key<R>(
    key: impl AsRef<OsStr>,
    fail: impl FnOnce(SearchError) -> R,
    succeed: impl FnOnce(String) -> R,
) -> R {
    let key_str = key.as_ref().to_string_lossy().to_string();
    match std::env::var(key) {
        Ok(value) => succeed(value),
        Err(_) => {
            let error = SearchError::Internal(format!("Missing config key: {key_str}"));
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

pub fn validate_config_key(key: impl AsRef<OsStr>) -> Result<String, SearchError> {
    let key_str = key.as_ref().to_string_lossy().to_string();
    std::env::var(key).map_err(|_| SearchError::Internal(format!("Missing config key: {key_str}")))
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
    get_config_with_default("SEARCH_PROVIDER_TIMEOUT", "30")
        .parse()
        .unwrap_or(30)
}

pub fn get_max_retries_config() -> u32 {
    get_config_with_default("SEARCH_PROVIDER_MAX_RETRIES", "3")
        .parse()
        .unwrap_or(3)
}

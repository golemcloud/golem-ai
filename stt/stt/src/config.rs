use crate::golem::stt::types::SttError;
use std::ffi::OsStr;

pub fn with_config_key<R>(
    key: impl AsRef<OsStr>,
    fail: impl FnOnce(SttError) -> R,
    succeed: impl FnOnce(String) -> R,
) -> R {
    let key_str = key.as_ref().to_string_lossy().to_string();
    match std::env::var(key) {
        Ok(value) => succeed(value),
        Err(_) => {
            let error = SttError::InternalError(format!("Missing config key: {key_str}"));
            fail(error)
        }
    }
}

pub fn get_config_optional(key: impl AsRef<OsStr>) -> Option<String> {
    std::env::var(key).ok()
}

pub fn get_config_or_default(key: impl AsRef<OsStr>, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

pub fn get_timeout() -> u64 {
    get_config_or_default("STT_PROVIDER_TIMEOUT", "30")
        .parse()
        .unwrap_or(30)
}

pub fn get_max_retries() -> u32 {
    get_config_or_default("STT_PROVIDER_MAX_RETRIES", "3")
        .parse()
        .unwrap_or(3)
}
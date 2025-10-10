use std::{env, ffi::OsStr};

use crate::golem::tts::types::TtsError;

pub fn get_env(key: impl AsRef<OsStr>) -> Result<String, TtsError> {
    let key_str = key.as_ref().to_string_lossy().to_string();
    env::var(&key_str)
        .map_err(|_| TtsError::InvalidConfiguration(format!("Missing config key {key_str}")))
}

pub fn get_parsed_env<T: std::str::FromStr>(key: impl AsRef<OsStr>, default: T) -> T {
    let key_str = key.as_ref().to_string_lossy();
    match env::var(&*key_str) {
        Ok(val) => val.parse().unwrap_or(default),
        Err(_) => default,
    }
}

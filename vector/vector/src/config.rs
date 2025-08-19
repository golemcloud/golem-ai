use crate::exports::golem::vector::types::VectorError;
use std::ffi::OsStr;

/// Gets an expected configuration value from the environment, and fails if it is not found
/// using the `fail` function. Otherwise, it runs `succeed` with the configuration value.
pub fn with_config_key<R>(
    key: impl AsRef<OsStr>,
    fail: impl FnOnce(VectorError) -> R,
    succeed: impl FnOnce(String) -> R,
) -> R {
    let key_str = key.as_ref().to_string_lossy().to_string();
    match std::env::var(&key) {
        Ok(value) => succeed(value),
        Err(_) => {
            let error = VectorError::InvalidParams(format!("Missing config key: {key_str}"));
            fail(error)
        }
    }
}

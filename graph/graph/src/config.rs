use std::ffi::OsStr;

#[derive(Debug, Clone)]
pub enum GraphError {
    ConfigurationError(String),
    InvalidQuery,
    BackendError(String),
    ConnectionFailed(String),
    UnsupportedFeature(String),
}

/// Gets an expected configuration value from the environment, and fails if its is not found
/// using the `fail` function. Otherwise, it runs `succeed` with the configuration value.
pub fn with_graph_config<R>(
    key: impl AsRef<OsStr>,
    fail: impl FnOnce(GraphError) -> R,
    succeed: impl FnOnce(String) -> R,
) -> R {
    let key_str = key.as_ref().to_string_lossy().to_string();
    match std::env::var(&key) {
        Ok(value) => succeed(value),
        Err(_) => {
            let error = GraphError::ConfigurationError(format!("Missing config key: {key_str}"));
            fail(error)
        }
    }
}

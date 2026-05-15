use crate::model::{Error, ErrorCode};
use std::ffi::OsStr;
use std::fmt;
#[cfg(feature = "golem")]
use std::sync::Arc;

/// A source from which a secret string value can be obtained on demand.
///
/// `SecretSource` deliberately defers materialization. In golem mode the
/// underlying value lives in the agent host's secret store and may be
/// rotated at any time, so callers must call [`SecretSource::get`]
/// **immediately before each outgoing request** that uses the secret
/// (e.g. right before setting an `Authorization` header). Never cache
/// the resolved `String` across requests.
///
/// Two variants exist:
///
/// * `Plain` — a value that has already been read from the environment,
///   command line, or another in-process source. Always available.
/// * `Handle` — a `golem_rust::agentic::Secret<String>` reference that
///   is fetched from the agent host on every `.get()` call. Only
///   compiled in when the `golem` feature is enabled.
///
/// `SecretSource` is cheap to clone and intentionally `Send + Sync` so it
/// can be stored in long-lived configuration structs that flow through
/// the durable embed machinery.
#[derive(Clone)]
pub struct SecretSource(SecretSourceInner);

#[derive(Clone)]
enum SecretSourceInner {
    Plain(String),
    #[cfg(feature = "golem")]
    Handle(Arc<golem_rust::agentic::Secret<String>>),
}

impl SecretSource {
    /// Wraps an already-known string value as a `SecretSource`.
    ///
    /// Use this for env-var-based configuration, tests, and any other
    /// case where the secret value is materialized in-process.
    pub fn from_plain(value: impl Into<String>) -> Self {
        Self(SecretSourceInner::Plain(value.into()))
    }

    /// Wraps a host-bound `Secret<String>` handle so that every `.get()`
    /// call fetches the current value from the agent host.
    ///
    /// Only available when the `golem` feature is enabled.
    #[cfg(feature = "golem")]
    pub fn from_handle(handle: golem_rust::agentic::Secret<String>) -> Self {
        Self(SecretSourceInner::Handle(Arc::new(handle)))
    }

    /// Returns the current secret value.
    ///
    /// Callers MUST call this right before each outgoing request that
    /// uses the secret so that hot-rotated host secrets take effect on
    /// the very next request.
    pub fn get(&self) -> String {
        match &self.0 {
            SecretSourceInner::Plain(s) => s.clone(),
            #[cfg(feature = "golem")]
            SecretSourceInner::Handle(handle) => handle.get(),
        }
    }
}

impl fmt::Debug for SecretSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretSource([redacted])")
    }
}

pub fn with_config_key<R>(
    key: impl AsRef<OsStr>,
    fail: impl FnOnce(Error) -> R,
    succeed: impl FnOnce(String) -> R,
) -> R {
    let key_str = key.as_ref().to_string_lossy().to_string();
    match std::env::var(key) {
        Ok(value) => succeed(value),
        Err(_) => {
            let error = Error {
                code: ErrorCode::AuthenticationFailed,
                message: format!("Missing config key: {key_str}"),
                provider_error_json: None,
            };
            fail(error)
        }
    }
}

pub fn get_config_key(key: impl AsRef<OsStr>) -> Result<String, Error> {
    let key_str = key.as_ref().to_string_lossy().to_string();
    std::env::var(key).map_err(|_| Error {
        code: ErrorCode::AuthenticationFailed,
        message: format!("Missing config key: {key_str}"),
        provider_error_json: None,
    })
}

pub fn get_config_key_or_none(key: impl AsRef<OsStr>) -> Option<String> {
    std::env::var(key).ok()
}

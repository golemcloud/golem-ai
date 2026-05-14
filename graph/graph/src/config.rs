use crate::model::connection::ConnectionConfig;
use std::env;
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
/// the durable graph machinery.
///
/// # Interplay with `ConnectionConfig`
///
/// The graph subsystem already exposes an explicit `ConnectionConfig`
/// type that carries provider-specific configuration via
/// `provider_config: Vec<(String, String)>`. Provider crates (e.g.
/// `golem-ai-graph-neo4j`) additionally expose a typed
/// `<Provider>Config` that holds `SecretSource` fields and produces a
/// `ConnectionConfig` on demand. The translation must happen
/// **per top-level call** so that host-rotated secrets take effect on
/// the very next request.
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

/// Retrieves a configuration value from an environment variable, checking the provider_config first.
pub fn with_config_key(config: &ConnectionConfig, key: &str) -> Option<String> {
    config
        .provider_config
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.clone())
        .or_else(|| env::var(key).ok())
}

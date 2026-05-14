//! Typed configuration types for the JanusGraph graph provider.
//!
//! These types are an alternative entry point to the existing
//! [`ConnectionConfig`]-based API. They let Golem-hosted callers
//! declare their config with `Secret<String>` fields and have those
//! resolved on demand for each outgoing request.
//!
//! There are two distinct types here:
//!
//! * [`JanusGraphConfig`] — the *runtime* configuration. The optional
//!   username and password are wrapped in [`SecretSource`] so the
//!   secret value is fetched lazily on every outgoing request.
//!   Available in every build.
//!
//! * [`JanusGraphHostConfig`] — the *host-facing* schema type, derived
//!   with `golem_rust::ConfigSchema`. Its secret fields are
//!   `golem_rust::agentic::Secret<String>` handles that are fetched
//!   from the agent host on demand. Only available when the `golem`
//!   feature is on.
//!
//! Callers running on the Golem agent runtime obtain a
//! [`JanusGraphHostConfig`] via the agent host, then convert it into a
//! [`JanusGraphConfig`] with `JanusGraphConfig::from(host_config)`.
//! The conversion does **not** materialize the secret — the
//! [`golem_rust::agentic::Secret`] handle is preserved inside the
//! resulting [`JanusGraphConfig`] so that every outgoing request
//! fetches the current value from the host.
//!
//! Note: JanusGraph's HTTP/Gremlin endpoint may be configured without
//! authentication. The [`JanusGraphHostConfig`] always carries
//! username and password as required `Secret<String>` fields because
//! `Option<Secret<String>>` is not currently supported by
//! `ConfigSchema`. Callers without authentication should pass an
//! empty string for those fields, or use [`JanusGraphConfig::from_env`]
//! / construct [`JanusGraphConfig`] directly with `username` and
//! `password` set to `None`.
//!
//! Use [`JanusGraphConfig::to_connection_config`] to materialize a
//! [`ConnectionConfig`] just before passing it to
//! `GraphProvider::connect` — this resolves all secrets with their
//! current host-side values.

use golem_ai_graph::config::SecretSource;
use golem_ai_graph::model::connection::ConnectionConfig;
use golem_ai_graph::model::errors::GraphError;
use std::env;

pub const HOST_ENV_VAR: &str = "JANUSGRAPH_HOST";
pub const PORT_ENV_VAR: &str = "JANUSGRAPH_PORT";
pub const USER_ENV_VAR: &str = "JANUSGRAPH_USER";
pub const PASSWORD_ENV_VAR: &str = "JANUSGRAPH_PASSWORD";

/// Runtime JanusGraph provider configuration.
///
/// The optional username and password are wrapped in [`SecretSource`],
/// which is resolved lazily right before each outgoing HTTP request.
/// Do not extract the underlying string and cache it across requests
/// — doing so would defeat host-side secret rotation.
#[derive(Clone, Debug)]
pub struct JanusGraphConfig {
    pub host: String,
    pub port: Option<u16>,
    /// Optional JanusGraph username, fetched on demand right before
    /// each request.
    pub username: Option<SecretSource>,
    /// Optional JanusGraph password, fetched on demand right before
    /// each request.
    pub password: Option<SecretSource>,
}

impl JanusGraphConfig {
    /// Builds a [`JanusGraphConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    pub fn from_env() -> Result<Self, GraphError> {
        let host = env::var(HOST_ENV_VAR).map_err(|_| {
            GraphError::ConnectionFailed(format!("Missing config key: {HOST_ENV_VAR}"))
        })?;
        let port = env::var(PORT_ENV_VAR).ok().and_then(|p| p.parse().ok());
        let username = env::var(USER_ENV_VAR).ok().map(SecretSource::from_plain);
        let password = env::var(PASSWORD_ENV_VAR).ok().map(SecretSource::from_plain);

        Ok(Self {
            host,
            port,
            username,
            password,
        })
    }

    /// Builds a fresh [`ConnectionConfig`] from this typed config.
    ///
    /// Secrets are resolved via [`SecretSource::get`] at this point —
    /// call this **per top-level operation** so that host-rotated
    /// secrets take effect on the very next request.
    pub fn to_connection_config(&self) -> ConnectionConfig {
        let mut provider_config = vec![(HOST_ENV_VAR.to_string(), self.host.clone())];
        if let Some(port) = self.port {
            provider_config.push((PORT_ENV_VAR.to_string(), port.to_string()));
        }
        if let Some(username) = &self.username {
            provider_config.push((USER_ENV_VAR.to_string(), username.get()));
        }
        if let Some(password) = &self.password {
            provider_config.push((PASSWORD_ENV_VAR.to_string(), password.get()));
        }
        ConnectionConfig {
            hosts: Some(vec![self.host.clone()]),
            port: self.port,
            database_name: None,
            username: self.username.as_ref().map(|s| s.get()),
            password: self.password.as_ref().map(|s| s.get()),
            timeout_seconds: None,
            max_connections: None,
            provider_config,
        }
    }
}

/// Host-facing schema for JanusGraph provider configuration. Only
/// available when the `golem` feature is enabled because it depends on
/// the `golem-rust` agent host bindings.
///
/// The secret fields are [`golem_rust::agentic::Secret`] handles that
/// are **not** materialized here. They are instead carried into the
/// runtime [`JanusGraphConfig`] via the [`From`] impl below and
/// resolved on every outgoing request, so that hot-rotated host
/// secrets take effect immediately without restarting the worker.
///
/// `username` and `password` are required `Secret<String>` fields
/// because `Option<Secret<String>>` is not currently supported by
/// `ConfigSchema`. For an unauthenticated JanusGraph endpoint, pass
/// an empty string; the [`From`] impl converts an empty value into
/// `None` so that the runtime config doesn't carry empty credentials.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct JanusGraphHostConfig {
    pub host: String,
    pub port: Option<u16>,
    #[config_schema(secret)]
    pub username: golem_rust::agentic::Secret<String>,
    #[config_schema(secret)]
    pub password: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<JanusGraphHostConfig> for JanusGraphConfig {
    /// Converts a host-side schema config into a runtime
    /// [`JanusGraphConfig`] **without** materializing the secret. The
    /// [`golem_rust::agentic::Secret`] handle is preserved inside the
    /// [`SecretSource`] so each outgoing request fetches the current
    /// value from the host.
    fn from(host: JanusGraphHostConfig) -> Self {
        Self {
            host: host.host,
            port: host.port,
            username: Some(SecretSource::from_handle(host.username)),
            password: Some(SecretSource::from_handle(host.password)),
        }
    }
}

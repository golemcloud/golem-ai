//! Typed configuration types for the Neo4j graph provider.
//!
//! These types are an alternative entry point to the existing
//! [`ConnectionConfig`]-based API. They let Golem-hosted callers
//! declare their config with `Secret<String>` fields and have those
//! resolved on demand for each outgoing request.
//!
//! There are two distinct types here:
//!
//! * [`Neo4jConfig`] — the *runtime* configuration. Holds a
//!   [`SecretSource`] for the username and password so the secret
//!   value is fetched lazily on every outgoing request. Available in
//!   every build.
//!
//! * [`Neo4jHostConfig`] — the *host-facing* schema type, derived with
//!   `golem_rust::ConfigSchema`. Its secret fields are
//!   `golem_rust::agentic::Secret<String>` handles that are fetched
//!   from the agent host on demand. Only available when the `golem`
//!   feature is on.
//!
//! Callers running on the Golem agent runtime obtain a
//! [`Neo4jHostConfig`] via the agent host (e.g. by registering it with
//! `#[agent_config]`), then convert it into a [`Neo4jConfig`] with
//! `Neo4jConfig::from(host_config)`. The conversion does **not**
//! materialize the secret — the [`golem_rust::agentic::Secret`] handle
//! is preserved inside the resulting [`Neo4jConfig`] so that every
//! outgoing request fetches the current value from the host. This is
//! what allows host-side secret rotation to take effect on the next
//! request without restarting the worker.
//!
//! Callers outside Golem (or in tests) use [`Neo4jConfig::from_env`]
//! to read configuration from environment variables exactly the same
//! way the previous version of the crate did.
//!
//! Use [`Neo4jConfig::to_connection_config`] to materialize a
//! [`ConnectionConfig`] just before passing it to
//! `GraphProvider::connect` — this resolves all secrets with their
//! current host-side values.

use golem_ai_graph::config::SecretSource;
use golem_ai_graph::model::connection::ConnectionConfig;
use golem_ai_graph::model::errors::GraphError;
use std::env;

pub const HOST_ENV_VAR: &str = "NEO4J_HOST";
pub const PORT_ENV_VAR: &str = "NEO4J_PORT";
pub const USER_ENV_VAR: &str = "NEO4J_USER";
pub const PASSWORD_ENV_VAR: &str = "NEO4J_PASSWORD";

/// Runtime Neo4j provider configuration.
///
/// The username and password are wrapped in [`SecretSource`], which
/// is resolved lazily right before each outgoing HTTP request. Do not
/// extract the underlying string and cache it across requests — doing
/// so would defeat host-side secret rotation.
#[derive(Clone, Debug)]
pub struct Neo4jConfig {
    pub host: String,
    pub port: Option<u16>,
    /// Neo4j username, fetched on demand right before each request.
    pub username: SecretSource,
    /// Neo4j password, fetched on demand right before each request.
    pub password: SecretSource,
}

impl Neo4jConfig {
    /// Builds a [`Neo4jConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    pub fn from_env() -> Result<Self, GraphError> {
        let host = env::var(HOST_ENV_VAR).map_err(|_| {
            GraphError::ConnectionFailed(format!("Missing config key: {HOST_ENV_VAR}"))
        })?;
        let port = env::var(PORT_ENV_VAR).ok().and_then(|p| p.parse().ok());
        let username = env::var(USER_ENV_VAR).map_err(|_| {
            GraphError::ConnectionFailed(format!("Missing config key: {USER_ENV_VAR}"))
        })?;
        let password = env::var(PASSWORD_ENV_VAR).map_err(|_| {
            GraphError::ConnectionFailed(format!("Missing config key: {PASSWORD_ENV_VAR}"))
        })?;

        Ok(Self {
            host,
            port,
            username: SecretSource::from_plain(username),
            password: SecretSource::from_plain(password),
        })
    }

    /// Builds a fresh [`ConnectionConfig`] from this typed config.
    ///
    /// Secrets are resolved via [`SecretSource::get`] at this point —
    /// call this **per top-level operation** so that host-rotated
    /// secrets take effect on the very next request.
    pub fn to_connection_config(&self) -> ConnectionConfig {
        let mut provider_config = vec![
            (HOST_ENV_VAR.to_string(), self.host.clone()),
            (USER_ENV_VAR.to_string(), self.username.get()),
            (PASSWORD_ENV_VAR.to_string(), self.password.get()),
        ];
        if let Some(port) = self.port {
            provider_config.push((PORT_ENV_VAR.to_string(), port.to_string()));
        }
        ConnectionConfig {
            hosts: Some(vec![self.host.clone()]),
            port: self.port,
            database_name: None,
            username: Some(self.username.get()),
            password: Some(self.password.get()),
            timeout_seconds: None,
            max_connections: None,
            provider_config,
        }
    }
}

/// Host-facing schema for Neo4j provider configuration. Only available
/// when the `golem` feature is enabled because it depends on the
/// `golem-rust` agent host bindings.
///
/// The secret fields are [`golem_rust::agentic::Secret`] handles that
/// are **not** materialized here. They are instead carried into the
/// runtime [`Neo4jConfig`] via the [`From`] impl below and resolved on
/// every outgoing request, so that hot-rotated host secrets take
/// effect immediately without restarting the worker.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct Neo4jHostConfig {
    pub host: String,
    pub port: Option<u16>,
    #[config_schema(secret)]
    pub username: golem_rust::agentic::Secret<String>,
    #[config_schema(secret)]
    pub password: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<Neo4jHostConfig> for Neo4jConfig {
    /// Converts a host-side schema config into a runtime
    /// [`Neo4jConfig`] **without** materializing the secret. The
    /// [`golem_rust::agentic::Secret`] handle is preserved inside the
    /// [`SecretSource`] so each outgoing request fetches the current
    /// value from the host.
    fn from(host: Neo4jHostConfig) -> Self {
        Self {
            host: host.host,
            port: host.port,
            username: SecretSource::from_handle(host.username),
            password: SecretSource::from_handle(host.password),
        }
    }
}

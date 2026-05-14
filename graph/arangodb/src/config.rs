//! Typed configuration types for the ArangoDB graph provider.
//!
//! These types are an alternative entry point to the existing
//! [`ConnectionConfig`]-based API. They let Golem-hosted callers
//! declare their config with `Secret<String>` fields and have those
//! resolved on demand for each outgoing request.
//!
//! There are two distinct types here:
//!
//! * [`ArangoDbConfig`] — the *runtime* configuration. Holds a
//!   [`SecretSource`] for the username and password so the secret
//!   value is fetched lazily on every outgoing request. Available in
//!   every build.
//!
//! * [`ArangoDbHostConfig`] — the *host-facing* schema type, derived
//!   with `golem_rust::ConfigSchema`. Its secret fields are
//!   `golem_rust::agentic::Secret<String>` handles that are fetched
//!   from the agent host on demand. Only available when the `golem`
//!   feature is on.
//!
//! Callers running on the Golem agent runtime obtain an
//! [`ArangoDbHostConfig`] via the agent host (e.g. by registering it
//! with `#[agent_config]`), then convert it into an [`ArangoDbConfig`]
//! with `ArangoDbConfig::from(host_config)`. The conversion does
//! **not** materialize the secret — the
//! [`golem_rust::agentic::Secret`] handle is preserved inside the
//! resulting [`ArangoDbConfig`] so that every outgoing request fetches
//! the current value from the host. This is what allows host-side
//! secret rotation to take effect on the next request without
//! restarting the worker.
//!
//! Callers outside Golem (or in tests) use [`ArangoDbConfig::from_env`]
//! to read configuration from environment variables exactly the same
//! way the previous version of the crate did. The legacy
//! `ARANGODB_*` prefix is also accepted as a fallback for each
//! variable.
//!
//! Use [`ArangoDbConfig::to_connection_config`] to materialize a
//! [`ConnectionConfig`] just before passing it to
//! `GraphProvider::connect` — this resolves all secrets with their
//! current host-side values.

use golem_ai_graph::config::SecretSource;
use golem_ai_graph::model::connection::ConnectionConfig;
use golem_ai_graph::model::errors::GraphError;
use std::env;

pub const HOST_ENV_VAR: &str = "ARANGO_HOST";
pub const PORT_ENV_VAR: &str = "ARANGO_PORT";
pub const USER_ENV_VAR: &str = "ARANGO_USER";
pub const PASSWORD_ENV_VAR: &str = "ARANGO_PASSWORD";
pub const DATABASE_ENV_VAR: &str = "ARANGO_DATABASE";

/// Runtime ArangoDB provider configuration.
///
/// The username and password are wrapped in [`SecretSource`], which
/// is resolved lazily right before each outgoing HTTP request. Do not
/// extract the underlying string and cache it across requests — doing
/// so would defeat host-side secret rotation.
#[derive(Clone, Debug)]
pub struct ArangoDbConfig {
    pub host: String,
    pub port: Option<u16>,
    pub database: Option<String>,
    /// ArangoDB username, fetched on demand right before each request.
    pub username: SecretSource,
    /// ArangoDB password, fetched on demand right before each request.
    pub password: SecretSource,
}

fn env_with_fallback(primary: &str, fallback: &str) -> Option<String> {
    env::var(primary).ok().or_else(|| env::var(fallback).ok())
}

impl ArangoDbConfig {
    /// Builds an [`ArangoDbConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    /// The legacy `ARANGODB_*` prefix is also accepted as a fallback
    /// for each variable.
    pub fn from_env() -> Result<Self, GraphError> {
        let host = env_with_fallback(HOST_ENV_VAR, "ARANGODB_HOST").ok_or_else(|| {
            GraphError::ConnectionFailed(format!("Missing config key: {HOST_ENV_VAR}"))
        })?;
        let port = env_with_fallback(PORT_ENV_VAR, "ARANGODB_PORT")
            .and_then(|p| p.parse().ok());
        let username = env_with_fallback(USER_ENV_VAR, "ARANGODB_USER").ok_or_else(|| {
            GraphError::ConnectionFailed(format!("Missing config key: {USER_ENV_VAR}"))
        })?;
        let password = env_with_fallback(PASSWORD_ENV_VAR, "ARANGODB_PASSWORD").ok_or_else(
            || GraphError::ConnectionFailed(format!("Missing config key: {PASSWORD_ENV_VAR}")),
        )?;
        let database = env_with_fallback(DATABASE_ENV_VAR, "ARANGODB_DATABASE");

        Ok(Self {
            host,
            port,
            database,
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
        if let Some(db) = &self.database {
            provider_config.push((DATABASE_ENV_VAR.to_string(), db.clone()));
        }
        ConnectionConfig {
            hosts: Some(vec![self.host.clone()]),
            port: self.port,
            database_name: self.database.clone(),
            username: Some(self.username.get()),
            password: Some(self.password.get()),
            timeout_seconds: None,
            max_connections: None,
            provider_config,
        }
    }
}

/// Host-facing schema for ArangoDB provider configuration. Only
/// available when the `golem` feature is enabled because it depends on
/// the `golem-rust` agent host bindings.
///
/// The secret fields are [`golem_rust::agentic::Secret`] handles that
/// are **not** materialized here. They are instead carried into the
/// runtime [`ArangoDbConfig`] via the [`From`] impl below and resolved
/// on every outgoing request, so that hot-rotated host secrets take
/// effect immediately without restarting the worker.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct ArangoDbHostConfig {
    pub host: String,
    pub port: Option<u16>,
    pub database: Option<String>,
    #[config_schema(secret)]
    pub username: golem_rust::agentic::Secret<String>,
    #[config_schema(secret)]
    pub password: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<ArangoDbHostConfig> for ArangoDbConfig {
    /// Converts a host-side schema config into a runtime
    /// [`ArangoDbConfig`] **without** materializing the secret. The
    /// [`golem_rust::agentic::Secret`] handle is preserved inside the
    /// [`SecretSource`] so each outgoing request fetches the current
    /// value from the host.
    fn from(host: ArangoDbHostConfig) -> Self {
        Self {
            host: host.host,
            port: host.port,
            database: host.database,
            username: SecretSource::from_handle(host.username),
            password: SecretSource::from_handle(host.password),
        }
    }
}

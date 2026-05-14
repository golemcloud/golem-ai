//! Configuration types for the pgvector provider.
//! See `llm/openai/src/config.rs` for the full design rationale.

use golem_ai_vector::config::{get_config_key, SecretSource};
use golem_ai_vector::model::types::VectorError;

pub const CONNECTION_STRING_ENV_VAR: &str = "PGVECTOR_CONNECTION_STRING";

/// Runtime pgvector provider configuration that the caller passes into
/// every provider call.
///
/// The connection string is wrapped in a [`SecretSource`] because
/// PostgreSQL connection strings typically embed the password (e.g.
/// `postgres://user:password@host/db`). It is resolved lazily right
/// before each outgoing request — do not extract the underlying string
/// and cache it across requests.
#[derive(Clone, Debug)]
pub struct PgvectorConfig {
    /// PostgreSQL connection string (treated as a secret because it
    /// usually embeds a password).
    pub connection_string: SecretSource,
}

impl PgvectorConfig {
    /// Builds a [`PgvectorConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    ///
    /// Returns an error if the required `PGVECTOR_CONNECTION_STRING` is
    /// not set.
    pub fn from_env() -> Result<Self, VectorError> {
        Ok(Self {
            connection_string: SecretSource::from_plain(get_config_key(
                CONNECTION_STRING_ENV_VAR,
            )?),
        })
    }
}

/// Host-facing schema for pgvector provider configuration. Only
/// available when the `golem` feature is enabled because it depends on
/// the `golem-rust` agent host bindings.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct PgvectorHostConfig {
    #[config_schema(secret)]
    pub connection_string: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<PgvectorHostConfig> for PgvectorConfig {
    fn from(host: PgvectorHostConfig) -> Self {
        Self {
            connection_string: SecretSource::from_handle(host.connection_string),
        }
    }
}

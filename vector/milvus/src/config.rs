//! Configuration types for the Milvus vector provider.
//! See `llm/openai/src/config.rs` for the full design rationale.

use golem_ai_vector::config::{get_config_key, get_config_key_or_none, SecretSource};
use golem_ai_vector::model::types::VectorError;

pub const URI_ENV_VAR: &str = "MILVUS_URI";
pub const TOKEN_ENV_VAR: &str = "MILVUS_TOKEN";
pub const DATABASE_ENV_VAR: &str = "MILVUS_DATABASE";

/// Runtime Milvus provider configuration that the caller passes into
/// every provider call.
///
/// The optional bearer `token` is wrapped in a [`SecretSource`], which is
/// resolved lazily right before each outgoing HTTP request. Do not extract
/// the underlying string and cache it across requests — doing so would
/// defeat host-side secret rotation.
#[derive(Clone, Debug)]
pub struct MilvusConfig {
    /// The Milvus base URI, e.g. `http://localhost:19530`.
    pub uri: String,
    /// Optional bearer token, fetched on demand right before each request.
    pub token: Option<SecretSource>,
    /// Optional database name (defaults to `_default` inside the client).
    pub database: Option<String>,
}

impl MilvusConfig {
    /// Builds a [`MilvusConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    ///
    /// Returns an error if the required `MILVUS_URI` is not set.
    pub fn from_env() -> Result<Self, VectorError> {
        Ok(Self {
            uri: get_config_key(URI_ENV_VAR)?,
            token: get_config_key_or_none(TOKEN_ENV_VAR).map(SecretSource::from_plain),
            database: get_config_key_or_none(DATABASE_ENV_VAR),
        })
    }
}

/// Host-facing schema for Milvus provider configuration. Only available
/// when the `golem` feature is enabled because it depends on the
/// `golem-rust` agent host bindings.
///
/// The `token` field is a required [`Secret<String>`] handle (the agent
/// host schema does not currently support `Option<Secret<String>>`); set
/// it to an empty value if your Milvus deployment does not require
/// authentication.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct MilvusHostConfig {
    pub uri: String,
    #[config_schema(secret)]
    pub token: golem_rust::agentic::Secret<String>,
    pub database: Option<String>,
}

#[cfg(feature = "golem")]
impl From<MilvusHostConfig> for MilvusConfig {
    fn from(host: MilvusHostConfig) -> Self {
        Self {
            uri: host.uri,
            token: Some(SecretSource::from_handle(host.token)),
            database: host.database,
        }
    }
}

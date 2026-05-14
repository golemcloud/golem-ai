//! Configuration types for the Pinecone vector provider.
//! See `llm/openai/src/config.rs` for the full design rationale.

use golem_ai_vector::config::{get_config_key, get_config_key_or_none, SecretSource};
use golem_ai_vector::model::types::VectorError;

pub const API_KEY_ENV_VAR: &str = "PINECONE_API_KEY";
pub const ENVIRONMENT_ENV_VAR: &str = "PINECONE_ENVIRONMENT";

/// Runtime Pinecone provider configuration that the caller passes into
/// every provider call.
///
/// The API key is wrapped in a [`SecretSource`], which is resolved
/// lazily right before each outgoing HTTP request. Do not extract the
/// underlying string and cache it across requests — doing so would
/// defeat host-side secret rotation.
#[derive(Clone, Debug)]
pub struct PineconeConfig {
    /// The Pinecone API key, fetched on demand right before each request.
    pub api_key: SecretSource,
    /// Optional Pinecone environment (e.g. `us-west1-gcp`).
    pub environment: Option<String>,
}

impl PineconeConfig {
    /// Builds a [`PineconeConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    ///
    /// Returns an error if the required `PINECONE_API_KEY` is not set.
    pub fn from_env() -> Result<Self, VectorError> {
        Ok(Self {
            api_key: SecretSource::from_plain(get_config_key(API_KEY_ENV_VAR)?),
            environment: get_config_key_or_none(ENVIRONMENT_ENV_VAR),
        })
    }
}

/// Host-facing schema for Pinecone provider configuration. Only
/// available when the `golem` feature is enabled because it depends on
/// the `golem-rust` agent host bindings.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct PineconeHostConfig {
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
    pub environment: Option<String>,
}

#[cfg(feature = "golem")]
impl From<PineconeHostConfig> for PineconeConfig {
    fn from(host: PineconeHostConfig) -> Self {
        Self {
            api_key: SecretSource::from_handle(host.api_key),
            environment: host.environment,
        }
    }
}

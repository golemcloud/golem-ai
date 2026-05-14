//! Configuration types for the Qdrant vector provider.
//! See `llm/openai/src/config.rs` for the full design rationale.

use golem_ai_vector::config::{get_config_key, get_config_key_or_none, SecretSource};
use golem_ai_vector::model::types::VectorError;

pub const URL_ENV_VAR: &str = "QDRANT_URL";
pub const API_KEY_ENV_VAR: &str = "QDRANT_API_KEY";

/// Runtime Qdrant provider configuration that the caller passes into
/// every provider call.
///
/// The optional `api_key` is wrapped in a [`SecretSource`], which is
/// resolved lazily right before each outgoing HTTP request. Do not
/// extract the underlying string and cache it across requests — doing
/// so would defeat host-side secret rotation.
#[derive(Clone, Debug)]
pub struct QdrantConfig {
    /// The Qdrant base URL, e.g. `http://localhost:6333`.
    pub url: String,
    /// Optional API key, fetched on demand right before each request.
    pub api_key: Option<SecretSource>,
}

impl QdrantConfig {
    /// Builds a [`QdrantConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    ///
    /// Returns an error if the required `QDRANT_URL` is not set.
    pub fn from_env() -> Result<Self, VectorError> {
        Ok(Self {
            url: get_config_key(URL_ENV_VAR)?,
            api_key: get_config_key_or_none(API_KEY_ENV_VAR).map(SecretSource::from_plain),
        })
    }
}

/// Host-facing schema for Qdrant provider configuration. Only available
/// when the `golem` feature is enabled because it depends on the
/// `golem-rust` agent host bindings.
///
/// The `api_key` field is a required [`Secret<String>`] handle (the
/// agent host schema does not currently support `Option<Secret<String>>`);
/// set it to an empty value if your Qdrant deployment does not require
/// authentication.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct QdrantHostConfig {
    pub url: String,
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<QdrantHostConfig> for QdrantConfig {
    fn from(host: QdrantHostConfig) -> Self {
        Self {
            url: host.url,
            api_key: Some(SecretSource::from_handle(host.api_key)),
        }
    }
}

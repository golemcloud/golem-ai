//! Configuration types for the OpenAI embed provider.
//! See `llm/openai/src/config.rs` for the full design rationale.

use golem_ai_embed::config::{get_config_key, get_config_key_or_none, SecretSource};
use golem_ai_embed::model::Error;

pub const API_KEY_ENV_VAR: &str = "OPENAI_API_KEY";
pub const BASE_URL_ENV_VAR: &str = "OPENAI_BASE_URL";

/// Runtime OpenAI embed provider configuration that the caller passes
/// into every provider call.
///
/// The API key is wrapped in a [`SecretSource`], which is resolved
/// lazily right before each outgoing HTTP request. Do not extract the
/// underlying string and cache it across requests — doing so would
/// defeat host-side secret rotation.
#[derive(Clone, Debug)]
pub struct OpenAiEmbedConfig {
    /// The OpenAI API key, fetched on demand right before each request.
    pub api_key: SecretSource,
    /// Optional base URL override (defaults to OpenAI's public endpoint
    /// when `None`).
    pub base_url: Option<String>,
}

impl OpenAiEmbedConfig {
    /// Builds an [`OpenAiEmbedConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    ///
    /// Returns an error if the required `OPENAI_API_KEY` is not set.
    pub fn from_env() -> Result<Self, Error> {
        Ok(Self {
            api_key: SecretSource::from_plain(get_config_key(API_KEY_ENV_VAR)?),
            base_url: get_config_key_or_none(BASE_URL_ENV_VAR),
        })
    }
}

/// Host-facing schema for OpenAI embed provider configuration. Only
/// available when the `golem` feature is enabled because it depends on
/// the `golem-rust` agent host bindings.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct OpenAiEmbedHostConfig {
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
    pub base_url: Option<String>,
}

#[cfg(feature = "golem")]
impl From<OpenAiEmbedHostConfig> for OpenAiEmbedConfig {
    fn from(host: OpenAiEmbedHostConfig) -> Self {
        Self {
            api_key: SecretSource::from_handle(host.api_key),
            base_url: host.base_url,
        }
    }
}

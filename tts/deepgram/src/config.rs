//! Configuration types for the Deepgram TTS provider.
//!
//! See `tts/tts/src/config.rs` for the design rationale.

use golem_ai_tts::config::{get_config_with_default, validate_config_key, SecretSource};
use golem_ai_tts::model::types::TtsError;

pub const API_KEY_ENV_VAR: &str = "DEEPGRAM_API_KEY";
pub const API_VERSION_ENV_VAR: &str = "DEEPGRAM_API_VERSION";
pub const MODEL_ENV_VAR: &str = "DEEPGRAM_MODEL";

/// Runtime Deepgram provider configuration that the caller passes into
/// every provider call.
///
/// The API key is wrapped in a [`SecretSource`], which is resolved
/// lazily right before each outgoing HTTP request.
#[derive(Clone, Debug)]
pub struct DeepgramConfig {
    /// The Deepgram API key, fetched on demand right before each request.
    pub api_key: SecretSource,
    /// API version (defaults to `"v1"`).
    pub api_version: String,
    /// Default model (e.g. `"aura-2-asteria-en"`).
    pub model: String,
}

impl DeepgramConfig {
    /// Builds a [`DeepgramConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    pub fn from_env() -> Result<Self, TtsError> {
        Ok(Self {
            api_key: SecretSource::from_plain(validate_config_key(API_KEY_ENV_VAR)?),
            api_version: get_config_with_default(API_VERSION_ENV_VAR, "v1"),
            model: get_config_with_default(MODEL_ENV_VAR, "aura-2-asteria-en"),
        })
    }
}

/// Host-facing schema for Deepgram provider configuration. Only available
/// when the `golem` feature is enabled.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct DeepgramHostConfig {
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
    pub api_version: String,
    pub model: String,
}

#[cfg(feature = "golem")]
impl From<DeepgramHostConfig> for DeepgramConfig {
    fn from(host: DeepgramHostConfig) -> Self {
        Self {
            api_key: SecretSource::from_handle(host.api_key),
            api_version: host.api_version,
            model: host.model,
        }
    }
}

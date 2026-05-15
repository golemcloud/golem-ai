//! Configuration types for the ElevenLabs TTS provider.
//!
//! See `tts/tts/src/config.rs` for the design rationale.

use golem_ai_tts::config::{get_config_with_default, validate_config_key, SecretSource};
use golem_ai_tts::model::types::TtsError;

pub const API_KEY_ENV_VAR: &str = "ELEVENLABS_API_KEY";
pub const MODEL_VERSION_ENV_VAR: &str = "ELEVENLABS_MODEL_VERSION";

const DEFAULT_MODEL_VERSION: &str = "eleven_multilingual_v2";

/// Runtime ElevenLabs provider configuration.
///
/// The API key is wrapped in a [`SecretSource`], which is resolved
/// lazily right before each outgoing HTTP request.
#[derive(Clone, Debug)]
pub struct ElevenLabsConfig {
    /// The ElevenLabs API key, fetched on demand right before each request.
    pub api_key: SecretSource,
    /// Default model version (e.g. `"eleven_multilingual_v2"`).
    pub model_version: String,
}

impl ElevenLabsConfig {
    /// Builds an [`ElevenLabsConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    pub fn from_env() -> Result<Self, TtsError> {
        Ok(Self {
            api_key: SecretSource::from_plain(validate_config_key(API_KEY_ENV_VAR)?),
            model_version: get_config_with_default(MODEL_VERSION_ENV_VAR, DEFAULT_MODEL_VERSION),
        })
    }
}

/// Host-facing schema for ElevenLabs provider configuration. Only available
/// when the `golem` feature is enabled.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct ElevenLabsHostConfig {
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
    pub model_version: String,
}

#[cfg(feature = "golem")]
impl From<ElevenLabsHostConfig> for ElevenLabsConfig {
    fn from(host: ElevenLabsHostConfig) -> Self {
        Self {
            api_key: SecretSource::from_handle(host.api_key),
            model_version: host.model_version,
        }
    }
}

//! Configuration types for the OpenAI Whisper provider.
//!
//! There are two distinct types here:
//!
//! * [`WhisperConfig`] — the *runtime* configuration that the provider
//!   implementation actually consumes. It holds a [`SecretSource`] for
//!   the API key (so the secret value is fetched lazily on every
//!   outgoing request) and is the value that the caller passes into
//!   `TranscriptionProvider::transcribe` / `transcribe_many`.
//!   Available in every build.
//!
//! * [`WhisperHostConfig`] — the *host-facing* schema type, derived with
//!   `golem_rust::ConfigSchema`. Its `api_key` field is a
//!   `golem_rust::agentic::Secret<String>` handle that is fetched from
//!   the agent host on demand. Only available when the `golem` feature
//!   is on.
//!
//! See `llm/openai/src/config.rs` for the full design rationale.

use golem_ai_stt::config::{get_config_key, SecretSource};
use golem_ai_stt::model::types::SttError;

pub const API_KEY_ENV_VAR: &str = "OPENAI_API_KEY";

/// Runtime Whisper provider configuration that the caller passes into
/// every provider call.
///
/// The API key is wrapped in a [`SecretSource`], which is resolved
/// lazily right before each outgoing HTTP request. Do not extract the
/// underlying string and cache it across requests — doing so would
/// defeat host-side secret rotation.
#[derive(Clone, Debug)]
pub struct WhisperConfig {
    /// The OpenAI API key, fetched on demand right before each request.
    pub api_key: SecretSource,
}

impl WhisperConfig {
    /// Builds a [`WhisperConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    ///
    /// Returns an error if the required `OPENAI_API_KEY` is not set.
    pub fn from_env() -> Result<Self, SttError> {
        Ok(Self {
            api_key: SecretSource::from_plain(get_config_key(API_KEY_ENV_VAR)?),
        })
    }
}

/// Host-facing schema for Whisper provider configuration. Only available
/// when the `golem` feature is enabled because it depends on the
/// `golem-rust` agent host bindings.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct WhisperHostConfig {
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<WhisperHostConfig> for WhisperConfig {
    fn from(host: WhisperHostConfig) -> Self {
        Self {
            api_key: SecretSource::from_handle(host.api_key),
        }
    }
}

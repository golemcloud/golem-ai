//! Configuration types for the Deepgram Speech-to-Text provider.
//!
//! See `llm/openai/src/config.rs` for the design rationale of the
//! two-config-types pattern (runtime + host-facing).

use golem_ai_stt::config::{get_config_key, get_config_key_or_none, SecretSource};
use golem_ai_stt::model::types::SttError;

pub const API_TOKEN_ENV_VAR: &str = "DEEPGRAM_API_TOKEN";
pub const ENDPOINT_ENV_VAR: &str = "DEEPGRAM_ENDPOINT";

const DEFAULT_ENDPOINT: &str = "https://api.deepgram.com/v1/listen";

/// Runtime Deepgram provider configuration that the caller passes into
/// every provider call.
///
/// The API key is wrapped in a [`SecretSource`], which is resolved
/// lazily right before each outgoing HTTP request. The endpoint is a
/// plain [`String`] because it is not a secret and does not rotate.
#[derive(Clone, Debug)]
pub struct DeepgramConfig {
    /// Deepgram API token, fetched on demand right before each request.
    pub api_key: SecretSource,
    /// Deepgram endpoint URL (defaults to the public Deepgram API).
    pub endpoint: String,
}

impl DeepgramConfig {
    /// Builds a [`DeepgramConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    ///
    /// `DEEPGRAM_API_TOKEN` is required. `DEEPGRAM_ENDPOINT` defaults
    /// to `https://api.deepgram.com/v1/listen` if not set.
    pub fn from_env() -> Result<Self, SttError> {
        Ok(Self {
            api_key: SecretSource::from_plain(get_config_key(API_TOKEN_ENV_VAR)?),
            endpoint: get_config_key_or_none(ENDPOINT_ENV_VAR)
                .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string()),
        })
    }
}

/// Host-facing schema for Deepgram provider configuration. Only available
/// when the `golem` feature is enabled because it depends on the
/// `golem-rust` agent host bindings.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct DeepgramHostConfig {
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
    pub endpoint: String,
}

#[cfg(feature = "golem")]
impl From<DeepgramHostConfig> for DeepgramConfig {
    fn from(host: DeepgramHostConfig) -> Self {
        Self {
            api_key: SecretSource::from_handle(host.api_key),
            endpoint: host.endpoint,
        }
    }
}

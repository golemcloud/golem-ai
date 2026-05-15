//! Configuration types for the Stability AI provider.
//!
//! See `llm/openai/src/config.rs` for the full design rationale.

use golem_ai_video::config::{get_config_key, SecretSource};
use golem_ai_video::model::types::VideoError;

pub const API_KEY_ENV_VAR: &str = "STABILITY_API_KEY";

/// Runtime Stability provider configuration that the caller passes into
/// every provider call.
///
/// The API key is wrapped in a [`SecretSource`], which is resolved
/// lazily right before each outgoing HTTP request. Do not extract the
/// underlying string and cache it across requests — doing so would
/// defeat host-side secret rotation.
#[derive(Clone, Debug)]
pub struct StabilityConfig {
    /// The Stability API key, fetched on demand right before each request.
    pub api_key: SecretSource,
}

impl StabilityConfig {
    /// Builds a [`StabilityConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    ///
    /// Returns an error if the required `STABILITY_API_KEY` is not set.
    pub fn from_env() -> Result<Self, VideoError> {
        Ok(Self {
            api_key: SecretSource::from_plain(get_config_key(API_KEY_ENV_VAR)?),
        })
    }
}

/// Host-facing schema for Stability provider configuration. Only available
/// when the `golem` feature is enabled because it depends on the
/// `golem-rust` agent host bindings.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct StabilityHostConfig {
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<StabilityHostConfig> for StabilityConfig {
    fn from(host: StabilityHostConfig) -> Self {
        Self {
            api_key: SecretSource::from_handle(host.api_key),
        }
    }
}

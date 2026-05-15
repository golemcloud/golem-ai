//! Configuration types for the VoyageAI embed provider.
//! See `llm/openai/src/config.rs` for the full design rationale.

use golem_ai_embed::config::{get_config_key, SecretSource};
use golem_ai_embed::model::Error;

pub const API_KEY_ENV_VAR: &str = "VOYAGEAI_API_KEY";

#[derive(Clone, Debug)]
pub struct VoyageAiConfig {
    pub api_key: SecretSource,
}

impl VoyageAiConfig {
    pub fn from_env() -> Result<Self, Error> {
        Ok(Self {
            api_key: SecretSource::from_plain(get_config_key(API_KEY_ENV_VAR)?),
        })
    }
}

#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct VoyageAiHostConfig {
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<VoyageAiHostConfig> for VoyageAiConfig {
    fn from(host: VoyageAiHostConfig) -> Self {
        Self {
            api_key: SecretSource::from_handle(host.api_key),
        }
    }
}

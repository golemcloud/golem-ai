//! Configuration types for the OpenRouter provider.
//! See `llm/openai/src/config.rs` for the full design rationale.

use golem_ai_llm::config::{get_config_key, SecretSource};
use golem_ai_llm::model::Error;

pub const API_KEY_ENV_VAR: &str = "OPENROUTER_API_KEY";

#[derive(Clone, Debug)]
pub struct OpenRouterConfig {
    pub api_key: SecretSource,
}

impl OpenRouterConfig {
    pub fn from_env() -> Result<Self, Error> {
        Ok(Self {
            api_key: SecretSource::from_plain(get_config_key(API_KEY_ENV_VAR)?),
        })
    }
}

#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct OpenRouterHostConfig {
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<OpenRouterHostConfig> for OpenRouterConfig {
    fn from(host: OpenRouterHostConfig) -> Self {
        Self {
            api_key: SecretSource::from_handle(host.api_key),
        }
    }
}

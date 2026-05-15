//! Configuration types for the Cohere embed provider.
//! See `llm/openai/src/config.rs` for the full design rationale.

use golem_ai_embed::config::{get_config_key, SecretSource};
use golem_ai_embed::model::Error;

pub const API_KEY_ENV_VAR: &str = "COHERE_API_KEY";

#[derive(Clone, Debug)]
pub struct CohereConfig {
    pub api_key: SecretSource,
}

impl CohereConfig {
    pub fn from_env() -> Result<Self, Error> {
        Ok(Self {
            api_key: SecretSource::from_plain(get_config_key(API_KEY_ENV_VAR)?),
        })
    }
}

#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct CohereHostConfig {
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<CohereHostConfig> for CohereConfig {
    fn from(host: CohereHostConfig) -> Self {
        Self {
            api_key: SecretSource::from_handle(host.api_key),
        }
    }
}

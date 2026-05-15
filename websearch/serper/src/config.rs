//! Configuration types for the Serper web search provider.
//! See `llm/openai/src/config.rs` for the full design rationale.

use golem_ai_web_search::config::{get_config_key, SecretSource};
use golem_ai_web_search::model::web_search::SearchError;

pub const API_KEY_ENV_VAR: &str = "SERPER_API_KEY";

#[derive(Clone, Debug)]
pub struct SerperConfig {
    pub api_key: SecretSource,
}

impl SerperConfig {
    pub fn from_env() -> Result<Self, SearchError> {
        Ok(Self {
            api_key: SecretSource::from_plain(get_config_key(API_KEY_ENV_VAR)?),
        })
    }
}

#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct SerperHostConfig {
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<SerperHostConfig> for SerperConfig {
    fn from(host: SerperHostConfig) -> Self {
        Self {
            api_key: SecretSource::from_handle(host.api_key),
        }
    }
}

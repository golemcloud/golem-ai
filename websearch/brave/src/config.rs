//! Configuration types for the Brave web search provider.
//! See `llm/openai/src/config.rs` for the full design rationale.

use golem_ai_web_search::config::{get_config_key, SecretSource};
use golem_ai_web_search::model::web_search::SearchError;

pub const API_KEY_ENV_VAR: &str = "BRAVE_API_KEY";

#[derive(Clone, Debug)]
pub struct BraveConfig {
    pub api_key: SecretSource,
}

impl BraveConfig {
    pub fn from_env() -> Result<Self, SearchError> {
        Ok(Self {
            api_key: SecretSource::from_plain(get_config_key(API_KEY_ENV_VAR)?),
        })
    }
}

#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct BraveHostConfig {
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<BraveHostConfig> for BraveConfig {
    fn from(host: BraveHostConfig) -> Self {
        Self {
            api_key: SecretSource::from_handle(host.api_key),
        }
    }
}

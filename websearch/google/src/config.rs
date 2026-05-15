//! Configuration types for the Google Custom Search provider.
//! See `llm/openai/src/config.rs` for the full design rationale.

use golem_ai_web_search::config::{get_config_key, SecretSource};
use golem_ai_web_search::model::web_search::SearchError;

pub const API_KEY_ENV_VAR: &str = "GOOGLE_API_KEY";
pub const SEARCH_ENGINE_ID_ENV_VAR: &str = "GOOGLE_SEARCH_ENGINE_ID";

#[derive(Clone, Debug)]
pub struct GoogleConfig {
    pub api_key: SecretSource,
    pub search_engine_id: String,
}

impl GoogleConfig {
    pub fn from_env() -> Result<Self, SearchError> {
        Ok(Self {
            api_key: SecretSource::from_plain(get_config_key(API_KEY_ENV_VAR)?),
            search_engine_id: get_config_key(SEARCH_ENGINE_ID_ENV_VAR)?,
        })
    }
}

#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct GoogleHostConfig {
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
    pub search_engine_id: String,
}

#[cfg(feature = "golem")]
impl From<GoogleHostConfig> for GoogleConfig {
    fn from(host: GoogleHostConfig) -> Self {
        Self {
            api_key: SecretSource::from_handle(host.api_key),
            search_engine_id: host.search_engine_id,
        }
    }
}

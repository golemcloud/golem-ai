//! Configuration types for the Algolia search provider.
//! See `llm/openai/src/config.rs` for the full design rationale.

use golem_ai_search::config::{get_config_key, SecretSource};
use golem_ai_search::model::SearchError;

pub const APPLICATION_ID_ENV_VAR: &str = "ALGOLIA_APPLICATION_ID";
pub const API_KEY_ENV_VAR: &str = "ALGOLIA_API_KEY";

#[derive(Clone, Debug)]
pub struct AlgoliaConfig {
    pub application_id: String,
    pub api_key: SecretSource,
}

impl AlgoliaConfig {
    pub fn from_env() -> Result<Self, SearchError> {
        Ok(Self {
            application_id: get_config_key(APPLICATION_ID_ENV_VAR)?,
            api_key: SecretSource::from_plain(get_config_key(API_KEY_ENV_VAR)?),
        })
    }
}

#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct AlgoliaHostConfig {
    pub application_id: String,
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<AlgoliaHostConfig> for AlgoliaConfig {
    fn from(host: AlgoliaHostConfig) -> Self {
        Self {
            application_id: host.application_id,
            api_key: SecretSource::from_handle(host.api_key),
        }
    }
}

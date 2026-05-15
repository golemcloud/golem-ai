//! Configuration types for the Typesense search provider.
//! See `llm/openai/src/config.rs` for the full design rationale.

use golem_ai_search::config::{get_config_key, SecretSource};
use golem_ai_search::model::SearchError;

pub const API_KEY_ENV_VAR: &str = "TYPESENSE_API_KEY";
pub const BASE_URL_ENV_VAR: &str = "TYPESENSE_BASE_URL";

#[derive(Clone, Debug)]
pub struct TypesenseConfig {
    pub api_key: SecretSource,
    pub base_url: String,
}

impl TypesenseConfig {
    pub fn from_env() -> Result<Self, SearchError> {
        Ok(Self {
            api_key: SecretSource::from_plain(get_config_key(API_KEY_ENV_VAR)?),
            base_url: get_config_key(BASE_URL_ENV_VAR)?,
        })
    }
}

#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct TypesenseHostConfig {
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
    pub base_url: String,
}

#[cfg(feature = "golem")]
impl From<TypesenseHostConfig> for TypesenseConfig {
    fn from(host: TypesenseHostConfig) -> Self {
        Self {
            api_key: SecretSource::from_handle(host.api_key),
            base_url: host.base_url,
        }
    }
}

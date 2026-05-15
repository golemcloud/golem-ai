//! Configuration types for the OpenSearch search provider.
//! See `llm/openai/src/config.rs` for the full design rationale.
//!
//! NOTE: All authentication-related fields (`username`, `password`,
//! `api_key`) are optional in [`OpenSearchConfig`] because OpenSearch
//! can run without auth, with basic auth, or with API keys. The
//! host-side schema [`OpenSearchHostConfig`] requires all three secrets
//! because the `golem_rust::ConfigSchema` derive does not currently
//! support `Option<Secret<String>>`. Callers using the agent host
//! should provide all three values; unused ones can be empty strings.

use golem_ai_search::config::{get_config_key, get_config_key_or_none, SecretSource};
use golem_ai_search::model::SearchError;

pub const BASE_URL_ENV_VAR: &str = "OPENSEARCH_BASE_URL";
pub const USERNAME_ENV_VAR: &str = "OPENSEARCH_USERNAME";
pub const PASSWORD_ENV_VAR: &str = "OPENSEARCH_PASSWORD";
pub const API_KEY_ENV_VAR: &str = "OPENSEARCH_API_KEY";

#[derive(Clone, Debug)]
pub struct OpenSearchConfig {
    pub base_url: String,
    pub username: Option<SecretSource>,
    pub password: Option<SecretSource>,
    pub api_key: Option<SecretSource>,
}

impl OpenSearchConfig {
    pub fn from_env() -> Result<Self, SearchError> {
        Ok(Self {
            base_url: get_config_key(BASE_URL_ENV_VAR)?,
            username: get_config_key_or_none(USERNAME_ENV_VAR).map(SecretSource::from_plain),
            password: get_config_key_or_none(PASSWORD_ENV_VAR).map(SecretSource::from_plain),
            api_key: get_config_key_or_none(API_KEY_ENV_VAR).map(SecretSource::from_plain),
        })
    }
}

#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct OpenSearchHostConfig {
    pub base_url: String,
    #[config_schema(secret)]
    pub username: golem_rust::agentic::Secret<String>,
    #[config_schema(secret)]
    pub password: golem_rust::agentic::Secret<String>,
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<OpenSearchHostConfig> for OpenSearchConfig {
    fn from(host: OpenSearchHostConfig) -> Self {
        Self {
            base_url: host.base_url,
            username: Some(SecretSource::from_handle(host.username)),
            password: Some(SecretSource::from_handle(host.password)),
            api_key: Some(SecretSource::from_handle(host.api_key)),
        }
    }
}

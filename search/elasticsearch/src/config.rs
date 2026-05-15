//! Configuration types for the Elasticsearch search provider.
//! See `llm/openai/src/config.rs` for the full design rationale.
//!
//! NOTE: All authentication-related fields (`username`, `password`,
//! `api_key`) are optional in [`ElasticsearchConfig`] because
//! Elasticsearch can run without auth, with basic auth, or with API
//! keys. The host-side schema [`ElasticsearchHostConfig`] requires all
//! three secrets because the `golem_rust::ConfigSchema` derive does
//! not currently support `Option<Secret<String>>`. Callers using the
//! agent host should provide all three values; unused ones can be
//! empty strings (the runtime treats them as set, but Elasticsearch
//! will simply use whichever scheme matches).

use golem_ai_search::config::{get_config_key, get_config_key_or_none, SecretSource};
use golem_ai_search::model::SearchError;

pub const URL_ENV_VAR: &str = "ELASTICSEARCH_URL";
pub const USERNAME_ENV_VAR: &str = "ELASTICSEARCH_USERNAME";
pub const PASSWORD_ENV_VAR: &str = "ELASTICSEARCH_PASSWORD";
pub const API_KEY_ENV_VAR: &str = "ELASTICSEARCH_API_KEY";

#[derive(Clone, Debug)]
pub struct ElasticsearchConfig {
    pub url: String,
    pub username: Option<SecretSource>,
    pub password: Option<SecretSource>,
    pub api_key: Option<SecretSource>,
}

impl ElasticsearchConfig {
    pub fn from_env() -> Result<Self, SearchError> {
        Ok(Self {
            url: get_config_key(URL_ENV_VAR)?,
            username: get_config_key_or_none(USERNAME_ENV_VAR).map(SecretSource::from_plain),
            password: get_config_key_or_none(PASSWORD_ENV_VAR).map(SecretSource::from_plain),
            api_key: get_config_key_or_none(API_KEY_ENV_VAR).map(SecretSource::from_plain),
        })
    }
}

#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct ElasticsearchHostConfig {
    pub url: String,
    #[config_schema(secret)]
    pub username: golem_rust::agentic::Secret<String>,
    #[config_schema(secret)]
    pub password: golem_rust::agentic::Secret<String>,
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<ElasticsearchHostConfig> for ElasticsearchConfig {
    fn from(host: ElasticsearchHostConfig) -> Self {
        Self {
            url: host.url,
            username: Some(SecretSource::from_handle(host.username)),
            password: Some(SecretSource::from_handle(host.password)),
            api_key: Some(SecretSource::from_handle(host.api_key)),
        }
    }
}

//! Configuration types for the Meilisearch search provider.
//! See `llm/openai/src/config.rs` for the full design rationale.
//!
//! NOTE: `api_key` is optional in [`MeilisearchConfig`] because
//! Meilisearch can run without authentication. The host-side schema
//! [`MeilisearchHostConfig`] requires it because the
//! `golem_rust::ConfigSchema` derive does not currently support
//! `Option<Secret<String>>`.

use golem_ai_search::config::{get_config_key, get_config_key_or_none, SecretSource};
use golem_ai_search::model::SearchError;

pub const BASE_URL_ENV_VAR: &str = "MEILISEARCH_BASE_URL";
pub const API_KEY_ENV_VAR: &str = "MEILISEARCH_API_KEY";

#[derive(Clone, Debug)]
pub struct MeilisearchConfig {
    pub base_url: String,
    pub api_key: Option<SecretSource>,
}

impl MeilisearchConfig {
    pub fn from_env() -> Result<Self, SearchError> {
        Ok(Self {
            base_url: get_config_key(BASE_URL_ENV_VAR)?,
            api_key: get_config_key_or_none(API_KEY_ENV_VAR).map(SecretSource::from_plain),
        })
    }
}

#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct MeilisearchHostConfig {
    pub base_url: String,
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<MeilisearchHostConfig> for MeilisearchConfig {
    fn from(host: MeilisearchHostConfig) -> Self {
        Self {
            base_url: host.base_url,
            api_key: Some(SecretSource::from_handle(host.api_key)),
        }
    }
}

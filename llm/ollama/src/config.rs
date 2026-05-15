//! Configuration types for the Ollama provider.
//!
//! Ollama has no API key; only a base URL. The `Option<String>` lets
//! callers fall back to the standard `http://localhost:11434` default
//! by passing `None`.

use golem_ai_llm::config::get_config_key_or_none;
use golem_ai_llm::model::Error;

pub const BASE_URL_ENV_VAR: &str = "GOLEM_OLLAMA_BASE_URL";
pub const DEFAULT_BASE_URL: &str = "http://localhost:11434";

#[derive(Clone, Debug)]
pub struct OllamaConfig {
    /// Base URL for the Ollama server (defaults to
    /// `http://localhost:11434` when `None`).
    pub base_url: Option<String>,
}

impl OllamaConfig {
    /// Reads `GOLEM_OLLAMA_BASE_URL` from the environment. Always
    /// succeeds; missing env var simply yields `None`, which is later
    /// resolved to the standard default.
    pub fn from_env() -> Result<Self, Error> {
        Ok(Self {
            base_url: get_config_key_or_none(BASE_URL_ENV_VAR),
        })
    }
}

#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct OllamaHostConfig {
    pub base_url: Option<String>,
}

#[cfg(feature = "golem")]
impl From<OllamaHostConfig> for OllamaConfig {
    fn from(host: OllamaHostConfig) -> Self {
        Self {
            base_url: host.base_url,
        }
    }
}

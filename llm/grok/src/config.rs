//! Configuration types for the Grok (xAI) provider.
//!
//! See `llm/openai/src/config.rs` for the full design rationale.
//! Two types:
//!
//! * `GrokConfig` — runtime config holding a `SecretSource`. Available always.
//! * `GrokHostConfig` — host-facing schema with `Secret<String>`, derives
//!   `golem_rust::ConfigSchema`. Only with `feature = "golem"`. Convert via
//!   `From<GrokHostConfig> for GrokConfig` — the `Secret<String>` handle is
//!   preserved (NOT materialized) so each outgoing request fetches the
//!   current value via `.get()`.

use golem_ai_llm::config::{get_config_key, SecretSource};
use golem_ai_llm::model::Error;

pub const API_KEY_ENV_VAR: &str = "XAI_API_KEY";

#[derive(Clone, Debug)]
pub struct GrokConfig {
    pub api_key: SecretSource,
}

impl GrokConfig {
    pub fn from_env() -> Result<Self, Error> {
        Ok(Self {
            api_key: SecretSource::from_plain(get_config_key(API_KEY_ENV_VAR)?),
        })
    }
}

#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct GrokHostConfig {
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<GrokHostConfig> for GrokConfig {
    fn from(host: GrokHostConfig) -> Self {
        Self {
            api_key: SecretSource::from_handle(host.api_key),
        }
    }
}

//! Configuration types for the Anthropic provider.
//!
//! There are two distinct types here:
//!
//! * [`AnthropicConfig`] — the *runtime* configuration that the
//!   provider implementation actually consumes. It holds a
//!   [`SecretSource`] for the API key (so the secret value is fetched
//!   lazily on every outgoing request) and is the value that the
//!   caller passes into `LlmProvider::send` / `LlmProvider::stream`.
//!   Available in every build.
//!
//! * [`AnthropicHostConfig`] — the *host-facing* schema type, derived
//!   with `golem_rust::ConfigSchema`. Its `api_key` field is a
//!   `golem_rust::agentic::Secret<String>` handle that is fetched from
//!   the agent host on demand. Only available when the `golem` feature
//!   is on.
//!
//! Callers running on the Golem agent runtime obtain an
//! [`AnthropicHostConfig`] via the agent host (e.g. by registering it
//! with `#[agent_config]`), then convert it into an [`AnthropicConfig`]
//! with `AnthropicConfig::from(host_config)`. The conversion does
//! **not** materialize the secret — the [`Secret<String>`] handle is
//! preserved inside the resulting [`AnthropicConfig`] so that every
//! outgoing HTTP request fetches the current value from the host. This
//! is what allows host-side secret rotation to take effect on the next
//! request without restarting the worker.
//!
//! Callers outside Golem (or in tests) use [`AnthropicConfig::from_env`]
//! to read configuration from environment variables exactly the same
//! way the previous version of the crate did.

use golem_ai_llm::config::{get_config_key, SecretSource};
use golem_ai_llm::model::Error;

pub const API_KEY_ENV_VAR: &str = "ANTHROPIC_API_KEY";

/// Runtime Anthropic provider configuration that the caller passes into
/// every provider call.
///
/// The API key is wrapped in a [`SecretSource`], which is resolved
/// lazily right before each outgoing HTTP request. Do not extract the
/// underlying string and cache it across requests — doing so would
/// defeat host-side secret rotation.
#[derive(Clone, Debug)]
pub struct AnthropicConfig {
    /// The Anthropic API key, fetched on demand right before each request.
    pub api_key: SecretSource,
}

impl AnthropicConfig {
    /// Builds an [`AnthropicConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    ///
    /// Returns an error if the required `ANTHROPIC_API_KEY` is not set.
    pub fn from_env() -> Result<Self, Error> {
        Ok(Self {
            api_key: SecretSource::from_plain(get_config_key(API_KEY_ENV_VAR)?),
        })
    }
}

/// Host-facing schema for Anthropic provider configuration. Only
/// available when the `golem` feature is enabled because it depends on
/// the `golem-rust` agent host bindings.
///
/// The `api_key` field is a [`Secret<String>`] handle that is **not**
/// materialized here. It is instead carried into the runtime
/// [`AnthropicConfig`] via the [`From`] impl below and resolved on
/// every outgoing request, so that hot-rotated host secrets take
/// effect immediately without restarting the worker.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct AnthropicHostConfig {
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<AnthropicHostConfig> for AnthropicConfig {
    /// Converts a host-side schema config into a runtime
    /// [`AnthropicConfig`] **without** materializing the secret. The
    /// [`Secret<String>`] handle is preserved inside the
    /// [`SecretSource`] so each outgoing request fetches the current
    /// value from the host.
    fn from(host: AnthropicHostConfig) -> Self {
        Self {
            api_key: SecretSource::from_handle(host.api_key),
        }
    }
}

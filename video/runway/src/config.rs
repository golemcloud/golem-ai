//! Configuration types for the Runway provider.
//!
//! There are two distinct types here:
//!
//! * [`RunwayConfig`] — the *runtime* configuration that the provider
//!   implementation actually consumes. It holds a [`SecretSource`] for
//!   the API key (so the secret value is fetched lazily on every
//!   outgoing request) and is the value that the caller passes into
//!   `VideoGenerationProvider`/`LipSyncProvider`/`AdvancedVideoGenerationProvider`
//!   methods. Available in every build.
//!
//! * [`RunwayHostConfig`] — the *host-facing* schema type, derived with
//!   `golem_rust::ConfigSchema`. Its `api_key` field is a
//!   `golem_rust::agentic::Secret<String>` handle that is fetched from
//!   the agent host on demand. Only available when the `golem` feature
//!   is on.
//!
//! See `llm/openai/src/config.rs` for the full design rationale.

use golem_ai_video::config::{get_config_key, SecretSource};
use golem_ai_video::model::types::VideoError;

pub const API_KEY_ENV_VAR: &str = "RUNWAY_API_KEY";

/// Runtime Runway provider configuration that the caller passes into
/// every provider call.
///
/// The API key is wrapped in a [`SecretSource`], which is resolved
/// lazily right before each outgoing HTTP request. Do not extract the
/// underlying string and cache it across requests — doing so would
/// defeat host-side secret rotation.
#[derive(Clone, Debug)]
pub struct RunwayConfig {
    /// The Runway API key, fetched on demand right before each request.
    pub api_key: SecretSource,
}

impl RunwayConfig {
    /// Builds a [`RunwayConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    ///
    /// Returns an error if the required `RUNWAY_API_KEY` is not set.
    pub fn from_env() -> Result<Self, VideoError> {
        Ok(Self {
            api_key: SecretSource::from_plain(get_config_key(API_KEY_ENV_VAR)?),
        })
    }
}

/// Host-facing schema for Runway provider configuration. Only available
/// when the `golem` feature is enabled because it depends on the
/// `golem-rust` agent host bindings.
///
/// The `api_key` field is a [`Secret<String>`] handle that is **not**
/// materialized here. It is instead carried into the runtime
/// [`RunwayConfig`] via the [`From`] impl below and resolved on every
/// outgoing request, so that hot-rotated host secrets take effect
/// immediately without restarting the worker.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct RunwayHostConfig {
    #[config_schema(secret)]
    pub api_key: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<RunwayHostConfig> for RunwayConfig {
    /// Converts a host-side schema config into a runtime
    /// [`RunwayConfig`] **without** materializing the secret. The
    /// [`Secret<String>`] handle is preserved inside the
    /// [`SecretSource`] so each outgoing request fetches the current
    /// value from the host.
    fn from(host: RunwayHostConfig) -> Self {
        Self {
            api_key: SecretSource::from_handle(host.api_key),
        }
    }
}

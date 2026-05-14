//! Configuration types for the Kling provider.
//!
//! Kling uses two secrets (an access key and a secret key) that are
//! combined client-side to sign JWT tokens before each request. They
//! are held as [`SecretSource`]s so they are resolved on demand right
//! before each top-level provider call.
//!
//! See `llm/openai/src/config.rs` for the full design rationale.

use golem_ai_video::config::{get_config_key, SecretSource};
use golem_ai_video::model::types::VideoError;

pub const ACCESS_KEY_ENV_VAR: &str = "KLING_ACCESS_KEY";
pub const SECRET_KEY_ENV_VAR: &str = "KLING_SECRET_KEY";

/// Runtime Kling provider configuration that the caller passes into
/// every provider call.
///
/// Both keys are wrapped in [`SecretSource`]s, which are resolved
/// lazily right before each outgoing JWT signing operation. Do not
/// extract the underlying strings and cache them across requests —
/// doing so would defeat host-side secret rotation.
#[derive(Clone, Debug)]
pub struct KlingConfig {
    /// The Kling access key, fetched on demand right before each request.
    pub access_key: SecretSource,
    /// The Kling secret key, fetched on demand right before each request.
    pub secret_key: SecretSource,
}

impl KlingConfig {
    /// Builds a [`KlingConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    ///
    /// Returns an error if any of the required `KLING_ACCESS_KEY` or
    /// `KLING_SECRET_KEY` environment variables are not set.
    pub fn from_env() -> Result<Self, VideoError> {
        Ok(Self {
            access_key: SecretSource::from_plain(get_config_key(ACCESS_KEY_ENV_VAR)?),
            secret_key: SecretSource::from_plain(get_config_key(SECRET_KEY_ENV_VAR)?),
        })
    }
}

/// Host-facing schema for Kling provider configuration. Only available
/// when the `golem` feature is enabled because it depends on the
/// `golem-rust` agent host bindings.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct KlingHostConfig {
    #[config_schema(secret)]
    pub access_key: golem_rust::agentic::Secret<String>,
    #[config_schema(secret)]
    pub secret_key: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<KlingHostConfig> for KlingConfig {
    fn from(host: KlingHostConfig) -> Self {
        Self {
            access_key: SecretSource::from_handle(host.access_key),
            secret_key: SecretSource::from_handle(host.secret_key),
        }
    }
}

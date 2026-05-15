//! Configuration types for the Amazon Bedrock provider.
//!
//! Bedrock requires AWS credentials (access key id, secret access
//! key, optional session token) and a region. The credentials are
//! held as [`SecretSource`]s so they are resolved on demand right
//! before each top-level provider call. The region is a plain
//! `String` because it is not a secret and does not rotate.
//!
//! See `llm/openai/src/config.rs` for the full design rationale.

use golem_ai_llm::config::{get_config_key, get_config_key_or_none, SecretSource};
use golem_ai_llm::model::Error;

pub const ACCESS_KEY_ID_ENV_VAR: &str = "AWS_ACCESS_KEY_ID";
pub const SECRET_ACCESS_KEY_ENV_VAR: &str = "AWS_SECRET_ACCESS_KEY";
pub const SESSION_TOKEN_ENV_VAR: &str = "AWS_SESSION_TOKEN";
pub const REGION_ENV_VAR: &str = "AWS_REGION";

/// Runtime Bedrock provider configuration that the caller passes into
/// every provider call.
///
/// The AWS credentials are wrapped in [`SecretSource`]s, which are
/// resolved lazily right before each outgoing request. The region is a
/// plain [`String`] because it is not a secret and does not rotate.
///
/// Do not extract the underlying credential strings and cache them
/// across requests — doing so would defeat host-side secret rotation.
#[derive(Clone, Debug)]
pub struct BedrockConfig {
    /// The AWS access key id, fetched on demand right before each request.
    pub access_key_id: SecretSource,
    /// The AWS secret access key, fetched on demand right before each request.
    pub secret_access_key: SecretSource,
    /// Optional AWS session token (only required when using temporary
    /// credentials), fetched on demand right before each request.
    pub session_token: Option<SecretSource>,
    /// AWS region name (e.g. `"us-east-1"`).
    pub region: String,
}

impl BedrockConfig {
    /// Builds a [`BedrockConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    ///
    /// Returns an error if any of the required `AWS_ACCESS_KEY_ID`,
    /// `AWS_SECRET_ACCESS_KEY`, or `AWS_REGION` environment variables
    /// are not set. `AWS_SESSION_TOKEN` is optional.
    pub fn from_env() -> Result<Self, Error> {
        Ok(Self {
            access_key_id: SecretSource::from_plain(get_config_key(ACCESS_KEY_ID_ENV_VAR)?),
            secret_access_key: SecretSource::from_plain(get_config_key(SECRET_ACCESS_KEY_ENV_VAR)?),
            session_token: get_config_key_or_none(SESSION_TOKEN_ENV_VAR)
                .map(SecretSource::from_plain),
            region: get_config_key(REGION_ENV_VAR)?,
        })
    }
}

/// Host-facing schema for Bedrock provider configuration. Only
/// available when the `golem` feature is enabled because it depends on
/// the `golem-rust` agent host bindings.
///
/// The credential fields are [`Secret<String>`] handles that are
/// **not** materialized here. They are instead carried into the
/// runtime [`BedrockConfig`] via the [`From`] impl below and resolved
/// on every outgoing request, so that hot-rotated host secrets take
/// effect immediately without restarting the worker.
///
/// Note: the `session_token` field is required in the host schema
/// because the `ConfigSchema` derive does not currently support
/// `Option<Secret<T>>`. Callers that do not use session tokens can
/// supply an empty string and translate it back to `None` themselves
/// before constructing a [`BedrockConfig`]; alternatively they can
/// build a [`BedrockConfig`] directly.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct BedrockHostConfig {
    #[config_schema(secret)]
    pub access_key_id: golem_rust::agentic::Secret<String>,
    #[config_schema(secret)]
    pub secret_access_key: golem_rust::agentic::Secret<String>,
    #[config_schema(secret)]
    pub session_token: golem_rust::agentic::Secret<String>,
    pub region: String,
}

#[cfg(feature = "golem")]
impl From<BedrockHostConfig> for BedrockConfig {
    /// Converts a host-side schema config into a runtime
    /// [`BedrockConfig`] **without** materializing the secrets. The
    /// [`Secret<String>`] handles are preserved inside the
    /// [`SecretSource`]s so each outgoing request fetches the current
    /// value from the host.
    fn from(host: BedrockHostConfig) -> Self {
        Self {
            access_key_id: SecretSource::from_handle(host.access_key_id),
            secret_access_key: SecretSource::from_handle(host.secret_access_key),
            session_token: Some(SecretSource::from_handle(host.session_token)),
            region: host.region,
        }
    }
}

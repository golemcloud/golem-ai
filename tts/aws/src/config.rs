//! Configuration types for the AWS Polly TTS provider.
//!
//! AWS Polly requires AWS credentials (access key id, secret access
//! key, optional session token) and a region. The credentials are
//! held as [`SecretSource`]s so they are resolved on demand right
//! before each top-level provider call. The region is a plain
//! `String` because it is not a secret and does not rotate.
//!
//! See `tts/tts/src/config.rs` for the design rationale.

use golem_ai_tts::config::{get_optional_config, validate_config_key, SecretSource};
use golem_ai_tts::model::types::TtsError;

pub const ACCESS_KEY_ID_ENV_VAR: &str = "AWS_ACCESS_KEY_ID";
pub const SECRET_ACCESS_KEY_ENV_VAR: &str = "AWS_SECRET_ACCESS_KEY";
pub const SESSION_TOKEN_ENV_VAR: &str = "AWS_SESSION_TOKEN";
pub const REGION_ENV_VAR: &str = "AWS_REGION";

const DEFAULT_REGION: &str = "us-east-1";

/// Runtime AWS Polly provider configuration.
///
/// The AWS credentials are wrapped in [`SecretSource`]s, which are
/// resolved lazily right before each outgoing request. The region is a
/// plain [`String`] because it is not a secret and does not rotate.
#[derive(Clone, Debug)]
pub struct AwsConfig {
    /// AWS access key id, fetched on demand right before each request.
    pub access_key_id: SecretSource,
    /// AWS secret access key, fetched on demand right before each request.
    pub secret_access_key: SecretSource,
    /// Optional AWS session token (only required when using temporary
    /// credentials), fetched on demand right before each request.
    pub session_token: Option<SecretSource>,
    /// AWS region name (e.g. `"us-east-1"`).
    pub region: String,
}

impl AwsConfig {
    /// Builds an [`AwsConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    ///
    /// `AWS_REGION` defaults to `"us-east-1"` if not set.
    /// `AWS_SESSION_TOKEN` is optional.
    pub fn from_env() -> Result<Self, TtsError> {
        Ok(Self {
            access_key_id: SecretSource::from_plain(validate_config_key(ACCESS_KEY_ID_ENV_VAR)?),
            secret_access_key: SecretSource::from_plain(validate_config_key(
                SECRET_ACCESS_KEY_ENV_VAR,
            )?),
            session_token: get_optional_config(SESSION_TOKEN_ENV_VAR).map(SecretSource::from_plain),
            region: get_optional_config(REGION_ENV_VAR)
                .unwrap_or_else(|| DEFAULT_REGION.to_string()),
        })
    }
}

/// Host-facing schema for AWS Polly provider configuration.
///
/// The credential fields are [`Secret<String>`] handles that are
/// **not** materialized here. They are instead carried into the
/// runtime [`AwsConfig`] via the [`From`] impl below.
///
/// Note: the `session_token` field is required in the host schema
/// because the `ConfigSchema` derive does not currently support
/// `Option<Secret<T>>`. Callers that do not use session tokens can
/// supply an empty string and translate it back to `None` themselves
/// before constructing an [`AwsConfig`]; alternatively they can build
/// an [`AwsConfig`] directly. The `From` impl below treats a
/// session-token handle as always present (`Some(...)`).
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct AwsHostConfig {
    #[config_schema(secret)]
    pub access_key_id: golem_rust::agentic::Secret<String>,
    #[config_schema(secret)]
    pub secret_access_key: golem_rust::agentic::Secret<String>,
    #[config_schema(secret)]
    pub session_token: golem_rust::agentic::Secret<String>,
    pub region: String,
}

#[cfg(feature = "golem")]
impl From<AwsHostConfig> for AwsConfig {
    fn from(host: AwsHostConfig) -> Self {
        Self {
            access_key_id: SecretSource::from_handle(host.access_key_id),
            secret_access_key: SecretSource::from_handle(host.secret_access_key),
            session_token: Some(SecretSource::from_handle(host.session_token)),
            region: host.region,
        }
    }
}

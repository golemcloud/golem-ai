//! Configuration types for the AWS Transcribe provider.
//!
//! AWS Transcribe requires AWS credentials (access key, secret key), a
//! region, and an S3 bucket name. The credentials are held as
//! [`SecretSource`]s so they are resolved on demand right before each
//! top-level provider call. The region and bucket name are plain
//! `String`s because they are not secrets and do not rotate.
//!
//! See `llm/openai/src/config.rs` for the design rationale of the
//! two-config-types pattern (runtime + host-facing). Note that this
//! crate uses the AWS env var names `AWS_ACCESS_KEY` and
//! `AWS_SECRET_KEY` (without the trailing `_ID` / `_ACCESS`) to match
//! the names the previous version of this crate read from the
//! environment.

use golem_ai_stt::config::{get_config_key, SecretSource};
use golem_ai_stt::model::types::SttError;

pub const REGION_ENV_VAR: &str = "AWS_REGION";
pub const ACCESS_KEY_ENV_VAR: &str = "AWS_ACCESS_KEY";
pub const SECRET_KEY_ENV_VAR: &str = "AWS_SECRET_KEY";
pub const BUCKET_NAME_ENV_VAR: &str = "AWS_BUCKET_NAME";

/// Runtime AWS Transcribe provider configuration that the caller passes
/// into every provider call.
///
/// The AWS credentials are wrapped in [`SecretSource`]s, which are
/// resolved lazily right before each outgoing request. The region and
/// bucket name are plain [`String`]s because they are not secrets and
/// do not rotate.
#[derive(Clone, Debug)]
pub struct AwsConfig {
    /// AWS access key, fetched on demand right before each request.
    pub access_key: SecretSource,
    /// AWS secret key, fetched on demand right before each request.
    pub secret_key: SecretSource,
    /// AWS region name (e.g. `"us-east-1"`).
    pub region: String,
    /// S3 bucket name used to stage audio for transcription jobs.
    pub bucket_name: String,
}

impl AwsConfig {
    /// Builds an [`AwsConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    ///
    /// Returns an error if any of the required `AWS_REGION`,
    /// `AWS_ACCESS_KEY`, `AWS_SECRET_KEY`, or `AWS_BUCKET_NAME`
    /// environment variables are not set.
    pub fn from_env() -> Result<Self, SttError> {
        Ok(Self {
            access_key: SecretSource::from_plain(get_config_key(ACCESS_KEY_ENV_VAR)?),
            secret_key: SecretSource::from_plain(get_config_key(SECRET_KEY_ENV_VAR)?),
            region: get_config_key(REGION_ENV_VAR)?,
            bucket_name: get_config_key(BUCKET_NAME_ENV_VAR)?,
        })
    }
}

/// Host-facing schema for AWS Transcribe provider configuration.
///
/// Only available when the `golem` feature is enabled because it
/// depends on the `golem-rust` agent host bindings.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct AwsHostConfig {
    #[config_schema(secret)]
    pub access_key: golem_rust::agentic::Secret<String>,
    #[config_schema(secret)]
    pub secret_key: golem_rust::agentic::Secret<String>,
    pub region: String,
    pub bucket_name: String,
}

#[cfg(feature = "golem")]
impl From<AwsHostConfig> for AwsConfig {
    fn from(host: AwsHostConfig) -> Self {
        Self {
            access_key: SecretSource::from_handle(host.access_key),
            secret_key: SecretSource::from_handle(host.secret_key),
            region: host.region,
            bucket_name: host.bucket_name,
        }
    }
}

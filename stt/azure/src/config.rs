//! Configuration types for the Azure Speech-to-Text provider.
//!
//! See `llm/openai/src/config.rs` for the design rationale of the
//! two-config-types pattern (runtime + host-facing).

use golem_ai_stt::config::{get_config_key, SecretSource};
use golem_ai_stt::model::types::SttError;

pub const REGION_ENV_VAR: &str = "AZURE_REGION";
pub const SUBSCRIPTION_KEY_ENV_VAR: &str = "AZURE_SUBSCRIPTION_KEY";

/// Runtime Azure provider configuration that the caller passes into
/// every provider call.
///
/// The subscription key is wrapped in a [`SecretSource`], which is
/// resolved lazily right before each outgoing HTTP request. The region
/// is a plain [`String`] because it is not a secret and does not rotate.
#[derive(Clone, Debug)]
pub struct AzureConfig {
    /// Azure subscription key, fetched on demand right before each request.
    pub subscription_key: SecretSource,
    /// Azure region name (e.g. `"eastus"`).
    pub region: String,
}

impl AzureConfig {
    /// Builds an [`AzureConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    ///
    /// Returns an error if any of the required `AZURE_REGION` or
    /// `AZURE_SUBSCRIPTION_KEY` environment variables are not set.
    pub fn from_env() -> Result<Self, SttError> {
        Ok(Self {
            subscription_key: SecretSource::from_plain(get_config_key(SUBSCRIPTION_KEY_ENV_VAR)?),
            region: get_config_key(REGION_ENV_VAR)?,
        })
    }
}

/// Host-facing schema for Azure provider configuration. Only available
/// when the `golem` feature is enabled because it depends on the
/// `golem-rust` agent host bindings.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct AzureHostConfig {
    #[config_schema(secret)]
    pub subscription_key: golem_rust::agentic::Secret<String>,
    pub region: String,
}

#[cfg(feature = "golem")]
impl From<AzureHostConfig> for AzureConfig {
    fn from(host: AzureHostConfig) -> Self {
        Self {
            subscription_key: SecretSource::from_handle(host.subscription_key),
            region: host.region,
        }
    }
}

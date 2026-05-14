//! Configuration types for the Google Veo provider.
//!
//! Veo combines two non-secret configuration values (the GCP project
//! id and the service-account client email) with a single secret (the
//! service-account private key). The private key is wrapped in a
//! [`SecretSource`] so it can be fetched on demand right before each
//! JWT signing operation.
//!
//! See `llm/openai/src/config.rs` for the full design rationale.

use golem_ai_video::config::{get_config_key, SecretSource};
use golem_ai_video::model::types::VideoError;

pub const PROJECT_ID_ENV_VAR: &str = "VEO_PROJECT_ID";
pub const CLIENT_EMAIL_ENV_VAR: &str = "VEO_CLIENT_EMAIL";
pub const PRIVATE_KEY_ENV_VAR: &str = "VEO_PRIVATE_KEY";

/// Runtime Veo provider configuration that the caller passes into
/// every provider call.
///
/// The private key is wrapped in a [`SecretSource`], which is resolved
/// lazily right before each JWT signing operation. Do not extract the
/// underlying string and cache it across requests — doing so would
/// defeat host-side secret rotation.
#[derive(Clone, Debug)]
pub struct VeoConfig {
    /// The GCP project id (not a secret).
    pub project_id: String,
    /// The GCP service account client email (not a secret).
    pub client_email: String,
    /// The GCP service account private key, fetched on demand right
    /// before each JWT signing operation.
    pub private_key: SecretSource,
}

impl VeoConfig {
    /// Builds a [`VeoConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    ///
    /// Returns an error if any of the required `VEO_PROJECT_ID`,
    /// `VEO_CLIENT_EMAIL`, or `VEO_PRIVATE_KEY` environment variables
    /// are not set.
    pub fn from_env() -> Result<Self, VideoError> {
        Ok(Self {
            project_id: get_config_key(PROJECT_ID_ENV_VAR)?,
            client_email: get_config_key(CLIENT_EMAIL_ENV_VAR)?,
            private_key: SecretSource::from_plain(get_config_key(PRIVATE_KEY_ENV_VAR)?),
        })
    }
}

/// Host-facing schema for Veo provider configuration. Only available
/// when the `golem` feature is enabled because it depends on the
/// `golem-rust` agent host bindings.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct VeoHostConfig {
    pub project_id: String,
    pub client_email: String,
    #[config_schema(secret)]
    pub private_key: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<VeoHostConfig> for VeoConfig {
    fn from(host: VeoHostConfig) -> Self {
        Self {
            project_id: host.project_id,
            client_email: host.client_email,
            private_key: SecretSource::from_handle(host.private_key),
        }
    }
}

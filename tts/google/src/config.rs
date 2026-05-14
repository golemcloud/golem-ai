//! Configuration types for the Google Cloud TTS provider.
//!
//! Note: `GOOGLE_APPLICATION_CREDENTIALS` is the **path** to a JSON
//! service-account credentials file, so it is classified as plain
//! config rather than as a secret. The actual private key material
//! lives inside that file on disk.
//!
//! See `tts/tts/src/config.rs` for the design rationale.

use golem_ai_tts::config::get_optional_config;
use golem_ai_tts::model::types::TtsError;

pub const CREDENTIALS_ENV_VAR: &str = "GOOGLE_APPLICATION_CREDENTIALS";
pub const PROJECT_ENV_VAR: &str = "GOOGLE_CLOUD_PROJECT";

/// Runtime Google Cloud TTS provider configuration.
///
/// Both fields are plain (non-secret) configuration values. The path
/// in `credentials_path` may point to a JSON file containing private
/// key material, but the path itself is not secret.
#[derive(Clone, Debug, Default)]
pub struct GoogleConfig {
    /// Path to the service-account credentials JSON file. When `None`,
    /// the client falls back to the GCE/GKE metadata service.
    pub credentials_path: Option<String>,
    /// Optional Google Cloud project id (mostly informational; not all
    /// TTS requests need it).
    pub project_id: Option<String>,
}

impl GoogleConfig {
    /// Builds a [`GoogleConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    pub fn from_env() -> Result<Self, TtsError> {
        Ok(Self {
            credentials_path: get_optional_config(CREDENTIALS_ENV_VAR),
            project_id: get_optional_config(PROJECT_ENV_VAR),
        })
    }
}

/// Host-facing schema for Google Cloud TTS provider configuration.
/// Only available when the `golem` feature is enabled.
///
/// Both fields are plain `String`s here because the credentials path
/// itself is not secret. (The `From` impl below converts an empty
/// string to `None` for backward compatibility with hosts that lack
/// optional-string support.)
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct GoogleHostConfig {
    pub credentials_path: String,
    pub project_id: String,
}

#[cfg(feature = "golem")]
impl From<GoogleHostConfig> for GoogleConfig {
    fn from(host: GoogleHostConfig) -> Self {
        Self {
            credentials_path: if host.credentials_path.is_empty() {
                None
            } else {
                Some(host.credentials_path)
            },
            project_id: if host.project_id.is_empty() {
                None
            } else {
                Some(host.project_id)
            },
        }
    }
}

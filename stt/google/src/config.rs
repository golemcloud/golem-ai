//! Configuration types for the Google Speech-to-Text provider.
//!
//! Google Speech-to-Text is authenticated with a service account key,
//! which has a `private_key` field that is treated as a secret. All
//! other fields (project id, client email, location, bucket, and the
//! optional credentials file path) are non-secret configuration.
//!
//! See `llm/openai/src/config.rs` for the design rationale of the
//! two-config-types pattern (runtime + host-facing).

use golem_ai_stt::config::{get_config_key, get_config_key_or_none, SecretSource};
use golem_ai_stt::model::types::SttError;

pub const LOCATION_ENV_VAR: &str = "GOOGLE_LOCATION";
pub const BUCKET_NAME_ENV_VAR: &str = "GOOGLE_BUCKET_NAME";
pub const APPLICATION_CREDENTIALS_ENV_VAR: &str = "GOOGLE_APPLICATION_CREDENTIALS";
pub const PROJECT_ID_ENV_VAR: &str = "GOOGLE_PROJECT_ID";
pub const CLIENT_EMAIL_ENV_VAR: &str = "GOOGLE_CLIENT_EMAIL";
pub const PRIVATE_KEY_ENV_VAR: &str = "GOOGLE_PRIVATE_KEY";

/// Runtime Google Speech-to-Text provider configuration.
///
/// The `private_key` is the only true secret and is wrapped in a
/// [`SecretSource`] so it is resolved on demand right before each
/// top-level provider call. The other fields are plain `String`s.
///
/// `application_credentials_path` is an optional path to a Google
/// service account JSON file. When set, the file is read each time a
/// top-level provider call is made and its contents take priority over
/// the explicit `project_id` / `client_email` / `private_key` fields.
/// Note that `GOOGLE_APPLICATION_CREDENTIALS` is classified as
/// configuration (a path) rather than as a secret; the secret value
/// itself lives inside the JSON file (when used) or in `private_key`.
#[derive(Clone, Debug)]
pub struct GoogleConfig {
    /// Google location (e.g. `"us-central1"`).
    pub location: String,
    /// GCS bucket name used to stage audio for transcription jobs.
    pub bucket_name: String,
    /// Optional path to a Google service account JSON file. When set,
    /// the file is read on every top-level provider call and its
    /// contents take precedence over `project_id`, `client_email`,
    /// and `private_key`.
    pub application_credentials_path: Option<String>,
    /// Google Cloud project id.
    pub project_id: Option<String>,
    /// Google service account client email.
    pub client_email: Option<String>,
    /// Google service account private key, fetched on demand right
    /// before each request.
    pub private_key: Option<SecretSource>,
}

impl GoogleConfig {
    /// Builds a [`GoogleConfig`] by reading the same environment
    /// variables that earlier versions of the crate consulted internally.
    ///
    /// `GOOGLE_LOCATION` and `GOOGLE_BUCKET_NAME` are always required.
    ///
    /// Either:
    /// * `GOOGLE_APPLICATION_CREDENTIALS` is set (path to a JSON
    ///   service account file), OR
    /// * `GOOGLE_PROJECT_ID`, `GOOGLE_CLIENT_EMAIL`, and
    ///   `GOOGLE_PRIVATE_KEY` are all set.
    pub fn from_env() -> Result<Self, SttError> {
        let location = get_config_key(LOCATION_ENV_VAR)?;
        let bucket_name = get_config_key(BUCKET_NAME_ENV_VAR)?;
        let application_credentials_path = get_config_key_or_none(APPLICATION_CREDENTIALS_ENV_VAR);

        let (project_id, client_email, private_key) = if application_credentials_path.is_some() {
            (None, None, None)
        } else {
            (
                Some(get_config_key(PROJECT_ID_ENV_VAR)?),
                Some(get_config_key(CLIENT_EMAIL_ENV_VAR)?),
                Some(SecretSource::from_plain(get_config_key(
                    PRIVATE_KEY_ENV_VAR,
                )?)),
            )
        };

        Ok(Self {
            location,
            bucket_name,
            application_credentials_path,
            project_id,
            client_email,
            private_key,
        })
    }
}

/// Host-facing schema for Google Speech-to-Text provider configuration.
///
/// Only available when the `golem` feature is enabled because it
/// depends on the `golem-rust` agent host bindings.
///
/// The host schema does NOT include `application_credentials_path`
/// because reading a JSON file on the host side is not relevant; the
/// individual fields (`project_id`, `client_email`, `private_key`) are
/// supplied directly. Callers that need file-based credentials should
/// build a [`GoogleConfig`] directly from environment variables.
#[cfg(feature = "golem")]
#[derive(golem_rust::ConfigSchema)]
pub struct GoogleHostConfig {
    pub location: String,
    pub bucket_name: String,
    pub project_id: String,
    pub client_email: String,
    #[config_schema(secret)]
    pub private_key: golem_rust::agentic::Secret<String>,
}

#[cfg(feature = "golem")]
impl From<GoogleHostConfig> for GoogleConfig {
    fn from(host: GoogleHostConfig) -> Self {
        Self {
            location: host.location,
            bucket_name: host.bucket_name,
            application_credentials_path: None,
            project_id: Some(host.project_id),
            client_email: Some(host.client_email),
            private_key: Some(SecretSource::from_handle(host.private_key)),
        }
    }
}

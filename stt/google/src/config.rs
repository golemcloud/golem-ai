use serde::Deserialize;
use golem_stt::golem::stt::types::SttError;

#[derive(Debug, Deserialize)]
pub struct GoogleConfig {
    pub endpoint: Option<String>,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub log_level: Option<String>,
    pub credentials_json: String,
    pub project_id: Option<String>,
}

impl GoogleConfig {
    pub fn load() -> Result<Self, SttError> {
        let timeout_secs = std::env::var("STT_PROVIDER_TIMEOUT")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30);
        let max_retries = std::env::var("STT_PROVIDER_MAX_RETRIES")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(3);
        let endpoint = std::env::var("STT_PROVIDER_ENDPOINT").ok();
        let log_level = std::env::var("STT_PROVIDER_LOG_LEVEL").ok();
        let project_id = std::env::var("GOOGLE_CLOUD_PROJECT").ok();

        let creds_path_or_json = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
            .map_err(|_| SttError::Unauthorized("missing GOOGLE_APPLICATION_CREDENTIALS".into()))?;
        let credentials_json = if std::path::Path::new(&creds_path_or_json).exists() {
            std::fs::read_to_string(&creds_path_or_json)
                .map_err(|e| SttError::Unauthorized(format!("cannot read creds file: {e}")))?
        } else {
            creds_path_or_json
        };

        Ok(Self {
            endpoint,
            timeout_secs,
            max_retries,
            log_level,
            credentials_json,
            project_id,
        })
    }
} 
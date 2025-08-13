use serde::Deserialize;
use golem_stt::golem::stt::types::SttError;

#[derive(Debug, Deserialize)]
pub struct AzureConfig {
    pub endpoint: Option<String>,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub log_level: Option<String>,
    pub subscription_key: String,
    pub region: String,
    pub max_buffer_bytes: usize,
    pub max_concurrency: usize,
}

impl AzureConfig {
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

        let max_buffer_bytes = std::env::var("STT_BUFFER_LIMIT_BYTES")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(5_000_000);

        let subscription_key = std::env::var("AZURE_SPEECH_KEY")
            .map_err(|_| SttError::Unauthorized("missing AZURE_SPEECH_KEY".into()))?;
        let region = std::env::var("AZURE_SPEECH_REGION")
            .map_err(|_| SttError::Unauthorized("missing AZURE_SPEECH_REGION".into()))?;
        let max_concurrency = std::env::var("STT_MAX_CONCURRENCY").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(8);

        Ok(Self {
            endpoint,
            timeout_secs,
            max_retries,
            log_level,
            subscription_key,
            region,
            max_buffer_bytes,
            max_concurrency,
        })
    }
}

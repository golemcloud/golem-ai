pub struct DeepgramConfig {
    pub api_key: String,
    pub endpoint: String,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub max_concurrency: usize,
    pub default_model: Option<String>,
}

impl DeepgramConfig {
    pub fn load() -> Result<Self, golem_stt::golem::stt::types::SttError> {
        use std::env;
        let api_key = env::var("DEEPGRAM_API_KEY").map_err(|_| golem_stt::golem::stt::types::SttError::Unauthorized("missing DEEPGRAM_API_KEY".to_string()))?;
        let endpoint = env::var("STT_PROVIDER_ENDPOINT").ok().unwrap_or_else(|| "https://api.deepgram.com/v1".to_string());
        let timeout_secs = env::var("STT_PROVIDER_TIMEOUT").ok().and_then(|v| v.parse().ok()).unwrap_or(30u64);
        let max_retries = env::var("STT_PROVIDER_MAX_RETRIES").ok().and_then(|v| v.parse().ok()).unwrap_or(3u32);
        let max_concurrency = env::var("STT_MAX_CONCURRENCY").ok().and_then(|v| v.parse().ok()).unwrap_or(4usize);
        let default_model = env::var("DEEPGRAM_MODEL").ok();
        Ok(Self { api_key, endpoint, timeout_secs, max_retries, max_concurrency, default_model })
    }
}

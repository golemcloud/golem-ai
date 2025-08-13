use golem_stt::golem::stt::types::SttError;

pub struct AmazonConfig {
    pub endpoint: Option<String>,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub session_token: Option<String>,
    pub s3_bucket: Option<String>,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub max_buffer_bytes: usize,
    pub max_concurrency: usize,
}

impl AmazonConfig {
    pub fn load() -> Result<Self, SttError> {
        let endpoint = std::env::var("STT_PROVIDER_ENDPOINT").ok();
        let region = std::env::var("AWS_REGION").map_err(|_| SttError::Unauthorized("missing AWS_REGION".into()))?;
        let access_key = std::env::var("AWS_ACCESS_KEY_ID").map_err(|_| SttError::Unauthorized("missing AWS_ACCESS_KEY_ID".into()))?;
        let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY").map_err(|_| SttError::Unauthorized("missing AWS_SECRET_ACCESS_KEY".into()))?;
        let session_token = std::env::var("AWS_SESSION_TOKEN").ok();
        let s3_bucket = std::env::var("S3_BUCKET").ok();
        let timeout_secs = std::env::var("STT_PROVIDER_TIMEOUT").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(30);
        let max_retries = std::env::var("STT_PROVIDER_MAX_RETRIES").ok().and_then(|v| v.parse::<u32>().ok()).unwrap_or(3);
        let max_buffer_bytes = std::env::var("STT_BUFFER_LIMIT_BYTES").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(5_000_000);
        let max_concurrency = std::env::var("STT_MAX_CONCURRENCY").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(8);
        Ok(Self { endpoint, region, access_key, secret_key, session_token, s3_bucket, timeout_secs, max_retries, max_buffer_bytes, max_concurrency })
    }
}


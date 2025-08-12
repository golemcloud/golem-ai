use golem_stt::config::AwsConfig;
use golem_stt::errors::InternalSttError;
use golem_stt::exports::golem::stt::transcription::TranscribeOptions;
use golem_stt::exports::golem::stt::types::AudioConfig;
use golem_stt::http::HttpClient;

#[derive(Clone)]
pub struct AwsClient {
    pub cfg: AwsConfig,
    http: HttpClient,
}

impl AwsClient {
    pub fn new(cfg: AwsConfig) -> Result<Self, InternalSttError> {
        cfg.access_key_id
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_ACCESS_KEY_ID not set"))?;
        cfg.secret_access_key
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_SECRET_ACCESS_KEY not set"))?;
        cfg.region
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_REGION not set"))?;

        let http = HttpClient::new(cfg.common.timeout_secs, cfg.common.max_retries)?;
        Ok(Self { cfg, http })
    }

    fn region(&self) -> Result<String, InternalSttError> {
        self.cfg
            .region
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_REGION not set"))
            .cloned()
    }



    pub async fn transcribe(
        &self,
        _audio: Vec<u8>,
        _config: &AudioConfig,
        _options: &Option<TranscribeOptions<'_>>,
    ) -> Result<(u16, String), InternalSttError> {
        // AWS Transcribe requires files to be uploaded to S3 first
        // This is a fundamental limitation - we cannot send raw audio bytes

        // For a proper implementation, this would need:
        // 1. S3 bucket configuration (AWS_S3_BUCKET environment variable)
        // 2. Upload audio to S3 using AWS S3 API
        // 3. Start transcription job with S3 URI using StartTranscriptionJob API
        // 4. Poll for completion using GetTranscriptionJob API
        // 5. Download and return results

        Err(InternalSttError::unsupported_operation(
            "AWS Transcribe requires audio files to be uploaded to S3. Direct audio transcription is not supported. Please upload your audio to S3 and use the transcription job API instead."
        ))
    }
}

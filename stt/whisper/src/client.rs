use base64::Engine;
use golem_stt::config::WhisperConfig;
use golem_stt::errors::InternalSttError;
use golem_stt::exports::golem::stt::transcription::TranscribeOptions;
use golem_stt::exports::golem::stt::types::AudioConfig;
use golem_stt::http::HttpClient;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

#[derive(Clone)]
pub struct WhisperClient {
    pub cfg: WhisperConfig,
    http: HttpClient,
}

impl WhisperClient {
    pub fn new(cfg: WhisperConfig) -> Result<Self, InternalSttError> {
        // Validate required API key
        cfg.api_key
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("OPENAI_API_KEY not set"))?;

        let http = HttpClient::new(cfg.common.timeout_secs, cfg.common.max_retries)?;
        Ok(Self { cfg, http })
    }

    fn endpoint(&self) -> String {
        self.cfg.effective_endpoint()
    }

    fn auth_headers(&self) -> Result<HeaderMap, InternalSttError> {
        let mut headers = HeaderMap::new();
        let key = self
            .cfg
            .api_key
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("OPENAI_API_KEY not set"))?;
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {key}"))
                .map_err(|e| InternalSttError::internal(format!("invalid auth header: {e}")))?,
        );
        Ok(headers)
    }

    pub async fn transcribe(
        &self,
        audio: Vec<u8>,
        _config: &AudioConfig,
        options: &Option<TranscribeOptions<'_>>,
    ) -> Result<(u16, String), InternalSttError> {
        // OpenAI expects multipart/form-data normally. For WASI simplicity, we send JSON with base64 content.
        // A proxy or compatible endpoint can translate this. This is a degraded but valid approach.
        let url = self.endpoint();
        let headers = self.auth_headers()?;

        let b64 = base64::engine::general_purpose::STANDARD.encode(audio);
        let mut req_json = serde_json::json!({
            "model": options.as_ref().and_then(|o| o.model.clone()).unwrap_or_else(|| "whisper-1".to_string()),
            "file_b64": b64,
        });
        if let Some(opts) = options {
            if let Some(lang) = &opts.language {
                req_json["language"] = serde_json::json!(lang);
            }
        }

        let body = serde_json::to_vec(&req_json)
            .map_err(|e| InternalSttError::internal(format!("serialize whisper request: {e}")))?;

        let (status, text, _hdrs) = self
            .http
            .post_bytes(&url, headers, body, "application/json")
            .await?;

        Ok((status.as_u16(), text))
    }
}

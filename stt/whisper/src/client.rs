use golem_stt::config::WhisperConfig;
use golem_stt::errors::InternalSttError;
use golem_stt::exports::golem::stt::transcription::TranscribeOptions;
use golem_stt::exports::golem::stt::types::{AudioConfig, AudioFormat};
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

    fn build_multipart_body(
        &self,
        audio: Vec<u8>,
        config: &AudioConfig,
        options: &Option<TranscribeOptions<'_>>,
    ) -> Result<(Vec<u8>, String), InternalSttError> {
        // Build multipart/form-data manually for OpenAI Whisper API
        let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
        let mut body = Vec::new();

        // Add model field
        let model = options
            .as_ref()
            .and_then(|o| o.model.clone())
            .unwrap_or_else(|| "whisper-1".to_string());

        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"model\"\r\n\r\n");
        body.extend_from_slice(model.as_bytes());
        body.extend_from_slice(b"\r\n");

        // Add language field if provided
        if let Some(opts) = options {
            if let Some(lang) = &opts.language {
                body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
                body.extend_from_slice(
                    b"Content-Disposition: form-data; name=\"language\"\r\n\r\n",
                );
                body.extend_from_slice(lang.as_bytes());
                body.extend_from_slice(b"\r\n");
            }
        }

        // Add file field
        let filename = match config.format {
            AudioFormat::Wav => "audio.wav",
            AudioFormat::Mp3 => "audio.mp3",
            AudioFormat::Flac => "audio.flac",
            AudioFormat::Ogg => "audio.ogg",
            AudioFormat::Aac => "audio.aac",
            AudioFormat::Pcm => "audio.wav",
        };

        let content_type = match config.format {
            AudioFormat::Wav => "audio/wav",
            AudioFormat::Mp3 => "audio/mpeg",
            AudioFormat::Flac => "audio/flac",
            AudioFormat::Ogg => "audio/ogg",
            AudioFormat::Aac => "audio/aac",
            AudioFormat::Pcm => "audio/wav",
        };

        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n",
                filename
            )
            .as_bytes(),
        );
        body.extend_from_slice(format!("Content-Type: {}\r\n\r\n", content_type).as_bytes());
        body.extend_from_slice(&audio);
        body.extend_from_slice(b"\r\n");

        // Add response_format field to request JSON response
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"response_format\"\r\n\r\n");
        body.extend_from_slice(b"verbose_json\r\n");

        // Add timestamp_granularities if timestamps are enabled
        if let Some(opts) = options {
            if opts.enable_timestamps.unwrap_or(false) {
                body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
                body.extend_from_slice(
                    b"Content-Disposition: form-data; name=\"timestamp_granularities[]\"\r\n\r\n",
                );
                body.extend_from_slice(b"word\r\n");
            }
        }

        // Close boundary
        body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        let content_type = format!("multipart/form-data; boundary={}", boundary);
        Ok((body, content_type))
    }

    pub async fn transcribe(
        &self,
        audio: Vec<u8>,
        config: &AudioConfig,
        options: &Option<TranscribeOptions<'_>>,
    ) -> Result<(u16, String), InternalSttError> {
        // Use real OpenAI Whisper API with multipart/form-data
        let url = self.endpoint();
        let mut headers = self.auth_headers()?;

        let (body, content_type) = self.build_multipart_body(audio, config, options)?;

        // Set the multipart content type
        headers.insert(
            "Content-Type",
            reqwest::header::HeaderValue::from_str(&content_type)
                .map_err(|e| InternalSttError::internal(format!("invalid content-type: {e}")))?,
        );

        let (status, text, _hdrs) = self
            .http
            .post_bytes(&url, headers, body, &content_type)
            .await?;

        Ok((status.as_u16(), text))
    }
}

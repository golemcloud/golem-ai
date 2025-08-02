use crate::component::VocabularyResource;
use base64::Engine;
use golem_stt::config::GoogleConfig;
use golem_stt::errors::InternalSttError;
use golem_stt::exports::golem::stt::transcription::TranscribeOptions;
use golem_stt::exports::golem::stt::types::{AudioConfig, AudioFormat};
use golem_stt::exports::golem::stt::vocabularies::GuestVocabulary;
use golem_stt::http::HttpClient;
use log::trace;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

#[derive(Clone)]
pub struct GoogleClient {
    pub cfg: GoogleConfig,
    http: HttpClient,
}

impl GoogleClient {
    pub fn new(cfg: GoogleConfig) -> Result<Self, InternalSttError> {
        // Validate required Google environment variables
        cfg.application_credentials.as_ref().ok_or_else(|| {
            InternalSttError::unauthorized("GOOGLE_APPLICATION_CREDENTIALS not set")
        })?;
        cfg.cloud_project
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("GOOGLE_CLOUD_PROJECT not set"))?;

        let http = HttpClient::new(cfg.common.timeout_secs, cfg.common.max_retries)?;
        Ok(Self { cfg, http })
    }

    fn content_type_for(format: &AudioFormat) -> &'static str {
        match format {
            AudioFormat::Wav => "audio/wav",
            AudioFormat::Mp3 => "audio/mpeg",
            AudioFormat::Flac => "audio/flac",
            AudioFormat::Ogg => "audio/ogg",
            AudioFormat::Aac => "audio/aac",
            AudioFormat::Pcm => "application/octet-stream",
        }
    }

    fn build_headers(&self, _ct: &str) -> Result<HeaderMap, InternalSttError> {
        let mut headers = HeaderMap::new();

        let token = self
            .cfg
            .access_token
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("GOOGLE_ACCESS_TOKEN not set"))?;
        let value = format!("Bearer {token}");
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&value)
                .map_err(|e| InternalSttError::internal(format!("invalid auth header: {e}")))?,
        );

        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_str("application/json")
                .map_err(|e| InternalSttError::internal(format!("invalid content-type: {e}")))?,
        );

        // The media upload endpoint will require a different content-type for raw audio requests;
        // we will wrap the audio in JSON with base64 content, hence application/json is correct.

        Ok(headers)
    }

    pub fn endpoint(&self) -> String {
        self.cfg.effective_endpoint()
    }

    #[allow(dead_code)]
    pub fn auth_header_value(&self) -> Option<String> {
        self.cfg
            .access_token
            .as_ref()
            .map(|token| format!("Bearer {token}"))
    }

    #[allow(dead_code)]
    pub fn stream_base_endpoint(&self) -> Option<String> {
        self.cfg.common.endpoint.clone()
    }

    #[allow(dead_code)]
    pub fn default_stream_base(&self) -> String {
        "wss://speech.googleapis.com".to_string()
    }

    fn build_request_body(
        &self,
        audio: Vec<u8>,
        config: &AudioConfig,
        options: &Option<TranscribeOptions>,
    ) -> Result<String, InternalSttError> {
        // Google expects JSON with config + audio
        // Audio is base64-encoded
        let audio_b64 = base64::engine::general_purpose::STANDARD.encode(audio);

        let mut gcfg = serde_json::json!({
            "encoding": match config.format {
                AudioFormat::Wav => "LINEAR16", // WAV PCM; Google often expects LINEAR16 for WAV
                AudioFormat::Mp3 => "MP3",
                AudioFormat::Flac => "FLAC",
                AudioFormat::Ogg => "OGG_OPUS",
                AudioFormat::Aac => "AAC",
                AudioFormat::Pcm => "LINEAR16",
            }
        });

        if let Some(rate) = config.sample_rate {
            gcfg["sampleRateHertz"] = serde_json::json!(rate);
        }

        if let Some(opts) = options {
            if let Some(lang) = &opts.language {
                gcfg["languageCode"] = serde_json::json!(lang);
            }
            if let Some(model) = &opts.model {
                gcfg["model"] = serde_json::json!(model);
            }
            if opts.enable_speaker_diarization.unwrap_or(false) {
                gcfg["diarizationConfig"] = serde_json::json!({
                    "enableSpeakerDiarization": true,
                    "minSpeakerCount": 2,
                    "maxSpeakerCount": 6
                });
            }
            if opts.enable_timestamps.unwrap_or(true) {
                gcfg["enableWordTimeOffsets"] = serde_json::json!(true);
            }
            if opts.enable_word_confidence.unwrap_or(false) {
                gcfg["enableWordConfidence"] = serde_json::json!(true);
            }
            // Handle custom vocabulary and speech context
            let mut speech_contexts = Vec::new();

            // Add vocabulary phrases if provided
            if let Some(vocab) = &opts.vocabulary {
                let phrases = vocab.get::<VocabularyResource>().get_phrases();
                if !phrases.is_empty() {
                    speech_contexts.push(serde_json::json!({
                        "phrases": phrases
                    }));
                }
            }

            // Add speech context phrases if provided
            if let Some(ctx) = &opts.speech_context {
                if !ctx.is_empty() {
                    speech_contexts.push(serde_json::json!({
                        "phrases": ctx
                    }));
                }
            }

            // Set speechContexts if we have any
            if !speech_contexts.is_empty() {
                gcfg["speechContexts"] = serde_json::json!(speech_contexts);
            }
            if let Some(_prof) = opts.profanity_filter {
                // Google uses "profanityFilter" boolean
                gcfg["profanityFilter"] = serde_json::json!(_prof);
            }
        }

        let req = serde_json::json!({
            "config": gcfg,
            "audio": { "content": audio_b64 }
        });

        serde_json::to_string(&req)
            .map_err(|e| InternalSttError::internal(format!("serialize google request: {e}")))
    }

    pub async fn transcribe(
        &self,
        audio: Vec<u8>,
        config: &AudioConfig,
        options: &Option<TranscribeOptions<'_>>,
    ) -> Result<String, InternalSttError> {
        let url = self.endpoint();
        let headers = self.build_headers(Self::content_type_for(&config.format))?;
        let body = self.build_request_body(audio, config, options)?;

        trace!("Google POST URL: {url}");

        let (status, text, _resp_headers) = self
            .http
            .post_bytes(&url, headers, body.into_bytes(), "application/json")
            .await?;

        if !status.is_success() {
            // Return the raw error body; mapping to WIT errors will be done at component level
            return Err(InternalSttError::failed(format!(
                "google stt error: status={status}, body={text}"
            )));
        }

        Ok(text)
    }
}

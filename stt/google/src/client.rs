use crate::component::VocabularyResource;
use base64::Engine;
use golem_stt::config::GoogleConfig;
use golem_stt::errors::InternalSttError;
use golem_stt::exports::golem::stt::transcription::TranscribeOptions;
use golem_stt::exports::golem::stt::types::{AudioConfig, AudioFormat};
use golem_stt::exports::golem::stt::vocabularies::GuestVocabulary;
use golem_stt::http::HttpClient;
use log::trace;
use golem_stt::errors::extract_google_error_message;
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

        let token = match &self.cfg.access_token {
            Some(t) => t.clone(),
            None => self.generate_access_token_from_credentials()?,
        };
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

    // Streaming via native websockets/gRPC is not supported in WASI; handled at component level as unsupported

    fn build_request_body(
        &self,
        audio: &[u8],
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
        let body = self.build_request_body(&audio, config, options)?;

        trace!("Google POST URL: {url}");

        let (status, text, _resp_headers) = self
            .http
            .post_bytes(&url, headers, body.into_bytes(), "application/json")
            .await?;

        if !status.is_success() {
            // Fallback to long-running recognize for long audio limits
            let message = extract_google_error_message(&text);
            // Google may return 400 for content too long or require longrunning; also 413 Payload Too Large, or 422
            let code = status.as_u16();
            let m = message.to_ascii_lowercase();
            if code == 400 || code == 413 || m.contains("longrunning") || m.contains("too") || m.contains("length")
            {
                return self.longrunning_transcribe(&audio, config, options).await;
            }
            return Err(InternalSttError::failed(format!(
                "google stt error: status={status}, body={message}"
            )));
        }

        Ok(text)
    }
}

impl GoogleClient {
    async fn longrunning_transcribe(
        &self,
        audio: &[u8],
        config: &AudioConfig,
        options: &Option<TranscribeOptions<'_>>,
    ) -> Result<String, InternalSttError> {
        let url = "https://speech.googleapis.com/v1p1beta1/speech:longrunningrecognize".to_string();
        let headers = self.build_headers(Self::content_type_for(&config.format))?;
        // Long-running recognize with inline base64 audio content for larger durations
        let body = self.build_request_body(audio, config, options)?;

        let (status, text, _hdrs) = self
            .http
            .post_bytes(&url, headers, body.into_bytes(), "application/json")
            .await?;
        if !status.is_success() {
            let msg = extract_google_error_message(&text);
            return Err(InternalSttError::failed(format!(
                "google longrunning error {status}: {msg}"
            )));
        }

        // Parse operation name
        let v: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| InternalSttError::internal(format!("parse operation: {e}")))?;
        let name = v
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| InternalSttError::internal("missing operation name"))?;

        // Poll operation until done
        let operations_url = format!("https://speech.googleapis.com/v1/operations/{}", name);
        let start = std::time::Instant::now();
        let max = std::time::Duration::from_secs(self.cfg.common.timeout_secs.saturating_mul(10));
        loop {
            let mut hdrs = HeaderMap::new();
            // Authorization header only
            let token = match &self.cfg.access_token {
                Some(t) => t.clone(),
                None => self.generate_access_token_from_credentials()?,
            };
            let value = format!("Bearer {token}");
            hdrs.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&value)
                    .map_err(|e| InternalSttError::internal(format!("invalid auth header: {e}")))?,
            );

            let (s, body, _h) = self.http.get(&operations_url, hdrs).await?;
            if !s.is_success() {
                let msg = extract_google_error_message(&body);
                return Err(InternalSttError::failed(format!(
                    "operations get error {s}: {msg}"
                )));
            }
            let ov: serde_json::Value = serde_json::from_str(&body)
                .map_err(|e| InternalSttError::internal(format!("parse operation get: {e}")))?;
            if ov.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
                // Extract response.results and normalize to the same shape as sync recognize
                if let Some(resp) = ov.get("response") {
                    let results = resp.get("results").cloned().unwrap_or(serde_json::json!([]));
                    let normalized = serde_json::json!({
                        "results": results
                    });
                    return serde_json::to_string(&normalized)
                        .map_err(|e| InternalSttError::internal(format!("serialize normalized: {e}")));
                }
                // No response
                return Ok("{\"results\":[]}".to_string());
            }
            if start.elapsed() > max {
                return Err(InternalSttError::timeout("google longrunning operation timed out"));
            }
            // Backoff 1s between polls
            wstd::task::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
    fn generate_access_token_from_credentials(&self) -> Result<String, InternalSttError> {
        // Read service account JSON
        let path = self
            .cfg
            .application_credentials
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("GOOGLE_APPLICATION_CREDENTIALS not set"))?;

        let contents = std::fs::read_to_string(path)
            .map_err(|e| InternalSttError::internal(format!("read credentials: {e}")))?;
        let json: serde_json::Value = serde_json::from_str(&contents)
            .map_err(|e| InternalSttError::internal(format!("parse credentials json: {e}")))?;
        let client_email = json
            .get("client_email")
            .and_then(|v| v.as_str())
            .ok_or_else(|| InternalSttError::unauthorized("client_email missing in credentials"))?;
        let private_key = json
            .get("private_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| InternalSttError::unauthorized("private_key missing in credentials"))?;

        // Scope for Cloud Speech
        let scope = "https://www.googleapis.com/auth/cloud-platform";

        // Reuse the JWT+exchange logic from video::veo auth module pattern, implemented inline here
        use data_encoding::BASE64URL_NOPAD;
        use rsa::pkcs1v15::Pkcs1v15Sign;
        use rsa::pkcs8::DecodePrivateKey;
        use rsa::RsaPrivateKey;
        use sha2::{Digest, Sha256};

        // Normalize key newlines
        let processed_key = private_key.replace("\\n", "\n");
        let private_key = RsaPrivateKey::from_pkcs8_pem(&processed_key)
            .map_err(|e| InternalSttError::internal(format!("parse private key: {e}")))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| InternalSttError::internal(format!("time: {e}")))?
            .as_secs();
        let header = serde_json::json!({"alg":"RS256","typ":"JWT"});
        let payload = serde_json::json!({
            "iss": client_email,
            "scope": scope,
            "aud": "https://oauth2.googleapis.com/token",
            "iat": now,
            "exp": now + 120
        });
        let encoded_header = BASE64URL_NOPAD
            .encode(&serde_json::to_vec(&header).map_err(|e| InternalSttError::internal(format!("{e}")))?);
        let encoded_payload = BASE64URL_NOPAD
            .encode(&serde_json::to_vec(&payload).map_err(|e| InternalSttError::internal(format!("{e}")))?);
        let signing_input = format!("{encoded_header}.{encoded_payload}");

        // RS256 over SHA-256 DigestInfo per PKCS#1 v1.5
        const SHA256_PREFIX: &[u8] = &[
            0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, 0x05,
            0x00, 0x04, 0x20,
        ];
        let mut hasher = Sha256::new();
        hasher.update(signing_input.as_bytes());
        let hash = hasher.finalize();
        let mut digest_info = Vec::new();
        digest_info.extend_from_slice(SHA256_PREFIX);
        digest_info.extend_from_slice(&hash);
        let signature = private_key
            .sign(Pkcs1v15Sign::new_unprefixed(), &digest_info)
            .map_err(|e| InternalSttError::internal(format!("sign jwt: {e}")))?;
        let encoded_signature = BASE64URL_NOPAD.encode(&signature);
        let jwt = format!("{signing_input}.{encoded_signature}");

        // Exchange JWT for access token
        let body = format!(
            "grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer&assertion={jwt}"
        );

        let (status, text, _hdrs) = wstd::runtime::block_on(self.http.post_bytes(
            "https://oauth2.googleapis.com/token",
            HeaderMap::new(),
            body.into_bytes(),
            "application/x-www-form-urlencoded",
        ))?;

        if !status.is_success() {
            return Err(InternalSttError::unauthorized(format!(
                "token exchange failed {status}: {text}"
            )));
        }
        let v: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| InternalSttError::internal(format!("parse token response: {e}")))?;
        let token = v
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| InternalSttError::unauthorized("missing access_token in response"))?;
        Ok(token.to_string())
    }
}

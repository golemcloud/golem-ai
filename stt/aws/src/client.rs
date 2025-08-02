use crate::component::VocabularyResource;
use crate::sigv4::{sign, SigV4Params};
use golem_stt::config::AwsConfig;
use golem_stt::errors::InternalSttError;
use golem_stt::exports::golem::stt::transcription::TranscribeOptions;
use golem_stt::exports::golem::stt::types::{AudioConfig, AudioFormat};
use golem_stt::exports::golem::stt::vocabularies::GuestVocabulary;
use golem_stt::http::HttpClient;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use sha2::{Digest, Sha256};

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

    fn host(&self) -> Result<String, InternalSttError> {
        Ok(format!("transcribe.{}.amazonaws.com", self.region()?))
    }

    pub fn endpoint(&self) -> Result<String, InternalSttError> {
        self.cfg.effective_endpoint()
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

    fn sha256_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }

    fn query_params(
        &self,
        _config: &AudioConfig,
        options: &Option<TranscribeOptions>,
    ) -> Vec<(String, String)> {
        let mut q = Vec::new();
        if let Some(opts) = options {
            if let Some(lang) = &opts.language {
                q.push(("language-code".into(), lang.clone()));
            }
            if let Some(model) = &opts.model {
                q.push(("model".into(), model.clone()));
            }
            if opts.enable_speaker_diarization.unwrap_or(false) {
                q.push(("show-speaker-labels".into(), "true".into()));
            }
            if opts.enable_timestamps.unwrap_or(true) {
                q.push(("enable-timestamps".into(), "true".into()));
            }
            if let Some(vocab) = &opts.vocabulary {
                // Expose vocabulary name via the concrete resource type
                q.push((
                    "vocabulary-name".into(),
                    vocab.get::<VocabularyResource>().get_name(),
                ));
            }
        }
        q
    }

    pub async fn transcribe(
        &self,
        audio: Vec<u8>,
        config: &AudioConfig,
        options: &Option<TranscribeOptions<'_>>,
    ) -> Result<(u16, String), InternalSttError> {
        let url = self.endpoint()?;
        let ct = Self::content_type_for(&config.format);
        let query = self.query_params(config, options);

        let url_with_q = if query.is_empty() {
            url.clone()
        } else {
            let qs = serde_urlencoded::to_string(&query)
                .map_err(|e| InternalSttError::internal(format!("query serialize: {e}")))?;
            format!("{url}?{qs}")
        };

        let access_key = self
            .cfg
            .access_key_id
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_ACCESS_KEY_ID not set"))?
            .clone();
        let secret_key = self
            .cfg
            .secret_access_key
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AWS_SECRET_ACCESS_KEY not set"))?
            .clone();
        let session_token = self.cfg.session_token.clone();

        let now = time::OffsetDateTime::now_utc();
        let amz_date = now
            .format(
                &time::format_description::parse("[year][month][day]T[hour][minute][second]Z")
                    .unwrap(),
            )
            .map_err(|e| InternalSttError::internal(format!("time format: {e}")))?;
        let date_stamp = now
            .format(
                &time::format_description::parse("[year][month][day]").map_err(|e| {
                    InternalSttError::internal(format!("error parsing date format: {e}"))
                })?,
            )
            .map_err(|e| InternalSttError::internal(format!("date format: {e}")))?;

        let payload_sha256 = Self::sha256_hex(&audio);
        let region = self.region()?;
        let host = self.host()?;
        let uri = "/speech-to-text".to_string();
        let query_str = url_with_q
            .split_once('?')
            .map(|(_, qs)| qs.to_string())
            .unwrap_or_default();

        let (auth_header, signed_headers) = sign(SigV4Params {
            method: "POST".into(),
            service: "transcribe".into(),
            region,
            host: host.clone(),
            canonical_uri: uri.clone(),
            canonical_querystring: query_str.clone(),
            payload_sha256: payload_sha256.clone(),
            access_key,
            secret_key,
            session_token: session_token.clone(),
            amz_date: amz_date.clone(),
            date_stamp: date_stamp.clone(),
            content_type: Some(ct.into()),
        });

        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&auth_header)
                .map_err(|e| InternalSttError::internal(format!("auth header: {e}")))?,
        );
        for (k, v) in signed_headers {
            headers.insert(
                HeaderName::from_bytes(k.as_bytes())
                    .map_err(|e| InternalSttError::internal(format!("invalid header name: {e}")))?,
                HeaderValue::from_str(&v)
                    .map_err(|e| InternalSttError::internal(format!("signed header {k}: {e}")))?,
            );
        }
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_str(ct)
                .map_err(|e| InternalSttError::internal(format!("invalid content-type: {e}")))?,
        );

        let (status, text, _hdrs) = self
            .http
            .post_bytes(&url_with_q, headers, audio, ct)
            .await?;
        Ok((status.as_u16(), text))
    }
}

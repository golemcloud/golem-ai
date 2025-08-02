use crate::component::VocabularyResource;
use golem_stt::config::DeepgramConfig;
use golem_stt::errors::{extract_deepgram_error_message, InternalSttError};
use golem_stt::exports::golem::stt::transcription::TranscribeOptions;
use golem_stt::exports::golem::stt::types::{AudioConfig, AudioFormat};
use golem_stt::exports::golem::stt::vocabularies::GuestVocabulary;
use golem_stt::http::HttpClient;
use golem_stt::init_logging_from_env;
use golem_stt::mapping::{
    TranscriptAlternativeOut, TranscriptionMetadataOut, TranscriptionResultOut, WordSegmentOut,
};
use log::trace;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::Deserialize;

#[derive(Clone)]
pub struct DeepgramClient {
    pub cfg: DeepgramConfig,
    http: HttpClient,
}

impl DeepgramClient {
    pub fn new(cfg: DeepgramConfig) -> Result<Self, InternalSttError> {
        init_logging_from_env(cfg.common.log_level.clone());

        // Validate required Deepgram environment variable
        cfg.api_key
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("DEEPGRAM_API_KEY not set"))?;

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
            AudioFormat::Pcm => "audio/pcm",
        }
    }

    fn build_headers(&self) -> Result<HeaderMap, InternalSttError> {
        let mut headers = HeaderMap::new();
        if let Some(key) = &self.cfg.api_key {
            let value = format!("Token {key}");
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&value)
                    .map_err(|e| InternalSttError::internal(format!("invalid auth header: {e}")))?,
            );
        } else {
            return Err(InternalSttError::unauthorized(
                "DEEPGRAM_API_KEY is not set",
            ));
        }
        Ok(headers)
    }

    fn build_query_params(&self, opts: &Option<TranscribeOptions>) -> Vec<(String, String)> {
        let mut q = Vec::new();

        if let Some(o) = opts {
            if o.enable_timestamps.unwrap_or(true) {
                q.push(("words".into(), "true".into()));
            }
            if o.enable_speaker_diarization.unwrap_or(false) {
                q.push(("diarize".into(), "true".into()));
            }
            if let Some(lang) = &o.language {
                q.push(("language".into(), lang.clone()));
            }
            if let Some(model) = &o.model {
                q.push(("model".into(), model.clone()));
            }
            if o.profanity_filter.unwrap_or(false) {
                q.push(("profanity_filter".into(), "true".into()));
            }
            // Map vocabulary/speech-context to keyword boosting if provided.
            let mut all_keywords = Vec::new();

            // Add vocabulary phrases if provided
            if let Some(vocab) = &o.vocabulary {
                let vocab_phrases = vocab.get::<VocabularyResource>().get_phrases();
                all_keywords.extend(vocab_phrases);
            }

            // Add speech context phrases if provided
            if let Some(ctx) = &o.speech_context {
                all_keywords.extend(ctx.clone());
            }

            // Join with comma; Deepgram supports keywords/boosting via params.
            if !all_keywords.is_empty() {
                let keywords = all_keywords.join(",");
                q.push(("keywords".into(), keywords));
            }
            // enable-word-confidence and enable-timing-detail are implied by words=true for Deepgram.
        }

        q
    }

    pub async fn transcribe(
        &self,
        audio: Vec<u8>,
        config: &AudioConfig,
        options: &Option<TranscribeOptions<'_>>,
    ) -> Result<TranscriptionResultOut, InternalSttError> {
        let url = format!("{}/v1/listen", self.cfg.effective_endpoint());
        let headers = self.build_headers()?;
        let ct = Self::content_type_for(&config.format);
        let qp = self.build_query_params(options);
        let url = if qp.is_empty() {
            url
        } else {
            let qs = qp
                .iter()
                .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            format!("{url}?{qs}")
        };

        trace!("Deepgram POST URL: {url}");

        let (status, body, _headers) = self
            .http
            .post_bytes(&url, headers, audio.clone(), ct)
            .await?;

        if !status.is_success() {
            return Err(InternalSttError::failed(extract_deepgram_error_message(
                &body,
            )));
        }

        let parsed: DeepgramTranscript = serde_json::from_str(&body).map_err(|e| {
            InternalSttError::internal(format!("parse deepgram response: {e}, body: {body}"))
        })?;
        let size = u32::try_from(audio.len()).unwrap_or(u32::MAX);
        map_deepgram_to_out(parsed, size)
            .ok_or_else(|| InternalSttError::failed("empty transcription result"))
    }
}

#[derive(Debug, Deserialize)]
pub struct DeepgramTranscript {
    #[serde(default)]
    pub results: Option<DeepgramResults>,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub metadata: Option<DeepgramMetadata>,
}

#[derive(Debug, Deserialize)]
pub struct DeepgramMetadata {
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub model_uuid: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub duration: Option<f32>,
    #[serde(default)]
    pub language: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeepgramResults {
    #[serde(default)]
    pub channels: Vec<DeepgramChannel>,
}

#[derive(Debug, Deserialize)]
pub struct DeepgramChannel {
    #[serde(default)]
    pub alternatives: Vec<DeepgramAlternative>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeepgramAlternative {
    #[serde(default)]
    pub transcript: String,
    #[serde(default)]
    pub confidence: Option<f32>,
    #[serde(default)]
    pub words: Option<Vec<DeepgramWord>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeepgramWord {
    pub word: String,
    #[serde(default)]
    pub start: Option<f32>,
    #[serde(default)]
    pub end: Option<f32>,
    #[serde(default)]
    pub confidence: Option<f32>,
    #[serde(default)]
    pub speaker: Option<String>,
}

pub fn map_deepgram_to_out(
    dg: DeepgramTranscript,
    audio_size: u32,
) -> Option<TranscriptionResultOut> {
    let results = dg.results?;
    let alt = results.channels.first()?.alternatives.first()?;
    let text = alt.transcript.clone();
    let confidence = alt.confidence.unwrap_or(1.0);
    let words = alt
        .words
        .as_ref()
        .map(|ws| {
            ws.iter()
                .map(|w| WordSegmentOut {
                    text: w.word.clone(),
                    start_time: w.start.unwrap_or(0.0),
                    end_time: w.end.unwrap_or(0.0),
                    confidence: w.confidence,
                    speaker_id: w.speaker.clone(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Request id and metadata
    let req_id = dg
        .request_id
        .or_else(|| dg.metadata.as_ref().and_then(|m| m.request_id.clone()))
        .unwrap_or_else(|| "unknown".to_string());

    let duration = dg.metadata.as_ref().and_then(|m| m.duration).unwrap_or(0.0);

    let language = dg
        .metadata
        .as_ref()
        .and_then(|m| m.language.clone())
        .unwrap_or_else(|| "en".to_string());

    let model = dg
        .metadata
        .as_ref()
        .and_then(|m| m.model.clone())
        .or_else(|| dg.metadata.as_ref().and_then(|m| m.model_uuid.clone()));

    Some(TranscriptionResultOut {
        alternatives: vec![TranscriptAlternativeOut {
            text,
            confidence,
            words,
        }],
        metadata: TranscriptionMetadataOut {
            duration_seconds: duration,
            audio_size_bytes: audio_size,
            request_id: req_id,
            model,
            language,
        },
    })
}

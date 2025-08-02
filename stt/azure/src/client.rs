use crate::component::VocabularyResource;
use golem_stt::config::AzureConfig;
use golem_stt::errors::InternalSttError;
use golem_stt::exports::golem::stt::transcription::TranscribeOptions;
use golem_stt::exports::golem::stt::types::{AudioConfig, AudioFormat};
use golem_stt::exports::golem::stt::vocabularies::GuestVocabulary;
use golem_stt::http::HttpClient;
use golem_stt::mapping::{
    TranscriptAlternativeOut, TranscriptionMetadataOut, TranscriptionResultOut, WordSegmentOut,
};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::Deserialize;

#[derive(Clone)]
pub struct AzureClient {
    pub cfg: AzureConfig,
    http: HttpClient,
}

impl AzureClient {
    pub fn new(cfg: AzureConfig) -> Result<Self, InternalSttError> {
        // Validate required Azure environment variables
        cfg.speech_key
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AZURE_SPEECH_KEY not set"))?;
        cfg.speech_region
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AZURE_SPEECH_REGION not set"))?;

        let http = HttpClient::new(cfg.common.timeout_secs, cfg.common.max_retries)?;
        Ok(Self { cfg, http })
    }

    pub fn endpoint(&self) -> Result<String, InternalSttError> {
        self.cfg.effective_endpoint()
    }

    fn build_headers(&self) -> Result<HeaderMap, InternalSttError> {
        let mut headers = HeaderMap::new();

        let key = self
            .cfg
            .speech_key
            .as_ref()
            .ok_or_else(|| InternalSttError::unauthorized("AZURE_SPEECH_KEY not set"))?;

        headers.insert(
            "Ocp-Apim-Subscription-Key",
            HeaderValue::from_str(key)
                .map_err(|e| InternalSttError::internal(format!("invalid key header: {e}")))?,
        );

        headers.insert(CONTENT_TYPE, HeaderValue::from_static("audio/wav"));

        Ok(headers)
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

    fn query_params(
        &self,
        _config: &AudioConfig,
        options: &Option<TranscribeOptions>,
    ) -> Vec<(String, String)> {
        let mut q: Vec<(String, String)> = Vec::new();
        // language
        if let Some(opts) = options {
            if let Some(lang) = &opts.language {
                q.push(("language".to_string(), lang.clone()));
            }
        }
        // format
        q.push(("format".to_string(), "detailed".to_string()));
        q
    }

    fn build_phrase_list_header(&self, options: &Option<TranscribeOptions>) -> Option<String> {
        if let Some(opts) = options {
            let mut phrases = Vec::new();

            // Add vocabulary phrases if provided
            if let Some(vocab) = &opts.vocabulary {
                let vocab_phrases = vocab.get::<VocabularyResource>().get_phrases();
                phrases.extend(vocab_phrases);
            }

            // Add speech context phrases if provided
            if let Some(ctx) = &opts.speech_context {
                phrases.extend(ctx.clone());
            }

            if !phrases.is_empty() {
                // Azure expects phrase list as JSON in X-Microsoft-OutputFormat header or request body
                // For simplicity, we'll use a comma-separated list in a custom header
                return Some(phrases.join(","));
            }
        }
        None
    }

    pub async fn transcribe(
        &self,
        audio: Vec<u8>,
        config: &AudioConfig,
        options: &Option<TranscribeOptions<'_>>,
    ) -> Result<TranscriptionResultOut, InternalSttError> {
        let url = self.endpoint()?;
        let mut headers = self.build_headers()?;
        // Override content-type based on provided format when sending raw audio
        let ct = Self::content_type_for(&config.format);
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_str(ct)
                .map_err(|e| InternalSttError::internal(format!("invalid content-type: {e}")))?,
        );

        // Add phrase list header if vocabulary or speech context is provided
        if let Some(phrase_list) = self.build_phrase_list_header(options) {
            headers.insert(
                "X-Microsoft-PhraseList",
                HeaderValue::from_str(&phrase_list).map_err(|e| {
                    InternalSttError::internal(format!("invalid phrase list header: {e}"))
                })?,
            );
        }

        let query = self.query_params(config, options);
        let url_with_q = if query.is_empty() {
            url
        } else {
            let qs = serde_urlencoded::to_string(&query)
                .map_err(|e| InternalSttError::internal(format!("query serialize: {e}")))?;
            format!("{url}?{qs}")
        };

        let (status, text, hdrs) = self
            .http
            .post_bytes(&url_with_q, headers, audio.clone(), ct)
            .await?;

        if !status.is_success() {
            return Err(InternalSttError::failed(format!(
                "azure stt error: status={status}, body={text}"
            )));
        }

        let parsed: AzureTranscribeResponse = serde_json::from_str(&text)
            .map_err(|e| InternalSttError::internal(format!("azure parse error: {e}")))?;

        let request_id = hdrs
            .get("X-Request-Id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let model = options.as_ref().and_then(|o| o.model.clone());
        let language = options
            .as_ref()
            .and_then(|o| o.language.clone())
            .unwrap_or_else(|| "en-US".to_string());

        map_azure_to_out(parsed, audio.len() as u32, request_id, model, &language)
            .ok_or_else(|| InternalSttError::failed("empty transcription result"))
    }
}

#[derive(Debug, Deserialize)]
pub struct AzureTranscribeResponse {
    #[serde(default)]
    #[allow(dead_code)]
    pub recognition_status: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub offset: Option<String>,
    #[serde(default)]
    pub duration: Option<String>,
    #[serde(default)]
    pub display_text: Option<String>,
    // Some REST variants return an NBest field (similar to alternatives)
    #[serde(default)]
    pub nbest: Option<Vec<AzureNBest>>,
}

#[derive(Debug, Deserialize)]
pub struct AzureNBest {
    #[serde(default)]
    pub display: String,
    #[serde(default)]
    pub confidence: Option<f32>,
    // Word-level info available in detailed output
    #[serde(default)]
    pub words: Option<Vec<AzureWord>>,
}

#[derive(Debug, Deserialize)]
pub struct AzureWord {
    #[serde(default)]
    pub word: String,
    // durations are usually provided as offsets and durations in 100-ns units or timespan strings
    #[serde(default)]
    pub offset: Option<String>,
    #[serde(default)]
    pub duration: Option<String>,
    // Speaker diarization may be returned separately; keep optional
    #[serde(default)]
    pub speaker: Option<String>,
    // Sometimes no word-level confidence is exposed
    #[serde(default)]
    pub confidence: Option<f32>,
}

fn timespan_to_secs(ts: &Option<String>) -> f32 {
    // Azure often returns "PT#S" ISO8601 or 100ns ticks (e.g., "12345600").
    // We attempt a best-effort parse:
    if let Some(s) = ts {
        // Try ISO8601 "PT...S"
        if let Some(stripped) = s.strip_prefix("PT").and_then(|x| x.strip_suffix('S')) {
            if let Ok(v) = stripped.parse::<f32>() {
                return v;
            }
        }
        // Try ticks (100ns)
        if let Ok(ticks) = s.parse::<i64>() {
            return (ticks as f32) / 10_000_000.0;
        }
    }
    0.0
}

pub fn map_azure_to_out(
    resp: AzureTranscribeResponse,
    audio_size: u32,
    request_id: Option<String>,
    model: Option<String>,
    language: &str,
) -> Option<TranscriptionResultOut> {
    // Prefer NBest when available; else fallback to single display_text
    if let Some(nbest) = resp.nbest.as_ref().and_then(|v| v.first()) {
        let words = nbest
            .words
            .as_ref()
            .map(|ws| {
                ws.iter()
                    .map(|w| {
                        // Convert offset/duration to start/end seconds
                        let start = timespan_to_secs(&w.offset);
                        let end = start + timespan_to_secs(&w.duration);
                        WordSegmentOut {
                            text: w.word.clone(),
                            start_time: start,
                            end_time: end,
                            confidence: w.confidence,
                            speaker_id: w.speaker.clone(),
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let alt = TranscriptAlternativeOut {
            text: nbest.display.clone(),
            confidence: nbest.confidence.unwrap_or(1.0),
            words,
        };

        let metadata = TranscriptionMetadataOut {
            duration_seconds: timespan_to_secs(&resp.duration),
            audio_size_bytes: audio_size,
            request_id: request_id.unwrap_or_else(|| "unknown".to_string()),
            model,
            language: language.to_string(),
        };

        return Some(TranscriptionResultOut {
            alternatives: vec![alt],
            metadata,
        });
    }

    // Fallback when only display_text is present
    if let Some(text) = resp.display_text {
        let alt = TranscriptAlternativeOut {
            text,
            confidence: 1.0,
            words: vec![],
        };
        let metadata = TranscriptionMetadataOut {
            duration_seconds: timespan_to_secs(&resp.duration),
            audio_size_bytes: audio_size,
            request_id: request_id.unwrap_or_else(|| "unknown".to_string()),
            model,
            language: language.to_string(),
        };
        return Some(TranscriptionResultOut {
            alternatives: vec![alt],
            metadata,
        });
    }

    None
}

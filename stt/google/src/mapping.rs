use golem_stt::mapping::{
    TranscriptAlternativeOut, TranscriptionMetadataOut, TranscriptionResultOut, WordSegmentOut,
};
use serde::Deserialize;

/// Minimal Google STT response mapping.
/// Reference: https://cloud.google.com/speech-to-text/docs/reference/rest/v1/speech/recognize
#[derive(Debug, Deserialize)]
pub struct GoogleTranscribeResponse {
    #[serde(default)]
    pub results: Vec<GoogleResult>,
}

#[derive(Debug, Deserialize)]
pub struct GoogleResult {
    #[serde(default)]
    pub alternatives: Vec<GoogleAlternative>,
    #[serde(default)]
    pub language_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GoogleAlternative {
    #[serde(default)]
    pub transcript: String,
    #[serde(default)]
    pub confidence: Option<f32>,
    #[serde(default)]
    pub words: Option<Vec<GoogleWordInfo>>,
}

#[derive(Debug, Deserialize)]
pub struct GoogleWordInfo {
    #[serde(default)]
    pub word: String,
    #[serde(default)]
    pub start_time: Option<GoogleDuration>,
    #[serde(default)]
    pub end_time: Option<GoogleDuration>,
    #[serde(default)]
    pub speaker_tag: Option<i32>,
    // Google may not always provide per-word confidence
    #[serde(default)]
    pub confidence: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct GoogleDuration {
    // seconds as string per API, plus nanos
    #[serde(default)]
    pub seconds: Option<String>,
    #[serde(default)]
    pub nanos: Option<i32>,
}

fn duration_to_secs(d: &Option<GoogleDuration>) -> f32 {
    if let Some(dur) = d {
        let secs = dur
            .seconds
            .as_ref()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0) as f32;
        let nanos = dur.nanos.unwrap_or(0) as f32;
        secs + nanos / 1_000_000_000.0
    } else {
        0.0
    }
}

pub fn map_google_to_out(
    resp: GoogleTranscribeResponse,
    audio_size: u32,
    request_id: Option<String>,
    model: Option<String>,
    language_fallback: &str,
) -> Option<TranscriptionResultOut> {
    let first = resp.results.first()?;
    let alt = first.alternatives.first()?;

    let words = alt
        .words
        .as_ref()
        .map(|ws| {
            ws.iter()
                .map(|w| WordSegmentOut {
                    text: w.word.clone(),
                    start_time: duration_to_secs(&w.start_time),
                    end_time: duration_to_secs(&w.end_time),
                    confidence: w.confidence,
                    speaker_id: w
                        .speaker_tag
                        .map(|t| t.to_string()),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let alternative = TranscriptAlternativeOut {
        text: alt.transcript.clone(),
        confidence: alt.confidence.unwrap_or(1.0),
        words,
    };

    let language = first
        .language_code
        .clone()
        .unwrap_or_else(|| language_fallback.to_string());

    // Google REST does not always return request id or exact duration/model in recognize.
    let metadata = TranscriptionMetadataOut {
        duration_seconds: 0.0,
        audio_size_bytes: audio_size,
        request_id: request_id.unwrap_or_else(|| "unknown".to_string()),
        model,
        language,
    };

    Some(TranscriptionResultOut {
        alternatives: vec![alternative],
        metadata,
    })
}

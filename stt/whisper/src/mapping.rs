use golem_stt::mapping::{
    TranscriptAlternativeOut, TranscriptionMetadataOut, TranscriptionResultOut, WordSegmentOut,
};

/// Whisper/OpenAI-style output mapping (degraded features).
/// Whisper typically does not provide speaker diarization or per-word confidence in basic outputs.
/// If using WhisperX or timestamps=true variants, you may get word timings.
/// This mapper is designed to gracefully degrade where fields are unavailable.
#[derive(Debug)]
pub struct WhisperWord {
    pub text: String,
    pub start: Option<f32>,
    pub end: Option<f32>,
    pub confidence: Option<f32>, // often None for Whisper base
}

#[derive(Debug)]
pub struct WhisperAlt {
    pub text: String,
    pub words: Option<Vec<WhisperWord>>,
}

#[derive(Debug)]
pub struct WhisperBatchResult {
    pub alternative: WhisperAlt,
    pub duration_seconds: Option<f32>,
    pub request_id: Option<String>,
    pub model: Option<String>,
    pub language: Option<String>,
}

pub fn map_whisper_to_out(
    resp: WhisperBatchResult,
    audio_size: u32,
    language_fallback: &str,
) -> Option<TranscriptionResultOut> {
    let words_vec: Vec<WordSegmentOut> = resp
        .alternative
        .words
        .unwrap_or_default()
        .into_iter()
        .map(|w| WordSegmentOut {
            text: w.text,
            start_time: w.start.unwrap_or(0.0),
            end_time: w.end.unwrap_or(0.0),
            confidence: w.confidence, // usually None for base Whisper
            speaker_id: None,         // diarization not supported
        })
        .collect();

    let alternative = TranscriptAlternativeOut {
        text: resp.alternative.text,
        confidence: 1.0, // Whisper usually omits overall confidence; assume max
        words: words_vec,
    };

    let metadata = TranscriptionMetadataOut {
        duration_seconds: resp.duration_seconds.unwrap_or(0.0),
        audio_size_bytes: audio_size,
        request_id: resp.request_id.unwrap_or_else(|| "unknown".to_string()),
        model: resp.model,
        language: resp
            .language
            .unwrap_or_else(|| language_fallback.to_string()),
    };

    Some(TranscriptionResultOut {
        alternatives: vec![alternative],
        metadata,
    })
}

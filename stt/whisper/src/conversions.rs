use golem_stt::exports::golem::stt::types as wit_types;
use serde::Deserialize;

// OpenAI whisper response structures
#[derive(Debug, Deserialize)]
struct WhisperWord {
    word: String,
    start: f32,
    end: f32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WhisperSegment {
    text: String,
    start: f32,
    end: f32,
    words: Option<Vec<WhisperWord>>,
}

#[derive(Debug, Deserialize)]
struct WhisperResponse {
    #[serde(default)]
    text: String,
    #[serde(default)]
    duration: Option<f32>,
    #[serde(default)]
    segments: Option<Vec<WhisperSegment>>,
}

// WhisperX integration removed for now - can be added later with proper dependencies

pub fn to_wit_result(
    body: &str,
    language: &str,
    model: Option<&str>,
) -> Result<wit_types::TranscriptionResult, wit_types::SttError> {
    let parsed: WhisperResponse = serde_json::from_str(body)
        .map_err(|e| wit_types::SttError::InternalError(format!("whisper parse error: {e}")))?;

    let mut words = Vec::new();
    if let Some(segments) = parsed.segments {
        for segment in segments {
            if let Some(segment_words) = segment.words {
                for word in segment_words {
                    words.push(wit_types::WordSegment {
                        text: word.word,
                        start_time: word.start,
                        end_time: word.end,
                        confidence: None, // Whisper doesn't provide word-level confidence
                        speaker_id: None, // Whisper doesn't provide speaker diarization
                    });
                }
            }
        }
    }

    let alternative = wit_types::TranscriptAlternative {
        text: parsed.text,
        confidence: 0.0, // Whisper doesn't provide overall confidence
        words,
    };

    let metadata = wit_types::TranscriptionMetadata {
        duration_seconds: parsed.duration.unwrap_or(0.0),
        audio_size_bytes: 0, // Not provided by Whisper API
        request_id: "whisper-api-response".to_string(), // Whisper API doesn't provide request IDs
        model: model.map(|s| s.to_string()),
        language: language.to_string(),
    };

    Ok(wit_types::TranscriptionResult {
        alternatives: vec![alternative],
        metadata,
    })
}

use golem_stt::errors::{extract_google_error_message, InternalSttError};

pub fn to_wit_error(err: InternalSttError) -> wit_types::SttError {
    match err {
        InternalSttError::InvalidAudio(m) => wit_types::SttError::InvalidAudio(m),
        InternalSttError::UnsupportedFormat(m) => wit_types::SttError::UnsupportedFormat(m),
        InternalSttError::UnsupportedLanguage(m) => wit_types::SttError::UnsupportedLanguage(m),
        InternalSttError::TranscriptionFailed(m) => wit_types::SttError::TranscriptionFailed(m),
        InternalSttError::Unauthorized(m) => wit_types::SttError::Unauthorized(m),
        InternalSttError::AccessDenied(m) => wit_types::SttError::AccessDenied(m),
        InternalSttError::QuotaExceeded(q) => {
            wit_types::SttError::QuotaExceeded(wit_types::QuotaInfo {
                used: q.used,
                limit: q.limit,
                reset_time: q.reset_time,
                unit: match q.unit {
                    golem_stt::errors::QuotaUnit::Seconds => wit_types::QuotaUnit::Seconds,
                    golem_stt::errors::QuotaUnit::Requests => wit_types::QuotaUnit::Requests,
                    golem_stt::errors::QuotaUnit::Credits => wit_types::QuotaUnit::Credits,
                },
            })
        }
        InternalSttError::RateLimited(s) => wit_types::SttError::RateLimited(s),
        InternalSttError::InsufficientCredits => wit_types::SttError::InsufficientCredits,
        InternalSttError::UnsupportedOperation(m) => wit_types::SttError::UnsupportedOperation(m),
        InternalSttError::ServiceUnavailable(m) => wit_types::SttError::ServiceUnavailable(m),
        InternalSttError::NetworkError(m) => wit_types::SttError::NetworkError(m),
        InternalSttError::InternalError(m) => wit_types::SttError::InternalError(m),
        InternalSttError::Timeout(m) => {
            wit_types::SttError::TranscriptionFailed(format!("Timeout: {m}"))
        }
    }
}

pub fn to_wit_error_from_whisper(status: u16, body: &str) -> wit_types::SttError {
    let message = extract_whisper_error_message(body);
    match status {
        400 => wit_types::SttError::UnsupportedFormat(message),
        401 => wit_types::SttError::Unauthorized(message),
        403 => wit_types::SttError::AccessDenied(message),
        404 => wit_types::SttError::UnsupportedOperation(message),
        429 => wit_types::SttError::RateLimited(1),
        s if s >= 500 => wit_types::SttError::ServiceUnavailable(message),
        _ => wit_types::SttError::TranscriptionFailed(message),
    }
}

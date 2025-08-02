use golem_stt::exports::golem::stt::types as wit_types;
use golem_stt::mapping::TranscriptionResultOut;
use serde::Deserialize;

// Azure "detailed" format response structures (subset)
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NBest {
    #[serde(default)]
    lexical: String,
    #[serde(default)]
    itn: String,
    #[serde(default)]
    #[serde(rename = "maskedItn")]
    #[allow(non_snake_case)]
    masked_itn: String,
    #[serde(default)]
    display: String,
    #[serde(default)]
    confidence: f32,
    #[serde(default)]
    words: Vec<WordInfo>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WordInfo {
    word: String,
    #[serde(default)]
    offset: u64, // 100-ns units
    #[serde(default)]
    duration: u64, // 100-ns units
    #[serde(default)]
    speaker: Option<String>,
    #[serde(default)]
    confidence: Option<f32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AzureDetailedResult {
    #[serde(rename = "RecognitionStatus")]
    recognition_status: Option<String>,
    #[serde(rename = "NBest")]
    nbest: Option<Vec<NBest>>,
}

#[allow(dead_code)]
fn ticks_100ns_to_secs(ticks: u64) -> f32 {
    // 10_000_000 ticks per second
    (ticks as f32) / 10_000_000.0
}

#[allow(dead_code)]
fn parse_azure_time(time_str: &Option<String>) -> f32 {
    if let Some(s) = time_str {
        // Try parsing as ticks (100ns units)
        if let Ok(ticks) = s.parse::<u64>() {
            return ticks_100ns_to_secs(ticks);
        }
        // Try parsing as ISO8601 duration "PT#S"
        if let Some(stripped) = s.strip_prefix("PT").and_then(|x| x.strip_suffix('S')) {
            if let Ok(v) = stripped.parse::<f32>() {
                return v;
            }
        }
    }
    0.0
}

#[allow(dead_code)]
pub fn azure_to_wit_result(
    azure_response: crate::client::AzureTranscribeResponse,
    audio_size_bytes: u32,
    language: String,
    model: Option<String>,
) -> Result<wit_types::TranscriptionResult, wit_types::SttError> {
    let mut alts: Vec<wit_types::TranscriptAlternative> = Vec::new();

    // Handle NBest results if available
    if let Some(nbest) = azure_response.nbest {
        if let Some(top) = nbest.first() {
            let mut words_out = Vec::new();
            if let Some(words) = &top.words {
                for w in words {
                    let start = parse_azure_time(&w.offset);
                    let end = start + parse_azure_time(&w.duration);
                    words_out.push(wit_types::WordSegment {
                        text: w.word.clone(),
                        start_time: start,
                        end_time: end,
                        confidence: w.confidence,
                        speaker_id: w.speaker.clone(),
                    });
                }
            }

            alts.push(wit_types::TranscriptAlternative {
                text: if !top.display.is_empty() {
                    top.display.clone()
                } else {
                    azure_response.display_text.unwrap_or_default()
                },
                confidence: top.confidence.unwrap_or(1.0),
                words: words_out,
            });
        }
    } else if let Some(display_text) = azure_response.display_text {
        // Fallback to simple display text
        alts.push(wit_types::TranscriptAlternative {
            text: display_text,
            confidence: 1.0,
            words: vec![],
        });
    }

    // Calculate duration from Azure response
    let duration = timespan_to_secs(&azure_response.duration);

    let metadata = wit_types::TranscriptionMetadata {
        duration_seconds: duration,
        audio_size_bytes,
        request_id: "azure-request".to_string(),
        model,
        language,
    };

    Ok(wit_types::TranscriptionResult {
        alternatives: alts,
        metadata,
    })
}

fn timespan_to_secs(ts: &Option<String>) -> f32 {
    // Azure often returns "PT#S" ISO8601 or 100ns ticks (e.g., "12345600").
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

pub fn to_wit_result(out: TranscriptionResultOut) -> wit_types::TranscriptionResult {
    let alternatives = out
        .alternatives
        .into_iter()
        .map(|a| wit_types::TranscriptAlternative {
            text: a.text,
            confidence: a.confidence,
            words: a
                .words
                .into_iter()
                .map(|w| wit_types::WordSegment {
                    text: w.text,
                    start_time: w.start_time,
                    end_time: w.end_time,
                    confidence: w.confidence,
                    speaker_id: w.speaker_id,
                })
                .collect(),
        })
        .collect();

    let md = out.metadata;
    let metadata = wit_types::TranscriptionMetadata {
        duration_seconds: md.duration_seconds,
        audio_size_bytes: md.audio_size_bytes,
        request_id: md.request_id,
        model: md.model,
        language: md.language,
    };

    wit_types::TranscriptionResult {
        alternatives,
        metadata,
    }
}

use golem_stt::errors::{extract_azure_error_message, InternalSttError};

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
    }
}

#[allow(dead_code)]
pub fn to_wit_error_from_azure(status: u16, body: &str) -> wit_types::SttError {
    let message = extract_azure_error_message(body);
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

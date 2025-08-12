use golem_stt::exports::golem::stt::types as wit_types;

#[derive(Debug, serde::Deserialize)]
struct GoogleWordInfo {
    #[serde(rename = "startTime")]
    start_time: Option<String>,
    #[serde(rename = "endTime")]
    end_time: Option<String>,
    word: Option<String>,
    confidence: Option<f32>,
    #[serde(rename = "speakerTag")]
    speaker_tag: Option<i32>,
}

#[derive(Debug, serde::Deserialize)]
struct GoogleAlternative {
    transcript: String,
    confidence: Option<f32>,
    words: Option<Vec<GoogleWordInfo>>,
}

#[derive(Debug, serde::Deserialize)]
struct GoogleResult {
    alternatives: Vec<GoogleAlternative>,
}

#[derive(Debug, serde::Deserialize)]
struct GoogleResponse {
    results: Option<Vec<GoogleResult>>,
}

fn parse_duration_secs(s: &str) -> f32 {
    // Formats like "12.345s" or "3.4s"
    let trimmed = s.trim_end_matches('s');
    trimmed.parse::<f32>().unwrap_or(0.0)
}

pub fn to_wit_result(
    body: &str,
    audio_size_bytes: u32,
    language: String,
    model: Option<String>,
) -> Result<wit_types::TranscriptionResult, wit_types::SttError> {
    let parsed: GoogleResponse = serde_json::from_str(body)
        .map_err(|e| wit_types::SttError::InternalError(format!("google parse error: {e}")))?;

    let mut alternatives_out: Vec<wit_types::TranscriptAlternative> = Vec::new();

    if let Some(results) = parsed.results {
        if let Some(first) = results.first() {
            if let Some(alt) = first.alternatives.first() {
                let words = alt
                    .words
                    .as_ref()
                    .map(|ws| {
                        ws.iter()
                            .map(|w| wit_types::WordSegment {
                                text: w.word.clone().unwrap_or_default(),
                                start_time: w
                                    .start_time
                                    .as_ref()
                                    .map(|s| parse_duration_secs(s))
                                    .unwrap_or(0.0),
                                end_time: w
                                    .end_time
                                    .as_ref()
                                    .map(|s| parse_duration_secs(s))
                                    .unwrap_or(0.0),
                                confidence: w.confidence,
                                speaker_id: w.speaker_tag.map(|t| t.to_string()),
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                alternatives_out.push(wit_types::TranscriptAlternative {
                    text: alt.transcript.clone(),
                    confidence: alt.confidence.unwrap_or(1.0),
                    words,
                });
            }
        }
    }

    let metadata = wit_types::TranscriptionMetadata {
        duration_seconds: 0.0, // Not provided directly by Google synchronous recognize
        audio_size_bytes,
        request_id: format!("google-{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()), // Google doesn't provide request IDs
        model,
        language,
    };

    Ok(wit_types::TranscriptionResult {
        alternatives: alternatives_out,
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
        InternalSttError::Timeout(m) => wit_types::SttError::TranscriptionFailed(format!("Timeout: {m}")),
    }
}

#[allow(dead_code)]
pub fn to_wit_error_from_google(status: u16, body: &str) -> wit_types::SttError {
    let message = extract_google_error_message(body);
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

use golem_stt::exports::golem::stt::types as wit_types;
use serde::Deserialize;

// This is a simplified structure similar to AWS Transcribe Streaming JSON result. For batch REST,
// providers often return a job artifact; in our simplified REST endpoint, assume direct JSON result.
#[derive(Debug, Deserialize)]
struct AwsAltWord {
    text: String,
    #[serde(default)]
    start_time: f32,
    #[serde(default)]
    end_time: f32,
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default)]
    speaker_label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AwsAlternative {
    transcript: String,
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default)]
    words: Vec<AwsAltWord>,
}

#[derive(Debug, Deserialize)]
struct AwsResponse {
    #[serde(default)]
    alternatives: Vec<AwsAlternative>,
}

pub fn to_wit_result(
    body: &str,
    audio_size_bytes: u32,
    language: String,
    model: Option<String>,
) -> Result<wit_types::TranscriptionResult, wit_types::SttError> {
    let parsed: AwsResponse = serde_json::from_str(body)
        .map_err(|e| wit_types::SttError::InternalError(format!("aws parse error: {e}")))?;

    let mut alts_out: Vec<wit_types::TranscriptAlternative> = Vec::new();

    for a in parsed.alternatives {
        let words = a
            .words
            .into_iter()
            .map(|w| wit_types::WordSegment {
                text: w.text,
                start_time: w.start_time,
                end_time: w.end_time,
                confidence: w.confidence,
                speaker_id: w.speaker_label,
            })
            .collect::<Vec<_>>();

        alts_out.push(wit_types::TranscriptAlternative {
            text: a.transcript,
            confidence: a.confidence.unwrap_or(1.0),
            words,
        });
    }

    let metadata = wit_types::TranscriptionMetadata {
        duration_seconds: 0.0,
        audio_size_bytes,
        request_id: "".to_string(),
        model,
        language,
    };

    Ok(wit_types::TranscriptionResult {
        alternatives: alts_out,
        metadata,
    })
}

use golem_stt::errors::{extract_aws_error_message, InternalSttError};

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

pub fn to_wit_error_from_aws(status: u16, body: &str) -> wit_types::SttError {
    let message = extract_aws_error_message(body);

    // Handle quota exceeded errors specifically
    if message.contains("quota") || message.contains("limit exceeded") {
        // AWS doesn't provide detailed quota info in error responses
        // Return a generic quota exceeded error
        return wit_types::SttError::QuotaExceeded(wit_types::QuotaInfo {
            used: 1, // Unknown, but exceeded
            limit: 1, // Unknown limit
            reset_time: 0, // AWS doesn't provide reset time
            unit: wit_types::QuotaUnit::Requests,
        });
    }

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

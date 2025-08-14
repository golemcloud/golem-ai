use golem_stt::exports::golem::stt::types as wit_types;

pub fn to_wit_result(
    body: &str,
    audio_size_bytes: u32,
    language: String,
    model: Option<String>,
) -> Result<wit_types::TranscriptionResult, wit_types::SttError> {
    // Parse actual AWS Transcribe transcript file JSON
    let v: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| wit_types::SttError::InternalError(format!("aws parse error: {e}, body: {body}")))?;

    let results = v.get("results").cloned().unwrap_or(serde_json::json!({}));
    let transcripts = results
        .get("transcripts")
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default();
    let full_text = transcripts
        .get(0)
        .and_then(|t| t.get("transcript"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    let mut words_out: Vec<wit_types::WordSegment> = Vec::new();
    if let Some(items) = results.get("items").and_then(|i| i.as_array()) {
        for item in items {
            let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if item_type == "pronunciation" {
                let start = item
                    .get("start_time")
                    .and_then(|s| s.as_str())
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(0.0);
                let end = item
                    .get("end_time")
                    .and_then(|s| s.as_str())
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(start);
                let alt0 = item
                    .get("alternatives")
                    .and_then(|a| a.as_array())
                    .and_then(|arr| arr.get(0))
                    .cloned()
                    .unwrap_or(serde_json::json!({}));
                let text = alt0
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                let confidence = alt0
                    .get("confidence")
                    .and_then(|c| c.as_str())
                    .and_then(|s| s.parse::<f32>().ok());
                words_out.push(wit_types::WordSegment {
                    text,
                    start_time: start,
                    end_time: end,
                    confidence,
                    speaker_id: None,
                });
            }
        }
    }

    let alternative = wit_types::TranscriptAlternative {
        text: full_text,
        confidence: 1.0,
        words: words_out,
    };

    let metadata = wit_types::TranscriptionMetadata {
        duration_seconds: 0.0,
        audio_size_bytes,
        request_id: "aws-transcribe".to_string(),
        model,
        language,
    };

    Ok(wit_types::TranscriptionResult {
        alternatives: vec![alternative],
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
        InternalSttError::Timeout(m) => {
            wit_types::SttError::TranscriptionFailed(format!("Timeout: {m}"))
        }
    }
}

pub fn to_wit_error_from_aws(status: u16, body: &str) -> wit_types::SttError {
    let message = extract_aws_error_message(body);

    // Handle quota exceeded errors specifically
    if message.contains("quota") || message.contains("limit exceeded") {
        // AWS doesn't provide detailed quota info in error responses
        // Return a generic quota exceeded error
        return wit_types::SttError::QuotaExceeded(wit_types::QuotaInfo {
            used: 1,       // Unknown, but exceeded
            limit: 1,      // Unknown limit
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

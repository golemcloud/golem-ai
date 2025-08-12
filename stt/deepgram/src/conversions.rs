use golem_stt::exports::golem::stt::types as wit_types;
use golem_stt::mapping::TranscriptionResultOut;

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

pub fn to_wit_error(err: golem_stt::errors::InternalSttError) -> wit_types::SttError {
    use golem_stt::errors::InternalSttError as I;
    match err {
        I::InvalidAudio(m) => wit_types::SttError::InvalidAudio(m),
        I::UnsupportedFormat(m) => wit_types::SttError::UnsupportedFormat(m),
        I::UnsupportedLanguage(m) => wit_types::SttError::UnsupportedLanguage(m),
        I::TranscriptionFailed(m) => wit_types::SttError::TranscriptionFailed(m),
        I::Unauthorized(m) => wit_types::SttError::Unauthorized(m),
        I::AccessDenied(m) => wit_types::SttError::AccessDenied(m),
        I::QuotaExceeded(q) => wit_types::SttError::QuotaExceeded(wit_types::QuotaInfo {
            used: q.used,
            limit: q.limit,
            reset_time: q.reset_time,
            unit: match q.unit {
                golem_stt::errors::QuotaUnit::Seconds => wit_types::QuotaUnit::Seconds,
                golem_stt::errors::QuotaUnit::Requests => wit_types::QuotaUnit::Requests,
                golem_stt::errors::QuotaUnit::Credits => wit_types::QuotaUnit::Credits,
            },
        }),
        I::RateLimited(s) => wit_types::SttError::RateLimited(s),
        I::InsufficientCredits => wit_types::SttError::InsufficientCredits,
        I::UnsupportedOperation(m) => wit_types::SttError::UnsupportedOperation(m),
        I::ServiceUnavailable(m) => wit_types::SttError::ServiceUnavailable(m),
        I::NetworkError(m) => wit_types::SttError::NetworkError(m),
        I::InternalError(m) => wit_types::SttError::InternalError(m),
        I::Timeout(m) => wit_types::SttError::TranscriptionFailed(format!("Timeout: {m}")),
    }
}

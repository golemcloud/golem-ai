use httpdate::parse_http_date;
use reqwest::{StatusCode};
use std::time::{Duration, SystemTime};

/// Local short alias for the WIT types.
use crate::bindings::exports::golem::tts::types as t;

/// Convert an HTTP failure into a typed tts-error, preserving useful context.
pub fn map_http_error(endpoint: &'static str,
                      status: StatusCode,
                      body: Option<&str>,
                      retry_after: Option<&str>) -> t::TtsError
{
    match status {
        StatusCode::UNAUTHORIZED => t::TtsError::Unauthorized(msg(endpoint, status, body)),
        StatusCode::FORBIDDEN => {
            // Heuristic: ElevenLabs may signal credits/permissions in the message
            if let Some(b) = body {
                let lower = b.to_ascii_lowercase();
                if (lower.contains("insufficient") && lower.contains("credit")) ||
                   lower.contains("out of characters") {
                    return t::TtsError::InsufficientCredits;
                }
            }
            t::TtsError::AccessDenied(msg(endpoint, status, body))
        }
        StatusCode::NOT_FOUND => {
            // Voices & TTS URLs imply voice lookup
            if endpoint.contains("/voices") || endpoint.contains("/text-to-speech/") {
                t::TtsError::VoiceNotFound(msg(endpoint, status, body))
            } else {
                t::TtsError::ModelNotFound(msg(endpoint, status, body))
            }
        }
        StatusCode::UNPROCESSABLE_ENTITY => {
            // 422: try to classify, else invalid-configuration
            classify_422(body).unwrap_or_else(|| t::TtsError::InvalidConfiguration(msg(endpoint, status, body)))
        }
        StatusCode::TOO_MANY_REQUESTS => {
            t::TtsError::RateLimited(parse_retry_after_seconds(retry_after))
        }
        StatusCode::SERVICE_UNAVAILABLE => {
            // If server gives explicit wait, prefer rate-limited with delay
            let secs = parse_retry_after_seconds(retry_after);
            if secs > 0 { t::TtsError::RateLimited(secs) }
            else { t::TtsError::ServiceUnavailable(msg(endpoint, status, body)) }
        }
        s if s.is_server_error() => t::TtsError::InternalError(msg(endpoint, status, body)),
        _ => t::TtsError::SynthesisFailed(msg(endpoint, status, body)),
    }
}

/// Body-aware refinement for 422 Unprocessable Entity.
fn classify_422(body: Option<&str>) -> Option<t::TtsError> {
    let b = body?;
    let lower = b.to_ascii_lowercase();
    if lower.contains("ssml") { return Some(t::TtsError::InvalidSsml(b.to_string())); }
    if lower.contains("language") && lower.contains("unsupported") {
        return Some(t::TtsError::UnsupportedLanguage(b.to_string()));
    }
    if lower.contains("length") || lower.contains("too long") || lower.contains("characters") {
        // Try to pull a number; else use a generic "too long"
        let limit = extract_first_number(&lower).unwrap_or(0);
        return Some(if limit > 0 { t::TtsError::TextTooLong(limit as u32) }
                    else { t::TtsError::InvalidText(b.to_string()) });
    }
    None
}

fn extract_first_number(s: &str) -> Option<usize> {
    let mut n = String::new();
    for ch in s.chars() {
        if ch.is_ascii_digit() { n.push(ch); }
        else if !n.is_empty() { break; }
    }
    if n.is_empty() { None } else { n.parse().ok() }
}

/// Parse Retry-After per spec: delta-seconds *or* HTTP-date.
/// Returns seconds to wait (0 if missing/invalid).
fn parse_retry_after_seconds(v: Option<&str>) -> u32 {
    if let Some(raw) = v {
        let raw = raw.trim();
        if let Ok(sec) = raw.parse::<u32>() { return sec; }
        if let Ok(when) = parse_http_date(raw) {
            if let Ok(dur) = when.duration_since(SystemTime::now()) {
                return dur.as_secs().min(u32::MAX as u64) as u32;
            }
        }
    }
    0
}

fn msg(endpoint: &str, status: StatusCode, body: Option<&str>) -> String {
    match body {
        Some(b) if !b.is_empty() => format!("{} -> {}: {}", endpoint, status.as_u16(), truncate(b)),
        _ => format!("{} -> {}", endpoint, status.as_u16()),
    }
}

/// Keep error messages compact.
fn truncate(s: &str) -> String {
    const MAX: usize = 400;
    if s.len() <= MAX { s.to_string() } else { format!("{}…", &s[..MAX]) }
}

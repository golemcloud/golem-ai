//! Core type definitions for TTS-ElevenLabs
//!
//! Maps WIT types to internal Rust representations.

use crate::wit_types;

// ============================================================
// INTERNAL TYPE ALIASES
// ============================================================

/// Re-export WIT error type for internal use
pub type TtsError = wit_types::TtsError;

/// Audio format enumeration
pub type AudioFormat = wit_types::AudioFormat;

/// Voice quality tier
pub type VoiceQuality = wit_types::VoiceQuality;

/// Voice gender classification
pub type VoiceGender = wit_types::VoiceGender;

/// Text input type (plain or SSML)
pub type TextType = wit_types::TextType;

// ============================================================
// ERROR CONSTRUCTION HELPERS
// ============================================================

/// Create an internal error with context
pub fn internal_error(message: &str) -> TtsError {
    return TtsError::InternalError(message.to_string());
}

/// Create a network error
pub fn network_error(message: &str) -> TtsError {
    return TtsError::NetworkError(message.to_string());
}

/// Create an unauthorized error
pub fn unauthorized_error(message: &str) -> TtsError {
    return TtsError::Unauthorized(message.to_string());
}

/// Create a voice not found error
pub fn voice_not_found_error(voice_id: &str) -> TtsError {
    return TtsError::VoiceNotFound(voice_id.to_string());
}

/// Create an unsupported operation error
pub fn unsupported_operation_error(operation: &str) -> TtsError {
    return TtsError::UnsupportedOperation(operation.to_string());
}

/// Create an invalid text error
pub fn invalid_text_error(message: &str) -> TtsError {
    return TtsError::InvalidText(message.to_string());
}

/// Create a synthesis failed error
pub fn synthesis_failed_error(message: &str) -> TtsError {
    return TtsError::SynthesisFailed(message.to_string());
}

/// Create a rate limited error
pub fn rate_limited_error(retry_after_seconds: u32) -> TtsError {
    return TtsError::RateLimited(retry_after_seconds);
}

/// Create a service unavailable error
pub fn service_unavailable_error(message: &str) -> TtsError {
    return TtsError::ServiceUnavailable(message.to_string());
}

// ============================================================
// ELEVENLABS API RESPONSE TYPES
// ============================================================

/// ElevenLabs voice from API response
#[derive(serde::Deserialize)]
pub struct ElevenLabsVoice {
    pub voice_id: String,
    pub name: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub preview_url: Option<String>,
    #[serde(default)]
    pub labels: Option<ElevenLabsVoiceLabels>,
    #[serde(default)]
    pub settings: Option<ElevenLabsVoiceSettings>,
}

/// Voice labels from ElevenLabs
#[derive(serde::Deserialize, Default)]
pub struct ElevenLabsVoiceLabels {
    #[serde(default)]
    pub accent: Option<String>,
    #[serde(default)]
    pub age: Option<String>,
    #[serde(default)]
    pub gender: Option<String>,
    #[serde(default)]
    pub use_case: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Voice settings from ElevenLabs
#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct ElevenLabsVoiceSettings {
    #[serde(default)]
    pub stability: Option<f64>,
    #[serde(default)]
    pub similarity_boost: Option<f64>,
    #[serde(default)]
    pub style: Option<f64>,
    #[serde(default)]
    pub use_speaker_boost: Option<bool>,
}

/// ElevenLabs voices list response
#[derive(serde::Deserialize)]
pub struct ElevenLabsVoicesResponse {
    pub voices: Vec<ElevenLabsVoice>,
}

/// ElevenLabs TTS request body
#[derive(serde::Serialize)]
pub struct ElevenLabsTtsRequest {
    pub text: String,
    pub model_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_settings: Option<ElevenLabsVoiceSettings>,
}

/// ElevenLabs error response
#[derive(serde::Deserialize)]
pub struct ElevenLabsErrorResponse {
    #[serde(default)]
    pub detail: Option<ElevenLabsErrorDetail>,
}

/// Error detail from ElevenLabs
#[derive(serde::Deserialize)]
pub struct ElevenLabsErrorDetail {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

/// ElevenLabs voice clone request
#[derive(serde::Serialize)]
pub struct ElevenLabsVoiceCloneRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub files: Vec<String>, // Base64 encoded audio files
}

/// ElevenLabs sound generation request
#[derive(serde::Serialize)]
pub struct ElevenLabsSoundGenRequest {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_influence: Option<f32>,
}

// ============================================================
// CONVERSION HELPERS
// ============================================================

/// Map ElevenLabs gender string to WIT enum
pub fn map_gender(gender_str: Option<&str>) -> VoiceGender {
    match gender_str {
        Some("male") => {
            return VoiceGender::Male;
        }
        Some("female") => {
            return VoiceGender::Female;
        }
        _ => {
            return VoiceGender::Neutral;
        }
    }
}

/// Map ElevenLabs category to voice quality
pub fn map_quality(category: Option<&str>) -> VoiceQuality {
    match category {
        Some("premade") => {
            return VoiceQuality::Standard;
        }
        Some("cloned") => {
            return VoiceQuality::Premium;
        }
        Some("generated") => {
            return VoiceQuality::Neural;
        }
        Some("professional") => {
            return VoiceQuality::Studio;
        }
        _ => {
            return VoiceQuality::Standard;
        }
    }
}

/// Map WIT audio format to ElevenLabs output format string
pub fn map_audio_format(format: AudioFormat) -> &'static str {
    match format {
        AudioFormat::Mp3 => {
            return "mp3_44100_128";
        }
        AudioFormat::Wav => {
            return "pcm_44100";
        }
        AudioFormat::Pcm => {
            return "pcm_44100";
        }
        AudioFormat::OggOpus => {
            return "mp3_44100_128"; // Fallback - ElevenLabs doesn't support Opus
        }
        AudioFormat::Aac => {
            return "mp3_44100_128"; // Fallback
        }
        AudioFormat::Flac => {
            return "pcm_44100"; // Fallback
        }
        AudioFormat::Mulaw => {
            return "ulaw_8000";
        }
        AudioFormat::Alaw => {
            return "ulaw_8000"; // Fallback - use ulaw
        }
    }
}

/// Count words in text (simple whitespace split)
pub fn count_words(text: &str) -> u32 {
    let mut count: u32 = 0;
    let mut in_word = false;

    for c in text.chars() {
        if c.is_whitespace() {
            in_word = false;
        } else if !in_word {
            in_word = true;
            count = count + 1;
        }
    }

    return count;
}

/// Estimate audio duration from character count (rough estimate)
pub fn estimate_duration(char_count: u32) -> f32 {
    // Average speaking rate: ~150 words per minute, ~5 chars per word
    // So ~750 chars per minute = 12.5 chars per second
    let chars_per_second: f32 = 12.5;
    return (char_count as f32) / chars_per_second;
}

//! Types and conversion helpers for AWS Polly
//!
//! Maps WIT types to AWS Polly API structures.

use crate::wit_types;
use serde::{Deserialize, Serialize};

// ============================================================
// AWS POLLY API STRUCTURES
// ============================================================

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PollySynthesizeRequest {
    pub output_format: String,
    pub text: String,
    pub voice_id: String,
    pub engine: Option<String>,
    pub language_code: Option<String>,
    pub sample_rate: Option<String>,
    pub text_type: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PollyVoiceList {
    pub voices: Vec<PollyVoice>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PollyVoice {
    pub id: String,
    pub name: String,
    pub gender: String,
    pub language_code: String,
    pub language_name: String,
    pub supported_engines: Vec<String>,
}

// ============================================================
// CONVERSION HELPERS
// ============================================================

pub fn map_gender(gender: &str) -> wit_types::VoiceGender {
    return match gender.to_lowercase().as_str() {
        "male" => wit_types::VoiceGender::Male,
        "female" => wit_types::VoiceGender::Female,
        _ => wit_types::VoiceGender::Neutral,
    };
}

pub fn map_audio_format(format: wit_types::AudioFormat) -> &'static str {
    return match format {
        wit_types::AudioFormat::Mp3 => "mp3",
        wit_types::AudioFormat::Pcm => "pcm",
        wit_types::AudioFormat::OggOpus => "ogg_vorbis",
        _ => "mp3",
    };
}

pub fn internal_error(msg: &str) -> wit_types::TtsError {
    return wit_types::TtsError::InternalError(format!("AWS Polly Error: {}", msg));
}

pub fn auth_error(msg: &str) -> wit_types::TtsError {
    return wit_types::TtsError::Unauthorized(msg.to_string());
}

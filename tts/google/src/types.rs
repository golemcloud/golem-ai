//! Google Cloud TTS API Types
//!
//! Mapping between WIT types and Google's JSON API.
//! Adheres to best practices (no unwrap, explicit error mapping).

use crate::bindings::exports::golem::tts::types::{
    AudioFormat, TtsError, VoiceGender, VoiceQuality,
};
use serde::{Deserialize, Serialize};

// ============================================================
// REQUEST TYPES
// ============================================================

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GoogleSynthesizeRequest {
    pub input: GoogleTextInput,
    pub voice: GoogleVoiceSelection,
    pub audio_config: GoogleAudioConfig,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GoogleTextInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssml: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GoogleVoiceSelection {
    pub language_code: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssml_gender: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GoogleAudioConfig {
    pub audio_encoding: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaking_rate: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pitch: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_gain_db: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate_hertz: Option<u32>,
}

// ============================================================
// RESPONSE TYPES
// ============================================================

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GoogleSynthesizeResponse {
    pub audio_content: String, // Base64 encoded
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GoogleVoiceListResponse {
    pub voices: Vec<GoogleVoice>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GoogleVoice {
    pub name: String,
    pub language_codes: Vec<String>,
    pub ssml_gender: String,
    pub natural_sample_rate_hertz: u32,
}

// ============================================================
// HELPERS & MAPPING
// ============================================================

pub fn map_audio_format(format: AudioFormat) -> &'static str {
    match format {
        AudioFormat::Mp3 => "MP3",
        AudioFormat::Wav => "LINEAR16",
        AudioFormat::Pcm => "LINEAR16",
        AudioFormat::OggOpus => "OGG_OPUS",
        AudioFormat::Mulaw => "MULAW",
        AudioFormat::Alaw => "ALAW",
        _ => "MP3",
    }
}

pub fn map_gender(gender: VoiceGender) -> &'static str {
    match gender {
        VoiceGender::Male => "MALE",
        VoiceGender::Female => "FEMALE",
        VoiceGender::Neutral => "NEUTRAL",
    }
}

pub fn reverse_map_gender(gender: &str) -> VoiceGender {
    match gender {
        "MALE" => VoiceGender::Male,
        "FEMALE" => VoiceGender::Female,
        _ => VoiceGender::Neutral,
    }
}

pub fn infer_quality(name: &str) -> VoiceQuality {
    if name.contains("Neural2") {
        VoiceQuality::Neural
    } else if name.contains("Studio") || name.contains("Polyglot") {
        VoiceQuality::Studio
    } else if name.contains("Wavenet") {
        VoiceQuality::Premium
    } else {
        VoiceQuality::Standard
    }
}

pub fn internal_error(msg: &str) -> TtsError {
    TtsError::InternalError(msg.to_string())
}

pub fn unauthorized_error(msg: &str) -> TtsError {
    TtsError::Unauthorized(msg.to_string())
}

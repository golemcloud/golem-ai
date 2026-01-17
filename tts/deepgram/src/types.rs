//! Deepgram Aura TTS API Types
//!
//! Mapping between WIT types and Deepgram's REST API.
//! Adheres to best practices (no unwrap, explicit error mapping).

use crate::bindings::exports::golem::tts::types::{
    AudioFormat, TtsError, VoiceGender, VoiceQuality,
};
use serde::{Deserialize, Serialize};

// ============================================================
// REQUEST TYPES
// ============================================================

#[derive(Serialize, Debug)]
pub struct DeepgramSpeakRequest {
    pub text: String,
}

// Deepgram options are passed as query parameters:
// model, encoding, sample_rate, container

// ============================================================
// RESPONSE TYPES
// ============================================================

// Deepgram /v1/speak returns raw audio bytes, not JSON
// Headers contain metadata like 'x-dg-request-id'

// ============================================================
// VOICE LIST TYPES
// ============================================================

#[derive(Deserialize, Debug)]
pub struct DeepgramVoice {
    pub name: String,
    pub accent: String,
    pub language: String,
    pub gender: String,
}

// Deepgram currently doesn't have a public "list voices" REST endpoint
// similar to ElevenLabs/Google. Most integrations use a hardcoded list
// or fetch from docs. However, for this component, we'll provide
// the latest known Aura voices.

pub const DEEPGRAM_VOICES: &[(&str, &str, &str)] = &[
    ("aura-asteria-en", "en", "Female"),
    ("aura-luna-en", "en", "Female"),
    ("aura-stella-en", "en", "Female"),
    ("aura-athena-en", "en", "Female"),
    ("aura-hera-en", "en", "Female"),
    ("aura-orion-en", "en", "Male"),
    ("aura-arcas-en", "en", "Male"),
    ("aura-perseus-en", "en", "Male"),
    ("aura-angus-en", "en", "Male"),
    ("aura-orpheus-en", "en", "Male"),
    ("aura-helios-en", "en", "Male"),
    ("aura-zeus-en", "en", "Male"),
];

// ============================================================
// HELPERS & MAPPING
// ============================================================

pub fn map_audio_format(format: AudioFormat) -> (&'static str, &'static str) {
    match format {
        AudioFormat::Mp3 => ("mp3", "none"),
        AudioFormat::Wav => ("linear16", "wav"),
        AudioFormat::Pcm => ("linear16", "none"),
        AudioFormat::OggOpus => ("opus", "ogg"),
        AudioFormat::Aac => ("aac", "none"),
        AudioFormat::Flac => ("flac", "none"),
        _ => ("mp3", "none"),
    }
}

pub fn reverse_map_gender(gender: &str) -> VoiceGender {
    match gender.to_lowercase().as_str() {
        "male" => VoiceGender::Male,
        "female" => VoiceGender::Female,
        _ => VoiceGender::Neutral,
    }
}

pub fn internal_error(msg: &str) -> TtsError {
    TtsError::InternalError(msg.to_string())
}

pub fn unauthorized_error(msg: &str) -> TtsError {
    TtsError::Unauthorized(msg.to_string())
}

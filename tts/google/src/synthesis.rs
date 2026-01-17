//! Synthesis implementation for Google Cloud TTS
//!
//! Handles text-to-speech conversion using Google's REST API.

use crate::bindings::exports::golem::tts::synthesis::{Guest, SynthesisOptions, ValidationResult};
use crate::bindings::exports::golem::tts::types::{
    SynthesisMetadata, SynthesisResult, TextInput, TextType, TimingInfo, TtsError,
};
use crate::client;
use crate::types;
use crate::voices::VoiceImpl;
use base64::{engine::general_purpose, Engine as _};

pub fn synthesize(
    input: TextInput,
    voice: &VoiceImpl,
    options: Option<SynthesisOptions>,
) -> Result<SynthesisResult, TtsError> {
    // 1. Map input
    let google_input = if input.text_type == TextType::Ssml {
        types::GoogleTextInput {
            text: None,
            ssml: Some(input.content.clone()),
        }
    } else {
        types::GoogleTextInput {
            text: Some(input.content.clone()),
            ssml: None,
        }
    };

    // 2. Map voice
    let google_voice = types::GoogleVoiceSelection {
        language_code: voice.language_code.clone(),
        name: voice.name.clone(),
        ssml_gender: Some(types::map_gender(voice.gender).to_string()),
    };

    // 3. Map config
    let mut audio_config = types::GoogleAudioConfig {
        audio_encoding: "MP3".to_string(), // Default
        speaking_rate: None,
        pitch: None,
        volume_gain_db: None,
        sample_rate_hertz: None,
    };

    if let Some(ref opts) = options {
        if let Some(ref cfg) = opts.audio_config {
            audio_config.audio_encoding = types::map_audio_format(cfg.format).to_string();
            audio_config.sample_rate_hertz = cfg.sample_rate;
        }
        if let Some(ref settings) = opts.voice_settings {
            audio_config.speaking_rate = settings.speed;
            audio_config.pitch = settings.pitch;
            audio_config.volume_gain_db = settings.volume;
        }
    }

    // 4. Construct request
    let request = types::GoogleSynthesizeRequest {
        input: google_input,
        voice: google_voice,
        audio_config,
    };

    let body_bytes = serde_json::to_vec(&request)
        .map_err(|_| types::internal_error("Failed to serialize request"))?;

    // 5. Call Google API
    let response_bytes = client::post_request("/text:synthesize", &body_bytes)?;
    let response: types::GoogleSynthesizeResponse = serde_json::from_slice(&response_bytes)
        .map_err(|_| types::internal_error("Failed to parse Google response"))?;

    // 6. Decode Base64 audio
    let audio_data = general_purpose::STANDARD
        .decode(&response.audio_content)
        .map_err(|_| types::internal_error("Failed to decode base64 audio"))?;

    // 7. Metadata
    let char_count = input.content.len() as u32;
    let metadata = SynthesisMetadata {
        duration_seconds: char_count as f32 / 12.0, // Rough estimate
        character_count: char_count,
        word_count: input.content.split_whitespace().count() as u32,
        audio_size_bytes: audio_data.len() as u32,
        request_id: "".to_string(), // Google doesn't easily expose this in standard response
        provider_info: Some("google".to_string()),
    };

    Ok(SynthesisResult {
        audio_data,
        metadata,
    })
}

pub fn synthesize_batch(
    inputs: Vec<TextInput>,
    voice: &VoiceImpl,
    options: Option<SynthesisOptions>,
) -> Result<Vec<SynthesisResult>, TtsError> {
    let mut results = Vec::new();
    for input in inputs {
        results.push(synthesize(input, voice, options.clone())?);
    }
    Ok(results)
}

pub fn get_timing_marks(
    _input: TextInput,
    _voice: &VoiceImpl,
) -> Result<Vec<TimingInfo>, TtsError> {
    // Google supports timing marks (timepoints) but it requires complex request structure
    // and is not available for all voices. We'll return empty for now.
    Ok(Vec::new())
}

pub fn validate_input(input: TextInput, _voice: &VoiceImpl) -> Result<ValidationResult, TtsError> {
    let char_count = input.content.len() as u32;
    if char_count > 5000 {
        return Ok(ValidationResult {
            is_valid: false,
            character_count: char_count,
            estimated_duration: None,
            warnings: Vec::new(),
            errors: vec![
                "Text exceeds Google's 5000 character limit for sync synthesis".to_string(),
            ],
        });
    }
    Ok(ValidationResult {
        is_valid: true,
        character_count: char_count,
        estimated_duration: Some(char_count as f32 / 12.0),
        warnings: Vec::new(),
        errors: Vec::new(),
    })
}

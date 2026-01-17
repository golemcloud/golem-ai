//! Synthesis implementation for Deepgram Aura TTS
//!
//! Handles text-to-speech conversion using Deepgram's REST API.

use crate::bindings::exports::golem::tts::synthesis::{SynthesisOptions, ValidationResult};
use crate::bindings::exports::golem::tts::types::{
    SynthesisMetadata, SynthesisResult, TextInput, TimingInfo, TtsError,
};
use crate::client;
use crate::types;
use crate::voices::VoiceImpl;

pub fn synthesize(
    input: TextInput,
    voice: &VoiceImpl,
    options: Option<SynthesisOptions>,
) -> Result<SynthesisResult, TtsError> {
    // 1. Determine model and encoding
    let model = voice.id.clone();
    let (encoding, container) = if let Some(ref opts) = options {
        if let Some(ref cfg) = opts.audio_config {
            types::map_audio_format(cfg.format)
        } else {
            ("mp3", "none")
        }
    } else {
        ("mp3", "none")
    };

    let mut query_params = format!(
        "model={}&encoding={}&container={}",
        model, encoding, container
    );

    if let Some(ref opts) = options {
        if let Some(ref cfg) = opts.audio_config {
            if let Some(sample_rate) = cfg.sample_rate {
                query_params.push_str(&format!("&sample_rate={}", sample_rate));
            }
        }
    }

    // 2. Construct request body
    let request = types::DeepgramSpeakRequest {
        text: input.content.clone(),
    };
    let body_bytes = serde_json::to_vec(&request)
        .map_err(|_| types::internal_error("Failed to serialize request"))?;

    // 3. Call Deepgram API
    let audio_data = client::post_speak(&query_params, &body_bytes)?;

    // 4. Metadata
    let char_count = input.content.len() as u32;
    let metadata = SynthesisMetadata {
        duration_seconds: char_count as f32 / 15.0, // Aura is fast, ~15-20 chars/sec
        character_count: char_count,
        word_count: input.content.split_whitespace().count() as u32,
        audio_size_bytes: audio_data.len() as u32,
        request_id: "".to_string(), // Could extract from headers if needed
        provider_info: Some("deepgram".to_string()),
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

pub fn get_timing_marks(_: TextInput, _: &VoiceImpl) -> Result<Vec<TimingInfo>, TtsError> {
    // Deepgram currently doesn't return timing marks in the REST response for /v1/speak
    Ok(Vec::new())
}

pub fn validate_input(input: TextInput, _: &VoiceImpl) -> Result<ValidationResult, TtsError> {
    let char_count = input.content.len() as u32;
    if char_count > 2000 {
        return Ok(ValidationResult {
            is_valid: false,
            character_count: char_count,
            estimated_duration: None,
            warnings: Vec::new(),
            errors: vec![
                "Deepgram Aura recommended limit is ~2000 characters for sync requests".to_string(),
            ],
        });
    }
    Ok(ValidationResult {
        is_valid: true,
        character_count: char_count,
        estimated_duration: Some(char_count as f32 / 15.0),
        warnings: Vec::new(),
        errors: Vec::new(),
    })
}

//! Synthesis interface implementation for AWS Polly

use crate::client;
use crate::types;
use crate::voices::VoiceImpl;
use crate::wit_synthesis;
use crate::wit_types;

pub fn synthesize(
    input: wit_types::TextInput,
    voice: &VoiceImpl,
    options: Option<wit_synthesis::SynthesisOptions>,
) -> Result<wit_types::SynthesisResult, wit_types::TtsError> {
    let engine = std::env::var("POLLY_ENGINE").unwrap_or_else(|_| voice.best_engine().to_string());

    let format = options
        .as_ref()
        .and_then(|o| o.audio_config.as_ref())
        .map(|c| types::map_audio_format(c.format.clone()))
        .unwrap_or("mp3");

    let audio_data = client::synthesize_api(voice.voice_id(), &input.content, &engine, format)?;

    return Ok(wit_types::SynthesisResult {
        audio_data,
        metadata: wit_types::SynthesisMetadata {
            duration_seconds: 0.0, // Polly doesn't return duration directly in sync call
            character_count: input.content.len() as u32,
            word_count: 0,
            audio_size_bytes: 0,
            request_id: "polly-request".to_string(),
            provider_info: Some("aws-polly".to_string()),
        },
    });
}

pub fn synthesize_batch(
    inputs: Vec<wit_types::TextInput>,
    voice: &VoiceImpl,
    options: Option<wit_synthesis::SynthesisOptions>,
) -> Result<Vec<wit_types::SynthesisResult>, wit_types::TtsError> {
    let mut results = Vec::new();
    for input in inputs {
        results.push(synthesize(input, voice, options.clone())?);
    }
    return Ok(results);
}

pub fn get_timing_marks(
    _input: wit_types::TextInput,
    _voice: &VoiceImpl,
) -> Result<Vec<wit_types::TimingInfo>, wit_types::TtsError> {
    // Polly supports speech marks but requires a separate call or specific format
    return Ok(Vec::new());
}

pub fn validate_input(
    input: wit_types::TextInput,
    _voice: &VoiceImpl,
) -> Result<wit_synthesis::ValidationResult, wit_types::TtsError> {
    let char_count = input.content.len() as u32;
    if char_count > 3000 {
        return Ok(wit_synthesis::ValidationResult {
            is_valid: false,
            character_count: char_count,
            estimated_duration: None,
            warnings: Vec::new(),
            errors: vec![
                "Text exceeds Polly's 3000 character limit for sync synthesis".to_string(),
            ],
        });
    }
    return Ok(wit_synthesis::ValidationResult {
        is_valid: true,
        character_count: char_count,
        estimated_duration: Some(char_count as f32 / 12.0), // Simple estimate
        warnings: Vec::new(),
        errors: Vec::new(),
    });
}

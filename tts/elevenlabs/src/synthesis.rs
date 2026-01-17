//! Synthesis interface implementation for ElevenLabs
//!
//! Core text-to-speech operations with durability.

use crate::client;
use crate::types;
use crate::voices::VoiceImpl;
use crate::wit_synthesis;
use crate::wit_types;

// ============================================================
// SYNTHESIS FUNCTIONS
// ============================================================

/// Synthesize text to speech audio
pub fn synthesize(
    input: wit_types::TextInput,
    voice: &VoiceImpl,
    options: Option<wit_synthesis::SynthesisOptions>,
) -> Result<wit_types::SynthesisResult, wit_types::TtsError> {
    // Validate input first (C5 - check before action)
    let validation = validate_input_internal(&input, voice)?;
    if !validation.is_valid {
        let error_msg = if validation.errors.is_empty() {
            "Invalid input".to_string()
        } else {
            validation.errors.join("; ")
        };
        return Err(types::invalid_text_error(&error_msg));
    }

    // Determine model and format
    let model_id = options
        .as_ref()
        .and_then(|o| o.model_version.clone())
        .unwrap_or_else(|| client::get_model_version());

    let audio_format = options
        .as_ref()
        .and_then(|o| o.audio_config.as_ref())
        .map(|c| types::map_audio_format(c.format.clone()))
        .unwrap_or("mp3_44100_128");

    // Call ElevenLabs API (wrapped in atomic operation internally)
    let audio_data =
        client::synthesize_api(voice.voice_id(), &input.content, &model_id, audio_format)?;

    // Build metadata
    let char_count = input.content.len() as u32;
    let word_count = types::count_words(&input.content);
    let duration = types::estimate_duration(char_count);
    let audio_size = audio_data.len() as u32;

    let request_id = generate_request_id();

    let metadata = wit_types::SynthesisMetadata {
        duration_seconds: duration,
        character_count: char_count,
        word_count,
        audio_size_bytes: audio_size,
        request_id,
        provider_info: Some("elevenlabs".to_string()),
    };

    let result = wit_types::SynthesisResult {
        audio_data,
        metadata,
    };

    return Ok(result);
}

/// Batch synthesize multiple inputs
pub fn synthesize_batch(
    inputs: Vec<wit_types::TextInput>,
    voice: &VoiceImpl,
    options: Option<wit_synthesis::SynthesisOptions>,
) -> Result<Vec<wit_types::SynthesisResult>, wit_types::TtsError> {
    let mut results: Vec<wit_types::SynthesisResult> = Vec::new();

    // Process each input sequentially (maintain order)
    for input in inputs.iter() {
        let result = synthesize(input.clone(), voice, options.clone())?;
        results.push(result);
    }

    return Ok(results);
}

/// Get timing marks for text (word boundaries)
pub fn get_timing_marks(
    input: wit_types::TextInput,
    _voice: &VoiceImpl,
) -> Result<Vec<wit_types::TimingInfo>, wit_types::TtsError> {
    // ElevenLabs doesn't provide timing marks directly
    // We'll generate estimated timing based on text analysis

    let mut timing_marks: Vec<wit_types::TimingInfo> = Vec::new();
    let mut current_time: f32 = 0.0;
    let mut text_offset: u32 = 0;

    // Split by whitespace and estimate timing
    let chars_per_second: f32 = 12.5;

    let words: Vec<&str> = input.content.split_whitespace().collect();

    for word in words.iter() {
        let word_len = word.len() as f32;
        let word_duration = word_len / chars_per_second;

        let timing = wit_types::TimingInfo {
            start_time_seconds: current_time,
            end_time_seconds: Some(current_time + word_duration),
            text_offset: Some(text_offset),
            mark_type: Some(wit_types::TimingMarkType::Word),
        };

        timing_marks.push(timing);

        current_time = current_time + word_duration;
        text_offset = text_offset + (word.len() as u32) + 1; // +1 for space
    }

    return Ok(timing_marks);
}

/// Validate input text before synthesis
pub fn validate_input(
    input: wit_types::TextInput,
    voice: &VoiceImpl,
) -> Result<wit_synthesis::ValidationResult, wit_types::TtsError> {
    return validate_input_internal(&input, voice);
}

// ============================================================
// INTERNAL HELPERS
// ============================================================

/// Internal validation implementation
fn validate_input_internal(
    input: &wit_types::TextInput,
    _voice: &VoiceImpl,
) -> Result<wit_synthesis::ValidationResult, wit_types::TtsError> {
    let mut warnings: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    let char_count = input.content.len() as u32;

    // Check for empty text
    if input.content.is_empty() {
        errors.push("Text content is empty".to_string());
    }

    // Check text length (ElevenLabs limit is around 5000 characters per request)
    let max_chars: u32 = 5000;
    if char_count > max_chars {
        errors.push(format!(
            "Text exceeds maximum length of {} characters",
            max_chars
        ));
    }

    // Warn for very long text
    if char_count > 2500 {
        warnings.push("Long text may take longer to process".to_string());
    }

    // Check for SSML if text type is SSML
    if input.text_type == wit_types::TextType::Ssml {
        // Basic SSML validation
        if !input.content.contains("<speak") {
            warnings.push("SSML should start with <speak> tag".to_string());
        }
    }

    let is_valid = errors.is_empty();
    let estimated_duration = if is_valid {
        Some(types::estimate_duration(char_count))
    } else {
        None
    };

    let result = wit_synthesis::ValidationResult {
        is_valid,
        character_count: char_count,
        estimated_duration,
        warnings,
        errors,
    };

    return Ok(result);
}

/// Generate a unique request ID
fn generate_request_id() -> String {
    // Use a combination of timestamp-like counter and random-ish bytes
    // In a real implementation, we'd use golem's idempotency key generation
    use core::sync::atomic::AtomicU64;
    use core::sync::atomic::Ordering;

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::SeqCst);

    return format!("tts-el-{:016x}", count);
}

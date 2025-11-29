use golem_tts::golem::tts::{types::VoiceGender, voices::Voice};

use crate::resources::VoiceResponse;

impl From<VoiceResponse> for Voice {
    fn from(voice: VoiceResponse) -> Self {
        Voice {
            id: voice.name.clone(),
            name: voice.name.clone(),
            language: voice
                .language_codes
                .first()
                .unwrap_or(&"en-US".to_string())
                .clone(),
            additional_languages: voice.language_codes.iter().skip(1).cloned().collect(),
            gender: match voice.ssml_gender.as_str() {
                "MALE" => VoiceGender::Male,
                "FEMALE" => VoiceGender::Female,
                _ => VoiceGender::Neutral,
            },
            supports_ssml: true,
            provider: "google".to_string(),
            sample_rate: vec![voice.natural_sample_rate_hertz],
            is_custom: false,
            is_cloned: false,
            preview_url: None,
            use_cases: vec!["general".to_string()],
            supported_formats: vec!["MP3".to_string(), "LINEAR16".to_string()],

            quality: "neural".to_string(), // Google TTS provides high quality voices
            description: Some(format!("Google TTS voice: {}", voice.name)),
        }
    }
}

pub fn estimate_audio_duration(audio_data: &[u8], content_type: &str) -> f32 {
    // Estimate duration based on audio format and file size
    match content_type {
        "audio/mpeg" | "audio/mp3" => {
            // MP3: approximately 16KB per second for typical settings
            audio_data.len() as f32 / 16000.0
        }
        "audio/wav" | "audio/pcm" => {
            // PCM 16-bit at 22050 Hz = ~44KB per second
            audio_data.len() as f32 / 44100.0
        }
        "audio/ogg" | "audio/ogg;codecs=opus" => {
            // OGG Opus: approximately 8KB per second
            audio_data.len() as f32 / 8000.0
        }
        _ => {
            // Default estimation for unknown formats
            audio_data.len() as f32 / 16000.0
        }
    }
}

pub fn strip_ssml_tags(ssml: &str) -> String {
    let mut result = String::new();
    let mut inside_tag = false;
    let chars = ssml.chars();

    for ch in chars {
        match ch {
            '<' => {
                inside_tag = true;
            }
            '>' => {
                inside_tag = false;
            }
            _ if !inside_tag => {
                result.push(ch);
            }
            _ => {} // Skip characters inside tags
        }
    }

    result
}

pub fn split_into_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current_sentence = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        current_sentence.push(ch);

        // Check for sentence endings
        if matches!(ch, '.' | '!' | '?') {
            // Check if this is likely the end of a sentence
            if let Some(&next_ch) = chars.peek() {
                if next_ch.is_whitespace() || next_ch.is_ascii_uppercase() {
                    // This looks like a sentence boundary
                    sentences.push(current_sentence.trim().to_string());
                    current_sentence.clear();
                }
            } else {
                // End of text, definitely a sentence
                sentences.push(current_sentence.trim().to_string());
                current_sentence.clear();
            }
        }
    }

    // Add any remaining text as a sentence
    if !current_sentence.trim().is_empty() {
        sentences.push(current_sentence.trim().to_string());
    }

    sentences
}

pub fn estimate_duration(text: &str) -> f32 {
    // Estimate based on average speaking rate
    // Google TTS typically speaks at ~150-175 words per minute
    // Average 5 characters per word including spaces and punctuation
    let char_count = text.len() as f32;
    let estimated_words = char_count / 5.0;
    let duration_minutes = estimated_words / 165.0; // Use middle ground of 165 WPM
    duration_minutes * 60.0 // Convert to seconds
}

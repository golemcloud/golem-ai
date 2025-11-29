use golem_tts::golem::tts::{
    advanced::{PronunciationEntry, TtsError},
    types::VoiceGender,
    voices::Voice,
};

use crate::resources::VoiceResponse;

impl From<VoiceResponse> for Voice {
    fn from(voice: VoiceResponse) -> Self {
        let voice_gender = voice.gender.to_lowercase();

        let gender = if voice_gender == "male" {
            VoiceGender::Male
        } else if voice_gender == "female" {
            VoiceGender::Female
        } else {
            VoiceGender::Neutral
        };

        let quality = voice
            .supported_engines
            .as_ref()
            .unwrap_or(&vec![])
            .join(",");

        Voice {
            id: voice.id,
            name: voice.name,
            language: voice.language_code,
            additional_languages: voice.additional_language_codes.unwrap_or_default(),
            gender,
            quality,
            description: None,
            provider: "polly".to_string(),
            sample_rate: vec![],
            supports_ssml: true,
            is_custom: false,
            is_cloned: false,
            preview_url: None,
            use_cases: vec!["general".to_string()],
            supported_formats: vec![
                "mp3".to_string(),
                "ogg_vorbis".to_string(),
                "pcm".to_string(),
            ],
        }
    }
}

pub fn create_pls_content(language_code: &str, entries: &[PronunciationEntry]) -> String {
    let mut pls = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<lexicon version="1.0" 
         xmlns="http://www.w3.org/2005/01/pronunciation-lexicon"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" 
         xsi:schemaLocation="http://www.w3.org/2005/01/pronunciation-lexicon 
                             http://www.w3.org/TR/2007/CR-pronunciation-lexicon-20071212/pls.xsd"
         alphabet="ipa" xml:lang="{}">"#,
        language_code
    );

    for entry in entries {
        pls.push_str(&format!(
            r#"
    <lexeme>
        <grapheme>{}</grapheme>
        <phoneme>{}</phoneme>
    </lexeme>"#,
            entry.word, entry.pronunciation
        ));
    }

    pls.push_str("\n</lexicon>");
    pls
}

pub fn parse_s3_location(location: &str) -> Result<(String, String), TtsError> {
    if !location.starts_with("s3://") {
        return Err(TtsError::InvalidConfiguration(
            "AWS Polly requires S3 location for long-form synthesis (s3://bucket/key)".to_string(),
        ));
    }

    let without_prefix = &location[5..];
    let parts: Vec<&str> = without_prefix.splitn(2, '/').collect();

    if parts.len() != 2 {
        return Err(TtsError::InvalidConfiguration(
            "Invalid S3 location format. Expected: s3://bucket/key".to_string(),
        ));
    }

    Ok((parts[0].to_string(), parts[1].to_string()))
}

pub fn estimate_audio_duration(audio_data: &[u8], content_type: &str) -> Option<f32> {
    match content_type {
        "audio/mpeg" | "audio/mp3" => Some(audio_data.len() as f32 / 16000.0),
        "audio/wav" | "audio/pcm" => Some(audio_data.len() as f32 / 44100.0),
        "audio/ogg" | "audio/ogg;codecs=opus" => Some(audio_data.len() as f32 / 48000.0),
        _ => None,
    }
}

pub fn estimate_text_duration(text: &str) -> f32 {
    let word_count = text.split_whitespace().count() as f32;
    word_count / 175.0 * 60.0
}

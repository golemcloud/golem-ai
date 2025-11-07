use golem_tts::golem::tts::{types::VoiceGender, voices::Voice};

use crate::resources::VoiceResponse;

impl From<&VoiceResponse> for Voice {
    fn from(v: &VoiceResponse) -> Self {
        let tags = v.metadata.tags.clone();
        let mut gender = VoiceGender::Neutral;
        for t in tags {
            if t.contains("feminine") {
                gender = VoiceGender::Female;
            } else if t.contains("masculine") {
                gender = VoiceGender::Male;
            }
        }

        let use_case = v.metadata.use_cases.clone().join(",");
        let name = v.name.clone();

        let mut quality = "standard".to_string();
        for q in ["standard", "premium", "neural", "studio"] {
            if v.metadata.tags.contains(&q.to_string()) {
                quality = q.to_string();
                break;
            }
        }

        Voice {
            id: v.uuid.clone(),             // Keep UUID for get_voice API
            name: v.canonical_name.clone(), // Use canonical_name for synthesis
            language: v.languages[0].clone(),
            additional_languages: v.languages.clone(),
            gender,
            quality: quality.to_string(),
            description: Some(format!("I am {}. I can help you with {}", name, use_case)),
            sample_rate: vec![8000, 16000, 24000, 32000, 48000, 22050],
            supported_formats: vec![
                "wav".to_string(),
                "mp3".to_string(),
                "linear16".to_string(),
                "mulaw".to_string(),
                "alaw".to_string(),
                "opus".to_string(),
                "flac".to_string(),
                "aac".to_string(),
            ],
            is_custom: false,
            is_cloned: false,
            preview_url: Some(v.metadata.sample.clone()),
            use_cases: v.metadata.use_cases.clone(),
            provider: "deepgram".to_string(),
            supports_ssml: false,
        }
    }
}

pub fn estimate_duration(text: &str) -> f32 {
    // Rough estimate: ~150 words per minute, ~5 characters per word
    let char_count = text.len() as f32;
    let estimated_words = char_count / 5.0;
    let duration_minutes = estimated_words / 150.0;
    duration_minutes * 60.0 // Convert to seconds
}

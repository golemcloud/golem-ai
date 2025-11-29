use std::io::Error;

use golem_tts::golem::tts::{types::VoiceGender, voices::Voice};

use crate::resources::VoiceResponse;

impl From<VoiceResponse> for Voice {
    fn from(voice: VoiceResponse) -> Self {
        let languages: Vec<String> = voice
            .verified_languages
            .as_ref()
            .map(|l| l.iter().map(|l| l.language.clone()).collect())
            .unwrap_or_default();

        let gender = voice
            .labels
            .as_ref()
            .and_then(|l| {
                l.gender.as_ref().map(|g| {
                    if g.contains("male") {
                        VoiceGender::Male
                    } else if g.contains("female") {
                        VoiceGender::Female
                    } else {
                        VoiceGender::Neutral
                    }
                })
            })
            .unwrap();

        Voice {
            id: voice.voice_id,
            name: voice.name,
            language: languages[0].clone(),
            additional_languages: languages,
            gender,
            quality: "standard".to_string(),
            description: voice.description,
            provider: "elevenlabs".to_string(),
            sample_rate: vec![],
            supports_ssml: true,
            is_custom: voice.is_owner.unwrap_or(false),
            is_cloned: voice.category.unwrap_or_default().contains("cloned"),
            preview_url: voice.preview_url,
            use_cases: vec![],
            supported_formats: vec![
                "mp3_22050_32".to_string(),
                "mp3_24000_48".to_string(),
                "mp3_44100_32".to_string(),
                "mp3_44100_64".to_string(),
                "mp3_44100_96".to_string(),
                "mp3_44100_128".to_string(),
                "mp3_44100_192".to_string(),
                "pcm_8000".to_string(),
                "pcm_16000".to_string(),
                "pcm_22050".to_string(),
                "pcm_24000".to_string(),
                "pcm_32000".to_string(),
                "pcm_44100".to_string(),
                "pcm_48000".to_string(),
                "ulaw_8000".to_string(),
                "alaw_8000".to_string(),
                "opus_48000_32".to_string(),
                "opus_48000_64".to_string(),
                "opus_48000_96".to_string(),
                "opus_48000_128".to_string(),
                "opus_48000_192".to_string(),
            ],
        }
    }
}

pub fn estimate_text_duration(text: &str) -> f32 {
    let word_count = text.split_whitespace().count() as f32;
    // ElevenLabs typically speaks at around 150-180 words per minute
    word_count / 165.0 * 60.0
}

/// Add a text form field to multipart body
pub fn add_form_field(
    body: &mut Vec<u8>,
    boundary: &str,
    name: &str,
    value: &str,
) -> Result<(), Error> {
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"{}\"\r\n", name).as_bytes(),
    );
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(value.as_bytes());
    body.extend_from_slice(b"\r\n");
    Ok(())
}

/// Add a file field to multipart body
pub fn add_file_field(
    body: &mut Vec<u8>,
    boundary: &str,
    name: &str,
    filename: &str,
    content_type: &str,
    data: &[u8],
) -> Result<(), Error> {
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
            name, filename
        )
        .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {}\r\n", content_type).as_bytes());
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(data);
    body.extend_from_slice(b"\r\n");
    Ok(())
}

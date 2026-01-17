//! Voice management for Deepgram Aura TTS
//!
//! Uses a static list of available Aura voices.

use crate::bindings::exports::golem::tts::types::{
    AudioFormat, TtsError, VoiceGender, VoiceQuality, VoiceSettings,
};
use crate::bindings::exports::golem::tts::voices::{
    GuestVoice, GuestVoiceResults, LanguageInfo, Voice as WitVoice, VoiceFilter, VoiceInfo,
    VoiceResults,
};
use crate::types;

pub struct VoiceImpl {
    pub(crate) id: String,
    pub(crate) language_code: String,
    pub(crate) gender: VoiceGender,
}

impl GuestVoice for VoiceImpl {
    fn get_id(&self) -> String {
        self.id.clone()
    }
    fn get_name(&self) -> String {
        self.id.clone()
    }
    fn get_provider_id(&self) -> Option<String> {
        Some("deepgram".to_string())
    }
    fn get_language(&self) -> String {
        self.language_code.clone()
    }
    fn get_additional_languages(&self) -> Vec<String> {
        Vec::new()
    }
    fn get_gender(&self) -> VoiceGender {
        self.gender
    }
    fn get_quality(&self) -> VoiceQuality {
        VoiceQuality::Neural
    }
    fn get_description(&self) -> Option<String> {
        None
    }
    fn supports_ssml(&self) -> bool {
        false
    } // Aura is text-only
    fn get_sample_rates(&self) -> Vec<u32> {
        vec![8000, 16000, 24000, 32000, 44100, 48000]
    }
    fn get_supported_formats(&self) -> Vec<AudioFormat> {
        vec![
            AudioFormat::Mp3,
            AudioFormat::Wav,
            AudioFormat::OggOpus,
            AudioFormat::Aac,
            AudioFormat::Flac,
        ]
    }
    fn update_settings(&self, _: VoiceSettings) -> Result<(), TtsError> {
        Ok(())
    }
    fn delete(&self) -> Result<(), TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Static voices cannot be deleted".to_string(),
        ))
    }
    fn clone_voice(&self) -> Result<WitVoice, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Voice cloning not supported by Deepgram Aura".to_string(),
        ))
    }
    fn preview(&self, _: String) -> Result<Vec<u8>, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Preview not implemented".to_string(),
        ))
    }
}

pub struct VoiceResultsImpl {
    pub(crate) voices: Vec<VoiceInfo>,
    pub(crate) current_index: usize,
}

impl GuestVoiceResults for VoiceResultsImpl {
    fn has_more(&self) -> bool {
        self.current_index < self.voices.len()
    }
    fn get_next(&self) -> Result<Vec<VoiceInfo>, TtsError> {
        Ok(self.voices[self.current_index..].to_vec())
    }
    fn get_total_count(&self) -> Option<u32> {
        Some(self.voices.len() as u32)
    }
}

pub fn list_voices(filter: Option<VoiceFilter>) -> Result<VoiceResults, TtsError> {
    let mut voices = Vec::new();
    for (id, lang, gender_str) in types::DEEPGRAM_VOICES {
        let gender = types::reverse_map_gender(gender_str);

        let info = VoiceInfo {
            id: id.to_string(),
            name: id.to_string(),
            language: lang.to_string(),
            additional_languages: Vec::new(),
            gender,
            quality: VoiceQuality::Neural,
            description: None,
            provider: "deepgram".to_string(),
            sample_rate: 24000,
            is_custom: false,
            is_cloned: false,
            preview_url: None,
            use_cases: Vec::new(),
        };

        if let Some(ref f) = filter {
            if let Some(ref l) = f.language {
                if *lang != l {
                    continue;
                }
            }
            if let Some(ref g) = f.gender {
                if gender != *g {
                    continue;
                }
            }
        }
        voices.push(info);
    }

    Ok(VoiceResults::new(VoiceResultsImpl {
        voices,
        current_index: 0,
    }))
}

pub fn get_voice(voice_id: String) -> Result<WitVoice, TtsError> {
    for (id, lang, gender_str) in types::DEEPGRAM_VOICES {
        if *id == voice_id {
            return Ok(WitVoice::new(VoiceImpl {
                id: id.to_string(),
                language_code: lang.to_string(),
                gender: types::reverse_map_gender(gender_str),
            }));
        }
    }
    Err(TtsError::VoiceNotFound(voice_id))
}

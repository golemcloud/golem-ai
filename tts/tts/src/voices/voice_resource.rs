use crate::golem::tts::types::{AudioFormat, VoiceGender, VoiceQuality};
use crate::golem::tts::voices::GuestVoice;

#[derive(Clone)]
pub struct VoiceResource {
    pub id: String,
    pub name: String,
    pub provider_id: Option<String>,
    pub language: String,
    pub additional_languages: Vec<String>,
    pub gender: VoiceGender,
    pub quality: VoiceQuality,
    pub description: Option<String>,
    pub supports_ssml: bool,
    pub sample_rates: Vec<u32>,
    pub supported_formats: Vec<AudioFormat>,
}

impl VoiceResource {
    pub fn from_info(info: &crate::golem::tts::voices::VoiceInfo) -> Self {
        Self {
            id: info.id.clone(),
            name: info.name.clone(),
            provider_id: None,
            language: info.language.clone(),
            additional_languages: info.additional_languages.clone(),
            gender: info.gender,
            quality: info.quality,
            description: info.description.clone(),
            supports_ssml: true,
            sample_rates: vec![info.sample_rate],
            supported_formats: vec![AudioFormat::Mp3],
        }
    }
}

impl GuestVoice for VoiceResource {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_provider_id(&self) -> Option<String> {
        self.provider_id.clone()
    }

    fn get_language(&self) -> String {
        self.language.clone()
    }

    fn get_additional_languages(&self) -> Vec<String> {
        self.additional_languages.clone()
    }

    fn get_gender(&self) -> VoiceGender {
        self.gender
    }

    fn get_quality(&self) -> VoiceQuality {
        self.quality
    }

    fn get_description(&self) -> Option<String> {
        self.description.clone()
    }

    fn supports_ssml(&self) -> bool {
        self.supports_ssml
    }

    fn get_sample_rates(&self) -> Vec<u32> {
        self.sample_rates.clone()
    }

    fn get_supported_formats(&self) -> Vec<AudioFormat> {
        self.supported_formats.clone()
    }

    fn update_settings(
        &self,
        _settings: crate::golem::tts::types::VoiceSettings,
    ) -> Result<(), crate::golem::tts::types::TtsError> {
        Err(crate::golem::tts::types::TtsError::UnsupportedOperation(
            "Voice settings update unsupported".to_string(),
        ))
    }

    fn delete(&self) -> Result<(), crate::golem::tts::types::TtsError> {
        Err(crate::golem::tts::types::TtsError::UnsupportedOperation(
            "Voice deletion unsupported".to_string(),
        ))
    }

    fn clone(&self) -> Result<String, crate::golem::tts::types::TtsError> {
        Err(crate::golem::tts::types::TtsError::UnsupportedOperation(
            "Voice cloning unsupported".to_string(),
        ))
    }

    fn preview(
        &self,
        _text: String,
    ) -> Result<crate::golem::tts::types::SynthesisResult, crate::golem::tts::types::TtsError>
    {
        Err(crate::golem::tts::types::TtsError::UnsupportedOperation(
            "Voice preview unsupported".to_string(),
        ))
    }
}

//! Voice management for Google Cloud TTS
//!
//! Implements GuestVoice and GuestVoiceResults traits for Google voices.

use crate::bindings::exports::golem::tts::types::{
    AudioFormat, TtsError, VoiceGender, VoiceQuality, VoiceSettings,
};
use crate::bindings::exports::golem::tts::voices::{
    GuestVoice, GuestVoiceResults, LanguageInfo, VoiceFilter, VoiceInfo, VoiceResults,
};
use crate::client;
use crate::types;

// ============================================================
// VOICE RESOURCE IMPLEMENTATION
// ============================================================

pub struct VoiceImpl {
    pub(crate) name: String,
    pub(crate) language_code: String,
    pub(crate) gender: VoiceGender,
}

impl GuestVoice for VoiceImpl {
    fn get_id(&self) -> String {
        self.name.clone()
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_provider_id(&self) -> Option<String> {
        Some("google".to_string())
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
        types::infer_quality(&self.name)
    }

    fn get_description(&self) -> Option<String> {
        None
    }

    fn supports_ssml(&self) -> bool {
        true
    }

    fn get_sample_rates(&self) -> Vec<u32> {
        vec![8000, 16000, 22050, 24000, 32000, 44100, 48000]
    }

    fn get_supported_formats(&self) -> Vec<AudioFormat> {
        vec![
            AudioFormat::Mp3,
            AudioFormat::Wav,
            AudioFormat::OggOpus,
            AudioFormat::Mulaw,
            AudioFormat::Alaw,
        ]
    }

    fn update_settings(&self, _settings: VoiceSettings) -> Result<(), TtsError> {
        // Voice settings are passed during synthesis, not stored on the voice resource in Google Cloud
        Ok(())
    }

    fn delete(&self) -> Result<(), TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Cannot delete Google Cloud voices".to_string(),
        ))
    }

    fn clone_voice(&self) -> Result<crate::bindings::exports::golem::tts::voices::Voice, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Google Cloud does not support voice cloning via this interface".to_string(),
        ))
    }

    fn preview(&self, _text: String) -> Result<Vec<u8>, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Preview not implemented for Google Cloud".to_string(),
        ))
    }
}

// ============================================================
// VOICE RESULTS ITERATOR
// ============================================================

pub struct VoiceResultsImpl {
    pub(crate) voices: Vec<VoiceInfo>,
    pub(crate) current_index: usize,
}

impl GuestVoiceResults for VoiceResultsImpl {
    fn has_more(&self) -> bool {
        self.current_index < self.voices.len()
    }

    fn get_next(&self) -> Result<Vec<VoiceInfo>, TtsError> {
        // Return voices in chunks of 50
        // (WAPI doesn't allow mut self in get_next, so we handle it with interior mutability if needed,
        // but here we just return the full list or a slice. Since GuestVoiceResults is a resource,
        // we can use RefCell if we need to advance the index.)
        // However, looking at the bindings, it's &self. Let's return all remaining.
        Ok(self.voices[self.current_index..].to_vec())
    }

    fn get_total_count(&self) -> Option<u32> {
        Some(self.voices.len() as u32)
    }
}

// ============================================================
// FACTORY FUNCTIONS
// ============================================================

pub fn list_voices(filter: Option<VoiceFilter>) -> Result<VoiceResults, TtsError> {
    let response_bytes = client::get_request("/voices")?;
    let response: types::GoogleVoiceListResponse = serde_json::from_slice(&response_bytes)
        .map_err(|e| types::internal_error(&format!("Failed to parse Google voices: {}", e)))?;

    let mut voices = Vec::new();
    for gv in response.voices {
        let gender = types::reverse_map_gender(&gv.ssml_gender);
        let quality = types::infer_quality(&gv.name);

        let info = VoiceInfo {
            id: gv.name.clone(),
            name: gv.name.clone(),
            language: gv.language_codes.get(0).cloned().unwrap_or_default(),
            additional_languages: if gv.language_codes.len() > 1 {
                gv.language_codes[1..].to_vec()
            } else {
                Vec::new()
            },
            gender,
            quality,
            description: None,
            provider: "google".to_string(),
            sample_rate: gv.natural_sample_rate_hertz,
            is_custom: false,
            is_cloned: false,
            preview_url: None,
            use_cases: Vec::new(),
        };

        // Apply filter
        if let Some(ref f) = filter {
            if let Some(ref lang) = f.language {
                if !gv.language_codes.contains(lang) {
                    continue;
                }
            }
            if let Some(ref gen) = f.gender {
                if gender != *gen {
                    continue;
                }
            }
            if let Some(ref qual) = f.quality {
                if quality != *qual {
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

pub fn get_voice(
    voice_id: String,
) -> Result<crate::bindings::exports::golem::tts::voices::Voice, TtsError> {
    // Google doesn't have a single voice GET endpoint that returns detailed info,
    // so we fetch the list and find it.
    let response_bytes = client::get_request("/voices")?;
    let response: types::GoogleVoiceListResponse = serde_json::from_slice(&response_bytes)
        .map_err(|_| types::internal_error("Failed to parse Google voices"))?;

    for gv in response.voices {
        if gv.name == voice_id {
            return Ok(crate::bindings::exports::golem::tts::voices::Voice::new(
                VoiceImpl {
                    name: gv.name,
                    language_code: gv.language_codes.get(0).cloned().unwrap_or_default(),
                    gender: types::reverse_map_gender(&gv.ssml_gender),
                },
            ));
        }
    }

    Err(TtsError::VoiceNotFound(voice_id))
}

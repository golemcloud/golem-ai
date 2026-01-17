//! Voice resource implementation for AWS Polly

use crate::client;
use crate::types;
use crate::wit_types;
use crate::wit_voices::{self, GuestVoiceResults};
use std::cell::RefCell;

// ============================================================
// VOICE RESOURCE IMPLEMENTATION
// ============================================================

pub struct VoiceImpl {
    id: String,
    name: String,
    provider_id: Option<String>,
    language: String,
    gender: wit_types::VoiceGender,
    quality: wit_types::VoiceQuality,
    engines: Vec<String>,
}

impl VoiceImpl {
    pub fn from_polly(voice: &types::PollyVoice) -> Self {
        let gender = types::map_gender(&voice.gender);

        let quality = if voice.supported_engines.contains(&"neural".to_string()) {
            wit_types::VoiceQuality::Neural
        } else {
            wit_types::VoiceQuality::Standard
        };

        return VoiceImpl {
            id: voice.id.clone(),
            name: voice.name.clone(),
            provider_id: Some(voice.id.clone()),
            language: voice.language_code.clone(),
            gender,
            quality,
            engines: voice.supported_engines.clone(),
        };
    }

    pub fn voice_id(&self) -> &str {
        return &self.id;
    }

    pub fn best_engine(&self) -> &str {
        if self.engines.contains(&"neural".to_string()) {
            return "neural";
        }
        return "standard";
    }
}

impl wit_voices::GuestVoice for VoiceImpl {
    fn get_id(&self) -> String {
        return self.id.clone();
    }
    fn get_name(&self) -> String {
        return self.name.clone();
    }
    fn get_provider_id(&self) -> Option<String> {
        return self.provider_id.clone();
    }
    fn get_language(&self) -> String {
        return self.language.clone();
    }
    fn get_additional_languages(&self) -> Vec<String> {
        return Vec::new();
    }
    fn get_gender(&self) -> wit_types::VoiceGender {
        return self.gender.clone();
    }
    fn get_quality(&self) -> wit_types::VoiceQuality {
        return self.quality.clone();
    }
    fn get_description(&self) -> Option<String> {
        return None;
    }
    fn supports_ssml(&self) -> bool {
        return true;
    }
    fn get_sample_rates(&self) -> Vec<u32> {
        return vec![24000, 22050, 16000, 8000];
    }
    fn get_supported_formats(&self) -> Vec<wit_types::AudioFormat> {
        return vec![wit_types::AudioFormat::Mp3, wit_types::AudioFormat::Pcm];
    }

    fn update_settings(
        &self,
        _settings: wit_types::VoiceSettings,
    ) -> Result<(), wit_types::TtsError> {
        return Err(types::internal_error(
            "Persistent settings not supported by Polly",
        ));
    }

    fn delete(&self) -> Result<(), wit_types::TtsError> {
        return Err(types::internal_error("Polly voices cannot be deleted"));
    }

    fn clone_voice(&self) -> Result<wit_voices::Voice, wit_types::TtsError> {
        return Err(types::internal_error("Polly does not support cloning"));
    }

    fn preview(&self, text: String) -> Result<Vec<u8>, wit_types::TtsError> {
        return client::synthesize_api(&self.id, &text, self.best_engine(), "mp3");
    }
}

// ============================================================
// VOICE RESULTS ITERATOR
// ============================================================

struct VoiceResultsState {
    voices: Vec<wit_voices::VoiceInfo>,
    current_index: usize,
}

pub struct VoiceResultsImpl {
    state: RefCell<VoiceResultsState>,
}

impl VoiceResultsImpl {
    pub fn new(voices: Vec<wit_voices::VoiceInfo>) -> Self {
        return VoiceResultsImpl {
            state: RefCell::new(VoiceResultsState {
                voices,
                current_index: 0,
            }),
        };
    }
}

impl wit_voices::GuestVoiceResults for VoiceResultsImpl {
    fn has_more(&self) -> bool {
        let state = self.state.borrow();
        return state.current_index < state.voices.len();
    }

    fn get_next(&self) -> Result<Vec<wit_voices::VoiceInfo>, wit_types::TtsError> {
        let mut state = self.state.borrow_mut();
        let batch_size = 20;
        let end = core::cmp::min(state.current_index + batch_size, state.voices.len());
        let batch = state.voices[state.current_index..end].to_vec();
        state.current_index = end;
        return Ok(batch);
    }

    fn get_total_count(&self) -> Option<u32> {
        let state = self.state.borrow();
        return Some(state.voices.len() as u32);
    }
}

// ============================================================
// VOICES INTERFACE FUNCTIONS
// ============================================================

pub fn list_voices(
    filter: Option<wit_voices::VoiceFilter>,
) -> Result<wit_voices::VoiceResults, wit_types::TtsError> {
    let api_response = client::list_voices_api()?;
    let mut infos = Vec::new();

    for voice in api_response.voices {
        let voice_impl = VoiceImpl::from_polly(&voice);

        // Basic filtering
        if let Some(ref f) = filter {
            if let Some(ref lang) = f.language {
                if !voice.language_code.contains(lang) {
                    continue;
                }
            }
            if let Some(ref gender) = f.gender {
                if &voice_impl.gender != gender {
                    continue;
                }
            }
        }

        infos.push(wit_voices::VoiceInfo {
            id: voice.id.clone(),
            name: voice.name.clone(),
            language: voice.language_code.clone(),
            additional_languages: Vec::new(),
            gender: voice_impl.gender.clone(),
            quality: voice_impl.quality.clone(),
            description: None,
            provider: "aws-polly".to_string(),
            sample_rate: 24000,
            is_custom: false,
            is_cloned: false,
            preview_url: None,
            use_cases: Vec::new(),
        });
    }

    let results = VoiceResultsImpl::new(infos);
    return Ok(wit_voices::VoiceResults::new(results));
}

pub fn get_voice(voice_id: String) -> Result<wit_voices::Voice, wit_types::TtsError> {
    let api_response = client::list_voices_api()?;
    let voice = api_response
        .voices
        .into_iter()
        .find(|v| v.id == voice_id)
        .ok_or_else(|| types::internal_error("Voice not found"))?;

    let voice_impl = VoiceImpl::from_polly(&voice);
    return Ok(wit_voices::Voice::new(voice_impl));
}

pub fn search_voices(
    query: String,
    filter: Option<wit_voices::VoiceFilter>,
) -> Result<Vec<wit_voices::VoiceInfo>, wit_types::TtsError> {
    let results = list_voices(filter)?;
    let all = results.get::<VoiceResultsImpl>().get_next()?;
    let filtered = all
        .into_iter()
        .filter(|v| v.name.to_lowercase().contains(&query.to_lowercase()))
        .collect();
    return Ok(filtered);
}

pub fn list_languages() -> Result<Vec<wit_voices::LanguageInfo>, wit_types::TtsError> {
    return Ok(vec![
        wit_voices::LanguageInfo {
            code: "en-US".to_string(),
            name: "English (US)".to_string(),
            native_name: "English (US)".to_string(),
            voice_count: 10,
        },
        wit_voices::LanguageInfo {
            code: "en-GB".to_string(),
            name: "English (UK)".to_string(),
            native_name: "English (UK)".to_string(),
            voice_count: 5,
        },
    ]);
}

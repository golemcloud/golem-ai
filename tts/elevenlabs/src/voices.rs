//! Voice resource implementation for ElevenLabs
//!
//! Implements the voices interface with durability-wrapped API calls.

use crate::client;
use crate::types;
use crate::wit_types;
use crate::wit_voices;
use std::cell::RefCell;

// ============================================================
// VOICE RESOURCE IMPLEMENTATION
// ============================================================

/// Voice resource holding ElevenLabs voice data
pub struct VoiceImpl {
    id: String,
    name: String,
    provider_id: Option<String>,
    language: String,
    additional_languages: Vec<String>,
    gender: wit_types::VoiceGender,
    quality: wit_types::VoiceQuality,
    description: Option<String>,
    sample_rates: Vec<u32>,
    supported_formats: Vec<wit_types::AudioFormat>,
}

impl VoiceImpl {
    /// Create voice from ElevenLabs API response
    pub fn from_elevenlabs(voice: &types::ElevenLabsVoice) -> Self {
        let gender_str = voice.labels.as_ref().and_then(|l| l.gender.as_deref());
        let gender = types::map_gender(gender_str);

        let category = voice.category.as_deref();
        let quality = types::map_quality(category);

        return VoiceImpl {
            id: voice.voice_id.clone(),
            name: voice.name.clone(),
            provider_id: Some(voice.voice_id.clone()),
            language: "en".to_string(), // ElevenLabs voices are primarily English
            additional_languages: Vec::new(),
            gender,
            quality,
            description: voice.description.clone(),
            sample_rates: vec![44100, 22050, 16000, 8000],
            supported_formats: vec![
                wit_types::AudioFormat::Mp3,
                wit_types::AudioFormat::Pcm,
                wit_types::AudioFormat::Mulaw,
            ],
        };
    }

    /// Get the voice ID for API calls
    pub fn voice_id(&self) -> &str {
        return &self.id;
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
        return self.additional_languages.clone();
    }

    fn get_gender(&self) -> wit_types::VoiceGender {
        return self.gender.clone();
    }

    fn get_quality(&self) -> wit_types::VoiceQuality {
        return self.quality.clone();
    }

    fn get_description(&self) -> Option<String> {
        return self.description.clone();
    }

    fn supports_ssml(&self) -> bool {
        return true; // ElevenLabs supports SSML
    }

    fn get_sample_rates(&self) -> Vec<u32> {
        return self.sample_rates.clone();
    }

    fn get_supported_formats(&self) -> Vec<wit_types::AudioFormat> {
        return self.supported_formats.clone();
    }

    fn update_settings(
        &self,
        _settings: wit_types::VoiceSettings,
    ) -> Result<(), wit_types::TtsError> {
        // ElevenLabs doesn't support updating voice settings persistently
        return Err(types::unsupported_operation_error(
            "Voice settings cannot be updated persistently",
        ));
    }

    fn delete(&self) -> Result<(), wit_types::TtsError> {
        // ElevenLabs only allows deleting cloned voices
        // This would require additional API call to check if voice is cloned
        return Err(types::unsupported_operation_error(
            "Voice deletion not implemented",
        ));
    }

    fn clone_voice(&self) -> Result<wit_voices::Voice, wit_types::TtsError> {
        return Err(types::unsupported_operation_error(
            "Voice cloning requires audio samples - use create_voice_clone",
        ));
    }

    fn preview(&self, text: String) -> Result<Vec<u8>, wit_types::TtsError> {
        // Synthesize a short preview
        let max_preview_len = 100;
        let preview_text = if text.len() > max_preview_len {
            &text[..max_preview_len]
        } else {
            &text
        };

        let audio = client::synthesize_api(
            &self.id,
            preview_text,
            &client::get_model_version(),
            "mp3_44100_128",
        )?;

        return Ok(audio);
    }
}

// ============================================================
// VOICE RESULTS ITERATOR
// ============================================================

/// State for VoiceResultsImpl
struct VoiceResultsState {
    voices: Vec<wit_voices::VoiceInfo>,
    current_index: usize,
    batch_size: usize,
}

/// Voice results iterator with interior mutability
pub struct VoiceResultsImpl {
    state: RefCell<VoiceResultsState>,
}

impl VoiceResultsImpl {
    pub fn new(voices: Vec<wit_voices::VoiceInfo>) -> Self {
        return VoiceResultsImpl {
            state: RefCell::new(VoiceResultsState {
                voices,
                current_index: 0,
                batch_size: 20,
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
        let end_index = core::cmp::min(state.current_index + state.batch_size, state.voices.len());
        let batch = state.voices[state.current_index..end_index].to_vec();
        state.current_index = end_index;
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

/// List available voices with optional filtering
pub fn list_voices(
    filter: Option<wit_voices::VoiceFilter>,
) -> Result<wit_voices::VoiceResults, wit_types::TtsError> {
    let api_response = client::list_voices_api()?;

    let mut voice_infos: Vec<wit_voices::VoiceInfo> = Vec::new();

    for voice in api_response.voices.iter() {
        let voice_impl = VoiceImpl::from_elevenlabs(voice);

        // Apply filter if provided
        if let Some(ref f) = filter {
            // Filter by language
            if let Some(ref lang) = f.language {
                if &voice_impl.language != lang {
                    continue;
                }
            }

            // Filter by gender
            if let Some(ref gender) = f.gender {
                if &voice_impl.gender != gender {
                    continue;
                }
            }

            // Filter by quality
            if let Some(ref quality) = f.quality {
                if &voice_impl.quality != quality {
                    continue;
                }
            }

            // Filter by search query (name contains)
            if let Some(ref query) = f.search_query {
                let name_lower = voice_impl.name.to_lowercase();
                let query_lower = query.to_lowercase();
                if !name_lower.contains(&query_lower) {
                    continue;
                }
            }
        }

        // Determine if voice is cloned
        let is_cloned = voice.category.as_deref() == Some("cloned");
        let is_custom = voice.category.as_deref() == Some("generated");

        let voice_info = wit_voices::VoiceInfo {
            id: voice.voice_id.clone(),
            name: voice.name.clone(),
            language: voice_impl.language.clone(),
            additional_languages: Vec::new(),
            gender: voice_impl.gender.clone(),
            quality: voice_impl.quality.clone(),
            description: voice.description.clone(),
            provider: "elevenlabs".to_string(),
            sample_rate: 44100,
            is_custom,
            is_cloned,
            preview_url: voice.preview_url.clone(),
            use_cases: Vec::new(),
        };

        voice_infos.push(voice_info);
    }

    let results = VoiceResultsImpl::new(voice_infos);
    return Ok(wit_voices::VoiceResults::new(results));
}

/// Get specific voice by ID
pub fn get_voice(voice_id: String) -> Result<wit_voices::Voice, wit_types::TtsError> {
    let api_voice = client::get_voice_api(&voice_id)?;
    let voice_impl = VoiceImpl::from_elevenlabs(&api_voice);
    return Ok(wit_voices::Voice::new(voice_impl));
}

/// Search voices by query and filter
pub fn search_voices(
    query: String,
    filter: Option<wit_voices::VoiceFilter>,
) -> Result<Vec<wit_voices::VoiceInfo>, wit_types::TtsError> {
    // Create filter with search query
    let search_filter = wit_voices::VoiceFilter {
        language: filter.as_ref().and_then(|f| f.language.clone()),
        gender: filter.as_ref().and_then(|f| f.gender.clone()),
        quality: filter.as_ref().and_then(|f| f.quality.clone()),
        supports_ssml: filter.as_ref().and_then(|f| f.supports_ssml),
        provider: filter.as_ref().and_then(|f| f.provider.clone()),
        search_query: Some(query),
    };

    // Get all matching voices
    let api_response = client::list_voices_api()?;
    let mut all_voices: Vec<wit_voices::VoiceInfo> = Vec::new();

    for voice in api_response.voices.iter() {
        let voice_impl = VoiceImpl::from_elevenlabs(voice);

        // Apply search filter
        if let Some(ref q) = search_filter.search_query {
            let name_lower = voice_impl.name.to_lowercase();
            let query_lower = q.to_lowercase();
            if !name_lower.contains(&query_lower) {
                continue;
            }
        }

        if let Some(ref lang) = search_filter.language {
            if &voice_impl.language != lang {
                continue;
            }
        }

        if let Some(ref gender) = search_filter.gender {
            if &voice_impl.gender != gender {
                continue;
            }
        }

        let is_cloned = voice.category.as_deref() == Some("cloned");
        let is_custom = voice.category.as_deref() == Some("generated");

        let voice_info = wit_voices::VoiceInfo {
            id: voice.voice_id.clone(),
            name: voice.name.clone(),
            language: voice_impl.language.clone(),
            additional_languages: Vec::new(),
            gender: voice_impl.gender.clone(),
            quality: voice_impl.quality.clone(),
            description: voice.description.clone(),
            provider: "elevenlabs".to_string(),
            sample_rate: 44100,
            is_custom,
            is_cloned,
            preview_url: voice.preview_url.clone(),
            use_cases: Vec::new(),
        };

        all_voices.push(voice_info);
    }

    return Ok(all_voices);
}

/// List supported languages
pub fn list_languages() -> Result<Vec<wit_voices::LanguageInfo>, wit_types::TtsError> {
    // ElevenLabs primarily supports these languages
    let languages = vec![
        wit_voices::LanguageInfo {
            code: "en".to_string(),
            name: "English".to_string(),
            native_name: "English".to_string(),
            voice_count: 100,
        },
        wit_voices::LanguageInfo {
            code: "es".to_string(),
            name: "Spanish".to_string(),
            native_name: "Español".to_string(),
            voice_count: 20,
        },
        wit_voices::LanguageInfo {
            code: "fr".to_string(),
            name: "French".to_string(),
            native_name: "Français".to_string(),
            voice_count: 20,
        },
        wit_voices::LanguageInfo {
            code: "de".to_string(),
            name: "German".to_string(),
            native_name: "Deutsch".to_string(),
            voice_count: 20,
        },
        wit_voices::LanguageInfo {
            code: "it".to_string(),
            name: "Italian".to_string(),
            native_name: "Italiano".to_string(),
            voice_count: 15,
        },
        wit_voices::LanguageInfo {
            code: "pt".to_string(),
            name: "Portuguese".to_string(),
            native_name: "Português".to_string(),
            voice_count: 15,
        },
        wit_voices::LanguageInfo {
            code: "pl".to_string(),
            name: "Polish".to_string(),
            native_name: "Polski".to_string(),
            voice_count: 10,
        },
        wit_voices::LanguageInfo {
            code: "ja".to_string(),
            name: "Japanese".to_string(),
            native_name: "日本語".to_string(),
            voice_count: 10,
        },
        wit_voices::LanguageInfo {
            code: "ko".to_string(),
            name: "Korean".to_string(),
            native_name: "한국어".to_string(),
            voice_count: 10,
        },
        wit_voices::LanguageInfo {
            code: "zh".to_string(),
            name: "Chinese".to_string(),
            native_name: "中文".to_string(),
            voice_count: 10,
        },
    ];

    return Ok(languages);
}

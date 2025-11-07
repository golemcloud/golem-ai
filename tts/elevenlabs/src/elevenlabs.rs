use std::cell::RefCell;

use golem_tts::{
    client::{ApiClient, TtsClient},
    config::get_env,
    golem::tts::{
        advanced::{
            AgeCategory, AudioSample, LanguageCode, PronunciationEntry, PronunciationLexicon,
            Voice, VoiceDesignParams,
        },
        synthesis::{SynthesisOptions, SynthesisResult, TextInput, TimingInfo, ValidationResult},
        types::{SynthesisMetadata, TextType, TtsError, VoiceGender},
        voices::{LanguageInfo, VoiceFilter},
    },
};
use log::trace;
use reqwest::{header::HeaderMap, Client, Method};
use uuid::Uuid;

use crate::{
    error::{from_http_error, unsupported},
    resources::{ElLongFormSynthesis, ElPronunciationLexicon, VoiceResponse},
    types::{
        AddVoiceResponse, CreateLexiconFromRulesRequest, CreateLexiconResponse, ListVoicesQuery,
        ListVoicesResponse, PronunciationRule, PvcCreateRequest, PvcCreateResponse,
        SoundEffectsRequest, SynthesisRequest, SynthesisVoiceSettings,
    },
    utils::{add_file_field, add_form_field, estimate_text_duration},
};

#[derive(Clone)]
pub struct Elevenlabs {
    pub client: ApiClient,
    api_key: String,
    base_url: String,
}



impl TtsClient for Elevenlabs {
  
    type ClientLongFormOperation = ElLongFormSynthesis;
    type ClientPronunciationLexicon = ElPronunciationLexicon;

    fn new() -> Result<Self, TtsError> {
        let api_key = get_env("ELEVENLABS_API_KEY")?;
        let base_url = get_env("TTS_PROVIDER_ENDPOINT")
            .ok()
            .unwrap_or("https://api.elevenlabs.io".to_string());

        let mut auth_headers = HeaderMap::new();
        auth_headers.insert("xi-api-key", api_key.parse().unwrap());

        trace!("Using base URL: {base_url}");

        let client = ApiClient::new(base_url.clone(), auth_headers)?;

        Ok(Self {
            client,
            api_key,
            base_url,
        })
    }

    fn synthesize(
        &self,
        input: TextInput,
        voice: String,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisResult, TtsError> {
        let voice_settings = SynthesisVoiceSettings {
            stability: options
                .as_ref()
                .and_then(|o| o.voice_settings.as_ref().and_then(|v| v.stability)),
            similarity_boost: options
                .as_ref()
                .and_then(|o| o.voice_settings.as_ref().and_then(|v| v.similarity)),
            style: options
                .as_ref()
                .and_then(|o| o.voice_settings.as_ref().and_then(|v| v.style)),
            use_speaker_boost: options.as_ref().and_then(|o| {
                o.voice_settings
                    .as_ref()
                    .and_then(|v| v.volume.map(|v| v > 0.0))
            }),
            speed: options
                .as_ref()
                .and_then(|o| o.voice_settings.as_ref().and_then(|v| v.speed)),
        };
        let body = SynthesisRequest {
            text: input.content.clone(),
            model_id: options.as_ref().and_then(|o| o.model_id.clone()),
            output_format: options
                .as_ref()
                .and_then(|o| o.audio_config.as_ref().map(|c| c.format.clone())),
            language_code: None, // Don't send language_code - not supported by all models
            voice_settings: Some(voice_settings),
            pronunciation_dictionary_locators: None,
            seed: options.as_ref().and_then(|o| o.seed.map(|s| s as i32)),
            previous_text: None,
            next_text: None,
            apply_text_normalization: None,
            previous_request_ids: None,
            next_request_ids: None,
            apply_language_text_normalization: None,
        };

        let result = self.client.retry_audio_request::<SynthesisRequest, (), _>(
            reqwest::Method::POST,
            &format!("/v1/text-to-speech/{}", voice),
            body,
            None,
            from_http_error,
        )?;
        Ok(SynthesisResult {
            audio_data: result.clone(),
            metadata: SynthesisMetadata {
                duration_seconds: 0.0,
                character_count: input.content.len() as u32,
                word_count: 0,
                audio_size_bytes: result.len() as u32,
                request_id: "".to_string(),
                provider_info: Some("ElevenLabs".to_string()),
            },
        })
    }

    fn synthesize_batch(
        &self,
        inputs: Vec<TextInput>,
        voice: String,
        options: Option<SynthesisOptions>,
    ) -> Result<Vec<SynthesisResult>, TtsError> {
        let mut results = Vec::with_capacity(inputs.len());
        for (index, input) in inputs.iter().enumerate() {
            trace!("Sending input #{index}");
            let voice_settings = SynthesisVoiceSettings {
                stability: options
                    .as_ref()
                    .and_then(|o| o.voice_settings.as_ref().and_then(|v| v.stability)),
                similarity_boost: options
                    .as_ref()
                    .and_then(|o| o.voice_settings.as_ref().and_then(|v| v.similarity)),
                style: options
                    .as_ref()
                    .and_then(|o| o.voice_settings.as_ref().and_then(|v| v.style)),
                use_speaker_boost: options.as_ref().and_then(|o| {
                    o.voice_settings
                        .as_ref()
                        .and_then(|v| v.volume.map(|v| v > 0.0))
                }),
                speed: options
                    .as_ref()
                    .and_then(|o| o.voice_settings.as_ref().and_then(|v| v.speed)),
            };
            let body = SynthesisRequest {
                text: input.content.clone(),
                model_id: options.as_ref().and_then(|o| o.model_id.clone()),
                output_format: options
                    .as_ref()
                    .and_then(|o| o.audio_config.as_ref().map(|c| c.format.clone())),
                language_code: None, // Don't send language_code - not supported by all models
                voice_settings: Some(voice_settings),
                pronunciation_dictionary_locators: None,
                seed: options.as_ref().and_then(|o| o.seed.map(|s| s as i32)),
                previous_text: if index > 0 {
                    inputs.get(index - 1).map(|i| i.content.clone())
                } else {
                    None
                },
                next_text: inputs.get(index + 1).map(|i| i.content.clone()),
                apply_text_normalization: None,
                previous_request_ids: None,
                next_request_ids: None,
                apply_language_text_normalization: None,
            };

            let result = self.client.retry_audio_request::<SynthesisRequest, (), _>(
                reqwest::Method::POST,
                &format!("/v1/text-to-speech/{}", voice),
                body,
                None,
                from_http_error,
            )?;
            results.push(SynthesisResult {
                audio_data: result.clone(),
                metadata: SynthesisMetadata {
                    duration_seconds: 0.0,
                    character_count: input.content.len() as u32,
                    word_count: 0,
                    audio_size_bytes: result.len() as u32,
                    request_id: "".to_string(),
                    provider_info: Some("ElevenLabs".to_string()),
                },
            });
        }
        Ok(results)
    }

    fn get_timing_marks(
        &self,
        _input: TextInput,
        _voice: String,
    ) -> Result<Vec<TimingInfo>, TtsError> {
        unsupported("Timing marks without audio synthesis is not supported by ElevenLabs")
    }

    fn validate_input(
        &self,
        input: TextInput,
        voice: String,
    ) -> Result<ValidationResult, TtsError> {
        let text = input.content;
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        // Check if text is empty
        if text.trim().is_empty() {
            errors.push("Text cannot be empty".to_string());
        }

        // ElevenLabs has a 5000 character limit for most voices
        if text.len() > 5000 {
            errors
                .push("Text exceeds maximum length of 5000 characters for ElevenLabs".to_string());
        }

        // Check voice validity
        if voice.is_empty() {
            errors.push("Voice ID cannot be empty".to_string());
        }

        // SSML validation for ElevenLabs
        if input.text_type == TextType::Ssml {
            if text.trim_start().starts_with('<') {
                if !text.contains("</speak>") || !text.contains("<speak") {
                    errors.push("Invalid SSML format - missing speak tags".to_string());
                }
            } else {
                errors.push(
                    "SSML text type specified but content doesn't start with SSML tags".to_string(),
                );
            }
        }

        // Warn about long text that may impact quality
        if text.len() > 2500 {
            warnings.push("Long text may reduce synthesis quality and speed".to_string());
        }

        // Warn about non-ASCII characters
        if text.chars().any(|c| c as u32 > 127) {
            warnings.push("Non-ASCII characters may affect pronunciation quality".to_string());
        }

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            character_count: text.len() as u32,
            estimated_duration: Some(estimate_text_duration(&text)),
        })
    }

    fn list_voices(
        &self,
        filter: Option<VoiceFilter>,
    ) -> Result<Vec<golem_tts::golem::tts::voices::Voice>, TtsError> {
        trace!("Listing available voices.");
        let body = ListVoicesQuery {
            next_page_token: None,
            page_size: Some(100),
            search: filter.as_ref().and_then(|f| f.search_query.clone()),
            sort: Some("name".to_string()),
            sort_direction: Some("asc".to_string()),
            voice_type: None,
            category: None,
            fine_tuning_state: None,
            collection_id: None,
            include_total_count: Some(true),
            voice_ids: None,
        };

        let response = self
            .client
            .retry_request::<ListVoicesResponse, _, ListVoicesQuery, _>(
                reqwest::Method::GET,
                "/v1/voices",
                "",
                Some(&body),
                from_http_error,
            )?;
        Ok(response
            .voices
            .iter()
            .map(|v| Voice::from(v.clone()))
            .collect())
    }

    fn get_voice(&self, voice_id: String) -> Result<Voice, TtsError> {
        let voice = self.client.retry_request::<VoiceResponse, _, (), _>(
            Method::GET,
            &format!("/v1/voices/{voice_id}"),
            "",
            None,
            from_http_error,
        )?;

        Ok(Voice::from(voice))
    }

    fn list_languages(&self) -> Result<Vec<LanguageInfo>, TtsError> {
        Ok(vec![
            LanguageInfo {
                code: "en-US".to_string(),
                name: "English (US)".to_string(),
                native_name: "English (US)".to_string(),
                voice_count: 100,
            },
            LanguageInfo {
                code: "en-GB".to_string(),
                name: "English (UK)".to_string(),
                native_name: "English (UK)".to_string(),
                voice_count: 80,
            },
            LanguageInfo {
                code: "en-AU".to_string(),
                name: "English (Australia)".to_string(),
                native_name: "English (Australia)".to_string(),
                voice_count: 50,
            },
            LanguageInfo {
                code: "en-CA".to_string(),
                name: "English (Canada)".to_string(),
                native_name: "English (Canada)".to_string(),
                voice_count: 40,
            },
            LanguageInfo {
                code: "ja".to_string(),
                name: "Japanese".to_string(),
                native_name: "日本語".to_string(),
                voice_count: 30,
            },
            LanguageInfo {
                code: "zh-CN".to_string(),
                name: "Chinese (Mandarin)".to_string(),
                native_name: "中文（普通话）".to_string(),
                voice_count: 25,
            },
            LanguageInfo {
                code: "de".to_string(),
                name: "German".to_string(),
                native_name: "Deutsch".to_string(),
                voice_count: 35,
            },
            LanguageInfo {
                code: "hi".to_string(),
                name: "Hindi".to_string(),
                native_name: "हिंदी".to_string(),
                voice_count: 20,
            },
            LanguageInfo {
                code: "fr".to_string(),
                name: "French (France)".to_string(),
                native_name: "Français (France)".to_string(),
                voice_count: 30,
            },
            LanguageInfo {
                code: "fr-CA".to_string(),
                name: "French (Canada)".to_string(),
                native_name: "Français (Canada)".to_string(),
                voice_count: 15,
            },
            LanguageInfo {
                code: "ko".to_string(),
                name: "Korean".to_string(),
                native_name: "한국어".to_string(),
                voice_count: 20,
            },
            LanguageInfo {
                code: "pt-BR".to_string(),
                name: "Portuguese (Brazil)".to_string(),
                native_name: "Português (Brasil)".to_string(),
                voice_count: 25,
            },
            LanguageInfo {
                code: "pt".to_string(),
                name: "Portuguese (Portugal)".to_string(),
                native_name: "Português (Portugal)".to_string(),
                voice_count: 15,
            },
            LanguageInfo {
                code: "it".to_string(),
                name: "Italian".to_string(),
                native_name: "Italiano".to_string(),
                voice_count: 25,
            },
            LanguageInfo {
                code: "es".to_string(),
                name: "Spanish (Spain)".to_string(),
                native_name: "Español (España)".to_string(),
                voice_count: 30,
            },
            LanguageInfo {
                code: "es-MX".to_string(),
                name: "Spanish (Mexico)".to_string(),
                native_name: "Español (México)".to_string(),
                voice_count: 20,
            },
            LanguageInfo {
                code: "id".to_string(),
                name: "Indonesian".to_string(),
                native_name: "Bahasa Indonesia".to_string(),
                voice_count: 10,
            },
            LanguageInfo {
                code: "nl".to_string(),
                name: "Dutch".to_string(),
                native_name: "Nederlands".to_string(),
                voice_count: 15,
            },
            LanguageInfo {
                code: "tr".to_string(),
                name: "Turkish".to_string(),
                native_name: "Türkçe".to_string(),
                voice_count: 15,
            },
            LanguageInfo {
                code: "fil".to_string(),
                name: "Filipino".to_string(),
                native_name: "Filipino".to_string(),
                voice_count: 10,
            },
            LanguageInfo {
                code: "pl".to_string(),
                name: "Polish".to_string(),
                native_name: "Polski".to_string(),
                voice_count: 15,
            },
            LanguageInfo {
                code: "sv".to_string(),
                name: "Swedish".to_string(),
                native_name: "Svenska".to_string(),
                voice_count: 10,
            },
            LanguageInfo {
                code: "bg".to_string(),
                name: "Bulgarian".to_string(),
                native_name: "Български".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "ro".to_string(),
                name: "Romanian".to_string(),
                native_name: "Română".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "ar-SA".to_string(),
                name: "Arabic (Saudi Arabia)".to_string(),
                native_name: "العربية (السعودية)".to_string(),
                voice_count: 12,
            },
            LanguageInfo {
                code: "ar-AE".to_string(),
                name: "Arabic (UAE)".to_string(),
                native_name: "العربية (الإمارات)".to_string(),
                voice_count: 10,
            },
            LanguageInfo {
                code: "cs".to_string(),
                name: "Czech".to_string(),
                native_name: "Čeština".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "el".to_string(),
                name: "Greek".to_string(),
                native_name: "Ελληνικά".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "fi".to_string(),
                name: "Finnish".to_string(),
                native_name: "Suomi".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "hr".to_string(),
                name: "Croatian".to_string(),
                native_name: "Hrvatski".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "ms".to_string(),
                name: "Malay".to_string(),
                native_name: "Bahasa Melayu".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "sk".to_string(),
                name: "Slovak".to_string(),
                native_name: "Slovenčina".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "da".to_string(),
                name: "Danish".to_string(),
                native_name: "Dansk".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "ta".to_string(),
                name: "Tamil".to_string(),
                native_name: "தமிழ்".to_string(),
                voice_count: 10,
            },
            LanguageInfo {
                code: "uk".to_string(),
                name: "Ukrainian".to_string(),
                native_name: "Українська".to_string(),
                voice_count: 10,
            },
            LanguageInfo {
                code: "ru".to_string(),
                name: "Russian".to_string(),
                native_name: "Русский".to_string(),
                voice_count: 15,
            },
        ])
    }

    fn create_voice_clone(
        &self,
        name: String,
        audio_samples: Vec<AudioSample>,
        description: Option<String>,
    ) -> Result<Voice, TtsError> {
        if audio_samples.is_empty() {
            return Err(TtsError::InvalidText(
                "At least one audio sample is required for voice cloning".to_string(),
            ));
        }

        let boundary = format!(
            "----boundary{}",
            Uuid::new_v4().to_string().replace("-", "")
        );
        let mut body = Vec::new();

        let _ = add_form_field(&mut body, &boundary, "name", &name);

        if let Some(desc) = description {
            let _ = add_form_field(&mut body, &boundary, "description", &desc);
        }

        for (i, sample) in audio_samples.iter().enumerate() {
            let filename = format!("sample_{}.wav", i);
            let _ = add_file_field(
                &mut body,
                &boundary,
                "files[]",
                &filename,
                "audio/wav",
                &sample.data,
            );
        }

        body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        let url = format!("{}/v1/voices/add", self.base_url);

        let request = Client::new()
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(body);

        let cloned_voice_result = match request.send() {
            Ok(response) => {
                if response.status().is_success() {
                    response.json::<AddVoiceResponse>().map_err(|e| {
                        TtsError::InternalError(format!("Failed to parse response: {}", e))
                    })
                } else {
                    Err(from_http_error(response))
                }
            }
            Err(err) => Err(TtsError::NetworkError(format!("Request failed: {}", err))),
        }?;
        let cloned_voice = self.get_voice(cloned_voice_result.voice_id)?;
        Ok(cloned_voice)
    }

    fn design_voice(
        &self,
        name: String,
        characteristics: VoiceDesignParams,
    ) -> Result<Voice, TtsError> {
        // Create voice description from characteristics
        let mut description_parts = Vec::new();

        // Add gender
        match characteristics.gender {
            VoiceGender::Male => description_parts.push("male voice".to_string()),
            VoiceGender::Female => description_parts.push("female voice".to_string()),
            VoiceGender::Neutral => description_parts.push("neutral voice".to_string()),
        }

        // Add age category
        let age_desc = match characteristics.age_category {
            AgeCategory::Child => "young child",
            AgeCategory::YoungAdult => "young adult",
            AgeCategory::MiddleAged => "middle-aged",
            AgeCategory::Elderly => "elderly",
        };
        description_parts.push(age_desc.to_string());

        // Add accent if provided
        if !characteristics.accent.is_empty() {
            description_parts.push(format!("with {} accent", characteristics.accent));
        }

        // Add personality traits
        if !characteristics.personality_traits.is_empty() {
            let traits = characteristics.personality_traits.join(", ");
            description_parts.push(format!("personality: {}", traits));
        }

        let description = description_parts.join(", ");

        // Create labels from characteristics
        let mut labels = std::collections::HashMap::new();
        labels.insert(
            "gender".to_string(),
            format!("{:?}", characteristics.gender),
        );
        labels.insert(
            "age".to_string(),
            format!("{:?}", characteristics.age_category),
        );
        if !characteristics.accent.is_empty() {
            labels.insert("accent".to_string(), characteristics.accent.clone());
        }

        let request = PvcCreateRequest {
            name: name.clone(),
            language: "en".to_string(),
            description: Some(description),
            labels: Some(labels),
        };

        let new_voice_result = self
            .client
            .retry_request::<PvcCreateResponse, PvcCreateRequest, (), _>(
                Method::POST,
                "/v1/voices/pvc",
                request,
                None,
                from_http_error,
            )?;
        let voice = self.get_voice(new_voice_result.voice_id)?;
        Ok(voice)
    }

    fn convert_voice(
        &self,
        input_audio: Vec<u8>,
        target_voice: String,
        preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, TtsError> {
        if input_audio.is_empty() {
            return Err(TtsError::InvalidText(
                "Input audio data cannot be empty".to_string(),
            ));
        }

        let boundary = format!(
            "----boundary{}",
            Uuid::new_v4().to_string().replace("-", "")
        );
        let mut body = Vec::new();

        let _ = add_file_field(
            &mut body,
            &boundary,
            "audio",
            "input_audio.wav",
            "audio/wav",
            &input_audio,
        );

        let _ = add_form_field(&mut body, &boundary, "model_id", "eleven_english_sts_v2");

        let _ = add_form_field(&mut body, &boundary, "output_format", "mp3_44100_128");

        let _ = add_form_field(&mut body, &boundary, "enable_logging", "false");

        let _ = add_form_field(&mut body, &boundary, "remove_background_noise", "false");

        // Add preserve_timing if specified (this might not be a real ElevenLabs parameter)
        if let Some(preserve) = preserve_timing {
            let _ = add_form_field(
                &mut body,
                &boundary,
                "preserve_timing",
                &preserve.to_string(),
            );
        }

        // Add final boundary
        body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        let url = format!("{}/v1/speech-to-speech/{}", self.base_url, target_voice);

        let request = Client::new()
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(body);

        match request.send() {
            Ok(response) => {
                if response.status().is_success() {
                    response
                        .bytes()
                        .map_err(|e| {
                            TtsError::InternalError(format!(
                                "Failed to read binary response: {}",
                                e
                            ))
                        })
                        .map(|bytes| bytes.to_vec())
                } else {
                    Err(from_http_error(response))
                }
            }
            Err(err) => Err(TtsError::NetworkError(format!("Request failed: {}", err))),
        }
    }

    fn generate_sound_effect(
        &self,
        description: String,
        duration_seconds: Option<f32>,
        style_influence: Option<f32>,
    ) -> Result<Vec<u8>, TtsError> {
        let request = SoundEffectsRequest {
            text: description,
            duration_seconds,
            prompt_influence: style_influence,
        };

        self.client
            .retry_audio_request::<SoundEffectsRequest, (), _>(
                Method::POST,
                "/v1/sound-generation",
                request,
                None,
                from_http_error,
            )
    }

    fn create_lexicon(
        &self,
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<Self::ClientPronunciationLexicon, TtsError> {
        let description = Some(format!(
            "Pronunciation dictionary for {} language",
            match language.as_str() {
                "en" => "English",
                "es" => "Spanish",
                "fr" => "French",
                "de" => "German",
                "hi" => "Hindi",
                _ => "multilingual",
            }
        ));

        let rules = match entries {
            Some(entries) => entries
                .into_iter()
                .map(|entry| {
                    // Check if pronunciation looks like IPA (contains special characters)
                    if entry
                        .pronunciation
                        .chars()
                        .any(|c| "əɪɛɔʊʌɑɒæɜɪʏøœɯɤɐɞɘɵɨɵʉɪʊ".contains(c))
                    {
                        PronunciationRule {
                            string_to_replace: entry.word,
                            rule_type: "phoneme".to_string(),
                            alias: None,
                            phoneme: Some(entry.pronunciation),
                            alphabet: Some("ipa".to_string()),
                        }
                    } else {
                        // Treat as alias if no IPA characters detected
                        PronunciationRule {
                            string_to_replace: entry.word,
                            rule_type: "alias".to_string(),
                            alias: Some(entry.pronunciation),
                            phoneme: None,
                            alphabet: None,
                        }
                    }
                })
                .collect(),
            None => vec![],
        };
        let request = CreateLexiconFromRulesRequest {
            rules,
            name: name.clone(),
            description: description
                .or_else(|| Some(format!("Pronunciation dictionary for {}", name))),
            workspace_access: Some("admin".to_string()),
        };

        let response = self
            .client
            .retry_request::<CreateLexiconResponse, CreateLexiconFromRulesRequest, (), _>(
                Method::POST,
                "/v1/pronunciation-dictionaries/add-from-rules",
                request,
                None,
                from_http_error,
            )?;

        Ok(ElPronunciationLexicon {
            id: response.id,
            name: response.name,
            language,
            version_id: RefCell::new(response.version_id),
            rules_count: RefCell::new(response.version_rules_num),
        })
    }

    fn synthesize_long_form(
        &self,
        _content: String,
        _voice: String,
        _output_location: String,
        _chapter_breaks: Option<Vec<u32>>,
    ) -> Result<ElLongFormSynthesis, TtsError> {
        unsupported("Long-form synthesis not yet implemented for ElevenLabs TTS")
    }
}

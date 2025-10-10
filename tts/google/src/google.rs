use base64::{engine::general_purpose, Engine};
use golem_tts::{
    client::{ApiClient, TtsClient},
    config::get_env,
    golem::tts::{
        advanced::{
            AudioSample, LanguageCode, PronunciationEntry, PronunciationLexicon, Voice,
            VoiceDesignParams,
        },
        synthesis::{SynthesisOptions, TextInput, ValidationResult},
        types::{
            SynthesisMetadata, SynthesisResult, TextType, TimingInfo, TimingMarkType, TtsError,
            VoiceGender,
        },
        voices::{LanguageInfo, VoiceFilter},
    },
};
use log::trace;
use reqwest::Method;
use std::sync::{Arc, Mutex};
use wstd::http::HeaderMap;

use crate::{
    error::{from_http_error, unsupported},
    resources::{GoogleLongFormOperation, GooglePronunciationLexicon},
    types::{
        AudioConfigData, ListVoicesResponse, SynthesisInput, SynthesisRequest, SynthesisResponse,
        VoiceSelectionParams,
    },
    utils::{estimate_audio_duration, estimate_duration, split_into_sentences, strip_ssml_tags},
};

#[derive(Clone, Debug)]
pub struct Google {
    pub base_url: String,
    pub token_data: Arc<Mutex<TokenData>>,
}

#[derive(Clone, Debug)]
pub struct TokenData {
    pub access_token: Option<String>,
    pub expires_at: Option<i64>,
}

impl Google {
    pub fn get_client(&self) -> Result<ApiClient, TtsError> {
        let token = self.get_access_token()?;
        let mut auth_headers = HeaderMap::new();
        auth_headers.insert(
            "Authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );
        auth_headers.insert("Content-Type", "application/json".parse().unwrap());
        ApiClient::new(self.base_url.clone(), auth_headers)
    }
}

impl TtsClient for Google {
 
    type ClientLongFormOperation = GoogleLongFormOperation;
    type ClientPronunciationLexicon = GooglePronunciationLexicon;

    fn new() -> Result<Self, TtsError> {
        let base_url = get_env("TTS_PROVIDER_ENDPOINT")
            .unwrap_or_else(|_| "https://texttospeech.googleapis.com".to_string());
        trace!("Using base URL: {base_url}");
        Ok(Self {
            base_url,
            token_data: Arc::new(Mutex::new(TokenData {
                access_token: None,
                expires_at: None,
            })),
        })
    }

    fn synthesize(
        &self,
        input: TextInput,
        voice: String,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisResult, TtsError> {
        let synthesis_input = match input.text_type {
            TextType::Ssml => SynthesisInput::Ssml {
                ssml: input.content.clone(),
            },
            _ => SynthesisInput::Text {
                text: input.content.clone(),
            },
        };

        let client = self.get_client()?;
        let result = client.retry_request::<ListVoicesResponse, (), (), _>(
            Method::GET,
            "/v1/voices",
            (),
            None,
            from_http_error,
        )?;
        let mut google_voice = None;
        for v in result.voices {
            if v.name == voice {
                google_voice = Some(v);
                break;
            }
        }
        if google_voice.is_none() {
            return Err(TtsError::VoiceNotFound(voice.to_string()));
        }

        let voice_params = VoiceSelectionParams {
            language_code: input.language.unwrap_or("en-US".to_string()),
            name: google_voice.as_ref().unwrap().name.clone(),
            ssml_gender: google_voice.as_ref().unwrap().ssml_gender.clone(),
        };

        let audio_config = AudioConfigData {
            audio_encoding: options
                .as_ref()
                .and_then(|c| c.audio_config.as_ref())
                .map(|c| c.format.clone())
                .unwrap_or("MP3".to_string()),
            sample_rate_hertz: options
                .as_ref()
                .and_then(|c| c.audio_config.as_ref())
                .and_then(|c| c.sample_rate),
            speaking_rate: options
                .as_ref()
                .and_then(|c| c.voice_settings.as_ref())
                .and_then(|c| c.speed),
            pitch: options
                .as_ref()
                .and_then(|c| c.voice_settings.as_ref())
                .and_then(|c| c.pitch),
            volume_gain_db: options
                .as_ref()
                .and_then(|c| c.voice_settings.as_ref())
                .and_then(|c| c.volume),
        };

        let body = SynthesisRequest {
            input: synthesis_input,
            voice: voice_params,
            audio_config,
        };
        let client = self.get_client()?;
        let response = client.retry_request::<SynthesisResponse, SynthesisRequest, (), _>(
            Method::POST,
            "/v1/text:synthesize",
            body,
            None,
            from_http_error,
        )?;

        let audio_data = general_purpose::STANDARD
            .decode(&response.audio_content)
            .map_err(|e| TtsError::InternalError(format!("Failed to decode base64 audio: {e}")))?;

        let duration = estimate_audio_duration(&audio_data, "audio/mpeg");
        let character_count = input.content.clone().len() as u32;
        let word_count = input.content.split_whitespace().count() as u32;
        let audio_size = audio_data.len() as u32;

        Ok(SynthesisResult {
            audio_data,
            metadata: SynthesisMetadata {
                duration_seconds: duration,
                character_count,
                word_count,
                audio_size_bytes: audio_size,
                request_id: format!(
                    "golem-tts-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis()
                ),
                provider_info: Some("Google Cloud Text-to-Speech".to_string()),
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
        for input in inputs {
            let result = self.synthesize(input, voice.clone(), options.clone())?;
            results.push(result);
        }

        Ok(results)
    }

    fn get_timing_marks(
        &self,
        input: TextInput,
        _voice: String,
    ) -> Result<Vec<TimingInfo>, TtsError> {
        let text = &input.content;
        let mut timing_marks = Vec::new();

        if text.trim().is_empty() {
            return Ok(timing_marks);
        }

        // Handle SSML vs plain text
        let clean_text = if input.text_type == TextType::Ssml {
            strip_ssml_tags(text)
        } else {
            text.to_string()
        };

        // Constants for timing estimation (Google TTS characteristics)
        const WORDS_PER_MINUTE: f32 = 165.0; // Average speaking rate
        const SECONDS_PER_WORD: f32 = 60.0 / WORDS_PER_MINUTE; // ~0.36 seconds per word
        const SENTENCE_PAUSE: f32 = 0.5; // Extra pause at sentence boundaries
        const PARAGRAPH_PAUSE: f32 = 1.0; // Extra pause at paragraph boundaries

        let mut current_time = 0.0;
        let mut text_offset = 0u32;

        // Split into paragraphs (double newlines or more)
        let paragraphs: Vec<&str> = clean_text.split("\n\n").collect();

        for (para_idx, paragraph) in paragraphs.iter().enumerate() {
            if para_idx > 0 {
                // Add paragraph timing mark
                timing_marks.push(TimingInfo {
                    start_time_seconds: current_time,
                    end_time_seconds: None,
                    text_offset: Some(text_offset),
                    mark_type: Some(TimingMarkType::Paragraph),
                });
                current_time += PARAGRAPH_PAUSE;
            }

            // Split paragraph into sentences
            let sentences = split_into_sentences(paragraph);

            for (sent_idx, sentence) in sentences.iter().enumerate() {
                if sent_idx > 0 {
                    // Add sentence timing mark
                    timing_marks.push(TimingInfo {
                        start_time_seconds: current_time,
                        end_time_seconds: None,
                        text_offset: Some(text_offset),
                        mark_type: Some(TimingMarkType::Sentence),
                    });
                    current_time += SENTENCE_PAUSE;
                }

                // Split sentence into words
                let words: Vec<&str> = sentence.split_whitespace().collect();

                for word in words {
                    // Add word timing mark
                    timing_marks.push(TimingInfo {
                        start_time_seconds: current_time,
                        end_time_seconds: Some(current_time + SECONDS_PER_WORD),
                        text_offset: Some(text_offset),
                        mark_type: Some(TimingMarkType::Word),
                    });

                    current_time += SECONDS_PER_WORD;
                    text_offset += word.len() as u32 + 1; // +1 for space
                }
            }
        }

        Ok(timing_marks)
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

        // Check text length (Google Cloud TTS has a 5000 character limit for plain text)
        if text.len() > 5000 {
            errors.push(
                "Text exceeds maximum length of 5000 characters for Google Cloud TTS".to_string(),
            );
        }

        // For SSML, check SSML-specific limits and structure
        if input.text_type == TextType::Ssml {
            // Check basic SSML structure
            if !text.trim_start().starts_with("<speak>") || !text.trim_end().ends_with("</speak>") {
                errors.push("SSML text must be enclosed in <speak> tags".to_string());
            }

            // Google Cloud TTS SSML has specific length limits
            if text.len() > 5000 {
                errors.push(
                    "SSML text exceeds maximum length of 5000 characters for Google Cloud TTS"
                        .to_string(),
                );
            }

            // Check for unsupported SSML tags or reserved characters without proper escaping
            let reserved_chars = ['"', '&', '\'', '<', '>'];
            for (i, line) in text.lines().enumerate() {
                // Skip checking inside SSML tags for reserved characters
                let mut inside_tag = false;
                let chars = line.chars().peekable();
                for ch in chars {
                    if ch == '<' {
                        inside_tag = true;
                    } else if ch == '>' {
                        inside_tag = false;
                    } else if !inside_tag && reserved_chars.contains(&ch) {
                        // Check if it's a properly escaped character
                        let context = line
                            .chars()
                            .skip(i.saturating_sub(5))
                            .take(10)
                            .collect::<String>();
                        if !context.contains("&amp;")
                            && !context.contains("&quot;")
                            && !context.contains("&apos;")
                            && !context.contains("&lt;")
                            && !context.contains("&gt;")
                        {
                            warnings.push(format!(
                                "Reserved character '{}' should be escaped in SSML",
                                ch
                            ));
                        }
                    }
                }
            }
        }

        // Check voice validity
        if voice.is_empty() {
            errors.push("Voice name cannot be empty".to_string());
        }

        // Warn about very long text that might affect performance
        if text.len() > 3000 {
            warnings.push(
                "Long text may take significant time to synthesize and could impact performance"
                    .to_string(),
            );
        }

        // Check for potential SSML compatibility issues with certain voices
        if input.text_type == TextType::Ssml {
            // Studio voices have limited SSML support
            if voice.contains("Studio") {
                warnings.push("Studio voices have limited SSML support. Some tags like <mark>, <emphasis>, <prosody pitch>, and <lang> may not work".to_string());
            }

            // Chirp 3: HD voices don't support SSML at all
            if voice.contains("Chirp3-HD") || voice.contains("Chirp-HD") {
                errors.push(
                    "Chirp 3: HD voices do not support SSML input. Please use plain text instead"
                        .to_string(),
                );
            }
        }

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            character_count: text.len() as u32,
            estimated_duration: Some(estimate_duration(&text)),
            warnings,
            errors,
        })
    }

    fn list_voices(
        &self,
        filter: Option<VoiceFilter>,
    ) -> Result<Vec<Voice>, TtsError> {
        let client = self.get_client()?;
        let result = client.retry_request::<ListVoicesResponse, (), (), _>(
            Method::GET,
            "/v1/voices",
            (),
            None,
            from_http_error,
        )?;

        let mut voices: Vec<Voice> = result
            .voices
            .into_iter()
            .map(|voice| Voice::from(voice))
            .collect();

        // Apply filters if provided
        if let Some(filter) = filter {
            voices.retain(|voice| {
                // Filter by language
                if let Some(ref lang) = filter.language {
                    if !voice.language.eq_ignore_ascii_case(lang) {
                        return false;
                    }
                }

                // Filter by gender
                if let Some(ref gender) = filter.gender {
                    if voice.gender != *gender {
                        return false;
                    }
                }

                // Filter by quality
                if let Some(ref quality) = filter.quality {
                    if !voice.quality.eq_ignore_ascii_case(quality) {
                        return false;
                    }
                }

                // Filter by SSML support
                if let Some(supports_ssml) = filter.supports_ssml {
                    if voice.supports_ssml != supports_ssml {
                        return false;
                    }
                }

                // Filter by search query (search in name, id, or description)
                if let Some(ref query) = filter.search_query {
                    let query_lower = query.to_lowercase();
                    let matches = voice.name.to_lowercase().contains(&query_lower)
                        || voice.id.to_lowercase().contains(&query_lower)
                        || voice
                            .description
                            .as_ref()
                            .map(|d| d.to_lowercase().contains(&query_lower))
                            .unwrap_or(false);
                    if !matches {
                        return false;
                    }
                }

                true
            });
        }

        Ok(voices)
    }

    fn get_voice(&self, voice_id: String) -> Result<Voice, TtsError> {
        let client = self.get_client()?;
        let result = client.retry_request::<ListVoicesResponse, (), (), _>(
            Method::GET,
            "/v1/voices",
            (),
            None,
            from_http_error,
        )?;
        for voice in result.voices {
            if voice.name == voice_id {
                return Ok(Voice::from(voice));
            }
        }
        Err(TtsError::VoiceNotFound(voice_id.to_string()))
    }

    fn list_languages(&self) -> Result<Vec<LanguageInfo>, TtsError> {
        Ok(vec![
            LanguageInfo {
                code: "af-ZA".to_string(),
                name: "Afrikaans (South Africa)".to_string(),
                native_name: "Afrikaans (Suid-Afrika)".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "ar-XA".to_string(),
                name: "Arabic".to_string(),
                native_name: "العربية".to_string(),
                voice_count: 32,
            },
            LanguageInfo {
                code: "eu-ES".to_string(),
                name: "Basque (Spain)".to_string(),
                native_name: "Euskera (Espainia)".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "bn-IN".to_string(),
                name: "Bengali (India)".to_string(),
                native_name: "বাংলা (ভারত)".to_string(),
                voice_count: 34,
            },
            LanguageInfo {
                code: "bg-BG".to_string(),
                name: "Bulgarian (Bulgaria)".to_string(),
                native_name: "Български (България)".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "ca-ES".to_string(),
                name: "Catalan (Spain)".to_string(),
                native_name: "Català (Espanya)".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "yue-HK".to_string(),
                name: "Chinese (Hong Kong)".to_string(),
                native_name: "中文 (香港)".to_string(),
                voice_count: 4,
            },
            LanguageInfo {
                code: "cs-CZ".to_string(),
                name: "Czech (Czech Republic)".to_string(),
                native_name: "Čeština (Česká republika)".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "da-DK".to_string(),
                name: "Danish (Denmark)".to_string(),
                native_name: "Dansk (Danmark)".to_string(),
                voice_count: 33,
            },
            LanguageInfo {
                code: "nl-BE".to_string(),
                name: "Dutch (Belgium)".to_string(),
                native_name: "Nederlands (België)".to_string(),
                voice_count: 32,
            },
            LanguageInfo {
                code: "nl-NL".to_string(),
                name: "Dutch (Netherlands)".to_string(),
                native_name: "Nederlands (Nederland)".to_string(),
                voice_count: 32,
            },
            LanguageInfo {
                code: "en-AU".to_string(),
                name: "English (Australia)".to_string(),
                native_name: "English (Australia)".to_string(),
                voice_count: 42,
            },
            LanguageInfo {
                code: "en-IN".to_string(),
                name: "English (India)".to_string(),
                native_name: "English (India)".to_string(),
                voice_count: 36,
            },
            LanguageInfo {
                code: "en-GB".to_string(),
                name: "English (UK)".to_string(),
                native_name: "English (UK)".to_string(),
                voice_count: 47,
            },
            LanguageInfo {
                code: "en-US".to_string(),
                name: "English (US)".to_string(),
                native_name: "English (US)".to_string(),
                voice_count: 51,
            },
            LanguageInfo {
                code: "fil-PH".to_string(),
                name: "Filipino (Philippines)".to_string(),
                native_name: "Filipino (Pilipinas)".to_string(),
                voice_count: 6,
            },
            LanguageInfo {
                code: "fi-FI".to_string(),
                name: "Finnish (Finland)".to_string(),
                native_name: "Suomi (Suomi)".to_string(),
                voice_count: 31,
            },
            LanguageInfo {
                code: "fr-CA".to_string(),
                name: "French (Canada)".to_string(),
                native_name: "Français (Canada)".to_string(),
                voice_count: 34,
            },
            LanguageInfo {
                code: "fr-FR".to_string(),
                name: "French (France)".to_string(),
                native_name: "Français (France)".to_string(),
                voice_count: 36,
            },
            LanguageInfo {
                code: "gl-ES".to_string(),
                name: "Galician (Spain)".to_string(),
                native_name: "Galego (España)".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "de-DE".to_string(),
                name: "German (Germany)".to_string(),
                native_name: "Deutsch (Deutschland)".to_string(),
                voice_count: 36,
            },
            LanguageInfo {
                code: "el-GR".to_string(),
                name: "Greek (Greece)".to_string(),
                native_name: "Ελληνικά (Ελλάδα)".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "gu-IN".to_string(),
                name: "Gujarati (India)".to_string(),
                native_name: "ગુજરાતી (ભારત)".to_string(),
                voice_count: 34,
            },
            LanguageInfo {
                code: "he-IL".to_string(),
                name: "Hebrew (Israel)".to_string(),
                native_name: "עברית (ישראל)".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "hi-IN".to_string(),
                name: "Hindi (India)".to_string(),
                native_name: "हिन्दी (भारत)".to_string(),
                voice_count: 36,
            },
            LanguageInfo {
                code: "hu-HU".to_string(),
                name: "Hungarian (Hungary)".to_string(),
                native_name: "Magyar (Magyarország)".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "is-IS".to_string(),
                name: "Icelandic (Iceland)".to_string(),
                native_name: "Íslenska (Ísland)".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "id-ID".to_string(),
                name: "Indonesian (Indonesia)".to_string(),
                native_name: "Bahasa Indonesia (Indonesia)".to_string(),
                voice_count: 34,
            },
            LanguageInfo {
                code: "it-IT".to_string(),
                name: "Italian (Italy)".to_string(),
                native_name: "Italiano (Italia)".to_string(),
                voice_count: 33,
            },
            LanguageInfo {
                code: "ja-JP".to_string(),
                name: "Japanese (Japan)".to_string(),
                native_name: "日本語 (日本)".to_string(),
                voice_count: 34,
            },
            LanguageInfo {
                code: "kn-IN".to_string(),
                name: "Kannada (India)".to_string(),
                native_name: "ಕನ್ನಡ (ಭಾರತ)".to_string(),
                voice_count: 34,
            },
            LanguageInfo {
                code: "ko-KR".to_string(),
                name: "Korean (South Korea)".to_string(),
                native_name: "한국어 (대한민국)".to_string(),
                voice_count: 35,
            },
            LanguageInfo {
                code: "lv-LV".to_string(),
                name: "Latvian (Latvia)".to_string(),
                native_name: "Latviešu (Latvija)".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "lt-LT".to_string(),
                name: "Lithuanian (Lithuania)".to_string(),
                native_name: "Lietuvių (Lietuva)".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "ms-MY".to_string(),
                name: "Malay (Malaysia)".to_string(),
                native_name: "Bahasa Melayu (Malaysia)".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "ml-IN".to_string(),
                name: "Malayalam (India)".to_string(),
                native_name: "മലയാളം (ഇന്ത്യ)".to_string(),
                voice_count: 34,
            },
            LanguageInfo {
                code: "cmn-CN".to_string(),
                name: "Mandarin Chinese".to_string(),
                native_name: "普通话 (中国大陆)".to_string(),
                voice_count: 38,
            },
            LanguageInfo {
                code: "cmn-TW".to_string(),
                name: "Mandarin Chinese (Taiwan)".to_string(),
                native_name: "國語 (台灣)".to_string(),
                voice_count: 6,
            },
            LanguageInfo {
                code: "mr-IN".to_string(),
                name: "Marathi (India)".to_string(),
                native_name: "मराठी (भारत)".to_string(),
                voice_count: 33,
            },
            LanguageInfo {
                code: "nb-NO".to_string(),
                name: "Norwegian (Norway)".to_string(),
                native_name: "Norsk (Norge)".to_string(),
                voice_count: 32,
            },
            LanguageInfo {
                code: "pl-PL".to_string(),
                name: "Polish (Poland)".to_string(),
                native_name: "Polski (Polska)".to_string(),
                voice_count: 32,
            },
            LanguageInfo {
                code: "pt-BR".to_string(),
                name: "Portuguese (Brazil)".to_string(),
                native_name: "Português (Brasil)".to_string(),
                voice_count: 35,
            },
            LanguageInfo {
                code: "pt-PT".to_string(),
                name: "Portuguese (Portugal)".to_string(),
                native_name: "Português (Portugal)".to_string(),
                voice_count: 4,
            },
            LanguageInfo {
                code: "pa-IN".to_string(),
                name: "Punjabi (India)".to_string(),
                native_name: "ਪੰਜਾਬੀ (ਭਾਰਤ)".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "ro-RO".to_string(),
                name: "Romanian (Romania)".to_string(),
                native_name: "Română (România)".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "ru-RU".to_string(),
                name: "Russian (Russia)".to_string(),
                native_name: "Русский (Россия)".to_string(),
                voice_count: 13,
            },
            LanguageInfo {
                code: "sr-RS".to_string(),
                name: "Serbian (Cyrillic)".to_string(),
                native_name: "Српски (Ћирилица)".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "sk-SK".to_string(),
                name: "Slovak (Slovakia)".to_string(),
                native_name: "Slovenčina (Slovensko)".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "es-ES".to_string(),
                name: "Spanish (Spain)".to_string(),
                native_name: "Español (España)".to_string(),
                voice_count: 40,
            },
            LanguageInfo {
                code: "es-US".to_string(),
                name: "Spanish (US)".to_string(),
                native_name: "Español (Estados Unidos)".to_string(),
                voice_count: 35,
            },
            LanguageInfo {
                code: "sv-SE".to_string(),
                name: "Swedish (Sweden)".to_string(),
                native_name: "Svenska (Sverige)".to_string(),
                voice_count: 38,
            },
            LanguageInfo {
                code: "ta-IN".to_string(),
                name: "Tamil (India)".to_string(),
                native_name: "தமிழ் (இந்தியா)".to_string(),
                voice_count: 34,
            },
            LanguageInfo {
                code: "te-IN".to_string(),
                name: "Telugu (India)".to_string(),
                native_name: "తెలుగు (భారత దేశం)".to_string(),
                voice_count: 32,
            },
            LanguageInfo {
                code: "th-TH".to_string(),
                name: "Thai (Thailand)".to_string(),
                native_name: "ไทย (ประเทศไทย)".to_string(),
                voice_count: 32,
            },
            LanguageInfo {
                code: "tr-TR".to_string(),
                name: "Turkish (Turkey)".to_string(),
                native_name: "Türkçe (Türkiye)".to_string(),
                voice_count: 37,
            },
            LanguageInfo {
                code: "uk-UA".to_string(),
                name: "Ukrainian (Ukraine)".to_string(),
                native_name: "Українська (Україна)".to_string(),
                voice_count: 32,
            },
            LanguageInfo {
                code: "ur-IN".to_string(),
                name: "Urdu (India)".to_string(),
                native_name: "اردو (بھارت)".to_string(),
                voice_count: 32,
            },
            LanguageInfo {
                code: "vi-VN".to_string(),
                name: "Vietnamese (Vietnam)".to_string(),
                native_name: "Tiếng Việt (Việt Nam)".to_string(),
                voice_count: 34,
            },
        ])
    }

    fn create_voice_clone(
        &self,
        _name: String,
        _audio_samples: Vec<AudioSample>,
        _description: Option<String>,
    ) -> Result<Voice, TtsError> {
        unsupported("Google TTS does not support voice cloning")
    }

    fn design_voice(
        &self,
        _name: String,
        _characteristics: VoiceDesignParams,
    ) -> Result<Voice, TtsError> {
        unsupported("Google TTS does not support voice design")
    }

    fn convert_voice(
        &self,
        _input_audio: Vec<u8>,
        _target_voice: String,
        _preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, TtsError> {
        unsupported("Google TTS does not support voice conversion")
    }

    fn generate_sound_effect(
        &self,
        _description: String,
        _duration_seconds: Option<f32>,
        _style_influence: Option<f32>,
    ) -> Result<Vec<u8>, TtsError> {
        unsupported("Google TTS does not support sound effect generation")
    }

    fn create_lexicon(
        &self,
        _name: String,
        _language: LanguageCode,
        _entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<Self::ClientPronunciationLexicon, TtsError> {
        unsupported("Google TTS does not support custom pronunciation lexicons")
    }

    fn synthesize_long_form(
        &self,
        _content: String,
        _voice: String,
        _output_location: String,
        _chapter_breaks: Option<Vec<u32>>,
    ) -> Result<Self::ClientLongFormOperation, TtsError> {
        unsupported("Google TTS long-form synthesis is currently in beta (v1beta1) and not yet supported")
    }
}

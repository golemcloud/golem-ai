use crate::client::{
    CreateVoiceRequest, ListVoicesParams, Model, TextToSpeechParams, TextToSpeechRequest,
    Voice as ElevenLabsVoice, VoiceSettings as ElevenLabsVoiceSettings,
};
use golem_tts::golem::tts::advanced::{AgeCategory, AudioSample, VoiceDesignParams};
use golem_tts::golem::tts::synthesis::{SynthesisOptions, ValidationResult};
use golem_tts::golem::tts::types::{
    AudioFormat, SynthesisMetadata, SynthesisResult, TextInput, TextType, TtsError, VoiceGender,
    VoiceQuality, VoiceSettings,
};
use golem_tts::golem::tts::voices::{LanguageInfo, VoiceFilter, VoiceInfo};
use log::info;

pub fn estimate_audio_duration(audio_data: &[u8], _sample_rate: u32) -> f32 {
    if audio_data.is_empty() {
        return 0.0;
    }

    let estimated_bitrate_bps = 96000;
    let bytes_per_second = estimated_bitrate_bps / 8;

    (audio_data.len() as f32) / (bytes_per_second as f32)
}

pub fn voice_filter_to_list_params(filter: Option<VoiceFilter>) -> Option<ListVoicesParams> {
    filter.map(|f| ListVoicesParams {
        next_page_token: None,
        page_size: None,
        search: f.search_query,
        sort: None,
        sort_direction: None,
        voice_type: match f.gender {
            Some(VoiceGender::Male) => Some("male".to_string()),
            Some(VoiceGender::Female) => Some("female".to_string()),
            Some(VoiceGender::Neutral) => Some("neutral".to_string()),
            None => None,
        },
        category: match f.quality {
            Some(VoiceQuality::Standard) => Some("standard".to_string()),
            Some(VoiceQuality::Premium) => Some("premium".to_string()),
            Some(VoiceQuality::Studio) => Some("studio".to_string()),
            Some(VoiceQuality::Neural) => Some("neural".to_string()),
            None => None,
        },
        include_total_count: Some(true),
    })
}

pub fn elevenlabs_voice_to_voice_info(voice: ElevenLabsVoice) -> VoiceInfo {
    let gender = infer_gender_from_name(&voice.name).unwrap_or(VoiceGender::Neutral);
    let quality = infer_quality_from_category(&voice.category).unwrap_or(VoiceQuality::Standard);

    let language = detect_voice_language(&voice);
    let additional_languages = detect_additional_languages(&voice);
    let use_cases = infer_use_cases_from_voice(&voice);
    let is_custom = voice.category.to_lowercase() == "cloned";
    let is_cloned = voice.category.to_lowercase() == "cloned";
    let preview_url = voice.preview_url.clone();
    let description = voice.description.clone();

    VoiceInfo {
        id: voice.voice_id,
        name: voice.name,
        language,
        additional_languages,
        gender,
        quality,
        description,
        provider: "elevenlabs".to_string(),
        sample_rate: 22050,
        is_custom,
        is_cloned,
        preview_url,
        use_cases,
    }
}

fn detect_voice_language(voice: &ElevenLabsVoice) -> String {
    if let Some(labels) = &voice.labels {
        if let Some(language) = extract_language_from_labels(labels) {
            return language;
        }
    }

    if let Some(language) = infer_language_from_name(&voice.name) {
        return language;
    }

    if let Some(description) = &voice.description {
        if let Some(language) = infer_language_from_description(description) {
            return language;
        }
    }

    match voice.category.to_lowercase().as_str() {
        "multilingual" => "en".to_string(),
        _ => "en".to_string(),
    }
}

fn detect_additional_languages(voice: &ElevenLabsVoice) -> Vec<String> {
    let mut languages = Vec::new();

    if voice.category.to_lowercase().contains("multilingual") {
        languages.extend_from_slice(&[
            "es".to_string(),
            "fr".to_string(),
            "de".to_string(),
            "it".to_string(),
            "pt".to_string(),
            "pl".to_string(),
            "zh".to_string(),
            "ja".to_string(),
            "hi".to_string(),
        ]);
    }

    if let Some(description) = &voice.description {
        languages.extend(extract_mentioned_languages(description));
    }

    let primary = detect_voice_language(voice);
    languages.retain(|lang| lang != &primary);
    languages.sort();
    languages.dedup();

    languages
}

fn extract_language_from_labels(labels: &serde_json::Value) -> Option<String> {
    if let Some(obj) = labels.as_object() {
        if let Some(lang) = obj.get("language").and_then(|v| v.as_str()) {
            return Some(normalize_language_code(lang));
        }
        if let Some(lang) = obj.get("accent").and_then(|v| v.as_str()) {
            return Some(accent_to_language_code(lang));
        }
        if let Some(lang) = obj.get("origin").and_then(|v| v.as_str()) {
            return Some(origin_to_language_code(lang));
        }
    }
    None
}

fn infer_language_from_name(name: &str) -> Option<String> {
    let name_lower = name.to_lowercase();

    if name_lower.contains("english")
        || name_lower.contains("american")
        || name_lower.contains("british")
    {
        return Some("en".to_string());
    }
    if name_lower.contains("spanish") || name_lower.contains("español") {
        return Some("es".to_string());
    }
    if name_lower.contains("french") || name_lower.contains("français") {
        return Some("fr".to_string());
    }
    if name_lower.contains("german") || name_lower.contains("deutsch") {
        return Some("de".to_string());
    }
    if name_lower.contains("italian") || name_lower.contains("italiano") {
        return Some("it".to_string());
    }
    if name_lower.contains("portuguese") || name_lower.contains("português") {
        return Some("pt".to_string());
    }
    if name_lower.contains("polish") || name_lower.contains("polski") {
        return Some("pl".to_string());
    }
    if name_lower.contains("chinese") || name_lower.contains("mandarin") {
        return Some("zh".to_string());
    }
    if name_lower.contains("japanese") || name_lower.contains("日本") {
        return Some("ja".to_string());
    }
    if name_lower.contains("hindi") || name_lower.contains("हिन्दी") {
        return Some("hi".to_string());
    }

    None
}

fn infer_language_from_description(description: &str) -> Option<String> {
    let desc_lower = description.to_lowercase();

    if desc_lower.contains("english")
        || desc_lower.contains("american accent")
        || desc_lower.contains("british accent")
    {
        return Some("en".to_string());
    }
    if desc_lower.contains("spanish")
        || desc_lower.contains("latino")
        || desc_lower.contains("hispanic")
    {
        return Some("es".to_string());
    }
    if desc_lower.contains("french") || desc_lower.contains("parisian") {
        return Some("fr".to_string());
    }
    if desc_lower.contains("german") || desc_lower.contains("bavarian") {
        return Some("de".to_string());
    }

    None
}

fn extract_mentioned_languages(text: &str) -> Vec<String> {
    let mut languages = Vec::new();
    let text_lower = text.to_lowercase();

    let language_patterns = [
        ("english", "en"),
        ("spanish", "es"),
        ("french", "fr"),
        ("german", "de"),
        ("italian", "it"),
        ("portuguese", "pt"),
        ("polish", "pl"),
        ("chinese", "zh"),
        ("japanese", "ja"),
        ("hindi", "hi"),
        ("arabic", "ar"),
        ("russian", "ru"),
    ];

    for (pattern, code) in language_patterns {
        if text_lower.contains(pattern) {
            languages.push(code.to_string());
        }
    }

    languages
}

fn infer_use_cases_from_voice(voice: &ElevenLabsVoice) -> Vec<String> {
    let mut use_cases = Vec::new();

    match voice.category.to_lowercase().as_str() {
        "narration" => use_cases.push("audiobooks".to_string()),
        "conversational" => {
            use_cases.extend_from_slice(&["chatbots".to_string(), "assistant".to_string()])
        }
        "news" => use_cases.push("news-reading".to_string()),
        "storytelling" => {
            use_cases.extend_from_slice(&["audiobooks".to_string(), "podcasts".to_string()])
        }
        _ => {}
    }

    if let Some(description) = &voice.description {
        let desc_lower = description.to_lowercase();

        if desc_lower.contains("narrator") || desc_lower.contains("storytelling") {
            use_cases.push("audiobooks".to_string());
        }
        if desc_lower.contains("news") || desc_lower.contains("anchor") {
            use_cases.push("news-reading".to_string());
        }
        if desc_lower.contains("assistant") || desc_lower.contains("helpful") {
            use_cases.push("assistant".to_string());
        }
        if desc_lower.contains("podcast") {
            use_cases.push("podcasts".to_string());
        }
        if desc_lower.contains("commercial") || desc_lower.contains("advertising") {
            use_cases.push("commercials".to_string());
        }
    }

    if use_cases.is_empty() {
        use_cases.push("general".to_string());
    }

    use_cases.sort();
    use_cases.dedup();
    use_cases
}

fn normalize_language_code(code: &str) -> String {
    match code.to_lowercase().as_str() {
        "en-us" | "en-gb" | "english" => "en".to_string(),
        "es-es" | "es-mx" | "spanish" => "es".to_string(),
        "fr-fr" | "french" => "fr".to_string(),
        "de-de" | "german" => "de".to_string(),
        "it-it" | "italian" => "it".to_string(),
        "pt-pt" | "pt-br" | "portuguese" => "pt".to_string(),
        _ => code.to_lowercase(),
    }
}

fn accent_to_language_code(accent: &str) -> String {
    match accent.to_lowercase().as_str() {
        "american" | "british" | "australian" => "en".to_string(),
        "mexican" | "argentinian" | "colombian" => "es".to_string(),
        "parisian" | "canadian" => "fr".to_string(),
        "bavarian" | "austrian" => "de".to_string(),
        _ => "en".to_string(),
    }
}

fn origin_to_language_code(origin: &str) -> String {
    match origin.to_lowercase().as_str() {
        "usa" | "uk" | "canada" | "australia" => "en".to_string(),
        "spain" | "mexico" | "argentina" => "es".to_string(),
        "france" => "fr".to_string(),
        "germany" | "austria" => "de".to_string(),
        "italy" => "it".to_string(),
        "portugal" | "brazil" => "pt".to_string(),
        "poland" => "pl".to_string(),
        "china" => "zh".to_string(),
        "japan" => "ja".to_string(),
        "india" => "hi".to_string(),
        _ => "en".to_string(),
    }
}

pub fn synthesis_options_to_tts_request(
    options: Option<SynthesisOptions>,
    model_version: &str,
) -> (TextToSpeechRequest, Option<TextToSpeechParams>) {
    let supports_language_code = !model_version.contains("multilingual");

    info!("[DEBUG] ElevenLabs synthesis_options_to_tts_request - Model compatibility check:");

    let default_request = TextToSpeechRequest {
        text: String::new(),
        model_id: Some(model_version.to_string()),
        language_code: if supports_language_code {
            Some("en".to_string())
        } else {
            None
        },
        voice_settings: None,
        pronunciation_dictionary_locators: None,
        seed: None,
        previous_text: None,
        next_text: None,
        previous_request_ids: None,
        next_request_ids: None,
        apply_text_normalization: Some("auto".to_string()),
        apply_language_text_normalization: Some(false),
        use_pvc_as_ivc: Some(false),
    };

    let default_params = TextToSpeechParams {
        enable_logging: Some(false),
        optimize_streaming_latency: None,
        output_format: Some("mp3_22050_32".to_string()),
    };

    if let Some(opts) = options {
        let mut request = default_request;
        let mut params = default_params;

        if let Some(voice_settings) = opts.voice_settings {
            request.voice_settings = Some(voice_settings_to_elevenlabs(voice_settings));
        }

        if let Some(audio_config) = opts.audio_config {
            params.output_format = Some(audio_format_to_string(audio_config.format));
        }

        if let Some(seed) = opts.seed {
            request.seed = Some(seed);
        }

        if let Some(model_version) = opts.model_version {
            request.model_id = Some(model_version);
        }

        if let Some(context) = opts.context {
            request.previous_text = context.previous_text;
            request.next_text = context.next_text;
        }

        (request, Some(params))
    } else {
        (default_request, Some(default_params))
    }
}

pub fn voice_settings_to_elevenlabs(settings: VoiceSettings) -> ElevenLabsVoiceSettings {
    let elevenlabs_settings = ElevenLabsVoiceSettings {
        stability: settings.stability.map(|s| s.clamp(0.0, 1.0)),
        similarity_boost: settings.similarity.map(|s| s.clamp(0.0, 1.0)),
        style: None,
        use_speaker_boost: None,
        speed: None,
    };
    info!(
        "[DEBUG]   Original stability: {:?} -> Clamped: {:?}",
        settings.stability, elevenlabs_settings.stability
    );
    info!(
        "[DEBUG]   Original similarity: {:?} -> Clamped: {:?}",
        settings.similarity, elevenlabs_settings.similarity_boost
    );

    elevenlabs_settings
}

pub fn audio_format_to_string(format: AudioFormat) -> String {
    match format {
        AudioFormat::Mp3 => "mp3_22050_32".to_string(),
        AudioFormat::Wav => "pcm_22050".to_string(),
        AudioFormat::Pcm => "pcm_22050".to_string(),
        AudioFormat::OggOpus => "mp3_22050_32".to_string(),
        AudioFormat::Aac => "mp3_22050_32".to_string(),
        AudioFormat::Flac => "pcm_22050".to_string(),
        AudioFormat::Mulaw => "pcm_22050".to_string(),
        AudioFormat::Alaw => "pcm_22050".to_string(),
    }
}

pub fn audio_data_to_synthesis_result(audio_data: Vec<u8>, text: &str) -> SynthesisResult {
    let audio_size = audio_data.len() as u32;

    let word_count = text.split_whitespace().count();
    let estimated_duration = if word_count > 0 {
        (word_count as f32 / 150.0) * 60.0
    } else {
        0.0
    };

    SynthesisResult {
        audio_data,
        metadata: SynthesisMetadata {
            duration_seconds: estimated_duration,
            character_count: text.len() as u32,
            word_count: word_count as u32,
            audio_size_bytes: audio_size,
            request_id: format!("elevenlabs-{}", uuid::Uuid::new_v4()),
            provider_info: Some("ElevenLabs".to_string()),
        },
    }
}

pub fn create_voice_request_from_samples(
    name: String,
    description: Option<String>,
    samples: Vec<AudioSample>,
) -> CreateVoiceRequest {
    use crate::client::AudioFile;

    let files = samples
        .into_iter()
        .map(|sample| AudioFile { data: sample.data })
        .collect();

    CreateVoiceRequest {
        name,
        description,
        files,
        labels: None,
    }
}

pub fn voice_design_params_to_create_request(params: VoiceDesignParams) -> CreateVoiceRequest {
    let gender_str = match params.gender {
        VoiceGender::Male => "male",
        VoiceGender::Female => "female",
        VoiceGender::Neutral => "neutral",
    };
    let age_str = match params.age_category {
        AgeCategory::Child => "child",
        AgeCategory::YoungAdult => "young_adult",
        AgeCategory::MiddleAged => "middle_aged",
        AgeCategory::Elderly => "elderly",
    };

    CreateVoiceRequest {
        name: format!("Generated_{}_{}_{}", gender_str, age_str, params.accent),
        description: Some(format!(
            "Generated voice with traits: {:?}",
            params.personality_traits
        )),
        files: vec![],
        labels: None,
    }
}

pub fn infer_gender_from_name(name: &str) -> Option<VoiceGender> {
    let name_lower = name.to_lowercase();
    if name_lower.contains("female") || name_lower.contains("woman") {
        Some(VoiceGender::Female)
    } else if name_lower.contains("male") || name_lower.contains("man") {
        Some(VoiceGender::Male)
    } else {
        None
    }
}

pub fn infer_quality_from_category(category: &str) -> Option<VoiceQuality> {
    match category.to_lowercase().as_str() {
        "premade" => Some(VoiceQuality::Standard),
        "cloned" => Some(VoiceQuality::Premium),
        "professional" => Some(VoiceQuality::Studio),
        _ => Some(VoiceQuality::Standard),
    }
}

pub fn models_to_language_info(models: Vec<Model>) -> Vec<LanguageInfo> {
    let mut language_map = std::collections::HashMap::new();

    for model in models {
        for lang in model.languages {
            let entry = language_map
                .entry(lang.language_id.clone())
                .or_insert_with(|| LanguageInfo {
                    code: lang.language_id.clone(),
                    name: lang.name.clone(),
                    native_name: get_native_language_name(&lang.language_id),
                    voice_count: 0,
                });
            entry.voice_count += 1;
        }
    }

    if language_map.is_empty() {
        return get_default_language_info();
    }

    let mut languages: Vec<LanguageInfo> = language_map.into_values().collect();
    languages.sort_by(|a, b| b.voice_count.cmp(&a.voice_count));
    languages
}

fn get_native_language_name(language_code: &str) -> String {
    match language_code {
        "en" | "en-US" | "en-GB" => "English".to_string(),
        "es" | "es-ES" | "es-MX" => "Español".to_string(),
        "fr" | "fr-FR" => "Français".to_string(),
        "de" | "de-DE" => "Deutsch".to_string(),
        "it" | "it-IT" => "Italiano".to_string(),
        "pt" | "pt-PT" | "pt-BR" => "Português".to_string(),
        "pl" | "pl-PL" => "Polski".to_string(),
        "hi" | "hi-IN" => "हिन्दी".to_string(),
        "ar" | "ar-SA" => "العربية".to_string(),
        "zh" | "zh-CN" => "中文".to_string(),
        "ja" | "ja-JP" => "日本語".to_string(),
        "ko" | "ko-KR" => "한국어".to_string(),
        "ru" | "ru-RU" => "Русский".to_string(),
        "nl" | "nl-NL" => "Nederlands".to_string(),
        "sv" | "sv-SE" => "Svenska".to_string(),
        "no" | "nb-NO" => "Norsk".to_string(),
        "da" | "da-DK" => "Dansk".to_string(),
        "fi" | "fi-FI" => "Suomi".to_string(),
        "tr" | "tr-TR" => "Türkçe".to_string(),
        "uk" | "uk-UA" => "Українська".to_string(),
        "cs" | "cs-CZ" => "Čeština".to_string(),
        "hu" | "hu-HU" => "Magyar".to_string(),
        "ro" | "ro-RO" => "Română".to_string(),
        "sk" | "sk-SK" => "Slovenčina".to_string(),
        "bg" | "bg-BG" => "Български".to_string(),
        "hr" | "hr-HR" => "Hrvatski".to_string(),
        "et" | "et-EE" => "Eesti".to_string(),
        "lv" | "lv-LV" => "Latviešu".to_string(),
        "lt" | "lt-LT" => "Lietuvių".to_string(),
        "sl" | "sl-SI" => "Slovenščina".to_string(),
        _ => language_code.to_string(),
    }
}

fn get_default_language_info() -> Vec<LanguageInfo> {
    vec![
        LanguageInfo {
            code: "en".to_string(),
            name: "English".to_string(),
            native_name: "English".to_string(),
            voice_count: 50,
        },
        LanguageInfo {
            code: "es".to_string(),
            name: "Spanish".to_string(),
            native_name: "Español".to_string(),
            voice_count: 20,
        },
        LanguageInfo {
            code: "fr".to_string(),
            name: "French".to_string(),
            native_name: "Français".to_string(),
            voice_count: 15,
        },
        LanguageInfo {
            code: "de".to_string(),
            name: "German".to_string(),
            native_name: "Deutsch".to_string(),
            voice_count: 10,
        },
        LanguageInfo {
            code: "it".to_string(),
            name: "Italian".to_string(),
            native_name: "Italiano".to_string(),
            voice_count: 8,
        },
        LanguageInfo {
            code: "pt".to_string(),
            name: "Portuguese".to_string(),
            native_name: "Português".to_string(),
            voice_count: 6,
        },
        LanguageInfo {
            code: "pl".to_string(),
            name: "Polish".to_string(),
            native_name: "Polski".to_string(),
            voice_count: 5,
        },
        LanguageInfo {
            code: "hi".to_string(),
            name: "Hindi".to_string(),
            native_name: "हिन्दी".to_string(),
            voice_count: 4,
        },
        LanguageInfo {
            code: "ar".to_string(),
            name: "Arabic".to_string(),
            native_name: "العربية".to_string(),
            voice_count: 3,
        },
        LanguageInfo {
            code: "zh".to_string(),
            name: "Chinese".to_string(),
            native_name: "中文".to_string(),
            voice_count: 2,
        },
    ]
}

pub fn validate_text_input(text: &str, model: Option<&str>) -> ValidationResult {
    let character_count = text.chars().count();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if text.trim().is_empty() {
        errors.push("Text cannot be empty".to_string());
    }

    if text.contains('\0') {
        warnings.push("Text contains null characters which may cause issues".to_string());
    }

    let max_chars = get_max_chars_for_model(model);
    if character_count > max_chars {
        warnings.push(format!(
            "Text length ({} characters) exceeds single request limit ({}). Will be automatically chunked.",
            character_count, max_chars
        ));
    }

    let is_valid = errors.is_empty();
    let estimated_duration = if is_valid {
        let word_count = text.split_whitespace().count();
        Some((word_count as f32 / 150.0) * 60.0)
    } else {
        None
    };

    ValidationResult {
        is_valid,
        character_count: character_count as u32,
        estimated_duration,
        warnings,
        errors,
    }
}

pub fn get_max_chars_for_model(model: Option<&str>) -> usize {
    if let Some(m) = model {
        if m.contains("turbo") {
            2500
        } else if m.contains("multilingual") {
            5000
        } else {
            4500
        }
    } else {
        4500
    }
}

pub fn split_text_intelligently(text: &str, max_chunk_size: usize) -> Vec<String> {
    if text.len() <= max_chunk_size {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();

    let paragraphs: Vec<&str> = text.split("\n\n").collect();

    for paragraph in paragraphs {
        if paragraph.len() > max_chunk_size {
            let sentences = split_by_sentences(paragraph);
            for sentence in sentences {
                if sentence.len() > max_chunk_size {
                    let clauses = split_by_clauses(&sentence, max_chunk_size);
                    for clause in clauses {
                        if current_chunk.len() + clause.len() < max_chunk_size {
                            if !current_chunk.is_empty() {
                                current_chunk.push(' ');
                            }
                            current_chunk.push_str(&clause);
                        } else {
                            if !current_chunk.is_empty() {
                                chunks.push(current_chunk.trim().to_string());
                                current_chunk.clear();
                            }
                            current_chunk = clause;
                        }
                    }
                } else if current_chunk.len() + sentence.len() < max_chunk_size {
                    if !current_chunk.is_empty() {
                        current_chunk.push(' ');
                    }
                    current_chunk.push_str(&sentence);
                } else {
                    if !current_chunk.is_empty() {
                        chunks.push(current_chunk.trim().to_string());
                        current_chunk.clear();
                    }
                    current_chunk = sentence;
                }
            }
        } else if current_chunk.len() + paragraph.len() + 2 <= max_chunk_size {
            if !current_chunk.is_empty() {
                current_chunk.push_str("\n\n");
            }
            current_chunk.push_str(paragraph);
        } else {
            if !current_chunk.is_empty() {
                chunks.push(current_chunk.trim().to_string());
                current_chunk.clear();
            }
            current_chunk = paragraph.to_string();
        }
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }

    chunks
        .into_iter()
        .filter(|chunk| !chunk.trim().is_empty())
        .collect()
}

fn split_by_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current_sentence = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        current_sentence.push(ch);

        if matches!(ch, '.' | '!' | '?') {
            if let Some(&next_char) = chars.peek() {
                if next_char.is_whitespace() || next_char.is_ascii_uppercase() {
                    sentences.push(current_sentence.trim().to_string());
                    current_sentence.clear();
                }
            } else {
                sentences.push(current_sentence.trim().to_string());
                current_sentence.clear();
            }
        }
    }

    if !current_sentence.trim().is_empty() {
        sentences.push(current_sentence.trim().to_string());
    }

    sentences
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .collect()
}

fn split_by_clauses(text: &str, max_size: usize) -> Vec<String> {
    if text.len() <= max_size {
        return vec![text.to_string()];
    }

    let mut clauses = Vec::new();
    let mut current_clause = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        current_clause.push(ch);

        if matches!(ch, ',' | ';' | ':') {
            if let Some(&next_char) = chars.peek() {
                if next_char.is_whitespace() {
                    clauses.push(current_clause.trim().to_string());
                    current_clause.clear();
                }
            }
        } else if current_clause.len() >= max_size {
            let last_space = current_clause.rfind(' ');
            if let Some(pos) = last_space {
                let (first_part, remainder) = current_clause.split_at(pos);
                clauses.push(first_part.trim().to_string());
                current_clause = remainder.trim().to_string();
            } else {
                clauses.push(current_clause.clone());
                current_clause.clear();
            }
        }
    }

    if !current_clause.trim().is_empty() {
        clauses.push(current_clause.trim().to_string());
    }

    clauses
        .into_iter()
        .filter(|c| !c.trim().is_empty())
        .collect()
}

pub fn validate_synthesis_input(
    input: &TextInput,
    options: Option<&SynthesisOptions>,
) -> Result<(), TtsError> {
    if input.content.trim().is_empty() {
        return Err(TtsError::InvalidText(
            "Text content cannot be empty".to_string(),
        ));
    }

    if input.text_type == TextType::Ssml {
        if let Err(msg) = validate_ssml_content(&input.content) {
            return Err(TtsError::InvalidSsml(msg));
        }
    }

    if let Some(ref language) = input.language {
        if !is_supported_language(language) {
            return Err(TtsError::UnsupportedLanguage(format!(
                "Language '{}' is not supported by ElevenLabs",
                language
            )));
        }
    }

    if let Some(opts) = options {
        if let Some(ref voice_settings) = opts.voice_settings {
            if let Err(msg) = validate_voice_settings(voice_settings) {
                return Err(TtsError::InvalidConfiguration(msg));
            }
        }
    }

    Ok(())
}

pub fn validate_ssml_content(content: &str) -> Result<(), String> {
    let mut tag_stack = Vec::new();
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            let mut tag = String::new();
            let mut is_closing = false;
            let mut is_self_closing = false;

            if chars.peek() == Some(&'/') {
                is_closing = true;
                chars.next();
            }

            let mut full_tag_content = String::new();
            while let Some(&ch) = chars.peek() {
                if ch == '>' {
                    break;
                }
                if ch == ' ' && tag.is_empty() {
                    tag = full_tag_content.clone();
                }
                full_tag_content.push(chars.next().unwrap());
            }

            if tag.is_empty() {
                tag = full_tag_content.clone();
            }

            if full_tag_content.ends_with('/') {
                is_self_closing = true;
                if tag.ends_with('/') {
                    tag = tag[..tag.len() - 1].to_string();
                }
            }

            for ch in chars.by_ref() {
                if ch == '>' {
                    break;
                }
            }

            if is_closing {
                if let Some(expected_tag) = tag_stack.pop() {
                    if expected_tag != tag {
                        return Err(format!("Unmatched closing tag: </{}>", tag));
                    }
                } else {
                    return Err(format!("Unmatched closing tag: </{}>", tag));
                }
            } else if !tag.is_empty()
                && !tag.starts_with('!')
                && !tag.starts_with('?')
                && !is_self_closing
            {
                tag_stack.push(tag);
            }
        }
    }

    if !tag_stack.is_empty() {
        return Err(format!("Unclosed tags: {:?}", tag_stack));
    }

    Ok(())
}

pub fn is_supported_language(language: &str) -> bool {
    let supported_languages = [
        "en", "en-US", "en-GB", "en-AU", "en-CA", "es", "es-ES", "es-MX", "es-AR", "fr", "fr-FR",
        "fr-CA", "de", "de-DE", "de-AT", "de-CH", "it", "it-IT", "pt", "pt-PT", "pt-BR", "pl",
        "pl-PL", "zh", "zh-CN", "zh-TW", "ja", "ja-JP", "hi", "hi-IN", "ko", "ko-KR", "nl",
        "nl-NL", "tr", "tr-TR", "sv", "sv-SE", "da", "da-DK", "no", "no-NO", "fi", "fi-FI",
    ];

    supported_languages.contains(&language)
}

pub fn validate_voice_settings(settings: &VoiceSettings) -> Result<(), String> {
    if let Some(speed) = settings.speed {
        if !(0.1..=5.0).contains(&speed) {
            return Err(format!(
                "Speed value {} is out of valid range (0.1-5.0)",
                speed
            ));
        }
    }

    if let Some(pitch) = settings.pitch {
        if !(-50.0..=50.0).contains(&pitch) {
            return Err(format!(
                "Pitch value {} is out of valid range (-50.0 to 50.0)",
                pitch
            ));
        }
    }

    if let Some(stability) = settings.stability {
        if !(0.0..=1.0).contains(&stability) {
            return Err(format!(
                "Stability value {} is out of valid range (0.0-1.0)",
                stability
            ));
        }
    }

    if let Some(similarity) = settings.similarity {
        if !(0.0..=1.0).contains(&similarity) {
            return Err(format!(
                "Similarity value {} is out of valid range (0.0-1.0)",
                similarity
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_filter_conversion() {
        let filter = VoiceFilter {
            gender: Some(VoiceGender::Female),
            quality: Some(VoiceQuality::Premium),
            language: Some("en".to_string()),
            supports_ssml: Some(true),
            provider: Some("ElevenLabs".to_string()),
            search_query: Some("narrator".to_string()),
        };

        let params = voice_filter_to_list_params(Some(filter)).unwrap();
        assert_eq!(params.voice_type, Some("female".to_string()));
        assert_eq!(params.category, Some("premium".to_string()));
        assert_eq!(params.search, Some("narrator".to_string()));
    }

    #[test]
    fn test_audio_format_conversion() {
        assert_eq!(audio_format_to_string(AudioFormat::Mp3), "mp3_22050_32");
        assert_eq!(audio_format_to_string(AudioFormat::Wav), "pcm_22050");
    }

    #[test]
    fn test_gender_inference() {
        assert_eq!(
            infer_gender_from_name("Sarah Female Voice"),
            Some(VoiceGender::Female)
        );
        assert_eq!(
            infer_gender_from_name("John Male Voice"),
            Some(VoiceGender::Male)
        );
        assert_eq!(infer_gender_from_name("Alex"), None);
    }

    #[test]
    fn test_quality_inference() {
        assert_eq!(
            infer_quality_from_category("premade"),
            Some(VoiceQuality::Standard)
        );
        assert_eq!(
            infer_quality_from_category("cloned"),
            Some(VoiceQuality::Premium)
        );
        assert_eq!(
            infer_quality_from_category("professional"),
            Some(VoiceQuality::Studio)
        );
    }

    #[test]
    fn test_validate_text_input() {
        let result = validate_text_input("Hello, world!", Some("eleven_multilingual_v2"));
        assert!(result.is_valid);
        assert_eq!(result.character_count, 13);

        let long_text = "a".repeat(6000);
        let result = validate_text_input(&long_text, Some("eleven_multilingual_v2"));
        assert!(result.is_valid);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_text_chunking() {
        let long_text =
            "This is a test. This is another sentence. And here's a third one. ".repeat(100);
        let chunks = split_text_intelligently(&long_text, 1000);
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.len() <= 1000);
            assert!(!chunk.is_empty());
        }
    }

    #[test]
    fn test_get_max_chars_for_model() {
        assert_eq!(get_max_chars_for_model(Some("eleven_turbo_v2")), 2500);
        assert_eq!(
            get_max_chars_for_model(Some("eleven_multilingual_v2")),
            5000
        );
        assert_eq!(get_max_chars_for_model(Some("eleven_monolingual_v1")), 4500);
        assert_eq!(get_max_chars_for_model(None), 4500);
    }

    #[test]
    fn test_validate_ssml_content() {
        assert!(validate_ssml_content("<speak>Hello <break time='1s'/> world</speak>").is_ok());

        assert!(validate_ssml_content("<speak>Hello <break>world</speak>").is_err());

        assert!(validate_ssml_content("<speak>Hello world").is_err());
    }

    #[test]
    fn test_validate_voice_settings() {
        use golem_tts::golem::tts::types::VoiceSettings;

        let settings = VoiceSettings {
            speed: Some(1.0),
            pitch: Some(0.0),
            volume: Some(1.0),
            stability: Some(0.5),
            similarity: Some(0.8),
            style: Some(0.3),
        };
        assert!(validate_voice_settings(&settings).is_ok());

        let invalid_settings = VoiceSettings {
            speed: None,
            pitch: None,
            volume: None,
            stability: Some(1.5),
            similarity: None,
            style: None,
        };
        assert!(validate_voice_settings(&invalid_settings).is_err());
    }
}

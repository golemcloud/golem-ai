use crate::client::{Model, TextToSpeechParams, TextToSpeechRequest};
use golem_tts::golem::tts::synthesis::{SynthesisOptions, ValidationResult};
use golem_tts::golem::tts::types::{
    AudioFormat, SynthesisMetadata, SynthesisResult, TextType, TtsError, VoiceGender, VoiceQuality,
    VoiceSettings,
};
use golem_tts::golem::tts::voices::{LanguageInfo, VoiceInfo};

pub fn estimate_audio_duration(audio_data: &[u8], sample_rate: u32) -> f32 {
    if audio_data.is_empty() {
        return 0.0;
    }

    let bytes_per_second = match sample_rate {
        8000 => 16000,
        16000 => 32000,
        22050 => 44100,
        24000 => 48000,
        48000 => 96000,
        _ => 48000,
    };

    (audio_data.len() as f32) / (bytes_per_second as f32)
}

pub fn deepgram_model_to_voice_info(model: Model) -> VoiceInfo {
    let gender = parse_gender(&model.gender);
    let quality = infer_quality_from_model(&model.voice_id);

    let language = normalize_language_code(&model.language);

    let is_custom = false;
    let is_cloned = false;

    VoiceInfo {
        id: model.voice_id.clone(),
        name: model.name.clone(),
        language: language.clone(),
        additional_languages: vec![],
        gender,
        quality,
        description: Some(format!(
            "{} voice with {} accent, {}. Characteristics: {}. Suitable for: {}",
            model.gender,
            model.accent,
            model.age,
            model.characteristics.join(", "),
            model.use_cases.join(", ")
        )),
        provider: "Deepgram".to_string(),
        sample_rate: get_default_sample_rate(),
        is_custom,
        is_cloned,
        preview_url: None,
        use_cases: model.use_cases.clone(),
    }
}

pub fn parse_gender(gender_str: &str) -> VoiceGender {
    match gender_str.to_lowercase().as_str() {
        "feminine" | "female" => VoiceGender::Female,
        "masculine" | "male" => VoiceGender::Male,
        _ => VoiceGender::Neutral,
    }
}

pub fn infer_quality_from_model(voice_id: &str) -> VoiceQuality {
    if voice_id.starts_with("aura-2-") {
        VoiceQuality::Premium
    } else {
        VoiceQuality::Standard
    }
}

pub fn normalize_language_code(code: &str) -> String {
    match code.to_lowercase().as_str() {
        "en-us" | "en-gb" | "en-au" | "en-ph" | "en-ie" => "en".to_string(),
        "es-es" | "es-mx" | "es-co" | "es-419" => "es".to_string(),
        _ => code.to_lowercase().chars().take(2).collect(),
    }
}

fn get_default_sample_rate() -> u32 {
    24000
}

pub fn validate_ssml(content: &str) -> Result<(), TtsError> {
    if content.trim_start().starts_with('<') && content.contains("speak") {
        return Err(TtsError::InvalidSsml(
            "Deepgram TTS does not support SSML markup".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_language_code(language: &str) -> Result<(), TtsError> {
    let supported_languages = [
        "en", "en-us", "en-gb", "en-au", "en-ph", "en-ie", "es", "es-es", "es-mx", "es-co",
        "es-419",
    ];

    let normalized = language.to_lowercase();
    let is_supported = supported_languages
        .iter()
        .any(|&lang| normalized == lang || normalized.starts_with(&format!("{}-", lang)));

    if !is_supported {
        return Err(TtsError::UnsupportedLanguage(format!(
            "Language '{}' is not supported by Deepgram TTS",
            language
        )));
    }

    Ok(())
}

pub fn validate_voice_settings(settings: &VoiceSettings) -> Result<(), TtsError> {
    let mut errors = Vec::new();

    if settings.speed.is_some() {
        errors.push("Speed adjustment not supported".to_string());
    }
    if settings.pitch.is_some() {
        errors.push("Pitch adjustment not supported".to_string());
    }
    if settings.volume.is_some() {
        errors.push("Volume adjustment not supported".to_string());
    }
    if settings.stability.is_some() {
        errors.push("Stability adjustment not supported".to_string());
    }
    if settings.similarity.is_some() {
        errors.push("Similarity adjustment not supported".to_string());
    }
    if settings.style.is_some() {
        errors.push("Style adjustment not supported".to_string());
    }

    if !errors.is_empty() {
        return Err(TtsError::InvalidConfiguration(format!(
            "Deepgram TTS does not support voice settings: {}",
            errors.join(", ")
        )));
    }

    Ok(())
}

pub fn synthesis_options_to_tts_request(
    text: String,
    options: Option<SynthesisOptions>,
) -> Result<(TextToSpeechRequest, Option<TextToSpeechParams>), TtsError> {
    let request = TextToSpeechRequest { text: text.clone() };

    let default_params = TextToSpeechParams {
        model: Some(get_default_model()),
        encoding: Some("linear16".to_string()),
        container: Some("wav".to_string()),
        sample_rate: Some(24000),
        bit_rate: None,
    };

    if let Some(opts) = options {
        let mut params = default_params;

        if let Some(ref voice_settings) = opts.voice_settings {
            validate_voice_settings(voice_settings)?;
        }

        if let Some(audio_config) = opts.audio_config {
            let (encoding, container, default_sample_rate, default_bit_rate) =
                audio_format_to_deepgram_params(audio_config.format);

            params.encoding = Some(encoding);
            params.container = container;

            match audio_config.format {
                AudioFormat::Mp3 | AudioFormat::Aac => {
                    params.sample_rate = None;
                }
                AudioFormat::OggOpus => {
                    params.sample_rate = None;
                }
                AudioFormat::Wav | AudioFormat::Pcm | AudioFormat::Flac => {
                    if let Some(user_rate) = audio_config.sample_rate {
                        let supported_rates = [8000, 16000, 24000, 32000, 48000];
                        if supported_rates.contains(&user_rate) {
                            params.sample_rate = Some(user_rate);
                        } else {
                            params.sample_rate = Some(24000);
                        }
                    } else {
                        params.sample_rate = Some(default_sample_rate);
                    }
                }
                _ => {
                    params.sample_rate = Some(default_sample_rate);
                }
            }

            match audio_config.format {
                AudioFormat::Mp3 | AudioFormat::OggOpus | AudioFormat::Aac => {
                    params.bit_rate = default_bit_rate;
                }
                _ => {
                    params.bit_rate = None;
                }
            }
        }

        if let Some(model_version) = opts.model_version {
            params.model = Some(model_version);
        }

        Ok((request, Some(params)))
    } else {
        Ok((request, Some(default_params)))
    }
}

fn get_default_model() -> String {
    std::env::var("DEEPGRAM_MODEL").unwrap_or_else(|_| "aura-2-asteria-en".to_string())
}

/// AudioFormat to Deepgram parameters
/// Based on Deepgram TTS API documentation: https://developers.deepgram.com/docs/text-to-speech
fn audio_format_to_deepgram_params(
    format: AudioFormat,
) -> (String, Option<String>, u32, Option<u32>) {
    match format {
        AudioFormat::Mp3 => ("mp3".to_string(), None, 22050, Some(48000)),
        AudioFormat::Wav => ("linear16".to_string(), Some("wav".to_string()), 24000, None),
        AudioFormat::Pcm => ("linear16".to_string(), None, 24000, None),
        AudioFormat::OggOpus => (
            "opus".to_string(),
            Some("ogg".to_string()),
            48000,
            Some(12000),
        ),
        AudioFormat::Aac => ("aac".to_string(), None, 22050, Some(48000)), // Deepgram's documented default
        AudioFormat::Flac => ("flac".to_string(), None, 48000, None),
        AudioFormat::Mulaw => ("mulaw".to_string(), Some("wav".to_string()), 8000, None),
        AudioFormat::Alaw => ("alaw".to_string(), Some("wav".to_string()), 8000, None),
    }
}

pub fn audio_data_to_synthesis_result(
    audio_data: Vec<u8>,
    text: &str,
    encoding: &str,
    sample_rate: u32,
) -> SynthesisResult {
    let audio_size = audio_data.len() as u32;
    let character_count = text.chars().count() as u32;
    let word_count = text.split_whitespace().count() as u32;

    let duration_seconds = estimate_audio_duration(&audio_data, sample_rate);

    let metadata = SynthesisMetadata {
        duration_seconds,
        character_count,
        word_count,
        audio_size_bytes: audio_size,
        request_id: format!("deepgram-{}", chrono::Utc::now().timestamp()),
        provider_info: Some(format!(
            "Deepgram TTS - Encoding: {}, Sample Rate: {}Hz",
            encoding, sample_rate
        )),
    };

    SynthesisResult {
        audio_data,
        metadata,
    }
}

pub fn models_to_language_info(models: Vec<Model>) -> Vec<LanguageInfo> {
    let mut language_map = std::collections::HashMap::new();

    for model in models {
        let lang_code = normalize_language_code(&model.language);

        if !language_map.contains_key(&lang_code) {
            let lang_info = LanguageInfo {
                code: lang_code.clone(),
                name: get_language_name(&lang_code),
                native_name: get_native_language_name(&lang_code),
                voice_count: 0,
            };
            language_map.insert(lang_code.clone(), lang_info);
        }

        if let Some(info) = language_map.get_mut(&lang_code) {
            info.voice_count += 1;
        }
    }

    let mut languages: Vec<LanguageInfo> = language_map.into_values().collect();
    languages.sort_by(|a, b| a.name.cmp(&b.name));
    languages
}

fn get_language_name(language_code: &str) -> String {
    match language_code {
        "en" => "English".to_string(),
        "es" => "Spanish".to_string(),
        _ => language_code.to_uppercase(),
    }
}

fn get_native_language_name(language_code: &str) -> String {
    match language_code {
        "en" => "English".to_string(),
        "es" => "Español".to_string(),
        _ => get_language_name(language_code),
    }
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
        if m.starts_with("aura-2-") {
            1000
        } else {
            2000
        }
    } else {
        1000
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

        if matches!(ch, ',' | ';' | ':') && current_clause.len() <= max_size {
            if let Some(&next_char) = chars.peek() {
                if next_char.is_whitespace() {
                    clauses.push(current_clause.trim().to_string());
                    current_clause.clear();
                }
            }
        } else if current_clause.len() >= max_size {
            if let Some(last_space) = current_clause.rfind(' ') {
                if last_space > max_size / 2 {
                    let (first_part, second_part) = current_clause.split_at(last_space);
                    clauses.push(first_part.trim().to_string());
                    current_clause = second_part.trim().to_string();
                }
            } else {
                clauses.push(current_clause.trim().to_string());
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

pub fn validate_synthesis_request(
    text: &str,
    text_type: TextType,
    language: Option<&str>,
    options: Option<&SynthesisOptions>,
) -> Result<(), TtsError> {
    if text.trim().is_empty() {
        return Err(TtsError::InvalidText("Text cannot be empty".to_string()));
    }

    if text_type == TextType::Ssml {
        validate_ssml(text)?;
    }

    if let Some(lang) = language {
        validate_language_code(lang)?;
    }

    if let Some(opts) = options {
        if let Some(ref voice_settings) = opts.voice_settings {
            validate_voice_settings(voice_settings)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gender() {
        assert_eq!(parse_gender("feminine"), VoiceGender::Female);
        assert_eq!(parse_gender("masculine"), VoiceGender::Male);
        assert_eq!(parse_gender("neutral"), VoiceGender::Neutral);
    }

    #[test]
    fn test_normalize_language_code() {
        assert_eq!(normalize_language_code("en-us"), "en");
        assert_eq!(normalize_language_code("es-mx"), "es");
        assert_eq!(normalize_language_code("en-gb"), "en");
    }

    #[test]
    fn test_audio_format_conversion() {
        let (encoding, container, sample_rate, _) =
            audio_format_to_deepgram_params(AudioFormat::Mp3);
        assert_eq!(encoding, "mp3");
        assert_eq!(container, None);
        assert_eq!(sample_rate, 22050);
    }
}

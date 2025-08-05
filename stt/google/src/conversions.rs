use crate::client::{
    AudioEncoding, RecognitionAudio, RecognitionConfig, RecognizeRequest, RecognizeResponse,
    SpeechContext, SpeechRecognitionAlternative, WordInfo,
};
use golem_stt::golem::stt::types::{
    AudioConfig, AudioFormat, SttError, TranscriptionMetadata,
    TranscriptionResult, TranscriptAlternative, WordSegment,
};
use golem_stt::golem::stt::transcription::TranscribeOptions;
use base64::prelude::*;

pub fn audio_format_to_encoding(format: &AudioFormat) -> Result<AudioEncoding, SttError> {
    match format {
        AudioFormat::Wav => Ok(AudioEncoding::Linear16),
        AudioFormat::Mp3 => Ok(AudioEncoding::Mp3),
        AudioFormat::Flac => Ok(AudioEncoding::Flac),
        AudioFormat::Ogg => Ok(AudioEncoding::OggOpus),
        AudioFormat::Aac => Err(SttError::UnsupportedFormat("AAC not supported by Google Speech".to_string())),
        AudioFormat::Pcm => Ok(AudioEncoding::Linear16),
    }
}

pub fn create_recognize_request(
    audio: &[u8],
    config: &AudioConfig,
    options: &Option<TranscribeOptions>,
) -> Result<RecognizeRequest, SttError> {
    let encoding = audio_format_to_encoding(&config.format)?;
    
    let audio_content = base64::prelude::BASE64_STANDARD.encode(audio);
    
    let mut recognition_config = RecognitionConfig {
        encoding,
        sample_rate_hertz: config.sample_rate.map(|rate| rate as i32),
        audio_channel_count: config.channels.map(|channels| channels as i32),
        language_code: None,
        alternative_language_codes: None,
        max_alternatives: Some(1),
        profanity_filter: None,
        speech_contexts: None,
        enable_word_time_offsets: None,
        enable_word_confidence: None,
        enable_automatic_punctuation: Some(true),
        model: None,
    };

    if let Some(opts) = options {
        if let Some(lang) = &opts.language {
            recognition_config.language_code = Some(lang.clone());
        }
        
        if let Some(model) = &opts.model {
            recognition_config.model = Some(model.clone());
        }
        
        if let Some(profanity_filter) = opts.profanity_filter {
            recognition_config.profanity_filter = Some(profanity_filter);
        }
        
        if let Some(enable_timestamps) = opts.enable_timestamps {
            recognition_config.enable_word_time_offsets = Some(enable_timestamps);
        }
        
        // Always enable word time offsets to get accurate duration
        recognition_config.enable_word_time_offsets = Some(true);
        
        if let Some(enable_word_confidence) = opts.enable_word_confidence {
            recognition_config.enable_word_confidence = Some(enable_word_confidence);
        }
        
        
        if let Some(speech_context) = &opts.speech_context {
            recognition_config.speech_contexts = Some(vec![SpeechContext {
                phrases: Some(speech_context.clone()),
                boost: Some(4.0), // Default boost value
            }]);
        }
        
    }

    Ok(RecognizeRequest {
        config: recognition_config,
        audio: RecognitionAudio {
            content: Some(audio_content),
            uri: None,
        },
        name: None,
    })
}

pub fn convert_response(
    response: RecognizeResponse,
    audio_size: usize,
    language: &str,
) -> Result<TranscriptionResult, SttError> {
    let results = response.results.unwrap_or_default();
    
    if results.is_empty() {
        return Ok(TranscriptionResult {
            alternatives: vec![],
            metadata: TranscriptionMetadata {
                duration_seconds: 0.0,
                audio_size_bytes: audio_size as u32,
                request_id: response.request_id.map(|id| id.to_string()).unwrap_or_else(|| "unknown".to_string()),
                model: None,
                language: language.to_string(),
            },
        });
    }

    let mut all_alternatives = vec![];
    let mut total_duration = 0.0f32;

    for result in results {
        if let Some(alternatives) = result.alternatives {
            for alt in alternatives {
                let alternative = convert_alternative(alt)?;
                if !alternative.words.is_empty() {
                    if let Some(last_word) = alternative.words.last() {
                        total_duration = total_duration.max(last_word.end_time);
                    }
                }
                all_alternatives.push(alternative);
            }
        }
    }

    // Use duration from Google's metadata if available, otherwise from word timestamps
    if total_duration == 0.0 {
        // Check if Google provided total_billed_time which contains duration
        if let Some(billed_time) = &response.total_billed_time {
            if let Some(parsed_duration) = parse_duration(billed_time) {
                total_duration = parsed_duration;
            }
        }
    }

    Ok(TranscriptionResult {
        alternatives: all_alternatives,
        metadata: TranscriptionMetadata {
            duration_seconds: total_duration,
            audio_size_bytes: audio_size as u32,
            request_id: response.request_id.map(|id| id.to_string()).unwrap_or_else(|| "unknown".to_string()),
            model: None,
            language: language.to_string(),
        },
    })
}

fn convert_alternative(alt: SpeechRecognitionAlternative) -> Result<TranscriptAlternative, SttError> {
    let text = alt.transcript.unwrap_or_default();
    let confidence = alt.confidence.unwrap_or(0.0);
    
    let words = if let Some(word_infos) = alt.words {
        word_infos.into_iter()
            .filter_map(|word_info| convert_word_info(word_info))
            .collect()
    } else {
        vec![]
    };

    Ok(TranscriptAlternative {
        text,
        confidence,
        words,
    })
}

fn convert_word_info(word_info: WordInfo) -> Option<WordSegment> {
    let word = word_info.word?;
    let start_time = parse_duration(&word_info.start_time?)?;
    let end_time = parse_duration(&word_info.end_time?)?;
    
    Some(WordSegment {
        text: word,
        start_time,
        end_time,
        confidence: word_info.confidence,
        speaker_id: word_info.speaker_tag.map(|tag| tag.to_string()),
    })
}

fn parse_duration(duration_str: &str) -> Option<f32> {
    // Google uses duration format like "1.234s"
    if let Some(stripped) = duration_str.strip_suffix('s') {
        stripped.parse::<f32>().ok()
    } else {
        duration_str.parse::<f32>().ok()
    }
}

pub fn get_supported_languages() -> Vec<golem_stt::golem::stt::languages::LanguageInfo> {
    vec![
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "en-US".to_string(),
            name: "English (United States)".to_string(),
            native_name: "English (United States)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "en-GB".to_string(),
            name: "English (United Kingdom)".to_string(),
            native_name: "English (United Kingdom)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "es-ES".to_string(),
            name: "Spanish (Spain)".to_string(),
            native_name: "Español (España)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "es-US".to_string(),
            name: "Spanish (United States)".to_string(),
            native_name: "Español (Estados Unidos)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "fr-FR".to_string(),
            name: "French (France)".to_string(),
            native_name: "Français (France)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "de-DE".to_string(),
            name: "German (Germany)".to_string(),
            native_name: "Deutsch (Deutschland)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "it-IT".to_string(),
            name: "Italian (Italy)".to_string(),
            native_name: "Italiano (Italia)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "pt-BR".to_string(),
            name: "Portuguese (Brazil)".to_string(),
            native_name: "Português (Brasil)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "ru-RU".to_string(),
            name: "Russian (Russia)".to_string(),
            native_name: "Русский (Россия)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "ja-JP".to_string(),
            name: "Japanese (Japan)".to_string(),
            native_name: "日本語（日本）".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "zh-CN".to_string(),
            name: "Chinese (Simplified, China)".to_string(),
            native_name: "中文（简体，中国）".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "ko-KR".to_string(),
            name: "Korean (South Korea)".to_string(),
            native_name: "한국어 (대한민국)".to_string(),
        },
    ]
}
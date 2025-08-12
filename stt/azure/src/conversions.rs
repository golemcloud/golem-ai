use crate::client::{
    TranscriptionRequest,
    AzureTranscriptionResponse, NBestItem,
};
use golem_stt::golem::stt::types::{
    AudioConfig, AudioFormat, SttError, TranscriptionMetadata,
    TranscriptionResult, TranscriptAlternative, WordSegment,
};
use golem_stt::golem::stt::transcription::TranscribeOptions;

pub fn audio_format_to_azure_format(format: &AudioFormat) -> Result<String, SttError> {
    match format {
        AudioFormat::Wav => Ok("wav".to_string()),
        AudioFormat::Mp3 => Ok("mp3".to_string()),
        AudioFormat::Flac => Ok("flac".to_string()),
        AudioFormat::Ogg => Ok("ogg".to_string()),
        AudioFormat::Aac => Ok("aac".to_string()),
        AudioFormat::Pcm => Ok("pcm".to_string()),
    }
}

pub fn create_realtime_transcription_request(
    audio: &[u8],
    config: &AudioConfig,
    options: &Option<TranscribeOptions>,
) -> Result<TranscriptionRequest, SttError> {
    let format = audio_format_to_azure_format(&config.format)?;
    
    let language = options
        .as_ref()
        .and_then(|opts| opts.language.as_ref())
        .cloned();

    let profanity_option = options
        .as_ref()
        .and_then(|opts| opts.profanity_filter)
        .map(|enabled| if enabled { "Masked".to_string() } else { "Raw".to_string() });

    Ok(TranscriptionRequest {
        audio_data: audio.to_vec(),
        language,
        format,
        profanity_option,
    })
}


pub fn convert_realtime_response(
    azure_response: AzureTranscriptionResponse,
    audio_size: usize,
    language: &str,
) -> Result<TranscriptionResult, SttError> {
    let mut alternatives = vec![];

    if azure_response.recognition_status == "Success" {
        if let Some(display_text) = azure_response.display_text {
            let alternative = TranscriptAlternative {
                text: display_text,
                confidence: 1.0, // Azure real-time API doesn't provide confidence in simple response
                words: vec![],
            };
            alternatives.push(alternative);
        }

        if let Some(n_best) = azure_response.n_best {
            for item in n_best {
                let words = extract_words_from_nbest_item(&item);
                let alternative = TranscriptAlternative {
                    text: item.display,
                    confidence: item.confidence,
                    words,
                };
                alternatives.push(alternative);
            }
        }
    }

    let duration = azure_response.duration
        .map(|d| d as f32 / 10_000_000.0) // Azure uses 100-nanosecond units
        .unwrap_or(0.0);

    Ok(TranscriptionResult {
        alternatives,
        metadata: TranscriptionMetadata {
            duration_seconds: duration,
            audio_size_bytes: audio_size as u32,
            request_id: "azure-realtime".to_string(),
            model: Some("Azure Speech Service".to_string()),
            language: language.to_string(),
        },
    })
}


fn extract_words_from_nbest_item(item: &NBestItem) -> Vec<WordSegment> {
    item.words.as_ref().map_or(vec![], |words| {
        words.iter().map(|word| {
            let start_time = word.offset as f32 / 10_000_000.0; // Convert from 100-nanosecond units
            let end_time = start_time + (word.duration as f32 / 10_000_000.0);
            
            WordSegment {
                text: word.word.clone(),
                start_time,
                end_time,
                confidence: word.confidence,
                speaker_id: None,
            }
        }).collect()
    })
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
            code: "en-AU".to_string(),
            name: "English (Australia)".to_string(),
            native_name: "English (Australia)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "en-CA".to_string(),
            name: "English (Canada)".to_string(),
            native_name: "English (Canada)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "es-ES".to_string(),
            name: "Spanish (Spain)".to_string(),
            native_name: "Español (España)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "es-MX".to_string(),
            name: "Spanish (Mexico)".to_string(),
            native_name: "Español (México)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "fr-FR".to_string(),
            name: "French (France)".to_string(),
            native_name: "Français (France)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "fr-CA".to_string(),
            name: "French (Canada)".to_string(),
            native_name: "Français (Canada)".to_string(),
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
            code: "pt-PT".to_string(),
            name: "Portuguese (Portugal)".to_string(),
            native_name: "Português (Portugal)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "ja-JP".to_string(),
            name: "Japanese (Japan)".to_string(),
            native_name: "日本語（日本）".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "ko-KR".to_string(),
            name: "Korean (South Korea)".to_string(),
            native_name: "한국어 (대한민국)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "zh-CN".to_string(),
            name: "Chinese (Simplified, China)".to_string(),
            native_name: "中文（简体，中国）".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "zh-TW".to_string(),
            name: "Chinese (Traditional, Taiwan)".to_string(),
            native_name: "中文（繁體，台灣）".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "ar-SA".to_string(),
            name: "Arabic (Saudi Arabia)".to_string(),
            native_name: "العربية (المملكة العربية السعودية)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "hi-IN".to_string(),
            name: "Hindi (India)".to_string(),
            native_name: "हिन्दी (भारत)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "ru-RU".to_string(),
            name: "Russian (Russia)".to_string(),
            native_name: "Русский (Россия)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "nl-NL".to_string(),
            name: "Dutch (Netherlands)".to_string(),
            native_name: "Nederlands (Nederland)".to_string(),
        },
    ]
}


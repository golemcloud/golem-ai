use crate::client::{
    PrerecordedTranscriptionRequest, DeepgramTranscriptionResponse, DeepgramAlternative, 
    DeepgramUtterance,
};
use golem_stt::golem::stt::types::{
    AudioConfig, AudioFormat, SttError, TranscriptionMetadata,
    TranscriptionResult, TranscriptAlternative, WordSegment,
};
use golem_stt::golem::stt::transcription::TranscribeOptions;

pub fn audio_format_to_deepgram_format(format: &AudioFormat) -> Result<&'static str, SttError> {
    match format {
        AudioFormat::Wav => Ok("wav"),
        AudioFormat::Mp3 => Ok("mp3"),
        AudioFormat::Flac => Ok("flac"),
        AudioFormat::Ogg => Ok("ogg"),
        AudioFormat::Aac => Ok("aac"),
        AudioFormat::Pcm => Ok("pcm"),
    }
}

pub fn create_prerecorded_request(
    audio: &[u8],
    config: &AudioConfig,
    options: &Option<TranscribeOptions>,
) -> Result<PrerecordedTranscriptionRequest, SttError> {
    let _format = audio_format_to_deepgram_format(&config.format)?;
    
    let language = options
        .as_ref()
        .and_then(|opts| opts.language.as_ref())
        .cloned();

    let model = options
        .as_ref()
        .and_then(|opts| opts.model.as_ref())
        .cloned();

    let punctuate = options
        .as_ref()
        .map(|opts| opts.enable_timestamps.unwrap_or(true))
        .unwrap_or(true);

    let diarize = options
        .as_ref()
        .and_then(|opts| opts.enable_speaker_diarization)
        .unwrap_or(false);

    let smart_format = options
        .as_ref()
        .and_then(|opts| opts.profanity_filter)
        .unwrap_or(true);

    let utterances = diarize; // Enable utterances when diarization is enabled

    let keywords = options
        .as_ref()
        .and_then(|opts| opts.speech_context.as_ref())
        .cloned();

    Ok(PrerecordedTranscriptionRequest {
        audio: audio.to_vec(),
        language,
        model,
        punctuate,
        diarize,
        smart_format,
        utterances,
        keywords,
    })
}

pub fn convert_deepgram_response(
    deepgram_response: DeepgramTranscriptionResponse,
    audio_size: usize,
    language: &str,
) -> Result<TranscriptionResult, SttError> {
    let mut alternatives = vec![];

    // Process alternatives from the first channel (Deepgram supports multi-channel)
    if let Some(first_channel) = deepgram_response.results.channels.first() {
        for alternative in &first_channel.alternatives {
            let words = extract_words_from_alternative(alternative);
            
            let transcript_alternative = TranscriptAlternative {
                text: alternative.transcript.clone(),
                confidence: alternative.confidence,
                words,
            };
            alternatives.push(transcript_alternative);
        }
    }

    // Process utterances if available (speaker diarization)
    if let Some(utterances) = deepgram_response.results.utterances {
        for utterance in utterances {
            let words = extract_words_from_utterance(&utterance);
            
            let transcript_alternative = TranscriptAlternative {
                text: utterance.transcript.clone(),
                confidence: utterance.confidence,
                words,
            };
            alternatives.push(transcript_alternative);
        }
    }

    Ok(TranscriptionResult {
        alternatives,
        metadata: TranscriptionMetadata {
            duration_seconds: deepgram_response.metadata.duration,
            audio_size_bytes: audio_size as u32,
            request_id: deepgram_response.metadata.request_id,
            model: deepgram_response.metadata.models.first().cloned(),
            language: language.to_string(),
        },
    })
}

fn extract_words_from_alternative(alternative: &DeepgramAlternative) -> Vec<WordSegment> {
    alternative.words.iter().map(|word| {
        WordSegment {
            text: word.punctuated_word.as_ref().unwrap_or(&word.word).clone(),
            start_time: word.start,
            end_time: word.end,
            confidence: Some(word.confidence),
            speaker_id: word.speaker.map(|s| s.to_string()),
        }
    }).collect()
}

fn extract_words_from_utterance(utterance: &DeepgramUtterance) -> Vec<WordSegment> {
    utterance.words.iter().map(|word| {
        WordSegment {
            text: word.punctuated_word.as_ref().unwrap_or(&word.word).clone(),
            start_time: word.start,
            end_time: word.end,
            confidence: Some(word.confidence),
            speaker_id: word.speaker.or(utterance.speaker).map(|s| s.to_string()),
        }
    }).collect()
}

pub fn get_supported_languages() -> Vec<golem_stt::golem::stt::languages::LanguageInfo> {
    vec![
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "en".to_string(),
            name: "English".to_string(),
            native_name: "English".to_string(),
        },
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
            code: "en-IN".to_string(),
            name: "English (India)".to_string(),
            native_name: "English (India)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "es".to_string(),
            name: "Spanish".to_string(),
            native_name: "Español".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "es-419".to_string(),
            name: "Spanish (Latin America)".to_string(),
            native_name: "Español (Latinoamérica)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "fr".to_string(),
            name: "French".to_string(),
            native_name: "Français".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "de".to_string(),
            name: "German".to_string(),
            native_name: "Deutsch".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "it".to_string(),
            name: "Italian".to_string(),
            native_name: "Italiano".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "pt".to_string(),
            name: "Portuguese".to_string(),
            native_name: "Português".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "pt-BR".to_string(),
            name: "Portuguese (Brazil)".to_string(),
            native_name: "Português (Brasil)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "nl".to_string(),
            name: "Dutch".to_string(),
            native_name: "Nederlands".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "ja".to_string(),
            name: "Japanese".to_string(),
            native_name: "日本語".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "ko".to_string(),
            name: "Korean".to_string(),
            native_name: "한국어".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "zh".to_string(),
            name: "Chinese".to_string(),
            native_name: "中文".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "zh-CN".to_string(),
            name: "Chinese (Simplified)".to_string(),
            native_name: "中文（简体）".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "zh-TW".to_string(),
            name: "Chinese (Traditional)".to_string(),
            native_name: "中文（繁體）".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "ru".to_string(),
            name: "Russian".to_string(),
            native_name: "Русский".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "ar".to_string(),
            name: "Arabic".to_string(),
            native_name: "العربية".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "hi".to_string(),
            name: "Hindi".to_string(),
            native_name: "हिन्दी".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "tr".to_string(),
            name: "Turkish".to_string(),
            native_name: "Türkçe".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "sv".to_string(),
            name: "Swedish".to_string(),
            native_name: "Svenska".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "da".to_string(),
            name: "Danish".to_string(),
            native_name: "Dansk".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "no".to_string(),
            name: "Norwegian".to_string(),
            native_name: "Norsk".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "pl".to_string(),
            name: "Polish".to_string(),
            native_name: "Polski".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "cs".to_string(),
            name: "Czech".to_string(),
            native_name: "Čeština".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "sk".to_string(),
            name: "Slovak".to_string(),
            native_name: "Slovenčina".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "uk".to_string(),
            name: "Ukrainian".to_string(),
            native_name: "Українська".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "ta".to_string(),
            name: "Tamil".to_string(),
            native_name: "தமிழ்".to_string(),
        },
    ]
}

// Helper function to get Deepgram model recommendations
pub fn get_recommended_model(language: &str, use_case: &str) -> String {
    match (language, use_case) {
        ("en" | "en-US", "general") => "nova-2".to_string(),
        ("en" | "en-US", "phone") => "nova-2-phonecall".to_string(),
        ("en" | "en-US", "meeting") => "nova-2-meeting".to_string(),
        ("en" | "en-US", "conversational") => "nova-2-conversationalai".to_string(),
        ("en" | "en-US", "finance") => "nova-2-finance".to_string(),
        ("en" | "en-US", "medical") => "nova-2-medical".to_string(),
        ("en" | "en-US", "drivethru") => "nova-2-drivethru".to_string(),
        ("en" | "en-US", "automotive") => "nova-2-automotive".to_string(),
        (_, _) => "nova-2".to_string(), // Default to nova-2 for all other cases
    }
}
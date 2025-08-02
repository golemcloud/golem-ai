use crate::client::{
    WhisperTranscriptionRequest, WhisperTranscriptionResponse, WhisperSegment,
};
use golem_stt::golem::stt::types::{
    AudioConfig, AudioFormat, SttError, TranscriptionMetadata,
    TranscriptionResult, TranscriptAlternative, WordSegment,
};
use golem_stt::golem::stt::transcription::TranscribeOptions;

pub fn audio_format_to_whisper_format(format: &AudioFormat) -> Result<&'static str, SttError> {
    match format {
        AudioFormat::Wav => Ok("wav"),
        AudioFormat::Mp3 => Ok("mp3"),
        AudioFormat::Flac => Ok("flac"),
        AudioFormat::Ogg => Ok("ogg"),
        AudioFormat::Aac => Ok("aac"),
        AudioFormat::Pcm => Ok("wav"), // PCM is typically in WAV container
    }
}

pub fn create_whisper_request(
    audio: &[u8],
    config: &AudioConfig,
    options: &Option<TranscribeOptions>,
) -> Result<WhisperTranscriptionRequest, SttError> {
    let _format = audio_format_to_whisper_format(&config.format)?;
    
    // Choose Whisper model based on requirements
    let model = options
        .as_ref()
        .and_then(|opts| opts.model.as_ref())
        .cloned()
        .unwrap_or_else(|| get_recommended_whisper_model(audio.len()));

    let language = options
        .as_ref()
        .and_then(|opts| opts.language.as_ref())
        .map(|lang| convert_language_code_to_whisper(lang));

    // Use vocabulary/context as prompt (Whisper's way of guiding transcription)
    let prompt = options
        .as_ref()
        .and_then(|opts| opts.speech_context.as_ref())
        .map(|keywords| keywords.join(" "));

    // Set response format to include timestamps if requested
    let response_format = if options
        .as_ref()
        .and_then(|opts| opts.enable_timestamps)
        .unwrap_or(false)
    {
        Some("verbose_json".to_string())
    } else {
        Some("json".to_string())
    };

    // Set timestamp granularities for word-level timestamps
    let timestamp_granularities = if options
        .as_ref()
        .and_then(|opts| opts.enable_word_confidence)
        .unwrap_or(false)
    {
        Some(vec!["word".to_string(), "segment".to_string()])
    } else if options
        .as_ref()
        .and_then(|opts| opts.enable_timestamps)
        .unwrap_or(false)
    {
        Some(vec!["segment".to_string()])
    } else {
        None
    };

    Ok(WhisperTranscriptionRequest {
        audio: audio.to_vec(),
        model,
        language,
        prompt,
        response_format,
        temperature: Some(0.0), // Use deterministic temperature
        timestamp_granularities,
    })
}

pub fn convert_whisper_response(
    whisper_response: WhisperTranscriptionResponse,
    audio_size: usize,
    requested_language: &str,
) -> Result<TranscriptionResult, SttError> {
    let mut alternatives = vec![];

    // Primary alternative from the main text
    let main_alternative = TranscriptAlternative {
        text: whisper_response.text.clone(),
        confidence: 1.0, // Whisper doesn't provide overall confidence, use 1.0
        words: extract_words_from_response(&whisper_response),
    };
    alternatives.push(main_alternative);

    // Additional alternatives from segments (if available)
    if let Some(segments) = &whisper_response.segments {
        for segment in segments {
            // Only add segments with low no_speech_prob as alternatives
            if segment.no_speech_prob < 0.5 && !segment.text.trim().is_empty() {
                let words = extract_words_from_segment(segment);
                let confidence = calculate_segment_confidence(segment);
                
                let alternative = TranscriptAlternative {
                    text: segment.text.trim().to_string(),
                    confidence,
                    words,
                };
                alternatives.push(alternative);
            }
        }
    }

    // Remove duplicate alternatives (keep only unique texts)
    alternatives.dedup_by(|a, b| a.text == b.text);

    let language = whisper_response.language
        .as_ref()
        .unwrap_or(&requested_language.to_string())
        .clone();

    Ok(TranscriptionResult {
        alternatives,
        metadata: TranscriptionMetadata {
            duration_seconds: whisper_response.duration.unwrap_or(0.0),
            audio_size_bytes: audio_size as u32,
            request_id: "whisper-transcription".to_string(),
            model: Some("OpenAI Whisper".to_string()),
            language,
        },
    })
}

fn extract_words_from_response(response: &WhisperTranscriptionResponse) -> Vec<WordSegment> {
    if let Some(words) = &response.words {
        words.iter().map(|word| {
            WordSegment {
                text: word.word.clone(),
                start_time: word.start,
                end_time: word.end,
                confidence: None, // Whisper doesn't provide word-level confidence
                speaker_id: None, // Whisper doesn't do speaker diarization
            }
        }).collect()
    } else if let Some(segments) = &response.segments {
        // Extract words from segments if word-level data isn't available
        let mut all_words = Vec::new();
        for segment in segments {
            if let Some(segment_words) = &segment.words {
                for word in segment_words {
                    all_words.push(WordSegment {
                        text: word.word.clone(),
                        start_time: word.start,
                        end_time: word.end,
                        confidence: None,
                        speaker_id: None,
                    });
                }
            }
        }
        all_words
    } else {
        vec![]
    }
}

fn extract_words_from_segment(segment: &WhisperSegment) -> Vec<WordSegment> {
    if let Some(words) = &segment.words {
        words.iter().map(|word| {
            WordSegment {
                text: word.word.clone(),
                start_time: word.start,
                end_time: word.end,
                confidence: None,
                speaker_id: None,
            }
        }).collect()
    } else {
        vec![]
    }
}

fn calculate_segment_confidence(segment: &WhisperSegment) -> f32 {
    // Use a combination of avg_logprob and no_speech_prob to estimate confidence
    let logprob_confidence = (segment.avg_logprob + 1.0).max(0.0).min(1.0);
    let speech_confidence = 1.0 - segment.no_speech_prob;
    
    // Weight both factors
    (logprob_confidence * 0.7 + speech_confidence * 0.3).max(0.0).min(1.0)
}

fn get_recommended_whisper_model(audio_size: usize) -> String {
    // Choose model based on audio size and performance requirements
    if audio_size > 10_000_000 { // > 10MB
        "whisper-1".to_string() // Use the standard model for large files
    } else {
        "whisper-1".to_string() // OpenAI currently only offers whisper-1
    }
}

fn convert_language_code_to_whisper(language: &str) -> String {
    // Convert common language codes to Whisper format (ISO 639-1)
    match language {
        "en-US" | "en-GB" | "en-AU" | "en-CA" | "en-IN" => "en".to_string(),
        "es-ES" | "es-MX" | "es-419" => "es".to_string(),
        "fr-FR" | "fr-CA" => "fr".to_string(),
        "de-DE" => "de".to_string(),
        "it-IT" => "it".to_string(),
        "pt-BR" | "pt-PT" => "pt".to_string(),
        "ja-JP" => "ja".to_string(),
        "ko-KR" => "ko".to_string(),
        "zh-CN" | "zh-TW" => "zh".to_string(),
        "ar-SA" => "ar".to_string(),
        "hi-IN" => "hi".to_string(),
        "ru-RU" => "ru".to_string(),
        "nl-NL" => "nl".to_string(),
        // For already ISO 639-1 codes, return as-is
        code if code.len() == 2 => code.to_string(),
        // Default to English for unrecognized codes
        _ => "en".to_string(),
    }
}

pub fn get_supported_languages() -> Vec<golem_stt::golem::stt::languages::LanguageInfo> {
    vec![
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "en".to_string(),
            name: "English".to_string(),
            native_name: "English".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "es".to_string(),
            name: "Spanish".to_string(),
            native_name: "Español".to_string(),
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
            code: "nl".to_string(),
            name: "Dutch".to_string(),
            native_name: "Nederlands".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "ru".to_string(),
            name: "Russian".to_string(),
            native_name: "Русский".to_string(),
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
            code: "bg".to_string(),
            name: "Bulgarian".to_string(),
            native_name: "Български".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "hr".to_string(),
            name: "Croatian".to_string(),
            native_name: "Hrvatski".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "da".to_string(),
            name: "Danish".to_string(),
            native_name: "Dansk".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "fi".to_string(),
            name: "Finnish".to_string(),
            native_name: "Suomi".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "he".to_string(),
            name: "Hebrew".to_string(),
            native_name: "עברית".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "hu".to_string(),
            name: "Hungarian".to_string(),
            native_name: "Magyar".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "is".to_string(),
            name: "Icelandic".to_string(),
            native_name: "Íslenska".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "id".to_string(),
            name: "Indonesian".to_string(),
            native_name: "Bahasa Indonesia".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "lv".to_string(),
            name: "Latvian".to_string(),
            native_name: "Latviešu".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "lt".to_string(),
            name: "Lithuanian".to_string(),
            native_name: "Lietuvių".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "no".to_string(),
            name: "Norwegian".to_string(),
            native_name: "Norsk".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "ro".to_string(),
            name: "Romanian".to_string(),
            native_name: "Română".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "sv".to_string(),
            name: "Swedish".to_string(),
            native_name: "Svenska".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "th".to_string(),
            name: "Thai".to_string(),
            native_name: "ไทย".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "vi".to_string(),
            name: "Vietnamese".to_string(),
            native_name: "Tiếng Việt".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "cy".to_string(),
            name: "Welsh".to_string(),
            native_name: "Cymraeg".to_string(),
        },
    ]
}
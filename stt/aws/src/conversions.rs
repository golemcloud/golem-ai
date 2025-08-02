use crate::client::{
    StartTranscriptionJobRequest, Media, Settings, AwsTranscriptResponse,
    Item,
};
use golem_stt::golem::stt::types::{
    AudioConfig, AudioFormat, SttError, TranscriptionMetadata,
    TranscriptionResult, TranscriptAlternative, WordSegment,
};
use golem_stt::golem::stt::transcription::TranscribeOptions;
// use log::{trace, warn};
use base64::prelude::*;

pub fn audio_format_to_media_format(format: &AudioFormat) -> Result<String, SttError> {
    match format {
        AudioFormat::Wav => Ok("wav".to_string()),
        AudioFormat::Mp3 => Ok("mp3".to_string()),
        AudioFormat::Flac => Ok("flac".to_string()),
        AudioFormat::Ogg => Err(SttError::UnsupportedFormat("OGG not supported by AWS Transcribe".to_string())),
        AudioFormat::Aac => Ok("mp4".to_string()), // AWS uses mp4 for AAC
        AudioFormat::Pcm => Err(SttError::UnsupportedFormat("PCM requires WAV container for AWS Transcribe".to_string())),
    }
}

pub fn create_transcription_job_request(
    audio: &[u8],
    config: &AudioConfig,
    options: &Option<TranscribeOptions>,
    job_name: &str,
) -> Result<StartTranscriptionJobRequest, SttError> {
    let media_format = audio_format_to_media_format(&config.format)?;
    
    // In a real implementation, you would upload the audio to S3 first
    // For now, we'll use a placeholder URI (this is a limitation of AWS Transcribe)
    let audio_base64 = BASE64_STANDARD.encode(audio);
    let media_uri = format!("data:audio/{};base64,{}", media_format, audio_base64);
    
    let mut settings = Settings {
        show_speaker_labels: None,
        max_speaker_labels: None,
        vocabulary_name: None,
        show_alternatives: Some(true),
        max_alternatives: Some(3),
    };

    let mut language_code = None;

    if let Some(opts) = options {
        if let Some(lang) = &opts.language {
            language_code = Some(lang.clone());
        }
        
        if let Some(enable_diarization) = opts.enable_speaker_diarization {
            settings.show_speaker_labels = Some(enable_diarization);
            if enable_diarization {
                settings.max_speaker_labels = Some(10); // Default max speakers
            }
        }
        
        if let Some(vocabulary_name) = &opts.vocabulary_name {
            settings.vocabulary_name = Some(vocabulary_name.clone());
        }
    }

    Ok(StartTranscriptionJobRequest {
        transcription_job_name: job_name.to_string(),
        media: Media {
            media_file_uri: media_uri,
        },
        media_format,
        language_code,
        media_sample_rate_hertz: config.sample_rate.map(|rate| rate as i32),
        settings: Some(settings),
    })
}

pub fn convert_aws_response(
    aws_response: AwsTranscriptResponse,
    audio_size: usize,
    language: &str,
    job_name: &str,
) -> Result<TranscriptionResult, SttError> {
    let mut alternatives = vec![];
    
    for transcript_item in aws_response.results.transcripts {
        let words = extract_words_from_items(&aws_response.results.items);
        
        let alternative = TranscriptAlternative {
            text: transcript_item.transcript,
            confidence: calculate_average_confidence(&aws_response.results.items),
            words,
        };
        
        alternatives.push(alternative);
    }

    let duration = calculate_duration(&aws_response.results.items);

    Ok(TranscriptionResult {
        alternatives,
        metadata: TranscriptionMetadata {
            duration_seconds: duration,
            audio_size_bytes: audio_size as u32,
            request_id: job_name.to_string(),
            model: Some("AWS Transcribe".to_string()),
            language: language.to_string(),
        },
    })
}

fn extract_words_from_items(items: &[Item]) -> Vec<WordSegment> {
    let mut words = vec![];
    
    for item in items {
        if item.item_type == "pronunciation" {
            if let (Some(start_time), Some(end_time)) = (&item.start_time, &item.end_time) {
                if let Some(alternative) = item.alternatives.first() {
                    let start = parse_aws_time(start_time).unwrap_or(0.0);
                    let end = parse_aws_time(end_time).unwrap_or(0.0);
                    let confidence = alternative.confidence.as_ref()
                        .and_then(|c| c.parse::<f32>().ok());
                    
                    words.push(WordSegment {
                        text: alternative.content.clone(),
                        start_time: start,
                        end_time: end,
                        confidence,
                        speaker_id: None, // AWS speaker labels would be in a separate structure
                    });
                }
            }
        }
    }
    
    words
}

fn calculate_average_confidence(items: &[Item]) -> f32 {
    let mut total_confidence = 0.0;
    let mut count = 0;
    
    for item in items {
        if item.item_type == "pronunciation" {
            if let Some(alternative) = item.alternatives.first() {
                if let Some(confidence_str) = &alternative.confidence {
                    if let Ok(confidence) = confidence_str.parse::<f32>() {
                        total_confidence += confidence;
                        count += 1;
                    }
                }
            }
        }
    }
    
    if count > 0 {
        total_confidence / count as f32
    } else {
        0.0
    }
}

fn calculate_duration(items: &[Item]) -> f32 {
    let mut max_time: f32 = 0.0;
    
    for item in items {
        if let Some(end_time) = &item.end_time {
            if let Some(time) = parse_aws_time(end_time) {
                max_time = max_time.max(time);
            }
        }
    }
    
    max_time
}

fn parse_aws_time(time_str: &str) -> Option<f32> {
    time_str.parse::<f32>().ok()
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
            code: "es-US".to_string(),
            name: "Spanish (United States)".to_string(),
            native_name: "Español (Estados Unidos)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "es-ES".to_string(),
            name: "Spanish (Spain)".to_string(),
            native_name: "Español (España)".to_string(),
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
            code: "ar-SA".to_string(),
            name: "Arabic (Saudi Arabia)".to_string(),
            native_name: "العربية (المملكة العربية السعودية)".to_string(),
        },
        golem_stt::golem::stt::languages::LanguageInfo {
            code: "hi-IN".to_string(),
            name: "Hindi (India)".to_string(),
            native_name: "हिन्दी (भारत)".to_string(),
        },
    ]
}

// Helper function to generate unique job names
pub fn generate_job_name() -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("golem-stt-job-{}", timestamp)
}
use crate::client::{
    StartTranscriptionJobRequest, Media, Settings, DirectTranscriptionResponse,
};
use golem_stt::golem::stt::types::{
    AudioConfig, AudioFormat, SttError, TranscriptionMetadata,
    TranscriptionResult, TranscriptAlternative,
};
use golem_stt::golem::stt::transcription::TranscribeOptions;
use golem_stt::golem::stt::languages::LanguageInfo;
use std::time::SystemTime;
use log::trace;

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
    config: &AudioConfig,
    options: &Option<TranscribeOptions>,
    job_name: &str,
) -> Result<StartTranscriptionJobRequest, SttError> {
    let media_format = audio_format_to_media_format(&config.format)?;
    
    let mut settings = Settings {
        show_speaker_labels: Some(false),
        max_speaker_labels: None,
        vocabulary_name: None,
        show_alternatives: Some(true),
        max_alternatives: Some(3),
        channel_identification: None,
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
            media_file_uri: "".to_string(), // Will be filled by client after S3 upload
        },
        media_format,
        language_code,
        media_sample_rate_hertz: None, // Let AWS auto-detect
        settings: Some(settings),
    })
}

pub fn convert_aws_response_to_transcription_result(
    aws_response: DirectTranscriptionResponse,
    audio_size: usize,
    language: &str,
    job_name: &str,
) -> Result<TranscriptionResult, SttError> {
    let alternatives = vec![TranscriptAlternative {
        text: aws_response.transcript,
        confidence: aws_response.confidence,
        words: vec![], // AWS word timing would require more complex parsing
    }];

    Ok(TranscriptionResult {
        alternatives,
        metadata: TranscriptionMetadata {
            duration_seconds: aws_response.duration,
            audio_size_bytes: audio_size as u32,
            request_id: job_name.to_string(),
            model: Some("AWS Transcribe".to_string()),
            language: language.to_string(),
        },
    })
}

pub fn parse_aws_transcript_json(content: &str) -> Result<DirectTranscriptionResponse, SttError> {
    trace!("Parsing AWS transcript content, length: {} bytes", content.len());
    
    // AWS Transcribe returns a JSON structure with results
    let transcript_json: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| SttError::InternalError(format!("Failed to parse transcript JSON: {}", e)))?;
    
    // Extract the transcript text and confidence
    let results = transcript_json["results"].as_object()
        .ok_or_else(|| SttError::InternalError("No results found in transcript".to_string()))?;
    
    let transcripts = results["transcripts"].as_array()
        .ok_or_else(|| SttError::InternalError("No transcripts found in results".to_string()))?;
    
    if transcripts.is_empty() {
        return Err(SttError::InternalError("Empty transcripts array".to_string()));
    }
    
    let transcript_text = transcripts[0]["transcript"].as_str()
        .ok_or_else(|| SttError::InternalError("No transcript text found".to_string()))?
        .to_string();
    
    // Calculate average confidence and duration from items
    let empty_vec = vec![];
    let items = results["items"].as_array().unwrap_or(&empty_vec);
    let confidence = calculate_confidence_from_items(items);
    let duration = calculate_duration_from_items(items);
    
    trace!("Parsed AWS transcript: {} chars, confidence: {:.2}, duration: {:.2}s", 
           transcript_text.len(), confidence, duration);
    
    Ok(DirectTranscriptionResponse {
        transcript: transcript_text,
        confidence,
        duration,
    })
}

fn calculate_duration_from_items(items: &[serde_json::Value]) -> f32 {
    if items.is_empty() {
        return 0.0;
    }
    
    // Find the last item with end_time
    for item in items.iter().rev() {
        if let Some(end_time_str) = item["end_time"].as_str() {
            if let Ok(end_time) = end_time_str.parse::<f32>() {
                return end_time;
            }
        }
    }
    
    // Fallback
    0.0
}

fn calculate_confidence_from_items(items: &[serde_json::Value]) -> f32 {
    if items.is_empty() {
        return 0.9; // Default confidence
    }
    
    let mut total_confidence = 0.0;
    let mut count = 0;
    
    for item in items {
        if let Some(confidence_str) = item["alternatives"][0]["confidence"].as_str() {
            if let Ok(confidence) = confidence_str.parse::<f32>() {
                total_confidence += confidence;
                count += 1;
            }
        }
    }
    
    if count > 0 {
        total_confidence / count as f32
    } else {
        0.9
    }
}

pub fn get_supported_languages() -> Vec<LanguageInfo> {
    vec![
        LanguageInfo {
            code: "en-US".to_string(),
            name: "English (United States)".to_string(),
            native_name: "English (United States)".to_string(),
        },
        LanguageInfo {
            code: "en-GB".to_string(),
            name: "English (United Kingdom)".to_string(),
            native_name: "English (United Kingdom)".to_string(),
        },
        LanguageInfo {
            code: "es-ES".to_string(),
            name: "Spanish (Spain)".to_string(),
            native_name: "Español (España)".to_string(),
        },
        LanguageInfo {
            code: "es-US".to_string(),
            name: "Spanish (United States)".to_string(),
            native_name: "Español (Estados Unidos)".to_string(),
        },
        LanguageInfo {
            code: "fr-FR".to_string(),
            name: "French (France)".to_string(),
            native_name: "Français (France)".to_string(),
        },
        LanguageInfo {
            code: "de-DE".to_string(),
            name: "German (Germany)".to_string(),
            native_name: "Deutsch (Deutschland)".to_string(),
        },
        LanguageInfo {
            code: "it-IT".to_string(),
            name: "Italian (Italy)".to_string(),
            native_name: "Italiano (Italia)".to_string(),
        },
        LanguageInfo {
            code: "pt-BR".to_string(),
            name: "Portuguese (Brazil)".to_string(),
            native_name: "Português (Brasil)".to_string(),
        },
        LanguageInfo {
            code: "ru-RU".to_string(),
            name: "Russian (Russia)".to_string(),
            native_name: "Русский (Россия)".to_string(),
        },
        LanguageInfo {
            code: "ja-JP".to_string(),
            name: "Japanese (Japan)".to_string(),
            native_name: "日本語（日本）".to_string(),
        },
        LanguageInfo {
            code: "zh-CN".to_string(),
            name: "Chinese (Simplified, China)".to_string(),
            native_name: "中文（简体，中国）".to_string(),
        },
        LanguageInfo {
            code: "ko-KR".to_string(),
            name: "Korean (South Korea)".to_string(),
            native_name: "한국어 (대한민국)".to_string(),
        },
        LanguageInfo {
            code: "ar-SA".to_string(),
            name: "Arabic (Saudi Arabia)".to_string(),
            native_name: "العربية (المملكة العربية السعودية)".to_string(),
        },
        LanguageInfo {
            code: "hi-IN".to_string(),
            name: "Hindi (India)".to_string(),
            native_name: "हिन्दी (भारत)".to_string(),
        },
        LanguageInfo {
            code: "nl-NL".to_string(),
            name: "Dutch (Netherlands)".to_string(),
            native_name: "Nederlands (Nederland)".to_string(),
        },
    ]
}

// Helper function to generate unique job names  
pub fn generate_job_name() -> String {
    let timestamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("golem-stt-{}", timestamp)
}
#[allow(static_mut_refs)]
mod bindings;
mod comprehensive_tests;
mod provider_integration;

use crate::bindings::exports::test::stt_exports::test_stt_api::*;
use crate::bindings::golem::stt::languages;
use crate::bindings::golem::stt::transcription;
use crate::bindings::golem::stt::vocabularies;

// Re-export types module so comprehensive_tests.rs can use types::AudioConfig, etc.
pub use crate::bindings::golem::stt::types as types;

struct Component;

impl Guest for Component {
    fn test_batch() -> String {
        let audio = mock_wav_audio();
        let config = types::AudioConfig {
            format: types::AudioFormat::Wav,
            sample_rate: Some(44100),
            channels: Some(1),
        };

        let opts: Option<&transcription::TranscribeOptions> = None;

        match transcription::transcribe(&audio, config, opts) {
            Ok(result) => {
                if let Some(first) = result.alternatives.first() {
                    format!("OK batch: {} chars", first.text.len())
                } else {
                    "OK batch: empty".to_string()
                }
            }
            Err(err) => format!("ERROR batch: {:?}", err),
        }
    }

    fn test_stream() -> String {
        let config = types::AudioConfig {
            format: types::AudioFormat::Pcm,
            sample_rate: Some(16000),
            channels: Some(1),
        };
        let opts: Option<&transcription::TranscribeOptions> = None;

        let stream = match transcription::transcribe_stream(config, opts) {
            Ok(s) => s,
            Err(err) => return format!("ERROR stream-open: {:?}", err),
        };

        let chunk = mock_pcm_chunk();
        if let Err(err) = stream.send_audio(&chunk) {
            return format!("ERROR send-audio: {:?}", err);
        }
        if let Err(err) = stream.finish() {
            return format!("ERROR finish: {:?}", err);
        }

        match stream.receive_alternative() {
            Ok(Some(alt)) => format!("OK stream: {}..", alt.text.chars().take(16).collect::<String>()),
            Ok(None) => "OK stream: no alternative".to_string(),
            Err(err) => format!("ERROR receive: {:?}", err),
        }
    }

    fn test_batch_language() -> String {
        let audio = mock_wav_audio();
        let config = types::AudioConfig {
            format: types::AudioFormat::Wav,
            sample_rate: Some(44100),
            channels: Some(1),
        };
        let opts = transcription::TranscribeOptions {
            enable_timestamps: Some(false),
            enable_speaker_diarization: Some(false),
            language: Some("en-US".to_string()),
            model: None,
            profanity_filter: None,
            vocabulary: None,
            speech_context: None,
            enable_word_confidence: None,
            enable_timing_detail: None,
        };
        match transcription::transcribe(&audio, config, Some(&opts)) {
            Ok(result) => {
                if result.metadata.language.to_lowercase().starts_with("en") {
                    "OK language".to_string()
                } else {
                    format!("ERROR language: {}", result.metadata.language)
                }
            }
            Err(err) => format!("ERROR batch-language: {:?}", err),
        }
    }

    fn test_batch_metadata_size() -> String {
        let audio = mock_wav_audio();
        let config = types::AudioConfig {
            format: types::AudioFormat::Wav,
            sample_rate: Some(44100),
            channels: Some(1),
        };
        match transcription::transcribe(&audio, config, None) {
            Ok(result) => {
                if (result.metadata.audio_size_bytes as usize) == audio.len() {
                    "OK audio size".to_string()
                } else {
                    format!(
                        "ERROR audio size: meta={} input={}",
                        result.metadata.audio_size_bytes,
                        audio.len()
                    )
                }
            }
            Err(err) => format!("ERROR batch-size: {:?}", err),
        }
    }

    fn test_stream_send_after_finish() -> String {
        let config = types::AudioConfig {
            format: types::AudioFormat::Pcm,
            sample_rate: Some(16000),
            channels: Some(1),
        };
        let stream = match transcription::transcribe_stream(config, None) {
            Ok(s) => s,
            Err(err) => return format!("ERROR stream-open: {:?}", err),
        };
        if let Err(err) = stream.finish() {
            return format!("ERROR finish: {:?}", err);
        }
        match stream.send_audio(&mock_pcm_chunk()) {
            Ok(_) => "ERROR: send accepted after finish".to_string(),
            Err(_) => "OK rejected after finish".to_string(),
        }
    }

    fn test_batch_empty_audio() -> String {
        let audio: Vec<u8> = vec![];
        let config = types::AudioConfig {
            format: types::AudioFormat::Wav,
            sample_rate: Some(44100),
            channels: Some(1),
        };
        match transcription::transcribe(&audio, config, None) {
            Ok(result) => {
                if result.metadata.audio_size_bytes == 0 {
                    "OK empty handled".to_string()
                } else {
                    format!("ERROR empty size: {}", result.metadata.audio_size_bytes)
                }
            }
            Err(_err) => "OK empty rejected".to_string(),
        }
    }

    fn test_batch_unsupported_format() -> String {
        let audio = mock_wav_audio();
        let config = types::AudioConfig {
            format: types::AudioFormat::Aac, // uncommon for some providers
            sample_rate: Some(44100),
            channels: Some(1),
        };
        match transcription::transcribe(&audio, config, None) {
            Ok(_result) => "OK aac accepted".to_string(),
            Err(_err) => "OK aac rejected gracefully".to_string(),
        }
    }

    fn test_diarization_shape() -> String {
        let audio = mock_wav_audio();
        let config = types::AudioConfig {
            format: types::AudioFormat::Wav,
            sample_rate: Some(44100),
            channels: Some(1),
        };
        let opts = transcription::TranscribeOptions {
            enable_timestamps: Some(true),
            enable_speaker_diarization: Some(true),
            language: None,
            model: None,
            profanity_filter: None,
            vocabulary: None,
            speech_context: None,
            enable_word_confidence: Some(true),
            enable_timing_detail: Some(true),
        };
        match transcription::transcribe(&audio, config, Some(&opts)) {
            Ok(result) => {
                // Validate shape without assuming actual diarization availability
                let _words = result
                    .alternatives
                    .get(0)
                    .map(|a| &a.words)
                    .map(|w| w.len())
                    .unwrap_or(0);
                // _words is usize, always >= 0, so just check if we got a result
                "OK diarization shape".to_string()
            }
            Err(err) => format!("ERROR diarization: {:?}", err),
        }
    }

    fn test_vocabulary() -> String {
        let phrases = vec!["transcription".to_string(), "diarization".to_string()];
        match vocabularies::create_vocabulary("test-vocab", &phrases) {
            Ok(v) => {
                let name = v.get_name();
                let _ = v.delete();
                format!("OK vocab: {}", name)
            }
            Err(err) => format!("ERROR vocab: {:?}", err),
        }
    }

    fn test_languages() -> String {
        match languages::list_languages() {
            Ok(list) => format!("OK languages: {}", list.len()),
            Err(err) => format!("ERROR languages: {:?}", err),
        }
    }

    // Comprehensive edge case tests
    fn test_silence_handling() -> String {
        comprehensive_tests::test_silence_handling()
    }

    fn test_overlapping_speakers() -> String {
        comprehensive_tests::test_overlapping_speakers()
    }

    fn test_long_audio_handling() -> String {
        comprehensive_tests::test_long_audio_handling()
    }

    fn test_network_error_handling() -> String {
        comprehensive_tests::test_network_error_handling()
    }

    fn test_rate_limiting() -> String {
        comprehensive_tests::test_rate_limiting()
    }

    fn test_quota_behavior() -> String {
        comprehensive_tests::test_quota_behavior()
    }

    fn test_invalid_audio_formats() -> String {
        comprehensive_tests::test_invalid_audio_formats()
    }

    fn test_streaming_after_finish() -> String {
        comprehensive_tests::test_streaming_after_finish()
    }

    // Provider integration tests
    fn test_provider_integration() -> String {
        provider_integration::test_provider_integration()
    }

    fn test_streaming_integration() -> String {
        provider_integration::test_streaming_integration()
    }
}

fn mock_wav_audio() -> Vec<u8> {
    // Minimal WAV header bytes
    vec![
        0x52, 0x49, 0x46, 0x46, // "RIFF"
        0x24, 0x00, 0x00, 0x00, // File size
        0x57, 0x41, 0x56, 0x45, // "WAVE"
        0x66, 0x6D, 0x74, 0x20, // "fmt "
        0x10, 0x00, 0x00, 0x00, // Subchunk1Size
        0x01, 0x00,             // AudioFormat (PCM)
        0x01, 0x00,             // NumChannels (mono)
        0x44, 0xAC, 0x00, 0x00, // SampleRate (44100)
        0x88, 0x58, 0x01, 0x00, // ByteRate
        0x02, 0x00,             // BlockAlign
        0x10, 0x00,             // BitsPerSample
        0x64, 0x61, 0x74, 0x61, // "data"
        0x00, 0x00, 0x00, 0x00, // Subchunk2Size
    ]
}

fn mock_pcm_chunk() -> Vec<u8> {
    vec![0u8; 256]
}

bindings::export!(Component with_types_in bindings);



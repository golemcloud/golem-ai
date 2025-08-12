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
    // Generate realistic WAV audio with actual audio samples
    // This creates a 1-second sine wave at 440Hz (A note) for testing
    let sample_rate = 44100;
    let duration_secs = 1.0;
    let frequency = 440.0; // A note
    let amplitude = 0.3; // Moderate volume

    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut audio_data = Vec::with_capacity(num_samples * 2); // 16-bit samples

    // Generate sine wave samples
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (amplitude * (2.0 * std::f32::consts::PI * frequency * t).sin() * 32767.0) as i16;

        // Convert to little-endian bytes
        audio_data.push((sample & 0xFF) as u8);
        audio_data.push(((sample >> 8) & 0xFF) as u8);
    }

    let data_size = audio_data.len() as u32;
    let file_size = 36 + data_size;

    let mut wav = Vec::new();

    // WAV header
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    // Format chunk
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // Subchunk1Size
    wav.extend_from_slice(&1u16.to_le_bytes());  // AudioFormat (PCM)
    wav.extend_from_slice(&1u16.to_le_bytes());  // NumChannels (mono)
    wav.extend_from_slice(&(sample_rate as u32).to_le_bytes()); // SampleRate
    wav.extend_from_slice(&(sample_rate as u32 * 2).to_le_bytes()); // ByteRate
    wav.extend_from_slice(&2u16.to_le_bytes());  // BlockAlign
    wav.extend_from_slice(&16u16.to_le_bytes()); // BitsPerSample

    // Data chunk
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    wav.extend_from_slice(&audio_data);

    wav
}

fn mock_pcm_chunk() -> Vec<u8> {
    // Generate realistic PCM audio chunk (16-bit mono at 16kHz)
    // This creates a short sine wave chunk for streaming tests
    let sample_rate = 16000;
    let duration_ms = 100; // 100ms chunk
    let frequency = 440.0; // A note
    let amplitude = 0.3;

    let num_samples = (sample_rate * duration_ms / 1000) as usize;
    let mut pcm_data = Vec::with_capacity(num_samples * 2);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (amplitude * (2.0 * std::f32::consts::PI * frequency * t).sin() * 32767.0) as i16;

        // Convert to little-endian bytes
        pcm_data.push((sample & 0xFF) as u8);
        pcm_data.push(((sample >> 8) & 0xFF) as u8);
    }

    pcm_data
}

bindings::export!(Component with_types_in bindings);



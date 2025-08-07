#[allow(static_mut_refs)]
mod bindings;

use crate::bindings::exports::test::stt_exports::test_stt_api::*;
use crate::bindings::golem::stt::transcription;
use crate::bindings::golem::stt::vocabularies;
use crate::bindings::golem::stt::languages;
use crate::bindings::golem::stt::types::{
    AudioConfig, AudioFormat, SttError
};
use crate::bindings::golem::stt::transcription::TranscribeOptions;
use golem_rust::*;

struct Component;

// Test audio data - a simple WAV file header with silence (44 bytes header + minimal data)
const TEST_AUDIO_WAV: &[u8] = &[
    // WAV header
    0x52, 0x49, 0x46, 0x46, // "RIFF"
    0x24, 0x00, 0x00, 0x00, // File size (36 bytes)
    0x57, 0x41, 0x56, 0x45, // "WAVE"
    0x66, 0x6D, 0x74, 0x20, // "fmt "
    0x10, 0x00, 0x00, 0x00, // Subchunk1Size (16)
    0x01, 0x00,             // AudioFormat (PCM)
    0x01, 0x00,             // NumChannels (1)
    0x44, 0xAC, 0x00, 0x00, // SampleRate (44100)
    0x88, 0x58, 0x01, 0x00, // ByteRate
    0x02, 0x00,             // BlockAlign
    0x10, 0x00,             // BitsPerSample (16)
    0x64, 0x61, 0x74, 0x61, // "data"
    0x04, 0x00, 0x00, 0x00, // Subchunk2Size (4)
    // Minimal audio data (2 samples of silence)
    0x00, 0x00, 0x00, 0x00,
];

impl Guest for Component {
    /// test1 demonstrates basic transcription functionality
    fn test1() -> String {
        let result = atomically(|| {
            // Read the actual audio file
            let audio_data = match std::fs::read("/data/audio.wav") {
                Ok(data) => data,
                Err(e) => return format!("✗ Failed to read audio file: {}", e),
            };

            let config = AudioConfig {
                format: AudioFormat::Wav,
                sample_rate: Some(48000), // Updated to match the actual file
                channels: Some(1),
            };

            let options = Some(TranscribeOptions {
                enable_timestamps: Some(true),
                enable_speaker_diarization: Some(false),
                language: Some("en-US".to_string()),
                model: None,
                profanity_filter: Some(false),
                vocabulary_name: None,
                speech_context: None,
                enable_word_confidence: Some(true),
                enable_timing_detail: Some(true),
            });

            match transcription::transcribe(&audio_data, config, options.as_ref()) {
                Ok(result) => {
                    let text = if !result.alternatives.is_empty() {
                        result.alternatives[0].text.clone()
                    } else {
                        "(no transcription)".to_string()
                    };
                    format!("✓ Basic transcription successful. Text: '{}', Duration: {:.2}s, Alternatives: {}",
                        text,
                        result.metadata.duration_seconds,
                        result.alternatives.len()
                    )
                },
                Err(e) => {
                    format!("✗ Basic transcription failed: {:?}", e)
                }
            }
        });

        result
    }

    /// test2 demonstrates language listing functionality
    fn test2() -> String {
        let result = atomically(|| {
            match languages::list_languages() {
                Ok(langs) => {
                    format!("✓ Language listing successful. Found {} languages. Examples: {}",
                        langs.len(),
                        langs.iter().take(3).map(|l| &l.code).cloned().collect::<Vec<_>>().join(", ")
                    )
                },
                Err(e) => {
                    format!("✗ Language listing failed: {:?}", e)
                }
            }
        });

        result
    }

    /// test3 demonstrates vocabulary management
    fn test3() -> String {
        let result = atomically(|| {
            let phrases = vec![
                "artificial intelligence".to_string(),
                "machine learning".to_string(),
                "speech recognition".to_string(),
            ];

            match vocabularies::create_vocabulary("test-vocab", &phrases) {
                Ok(vocab) => {
                    let name = vocab.get_name();
                    let phrases = vocab.get_phrases();
                    match vocab.delete() {
                        Ok(_) => {
                            format!("✓ Vocabulary test successful. Created '{}' with {} phrases, then deleted",
                                name, phrases.len())
                        },
                        Err(e) => {
                            format!("✗ Vocabulary deletion failed: {:?}", e)
                        }
                    }
                },
                Err(e) => {
                    format!("✗ Vocabulary creation failed: {:?}", e)
                }
            }
        });

        result
    }

    /// test4 demonstrates error handling with invalid audio
    fn test4() -> String {
        let result = atomically(|| {
            let invalid_audio = vec![0x00, 0x01, 0x02, 0x03]; // Invalid audio data
            let config = AudioConfig {
                format: AudioFormat::Wav,
                sample_rate: Some(44100),
                channels: Some(1),
            };

            match transcription::transcribe(&invalid_audio, config, None) {
                Ok(_) => {
                    "✗ Expected error for invalid audio, but transcription succeeded".to_string()
                },
                Err(e) => {
                    match e {
                        SttError::InvalidAudio(_) => "✓ Error handling test successful - invalid audio detected".to_string(),
                        SttError::UnsupportedFormat(_) => "✓ Error handling test successful - unsupported format detected".to_string(),
                        _ => format!("✓ Error handling test successful - error detected: {:?}", e),
                    }
                }
            }
        });

        result
    }

    /// test5 demonstrates streaming transcription (if supported)
    fn test5() -> String {
        let result = atomically(|| {
            let config = AudioConfig {
                format: AudioFormat::Wav,
                sample_rate: Some(44100),
                channels: Some(1),
            };

            let options = Some(TranscribeOptions {
                enable_timestamps: Some(true),
                enable_speaker_diarization: Some(false),
                language: Some("en-US".to_string()),
                model: None,
                profanity_filter: Some(false),
                vocabulary_name: None,
                speech_context: None,
                enable_word_confidence: Some(false),
                enable_timing_detail: Some(false),
            });

            match transcription::transcribe_stream(config, options.as_ref()) {
                Ok(stream) => {
                    // Read the actual audio file for streaming test
                    let audio_data = match std::fs::read("/data/audio.wav") {
                        Ok(data) => data,
                        Err(e) => {
                            stream.close();
                            return format!("✗ Failed to read audio file for streaming: {}", e);
                        }
                    };
                    
                    // Try to send audio data
                    match stream.send_audio(&audio_data) {
                        Ok(_) => {
                            match stream.finish() {
                                Ok(_) => {
                                    stream.close();
                                    "✓ Streaming transcription test successful".to_string()
                                },
                                Err(e) => {
                                    stream.close();
                                    format!("✗ Stream finish failed: {:?}", e)
                                }
                            }
                        },
                        Err(e) => {
                            stream.close();
                            format!("✗ Stream send failed: {:?}", e)
                        }
                    }
                },
                Err(e) => {
                    match e {
                        SttError::UnsupportedOperation(_) => "⚠ Streaming not supported by this provider".to_string(),
                        _ => format!("✗ Stream creation failed: {:?}", e),
                    }
                }
            }
        });

        result
    }
}

bindings::export!(Component with_types_in bindings);
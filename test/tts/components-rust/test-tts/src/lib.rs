#[allow(static_mut_refs)]
mod bindings;

use crate::bindings::exports::test::tts_exports::test_tts_api::*;
use crate::bindings::golem::tts::types::{
    TtsError, TextInput, AudioConfig, VoiceSettings, AudioFormat, AudioEffects,
    TextType, VoiceGender, VoiceQuality, TimingInfo,
    SynthesisResult, SynthesisMetadata, QuotaInfo
};
use crate::bindings::golem::tts::voices::{
    Voice, VoiceFilter, VoiceInfo, VoiceResults, LanguageInfo,
    list_voices, get_voice, search_voices, list_languages
};
use crate::bindings::golem::tts::synthesis::{
    SynthesisOptions, SynthesisContext, ValidationResult,
    synthesize, synthesize_batch, validate_input, get_timing_marks
};
use crate::bindings::golem::tts::streaming::{
    SynthesisStream, StreamStatus, VoiceConversionStream,
    create_stream, create_voice_conversion_stream
};
use crate::bindings::golem::tts::advanced::{
    AudioSample, VoiceDesignParams, AgeCategory, PronunciationEntry,
    PronunciationLexicon, LongFormOperation, OperationStatus, LongFormResult,
    create_voice_clone, design_voice, convert_voice, generate_sound_effect,
    create_lexicon, synthesize_long_form
};
use crate::bindings::test::helper_client::test_helper_client::TestHelperApi;
use golem_rust::atomically;
use std::fs;
use std::thread;
use std::time::Duration;

struct Component;

#[cfg(feature = "elevenlabs")]
const TEST_PROVIDER: &'static str = "ELEVENLABS";
#[cfg(feature = "deepgram")]
const TEST_PROVIDER: &'static str = "DEEPGRAM";
#[cfg(feature = "google")]
const TEST_PROVIDER: &'static str = "GOOGLE";
#[cfg(feature = "aws")]
const TEST_PROVIDER: &'static str = "AWS";

// Test constants
const SHORT_TEXT: &str = "Hello, this is a test of text-to-speech synthesis.";
const MEDIUM_TEXT: &str = "This is a longer text for testing TTS functionality. It contains multiple sentences. Each sentence should be synthesized clearly and with proper pronunciation.";
const LONG_TEXT: &str = "This is a comprehensive test of long-form text-to-speech synthesis. The text should be processed efficiently and produce high-quality audio output. This test verifies that the TTS system can handle extended content while maintaining consistent voice quality, proper pacing, and accurate pronunciation throughout the entire synthesis process. The system should demonstrate robust performance across various text lengths and complexities.";
const SSML_TEXT: &str = r#"<speak>
    <p>Welcome to our <emphasis level="strong">advanced</emphasis> text-to-speech testing.</p>
    <break time="1s"/>
    <p>This sentence has a <prosody rate="slow">slow speaking rate</prosody>.</p>
    <p>And this one has a <prosody pitch="high">higher pitch</prosody>.</p>
</speak>"#;

impl Guest for Component {
    /// test0 demonstrates basic voice discovery and metadata retrieval
    fn test0() -> String {
        println!("Test0: Voice discovery and metadata retrieval");
        let mut results = Vec::new();

        // Testing voice listing with no filters
        println!("Listing all available voices...");
        match list_voices(None) {
            Ok(voice_results) => {
                results.push("✓ Voice listing successful".to_string());
                
                // Testing voice results iteration
                let mut voice_count = 0;
                while voice_results.has_more() {
                    match voice_results.get_next() {
                        Ok(voices) => {
                            voice_count += voices.len();
                            for voice_info in voices.iter() {
                                println!("Found voice: {} ({})", voice_info.name, voice_info.language);
                            }
                            if voices.len() < 10 { break; } 
                        }
                        Err(e) => {
                            results.push(format!("✗ Error getting voice batch: {:?}", e));
                            break;
                        }
                    }
                }
                results.push(format!("✓ Found {} voices total", voice_count));
                
                if let Some(total) = voice_results.get_total_count() {
                    results.push(format!("✓ Total voice count: {}", total));
                }
            }
            Err(e) => results.push(format!("✗ Voice listing failed: {:?}", e)),
        }

        println!("Testing voice filtering...");
        let filter = VoiceFilter {
            language: Some("en-US".to_string()),
            gender: Some(VoiceGender::Female),
            quality: Some(VoiceQuality::Neural),
            supports_ssml: Some(true),
            provider: None,
            search_query: None,
        };

        match list_voices(Some(&filter)) {
            Ok(filtered_results) => {
                results.push("✓ Voice filtering successful".to_string());
                if let Some(total) = filtered_results.get_total_count() {
                    results.push(format!("✓ Filtered results: {} voices", total));
                }
            }
            Err(e) => results.push(format!("✗ Voice filtering failed: {:?}", e)),
        }

        println!("Testing language discovery...");
        match list_languages() {
            Ok(languages) => {
                results.push(format!("✓ Found {} supported languages", languages.len()));
                for lang in languages.iter().take(5) {
                    println!("Language: {} ({}) - {} voices", lang.name, lang.code, lang.voice_count);
                }
            }
            Err(e) => results.push(format!("✗ Language listing failed: {:?}", e)),
        }

        println!("Testing voice search...");
        match search_voices("natural", None) {
            Ok(search_results) => {
                results.push(format!("✓ Voice search found {} results", search_results.len()));
            }
            Err(e) => results.push(format!("✗ Voice search failed: {:?}", e)),
        }

        results.join("\n")
    }

    /// test2 demonstrates basic text-to-speech synthesis with various configurations
    fn test2() -> String {
        println!("Test2: Basic text-to-speech synthesis");
        let mut results = Vec::new();

        // Get a voice for testing
        let voice = match get_test_voice() {
            Ok(v) => v,
            Err(e) => return format!("Failed to get test voice: {}", e),
        };

        println!("Testing basic synthesis...");
        let text_input = TextInput {
            content: SHORT_TEXT.to_string(),
            text_type: TextType::Plain,
            language: Some("en-US".to_string()),
        };

        match synthesize(&text_input, &voice, None) {
            Ok(result) => {
                results.push("✓ Basic synthesis successful".to_string());
                save_audio_result(&result.audio_data, "test2-basic", "mp3");
            }
            Err(e) => results.push(format!("✗ Basic synthesis failed: {:?}", e)),
        }

        println!("Testing synthesis with audio configuration...");
        let audio_config = AudioConfig {
            format: AudioFormat::Wav,
            sample_rate: Some(22050),
            bit_rate: None,
            channels: Some(1),
        };

        let options = SynthesisOptions {
            audio_config: Some(audio_config),
            voice_settings: None,
            audio_effects: None,
            enable_timing: Some(true),
            enable_word_timing: Some(true),
            seed: None,
            model_version: None,
            context: None,
        };

        match synthesize(&text_input, &voice, Some(&options)) {
            Ok(result) => {
                results.push("✓ Synthesis with audio config successful".to_string());
                save_audio_result(&result.audio_data, "test2-config", "wav");
            }
            Err(e) => results.push(format!("✗ Synthesis with audio config failed: {:?}", e)),
        }

        // Testing synthesis with voice settings
        println!("Testing synthesis with voice settings...");
        let voice_settings = VoiceSettings {
            speed: Some(1.2),
            pitch: Some(2.0),
            volume: Some(0.0),
            stability: Some(0.8),
            similarity: Some(0.9),
            style: Some(0.5),
        };

        let voice_options = SynthesisOptions {
            audio_config: None,
            voice_settings: Some(voice_settings),
            audio_effects: Some(vec![AudioEffects::NoiseReduction, AudioEffects::HeadphoneOptimized]),
            enable_timing: None,
            enable_word_timing: None,
            seed: Some(42),
            model_version: None,
            context: None,
        };

        match synthesize(&text_input, &voice, Some(&voice_options)) {
            Ok(result) => {
                results.push("✓ Synthesis with voice settings successful".to_string());
                save_audio_result(&result.audio_data, "test2-voice-settings", "mp3");
            }
            Err(e) => results.push(format!("✗ Synthesis with voice settings failed: {:?}", e)),
        }

        results.join("\n")
    }

    /// test3 demonstrates SSML support and advanced text processing
    fn test3() -> String {
        println!("Test3: SSML support and advanced text processing");
        let mut results = Vec::new();

        let voice = match get_test_voice() {
            Ok(v) => v,
            Err(e) => return format!("Failed to get test voice: {}", e),
        };

        // Test SSML synthesis
        println!("Testing SSML synthesis...");
        let ssml_input = TextInput {
            content: SSML_TEXT.to_string(),
            text_type: TextType::Ssml,
            language: Some("en-US".to_string()),
        };

        match synthesize(&ssml_input.clone(), &voice, None) {
            Ok(result) => {
                results.push("✓ SSML synthesis successful".to_string());
                results.push(format!("✓ SSML audio duration: {:.2}s", result.metadata.duration_seconds));
                save_audio_result(&result.audio_data, "test3-ssml", "mp3");
            }
            Err(e) => results.push(format!("✗ SSML synthesis failed: {:?}", e)),
        }

        // Test input validation
        println!("Testing input validation...");
        match validate_input(&ssml_input.clone(), &voice) {
            Ok(validation) => {
                results.push(format!("✓ Input validation: valid={}", validation.is_valid));
                results.push(format!("✓ Character count: {}", validation.character_count));
                if let Some(duration) = validation.estimated_duration {
                    results.push(format!("✓ Estimated duration: {:.2}s", duration));
                }
                if !validation.warnings.is_empty() {
                    results.push(format!("⚠ Warnings: {}", validation.warnings.join(", ")));
                }
            }
            Err(e) => results.push(format!("✗ Input validation failed: {:?}", e)),
        }

        // Test timing marks
        println!("Testing timing marks extraction...");
        match get_timing_marks(&ssml_input, &voice) {
            Ok(timing_marks) => {
                results.push(format!("✓ Retrieved {} timing marks", timing_marks.len()));
                for (i, mark) in timing_marks.iter().take(3).enumerate() {
                    results.push(format!("  Mark {}: start={:.2}s, offset={:?}", 
                        i + 1, mark.start_time_seconds, mark.text_offset));
                }
            }
            Err(e) => results.push(format!("✗ Timing marks extraction failed: {:?}", e)),
        }

        // Test batch synthesis
        println!("Testing batch synthesis...");
        let batch_inputs = vec![
            TextInput {
                content: "First batch item.".to_string(),
                text_type: TextType::Plain,
                language: Some("en-US".to_string()),
            },
            TextInput {
                content: "Second batch item.".to_string(),
                text_type: TextType::Plain,
                language: Some("en-US".to_string()),
            },
        ];

        match synthesize_batch(&batch_inputs, &voice, None) {
            Ok(batch_results) => {
                results.push(format!("✓ Batch synthesis completed: {} items", batch_results.len()));
                for (i, result) in batch_results.iter().enumerate() {
                    save_audio_result(&result.audio_data, &format!("test3-batch-{}", i), "mp3");
                }
            }
            Err(e) => results.push(format!("✗ Batch synthesis failed: {:?}", e)),
        }

        results.join("\n")
    }

    /// test4 demonstrates streaming synthesis lifecycle
    fn test4() -> String {
        println!("Test4: Streaming synthesis lifecycle");
        let mut results = Vec::new();

        let voice = match get_test_voice() {
            Ok(v) => v,
            Err(e) => return format!("Failed to get test voice: {}", e),
        };

        // Test streaming synthesis
        println!("Creating streaming synthesis session...");
        let stream_options = SynthesisOptions {
            audio_config: Some(AudioConfig {
                format: AudioFormat::Wav,
                sample_rate: Some(24000),
                bit_rate: None, 
                channels: Some(1),
            }),
            voice_settings: None,
            audio_effects: None,
            enable_timing: Some(true),
            enable_word_timing: None,
            seed: None,
            model_version: None,
            context: None,
        };

        match create_stream(&voice, Some(&stream_options)) {
            Ok(stream) => {
                results.push("✓ Streaming session created".to_string());
                
                // Send text chunks
                let text_chunks = vec![
                    "This is the first chunk of streaming text. ",
                    "Here comes the second chunk with more content. ",
                    "And finally, the third chunk to complete the stream.",
                ];

                for (i, chunk) in text_chunks.iter().enumerate() {
                    let text_input = TextInput {
                        content: chunk.to_string(),
                        text_type: TextType::Plain,
                        language: Some("en-US".to_string()),
                    };

                    match stream.send_text(&text_input) {
                        Ok(_) => println!("Sent chunk {}", i + 1),
                        Err(e) => {
                            results.push(format!("✗ Failed to send chunk {}: {:?}", i + 1, e));
                            break;
                        }
                    }
                }

                // Signal end of input
                match stream.finish() {
                    Ok(_) => results.push("✓ Stream finished successfully".to_string()),
                    Err(e) => results.push(format!("✗ Stream finish failed: {:?}", e)),
                }

                // Collect streaming audio chunks
                let mut audio_data = Vec::new();
                let mut chunk_count = 0;
                let max_attempts = 30;
                let mut attempts = 0;

                while attempts < max_attempts {
                    if !stream.has_pending_audio() && 
                       matches!(stream.get_status(), StreamStatus::Finished) {
                        break;
                    }

                    match stream.receive_chunk() {
                        Ok(Some(chunk)) => {
                            chunk_count += 1;
                            audio_data.extend_from_slice(&chunk.data);
                            results.push(format!("Received chunk {} (seq: {}, final: {})", 
                                chunk_count, chunk.sequence_number, chunk.is_final));

                            if chunk.is_final {
                                break;
                            }
                        }
                        Ok(None) => {
                            thread::sleep(Duration::from_millis(100));
                        }
                        Err(e) => {
                            results.push(format!("✗ Chunk reception failed: {:?}", e));
                            break;
                        }
                    }
                    attempts += 1;
                }

                results.push(format!("✓ Received {} audio chunks", chunk_count));
                results.push(format!("✓ Total audio data: {} bytes", audio_data.len()));
                
                if !audio_data.is_empty() {
                    save_audio_result(&audio_data, "test4-streaming", "wav");
                }

                // Clean up
                stream.close();
                results.push("✓ Stream closed successfully".to_string());
            }
            Err(e) => results.push(format!("✗ Stream creation failed: {:?}", e)),
        }

        results.join("\n")
    }

    /// test5 demonstrates voice cloning and custom voice creation
    fn test5() -> String {
        println!("Test5: Voice cloning and custom voice creation");
        let mut results = Vec::new();

        // Test voice cloning (may not be supported by all providers)
        println!("Testing voice cloning...");
        let audio_samples = vec![
            AudioSample {
                data: create_dummy_audio_data(),
                transcript: Some("This is a sample transcript for voice cloning.".to_string()),
                quality_rating: Some(8),
            }
        ];

        match create_voice_clone(
            "test-clone-voice",
            &audio_samples,
            Some("A test cloned voice")
        ) {
            Ok(cloned_voice) => {
                results.push("✓ Voice cloning successful".to_string());
                
                // Test synthesis with cloned voice
                let text_input = TextInput {
                    content: "Testing synthesis with cloned voice.".to_string(),
                    text_type: TextType::Plain,
                    language: Some("en-US".to_string()),
                };

                match synthesize(&text_input, &cloned_voice, None) {
                    Ok(result) => {
                        results.push("✓ Synthesis with cloned voice successful".to_string());
                        save_audio_result(&result.audio_data, "test5-cloned", "mp3");
                    }
                    Err(e) => results.push(format!("✗ Synthesis with cloned voice failed: {:?}", e)),
                }

                // Clean up cloned voice
                match cloned_voice.delete() {
                    Ok(_) => results.push("✓ Cloned voice deleted successfully".to_string()),
                    Err(e) => results.push(format!("⚠ Cloned voice deletion failed: {:?}", e)),
                }
            }
            Err(TtsError::UnsupportedOperation(_)) => {
                results.push("⚠ Voice cloning not supported by provider".to_string());
            }
            Err(e) => results.push(format!("✗ Voice cloning failed: {:?}", e)),
        }

        // Test voice design
        println!("Testing voice design...");
        let design_params = VoiceDesignParams {
            gender: VoiceGender::Female,
            age_category: AgeCategory::YoungAdult,
            accent: "american".to_string(),
            personality_traits: vec!["friendly".to_string(), "calm".to_string()],
            reference_voice: None,
        };

        match design_voice("test-designed-voice", &design_params) {
            Ok(designed_voice) => {
                results.push("✓ Voice design successful".to_string());
                results.push(format!("✓ Designed voice ID: {}", designed_voice.get_id()));
                
                // Test with designed voice
                let text_input = TextInput {
                    content: "Testing synthesis with designed voice.".to_string(),
                    text_type: TextType::Plain,
                    language: Some("en-US".to_string()),
                };

                match synthesize(&text_input, &designed_voice, None) {
                    Ok(result) => {
                        results.push("✓ Synthesis with designed voice successful".to_string());
                        save_audio_result(&result.audio_data, "test5-designed", "mp3");
                    }
                    Err(e) => results.push(format!("✗ Synthesis with designed voice failed: {:?}", e)),
                }

                // Clean up
                let _ = designed_voice.delete();
            }
            Err(TtsError::UnsupportedOperation(_)) => {
                results.push("⚠ Voice design not supported by provider".to_string());
            }
            Err(e) => results.push(format!("✗ Voice design failed: {:?}", e)),
        }

        results.join("\n")
    }

    /// test6 demonstrates audio format validation and quality verification
    fn test6() -> String {
        println!("Test6: Audio format validation and quality verification");
        let mut results = Vec::new();

        let voice = match get_test_voice() {
            Ok(v) => v,
            Err(e) => return format!("Failed to get test voice: {}", e),
        };

        // Test different audio formats
        let formats = vec![
            (AudioFormat::Mp3, "mp3"),
            (AudioFormat::Wav, "wav"),
            (AudioFormat::OggOpus, "oggopus"),
            (AudioFormat::Aac, "aac"),
        ];

        let text_input = TextInput {
            content: MEDIUM_TEXT.to_string(),
            text_type: TextType::Plain,
            language: Some("en-US".to_string()),
        };

        for (format, extension) in formats {
            println!("Testing format: {:?}", format);
            let audio_config = AudioConfig {
                format: format.clone(),
                sample_rate: Some(22050),
                bit_rate: Some(128),
                channels: Some(1),
            };

            let options = SynthesisOptions {
                audio_config: Some(audio_config),
                voice_settings: None,
                audio_effects: None,
                enable_timing: None,
                enable_word_timing: None,
                seed: None,
                model_version: None,
                context: None,
            };

            match synthesize(&text_input, &voice, Some(&options)) {
                Ok(result) => {
                    results.push(format!("✓ {} format synthesis successful", extension.to_uppercase()));
                    results.push(format!("  Audio size: {} bytes", result.audio_data.len()));
                    results.push(format!("  Duration: {:.2}s", result.metadata.duration_seconds));
                    save_audio_result(&result.audio_data, &format!("test6-{}", extension), extension);
                }
                Err(e) => results.push(format!("✗ {} format failed: {:?}", extension.to_uppercase(), e)),
            }
        }

        // Test different sample rates
        let sample_rates = vec![8000, 16000, 22050, 44100];
        for rate in sample_rates {
            println!("Testing sample rate: {}Hz", rate);
            let audio_config = AudioConfig {
                format: AudioFormat::Wav,
                sample_rate: Some(rate),
                bit_rate: None,
                channels: Some(1),
            };

            let options = SynthesisOptions {
                audio_config: Some(audio_config),
                voice_settings: None,
                audio_effects: None,
                enable_timing: None,
                enable_word_timing: None,
                seed: None,
                model_version: None,
                context: None,
            };

            match synthesize(&text_input, &voice, Some(&options)) {
                Ok(result) => {
                    results.push(format!("✓ {}Hz sample rate successful", rate));
                    save_audio_result(&result.audio_data, &format!("test6-{}hz", rate), "wav");
                }
                Err(e) => results.push(format!("✗ {}Hz sample rate failed: {:?}", rate, e)),
            }
        }

        results.join("\n")
    }

    /// test7 demonstrates custom pronunciation and lexicon management
    fn test7() -> String {
        println!("Test7: Custom pronunciation and lexicon management");
        let mut results = Vec::new();

        // Test lexicon creation
        println!("Testing lexicon creation...");
        let pronunciation_entries = vec![
            PronunciationEntry {
                word: "Golem".to_string(),
                pronunciation: "GOH-lem".to_string(),
                part_of_speech: Some("noun".to_string()),
            },
            PronunciationEntry {
                word: "API".to_string(),
                pronunciation: "ay-pee-AY".to_string(),
                part_of_speech: Some("noun".to_string()),
            },
        ];

        results.push(TEST_PROVIDER.to_string());

        match create_lexicon(
            "testlexicon",
            &"en-US".to_string(),
            Some(&pronunciation_entries)
        ) {
            Ok(lexicon) => {
                results.push("✓ Lexicon creation successful".to_string());
                results.push(format!("✓ Lexicon name: {}", lexicon.get_name()));
                results.push(format!("✓ Lexicon language: {}", lexicon.get_language()));
                results.push(format!("✓ Entry count: {}", lexicon.get_entry_count()));

                // Test adding entries
                match lexicon.add_entry("synthesis", "SIN-thuh-sis") {
                    Ok(_) => results.push("✓ Entry addition successful".to_string()),
                    Err(e) => results.push(format!("✗ Entry addition failed: {:?}", e)),
                }

                // Test lexicon export
                match lexicon.export_content() {
                    Ok(content) => {
                        results.push("✓ Lexicon export successful".to_string());
                        results.push(format!("  Content length: {} characters", content.len()));
                    }
                    Err(e) => results.push(format!("✗ Lexicon export failed: {:?}", e)),
                }

                // Test removing entries
                match lexicon.remove_entry("API") {
                    Ok(_) => results.push("✓ Entry removal successful".to_string()),
                    Err(e) => results.push(format!("✗ Entry removal failed: {:?}", e)),
                }
            }
            Err(TtsError::UnsupportedOperation(_)) => {
                results.push("⚠ Lexicon management not supported by provider".to_string());
            }
            Err(e) => results.push(format!("✗ Lexicon creation failed: {:?}", e)),
        }

        // Test sound effect generation
        println!("Testing sound effect generation...");
        match generate_sound_effect(
            "Ocean waves gently lapping against the shore",
            Some(5.0),
            Some(0.7)
        ) {
            Ok(sound_effect) => {
                results.push("✓ Sound effect generation successful".to_string());
                results.push(format!("✓ Sound effect size: {} bytes", sound_effect.len()));
                save_audio_result(&sound_effect, "test7-sound-effect", "mp3");
            }
            Err(TtsError::UnsupportedOperation(_)) => {
                results.push("⚠ Sound effect generation not supported by provider".to_string());
            }
            Err(e) => results.push(format!("✗ Sound effect generation failed: {:?}", e)),
        }

        results.join("\n")
    }

    /// test8 demonstrates authentication and authorization scenarios
    fn test8() -> String {
        println!("Test8: Authentication and authorization scenarios");
        let mut results = Vec::new();

        // Test with potentially invalid credentials (simulated)
        println!("Testing authentication scenarios...");
        
        // Most authentication errors will be caught during voice discovery
        match list_voices(None) {
            Ok(_) => results.push("✓ Authentication successful".to_string()),
            Err(TtsError::Unauthorized(msg)) => {
                results.push(format!("✗ Unauthorized access: {}", msg));
            }
            Err(TtsError::AccessDenied(msg)) => {
                results.push(format!("✗ Access denied: {}", msg));
            }
            Err(e) => results.push(format!("⚠ Other authentication error: {:?}", e)),
        }

        // Test quota information retrieval through error handling
        println!("Testing quota scenarios...");
        let voice = match get_test_voice() {
            Ok(v) => v,
            Err(e) => return format!("Failed to get test voice: {}", e),
        };

        // Attempting synthesis that might hit quotas
        let large_text = LONG_TEXT.repeat(10); // Very long text
        let text_input = TextInput {
            content: large_text,
            text_type: TextType::Plain,
            language: Some("en-US".to_string()),
        };

        match synthesize(&text_input, &voice, None) {
            Ok(result) => {
                results.push("✓ Large text synthesis successful".to_string());
                results.push(format!("  Characters: {}", result.metadata.character_count));
                save_audio_result(&result.audio_data, "test8-large", "mp3");
            }
            Err(TtsError::QuotaExceeded(quota_info)) => {
                results.push(format!("⚠ Quota exceeded: used={}/{} {}", 
                    quota_info.used, quota_info.limit, format!("{:?}", quota_info.unit)));
                results.push(format!("  Reset time: {}", quota_info.reset_time));
            }
            Err(TtsError::RateLimited(retry_after)) => {
                results.push(format!("⚠ Rate limited, retry after {} seconds", retry_after));
            }
            Err(TtsError::InsufficientCredits) => {
                results.push("⚠ Insufficient credits".to_string());
            }
            Err(e) => results.push(format!("✗ Large text synthesis failed: {:?}", e)),
        }

        results.join("\n")
    }

    /// test9 demonstrates error handling for malformed inputs and edge cases
    fn test9() -> String {
        println!("Test9: Error handling for malformed inputs and edge cases");
        let mut results = Vec::new();

        let voice = match get_test_voice() {
            Ok(v) => v,
            Err(e) => return format!("Failed to get test voice: {}", e),
        };

        // Test with empty text
        println!("Testing empty text handling...");
        let empty_input = TextInput {
            content: "".to_string(),
            text_type: TextType::Plain,
            language: Some("en-US".to_string()),
        };

        match synthesize(&empty_input, &voice, None) {
            Ok(_) => results.push("✓ Empty text handled gracefully".to_string()),
            Err(TtsError::InvalidText(msg)) => {
                results.push(format!("✓ Empty text properly rejected: {}", msg));
            }
            Err(e) => results.push(format!("⚠ Unexpected empty text error: {:?}", e)),
        }

        // Test with malformed SSML
        println!("Testing malformed SSML handling...");
        let bad_ssml = TextInput {
            content: "<speak><unclosed>Bad SSML</speak>".to_string(),
            text_type: TextType::Ssml,
            language: Some("en-US".to_string()),
        };

        match synthesize(&bad_ssml, &voice, None) {
            Ok(_) => results.push("⚠ Malformed SSML was accepted".to_string()),
            Err(TtsError::InvalidSsml(msg)) => {
                results.push(format!("✓ Malformed SSML properly rejected: {}", msg));
            }
            Err(e) => results.push(format!("⚠ Unexpected SSML error: {:?}", e)),
        }

        // Test with unsupported language
        println!("Testing unsupported language handling...");
        let unsupported_lang = TextInput {
            content: "Test text".to_string(),
            text_type: TextType::Plain,
            language: Some("xx-XX".to_string()), // Invalid language code
        };

        match synthesize(&unsupported_lang, &voice, None) {
            Ok(_) => results.push("⚠ Unsupported language was accepted".to_string()),
            Err(TtsError::UnsupportedLanguage(msg)) => {
                results.push(format!("✓ Unsupported language properly rejected: {}", msg));
            }
            Err(e) => results.push(format!("⚠ Unexpected language error: {:?}", e)),
        }

        // Test with invalid voice settings
        println!("Testing invalid voice settings...");
        let invalid_settings = VoiceSettings {
            speed: Some(10.0), // Way too fast
            pitch: Some(100.0), // Way too high
            volume: Some(200.0), // Way too loud
            stability: Some(2.0), // Out of range
            similarity: Some(-1.0), // Out of range
            style: Some(5.0), // Out of range
        };

        let options = SynthesisOptions {
            audio_config: None,
            voice_settings: Some(invalid_settings),
            audio_effects: None,
            enable_timing: None,
            enable_word_timing: None,
            seed: None,
            model_version: None,
            context: None,
        };

        let test_input = TextInput {
            content: "Test with invalid settings".to_string(),
            text_type: TextType::Plain,
            language: Some("en-US".to_string()),
        };

        match synthesize(&test_input, &voice, Some(&options)) {
            Ok(_) => results.push("⚠ Invalid voice settings were accepted (may be clamped)".to_string()),
            Err(TtsError::InvalidConfiguration(msg)) => {
                results.push(format!("✓ Invalid settings properly rejected: {}", msg));
            }
            Err(e) => results.push(format!("⚠ Unexpected settings error: {:?}", e)),
        }

        println!("Testing non-existent voice handling...");
        match get_voice("non-existent-voice-id-12345") {
            Ok(_) => results.push("⚠ Non-existent voice was found".to_string()),
            Err(TtsError::VoiceNotFound(msg)) => {
                results.push(format!("✓ Non-existent voice properly rejected: {}", msg));
            }
            Err(e) => results.push(format!("⚠ Unexpected voice error: {:?}", e)),
        }

        results.join("\n")
    }

    /// test10 demonstrates long-form content synthesis (>5000 characters)
    fn test10() -> String {
        println!("Test10: Long-form content synthesis");
        let mut results = Vec::new();

        // Creating very long content (>5000 characters)
        let long_content = LONG_TEXT.repeat(25);
        results.push(format!("Testing with {} characters", long_content.len()));

        let voice = match get_test_voice() {
            Ok(v) => v,
            Err(e) => return format!("Failed to get test voice: {}", e),
        };

        // Test regular synthesis with long content
        println!("Testing regular synthesis with long content...");
        let text_input = TextInput {
            content: long_content.clone(),
            text_type: TextType::Plain,
            language: Some("en-US".to_string()),
        };

        match synthesize(&text_input, &voice, None) {
            Ok(result) => {
                results.push("✓ Long-form synthesis successful".to_string());
                results.push(format!("✓ Audio duration: {:.2}s", result.metadata.duration_seconds));
                results.push(format!("✓ Characters processed: {}", result.metadata.character_count));
                save_audio_result(&result.audio_data, "test10-long", "mp3");
            }
            Err(TtsError::TextTooLong(max_length)) => {
                results.push(format!("⚠ Text too long, max allowed: {} characters", max_length));
            }
            Err(e) => results.push(format!("✗ Long-form synthesis failed: {:?}", e)),
        }

        // Test specialized long-form synthesis
        println!("Testing specialized long-form synthesis...");
        let chapter_breaks = Some(vec![1000, 2000, 3000, 4000]); 
        
        match synthesize_long_form(
            &long_content,
            &voice,
            "/output/test10-long-form.mp3",
            chapter_breaks.as_deref()
        ) {
            Ok(operation) => {
                results.push("✓ Long-form operation started".to_string());
                
                // Poll for completion
                let mut attempts = 0;
                let max_attempts = 30;
                
                while attempts < max_attempts {
                    match operation.get_status() {
                        OperationStatus::Pending => {
                            results.push("⏳ Long-form operation pending...".to_string());
                        }
                        OperationStatus::Processing => {
                            let progress = operation.get_progress();
                            results.push(format!("⏳ Long-form processing: {:.1}%", progress * 100.0));
                        }
                        OperationStatus::Completed => {
                            match operation.get_result() {
                                Ok(result) => {
                                    results.push("✓ Long-form synthesis completed".to_string());
                                    results.push(format!("✓ Output location: {}", result.output_location));
                                    results.push(format!("✓ Total duration: {:.2}s", result.total_duration));
                                    if let Some(chapters) = result.chapter_durations {
                                        results.push(format!("✓ Chapters: {}", chapters.len()));
                                    }
                                    break;
                                }
                                Err(e) => {
                                    results.push(format!("✗ Long-form result error: {:?}", e));
                                    break;
                                }
                            }
                        }
                        OperationStatus::Failed => {
                            results.push("✗ Long-form operation failed".to_string());
                            break;
                        }
                        OperationStatus::Cancelled => {
                            results.push("⚠ Long-form operation cancelled".to_string());
                            break;
                        }
                    }
                    
                    thread::sleep(Duration::from_secs(2));
                    attempts += 1;
                }
                
                if attempts >= max_attempts {
                    results.push("⚠ Long-form operation timeout".to_string());
                    let _ = operation.cancel();
                }
            }
            Err(TtsError::UnsupportedOperation(_)) => {
                results.push("⚠ Specialized long-form synthesis not supported".to_string());
            }
            Err(e) => results.push(format!("✗ Long-form operation failed: {:?}", e)),
        }

        results.join("\n")
    }

    /// test11 demonstrates durability semantics across operation boundaries
    fn test11() -> String {
        println!("Test11: Durability semantics verification");
        let mut results = Vec::new();

        let voice = match get_test_voice() {
            Ok(v) => v,
            Err(e) => return format!("Failed to get test voice: {}", e),
        };

        // Test durability with streaming synthesis
        println!("Testing durability with streaming synthesis...");
        
        let worker_name = std::env::var("GOLEM_WORKER_NAME").unwrap_or_else(|_| "test-worker".to_string());
        let mut round = 0;

        let stream_options = SynthesisOptions {
            audio_config: Some(AudioConfig {
                format: AudioFormat::Wav,
                sample_rate: Some(24000),
                bit_rate: None,
                channels: Some(1),
            }),
            voice_settings: None,
            audio_effects: None,
            enable_timing: Some(true),
            enable_word_timing: None,
            seed: None,
            model_version: None,
            context: None,
        };

        match create_stream(&voice, Some(&stream_options)) {
            Ok(stream) => {
                results.push("✓ Streaming session created for durability test".to_string());
                
                // Send initial text
                let text_input = TextInput {
                    content: "This is a durability test for TTS streaming.".to_string(),
                    text_type: TextType::Plain,
                    language: Some("en-US".to_string()),
                };

                match stream.send_text(&text_input) {
                    Ok(_) => results.push("✓ Initial text sent".to_string()),
                    Err(e) => {
                        results.push(format!("✗ Failed to send initial text: {:?}", e));
                        return results.join("\n");
                    }
                }

                // Simulate crash after first round (durability test)
                atomically(|| {
                    let client = TestHelperApi::new(&worker_name);
                    let counter = client.blocking_inc_and_get();
                    if counter == 1 {
                        panic!("Simulating crash during durability test");
                    }
                });

                // Continue after recovery
                round += 1;
                results.push(format!("✓ Continued after recovery (round {})", round));

                // Send more text to verify stream state persistence
                let text_input2 = TextInput {
                    content: " This text is sent after recovery.".to_string(),
                    text_type: TextType::Plain,
                    language: Some("en-US".to_string()),
                };

                match stream.send_text(&text_input2) {
                    Ok(_) => results.push("✓ Text sent after recovery successful".to_string()),
                    Err(e) => results.push(format!("⚠ Text after recovery failed: {:?}", e)),
                }

                // Finish stream
                match stream.finish() {
                    Ok(_) => results.push("✓ Stream finished after recovery".to_string()),
                    Err(e) => results.push(format!("⚠ Stream finish after recovery failed: {:?}", e)),
                }

                let mut audio_data = Vec::new();
                let mut attempts = 0;
                while attempts < 20 && (stream.has_pending_audio() || 
                                       !matches!(stream.get_status(), StreamStatus::Finished)) {
                    match stream.receive_chunk() {
                        Ok(Some(chunk)) => {
                            audio_data.extend_from_slice(&chunk.data);
                            if chunk.is_final { break; }
                        }
                        Ok(None) => thread::sleep(Duration::from_millis(100)),
                        Err(e) => {
                            results.push(format!("⚠ Chunk reception after recovery failed: {:?}", e));
                            break;
                        }
                    }
                    attempts += 1;
                }

                if !audio_data.is_empty() {
                    results.push(format!("✓ Audio collected after recovery: {} bytes", audio_data.len()));
                    save_audio_result(&audio_data, "test11-durability", "wav");
                } else {
                    results.push("⚠ No audio collected after recovery".to_string());
                }

                stream.close();
            }
            Err(e) => results.push(format!("✗ Durability test stream creation failed: {:?}", e)),
        }

        results.join("\n")
    }

    /// test12 demonstrates provider-specific features and comprehensive integration
    fn test12() -> String {
        println!("Test12: Provider-specific features and comprehensive integration");
        let mut results = Vec::new();

        let voice = match get_test_voice() {
            Ok(v) => v,
            Err(e) => return format!("Failed to get test voice: {}", e),
        };

        // Test voice capabilities
        println!("Testing voice capabilities...");
        results.push(format!("Voice ID: {}", voice.get_id()));
        results.push(format!("Voice name: {}", voice.get_name()));
        results.push(format!("Language: {}", voice.get_language()));
        results.push(format!("Gender: {:?}", voice.get_gender()));
        results.push(format!("Quality: {:?}", voice.get_quality()));
        results.push(format!("SSML support: {}", voice.supports_ssml()));
        
        let sample_rates = voice.get_sample_rates();
        results.push(format!("Sample rates: {:?}", sample_rates));
        
        let formats = voice.get_supported_formats();
        results.push(format!("Supported formats: {:?}", formats));

        // Test voice preview
        println!("Testing voice preview...");
        match voice.preview("This is a voice preview sample.") {
            Ok(preview_audio) => {
                results.push("✓ Voice preview successful".to_string());
                results.push(format!("✓ Preview audio size: {} bytes", preview_audio.len()));
                save_audio_result(&preview_audio, "test12-preview", "mp3");
            }
            Err(e) => results.push(format!("✗ Voice preview failed: {:?}", e)),
        }

        // Test voice cloning
        println!("Testing voice cloning (if supported)...");
        match voice.clone() {
            Ok(cloned) => {
                results.push("✓ Voice cloning successful".to_string());
                results.push(format!("✓ Cloned voice ID: {}", cloned.get_id()));
                let _ = cloned.delete(); // Clean up
            }
            Err(TtsError::UnsupportedOperation(_)) => {
                results.push("⚠ Voice cloning not supported".to_string());
            }
            Err(e) => results.push(format!("✗ Voice cloning failed: {:?}", e)),
        }

        // Test voice-to-voice conversion
        println!("Testing voice-to-voice conversion...");
        let input_audio = create_dummy_audio_data();
        match convert_voice(&input_audio, &voice, Some(true)) {
            Ok(converted_audio) => {
                results.push("✓ Voice conversion successful".to_string());
                results.push(format!("✓ Converted audio size: {} bytes", converted_audio.len()));
                save_audio_result(&converted_audio, "test12-converted", "mp3");
            }
            Err(TtsError::UnsupportedOperation(_)) => {
                results.push("⚠ Voice conversion not supported".to_string());
            }
            Err(e) => results.push(format!("✗ Voice conversion failed: {:?}", e)),
        }

        println!("Testing voice conversion streaming...");
        match create_voice_conversion_stream(&voice, None) {
            Ok(conversion_stream) => {
                results.push("✓ Voice conversion stream created".to_string());
                
                let audio_chunks = vec![
                    create_dummy_audio_data(),
                    create_dummy_audio_data(),
                ];

                for (i, chunk) in audio_chunks.iter().enumerate() {
                    match conversion_stream.send_audio(&chunk.clone()) {
                        Ok(_) => println!("Sent audio chunk {}", i + 1),
                        Err(e) => {
                            results.push(format!("✗ Failed to send audio chunk {}: {:?}", i + 1, e));
                            break;
                        }
                    }
                }

                match conversion_stream.finish() {
                    Ok(_) => results.push("✓ Conversion stream finished".to_string()),
                    Err(e) => results.push(format!("✗ Conversion stream finish failed: {:?}", e)),
                }

                let mut converted_data = Vec::new();
                let mut attempts = 0;
                while attempts < 10 {
                    match conversion_stream.receive_converted() {
                        Ok(Some(chunk)) => {
                            converted_data.extend_from_slice(&chunk.data);
                            if chunk.is_final { break; }
                        }
                        Ok(None) => thread::sleep(Duration::from_millis(100)),
                        Err(e) => {
                            results.push(format!("⚠ Conversion chunk reception failed: {:?}", e));
                            break;
                        }
                    }
                    attempts += 1;
                }

                if !converted_data.is_empty() {
                    results.push(format!("✓ Conversion stream audio: {} bytes", converted_data.len()));
                    save_audio_result(&converted_data, "test12-stream-converted", "mp3");
                }

                conversion_stream.close();
            }
            Err(TtsError::UnsupportedOperation(_)) => {
                results.push("⚠ Voice conversion streaming not supported".to_string());
            }
            Err(e) => results.push(format!("✗ Voice conversion stream failed: {:?}", e)),
        }

        println!("Testing comprehensive synthesis...");
        let comprehensive_text = TextInput {
            content: format!("{}\n\n{}", SSML_TEXT, MEDIUM_TEXT),
            text_type: TextType::Ssml,
            language: Some("en-US".to_string()),
        };

        let comprehensive_options = SynthesisOptions {
            audio_config: Some(AudioConfig {
                format: AudioFormat::Wav,
                sample_rate: Some(22050),
                bit_rate: Some(128),
                channels: Some(1),
            }),
            voice_settings: Some(VoiceSettings {
                speed: Some(1.1),
                pitch: Some(1.0),
                volume: Some(0.0),
                stability: Some(0.8),
                similarity: Some(0.9),
                style: Some(0.6),
            }),
            audio_effects: Some(vec![
                AudioEffects::NoiseReduction,
                AudioEffects::HeadphoneOptimized,
            ]),
            enable_timing: Some(true),
            enable_word_timing: Some(true),
            seed: Some(42),
            model_version: None,
            context: Some(SynthesisContext {
                previous_text: Some("Previous context for better synthesis.".to_string()),
                next_text: Some("Next context for continuity.".to_string()),
                topic: Some("Technology and AI".to_string()),
                emotion: Some("friendly".to_string()),
                speaking_style: Some("conversational".to_string()),
            }),
        };

        match synthesize(&comprehensive_text, &voice, Some(&comprehensive_options)) {
            Ok(result) => {
                results.push("✓ Comprehensive synthesis successful".to_string());
                results.push(format!("✓ Duration: {:.2}s", result.metadata.duration_seconds));
                results.push(format!("✓ Words: {}", result.metadata.word_count));
                results.push(format!("✓ Characters: {}", result.metadata.character_count));
                results.push(format!("✓ Audio size: {} bytes", result.metadata.audio_size_bytes));
                save_audio_result(&result.audio_data, "test12-comprehensive", "wav");
            }
            Err(e) => results.push(format!("✗ Comprehensive synthesis failed: {:?}", e)),
        }

        results.join("\n")
    }
}

// Helper functions

fn get_test_voice() -> Result<Voice, String> {
    match list_voices(None) {
        Ok(voice_results) => {
            if voice_results.has_more() {
                match voice_results.get_next() {
                    Ok(voices) => {
                        if let Some(voice_info) = voices.first() {
                            match get_voice(&voice_info.id.clone()) {
                                Ok(voice) => Ok(voice),
                                Err(e) => Err(format!("Failed to get voice {}: {:?}", voice_info.id, e)),
                            }
                        } else {
                            Err("No voices available".to_string())
                        }
                    }
                    Err(e) => Err(format!("Failed to get voice list: {:?}", e)),
                }
            } else {
                Err("No voices available".to_string())
            }
        }
        Err(e) => Err(format!("Failed to list voices: {:?}", e)),
    }
}

fn save_audio_result(audio_data: &[u8], test_name: &str, extension: &str) {
    if let Err(_) = fs::create_dir_all("/output") {
        println!("Failed to create output directory");
        return;
    }

    let filename = format!("/output/audio-{}.{}", test_name, extension);
    match fs::write(&filename, audio_data) {
        Ok(_) => println!("Audio saved to: {}", filename),
        Err(e) => println!("Failed to save audio to {}: {}", filename, e),
    }
}

fn create_dummy_audio_data() -> Vec<u8> {
    vec![0u8; 1024]
}

bindings::export!(Component with_types_in bindings);
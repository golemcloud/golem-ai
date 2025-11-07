#[allow(static_mut_refs)]
mod bindings;

use std::fs;
use std::path::Path;

use crate::bindings::exports::test::tts_exports::test_tts_api::*;
use crate::bindings::golem::tts::voices::list_voices;
use crate::bindings::test::helper_client::test_helper_client::TestHelperApi;

use crate::bindings::golem::tts::advanced::{
    convert_voice, design_voice, generate_sound_effect, AgeCategory, OperationStatus,
    VoiceDesignParams,
};
use crate::bindings::golem::tts::synthesis::{SynthesisResult, VoiceSettings};
use crate::bindings::golem::tts::{
    advanced::{
        create_lexicon, create_voice_clone, synthesize_long_form, AudioSample, PronunciationEntry,
    },
    synthesis::{get_timing_marks, synthesize, synthesize_batch, validate_input, SynthesisOptions},
    types::{AudioConfig, TextInput, TextType, VoiceGender},
    voices::{get_voice, VoiceFilter},
};

use golem_rust::atomically;
use log::trace;

use std::thread;
use std::time::Duration;

struct Component;

#[cfg(feature = "deepgram")]
const VOICE_UUID: &'static str = "87f1a83a-8064-465c-ae3d-4e5ab800d4ed"; // UUID for get_voice, canonical name stored in voice.name

#[cfg(feature = "deepgram")]
const MODEL: &str = "aura-2";

#[cfg(feature = "deepgram")]
const TARGET_VOICE: &'static str = "aura-2-amalthea-en";

#[cfg(feature = "elevenlabs")]
const VOICE_UUID: &'static str = "CwhRBWXzGAHq8TQ4Fs17"; // Roger

#[cfg(feature = "elevenlabs")]
const MODEL: &str = "eleven_v2_flash";

#[cfg(feature = "elevenlabs")]
const TARGET_VOICE: &'static str = "JBFqnCBsd6RMkjVDRZzb"; // George

#[cfg(feature = "polly")]
const VOICE_UUID: &'static str = "Danielle";

#[cfg(feature = "polly")]
const MODEL: &str = "generative";

#[cfg(feature = "polly")]
const TARGET_VOICE: &'static str = "Joanna";

#[cfg(feature = "google")]
const VOICE_UUID: &'static str = "en-US-Wavenet-A";

#[cfg(feature = "google")]
const MODEL: &str = "standard";

#[cfg(feature = "google")]
const TARGET_VOICE: &'static str = "en-US-Wavenet-B";

const TEXT:&str = "In a quiet coastal village, mornings begin with the scent of salt in the air and the rhythm of waves meeting the shore.
Fishermen set out at dawn, their small boats cutting across the glassy water, while children race barefoot along the beach, chasing the tide. 
The old lighthouse, weathered but steadfast, watches over them all, its lantern dim in the morning light, waiting for night to reclaim its glow.";

// SSML using only tags supported by ALL providers (Polly, Google TTS, ElevenLabs)
// Note: Only <speak> and <break> are universally supported
// <p> and <s> tags are NOT supported by ElevenLabs, so we use line breaks instead
const SSML: &str = r#"
    <speak>
        In a quiet coastal village, mornings begin with the scent of salt in the air
        and the rhythm of waves meeting the shore. <break time="400ms"/>
        Fishermen set out at dawn, their boats tracing silver lines across calm water,
        while children race barefoot along the beach, chasing the tide.
        <break time="500ms"/>
        The old lighthouse - weathered yet steadfast - watches over them,
        its lantern dim in the early light, waiting for night to reclaim its glow.
        <break time="300ms"/>
        It is a place where routines are gentle,
        and time seems to breathe.
    </speak>
"#;

impl Guest for Component {
    // Test Getting a specific voice
    fn test1() -> String {
        trace!("Getting a specific voice.");
        match get_voice(VOICE_UUID) {
            Ok(voice) => {
                trace!("Recived voice: {:?}", voice);
                format!("✅ Voice ID: {VOICE_UUID} Voice: {:?}", voice)
            }
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        }
    }

    fn test2() -> String {
        trace!("Listing all voices.");
        let filter = VoiceFilter {
            language: Some("en-US".to_string()),
            gender: Some(VoiceGender::Male),
            quality: None,
            supports_ssml: Some(true),
            provider: None,
            search_query: None,
        };
        match list_voices(Some(&filter)) {
            Ok(voices) => {
                trace!("Recived voice: {:?}", voices);
                format!("✅ Voices: {:?}", voices)
            }
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        }
    }

    // Test Synthesizing text
    fn test3() -> String {
        let voice = match get_voice(VOICE_UUID) {
            Ok(voices) => voices,
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        };
        let mut test_result = String::new();
        test_result.push_str("Test speech synthesis summary:  ");

        trace!("Sending text without options.");
        let test_name = "1. Test speech synthesis";

        let text_input = TextInput {
            content: TEXT.to_string(),
            text_type: TextType::Plain,
            language: Some("en-US".to_string()),
        };

        let options = SynthesisOptions {
            audio_config: Some(AudioConfig {
                format: "mp3".to_string(),
                sample_rate: Some(22050),
                bit_rate: None,
                channels: Some(1),
            }),
            voice_settings: Some(VoiceSettings {
                speed: Some(1.2),
                pitch: Some(2.0),
                volume: Some(0.0),
                stability: Some(0.8),
                similarity: Some(0.9),
                style: Some(0.5),
            }),
            audio_effects: None,
            enable_timing: None,
            enable_word_timing: None,
            seed: None,
            model_id: Some(MODEL.to_string()),
            context: None,
        };

        match synthesize(&text_input, &voice, Some(&options)) {
            Ok(result) => {
                trace!("Recived result: {:?}", result.metadata);
                let dir = "/test-audio-files";
                let name = "test3-without-options.mp3";
                let storage_msg = save_audio(&result.audio_data, dir, name);
                push_result(&mut test_result, result, test_name, format!("{dir}/{name}"));
                test_result.push_str(&format!("\n{}\n", storage_msg));
            }
            Err(err) => {
                test_result.push_str(&format!("{test_name} ❌  "));
                test_result.push_str(&format!("ERROR : {:?}  ", err));
            }
        };

        test_result
    }

    // Test Synthesizing text with SSML
    fn test4() -> String {
        let voice = match get_voice(VOICE_UUID) {
            Ok(voices) => voices,
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        };
        let mut test_result = String::new();
        test_result.push_str("Test speech synthesis using SSML summary:  ");

        trace!("Sending SSML without options.");
        let test_name = "1. Test speech synthesis using SSML ";

        let text_input = TextInput {
            content: SSML.to_string(),
            text_type: TextType::Ssml,
            language: Some("en-US".to_string()),
        };

        let options = SynthesisOptions {
            audio_config: Some(AudioConfig {
                format: "mp3".to_string(),
                sample_rate: Some(22050),
                bit_rate: None,
                channels: Some(1),
            }),
            voice_settings: Some(VoiceSettings {
                speed: Some(1.2),
                pitch: Some(2.0),
                volume: Some(0.0),
                stability: Some(0.8),
                similarity: Some(0.9),
                style: Some(0.5),
            }),
            audio_effects: None,
            enable_timing: None,
            enable_word_timing: None,
            seed: None,
            model_id: Some(MODEL.to_string()),
            context: None,
        };

        match synthesize(&text_input, &voice, Some(&options)) {
            Ok(result) => {
                trace!("Recived result: {:?}", result.metadata);
                let dir = "/test-audio-files";
                let name = "test4-ssml.mp3";
                let storage_msg = save_audio(&result.audio_data, dir, name);
                push_result(&mut test_result, result, test_name, format!("{dir}/{name}"));
                test_result.push_str(&format!("\n{}\n", storage_msg));
            }
            Err(err) => {
                test_result.push_str(&format!("{test_name} ❌  "));
                test_result.push_str(&format!("ERROR : {:?}  ", err));
            }
        };

        test_result
    }

    // Test Batch synthesis for multiple inputs
    fn test5() -> String {
        let voice = match get_voice(VOICE_UUID) {
            Ok(voices) => voices,
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        };
        let mut test_result = String::new();
        test_result.push_str("Test batch speech synthesis summary:  ");

        trace!("Sending batch text inputs.");
        let test_name = "1. Test batch speech synthesis";

        let batch_inputs = vec![
            TextInput {
                content: "I am first sentence of the batch inputs.".to_string(),
                text_type: TextType::Plain,
                language: Some("en-US".to_string()),
            },
            TextInput {
                content: "I am seconds sentence of the batch inputs.".to_string(),
                text_type: TextType::Plain,
                language: Some("en-US".to_string()),
            },
        ];

        let options = SynthesisOptions {
            audio_config: Some(AudioConfig {
                format: "mp3".to_string(),
                sample_rate: Some(22050),
                bit_rate: None,
                channels: Some(1),
            }),
            voice_settings: Some(VoiceSettings {
                speed: Some(1.2),
                pitch: Some(2.0),
                volume: Some(0.0),
                stability: Some(0.8),
                similarity: Some(0.9),
                style: Some(0.5),
            }),
            audio_effects: None,
            enable_timing: None,
            enable_word_timing: None,
            seed: None,
            model_id: Some(MODEL.to_string()),
            context: None,
        };

        match synthesize_batch(&batch_inputs, &voice, Some(&options)) {
            Ok(batch) => {
                test_result.push_str(&format!("{test_name} ✅  "));
                let mut index = 1;
                for result in batch {
                    trace!("#{index} Recived result: {:?}  ", result.metadata);
                    let dir = "/test-audio-files";
                    let name = format!("test5-batch-{}.mp3", index);
                    let storage_msg = save_audio(&result.audio_data, dir, &name);
                    test_result.push_str(&format!("Batch Item #{index}:  "));
                    push_result(&mut test_result, result, test_name, format!("{dir}/{name}"));
                    test_result.push_str(&format!("\n{}\n", storage_msg));
                    index += 1;
                }
            }
            Err(err) => {
                test_result.push_str(&format!("{test_name} ❌  "));
                test_result.push_str(&format!("ERROR : {:?}  ", err));
            }
        };

        test_result
    }

    // Test Validate text before synthesis & Get timing information without audio synthesis
    fn test6() -> String {
        let voice = match get_voice(VOICE_UUID) {
            Ok(voices) => voices,
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        };
        let ssml_input = TextInput {
            content: SSML.to_string(),
            text_type: TextType::Ssml,
            language: Some("en-US".to_string()),
        };

        let mut test_result = String::new();
        test_result.push_str("Test input validation & timing marks summary:  ");

        trace!("Testing input validation...");
        let test_name = "1. Test input validation";
        match validate_input(&ssml_input.clone(), &voice) {
            Ok(validation) => {
                trace!("Validation result: {:?}  ", validation);
                test_result.push_str(&format!("{test_name} ✅  "));
                test_result.push_str(&format!("Is Valid: {:?}  ", validation.is_valid));
                test_result.push_str(&format!(
                    "Character Count: {:?}  ",
                    validation.character_count
                ));
                test_result.push_str(&format!(
                    "Estimated Duration: {:?}  ",
                    validation.estimated_duration
                ));
                test_result.push_str(&format!("Errors: {:?}  ", validation.errors));
                test_result.push_str(&format!("Warnings: {:?}  ", validation.warnings));
            }
            Err(err) => {
                test_result.push_str(&format!("{test_name} ❌  "));
                test_result.push_str(&format!("ERROR : {:?}  ", err))
            }
        }

        trace!("Testing timing marks.");
        let test_name = "2. Test timing marks";
        match get_timing_marks(&ssml_input, &voice) {
            Ok(timing_marks) => {
                test_result.push_str(&format!("{test_name} ✅  "));
                let index = 1;
                for mark in timing_marks {
                    trace!("Timing mark #{} : {:?}  ", index, mark);
                    test_result.push_str(&format!("Timing mark #{index}"));
                    test_result.push_str(&format!("{:?}  ", mark));
                }
            }
            Err(err) => {
                test_result.push_str(&format!("{test_name} ❌  "));
                test_result.push_str(&format!("ERROR : {:?}  ", err))
            }
        };

        test_result
    }

    // Test advanced voice operation
    // 1. Create voice clone
    // 2. Design voice
    // 3. Voice to voice
    // 4. Generate sound effects
    fn test7() -> String {
        let voice = match get_voice(VOICE_UUID) {
            Ok(voices) => voices,
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        };

        let mut test_result = String::new();
        test_result.push_str("Test advanced voice operations summary:  ");

        let text_input = TextInput {
            content: TEXT.to_string(),
            text_type: TextType::Plain,
            language: Some("en-US".to_string()),
        };

        let sample_audio_data = match synthesize(&text_input, &voice, None) {
            Ok(result) => result.audio_data,
            Err(err) => {
                return format!("❌ ERROR generating sample audio: {:?}", err);
            }
        };

        // Test 1: Create voice clone
        trace!("Testing voice cloning...");
        let test_name = "1. Test voice clone";

        let audio_samples = vec![AudioSample {
            data: sample_audio_data.clone(),
            transcript: Some(TEXT.to_string()),
            quality_rating: Some(8),
        }];

        match create_voice_clone(
            "test-voice-clone",
            &audio_samples,
            Some("Test clone description"),
        ) {
            Ok(cloned_voice) => {
                let id = cloned_voice.id;
                trace!("Voice cloned successfully: {id}");
                test_result.push_str(&format!("{test_name} ✅  "));
                test_result.push_str(&format!("Cloned Voice ID: {id}  "));
                test_result.push_str(&format!("Cloned Voice Name: {}  ", cloned_voice.name));
            }
            Err(err) => {
                trace!("Failed to clone voice.");
                test_result.push_str(&format!("{test_name} ❌  "));
                test_result.push_str(&format!("ERROR : {:?}  ", err));
            }
        }

        // Test 2: Design voice
        trace!("Testing voice design...");
        let test2_name = "2. Test voice design";
        let design_params = VoiceDesignParams {
            gender: VoiceGender::Female,
            age_category: AgeCategory::YoungAdult,
            accent: "american".to_string(),
            personality_traits: vec!["friendly".to_string(), "energetic".to_string()],
            reference_voice: Some(VOICE_UUID.to_string()),
        };

        match design_voice("test-designed-voice", &design_params) {
            Ok(designed_voice) => {
                trace!(
                    "Voice designed successfully: {:?}",
                    designed_voice.id.clone()
                );
                test_result.push_str(&format!("{test2_name} ✅  "));
                test_result.push_str(&format!(
                    "Designed Voice ID: {}  ",
                    designed_voice.id.clone()
                ));
                test_result.push_str(&format!(
                    "Designed Voice Name: {}  ",
                    designed_voice.name.clone()
                ));
            }
            Err(err) => {
                trace!("Failed to design voice.");
                test_result.push_str(&format!("{test2_name} ❌  "));
                test_result.push_str(&format!("ERROR : {:?}  ", err));
            }
        }

        // Test 3: Voice-to-voice conversion
        trace!("Testing voice-to-voice conversion...");
        let test3_name = "3. Test voice-to-voice conversion";

        let target_voice = match get_voice(TARGET_VOICE) {
            Ok(voices) => voices,
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        };

        match convert_voice(&sample_audio_data, &target_voice, Some(true)) {
            Ok(converted_audio) => {
                trace!(
                    "Voice conversion successful, audio size: {}",
                    converted_audio.len()
                );
                test_result.push_str(&format!("{test3_name} ✅  "));
                test_result.push_str(&format!(
                    "Converted audio size: {} bytes  ",
                    converted_audio.len()
                ));

                let dir = "/test-audio-files";
                let name = "test7-voice-conversion.mp3";
                let storage_msg = save_audio(&converted_audio, dir, name);
                test_result.push_str(&format!("💾 Audio saved at {dir}/{name}  "));
                test_result.push_str(&format!("\n{}\n", storage_msg));
            }
            Err(err) => {
                trace!("Failed to convert voice.");
                test_result.push_str(&format!("{test3_name} ❌  "));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
            }
        }

        // Test 4: Generate sound effect
        trace!("Testing sound effect generation...");
        let test4_name = "4. Test sound effect generation";
        let description = "Ocean waves crashing on a beach";

        match generate_sound_effect(description, Some(10.0), Some(0.8)) {
            Ok(sound_effect) => {
                trace!(
                    "Sound effect generated successfully, audio size: {}",
                    sound_effect.len()
                );
                test_result.push_str(&format!("{test4_name} ✅\n"));
                test_result.push_str(&format!(
                    "Sound effect size: {} bytes\n",
                    sound_effect.len()
                ));

                let dir = "/test-audio-files";
                let name = "test7-sound-effect.mp3";
                let storage_msg = save_audio(&sound_effect, dir, name);
                test_result.push_str(&format!("💾 Audio saved at {dir}/{name}\n"));
                test_result.push_str(&format!("{}\n", storage_msg));
            }
            Err(err) => {
                trace!("Failed to generate sound effect.");
                test_result.push_str(&format!("{test4_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
            }
        }

        test_result
    }

    // Text long for synthesis
    fn test8() -> String {
        let voice = match get_voice(VOICE_UUID) {
            Ok(voices) => voices,
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        };

        let mut test_result = String::new();
        test_result.push_str("Test long-form synthesis summary:\n");

        trace!("Testing long-form synthesis...");
        let test_name = "Test long-form synthesis";

        let long_content = format!("{}\n\n{}\n\n{}", TEXT, TEXT, TEXT);

        let chapter_breaks = [0, TEXT.len() as u32, (TEXT.len() * 2) as u32];

        match synthesize_long_form(&long_content, &voice, Some(&chapter_breaks)) {
            Ok(operation) => {
                // Monitor the operation progress
                let mut attempts = 0;
                let max_attempts = 30;

                while attempts < max_attempts {
                    if attempts == 3 {
                        // Simulate crash
                        let agent_name = std::env::var("GOLEM_WORKER_NAME").unwrap();
                        mimic_crash(&agent_name);
                    }

                    let status = operation.get_status();
                    let progress: f32 = operation.get_progress();
                    trace!(
                        "Operation status: {:?}, progress: {:.2}%",
                        status,
                        progress * 100.0
                    );

                    match status {
                        OperationStatus::Completed => {
                            trace!("Long-form synthesis completed successfully");
                            test_result.push_str(&format!("{test_name} ✅\n"));

                            trace!("Getting long-form synthesis result...");
                            match operation.get_result() {
                                Ok(result) => {
                                    trace!("Recieved Long-form synthesis result");
                                    test_result.push_str("1. Test get operation result ✅\n");
                                    test_result.push_str(&format!(
                                        "Output location: {}\n",
                                        result.output_location
                                    ));
                                    test_result.push_str(&format!(
                                        "Total duration: {:.2}s\n",
                                        result.total_duration
                                    ));
                                    test_result.push_str(&format!(
                                        "Chapter durations: {:?}\n",
                                        result.chapter_durations
                                    ));
                                    test_result.push_str(&format!(
                                        "Request ID: {}\n",
                                        result.metadata.request_id
                                    ));
                                    test_result.push_str(&format!(
                                        "💾 Audio saved at {}\n",
                                        result.output_location
                                    ));
                                }
                                Err(err) => {
                                    trace!("Failed to get long-form synthesis result");
                                    test_result.push_str("1. Test get operation result ❌\n");
                                    test_result
                                        .push_str(&format!("ERROR getting result: {:?}\n", err));
                                }
                            }
                            break;
                        }
                        OperationStatus::Failed => {
                            trace!("Failed to synthesize long-form content. operation failed !");
                            test_result.push_str(&format!("{test_name} ❌\n"));
                            test_result.push_str("operation failed !");
                            match operation.get_result() {
                                Ok(result) => {
                                    if !result.output_location.is_empty() {
                                        trace!("This should not return result if operation failed: {:?}\n",result);
                                        test_result.push_str("This should not return result if operation failed ❌\n");
                                        test_result.push_str(&format!(
                                            "Output location: {}\n",
                                            result.output_location
                                        ));
                                        test_result.push_str(&format!(
                                            "Total duration: {:.2}s\n",
                                            result.total_duration
                                        ));
                                        test_result.push_str(&format!(
                                            "Chapter durations: {:?}\n",
                                            result.chapter_durations
                                        ));
                                        test_result.push_str(&format!(
                                            "Request ID: {}\n",
                                            result.metadata.request_id
                                        ));
                                        test_result.push_str(&format!(
                                            "💾 Audio saved at {}\n",
                                            result.output_location
                                        ));
                                    }
                                }
                                Err(err) => {
                                    test_result.push_str(&format!("ERROR: {:?}\n", err));
                                }
                            };
                            break;
                        }
                        OperationStatus::Cancelled => {
                            trace!("Failed to synthesize long-form content. operation cancelled !");
                            test_result.push_str(&format!("{test_name} ❌\n"));
                            test_result.push_str("operation cancelled !");
                            match operation.get_result() {
                                Ok(result) => {
                                    if !result.output_location.is_empty() {
                                        trace!("Operation returned result even if operation cancelled: {:?}\n",result);
                                        test_result.push_str("Operation returned result even if operation cancelled ⚠️\n");
                                        test_result.push_str(&format!(
                                            "Output location: {}\n",
                                            result.output_location
                                        ));
                                        test_result.push_str(&format!(
                                            "Total duration: {:.2}s\n",
                                            result.total_duration
                                        ));
                                        test_result.push_str(&format!(
                                            "Chapter durations: {:?}\n",
                                            result.chapter_durations
                                        ));
                                        test_result.push_str(&format!(
                                            "Request ID: {}\n",
                                            result.metadata.request_id
                                        ));
                                        test_result.push_str(&format!(
                                            "💾 Audio saved at {}\n",
                                            result.output_location
                                        ));
                                    }
                                }
                                Err(err) => {
                                    test_result.push_str(&format!("ERROR: {:?}\n", err));
                                }
                            };

                            break;
                        }
                        _ => {
                            // Still processing
                            thread::sleep(Duration::from_millis(500));
                        }
                    }

                    attempts += 1;
                }

                if attempts >= max_attempts {
                    test_result.push_str("2. Test long-form synthesis completion ❌\n");
                    test_result.push_str("ERROR: Operation timed out\n");
                }
            }
            Err(err) => {
                test_result.push_str(&format!("{test_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
                return test_result;
            }
        }

        test_result
    }

    // Test pronunciation lexicons
    fn test9() -> String {
        let mut test_result = String::new();
        test_result.push_str("Test pronunciation lexicons summary:\n");

        trace!("Creating pronunciation lexicon...");
        let test_name = "1. Test create lexicon";

        let pronunciation_entries = [
            PronunciationEntry {
                word: "Golem".to_string(),
                pronunciation: "GOH-lem".to_string(),
                part_of_speech: Some("noun".to_string()),
            },
            PronunciationEntry {
                word: "synthesis".to_string(),
                pronunciation: "SIN-thuh-sis".to_string(),
                part_of_speech: Some("noun".to_string()),
            },
        ];

        let lexicon = match create_lexicon("testlexicon", "en-US", Some(&pronunciation_entries)) {
            Ok(lexicon) => {
                trace!("Lexicon created successfully: {}", lexicon.get_name());
                test_result.push_str(&format!("{test_name} ✅\n"));
                test_result.push_str(&format!("Lexicon name: {}\n", lexicon.get_name()));
                test_result.push_str(&format!("Lexicon language: {}\n", lexicon.get_language()));
                test_result.push_str(&format!("Entry count: {}\n", lexicon.get_entry_count()));
                lexicon
            }
            Err(err) => {
                trace!("Failed to create lexicon.");
                test_result.push_str(&format!("{test_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
                return test_result;
            }
        };

        trace!("Adding entry to lexicon...");
        let test2_name = "2. Test add lexicon entry";

        match lexicon.add_entry("coastal", "KOHS-tuhl") {
            Ok(_) => {
                trace!("Entry added successfully");
                test_result.push_str(&format!("{test2_name} ✅\n"));
                test_result.push_str(&format!(
                    "Updated entry count: {}\n",
                    lexicon.get_entry_count()
                ));
            }
            Err(err) => {
                trace!("Failed to add entry to lexicon.");
                test_result.push_str(&format!("{test2_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
            }
        }

        trace!("Exporting lexicon content...");
        let test3_name = "3. Test export lexicon";

        // Simulate crash
        let agent_name = std::env::var("GOLEM_WORKER_NAME").unwrap();
        mimic_crash(&agent_name);

        match lexicon.export_content() {
            Ok(content) => {
                trace!(
                    "Lexicon exported successfully, content length: {}",
                    content.len()
                );
                test_result.push_str(&format!("{test3_name} ✅\n"));
                test_result.push_str(&format!(
                    "Exported content length: {} characters\n",
                    content.len()
                ));
                if content.len() < 500 {
                    test_result.push_str(&format!("Content preview: {}\n", content));
                }
            }
            Err(err) => {
                trace!("Failed to export lexicon.");
                test_result.push_str(&format!("{test3_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
            }
        }

        trace!("Removing entry from lexicon...");
        let test4_name = "4. Test remove lexicon entry";

        match lexicon.remove_entry("coastal") {
            Ok(_) => {
                trace!("Entry removed successfully");
                test_result.push_str(&format!("{test4_name} ✅\n"));
                test_result.push_str(&format!(
                    "Final entry count: {}\n",
                    lexicon.get_entry_count()
                ));
            }
            Err(err) => {
                trace!("Failed to remove entry from lexicon.");
                test_result.push_str(&format!("{test4_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
            }
        }

        test_result
    }
}

fn save_audio(audio_data: &[u8], dir: &str, name: &str) -> String {
    // Create directory if it doesn't exist
    if let Err(e) = fs::create_dir_all(dir) {
        let msg = format!("❌ Failed to create directory {}: {}", dir, e);
        trace!("{}", msg);
        return msg;
    }

    // Save the file
    let path = Path::new(dir).join(name);
    match fs::write(&path, audio_data) {
        Ok(_) => {
            // Try to get file size for verification
            let size_info = match fs::metadata(&path) {
                Ok(metadata) => format!("Size: {} bytes", metadata.len()),
                Err(_) => format!("Size: {} bytes (from data)", audio_data.len()),
            };

            let msg = format!(
                "✅ Audio saved to worker filesystem: {:?}, {}",
                path, size_info
            );
            trace!("{}", msg);
            msg
        }
        Err(e) => {
            let msg = format!("❌ Failed to save audio {:?}: {}", path, e);
            trace!("{}", msg);
            msg
        }
    }
}

fn push_result(
    test_result: &mut String,
    result: SynthesisResult,
    test_name: &str,
    audio_file_location: String,
) {
    test_result.push_str(&format!("{test_name} ✅ \n"));
    test_result.push_str(&format!("Request ID: {:?}\n", result.metadata.request_id));
    test_result.push_str(&format!(
        "Audio Size: {:?}\n",
        result.metadata.audio_size_bytes
    ));
    test_result.push_str(&format!(
        "Character Count: {:?}\n",
        result.metadata.character_count
    ));
    test_result.push_str(&format!(
        "Duration in seconds: {:?}\n",
        result.metadata.duration_seconds
    ));
    test_result.push_str(&format!(
        "Provider Info: {:?}\n",
        result.metadata.provider_info
    ));
    test_result.push_str(&format!("Word Count: {:?}\n", result.metadata.word_count));
    test_result.push_str(&format!("💾 Audio saved at {audio_file_location}"));
}

fn mimic_crash(agent_name: &str) {
    atomically(|| {
        let client = TestHelperApi::new(&agent_name);
        let answer = client.blocking_inc_and_get();
        if answer == 1 {
            panic!("Simulating crash during durability test")
        }
    });
}

bindings::export!(Component with_types_in bindings);

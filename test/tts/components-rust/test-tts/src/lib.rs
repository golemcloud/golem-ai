#[allow(static_mut_refs)]
mod bindings;

use crate::bindings::exports::test::tts_exports::test_tts_api::*;
use crate::bindings::golem::tts::synthesis::{synthesize, SynthesisOptions};
use crate::bindings::golem::tts::types::{AudioFormat, TextInput, TextType, VoiceSettings};
use crate::bindings::golem::tts::voices::{get_voice, list_voices, VoiceFilter};

struct Component;

impl Guest for Component {
    fn test_list_voices() -> Result<String, String> {
        let voices = list_voices(&None).map_err(|err| format!("error: {err:?}"))?;
        Ok(format!("{voices:?}"))
    }

    fn test_synthesize() -> Result<String, String> {
        let voices = list_voices(&None).map_err(|err| format!("error: {err:?}"))?;
        let first_voice = voices
            .first()
            .ok_or_else(|| "no voices returned".to_string())?;
        let voice = get_voice(&first_voice.id).map_err(|err| format!("error: {err:?}"))?;
        let input = TextInput {
            content: "Hello from Golem TTS".to_string(),
            text_type: TextType::Plain,
            language: None,
        };
        let options = SynthesisOptions {
            audio_config: Some(crate::bindings::golem::tts::types::AudioConfig {
                format: AudioFormat::Mp3,
                sample_rate: None,
                bit_rate: None,
                channels: None,
            }),
            voice_settings: Some(VoiceSettings {
                speed: None,
                pitch: None,
                volume: None,
                stability: None,
                similarity: None,
                style: None,
            }),
            // `audio_effects` is currently unsupported by the TTS providers and
            // is reserved for future use; it is explicitly set to `None` here.
            audio_effects: None,
            enable_timing: None,
            enable_word_timing: None,
            seed: None,
            model_version: None,
            context: None,
        };
        let result = synthesize(&input, &first_voice.id, &Some(options))
            .map_err(|err| format!("error: {err:?}"))?;
        Ok(format!("audio_bytes={}", result.audio_data.len()))
    }
}

bindings::export!(Component with_types_in bindings);

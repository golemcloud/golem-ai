//! TTS-Deepgram WASM Component
//!
//! Full implementation of Deepgram Aura TTS provider for Golem Cloud.
//! Adheres to best practices (no unwrap, explicit error mapping, durability).

#[allow(warnings)]
mod bindings;
mod client;
mod synthesis;
mod types;
mod voices;

use voices::{VoiceImpl, VoiceResultsImpl};

struct TtsDeepgramComponent;

// ============================================================
// VOICE INTERFACE
// ============================================================

impl bindings::exports::golem::tts::voices::Guest for TtsDeepgramComponent {
    type Voice = VoiceImpl;
    type VoiceResults = VoiceResultsImpl;

    fn list_voices(
        filter: Option<bindings::exports::golem::tts::voices::VoiceFilter>,
    ) -> Result<
        bindings::exports::golem::tts::voices::VoiceResults,
        bindings::exports::golem::tts::types::TtsError,
    > {
        voices::list_voices(filter)
    }

    fn get_voice(
        voice_id: String,
    ) -> Result<
        bindings::exports::golem::tts::voices::Voice,
        bindings::exports::golem::tts::types::TtsError,
    > {
        voices::get_voice(voice_id)
    }

    fn search_voices(
        query: String,
        filter: Option<bindings::exports::golem::tts::voices::VoiceFilter>,
    ) -> Result<
        Vec<bindings::exports::golem::tts::voices::VoiceInfo>,
        bindings::exports::golem::tts::types::TtsError,
    > {
        let results = voices::list_voices(filter)?;
        let voices_list = results.get::<VoiceResultsImpl>().voices.clone();

        let query = query.to_lowercase();
        Ok(voices_list
            .into_iter()
            .filter(|v| {
                v.name.to_lowercase().contains(&query) || v.id.to_lowercase().contains(&query)
            })
            .collect())
    }

    fn list_languages() -> Result<
        Vec<bindings::exports::golem::tts::voices::LanguageInfo>,
        bindings::exports::golem::tts::types::TtsError,
    > {
        let mut languages = std::collections::HashMap::new();
        for (id, lang, _) in types::DEEPGRAM_VOICES {
            let entry = languages
                .entry(lang.to_string())
                .or_insert((0, lang.to_string()));
            entry.0 += 1;
        }

        Ok(languages
            .into_iter()
            .map(
                |(code, (count, name))| bindings::exports::golem::tts::voices::LanguageInfo {
                    code,
                    name: name.clone(),
                    native_name: name,
                    voice_count: count,
                },
            )
            .collect())
    }
}

// ============================================================
// SYNTHESIS INTERFACE
// ============================================================

impl bindings::exports::golem::tts::synthesis::Guest for TtsDeepgramComponent {
    fn synthesize(
        input: bindings::exports::golem::tts::types::TextInput,
        voice: bindings::exports::golem::tts::voices::VoiceBorrow<'_>,
        options: Option<bindings::exports::golem::tts::synthesis::SynthesisOptions>,
    ) -> Result<
        bindings::exports::golem::tts::types::SynthesisResult,
        bindings::exports::golem::tts::types::TtsError,
    > {
        synthesis::synthesize(input, voice.get::<VoiceImpl>(), options)
    }

    fn synthesize_batch(
        inputs: Vec<bindings::exports::golem::tts::types::TextInput>,
        voice: bindings::exports::golem::tts::voices::VoiceBorrow<'_>,
        options: Option<bindings::exports::golem::tts::synthesis::SynthesisOptions>,
    ) -> Result<
        Vec<bindings::exports::golem::tts::types::SynthesisResult>,
        bindings::exports::golem::tts::types::TtsError,
    > {
        synthesis::synthesize_batch(inputs, voice.get::<VoiceImpl>(), options)
    }

    fn get_timing_marks(
        input: bindings::exports::golem::tts::types::TextInput,
        voice: bindings::exports::golem::tts::voices::VoiceBorrow<'_>,
    ) -> Result<
        Vec<bindings::exports::golem::tts::types::TimingInfo>,
        bindings::exports::golem::tts::types::TtsError,
    > {
        synthesis::get_timing_marks(input, voice.get::<VoiceImpl>())
    }

    fn validate_input(
        input: bindings::exports::golem::tts::types::TextInput,
        voice: bindings::exports::golem::tts::voices::VoiceBorrow<'_>,
    ) -> Result<
        bindings::exports::golem::tts::synthesis::ValidationResult,
        bindings::exports::golem::tts::types::TtsError,
    > {
        synthesis::validate_input(input, voice.get::<VoiceImpl>())
    }
}

// ============================================================
// STREAMING INTERFACE (Placeholders)
// ============================================================

impl bindings::exports::golem::tts::streaming::Guest for TtsDeepgramComponent {
    type SynthesisStream = StreamPlaceholder;
    type VoiceConversionStream = ConvStreamPlaceholder;

    fn create_stream(
        _: bindings::exports::golem::tts::voices::VoiceBorrow<'_>,
        _: Option<bindings::exports::golem::tts::synthesis::SynthesisOptions>,
    ) -> Result<
        bindings::exports::golem::tts::streaming::SynthesisStream,
        bindings::exports::golem::tts::types::TtsError,
    > {
        Err(
            bindings::exports::golem::tts::types::TtsError::UnsupportedOperation(
                "Streaming not yet implemented for Deepgram".to_string(),
            ),
        )
    }

    fn create_voice_conversion_stream(
        _: bindings::exports::golem::tts::voices::VoiceBorrow<'_>,
        _: Option<bindings::exports::golem::tts::synthesis::SynthesisOptions>,
    ) -> Result<
        bindings::exports::golem::tts::streaming::VoiceConversionStream,
        bindings::exports::golem::tts::types::TtsError,
    > {
        Err(
            bindings::exports::golem::tts::types::TtsError::UnsupportedOperation(
                "Voice conversion not supported by Deepgram".to_string(),
            ),
        )
    }
}

pub struct StreamPlaceholder;
impl bindings::exports::golem::tts::streaming::GuestSynthesisStream for StreamPlaceholder {
    fn send_text(
        &self,
        _: bindings::exports::golem::tts::types::TextInput,
    ) -> Result<(), bindings::exports::golem::tts::types::TtsError> {
        Ok(())
    }
    fn finish(&self) -> Result<(), bindings::exports::golem::tts::types::TtsError> {
        Ok(())
    }
    fn receive_chunk(
        &self,
    ) -> Result<
        Option<bindings::exports::golem::tts::types::AudioChunk>,
        bindings::exports::golem::tts::types::TtsError,
    > {
        Ok(None)
    }
    fn has_pending_audio(&self) -> bool {
        false
    }
    fn get_status(&self) -> bindings::exports::golem::tts::streaming::StreamStatus {
        bindings::exports::golem::tts::streaming::StreamStatus::Closed
    }
    fn close(&self) {}
}

pub struct ConvStreamPlaceholder;
impl bindings::exports::golem::tts::streaming::GuestVoiceConversionStream
    for ConvStreamPlaceholder
{
    fn send_audio(&self, _: Vec<u8>) -> Result<(), bindings::exports::golem::tts::types::TtsError> {
        Ok(())
    }
    fn receive_converted(
        &self,
    ) -> Result<
        Option<bindings::exports::golem::tts::types::AudioChunk>,
        bindings::exports::golem::tts::types::TtsError,
    > {
        Ok(None)
    }
    fn finish(&self) -> Result<(), bindings::exports::golem::tts::types::TtsError> {
        Ok(())
    }
    fn close(&self) {}
}

// ============================================================
// ADVANCED INTERFACE (Placeholders)
// ============================================================

impl bindings::exports::golem::tts::advanced::Guest for TtsDeepgramComponent {
    type PronunciationLexicon = LexPlaceholder;
    type LongFormOperation = LongPlaceholder;

    fn create_voice_clone(
        _: String,
        _: Vec<bindings::exports::golem::tts::advanced::AudioSample>,
        _: Option<String>,
    ) -> Result<
        bindings::exports::golem::tts::voices::Voice,
        bindings::exports::golem::tts::types::TtsError,
    > {
        Err(
            bindings::exports::golem::tts::types::TtsError::UnsupportedOperation(
                "Voice cloning not supported by Deepgram".to_string(),
            ),
        )
    }

    fn design_voice(
        _: String,
        _: bindings::exports::golem::tts::advanced::VoiceDesignParams,
    ) -> Result<
        bindings::exports::golem::tts::voices::Voice,
        bindings::exports::golem::tts::types::TtsError,
    > {
        Err(
            bindings::exports::golem::tts::types::TtsError::UnsupportedOperation(
                "Voice design not supported by Deepgram".to_string(),
            ),
        )
    }

    fn convert_voice(
        _: Vec<u8>,
        _: bindings::exports::golem::tts::voices::VoiceBorrow<'_>,
        _: Option<bool>,
    ) -> Result<Vec<u8>, bindings::exports::golem::tts::types::TtsError> {
        Err(
            bindings::exports::golem::tts::types::TtsError::UnsupportedOperation(
                "Voice conversion not supported by Deepgram".to_string(),
            ),
        )
    }

    fn generate_sound_effect(
        _: String,
        _: Option<f32>,
        _: Option<f32>,
    ) -> Result<Vec<u8>, bindings::exports::golem::tts::types::TtsError> {
        Err(
            bindings::exports::golem::tts::types::TtsError::UnsupportedOperation(
                "Sound effects not supported by Deepgram".to_string(),
            ),
        )
    }

    fn create_lexicon(
        _: String,
        _: String,
        _: Option<Vec<bindings::exports::golem::tts::advanced::PronunciationEntry>>,
    ) -> Result<
        bindings::exports::golem::tts::advanced::PronunciationLexicon,
        bindings::exports::golem::tts::types::TtsError,
    > {
        Err(
            bindings::exports::golem::tts::types::TtsError::UnsupportedOperation(
                "Lexicon management not supported via this interface".to_string(),
            ),
        )
    }

    fn synthesize_long_form(
        _: String,
        _: bindings::exports::golem::tts::voices::VoiceBorrow<'_>,
        _: String,
        _: Option<Vec<u32>>,
    ) -> Result<
        bindings::exports::golem::tts::advanced::LongFormOperation,
        bindings::exports::golem::tts::types::TtsError,
    > {
        Err(
            bindings::exports::golem::tts::types::TtsError::UnsupportedOperation(
                "Long-form synthesis not supported via this interface".to_string(),
            ),
        )
    }
}

pub struct LexPlaceholder;
impl bindings::exports::golem::tts::advanced::GuestPronunciationLexicon for LexPlaceholder {
    fn get_name(&self) -> String {
        String::new()
    }
    fn get_language(&self) -> String {
        String::new()
    }
    fn get_entry_count(&self) -> u32 {
        0
    }
    fn add_entry(
        &self,
        _: String,
        _: String,
    ) -> Result<(), bindings::exports::golem::tts::types::TtsError> {
        Ok(())
    }
    fn remove_entry(
        &self,
        _: String,
    ) -> Result<(), bindings::exports::golem::tts::types::TtsError> {
        Ok(())
    }
    fn export_content(&self) -> Result<String, bindings::exports::golem::tts::types::TtsError> {
        Ok(String::new())
    }
}

pub struct LongPlaceholder;
impl bindings::exports::golem::tts::advanced::GuestLongFormOperation for LongPlaceholder {
    fn get_status(&self) -> bindings::exports::golem::tts::advanced::OperationStatus {
        bindings::exports::golem::tts::advanced::OperationStatus::Pending
    }
    fn get_progress(&self) -> f32 {
        0.0
    }
    fn cancel(&self) -> Result<(), bindings::exports::golem::tts::types::TtsError> {
        Ok(())
    }
    fn get_result(
        &self,
    ) -> Result<
        bindings::exports::golem::tts::advanced::LongFormResult,
        bindings::exports::golem::tts::types::TtsError,
    > {
        Err(
            bindings::exports::golem::tts::types::TtsError::InternalError(
                "Not implemented".to_string(),
            ),
        )
    }
}

bindings::export!(TtsDeepgramComponent with_types_in bindings);

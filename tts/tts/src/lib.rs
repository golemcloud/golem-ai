pub mod config;
pub mod durability;
pub mod error;
pub mod model;

use crate::model::advanced::{
    AudioSample, LongFormOperation, LongFormResult, OperationStatus, PronunciationEntry,
    PronunciationLexicon, VoiceDesignParams,
};
use crate::model::streaming::{
    AudioChunk, StreamStatus, SynthesisOptions, SynthesisStream, TextInput, VoiceBorrow,
    VoiceConversionStream,
};
use crate::model::synthesis::{SynthesisResult, TimingInfo, ValidationResult};
use crate::model::voices::{
    AudioFormat, LanguageCode, LanguageInfo, TtsError, Voice, VoiceFilter, VoiceGender, VoiceInfo,
    VoiceQuality, VoiceResults, VoiceSettings,
};
use std::cell::RefCell;
use std::str::FromStr;

pub trait VoiceResultsInterface: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn has_more(&self) -> bool;
    fn get_next(&self) -> Result<Vec<VoiceInfo>, TtsError>;
    fn get_total_count(&self) -> Option<u32>;
}

pub trait VoiceProvider {
    type Voice: VoiceInterface;
    type VoiceResults: VoiceResultsInterface;

    fn list_voices(filter: Option<VoiceFilter>) -> Result<VoiceResults, TtsError>;
    fn get_voice(voice_id: String) -> Result<Voice, TtsError>;
    fn search_voices(filter: Option<VoiceFilter>) -> Result<Vec<VoiceInfo>, TtsError>;
    fn list_languages() -> Result<Vec<LanguageInfo>, TtsError>;
}

pub trait SynthesisStreamInterface: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn send_text(&self, input: TextInput) -> Result<(), model::streaming::TtsError>;
    fn finish(&self) -> Result<(), model::streaming::TtsError>;
    fn receive_chunk(&self) -> Result<Option<AudioChunk>, model::streaming::TtsError>;
    fn has_pending_audio(&self) -> bool;
    fn get_status(&self) -> StreamStatus;
    fn close(&self);
}

pub trait VoiceInterface: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn get_id(&self) -> String;
    fn get_name(&self) -> String;
    fn get_provider_id(&self) -> Option<String>;
    fn get_language(&self) -> LanguageCode;
    fn get_additional_languages(&self) -> Vec<LanguageCode>;
    fn get_gender(&self) -> VoiceGender;
    fn get_quality(&self) -> VoiceQuality;
    fn get_description(&self) -> Option<String>;
    fn supports_ssml(&self) -> bool;
    fn get_sample_rates(&self) -> Vec<u32>;
    fn get_supported_formats(&self) -> Vec<AudioFormat>;
    fn update_settings(&self, settings: VoiceSettings) -> Result<(), TtsError>;
    fn delete(&self) -> Result<(), TtsError>;
    fn clone(&self) -> Result<Voice, TtsError>;
    fn preview(&self, text: String) -> Result<Vec<u8>, TtsError>;
}

pub trait VoiceConversionStreamInterface: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn send_audio(&self, audio_data: Vec<u8>) -> Result<(), model::streaming::TtsError>;
    fn receive_converted(&self) -> Result<Option<AudioChunk>, model::streaming::TtsError>;
    fn finish(&self) -> Result<(), model::streaming::TtsError>;
    fn close(&self);
}

pub trait StreamingVoiceProvider {
    type SynthesisStream: SynthesisStreamInterface;
    type VoiceConversionStream: VoiceConversionStreamInterface;

    fn create_stream(
        voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisStream, model::streaming::TtsError>;

    fn create_voice_conversion_stream(
        target_voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<VoiceConversionStream, model::streaming::TtsError>;
}

pub trait SynthesizeProvider {
    fn synthesize(
        input: model::synthesis::TextInput,
        voice: model::synthesis::VoiceBorrow<'_>,
        options: Option<model::synthesis::SynthesisOptions>,
    ) -> Result<SynthesisResult, model::synthesis::TtsError>;

    fn synthesize_batch(
        inputs: Vec<model::synthesis::TextInput>,
        voice: model::synthesis::VoiceBorrow<'_>,
        options: Option<model::synthesis::SynthesisOptions>,
    ) -> Result<Vec<SynthesisResult>, model::synthesis::TtsError>;

    fn get_timing_marks(
        input: model::synthesis::TextInput,
        voice: model::synthesis::VoiceBorrow<'_>,
    ) -> Result<Vec<TimingInfo>, model::synthesis::TtsError>;

    fn validate_input(
        input: model::synthesis::TextInput,
        voice: model::synthesis::VoiceBorrow<'_>,
    ) -> Result<ValidationResult, model::synthesis::TtsError>;
}

pub trait PronunciationLexiconInterface: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn get_name(&self) -> String;
    fn get_language(&self) -> model::advanced::LanguageCode;
    fn get_entry_count(&self) -> u32;
    fn add_entry(
        &self,
        word: String,
        pronunciation: String,
    ) -> Result<(), model::advanced::TtsError>;
    fn remove_entry(&self, word: String) -> Result<(), model::advanced::TtsError>;
    fn export_content(&self) -> Result<String, model::advanced::TtsError>;
}

pub trait LongFormOperationInterface: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn get_status(&self) -> OperationStatus;
    fn get_progress(&self) -> f32;
    fn cancel(&self) -> Result<(), model::advanced::TtsError>;
    fn get_result(&self) -> Result<LongFormResult, model::advanced::TtsError>;
}

pub trait AdvancedTtsProvider {
    type PronunciationLexicon: PronunciationLexiconInterface;
    type LongFormOperation: LongFormOperationInterface;

    fn create_voice_clone(
        name: String,
        audio_samples: Vec<AudioSample>,
        description: Option<String>,
    ) -> Result<model::advanced::Voice, model::advanced::TtsError>;

    fn design_voice(
        name: String,
        characteristics: VoiceDesignParams,
    ) -> Result<model::advanced::Voice, model::advanced::TtsError>;

    fn convert_voice(
        input_audio: Vec<u8>,
        target_voice: model::advanced::VoiceBorrow<'_>,
        preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, model::advanced::TtsError>;

    fn generate_sound_effect(
        description: String,
        duration_seconds: Option<f32>,
        style_influence: Option<f32>,
    ) -> Result<Vec<u8>, model::advanced::TtsError>;

    fn create_lexicon(
        name: String,
        language: model::advanced::LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<PronunciationLexicon, model::advanced::TtsError>;

    fn synthesize_long_form(
        content: String,
        voice: model::advanced::VoiceBorrow<'_>,
        output_location: String,
        chapter_breaks: Option<Vec<u32>>,
    ) -> Result<LongFormOperation, model::advanced::TtsError>;
}

struct LoggingState {
    logging_initialized: bool,
}

impl LoggingState {
    fn init(&mut self) {
        if !self.logging_initialized {
            let _ = wasi_logger::Logger::install();
            let max_level: log::LevelFilter = log::LevelFilter::from_str(
                &std::env::var("TTS_PROVIDER_LOG_LEVEL").unwrap_or_default(),
            )
            .unwrap_or(log::LevelFilter::Info);
            log::set_max_level(max_level);
            self.logging_initialized = true;
        }
    }
}

thread_local! {
    static LOGGING_STATE: RefCell<LoggingState> = const { RefCell::new(LoggingState {
        logging_initialized: false,
    }) };
}

pub fn init_logging() {
    LOGGING_STATE.with_borrow_mut(|state| state.init());
}

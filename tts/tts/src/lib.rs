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
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;

pub type TtsFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, TtsError>> + 'a>>;

pub trait VoiceResultsInterface: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn has_more(&self) -> bool;
    fn get_next(&self) -> TtsFuture<'_, Vec<VoiceInfo>>;
    fn get_total_count(&self) -> Option<u32>;
}

#[allow(async_fn_in_trait)]
pub trait VoiceProvider {
    type Voice: VoiceInterface;
    type VoiceResults: VoiceResultsInterface;

    /// Provider-specific configuration (API keys, regions, etc.) that the
    /// caller resolves once and passes in. Each provider crate defines its
    /// own concrete config type.
    type ProviderConfig: Clone + 'static;

    async fn list_voices(
        provider_config: Self::ProviderConfig,
        filter: Option<VoiceFilter>,
    ) -> Result<VoiceResults, TtsError>;
    async fn get_voice(
        provider_config: Self::ProviderConfig,
        voice_id: String,
    ) -> Result<Voice, TtsError>;
    async fn search_voices(
        provider_config: Self::ProviderConfig,
        filter: Option<VoiceFilter>,
    ) -> Result<Vec<VoiceInfo>, TtsError>;
    async fn list_languages(
        provider_config: Self::ProviderConfig,
    ) -> Result<Vec<LanguageInfo>, TtsError>;
}

pub trait SynthesisStreamInterface: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn send_text(&self, input: TextInput) -> TtsFuture<'_, ()>;
    fn finish(&self) -> TtsFuture<'_, ()>;
    fn receive_chunk(&self) -> TtsFuture<'_, Option<AudioChunk>>;
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
    fn update_settings(&self, settings: VoiceSettings) -> TtsFuture<'_, ()>;
    fn delete(&self) -> TtsFuture<'_, ()>;
    fn clone(&self) -> Result<Voice, TtsError>;
    fn preview(&self, text: String) -> TtsFuture<'_, Vec<u8>>;
}

pub trait VoiceConversionStreamInterface: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn send_audio(&self, audio_data: Vec<u8>) -> TtsFuture<'_, ()>;
    fn receive_converted(&self) -> TtsFuture<'_, Option<AudioChunk>>;
    fn finish(&self) -> TtsFuture<'_, ()>;
    fn close(&self);
}

#[allow(async_fn_in_trait)]
pub trait StreamingVoiceProvider {
    type SynthesisStream: SynthesisStreamInterface;
    type VoiceConversionStream: VoiceConversionStreamInterface;

    /// Provider-specific configuration; see [`VoiceProvider::ProviderConfig`].
    type ProviderConfig: Clone + 'static;

    async fn create_stream(
        provider_config: Self::ProviderConfig,
        voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisStream, model::streaming::TtsError>;

    async fn create_voice_conversion_stream(
        provider_config: Self::ProviderConfig,
        target_voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<VoiceConversionStream, model::streaming::TtsError>;
}

#[allow(async_fn_in_trait)]
pub trait SynthesizeProvider {
    /// Provider-specific configuration; see [`VoiceProvider::ProviderConfig`].
    type ProviderConfig: Clone + 'static;

    async fn synthesize(
        provider_config: Self::ProviderConfig,
        input: model::synthesis::TextInput,
        voice: model::synthesis::VoiceBorrow<'_>,
        options: Option<model::synthesis::SynthesisOptions>,
    ) -> Result<SynthesisResult, model::synthesis::TtsError>;

    async fn synthesize_batch(
        provider_config: Self::ProviderConfig,
        inputs: Vec<model::synthesis::TextInput>,
        voice: model::synthesis::VoiceBorrow<'_>,
        options: Option<model::synthesis::SynthesisOptions>,
    ) -> Result<Vec<SynthesisResult>, model::synthesis::TtsError>;

    async fn get_timing_marks(
        provider_config: Self::ProviderConfig,
        input: model::synthesis::TextInput,
        voice: model::synthesis::VoiceBorrow<'_>,
    ) -> Result<Vec<TimingInfo>, model::synthesis::TtsError>;

    async fn validate_input(
        provider_config: Self::ProviderConfig,
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
    fn add_entry(&self, word: String, pronunciation: String) -> TtsFuture<'_, ()>;
    fn remove_entry(&self, word: String) -> TtsFuture<'_, ()>;
    fn export_content(&self) -> TtsFuture<'_, String>;
}

pub trait LongFormOperationInterface: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn get_status(&self) -> TtsFuture<'_, OperationStatus>;
    fn get_progress(&self) -> TtsFuture<'_, f32>;
    fn cancel(&self) -> TtsFuture<'_, ()>;
    fn get_result(&self) -> TtsFuture<'_, LongFormResult>;
}

#[allow(async_fn_in_trait)]
pub trait AdvancedTtsProvider {
    type PronunciationLexicon: PronunciationLexiconInterface;
    type LongFormOperation: LongFormOperationInterface;

    /// Provider-specific configuration; see [`VoiceProvider::ProviderConfig`].
    type ProviderConfig: Clone + 'static;

    async fn create_voice_clone(
        provider_config: Self::ProviderConfig,
        name: String,
        audio_samples: Vec<AudioSample>,
        description: Option<String>,
    ) -> Result<model::advanced::Voice, model::advanced::TtsError>;

    async fn design_voice(
        provider_config: Self::ProviderConfig,
        name: String,
        characteristics: VoiceDesignParams,
    ) -> Result<model::advanced::Voice, model::advanced::TtsError>;

    async fn convert_voice(
        provider_config: Self::ProviderConfig,
        input_audio: Vec<u8>,
        target_voice: model::advanced::VoiceBorrow<'_>,
        preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, model::advanced::TtsError>;

    async fn generate_sound_effect(
        provider_config: Self::ProviderConfig,
        description: String,
        duration_seconds: Option<f32>,
        style_influence: Option<f32>,
    ) -> Result<Vec<u8>, model::advanced::TtsError>;

    async fn create_lexicon(
        provider_config: Self::ProviderConfig,
        name: String,
        language: model::advanced::LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<PronunciationLexicon, model::advanced::TtsError>;

    async fn synthesize_long_form(
        provider_config: Self::ProviderConfig,
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

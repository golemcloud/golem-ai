pub mod config;
pub mod durability;
pub mod error;
pub mod guest;
pub mod http;
pub mod languages;
mod retry;
pub mod runtime;
pub mod transcription;

pub mod model;

use crate::model::languages::LanguageInfo;
use crate::model::transcription::{
    MultiTranscriptionResult, SttError, TranscriptionRequest, TranscriptionResult,
};
use std::{cell::RefCell, str::FromStr};

/// Trait for providers that expose a static set of supported languages.
///
/// `LanguageProvider` does not require a `ProviderConfig` because every
/// stt provider implementation derives its language list from a static
/// in-process table; no host-side or per-request configuration is
/// needed for this lookup.
pub trait LanguageProvider {
    fn list_languages() -> Result<Vec<LanguageInfo>, model::languages::SttError>;
}

#[allow(async_fn_in_trait)]
pub trait TranscriptionProvider {
    /// Provider-specific configuration (API keys, regions, bucket
    /// names, etc.) that the caller resolves once and passes in. Each
    /// provider crate defines its own concrete config type; see e.g.
    /// `golem_ai_stt_whisper::WhisperConfig`.
    type ProviderConfig: Clone + 'static;

    async fn transcribe(
        provider_config: Self::ProviderConfig,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult, SttError>;

    async fn transcribe_many(
        provider_config: Self::ProviderConfig,
        requests: Vec<TranscriptionRequest>,
    ) -> Result<MultiTranscriptionResult, SttError>;
}

pub struct LoggingState {
    logging_initialized: bool,
}

impl LoggingState {
    pub fn init(&mut self) {
        if !self.logging_initialized {
            eprintln!("Init logging");
            let _ = wasi_logger::Logger::install();
            let max_level: log::LevelFilter = log::LevelFilter::from_str(
                &std::env::var("STT_PROVIDER_LOG_LEVEL").unwrap_or_default(),
            )
            .unwrap_or(log::LevelFilter::Warn);
            eprintln!("Setting log level to {max_level}");
            log::set_max_level(max_level);
            self.logging_initialized = true;
        }
    }
}

thread_local! {
    /// This holds the state of our application.
    pub static LOGGING_STATE: RefCell<LoggingState> = const { RefCell::new(LoggingState {
        logging_initialized: false,
    }) };
}

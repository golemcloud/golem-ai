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

pub trait LanguageProvider {
    fn list_languages() -> Result<Vec<LanguageInfo>, model::languages::SttError>;
}

pub trait TranscriptionProvider {
    fn transcribe(request: TranscriptionRequest) -> Result<TranscriptionResult, SttError>;

    fn transcribe_many(
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

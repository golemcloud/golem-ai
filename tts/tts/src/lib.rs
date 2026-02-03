pub mod durability;
pub mod error;
pub mod guest;
pub mod http;
pub mod retry;
pub mod runtime;
pub mod voices;

wit_bindgen::generate!({
    path: "wit",
    world: "tts-library",
    generate_all,
    generate_unused_types: true,
    additional_derives: [PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue, Clone],
    pub_export_macro: true,
});

pub use crate::exports::golem;
pub use __export_tts_library_impl as export_tts;

use std::{cell::RefCell, str::FromStr};

pub struct LoggingState {
    logging_initialized: bool,
}

impl LoggingState {
    pub fn init(&mut self) {
        if !self.logging_initialized {
            let _ = wasi_logger::Logger::install();
            let max_level: log::LevelFilter = log::LevelFilter::from_str(
                &std::env::var("TTS_PROVIDER_LOG_LEVEL").unwrap_or_default(),
            )
            .unwrap_or(log::LevelFilter::Warn);
            log::set_max_level(max_level);
            self.logging_initialized = true;
        }
    }
}

thread_local! {
    pub static LOGGING_STATE: RefCell<LoggingState> = const { RefCell::new(LoggingState {
        logging_initialized: false,
    }) };
}

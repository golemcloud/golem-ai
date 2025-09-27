pub mod durability;
pub mod error;
pub mod guest;
pub mod http;
pub mod languages;
mod retry;
pub mod runtime;
pub mod transcription;

wit_bindgen::generate!({
    path: "../wit",
    world: "stt-library",
    generate_all,
    generate_unused_types: true,
    additional_derives: [PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue],
    pub_export_macro: true,
});

use std::{cell::RefCell, str::FromStr};

// re-export generated exports
pub use crate::exports::golem;
pub use __export_stt_library_impl as export_stt;

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

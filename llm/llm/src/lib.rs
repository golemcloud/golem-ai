pub mod chat_stream;
pub mod config;
pub mod durability;
pub mod error;

#[allow(dead_code)]
pub mod event_source;

wit_bindgen::generate!({
    path: "../wit",
    world: "llm-library",
    generate_all,
    generate_unused_types: true,
    additional_derives: [PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue],
    pub_export_macro: true,
});

pub use crate::exports::golem;
pub use __export_llm_library_impl as export_llm;
use std::cell::RefCell;
use std::str::FromStr;

struct LoggingState {
    logging_initialized: bool,
}

impl LoggingState {
    /// Initializes WASI logging based on the `GOLEM_LLM_LOG` environment variable.
    fn init(&mut self) {
        if !self.logging_initialized {
            let _ = wasi_logger::Logger::install();
            let max_level: log::LevelFilter =
                log::LevelFilter::from_str(&std::env::var("GOLEM_LLM_LOG").unwrap_or_default())
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

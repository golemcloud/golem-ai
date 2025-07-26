pub mod config;
pub mod durability;
pub mod error;
pub mod types;

wit_bindgen::generate!({
    path: "../wit",
    world: "websearch-library",
    generate_all,
    generate_unused_types: true,
    additional_derives: [
        PartialEq,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
        Clone,
    ],
    pub_export_macro: true,
});

// Export the generated bindings properly
pub use crate::exports::golem;
pub use __export_websearch_library_impl as export_websearch;

use std::cell::RefCell;
use std::str::FromStr;

/// Internal state for configuring WASI log levels during runtime.
struct LoggingState {
    logging_initialized: bool,
}

impl LoggingState {
    /// Initializes WASI logging based on the `GOLEM_WEB_SEARCH_LOG` environment variable.
    pub fn init(&mut self) {
        if !self.logging_initialized {
            let _ = wasi_logger::Logger::install();
            let max_level = log::LevelFilter::from_str(
                &std::env::var("GOLEM_WEB_SEARCH_LOG").unwrap_or_default(),
            )
            .unwrap_or(log::LevelFilter::Info);
            log::set_max_level(max_level);
            self.logging_initialized = true;
        }
    }
}

thread_local! {
    static LOGGING_STATE: RefCell<LoggingState> = const {
        RefCell::new(LoggingState {
            logging_initialized: false,
        })
    };
}

pub fn init_logging() {
    LOGGING_STATE.with_borrow_mut(|state| state.init());
}

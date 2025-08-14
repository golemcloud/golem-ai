pub mod durability;

wit_bindgen::generate!({
    path: "wit",
    world: "stt-library",
    generate_all,
    generate_unused_types: true,
    additional_derives: [PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue],
    pub_export_macro: true,
});

pub use crate::exports::golem;
pub use __export_stt_library_impl as export_stt;

use std::cell::RefCell;
use std::str::FromStr;
#[cfg(test)]
mod tests {
    use super::durability::retry::backoff_delay_ms;
    #[test]
    fn backoff_grows_and_caps() {
        let b = 100u64;
        let m = 5000u64;
        let d0 = backoff_delay_ms(0, b, m, 0);
        let d1 = backoff_delay_ms(1, b, m, 0);
        let d5 = backoff_delay_ms(5, b, m, 0);
        assert!(d1 > d0);
        assert!(d5 >= d1);
        assert!(d5 <= m);
    }

    
}

struct LoggingState {
    logging_initialized: bool,
}

impl LoggingState {
    fn init(&mut self) {
        if !self.logging_initialized {
            let _ = wasi_logger::Logger::install();
            let max_level: log::LevelFilter = log::LevelFilter::from_str(
                &std::env::var("GOLEM_STT_LOG").unwrap_or_default(),
            )
            .unwrap_or(log::LevelFilter::Info);
            log::set_max_level(max_level);
            self.logging_initialized = true;
        }
    }
}

thread_local! {
    static LOGGING_STATE: RefCell<LoggingState> = const { RefCell::new(LoggingState { logging_initialized: false }) };
}

pub fn init_logging() {
    LOGGING_STATE.with_borrow_mut(|state| state.init());
} 
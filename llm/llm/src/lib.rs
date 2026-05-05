pub mod chat_stream;
pub mod config;
pub mod durability;
pub mod error;
pub mod model;

#[allow(dead_code)]
pub mod event_source;

use crate::model::{ChatStream, Config, Error, Event, Response, StreamEvent};
use async_trait::async_trait;
use std::cell::RefCell;
use std::str::FromStr;

#[allow(async_fn_in_trait)]
pub trait LlmProvider {
    type ChatStream: ChatStreamInterface;

    async fn send(events: Vec<Event>, config: Config) -> Result<Response, Error>;

    async fn stream(events: Vec<Event>, config: Config) -> ChatStream;
}

/// `ChatStreamInterface` is used as `Box<dyn ChatStreamInterface>` inside `ChatStream`, so its
/// async methods have to go through the `#[async_trait]` macro (boxed futures) to remain
/// dyn-compatible. `?Send` because WASM is single-threaded.
#[async_trait(?Send)]
pub trait ChatStreamInterface: 'static {
    async fn poll_next(&self) -> Option<Vec<Result<StreamEvent, Error>>>;

    async fn get_next(&self) -> Vec<Result<StreamEvent, Error>>;

    fn as_any(&self) -> &dyn std::any::Any;

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

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

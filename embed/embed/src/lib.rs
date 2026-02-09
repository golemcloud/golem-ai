pub mod config;
pub mod durability;
pub mod error;
pub mod model;

use crate::model::{Config, ContentPart, EmbeddingResponse, Error, RerankResponse};
use std::cell::RefCell;
use std::str::FromStr;

pub trait EmbeddingProvider {
    fn generate(inputs: Vec<ContentPart>, config: Config) -> Result<EmbeddingResponse, Error>;

    fn rerank(
        query: String,
        documents: Vec<String>,
        config: Config,
    ) -> Result<RerankResponse, Error>;
}

pub struct LoggingState {
    logging_initialized: bool,
}

impl LoggingState {
    pub fn init(&mut self) {
        if !self.logging_initialized {
            let _ = wasi_logger::Logger::install();
            let max_level: log::LevelFilter =
                log::LevelFilter::from_str(&std::env::var("GOLEM_EMBED_LOG").unwrap_or_default())
                    .unwrap_or(log::LevelFilter::Info);
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

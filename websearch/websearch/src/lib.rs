pub mod config;
pub mod durability;
pub mod error;
pub mod model;
pub mod types;

use crate::model::web_search::{
    SearchError, SearchMetadata, SearchParams, SearchResult, SearchSession,
};
use std::cell::RefCell;
use std::str::FromStr;

pub trait SearchSessionInterface: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn next_page(&self) -> Result<Vec<SearchResult>, SearchError>;
    fn get_metadata(&self) -> Option<SearchMetadata>;
}

pub trait WebSearchProvider {
    type SearchSession: SearchSessionInterface;

    fn start_search(params: SearchParams) -> Result<SearchSession, SearchError>;
    fn search_once(
        params: SearchParams,
    ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError>;
}

struct LoggingState {
    logging_initialized: bool,
}

impl LoggingState {
    fn init(&mut self) {
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

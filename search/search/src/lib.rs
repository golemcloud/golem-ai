pub mod config;
pub mod durability;
pub mod error;
pub mod model;

use crate::model::{
    CreateIndexOptions, Doc, DocumentId, IndexName, Schema, SearchError, SearchHit, SearchQuery,
    SearchResults, SearchStream,
};
use std::cell::RefCell;
use std::str::FromStr;

pub trait SearchStreamInterface: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn get_next(&self) -> Option<Vec<SearchHit>>;
    fn blocking_get_next(&self) -> Vec<SearchHit>;
}

pub trait SearchProvider {
    type SearchStream: SearchStreamInterface;

    fn create_index(options: CreateIndexOptions) -> Result<(), SearchError>;
    fn delete_index(name: IndexName) -> Result<(), SearchError>;
    fn list_indexes() -> Result<Vec<IndexName>, SearchError>;
    fn upsert(index: IndexName, doc: Doc) -> Result<(), SearchError>;
    fn upsert_many(index: IndexName, docs: Vec<Doc>) -> Result<(), SearchError>;
    fn delete(index: IndexName, id: DocumentId) -> Result<(), SearchError>;
    fn delete_many(index: IndexName, ids: Vec<DocumentId>) -> Result<(), SearchError>;
    fn get(index: IndexName, id: DocumentId) -> Result<Option<Doc>, SearchError>;
    fn search(index: IndexName, query: SearchQuery) -> Result<SearchResults, SearchError>;
    fn stream_search(index: IndexName, query: SearchQuery) -> Result<SearchStream, SearchError>;
    fn get_schema(index: IndexName) -> Result<Schema, SearchError>;
    fn update_schema(index: IndexName, schema: Schema) -> Result<(), SearchError>;
}

impl<'a> From<&'a SearchError> for SearchError {
    fn from(value: &'a SearchError) -> Self {
        value.clone()
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
                &std::env::var("SEARCH_PROVIDER_LOG_LEVEL").unwrap_or_default(),
            )
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

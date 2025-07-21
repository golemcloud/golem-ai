use crate::client::AlgoliaSearchApi;
use crate::conversions::{
    algolia_object_to_doc, algolia_response_to_search_results, algolia_settings_to_schema,
    create_retry_query, doc_to_algolia_object, schema_to_algolia_settings,
    search_query_to_algolia_query,
};
use golem_rust::wasm_rpc::Pollable;
use golem_search::config::with_config_keys;
use golem_search::durability::{DurableSearch, ExtendedGuest};
use golem_search::golem::search::core::{Guest, GuestSearchStream, SearchStream};
use golem_search::golem::search::types::{
    Doc, DocumentId, IndexName, Schema, SearchError, SearchHit, SearchQuery, SearchResults,
};
use golem_search::LOGGING_STATE;
use std::cell::{Cell, RefCell};

mod client;
mod conversions;

struct AlgoliaSearchStream {
    client: AlgoliaSearchApi,
    index_name: String,
    query: SearchQuery,
    current_page: Cell<u32>,
    finished: Cell<bool>,
    last_response: RefCell<Option<SearchResults>>,
}

impl AlgoliaSearchStream {
    pub fn new(client: AlgoliaSearchApi, index_name: String, query: SearchQuery) -> Self {
        Self {
            client,
            index_name,
            query: query.clone(),
            current_page: Cell::new(query.page.unwrap_or(0)),
            finished: Cell::new(false),
            last_response: RefCell::new(None),
        }
    }

    pub fn subscribe(&self) -> Pollable {
        golem_rust::bindings::wasi::clocks::monotonic_clock::subscribe_duration(0)
    }
}

impl GuestSearchStream for AlgoliaSearchStream {
    fn get_next(&self) -> Option<Vec<SearchHit>> {
        if self.finished.get() {
            return Some(vec![]);
        }

        let mut search_query = self.query.clone();
        search_query.page = Some(self.current_page.get());

        let algolia_query = search_query_to_algolia_query(search_query);

        match self.client.search(&self.index_name, &algolia_query) {
            Ok(response) => {
                let search_results = algolia_response_to_search_results(response);

                let current_page = self.current_page.get();
                let total_pages = if let (Some(total), Some(per_page)) =
                    (search_results.total, search_results.per_page)
                {
                    total.div_ceil(per_page)
                } else {
                    current_page + 1
                };

                if current_page >= total_pages || search_results.hits.is_empty() {
                    self.finished.set(true);
                }

                self.current_page.set(current_page + 1);

                let hits = search_results.hits.clone();
                *self.last_response.borrow_mut() = Some(search_results);

                Some(hits)
            }
            Err(_) => {
                self.finished.set(true);
                Some(vec![])
            }
        }
    }

    fn blocking_get_next(&self) -> Vec<SearchHit> {
        self.get_next().unwrap_or_default()
    }
}

struct AlgoliaComponent;

impl AlgoliaComponent {
    const APPLICATION_ID_ENV_VAR: &'static str = "ALGOLIA_APPLICATION_ID";
    const API_KEY_ENV_VAR: &'static str = "ALGOLIA_API_KEY";

    fn create_client() -> Result<AlgoliaSearchApi, SearchError> {
        with_config_keys(
            &[Self::APPLICATION_ID_ENV_VAR, Self::API_KEY_ENV_VAR],
            |keys| {
                if keys.len() != 2 {
                    return Err(SearchError::Internal(
                        "Missing Algolia credentials".to_string(),
                    ));
                }

                let application_id = keys[0].clone();
                let api_key = keys[1].clone();

                Ok(AlgoliaSearchApi::new(application_id, api_key))
            },
        )
    }
}

impl Guest for AlgoliaComponent {
    type SearchStream = AlgoliaSearchStream;

    fn create_index(_name: IndexName, _schema: Option<Schema>) -> Result<(), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        // Algolia doesn't require explicit index creation - indices are created automatically
        // when you first add documents.
        // providers that don't support index creation should return unsupported.
        Err(SearchError::Unsupported)
    }

    fn delete_index(name: IndexName) -> Result<(), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;

        match client.delete_index(&name) {
            Ok(response) => {
                let _ = response;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn list_indexes() -> Result<Vec<IndexName>, SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;

        match client.list_indexes() {
            Ok(response) => Ok(response.items.into_iter().map(|item| item.name).collect()),
            Err(e) => Err(e),
        }
    }

    fn upsert(index: IndexName, doc: Doc) -> Result<(), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;
        let algolia_object = doc_to_algolia_object(doc).map_err(SearchError::InvalidQuery)?;

        match client.save_object(&index, &algolia_object) {
            Ok(response) => {
                let _ = response;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn upsert_many(index: IndexName, docs: Vec<Doc>) -> Result<(), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;
        let mut algolia_objects = Vec::new();

        for doc in docs {
            let algolia_object = doc_to_algolia_object(doc).map_err(SearchError::InvalidQuery)?;
            algolia_objects.push(algolia_object);
        }

        match client.save_objects(&index, &algolia_objects) {
            Ok(response) => {
                let _ = response;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn delete(index: IndexName, id: DocumentId) -> Result<(), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;

        match client.delete_object(&index, &id) {
            Ok(response) => {
                let _ = response;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn delete_many(index: IndexName, ids: Vec<DocumentId>) -> Result<(), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;

        match client.delete_objects(&index, &ids) {
            Ok(response) => {
                let _ = response;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn get(index: IndexName, id: DocumentId) -> Result<Option<Doc>, SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;

        match client.get_object(&index, &id) {
            Ok(Some(algolia_object)) => Ok(Some(algolia_object_to_doc(algolia_object))),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn search(index: IndexName, query: SearchQuery) -> Result<SearchResults, SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;
        let algolia_query = search_query_to_algolia_query(query);

        match client.search(&index, &algolia_query) {
            Ok(response) => Ok(algolia_response_to_search_results(response)),
            Err(e) => Err(e),
        }
    }

    fn stream_search(index: IndexName, query: SearchQuery) -> Result<SearchStream, SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;
        let stream = AlgoliaSearchStream::new(client, index, query);
        Ok(SearchStream::new(stream))
    }

    fn get_schema(index: IndexName) -> Result<Schema, SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;

        match client.get_settings(&index) {
            Ok(settings) => Ok(algolia_settings_to_schema(settings)),
            Err(e) => Err(e),
        }
    }

    fn update_schema(index: IndexName, schema: Schema) -> Result<(), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;
        let settings = schema_to_algolia_settings(schema);

        client.set_settings(&index, &settings)?;

        Ok(())
    }
}

impl ExtendedGuest for AlgoliaComponent {
    fn unwrapped_stream(index: IndexName, query: SearchQuery) -> Self::SearchStream {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()
            .unwrap_or_else(|_| AlgoliaSearchApi::new("dummy".to_string(), "dummy".to_string()));

        AlgoliaSearchStream::new(client, index, query)
    }

    fn retry_query(original_query: &SearchQuery, partial_hits: &[SearchHit]) -> SearchQuery {
        create_retry_query(original_query, partial_hits)
    }

    fn subscribe(stream: &Self::SearchStream) -> Pollable {
        stream.subscribe()
    }
}

type DurableAlgoliaComponent = DurableSearch<AlgoliaComponent>;

golem_search::export_search!(DurableAlgoliaComponent with_types_in golem_search);

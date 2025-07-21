use crate::client::OpenSearchApi;
use crate::conversions::{
    create_retry_query, doc_to_opensearch_document, opensearch_document_to_doc,
    opensearch_mappings_to_schema, opensearch_response_to_search_results,
    opensearch_scroll_response_to_search_results, schema_to_opensearch_settings,
    search_query_to_opensearch_request,
};
use golem_rust::wasm_rpc::Pollable;
use golem_search::config::with_config_keys;
use golem_search::durability::{DurableSearch, ExtendedGuest};
use golem_search::golem::search::core::{Guest, GuestSearchStream, SearchStream};
use golem_search::golem::search::types::{
    Doc, DocumentId, IndexName, Schema, SearchError, SearchHit, SearchQuery, SearchResults,
};
use golem_search::LOGGING_STATE;
use log::trace;
use std::cell::{Cell, RefCell};

mod client;
mod conversions;

/// Uses scroll API for streaming large result sets with fallback to pagination
struct OpenSearchSearchStream {
    client: OpenSearchApi,
    index_name: String,
    query: SearchQuery,
    scroll_id: RefCell<Option<String>>,
    finished: Cell<bool>,
    current_offset: Cell<u32>,
    use_scroll: Cell<bool>,
    scroll_failed: Cell<bool>,
}

impl OpenSearchSearchStream {
    pub fn new(client: OpenSearchApi, index_name: String, query: SearchQuery) -> Self {
        Self {
            client,
            index_name,
            query: query.clone(),
            scroll_id: RefCell::new(None),
            finished: Cell::new(false),
            current_offset: Cell::new(query.offset.unwrap_or(0)),
            use_scroll: Cell::new(true), // Start with scroll, fallback to pagination if needed
            scroll_failed: Cell::new(false),
        }
    }

    pub fn subscribe(&self) -> Pollable {
        golem_rust::bindings::wasi::clocks::monotonic_clock::subscribe_duration(0)
    }
}

impl OpenSearchSearchStream {
    fn try_scroll_next(&self) -> Option<Option<Vec<SearchHit>>> {
        if self.scroll_id.borrow().is_none() {
            let mut os_query = search_query_to_opensearch_request(self.query.clone());
            os_query.from = Some(0);
            os_query.size = Some(self.query.per_page.unwrap_or(100)); // Larger page size for scroll

            match self
                .client
                .search_with_scroll(&self.index_name, &os_query, "1m")
            {
                Ok(response) => {
                    let scroll_id = response.scroll_id.clone();
                    *self.scroll_id.borrow_mut() = Some(scroll_id);

                    let search_results = opensearch_scroll_response_to_search_results(response);

                    if search_results.hits.is_empty() {
                        self.finished.set(true);
                        return Some(Some(vec![]));
                    }

                    Some(Some(search_results.hits))
                }
                Err(e) => {
                    trace!("Initial scroll search failed: {e:?}");
                    None
                }
            }
        } else {
            let scroll_id = self.scroll_id.borrow().clone().unwrap();

            match self.client.scroll(&scroll_id, "1m") {
                Ok(response) => {
                    let search_results = opensearch_scroll_response_to_search_results(response);

                    if search_results.hits.is_empty() {
                        self.finished.set(true);
                        return Some(Some(vec![]));
                    }

                    Some(Some(search_results.hits))
                }
                Err(e) => {
                    trace!("Scroll continuation failed: {e:?}");
                    None
                }
            }
        }
    }

    fn try_pagination_next(&self) -> Option<Vec<SearchHit>> {
        let mut os_query = search_query_to_opensearch_request(self.query.clone());
        os_query.from = Some(self.current_offset.get());
        os_query.size = Some(self.query.per_page.unwrap_or(10));

        match self.client.search(&self.index_name, &os_query) {
            Ok(response) => {
                let search_results = opensearch_response_to_search_results(response);

                if search_results.hits.is_empty() {
                    self.finished.set(true);
                    return Some(vec![]);
                }

                let current_offset = self.current_offset.get();
                let received_count = search_results.hits.len() as u32;
                self.current_offset.set(current_offset + received_count);

                if let Some(total) = search_results.total {
                    if self.current_offset.get() >= total {
                        self.finished.set(true);
                    }
                }

                Some(search_results.hits)
            }
            Err(e) => {
                trace!("Pagination search failed: {e:?}");
                self.finished.set(true);
                Some(vec![])
            }
        }
    }
}

impl GuestSearchStream for OpenSearchSearchStream {
    fn get_next(&self) -> Option<Vec<SearchHit>> {
        if self.finished.get() {
            return Some(vec![]);
        }

        if self.use_scroll.get() && !self.scroll_failed.get() {
            self.try_scroll_next().unwrap_or_else(|| {
                trace!("Scroll failed, falling back to pagination");
                self.scroll_failed.set(true);
                self.use_scroll.set(false);
                self.try_pagination_next()
            })
        } else {
            self.try_pagination_next()
        }
    }

    fn blocking_get_next(&self) -> Vec<SearchHit> {
        self.get_next().unwrap_or_default()
    }
}

struct OpenSearchComponent;

impl OpenSearchComponent {
    const BASE_URL_ENV_VAR: &'static str = "OPENSEARCH_BASE_URL";
    const USERNAME_ENV_VAR: &'static str = "OPENSEARCH_USERNAME";
    const PASSWORD_ENV_VAR: &'static str = "OPENSEARCH_PASSWORD";
    const API_KEY_ENV_VAR: &'static str = "OPENSEARCH_API_KEY";

    fn create_client() -> Result<OpenSearchApi, SearchError> {
        with_config_keys(&[Self::BASE_URL_ENV_VAR], |keys| {
            if keys.is_empty() {
                return Err(SearchError::Internal(
                    "Missing OpenSearch base URL".to_string(),
                ));
            }

            let base_url = keys[0].clone();

            let username = std::env::var(Self::USERNAME_ENV_VAR).ok();
            let password = std::env::var(Self::PASSWORD_ENV_VAR).ok();
            let api_key = std::env::var(Self::API_KEY_ENV_VAR).ok();
            {
                Ok(OpenSearchApi::new(base_url, username, password, api_key))
            }
        })
    }
}

impl Guest for OpenSearchComponent {
    type SearchStream = OpenSearchSearchStream;

    fn create_index(name: IndexName, schema: Option<Schema>) -> Result<(), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;

        let settings = schema.map(schema_to_opensearch_settings);
        client.create_index(&name, settings)?;

        Ok(())
    }

    fn delete_index(name: IndexName) -> Result<(), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;
        client.delete_index(&name)?;

        Ok(())
    }

    fn list_indexes() -> Result<Vec<IndexName>, SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;
        let indices = client.list_indices()?;
        Ok(indices.into_iter().map(|idx| idx.index).collect())
    }

    fn upsert(index: IndexName, doc: Doc) -> Result<(), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;
        let opensearch_doc = doc_to_opensearch_document(doc).map_err(SearchError::InvalidQuery)?;

        let doc_id = opensearch_doc
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        client.index_document(&index, &doc_id, &opensearch_doc)?;

        Ok(())
    }

    fn upsert_many(index: IndexName, docs: Vec<Doc>) -> Result<(), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;

        if docs.is_empty() {
            return Ok(());
        }

        let mut bulk_operations = Vec::new();
        for doc in docs {
            let opensearch_doc =
                doc_to_opensearch_document(doc).map_err(SearchError::InvalidQuery)?;

            let doc_id = opensearch_doc
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let action = serde_json::json!({
                "index": {
                    "_index": index,
                    "_id": doc_id
                }
            });
            bulk_operations.push(serde_json::to_string(&action).unwrap());
            bulk_operations.push(serde_json::to_string(&opensearch_doc).unwrap());
        }

        let bulk_body = bulk_operations.join("\n") + "\n";

        let _result = client.bulk_index(&bulk_body)?;

        Ok(())
    }

    fn delete(index: IndexName, id: DocumentId) -> Result<(), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;
        client.delete_document(&index, &id)?;

        Ok(())
    }

    fn delete_many(index: IndexName, ids: Vec<DocumentId>) -> Result<(), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;

        if ids.is_empty() {
            return Ok(());
        }

        let mut bulk_operations = Vec::new();
        for id in ids {
            let action = serde_json::json!({
                "delete": {
                    "_index": index,
                    "_id": id
                }
            });
            bulk_operations.push(serde_json::to_string(&action).unwrap());
        }

        let bulk_body = bulk_operations.join("\n") + "\n";
        client.bulk_index(&bulk_body)?;

        Ok(())
    }

    fn get(index: IndexName, id: DocumentId) -> Result<Option<Doc>, SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;

        match client.get_document(&index, &id)? {
            Some(opensearch_doc) => Ok(Some(opensearch_document_to_doc(opensearch_doc))),
            None => Ok(None),
        }
    }

    fn search(index: IndexName, query: SearchQuery) -> Result<SearchResults, SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;
        let opensearch_request = search_query_to_opensearch_request(query);

        let response = client.search(&index, &opensearch_request)?;
        Ok(opensearch_response_to_search_results(response))
    }

    fn stream_search(index: IndexName, query: SearchQuery) -> Result<SearchStream, SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;
        let stream = OpenSearchSearchStream::new(client, index, query);
        Ok(SearchStream::new(stream))
    }

    fn get_schema(index: IndexName) -> Result<Schema, SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;

        let mappings = client.get_mappings(&index)?;
        Ok(opensearch_mappings_to_schema(
            mappings,
            Some("id".to_string()),
        ))
    }

    fn update_schema(index: IndexName, schema: Schema) -> Result<(), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;
        let settings = schema_to_opensearch_settings(schema);

        if let Some(mappings) = settings.mappings {
            client.put_mappings(&index, &mappings)?;
        }

        Ok(())
    }
}

impl ExtendedGuest for OpenSearchComponent {
    fn unwrapped_stream(index: IndexName, query: SearchQuery) -> Self::SearchStream {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client().unwrap_or_else(|_| {
            OpenSearchApi::new("http://localhost:9200".to_string(), None, None, None)
        });

        OpenSearchSearchStream::new(client, index, query)
    }

    fn retry_query(original_query: &SearchQuery, partial_hits: &[SearchHit]) -> SearchQuery {
        create_retry_query(original_query, partial_hits)
    }

    fn subscribe(stream: &Self::SearchStream) -> Pollable {
        stream.subscribe()
    }
}

impl Drop for OpenSearchSearchStream {
    fn drop(&mut self) {
        // Clear any active scroll when the stream is dropped
        if let Some(scroll_id) = self.scroll_id.borrow().as_ref() {
            let _ = self.client.clear_scroll(scroll_id);
        }
    }
}

type DurableOpenSearchComponent = DurableSearch<OpenSearchComponent>;

golem_search::export_search!(DurableOpenSearchComponent with_types_in golem_search);

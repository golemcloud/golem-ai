use crate::client::MeilisearchApi;
use crate::conversions::{
    create_retry_query, doc_to_meilisearch_document, meilisearch_document_to_doc,
    meilisearch_response_to_search_results, meilisearch_settings_to_schema,
    schema_to_meilisearch_settings, search_query_to_meilisearch_request,
};
use golem_rust::golem_wasm::Pollable;
use golem_search::config::with_config_keys;
use golem_search::durability::{DurableSearch, ExtendedSearchProvider};
use golem_search::model::{CreateIndexOptions, SearchStream};
use golem_search::model::{
    Doc, DocumentId, IndexName, Schema, SearchError, SearchHit, SearchQuery, SearchResults,
};
use golem_search::{SearchProvider, SearchStreamInterface};
use std::cell::{Cell, RefCell};

mod client;
mod conversions;

/// Simple search stream implementation for Meilisearch
/// Since Meilisearch doesn't have native streaming, we implement pagination-based streaming
pub struct MeilisearchSearchStream {
    client: MeilisearchApi,
    index_name: String,
    query: SearchQuery,
    current_page: Cell<u32>,
    finished: Cell<bool>,
    last_response: RefCell<Option<SearchResults>>,
}

impl MeilisearchSearchStream {
    pub fn new(client: MeilisearchApi, index_name: String, query: SearchQuery) -> Self {
        Self {
            client,
            index_name,
            query: query.clone(),
            current_page: Cell::new(query.offset.unwrap_or(0) / query.page.unwrap_or(20)),
            finished: Cell::new(false),
            last_response: RefCell::new(None),
        }
    }

    pub fn subscribe(&self) -> Pollable {
        golem_rust::bindings::wasi::clocks::monotonic_clock::subscribe_duration(0)
    }
}

impl SearchStreamInterface for MeilisearchSearchStream {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn get_next(&self) -> Option<Vec<SearchHit>> {
        if self.finished.get() {
            return Some(vec![]);
        }

        let mut search_query = self.query.clone();
        let current_page = self.current_page.get();
        let limit = search_query.per_page.unwrap_or(20);

        search_query.offset = Some(current_page * limit);

        let meilisearch_request = search_query_to_meilisearch_request(search_query);

        match self.client.search(&self.index_name, &meilisearch_request) {
            Ok(response) => {
                let search_results = meilisearch_response_to_search_results(response);

                if search_results.hits.is_empty() {
                    self.finished.set(true);
                    return Some(vec![]);
                }

                if let (Some(total), Some(per_page)) =
                    (search_results.total, search_results.per_page)
                {
                    let current_offset = current_page * per_page;
                    let next_offset = current_offset + per_page;
                    if next_offset >= total {
                        self.finished.set(true);
                    }
                }

                if (search_results.hits.len() as u32) < limit {
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

pub struct Meilisearch;

impl Meilisearch {
    const BASE_URL_ENV_VAR: &'static str = "MEILISEARCH_BASE_URL";
    const API_KEY_ENV_VAR: &'static str = "MEILISEARCH_API_KEY";

    fn create_client() -> Result<MeilisearchApi, SearchError> {
        with_config_keys(&[Self::BASE_URL_ENV_VAR], |keys| {
            if keys.is_empty() {
                return Err(SearchError::Internal(
                    "Missing Meilisearch base URL".to_string(),
                ));
            }

            let base_url = keys[0].clone();

            let api_key = std::env::var(Self::API_KEY_ENV_VAR).ok();

            Ok(MeilisearchApi::new(base_url, api_key))
        })
    }
}

impl SearchProvider for Meilisearch {
    type SearchStream = MeilisearchSearchStream;

    fn create_index(options: CreateIndexOptions) -> Result<(), SearchError> {
        let client = Self::create_client()?;

        let create_request = client::MeilisearchCreateIndexRequest {
            uid: options.index_name.clone(),
            primary_key: Some("id".to_string()), // Default primary key
        };

        let task = client.create_index(&create_request)?;

        client.wait_for_task(task.task_uid)?;

        if let Some(schema) = options.schema {
            let settings = schema_to_meilisearch_settings(schema);
            let settings_task = client.update_settings(&options.index_name, &settings)?;
            client.wait_for_task(settings_task.task_uid)?;
        }

        Ok(())
    }

    fn delete_index(name: IndexName) -> Result<(), SearchError> {
        let client = Self::create_client()?;

        let task = client.delete_index(&name)?;
        client.wait_for_task(task.task_uid)?;

        Ok(())
    }

    fn list_indexes() -> Result<Vec<IndexName>, SearchError> {
        let client = Self::create_client()?;

        let response = client.list_indexes()?;
        Ok(response
            .results
            .into_iter()
            .map(|index| index.task_uid)
            .collect())
    }

    fn upsert(index: IndexName, doc: Doc) -> Result<(), SearchError> {
        let client = Self::create_client()?;
        let meilisearch_doc =
            doc_to_meilisearch_document(doc).map_err(SearchError::InvalidQuery)?;

        let task = client.add_documents(&index, &[meilisearch_doc])?;
        client.wait_for_task(task.task_uid)?;

        Ok(())
    }

    fn upsert_many(index: IndexName, docs: Vec<Doc>) -> Result<(), SearchError> {
        let client = Self::create_client()?;
        let mut meilisearch_docs = Vec::new();

        for doc in docs {
            let meilisearch_doc =
                doc_to_meilisearch_document(doc).map_err(SearchError::InvalidQuery)?;
            meilisearch_docs.push(meilisearch_doc);
        }

        let task = client.add_documents(&index, &meilisearch_docs)?;
        client.wait_for_task(task.task_uid)?;

        Ok(())
    }

    fn delete(index: IndexName, id: DocumentId) -> Result<(), SearchError> {
        let client = Self::create_client()?;

        let task = client.delete_document(&index, &id)?;
        client.wait_for_task(task.task_uid)?;

        Ok(())
    }

    fn delete_many(index: IndexName, ids: Vec<DocumentId>) -> Result<(), SearchError> {
        let client = Self::create_client()?;

        let task = client.delete_documents(&index, &ids)?;
        client.wait_for_task(task.task_uid)?;

        Ok(())
    }

    fn get(index: IndexName, id: DocumentId) -> Result<Option<Doc>, SearchError> {
        let client = Self::create_client()?;

        match client.get_document(&index, &id)? {
            Some(meilisearch_doc) => Ok(Some(meilisearch_document_to_doc(meilisearch_doc))),
            None => Ok(None),
        }
    }

    fn search(index: IndexName, query: SearchQuery) -> Result<SearchResults, SearchError> {
        let client = Self::create_client()?;
        let meilisearch_request = search_query_to_meilisearch_request(query);

        let response = client.search(&index, &meilisearch_request)?;
        Ok(meilisearch_response_to_search_results(response))
    }

    fn stream_search(index: IndexName, query: SearchQuery) -> Result<SearchStream, SearchError> {
        let client = Self::create_client()?;
        let stream = MeilisearchSearchStream::new(client, index, query);
        Ok(SearchStream::new(stream))
    }

    fn get_schema(index: IndexName) -> Result<Schema, SearchError> {
        let client = Self::create_client()?;

        let settings = client.get_settings(&index)?;
        Ok(meilisearch_settings_to_schema(settings))
    }

    fn update_schema(index: IndexName, schema: Schema) -> Result<(), SearchError> {
        let client = Self::create_client()?;
        let settings = schema_to_meilisearch_settings(schema);

        let _task = client.update_settings(&index, &settings)?;

        Ok(())
    }
}

impl ExtendedSearchProvider for Meilisearch {
    fn unwrapped_stream(index: IndexName, query: SearchQuery) -> Self::SearchStream {
        let client = Self::create_client()
            .unwrap_or_else(|_| MeilisearchApi::new("http://localhost:7700".to_string(), None));

        MeilisearchSearchStream::new(client, index, query)
    }

    fn retry_query(original_query: &SearchQuery, partial_hits: &[SearchHit]) -> SearchQuery {
        create_retry_query(original_query, partial_hits)
    }

    fn subscribe(stream: &Self::SearchStream) -> Pollable {
        stream.subscribe()
    }
}

pub type DurableMeilisearch = DurableSearch<Meilisearch>;

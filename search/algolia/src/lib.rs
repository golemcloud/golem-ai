use crate::client::AlgoliaSearchApi;
use crate::conversions::{
    algolia_object_to_doc, algolia_response_to_search_results, algolia_settings_to_schema,
    create_retry_query, doc_to_algolia_object, schema_to_algolia_settings,
    search_query_to_algolia_query,
};
use golem_ai_search::durability::{DurableSearch, ExtendedSearchProvider};
use golem_ai_search::model::{CreateIndexOptions, SearchStream};
use golem_ai_search::model::{
    Doc, DocumentId, IndexName, Schema, SearchError, SearchHit, SearchQuery, SearchResults,
};
use golem_ai_search::wasi_compat::{subscribe_zero, Pollable};
use golem_ai_search::{SearchProvider, SearchStreamInterface};
use std::cell::{Cell, RefCell};

mod client;
pub mod config;
mod conversions;

pub use crate::config::AlgoliaConfig;
#[cfg(feature = "golem")]
pub use crate::config::AlgoliaHostConfig;

pub struct AlgoliaSearchStream {
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
        subscribe_zero()
    }
}

impl SearchStreamInterface for AlgoliaSearchStream {
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

pub struct Algolia;

impl SearchProvider for Algolia {
    type SearchStream = AlgoliaSearchStream;
    type ProviderConfig = AlgoliaConfig;

    fn create_index(
        _provider_config: Self::ProviderConfig,
        _options: CreateIndexOptions,
    ) -> Result<(), SearchError> {
        // Algolia doesn't require explicit index creation - indices are created automatically
        // when you first add documents.
        // providers that don't support index creation should return unsupported.
        Err(SearchError::Unsupported)
    }

    fn delete_index(
        provider_config: Self::ProviderConfig,
        name: IndexName,
    ) -> Result<(), SearchError> {
        let client = AlgoliaSearchApi::new(&provider_config);

        match client.delete_index(&name) {
            Ok(response) => {
                let _ = response;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn list_indexes(provider_config: Self::ProviderConfig) -> Result<Vec<IndexName>, SearchError> {
        let client = AlgoliaSearchApi::new(&provider_config);

        match client.list_indexes() {
            Ok(response) => Ok(response.items.into_iter().map(|item| item.name).collect()),
            Err(e) => Err(e),
        }
    }

    fn upsert(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        doc: Doc,
    ) -> Result<(), SearchError> {
        let client = AlgoliaSearchApi::new(&provider_config);
        let algolia_object = doc_to_algolia_object(doc).map_err(SearchError::InvalidQuery)?;

        match client.save_object(&index, &algolia_object) {
            Ok(response) => {
                let _ = response;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn upsert_many(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        docs: Vec<Doc>,
    ) -> Result<(), SearchError> {
        let client = AlgoliaSearchApi::new(&provider_config);
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

    fn delete(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        id: DocumentId,
    ) -> Result<(), SearchError> {
        let client = AlgoliaSearchApi::new(&provider_config);

        match client.delete_object(&index, &id) {
            Ok(response) => {
                let _ = response;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn delete_many(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        ids: Vec<DocumentId>,
    ) -> Result<(), SearchError> {
        let client = AlgoliaSearchApi::new(&provider_config);

        match client.delete_objects(&index, &ids) {
            Ok(response) => {
                let _ = response;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn get(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        id: DocumentId,
    ) -> Result<Option<Doc>, SearchError> {
        let client = AlgoliaSearchApi::new(&provider_config);

        match client.get_object(&index, &id) {
            Ok(Some(algolia_object)) => Ok(Some(algolia_object_to_doc(algolia_object))),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn search(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        query: SearchQuery,
    ) -> Result<SearchResults, SearchError> {
        let client = AlgoliaSearchApi::new(&provider_config);
        let algolia_query = search_query_to_algolia_query(query);

        match client.search(&index, &algolia_query) {
            Ok(response) => Ok(algolia_response_to_search_results(response)),
            Err(e) => Err(e),
        }
    }

    fn stream_search(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        query: SearchQuery,
    ) -> Result<SearchStream, SearchError> {
        let client = AlgoliaSearchApi::new(&provider_config);
        let stream = AlgoliaSearchStream::new(client, index, query);
        Ok(SearchStream::new(stream))
    }

    fn get_schema(
        provider_config: Self::ProviderConfig,
        index: IndexName,
    ) -> Result<Schema, SearchError> {
        let client = AlgoliaSearchApi::new(&provider_config);

        match client.get_settings(&index) {
            Ok(settings) => Ok(algolia_settings_to_schema(settings)),
            Err(e) => Err(e),
        }
    }

    fn update_schema(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        schema: Schema,
    ) -> Result<(), SearchError> {
        let client = AlgoliaSearchApi::new(&provider_config);
        let settings = schema_to_algolia_settings(schema);

        client.set_settings(&index, &settings)?;

        Ok(())
    }
}

impl ExtendedSearchProvider for Algolia {
    fn unwrapped_stream(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        query: SearchQuery,
    ) -> Self::SearchStream {
        let client = AlgoliaSearchApi::new(&provider_config);
        AlgoliaSearchStream::new(client, index, query)
    }

    fn retry_query(original_query: &SearchQuery, partial_hits: &[SearchHit]) -> SearchQuery {
        create_retry_query(original_query, partial_hits)
    }

    fn subscribe(stream: &Self::SearchStream) -> Pollable {
        stream.subscribe()
    }
}

pub type DurableAlgolia = DurableSearch<Algolia>;

use crate::client::OpenSearchApi;
use crate::conversions::{
    create_retry_query, doc_to_opensearch_document, opensearch_document_to_doc,
    opensearch_mappings_to_schema, opensearch_response_to_search_results,
    opensearch_scroll_response_to_search_results, schema_to_opensearch_settings,
    search_query_to_opensearch_request,
};
use golem_ai_search::durability::{DurableSearch, ExtendedSearchProvider};
use golem_ai_search::model::{CreateIndexOptions, SearchStream};
use golem_ai_search::model::{
    Doc, DocumentId, IndexName, Schema, SearchError, SearchHit, SearchQuery, SearchResults,
};
use golem_ai_search::wasi_compat::{subscribe_zero, Pollable};
use golem_ai_search::{SearchProvider, SearchStreamInterface};
use log::trace;
use std::cell::{Cell, RefCell};

mod client;
pub mod config;
mod conversions;

pub use crate::config::OpenSearchConfig;
#[cfg(feature = "golem")]
pub use crate::config::OpenSearchHostConfig;

/// Uses scroll API for streaming large result sets with fallback to pagination
pub struct OpenSearchSearchStream {
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
        subscribe_zero()
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

impl SearchStreamInterface for OpenSearchSearchStream {
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

pub struct OpenSearch;

impl SearchProvider for OpenSearch {
    type SearchStream = OpenSearchSearchStream;
    type ProviderConfig = OpenSearchConfig;

    fn create_index(
        provider_config: Self::ProviderConfig,
        options: CreateIndexOptions,
    ) -> Result<(), SearchError> {
        let client = OpenSearchApi::new(&provider_config);

        let settings = options.schema.map(schema_to_opensearch_settings);
        client.create_index(&options.index_name, settings)?;

        Ok(())
    }

    fn delete_index(
        provider_config: Self::ProviderConfig,
        name: IndexName,
    ) -> Result<(), SearchError> {
        let client = OpenSearchApi::new(&provider_config);
        client.delete_index(&name)?;

        Ok(())
    }

    fn list_indexes(
        provider_config: Self::ProviderConfig,
    ) -> Result<Vec<IndexName>, SearchError> {
        let client = OpenSearchApi::new(&provider_config);
        let indices = client.list_indices()?;
        Ok(indices.into_iter().map(|idx| idx.index).collect())
    }

    fn upsert(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        doc: Doc,
    ) -> Result<(), SearchError> {
        let client = OpenSearchApi::new(&provider_config);
        let opensearch_doc = doc_to_opensearch_document(doc).map_err(SearchError::InvalidQuery)?;

        let doc_id = opensearch_doc
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        client.index_document(&index, &doc_id, &opensearch_doc)?;

        Ok(())
    }

    fn upsert_many(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        docs: Vec<Doc>,
    ) -> Result<(), SearchError> {
        let client = OpenSearchApi::new(&provider_config);

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

    fn delete(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        id: DocumentId,
    ) -> Result<(), SearchError> {
        let client = OpenSearchApi::new(&provider_config);
        client.delete_document(&index, &id)?;

        Ok(())
    }

    fn delete_many(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        ids: Vec<DocumentId>,
    ) -> Result<(), SearchError> {
        let client = OpenSearchApi::new(&provider_config);

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

    fn get(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        id: DocumentId,
    ) -> Result<Option<Doc>, SearchError> {
        let client = OpenSearchApi::new(&provider_config);

        match client.get_document(&index, &id)? {
            Some(opensearch_doc) => Ok(Some(opensearch_document_to_doc(opensearch_doc))),
            None => Ok(None),
        }
    }

    fn search(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        query: SearchQuery,
    ) -> Result<SearchResults, SearchError> {
        let client = OpenSearchApi::new(&provider_config);
        let opensearch_request = search_query_to_opensearch_request(query);

        let response = client.search(&index, &opensearch_request)?;
        Ok(opensearch_response_to_search_results(response))
    }

    fn stream_search(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        query: SearchQuery,
    ) -> Result<SearchStream, SearchError> {
        let client = OpenSearchApi::new(&provider_config);
        let stream = OpenSearchSearchStream::new(client, index, query);
        Ok(SearchStream::new(stream))
    }

    fn get_schema(
        provider_config: Self::ProviderConfig,
        index: IndexName,
    ) -> Result<Schema, SearchError> {
        let client = OpenSearchApi::new(&provider_config);

        let mappings = client.get_mappings(&index)?;
        Ok(opensearch_mappings_to_schema(
            mappings,
            Some("id".to_string()),
        ))
    }

    fn update_schema(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        schema: Schema,
    ) -> Result<(), SearchError> {
        let client = OpenSearchApi::new(&provider_config);
        let settings = schema_to_opensearch_settings(schema);

        if let Some(mappings) = settings.mappings {
            client.put_mappings(&index, &mappings)?;
        }

        Ok(())
    }
}

impl ExtendedSearchProvider for OpenSearch {
    fn unwrapped_stream(
        provider_config: Self::ProviderConfig,
        index: IndexName,
        query: SearchQuery,
    ) -> Self::SearchStream {
        let client = OpenSearchApi::new(&provider_config);
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

pub type DurableOpenSearch = DurableSearch<OpenSearch>;

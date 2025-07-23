use crate::client::{CollectionField, CollectionSchema, TypesenseSearchApi};
use crate::conversions::*;
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

/// Simple search stream implementation for Typesense
/// Since Typesense doesn't have native streaming, we implement pagination-based streaming
struct TypesenseSearchStream {
    client: TypesenseSearchApi,
    index_name: String,
    query: SearchQuery,
    current_page: Cell<u32>,
    finished: Cell<bool>,
    last_response: RefCell<Option<SearchResults>>,
}

impl TypesenseSearchStream {
    fn new(client: TypesenseSearchApi, index_name: String, query: SearchQuery) -> Self {
        Self {
            client,
            index_name,
            query: query.clone(),
            current_page: Cell::new(query.page.unwrap_or(1)),
            finished: Cell::new(false),
            last_response: RefCell::new(None),
        }
    }

    fn subscribe(&self) -> Pollable {
        // For non-streaming APIs, return an immediately ready pollable
        golem_rust::bindings::wasi::clocks::monotonic_clock::subscribe_duration(0)
    }
}

struct TypesenseComponent;

impl TypesenseComponent {
    const API_KEY_ENV_VAR: &'static str = "TYPESENSE_API_KEY";
    const BASE_URL_ENV_VAR: &'static str = "TYPESENSE_BASE_URL";

    fn create_client() -> Result<TypesenseSearchApi, SearchError> {
        with_config_keys(&[Self::API_KEY_ENV_VAR, Self::BASE_URL_ENV_VAR], |keys| {
            if keys.len() != 2 {
                return Err(SearchError::Internal(
                    "Missing Typesense credentials".to_string(),
                ));
            }

            let api_key = keys[0].clone();
            let base_url = keys[1].clone();

            Ok(TypesenseSearchApi::new(api_key, base_url))
        })
    }
}

impl GuestSearchStream for TypesenseSearchStream {
    fn get_next(&self) -> Option<Vec<SearchHit>> {
        if self.finished.get() {
            return Some(vec![]);
        }

        // Prepare query for current page
        let mut search_query = self.query.clone();
        search_query.page = Some(self.current_page.get());

        let typesense_query = search_query_to_typesense_query(search_query);

        match self.client.search(&self.index_name, &typesense_query) {
            Ok(response) => {
                let search_results = typesense_response_to_search_results(response);

                let current_page = self.current_page.get();
                let per_page = self.query.per_page.unwrap_or(20);
                let total_pages = if let Some(total) = search_results.total {
                    total.div_ceil(per_page) // Ceiling division
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
            Err(_e) => {
                self.finished.set(true);
                Some(vec![])
            }
        }
    }

    fn blocking_get_next(&self) -> Vec<SearchHit> {
        self.get_next().unwrap_or_default()
    }
}

impl Guest for TypesenseComponent {
    type SearchStream = TypesenseSearchStream;

    fn create_index(name: IndexName, schema: Option<Schema>) -> Result<(), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;

        let typesense_schema = schema
            .map(|s| schema_to_typesense_schema(s, &name))
            .unwrap_or_else(|| CollectionSchema {
                name: name.clone(),
                fields: vec![CollectionField {
                    name: "id".to_string(),
                    field_type: "string".to_string(),
                    facet: Some(false),
                    index: Some(true),
                    sort: Some(false),
                    optional: Some(false),
                }],
                default_sorting_field: None,
                enable_nested_fields: None,
                token_separators: None,
                symbols_to_index: None,
            });

        client.create_collection(&name, &typesense_schema)?;
        Ok(())
    }

    fn delete_index(name: IndexName) -> Result<(), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;
        client.delete_collection(&name)?;
        Ok(())
    }

    fn list_indexes() -> Result<Vec<IndexName>, SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;
        let response = client.list_collections()?;
        Ok(response
            .0
            .into_iter()
            .map(|collection| collection.name)
            .collect())
    }

    fn upsert(index: IndexName, doc: Doc) -> Result<(), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;
        let typesense_doc = doc_to_typesense_document(doc).map_err(SearchError::Internal)?;
        client.upsert_document(&index, &typesense_doc)?;
        Ok(())
    }

    fn upsert_many(index: IndexName, docs: Vec<Doc>) -> Result<(), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;
        let typesense_docs: Result<Vec<_>, _> = docs
            .iter()
            .map(|doc| doc_to_typesense_document(doc.clone()))
            .collect();
        let typesense_docs = typesense_docs.map_err(SearchError::Internal)?;
        client.index_documents(&index, &typesense_docs)?;
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
        // Typesense doesn't have bulk delete by IDs, so we use filter_by
        let filter = format!("id:[{}]", ids.join(","));
        client.delete_documents_by_query(&index, &filter)?;
        Ok(())
    }

    fn get(index: IndexName, id: DocumentId) -> Result<Option<Doc>, SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;

        // Typesense doesn't have a direct get document endpoint
        // We need to search for the specific document by ID using a filter-only search
        let query = SearchQuery {
            q: Some("*".to_string()),             // Match all documents
            filters: vec![format!("id:={}", id)], // Then filter by exact ID match
            sort: vec![],
            facets: vec![],
            page: Some(1),
            per_page: Some(1),
            offset: None,
            highlight: None,
            config: None,
        };

        let typesense_query = search_query_to_typesense_query(query);
        let response = client.search(&index, &typesense_query)?;
        let results = typesense_response_to_search_results(response);

        Ok(results.hits.into_iter().next().map(|hit| Doc {
            id: hit.id,
            content: hit.content.unwrap_or_else(|| "{}".to_string()),
        }))
    }

    fn search(index: IndexName, query: SearchQuery) -> Result<SearchResults, SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;
        let typesense_query = search_query_to_typesense_query(query);
        let response = client.search(&index, &typesense_query)?;
        Ok(typesense_response_to_search_results(response))
    }

    fn stream_search(index: IndexName, query: SearchQuery) -> Result<SearchStream, SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client()?;

        let stream = TypesenseSearchStream::new(client, index, query);

        let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            SearchStream::new(stream)
        })) {
            Ok(stream) => stream,
            Err(panic_info) => {
                if let Some(s) = panic_info.downcast_ref::<&str>() {
                    trace!("[DEBUG] Panic message: {s}");
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    trace!("[DEBUG] Panic message: {s}");
                } else {
                    trace!("[DEBUG] Panic message: <unknown>");
                }

                std::panic::resume_unwind(panic_info);
            }
        };

        Ok(result)
    }

    fn get_schema(index: IndexName) -> Result<Schema, SearchError> {
        let client = Self::create_client()?;

        // Typesense doesn't have a direct get schema endpoint for collections
        // We need to get the collection info from the list
        let collections = client.list_collections()?;

        let collection = collections
            .0
            .into_iter()
            .find(|c| c.name == index)
            .ok_or(SearchError::IndexNotFound)?;

        let schema = Schema {
            fields: collection
                .fields
                .into_iter()
                .map(collection_field_to_schema_field)
                .collect(),
            primary_key: collection.default_sorting_field,
        };

        Ok(schema)
    }

    fn update_schema(index: IndexName, schema: Schema) -> Result<(), SearchError> {
        // Typesense doesn't support updating schema after collection creation
        // We need to delete and recreate the collection
        let client = Self::create_client()?;

        let collections = client.list_collections()?;
        let exists = collections.0.iter().any(|c| c.name == index);

        if exists {
            client.delete_collection(&index)?;
        }

        let typesense_schema = schema_to_typesense_schema(schema, &index);
        client.create_collection(&index, &typesense_schema)?;

        Ok(())
    }
}

impl ExtendedGuest for TypesenseComponent {
    fn unwrapped_stream(index: IndexName, query: SearchQuery) -> Self::SearchStream {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        let client = Self::create_client().unwrap_or_else(|_e| {
            TypesenseSearchApi::new("dummy".to_string(), "http://localhost:8108".to_string())
        });

        let simplified_query = SearchQuery {
            q: query.q,
            filters: query.filters,
            sort: query.sort,
            facets: query.facets,
            page: Some(1),
            per_page: query.per_page.or(Some(20)),
            offset: None,
            highlight: None,
            config: query.config,
        };

        TypesenseSearchStream::new(client, index, simplified_query)
    }

    fn retry_query(original_query: &SearchQuery, partial_hits: &[SearchHit]) -> SearchQuery {
        trace!(
            "[DEBUG] retry_query called with {} partial hits",
            partial_hits.len()
        );

        let mut retry_query = original_query.clone();

        if !partial_hits.is_empty() {
            let current_offset = original_query.offset.unwrap_or(0);
            let received_count = partial_hits.len() as u32;
            retry_query.offset = Some(current_offset + received_count);
        }

        retry_query
    }

    fn subscribe(stream: &Self::SearchStream) -> Pollable {
        stream.subscribe()
    }
}

type DurableTypesenseComponent = DurableSearch<TypesenseComponent>;
golem_search::export_search!(DurableTypesenseComponent with_types_in golem_search);

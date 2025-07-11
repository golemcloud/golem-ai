use golem_search::{
    config::with_config_key, exports::golem::search::core::Guest, golem::search::types::{
        Doc, DocumentId, IndexName, Schema, SearchError, SearchHit, SearchQuery, SearchResults,
    }, LOGGING_STATE
};
use wit_bindgen_rt::async_support::StreamReader;

use crate::{
    client::{AlgoliaClient, IndexItem, ListIndexesQueryParams},
    conversions::{index_item_to_index_name, to_list_indices_request},
};

mod client;
mod conversions;

struct AlgoliaComponent;

impl AlgoliaComponent {
    const ENV_VAR_ALGOLIA_APP_ID: &'static str = "ALGOLIA_APP_ID";
    const ENV_VAR_ALGOLIA_API_KEY: &'static str = "ALGOLIA_API_KEY";

    fn delete_index(client: AlgoliaClient, index_name: IndexName) -> Result<(), SearchError> {
        client.delete_index(&index_name.to_string())
    }

    fn list_indexes(
        client: AlgoliaClient,
        hits_per_page: Option<u16>,
        page: Option<u16>,
    ) -> Result<Vec<IndexName>, SearchError> {
        let query_params = to_list_indices_request(hits_per_page, page);
        let result = client.list_indexes(query_params)?;
        Ok(index_item_to_index_name(result))
    }
}

impl Guest for AlgoliaComponent {
    #[doc = " Index lifecycle"]
    fn create_index(name: IndexName, schema: Option<Schema>) -> Result<(), SearchError> {
        todo!()
    }

    fn delete_index(name: IndexName) -> Result<(), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        with_config_key(&[Self::ENV_VAR_ALGOLIA_APP_ID, Self::ENV_VAR_ALGOLIA_API_KEY],  |err| Err(err), |keys| {
            let client = AlgoliaClient::new(&keys[Self::ENV_VAR_ALGOLIA_APP_ID], &keys[Self::ENV_VAR_ALGOLIA_API_KEY]);
            Self::delete_index(client, name)
        })
    }

    fn list_indexes(
        hits_per_page: Option<u16>,
        page: Option<u16>,
    ) -> Result<Vec<IndexName>, SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        with_config_key(&[Self::ENV_VAR_ALGOLIA_APP_ID, Self::ENV_VAR_ALGOLIA_API_KEY],  |err| Err(err), |keys| {
            let client = AlgoliaClient::new(&keys[Self::ENV_VAR_ALGOLIA_APP_ID], &keys[Self::ENV_VAR_ALGOLIA_API_KEY]);
            Self::list_indexes(client, hits_per_page, page)
        })
    }

    #[doc = " Document operations"]
    fn upsert(index: IndexName, doc: Doc) -> Result<(), SearchError> {
        todo!()
    }

    fn upsert_many(index: IndexName, docs: Vec<Doc>) -> Result<(), SearchError> {
        todo!()
    }

    fn delete(index: IndexName, id: DocumentId) -> Result<(), SearchError> {
        todo!()
    }

    fn delete_many(index: IndexName, ids: Vec<DocumentId>) -> Result<(), SearchError> {
        todo!()
    }

    fn get(index: IndexName, id: DocumentId) -> Result<Option<Doc>, SearchError> {
        todo!()
    }

    #[doc = " Query"]
    fn search(index: IndexName, query: SearchQuery) -> Result<SearchResults, SearchError> {
        todo!()
    }

    fn stream_search(
        index: IndexName,
        query: SearchQuery,
    ) -> Result<StreamReader<SearchHit>, SearchError> {
        todo!()
    }

    #[doc = " Schema inspection"]
    fn get_schema(index: IndexName) -> Result<Schema, SearchError> {
        todo!()
    }

    fn update_schema(index: IndexName, schema: Schema) -> Result<(), SearchError> {
        todo!()
    }
}

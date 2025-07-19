mod client;
mod conversions;

use std::cell::RefCell;

use crate::client::{BraveSearchApi, SearchRequest};
use crate::conversions::{params_to_request, response_to_results, validate_search_params};
use golem_web_search::golem::web_search::web_search::{
    Guest, GuestSearchSession, SearchError, SearchMetadata, SearchParams, SearchResult,
    SearchSession,
};

use golem_web_search::LOGGING_STATE;

struct BraveSearch {
    client: BraveSearchApi,
    request: SearchRequest,
    params: SearchParams,
    finished: bool,
    metadata: Option<SearchMetadata>,
    current_offset: u32,
}

impl BraveSearch {
    fn new(client: BraveSearchApi, request: SearchRequest, params: SearchParams) -> Self {
        Self {
            client,
            request,
            params,
            finished: false,
            metadata: None,
            current_offset: 0,
        }
    }

    fn next_page(&mut self) -> Result<Vec<SearchResult>, SearchError> {
        if self.finished {
            return Ok(vec![]);
        }

        // Update request with current offset
        let mut request = self.request.clone();
        request.offset = Some(self.current_offset);

        let response = self.client.search(request)?;
        let (results, metadata) = response_to_results(response, &self.params, self.current_offset);

        // Check if more results are available
        if let Some(ref meta) = metadata {
            let count = self.request.count.unwrap_or(10);
            let has_more_results = results.len() == (count as usize);
            let has_next_page = meta.next_page_token.is_some();
            self.finished = !has_more_results || !has_next_page;
            if !self.finished {
                self.current_offset += 1;
            }
        } else {
            self.finished = true;
        }

        self.metadata = metadata;
        Ok(results)
    }

    fn get_metadata(&self) -> Option<SearchMetadata> {
        self.metadata.clone()
    }
}

// Create a wrapper that implements GuestSearchSession properly
struct BraveSearchSession(RefCell<BraveSearch>);

impl BraveSearchSession {
    fn new(search: BraveSearch) -> Self {
        Self(RefCell::new(search))
    }
}

impl GuestSearchSession for BraveSearchSession {
    fn next_page(&self) -> Result<Vec<SearchResult>, SearchError> {
        let mut search = self.0.borrow_mut();
        search.next_page()
    }

    fn get_metadata(&self) -> Option<SearchMetadata> {
        let search = self.0.borrow();
        search.get_metadata()
    }
}

struct BraveSearchComponent;

impl BraveSearchComponent {
    const API_KEY_VAR: &'static str = "BRAVE_API_KEY";

    fn create_client() -> Result<BraveSearchApi, SearchError> {
        let api_key = Self::get_api_key()?;
        Ok(BraveSearchApi::new(api_key))
    }

    fn get_api_key() -> Result<String, SearchError> {
        std::env::var(Self::API_KEY_VAR).map_err(|_| {
            SearchError::BackendError("BRAVE_API_KEY environment variable not set".to_string())
        })
    }

    fn execute_search(
        params: SearchParams,
        api_key: String,
    ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
        validate_search_params(&params)?;

        let client = Self::create_client()?;
        let request = params_to_request(params.clone(), api_key.clone(), 0)?;

        let response = client.search(request)?;
        let (results, metadata) = response_to_results(response, &params, 0);

        Ok((results, metadata))
    }

    fn start_search_session(
        params: SearchParams,
        api_key: String,
    ) -> Result<BraveSearchSession, SearchError> {
        validate_search_params(&params)?;

        let client = Self::create_client()?;
        let request = params_to_request(params.clone(), api_key.clone(), 0)?;

        let search = BraveSearch::new(client, request, params);
        Ok(BraveSearchSession::new(search))
    }
}

impl Guest for BraveSearchComponent {
    type SearchSession = BraveSearchSession;

    fn start_search(params: SearchParams) -> Result<SearchSession, SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        match Self::start_search_session(params, Self::get_api_key()?) {
            Ok(session) => Ok(SearchSession::new(session)),
            Err(err) => Err(err),
        }
    }

    fn search_once(
        params: SearchParams,
    ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        Self::execute_search(params, Self::get_api_key()?)
    }
}

golem_web_search::export_websearch!(BraveSearchComponent with_types_in golem_web_search);

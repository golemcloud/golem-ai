mod client;
mod conversions;

use std::cell::RefCell;

use crate::client::{SearchRequest, SerperSearchApi};
use crate::conversions::{params_to_request, response_to_results, validate_search_params};
use golem_web_search::golem::web_search::web_search::{
    Guest, GuestSearchSession, SearchError, SearchMetadata, SearchParams, SearchResult,
    SearchSession,
};

use golem_web_search::LOGGING_STATE;

struct SerperSearch {
    client: SerperSearchApi,
    request: SearchRequest,
    params: SearchParams,
    finished: bool,
    metadata: Option<SearchMetadata>,
    current_page: u32,
}

impl SerperSearch {
    fn new(client: SerperSearchApi, request: SearchRequest, params: SearchParams) -> Self {
        Self {
            client,
            request,
            params,
            finished: false,
            metadata: None,
            current_page: 0,
        }
    }

    fn next_page(&mut self) -> Result<Vec<SearchResult>, SearchError> {
        if self.finished {
            return Ok(vec![]);
        }

        // Update request with current page
        let request =
            crate::conversions::params_to_request(self.params.clone(), self.current_page)?;

        let response = self.client.search(request)?;
        let (results, metadata) = response_to_results(response, &self.params, self.current_page);

        // Check if more results are available
        if let Some(ref meta) = metadata {
            let num_results = self.request.num.unwrap_or(10);
            let has_more_results = results.len() == (num_results as usize);
            let has_next_page = meta.next_page_token.is_some();
            self.finished = !has_more_results || !has_next_page;
            if !self.finished {
                self.current_page += 1;
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
struct SerperSearchSession(RefCell<SerperSearch>);

impl SerperSearchSession {
    fn new(search: SerperSearch) -> Self {
        Self(RefCell::new(search))
    }
}

impl GuestSearchSession for SerperSearchSession {
    fn next_page(&self) -> Result<Vec<SearchResult>, SearchError> {
        let mut search = self.0.borrow_mut();
        search.next_page()
    }

    fn get_metadata(&self) -> Option<SearchMetadata> {
        let search = self.0.borrow();
        search.get_metadata()
    }
}

struct SerperSearchComponent;

impl SerperSearchComponent {
    const API_KEY_VAR: &'static str = "SERPER_API_KEY";

    fn create_client() -> Result<SerperSearchApi, SearchError> {
        let api_key = std::env::var(Self::API_KEY_VAR).map_err(|_| {
            SearchError::BackendError("SERPER_API_KEY environment variable not set".to_string())
        })?;

        Ok(SerperSearchApi::new(api_key))
    }

    fn execute_search(
        params: SearchParams,
    ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
        validate_search_params(&params)?;

        let client = Self::create_client()?;
        let request = params_to_request(params.clone(), 1)?;

        let response = client.search(request)?;
        let (results, metadata) = response_to_results(response, &params, 1);

        Ok((results, metadata))
    }

    fn start_search_session(params: SearchParams) -> Result<SerperSearchSession, SearchError> {
        validate_search_params(&params)?;

        let client = Self::create_client()?;
        let request = params_to_request(params.clone(), 1)?;

        let search = SerperSearch::new(client, request, params);
        Ok(SerperSearchSession::new(search))
    }
}

impl Guest for SerperSearchComponent {
    type SearchSession = SerperSearchSession;

    fn start_search(params: SearchParams) -> Result<SearchSession, SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        match Self::start_search_session(params) {
            Ok(session) => Ok(SearchSession::new(session)),
            Err(err) => Err(err),
        }
    }

    fn search_once(
        params: SearchParams,
    ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        Self::execute_search(params)
    }
}

golem_web_search::export_websearch!(SerperSearchComponent with_types_in golem_web_search);

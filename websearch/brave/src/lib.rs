mod client;
mod conversions;

use std::cell::RefCell;

use crate::client::BraveSearchApi;
use crate::conversions::{params_to_request, response_to_results, validate_search_params};
use golem_web_search::durability::Durablewebsearch;
use golem_web_search::durability::ExtendedwebsearchGuest;
use golem_web_search::golem::web_search::web_search::{
    Guest, GuestSearchSession, SearchError, SearchMetadata, SearchParams, SearchResult,
    SearchSession,
};

// Define a custom ReplayState struct
#[derive(Debug, Clone, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct BraveReplayState {
    pub api_key: String,
    pub current_offset: u32,
    pub metadata: Option<SearchMetadata>,
    pub finished: bool,
}

struct BraveSearch {
    client: BraveSearchApi,
    params: SearchParams,
    metadata: Option<SearchMetadata>,
    current_offset: u32,
    finished: bool,
}

impl BraveSearch {
    fn new(client: BraveSearchApi, params: SearchParams) -> Self {
        Self {
            client,
            params,
            metadata: None,
            current_offset: 0,
            finished: false,
        }
    }

    fn next_page(&mut self) -> Result<Vec<SearchResult>, SearchError> {
        if self.finished {
            return Ok(Vec::new());
        }

        // Update request with current offset
        let request = crate::conversions::params_to_request(&self.params, self.current_offset)?;

        let response = self.client.search(request)?;
        let (results, metadata) = response_to_results(&response, &self.params, self.current_offset);

        self.finished = !response.query.more_results_available;
        self.current_offset += 1;
        self.metadata = Some(metadata);

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
    ) -> Result<(Vec<SearchResult>, SearchMetadata), SearchError> {
        validate_search_params(&params)?;

        let client = Self::create_client()?;
        let request = params_to_request(&params, 0)?;

        let response = client.search(request)?;
        let (results, metadata) = response_to_results(&response, &params, 0);

        Ok((results, metadata))
    }

    fn start_search_session(params: SearchParams) -> Result<BraveSearchSession, SearchError> {
        validate_search_params(&params)?;

        let client = Self::create_client()?;
        let search = BraveSearch::new(client, params);
        Ok(BraveSearchSession::new(search))
    }
}

impl Guest for BraveSearchComponent {
    type SearchSession = BraveSearchSession;

    fn start_search(params: SearchParams) -> Result<SearchSession, SearchError> {
        Self::start_search_session(params).map(SearchSession::new)
    }

    fn search_once(
        params: SearchParams,
    ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
        let (results, metadata) = Self::execute_search(params)?;
        Ok((results, Some(metadata)))
    }
}

// ExtendedwebsearchGuest implementation
impl ExtendedwebsearchGuest for BraveSearchComponent {
    type ReplayState = BraveReplayState;

    fn unwrapped_search_session(params: SearchParams) -> Result<Self::SearchSession, SearchError> {
        let api_key = Self::get_api_key()?;
        let client = BraveSearchApi::new(api_key.clone());
        let search = BraveSearch::new(client, params);
        Ok(BraveSearchSession::new(search))
    }

    fn session_to_state(session: &Self::SearchSession) -> Self::ReplayState {
        let search = session.0.borrow();
        BraveReplayState {
            api_key: search.client.api_key().clone(),
            current_offset: search.current_offset,
            metadata: search.metadata.clone(),
            finished: search.finished,
        }
    }

    fn session_from_state(
        state: &Self::ReplayState,
        params: SearchParams,
    ) -> Result<Self::SearchSession, SearchError> {
        let client = BraveSearchApi::new(state.api_key.clone());
        let mut search = BraveSearch::new(client, params);
        search.current_offset = state.current_offset;
        search.metadata = state.metadata.clone();
        search.finished = state.finished;
        Ok(BraveSearchSession::new(search))
    }
}

type DurableBraveComponent = Durablewebsearch<BraveSearchComponent>;
golem_web_search::export_websearch!(DurableBraveComponent with_types_in golem_web_search);

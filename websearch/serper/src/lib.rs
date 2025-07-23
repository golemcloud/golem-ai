mod client;
mod conversions;

use std::cell::RefCell;

use crate::client::SerperSearchApi;
use crate::conversions::{params_to_request, response_to_results, validate_search_params};
use golem_web_search::durability::Durablewebsearch;
use golem_web_search::durability::ExtendedwebsearchGuest;
use golem_web_search::golem::web_search::web_search::{
    Guest, GuestSearchSession, SearchError, SearchMetadata, SearchParams, SearchResult,
    SearchSession,
};

use golem_web_search::LOGGING_STATE;

#[derive(Debug, Clone, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct SerperReplayState {
    pub api_key: String,
    pub current_page: u32,
    pub metadata: Option<SearchMetadata>,
    pub finished: bool,
}

struct SerperSearch {
    client: SerperSearchApi,
    params: SearchParams,
    metadata: Option<SearchMetadata>,
    current_page: u32, // 1-based
    finished: bool,
}

impl SerperSearch {
    fn new(client: SerperSearchApi, params: SearchParams) -> Self {
        Self {
            client,
            params,
            metadata: None,
            current_page: 1, // 1-based
            finished: false,
        }
    }

    fn next_page(&mut self) -> Result<Vec<SearchResult>, SearchError> {
        if self.finished {
            return Ok(Vec::new());
        }

        let request =
            crate::conversions::params_to_request(self.params.clone(), self.current_page)?;
        let num_results = request.num.unwrap_or(10);
        let response = self.client.search(request)?;
        let (results, metadata) = response_to_results(response, &self.params, self.current_page);

        self.finished = results.len() < (num_results as usize);
        self.current_page += 1;
        self.metadata = Some(metadata);

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

    fn get_api_key() -> Result<String, SearchError> {
        std::env::var(Self::API_KEY_VAR).map_err(|_| {
            SearchError::BackendError("SERPER_API_KEY environment variable not set".to_string())
        })
    }

    fn create_client() -> Result<SerperSearchApi, SearchError> {
        let api_key = Self::get_api_key()?;
        Ok(SerperSearchApi::new(api_key))
    }

    fn execute_search(
        params: SearchParams,
    ) -> Result<(Vec<SearchResult>, SearchMetadata), SearchError> {
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
        let search = SerperSearch::new(client, params);
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
        let (results, metadata) = Self::execute_search(params)?;
        Ok((results, Some(metadata)))
    }
}

impl ExtendedwebsearchGuest for SerperSearchComponent {
    type ReplayState = SerperReplayState;

    fn unwrapped_search_session(params: SearchParams) -> Result<Self::SearchSession, SearchError> {
        let client = Self::create_client()?;
        let search = SerperSearch::new(client, params);
        Ok(SerperSearchSession::new(search))
    }

    fn session_to_state(session: &Self::SearchSession) -> Self::ReplayState {
        let search = session.0.borrow_mut();
        SerperReplayState {
            api_key: search.client.api_key().to_string(),
            current_page: search.current_page,
            metadata: search.metadata.clone(),
            finished: search.finished,
        }
    }

    fn session_from_state(
        state: &Self::ReplayState,
        params: SearchParams,
    ) -> Result<Self::SearchSession, SearchError> {
        let client = SerperSearchApi::new(state.api_key.clone());
        let mut search = SerperSearch::new(client, params);
        search.current_page = state.current_page;
        search.metadata = state.metadata.clone();
        search.finished = state.finished;
        Ok(SerperSearchSession::new(search))
    }
}

type DurableSerperComponent = Durablewebsearch<SerperSearchComponent>;
golem_web_search::export_websearch!(DurableSerperComponent with_types_in golem_web_search);

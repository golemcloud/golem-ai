mod client;
mod conversions;

use crate::client::GoogleSearchApi;
use crate::conversions::{params_to_request, response_to_results, validate_search_params};
use golem_web_search::durability::Durablewebsearch;
use golem_web_search::durability::ExtendedwebsearchGuest;
use golem_web_search::golem::web_search::web_search::{
    Guest, GuestSearchSession, SearchError, SearchMetadata, SearchParams, SearchResult,
    SearchSession,
};
use golem_web_search::LOGGING_STATE;
use std::cell::RefCell;

/// Start index for google search api pagination (which is 1-index based)
const INITIAL_START_INDEX: u32 = 1;

#[derive(Debug, Clone, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct GoogleReplayState {
    pub api_key: String,
    pub search_engine_id: String,
    pub current_page: u32,
    pub next_page_start_index: Option<u32>,
    pub metadata: Option<SearchMetadata>,
    pub finished: bool,
}

struct GoogleSearch {
    client: GoogleSearchApi,
    params: SearchParams,
    metadata: Option<SearchMetadata>,
    current_page: u32,
    next_page_start_index: Option<u32>,
    finished: bool,
}

impl GoogleSearch {
    fn new(client: GoogleSearchApi, params: SearchParams) -> Self {
        Self {
            client,
            params,
            metadata: None,
            current_page: 0,
            next_page_start_index: None,
            finished: false,
        }
    }

    fn next_page(&mut self) -> Result<Vec<SearchResult>, SearchError> {
        if self.finished {
            return Ok(Vec::new());
        }

        let current_start = self.next_page_start_index.unwrap_or(INITIAL_START_INDEX);
        let request = crate::conversions::params_to_request(&self.params, current_start)?;
        let response = self.client.search(request)?;

        let (results, metadata) = response_to_results(&response, &self.params, self.current_page);

        self.finished = response.next_page.is_none();
        self.current_page += 1;
        self.next_page_start_index = response.next_page.map(|np| np.start_index);
        self.metadata = Some(metadata);
        Ok(results)
    }

    fn get_metadata(&self) -> Option<SearchMetadata> {
        self.metadata.clone()
    }
}

// Create a wrapper that implements GuestSearchSession properly
struct GoogleSearchSession(RefCell<GoogleSearch>);

impl GoogleSearchSession {
    fn new(search: GoogleSearch) -> Self {
        Self(RefCell::new(search))
    }
}

impl GuestSearchSession for GoogleSearchSession {
    fn next_page(&self) -> Result<Vec<SearchResult>, SearchError> {
        let mut search = self.0.borrow_mut();
        search.next_page()
    }

    fn get_metadata(&self) -> Option<SearchMetadata> {
        let search = self.0.borrow();
        search.get_metadata()
    }
}

struct GoogleCustomSearchComponent;

impl GoogleCustomSearchComponent {
    const API_KEY_VAR: &'static str = "GOOGLE_API_KEY";
    const SEARCH_ENGINE_ID_VAR: &'static str = "GOOGLE_SEARCH_ENGINE_ID";

    fn create_client() -> Result<GoogleSearchApi, SearchError> {
        let api_key = std::env::var(Self::API_KEY_VAR).map_err(|_| {
            SearchError::BackendError("GOOGLE_API_KEY environment variable not set".to_string())
        })?;

        let search_engine_id = std::env::var(Self::SEARCH_ENGINE_ID_VAR).map_err(|_| {
            SearchError::BackendError(
                "GOOGLE_SEARCH_ENGINE_ID environment variable not set".to_string(),
            )
        })?;

        Ok(GoogleSearchApi::new(api_key, search_engine_id))
    }

    fn execute_search(
        params: SearchParams,
    ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
        validate_search_params(&params)?;

        let client = Self::create_client()?;
        let request = params_to_request(&params, INITIAL_START_INDEX)?;

        let response = client.search(request)?;
        let (results, metadata) = response_to_results(&response, &params, 0);

        Ok((results, Some(metadata)))
    }

    fn start_search_session(params: SearchParams) -> Result<GoogleSearchSession, SearchError> {
        validate_search_params(&params)?;

        let client = Self::create_client()?;
        let search = GoogleSearch::new(client, params);
        Ok(GoogleSearchSession::new(search))
    }
}

impl Guest for GoogleCustomSearchComponent {
    type SearchSession = GoogleSearchSession;

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

impl ExtendedwebsearchGuest for GoogleCustomSearchComponent {
    type ReplayState = GoogleReplayState;

    fn unwrapped_search_session(params: SearchParams) -> Result<Self::SearchSession, SearchError> {
        let client = Self::create_client()?;
        let search = GoogleSearch::new(client, params);
        Ok(GoogleSearchSession::new(search))
    }

    fn session_to_state(session: &Self::SearchSession) -> Self::ReplayState {
        let search = session.0.borrow_mut();
        GoogleReplayState {
            api_key: search.client.api_key().to_string(),
            search_engine_id: search.client.search_engine_id().to_string(),
            current_page: search.current_page,
            next_page_start_index: search.next_page_start_index,
            metadata: search.metadata.clone(),
            finished: search.finished,
        }
    }

    fn session_from_state(
        state: &Self::ReplayState,
        params: SearchParams,
    ) -> Result<Self::SearchSession, SearchError> {
        let client = GoogleSearchApi::new(state.api_key.clone(), state.search_engine_id.clone());
        let mut search = GoogleSearch::new(client, params);
        search.current_page = state.current_page;
        search.next_page_start_index = state.next_page_start_index;
        search.metadata = state.metadata.clone();
        search.finished = state.finished;

        Ok(GoogleSearchSession::new(search))
    }
}

type DurableGoogleComponent = Durablewebsearch<GoogleCustomSearchComponent>;
golem_web_search::export_websearch!(DurableGoogleComponent with_types_in golem_web_search);

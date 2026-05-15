mod client;
pub mod config;
mod conversions;

use crate::client::GoogleSearchApi;
use crate::conversions::{params_to_request, response_to_results, validate_search_params};
use golem_ai_web_search::durability::DurableWebSearch;
use golem_ai_web_search::durability::ExtendedWebSearchProvider;
use golem_ai_web_search::model::web_search::{
    SearchError, SearchMetadata, SearchParams, SearchResult, SearchSession,
};
use golem_ai_web_search::{SearchSessionInterface, WebSearchProvider};
use std::cell::RefCell;

pub use config::GoogleConfig;
#[cfg(feature = "golem")]
pub use config::GoogleHostConfig;

/// Start index for google search api pagination (which is 1-index based)
const INITIAL_START_INDEX: u32 = 1;

#[cfg(feature = "golem")]
#[derive(Debug, Clone, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct GoogleReplayState {
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
        let request = params_to_request(&self.params, current_start)?;
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
pub struct GoogleSearchSession(RefCell<GoogleSearch>);

impl GoogleSearchSession {
    fn new(search: GoogleSearch) -> Self {
        Self(RefCell::new(search))
    }
}

impl SearchSessionInterface for GoogleSearchSession {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn next_page(&self) -> Result<Vec<SearchResult>, SearchError> {
        let mut search = self.0.borrow_mut();
        search.next_page()
    }

    fn get_metadata(&self) -> Option<SearchMetadata> {
        let search = self.0.borrow();
        search.get_metadata()
    }
}

pub struct GoogleCustomSearch;

impl GoogleCustomSearch {
    fn execute_search(
        provider_config: &GoogleConfig,
        params: SearchParams,
    ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
        validate_search_params(&params)?;

        let client = GoogleSearchApi::new(provider_config);
        let request = params_to_request(&params, INITIAL_START_INDEX)?;

        let response = client.search(request)?;
        let (results, metadata) = response_to_results(&response, &params, 0);

        Ok((results, Some(metadata)))
    }

    fn start_search_session(
        provider_config: &GoogleConfig,
        params: SearchParams,
    ) -> Result<GoogleSearchSession, SearchError> {
        validate_search_params(&params)?;

        let client = GoogleSearchApi::new(provider_config);
        let search = GoogleSearch::new(client, params);
        Ok(GoogleSearchSession::new(search))
    }
}

impl WebSearchProvider for GoogleCustomSearch {
    type SearchSession = GoogleSearchSession;
    type ProviderConfig = GoogleConfig;

    fn start_search(
        provider_config: Self::ProviderConfig,
        params: SearchParams,
    ) -> Result<SearchSession, SearchError> {
        match Self::start_search_session(&provider_config, params) {
            Ok(session) => Ok(SearchSession::new(session)),
            Err(err) => Err(err),
        }
    }

    fn search_once(
        provider_config: Self::ProviderConfig,
        params: SearchParams,
    ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
        Self::execute_search(&provider_config, params)
    }
}

#[cfg(feature = "golem")]
impl ExtendedWebSearchProvider for GoogleCustomSearch {
    type ReplayState = GoogleReplayState;

    fn unwrapped_search_session(
        provider_config: Self::ProviderConfig,
        params: SearchParams,
    ) -> Result<Self::SearchSession, SearchError> {
        let client = GoogleSearchApi::new(&provider_config);
        let search = GoogleSearch::new(client, params);
        Ok(GoogleSearchSession::new(search))
    }

    fn session_to_state(session: &Self::SearchSession) -> Self::ReplayState {
        let search = session.0.borrow_mut();
        GoogleReplayState {
            current_page: search.current_page,
            next_page_start_index: search.next_page_start_index,
            metadata: search.metadata.clone(),
            finished: search.finished,
        }
    }

    fn session_from_state(
        provider_config: Self::ProviderConfig,
        state: &Self::ReplayState,
        params: SearchParams,
    ) -> Result<Self::SearchSession, SearchError> {
        let client = GoogleSearchApi::new(&provider_config);
        let mut search = GoogleSearch::new(client, params);
        search.current_page = state.current_page;
        search.next_page_start_index = state.next_page_start_index;
        search.metadata = state.metadata.clone();
        search.finished = state.finished;

        Ok(GoogleSearchSession::new(search))
    }
}

#[cfg(not(feature = "golem"))]
impl ExtendedWebSearchProvider for GoogleCustomSearch {}

pub type DurableGoogleCustomSearch = DurableWebSearch<GoogleCustomSearch>;

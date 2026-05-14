mod client;
pub mod config;
mod conversions;

use std::cell::RefCell;

use crate::client::BraveSearchApi;
use crate::conversions::{params_to_request, response_to_results, validate_search_params};
use golem_ai_web_search::durability::DurableWebSearch;
use golem_ai_web_search::durability::ExtendedWebSearchProvider;
use golem_ai_web_search::model::web_search::{
    SearchError, SearchMetadata, SearchParams, SearchResult, SearchSession,
};
use golem_ai_web_search::{SearchSessionInterface, WebSearchProvider};

pub use config::BraveConfig;
#[cfg(feature = "golem")]
pub use config::BraveHostConfig;

// Define a custom ReplayState struct
#[cfg(feature = "golem")]
#[derive(Debug, Clone, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct BraveReplayState {
    pub current_offset: u32,
    pub metadata: Option<SearchMetadata>,
    pub finished: bool,
}

struct BraveSearchSessionImpl {
    client: BraveSearchApi,
    params: SearchParams,
    metadata: Option<SearchMetadata>,
    current_offset: u32,
    finished: bool,
}

impl BraveSearchSessionImpl {
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
        let request = params_to_request(&self.params, self.current_offset)?;

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
pub struct BraveSearchSession(RefCell<BraveSearchSessionImpl>);

impl BraveSearchSession {
    fn new(search: BraveSearchSessionImpl) -> Self {
        Self(RefCell::new(search))
    }
}

impl SearchSessionInterface for BraveSearchSession {
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

pub struct BraveSearch;

impl BraveSearch {
    fn execute_search(
        provider_config: &BraveConfig,
        params: SearchParams,
    ) -> Result<(Vec<SearchResult>, SearchMetadata), SearchError> {
        validate_search_params(&params)?;

        let client = BraveSearchApi::new(provider_config);
        let request = params_to_request(&params, 0)?;

        let response = client.search(request)?;
        let (results, metadata) = response_to_results(&response, &params, 0);

        Ok((results, metadata))
    }

    fn start_search_session(
        provider_config: &BraveConfig,
        params: SearchParams,
    ) -> Result<BraveSearchSession, SearchError> {
        validate_search_params(&params)?;

        let client = BraveSearchApi::new(provider_config);
        let search = BraveSearchSessionImpl::new(client, params);
        Ok(BraveSearchSession::new(search))
    }
}

impl WebSearchProvider for BraveSearch {
    type SearchSession = BraveSearchSession;
    type ProviderConfig = BraveConfig;

    fn start_search(
        provider_config: Self::ProviderConfig,
        params: SearchParams,
    ) -> Result<SearchSession, SearchError> {
        Self::start_search_session(&provider_config, params).map(SearchSession::new)
    }

    fn search_once(
        provider_config: Self::ProviderConfig,
        params: SearchParams,
    ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
        let (results, metadata) = Self::execute_search(&provider_config, params)?;
        Ok((results, Some(metadata)))
    }
}

// ExtendedWebSearchProvider implementation
#[cfg(feature = "golem")]
impl ExtendedWebSearchProvider for BraveSearch {
    type ReplayState = BraveReplayState;

    fn unwrapped_search_session(
        provider_config: Self::ProviderConfig,
        params: SearchParams,
    ) -> Result<Self::SearchSession, SearchError> {
        let client = BraveSearchApi::new(&provider_config);
        let search = BraveSearchSessionImpl::new(client, params);
        Ok(BraveSearchSession::new(search))
    }

    fn session_to_state(session: &Self::SearchSession) -> Self::ReplayState {
        let search = session.0.borrow();
        BraveReplayState {
            current_offset: search.current_offset,
            metadata: search.metadata.clone(),
            finished: search.finished,
        }
    }

    fn session_from_state(
        provider_config: Self::ProviderConfig,
        state: &Self::ReplayState,
        params: SearchParams,
    ) -> Result<Self::SearchSession, SearchError> {
        let client = BraveSearchApi::new(&provider_config);
        let mut search = BraveSearchSessionImpl::new(client, params);
        search.current_offset = state.current_offset;
        search.metadata = state.metadata.clone();
        search.finished = state.finished;
        Ok(BraveSearchSession::new(search))
    }
}

#[cfg(not(feature = "golem"))]
impl ExtendedWebSearchProvider for BraveSearch {}

pub type DurableBraveSearch = DurableWebSearch<BraveSearch>;

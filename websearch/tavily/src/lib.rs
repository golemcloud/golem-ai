mod client;
mod conversions;

use crate::client::TavilySearchApi;
use crate::conversions::{params_to_request, response_to_results, validate_search_params};
use golem_web_search::durability::DurableWebSearch;
use golem_web_search::durability::ExtendedWebSearchProvider;
use golem_web_search::model::web_search::{
    SearchError, SearchMetadata, SearchParams, SearchResult, SearchSession,
};
use golem_web_search::{SearchSessionInterface, WebSearchProvider};
use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct TavilyReplayState {
    pub api_key: String,
    pub metadata: Option<SearchMetadata>,
    pub finished: bool,
}

struct TavilySearchSessionImpl {
    client: TavilySearchApi,
    params: SearchParams,
    metadata: Option<SearchMetadata>,
    finished: bool,
}

impl TavilySearchSessionImpl {
    fn new(client: TavilySearchApi, params: SearchParams) -> Self {
        Self {
            client,
            params,
            metadata: None,
            finished: false,
        }
    }

    fn next_page(&mut self) -> Result<Vec<SearchResult>, SearchError> {
        if self.finished {
            return Ok(Vec::new());
        }

        let request = params_to_request(&self.params)?;
        let response = self.client.search(request)?;
        let (results, metadata) = response_to_results(response, &self.params);

        self.finished = true;
        self.metadata = Some(metadata);
        Ok(results)
    }

    fn get_metadata(&self) -> Option<SearchMetadata> {
        self.metadata.clone()
    }
}

// Create a wrapper that implements GuestSearchSession properly
pub struct TavilySearchSession(RefCell<TavilySearchSessionImpl>);

impl TavilySearchSession {
    fn new(search: TavilySearchSessionImpl) -> Self {
        Self(RefCell::new(search))
    }
}

impl SearchSessionInterface for TavilySearchSession {
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

pub struct TavilySearch;

impl TavilySearch {
    const API_KEY_VAR: &'static str = "TAVILY_API_KEY";

    fn create_client() -> Result<TavilySearchApi, SearchError> {
        let api_key = Self::get_api_key()?;
        Ok(TavilySearchApi::new(api_key))
    }

    fn get_api_key() -> Result<String, SearchError> {
        std::env::var(Self::API_KEY_VAR).map_err(|_| {
            SearchError::BackendError("TAVILY_API_KEY environment variable not set".to_string())
        })
    }

    fn execute_search(
        params: SearchParams,
    ) -> Result<(Vec<SearchResult>, SearchMetadata), SearchError> {
        validate_search_params(&params)?;

        let client = Self::create_client()?;
        let request = params_to_request(&params)?;

        let response = client.search(request)?;
        let (results, metadata) = response_to_results(response, &params);

        // Unwrap the metadata Option since we know it should be Some
        Ok((results, metadata))
    }

    fn start_search_session(params: SearchParams) -> Result<TavilySearchSession, SearchError> {
        validate_search_params(&params)?;

        let client = Self::create_client()?;
        let search = TavilySearchSessionImpl::new(client, params);
        Ok(TavilySearchSession::new(search))
    }
}

impl WebSearchProvider for TavilySearch {
    type SearchSession = TavilySearchSession;

    fn start_search(params: SearchParams) -> Result<SearchSession, SearchError> {
        match Self::start_search_session(params) {
            Ok(session) => Ok(SearchSession::new(session)),
            Err(err) => Err(err),
        }
    }

    fn search_once(
        params: SearchParams,
    ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
        let (results, metadata) = Self::execute_search(params)?;
        Ok((results, Some(metadata)))
    }
}

impl ExtendedWebSearchProvider for TavilySearch {
    type ReplayState = TavilyReplayState;

    fn unwrapped_search_session(params: SearchParams) -> Result<Self::SearchSession, SearchError> {
        let client = Self::create_client()?;
        let search = TavilySearchSessionImpl::new(client, params);
        Ok(TavilySearchSession::new(search))
    }

    fn session_to_state(session: &Self::SearchSession) -> Self::ReplayState {
        let search = session.0.borrow_mut();
        TavilyReplayState {
            api_key: search.client.api_key().to_string(),
            metadata: search.metadata.clone(),
            finished: search.finished,
        }
    }
    fn session_from_state(
        state: &Self::ReplayState,
        params: SearchParams,
    ) -> Result<Self::SearchSession, SearchError> {
        let client = TavilySearchApi::new(state.api_key.clone());
        let mut search = TavilySearchSessionImpl::new(client, params);
        search.metadata = state.metadata.clone();
        search.finished = state.finished;
        Ok(TavilySearchSession::new(search))
    }
}

pub type DurableTavilySearch = DurableWebSearch<TavilySearch>;

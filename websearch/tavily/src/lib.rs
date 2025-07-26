mod client;
mod conversions;

use crate::client::TavilySearchApi;
use crate::conversions::{params_to_request, response_to_results, validate_search_params};
use golem_web_search::durability::Durablewebsearch;
use golem_web_search::durability::ExtendedwebsearchGuest;
use golem_web_search::golem::web_search::web_search::{
    Guest, GuestSearchSession, SearchError, SearchMetadata, SearchParams, SearchResult,
    SearchSession,
};
use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct TavilyReplayState {
    pub api_key: String,
    pub metadata: Option<SearchMetadata>,
    pub finished: bool,
}

struct TavilySearch {
    client: TavilySearchApi,
    params: SearchParams,
    metadata: Option<SearchMetadata>,
    finished: bool,
}

impl TavilySearch {
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

        let request = crate::conversions::params_to_request(&self.params)?;
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
struct TavilySearchSession(RefCell<TavilySearch>);

impl TavilySearchSession {
    fn new(search: TavilySearch) -> Self {
        Self(RefCell::new(search))
    }
}

impl GuestSearchSession for TavilySearchSession {
    fn next_page(&self) -> Result<Vec<SearchResult>, SearchError> {
        let mut search = self.0.borrow_mut();
        search.next_page()
    }
    fn get_metadata(&self) -> Option<SearchMetadata> {
        let search = self.0.borrow();
        search.get_metadata()
    }
}

struct TavilySearchComponent;

impl TavilySearchComponent {
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
        let search = TavilySearch::new(client, params);
        Ok(TavilySearchSession::new(search))
    }
}

impl Guest for TavilySearchComponent {
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

impl ExtendedwebsearchGuest for TavilySearchComponent {
    type ReplayState = TavilyReplayState;

    fn unwrapped_search_session(params: SearchParams) -> Result<Self::SearchSession, SearchError> {
        let client = Self::create_client()?;
        let search = TavilySearch::new(client, params);
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
        let mut search = TavilySearch::new(client, params);
        search.metadata = state.metadata.clone();
        search.finished = state.finished;
        Ok(TavilySearchSession::new(search))
    }
}

type DurableTavilyComponent = Durablewebsearch<TavilySearchComponent>;
golem_web_search::export_websearch!(DurableTavilyComponent with_types_in golem_web_search);

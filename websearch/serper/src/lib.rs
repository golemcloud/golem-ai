mod client;
pub mod config;
mod conversions;

use std::cell::RefCell;

use crate::client::SerperSearchApi;
use crate::conversions::{params_to_request, response_to_results, validate_search_params};
use golem_ai_web_search::durability::DurableWebSearch;
use golem_ai_web_search::durability::ExtendedWebSearchProvider;
use golem_ai_web_search::model::web_search::{
    SearchError, SearchMetadata, SearchParams, SearchResult, SearchSession,
};
use golem_ai_web_search::{SearchPageFuture, SearchSessionInterface, WebSearchProvider};

pub use config::SerperConfig;
#[cfg(feature = "golem")]
pub use config::SerperHostConfig;

#[cfg(feature = "golem")]
#[derive(Debug, Clone, PartialEq, golem_rust::FromSchema, golem_rust::IntoSchema)]
pub struct SerperReplayState {
    pub current_page: u32,
    pub metadata: Option<SearchMetadata>,
    pub finished: bool,
}

struct SerperSearchSessionImpl {
    client: SerperSearchApi,
    params: SearchParams,
    metadata: Option<SearchMetadata>,
    current_page: u32, // 1-based
    finished: bool,
}

impl SerperSearchSessionImpl {
    fn new(client: SerperSearchApi, params: SearchParams) -> Self {
        Self {
            client,
            params,
            metadata: None,
            current_page: 1, // 1-based
            finished: false,
        }
    }

    fn get_metadata(&self) -> Option<SearchMetadata> {
        self.metadata.clone()
    }
}

// Create a wrapper that implements GuestSearchSession properly
pub struct SerperSearchSession(RefCell<SerperSearchSessionImpl>);

impl SerperSearchSession {
    fn new(search: SerperSearchSessionImpl) -> Self {
        Self(RefCell::new(search))
    }
}

impl SearchSessionInterface for SerperSearchSession {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn next_page(&self) -> SearchPageFuture<'_> {
        Box::pin(async move {
            let (client, params, current_page) = {
                let search = self.0.borrow();
                if search.finished {
                    return Ok(Vec::new());
                }
                (
                    search.client.clone(),
                    search.params.clone(),
                    search.current_page,
                )
            };

            let request = params_to_request(params.clone(), current_page)?;
            let num_results = request.num.unwrap_or(10);
            let response = client.search(request).await?;
            let (results, metadata) = response_to_results(response, &params, current_page);

            let mut search = self.0.borrow_mut();
            search.finished = results.len() < (num_results as usize);
            search.current_page += 1;
            search.metadata = Some(metadata);

            Ok(results)
        })
    }

    fn get_metadata(&self) -> Option<SearchMetadata> {
        let search = self.0.borrow();
        search.get_metadata()
    }
}

pub struct SerperSearch;

impl SerperSearch {
    async fn execute_search(
        provider_config: &SerperConfig,
        params: SearchParams,
    ) -> Result<(Vec<SearchResult>, SearchMetadata), SearchError> {
        validate_search_params(&params)?;

        let client = SerperSearchApi::new(provider_config);
        let request = params_to_request(params.clone(), 1)?;

        let response = client.search(request).await?;
        let (results, metadata) = response_to_results(response, &params, 1);

        Ok((results, metadata))
    }

    fn start_search_session(
        provider_config: &SerperConfig,
        params: SearchParams,
    ) -> Result<SerperSearchSession, SearchError> {
        validate_search_params(&params)?;

        let client = SerperSearchApi::new(provider_config);
        let search = SerperSearchSessionImpl::new(client, params);
        Ok(SerperSearchSession::new(search))
    }
}

impl WebSearchProvider for SerperSearch {
    type SearchSession = SerperSearchSession;
    type ProviderConfig = SerperConfig;

    fn start_search(
        provider_config: Self::ProviderConfig,
        params: SearchParams,
    ) -> Result<SearchSession, SearchError> {
        match Self::start_search_session(&provider_config, params) {
            Ok(session) => Ok(SearchSession::new(session)),
            Err(err) => Err(err),
        }
    }

    async fn search_once(
        provider_config: Self::ProviderConfig,
        params: SearchParams,
    ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
        let (results, metadata) = Self::execute_search(&provider_config, params).await?;
        Ok((results, Some(metadata)))
    }
}

#[cfg(feature = "golem")]
impl ExtendedWebSearchProvider for SerperSearch {
    type ReplayState = SerperReplayState;

    fn unwrapped_search_session(
        provider_config: Self::ProviderConfig,
        params: SearchParams,
    ) -> Result<Self::SearchSession, SearchError> {
        let client = SerperSearchApi::new(&provider_config);
        let search = SerperSearchSessionImpl::new(client, params);
        Ok(SerperSearchSession::new(search))
    }

    fn session_to_state(session: &Self::SearchSession) -> Self::ReplayState {
        let search = session.0.borrow_mut();
        SerperReplayState {
            current_page: search.current_page,
            metadata: search.metadata.clone(),
            finished: search.finished,
        }
    }

    fn session_from_state(
        provider_config: Self::ProviderConfig,
        state: &Self::ReplayState,
        params: SearchParams,
    ) -> Result<Self::SearchSession, SearchError> {
        let client = SerperSearchApi::new(&provider_config);
        let mut search = SerperSearchSessionImpl::new(client, params);
        search.current_page = state.current_page;
        search.metadata = state.metadata.clone();
        search.finished = state.finished;
        Ok(SerperSearchSession::new(search))
    }
}

#[cfg(not(feature = "golem"))]
impl ExtendedWebSearchProvider for SerperSearch {}

pub type DurableSerperSearch = DurableWebSearch<SerperSearch>;

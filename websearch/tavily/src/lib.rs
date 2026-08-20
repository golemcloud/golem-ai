mod client;
pub mod config;
mod conversions;

use crate::client::TavilySearchApi;
use crate::conversions::{params_to_request, response_to_results, validate_search_params};
use golem_ai_web_search::durability::DurableWebSearch;
use golem_ai_web_search::durability::ExtendedWebSearchProvider;
use golem_ai_web_search::model::web_search::{
    SearchError, SearchMetadata, SearchParams, SearchResult, SearchSession,
};
use golem_ai_web_search::{SearchPageFuture, SearchSessionInterface, WebSearchProvider};
use std::cell::RefCell;

pub use config::TavilyConfig;
#[cfg(feature = "golem")]
pub use config::TavilyHostConfig;

#[cfg(feature = "golem")]
#[derive(Debug, Clone, PartialEq, golem_rust::FromSchema, golem_rust::IntoSchema)]
pub struct TavilyReplayState {
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

    fn next_page(&self) -> SearchPageFuture<'_> {
        Box::pin(async move {
            let (client, params) = {
                let search = self.0.borrow();
                if search.finished {
                    return Ok(Vec::new());
                }
                (search.client.clone(), search.params.clone())
            };

            let request = params_to_request(&params)?;
            let response = client.search(request).await?;
            let (results, metadata) = response_to_results(response, &params);

            let mut search = self.0.borrow_mut();
            search.finished = true;
            search.metadata = Some(metadata);
            Ok(results)
        })
    }
    fn get_metadata(&self) -> Option<SearchMetadata> {
        let search = self.0.borrow();
        search.get_metadata()
    }
}

pub struct TavilySearch;

impl TavilySearch {
    async fn execute_search(
        provider_config: &TavilyConfig,
        params: SearchParams,
    ) -> Result<(Vec<SearchResult>, SearchMetadata), SearchError> {
        validate_search_params(&params)?;

        let client = TavilySearchApi::new(provider_config);
        let request = params_to_request(&params)?;

        let response = client.search(request).await?;
        let (results, metadata) = response_to_results(response, &params);

        // Unwrap the metadata Option since we know it should be Some
        Ok((results, metadata))
    }

    fn start_search_session(
        provider_config: &TavilyConfig,
        params: SearchParams,
    ) -> Result<TavilySearchSession, SearchError> {
        validate_search_params(&params)?;

        let client = TavilySearchApi::new(provider_config);
        let search = TavilySearchSessionImpl::new(client, params);
        Ok(TavilySearchSession::new(search))
    }
}

impl WebSearchProvider for TavilySearch {
    type SearchSession = TavilySearchSession;
    type ProviderConfig = TavilyConfig;

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
impl ExtendedWebSearchProvider for TavilySearch {
    type ReplayState = TavilyReplayState;

    fn unwrapped_search_session(
        provider_config: Self::ProviderConfig,
        params: SearchParams,
    ) -> Result<Self::SearchSession, SearchError> {
        let client = TavilySearchApi::new(&provider_config);
        let search = TavilySearchSessionImpl::new(client, params);
        Ok(TavilySearchSession::new(search))
    }

    fn session_to_state(session: &Self::SearchSession) -> Self::ReplayState {
        let search = session.0.borrow_mut();
        TavilyReplayState {
            metadata: search.metadata.clone(),
            finished: search.finished,
        }
    }
    fn session_from_state(
        provider_config: Self::ProviderConfig,
        state: &Self::ReplayState,
        params: SearchParams,
    ) -> Result<Self::SearchSession, SearchError> {
        let client = TavilySearchApi::new(&provider_config);
        let mut search = TavilySearchSessionImpl::new(client, params);
        search.metadata = state.metadata.clone();
        search.finished = state.finished;
        Ok(TavilySearchSession::new(search))
    }
}

#[cfg(not(feature = "golem"))]
impl ExtendedWebSearchProvider for TavilySearch {}

pub type DurableTavilySearch = DurableWebSearch<TavilySearch>;

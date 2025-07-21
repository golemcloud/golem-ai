use crate::client::ExaSearchApi;
use crate::conversions::{convert_params_to_request, convert_response_to_results};
use golem_web_search::config::with_config_key;
use golem_web_search::durability::{DurableWebSearch, ExtendedGuest};
use golem_web_search::golem::web_search::types::{
    SearchError, SearchMetadata, SearchParams, SearchResult,
};
use golem_web_search::golem_web_search::web_search::web_search::{
    Guest, GuestSearchSession, SearchSession,
};
use golem_web_search::LOGGING_STATE;
use std::cell::RefCell;

mod client;
mod conversions;

struct ExaWebSearchComponent;

impl ExaWebSearchComponent {
    const API_KEY_ENV_VAR: &'static str = "EXA_API_KEY";
}

pub struct ExaSearchSession {
    client: ExaSearchApi,
    params: SearchParams,
    current_offset: RefCell<u32>,
    last_metadata: RefCell<Option<SearchMetadata>>,
    has_more_results: RefCell<bool>,
}

impl ExaSearchSession {
    fn new(client: ExaSearchApi, params: SearchParams) -> Self {
        Self {
            client,
            params,
            current_offset: RefCell::new(0),
            last_metadata: RefCell::new(None),
            has_more_results: RefCell::new(true),
        }
    }
}

impl GuestSearchSession for ExaSearchSession {
    fn next_page(&self) -> Result<SearchResult, SearchError> {
        if !*self.has_more_results.borrow() {
            return Err(SearchError::BackendError(
                "No more results available".to_string(),
            ));
        }

        let current_offset = *self.current_offset.borrow();
        
        // Exa has a limit of 100 results per request, so we'll limit pagination
        if current_offset >= 50 {  // Reasonable limit for pagination
            *self.has_more_results.borrow_mut() = false;
            return Err(SearchError::BackendError(
                "Maximum pagination limit reached".to_string(),
            ));
        }

        // Increment offset for next page
        let new_offset = current_offset + 1;
        *self.current_offset.borrow_mut() = new_offset;

        // Create request with current offset to simulate pagination
        // Since Exa doesn't support traditional offset, we'll use numResults and modify approach
        let request = convert_params_to_request(&self.params, Some(current_offset));
        let response = self.client.search(request)?;
        let (results, metadata) = convert_response_to_results(response, &self.params);

        *self.last_metadata.borrow_mut() = metadata.clone();

        if results.is_empty() {
            *self.has_more_results.borrow_mut() = false;
            return Err(SearchError::BackendError("No more results".to_string()));
        }

        // For Exa, limit pagination attempts
        if new_offset >= 10 {  // Limit to 10 pages 
            *self.has_more_results.borrow_mut() = false;
        }

        // Return the first result from this page
        results
            .into_iter()
            .next()
            .ok_or_else(|| SearchError::BackendError("No results returned".to_string()))
    }

    fn get_metadata(&self) -> Option<SearchMetadata> {
        self.last_metadata.borrow().clone()
    }
}

impl Guest for ExaWebSearchComponent {
    type SearchSession = ExaSearchSession;

    fn start_search(params: SearchParams) -> Result<SearchSession, SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        with_config_key(&[Self::API_KEY_ENV_VAR], Err, |keys| {
            let api_key = keys.get(Self::API_KEY_ENV_VAR).unwrap().to_owned();
            let client = ExaSearchApi::new(api_key);
            Ok(SearchSession::new(ExaSearchSession::new(client, params)))
        })
    }

    fn search_once(
        params: SearchParams,
    ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        with_config_key(&[Self::API_KEY_ENV_VAR], Err, |keys| {
            let api_key = keys.get(Self::API_KEY_ENV_VAR).unwrap().to_owned();
            let client = ExaSearchApi::new(api_key);
            let request = convert_params_to_request(&params, None);
            let response = client.search(request)?;
            let (results, metadata) = convert_response_to_results(response, &params);
            Ok((results, metadata))
        })
    }
}

impl ExtendedGuest for ExaWebSearchComponent {
    fn unwrapped_search_session(params: SearchParams) -> Result<ExaSearchSession, SearchError> {
        println!("[DURABILITY] unwrapped_search_session: Creating new ExaSearchSession");
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        with_config_key(&[Self::API_KEY_ENV_VAR], Err, |keys| {
            let api_key = keys.get(Self::API_KEY_ENV_VAR).unwrap().to_owned();
            let client = ExaSearchApi::new(api_key);
            Ok(ExaSearchSession::new(client, params))
        })
    }

    fn session_from_state(
        params: SearchParams,
        page_count: u32,
    ) -> Result<ExaSearchSession, SearchError> {
        println!("[DURABILITY] session_from_state: Creating ExaSearchSession from state, page_count: {page_count}");
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        with_config_key(&[Self::API_KEY_ENV_VAR], Err, |keys| {
            let api_key = keys.get(Self::API_KEY_ENV_VAR).unwrap().to_owned();
            let client = ExaSearchApi::new(api_key);
            let session = ExaSearchSession::new(client, params);

            // Adjust session state to reflect the page count (offset)
            *session.current_offset.borrow_mut() = page_count;

            Ok(session)
        })
    }

    fn retry_search_params(original_params: &SearchParams, page_count: u32) -> SearchParams {
        println!("[DURABILITY] retry_search_params: Adjusting params for page_count: {page_count}");
        // For Exa, we just return the original params
        // The page is handled internally by the session state
        original_params.clone()
    }
}

type DurableExaWebSearchComponent = DurableWebSearch<ExaWebSearchComponent>;

golem_web_search::export_web_search!(DurableExaWebSearchComponent with_types_in golem_web_search);

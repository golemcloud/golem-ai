use crate::client::BraveSearchApi;
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

struct BraveWebSearchComponent;

impl BraveWebSearchComponent {
    const API_KEY_ENV_VAR: &'static str = "BRAVE_API_KEY";
}

pub struct BraveSearchSession {
    client: BraveSearchApi,
    params: SearchParams,
    current_offset: RefCell<u32>,
    last_metadata: RefCell<Option<SearchMetadata>>,
    has_more_results: RefCell<bool>,
}

impl BraveSearchSession {
    fn new(client: BraveSearchApi, params: SearchParams) -> Self {
        Self {
            client,
            params,
            current_offset: RefCell::new(0),
            last_metadata: RefCell::new(None),
            has_more_results: RefCell::new(true),
        }
    }
}

impl GuestSearchSession for BraveSearchSession {
    fn next_page(&self) -> Result<SearchResult, SearchError> {
        if !*self.has_more_results.borrow() {
            return Err(SearchError::BackendError(
                "No more results available".to_string(),
            ));
        }

        let current_offset = *self.current_offset.borrow();

        if current_offset >= 9 {
            *self.has_more_results.borrow_mut() = false;
            return Err(SearchError::BackendError(
                "Maximum pagination limit reached".to_string(),
            ));
        }

        let new_offset = current_offset + 1;
        *self.current_offset.borrow_mut() = new_offset;

        let request = convert_params_to_request(&self.params, Some(new_offset));
        let response = self.client.search(request)?;
        let (results, metadata) =
            convert_response_to_results(response, &self.params, Some(current_offset));

        *self.last_metadata.borrow_mut() = metadata.clone();

        if results.is_empty() {
            *self.has_more_results.borrow_mut() = false;
            return Err(SearchError::BackendError("No more results".to_string()));
        }

        if let Some(metadata) = &metadata {
            let api_has_more = metadata.next_page_token.is_some();
            let within_limits = new_offset < 9;
            *self.has_more_results.borrow_mut() = api_has_more && within_limits;
        } else {
            *self.has_more_results.borrow_mut() = false;
        }

        results
            .into_iter()
            .next()
            .ok_or_else(|| SearchError::BackendError("No results returned".to_string()))
    }

    fn get_metadata(&self) -> Option<SearchMetadata> {
        self.last_metadata.borrow().clone()
    }
}

impl Guest for BraveWebSearchComponent {
    type SearchSession = BraveSearchSession;

    fn start_search(params: SearchParams) -> Result<SearchSession, SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        with_config_key(&[Self::API_KEY_ENV_VAR], Err, |keys| {
            let api_key = keys.get(Self::API_KEY_ENV_VAR).unwrap().to_owned();
            let client = BraveSearchApi::new(api_key);
            Ok(SearchSession::new(BraveSearchSession::new(client, params)))
        })
    }

    fn search_once(
        params: SearchParams,
    ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        with_config_key(&[Self::API_KEY_ENV_VAR], Err, |keys| {
            let api_key = keys.get(Self::API_KEY_ENV_VAR).unwrap().to_owned();
            let client = BraveSearchApi::new(api_key);
            let request = convert_params_to_request(&params, None);
            let response = client.search(request)?;
            let (results, metadata) = convert_response_to_results(response, &params, None);
            Ok((results, metadata))
        })
    }
}

impl ExtendedGuest for BraveWebSearchComponent {
    fn unwrapped_search_session(params: SearchParams) -> Result<BraveSearchSession, SearchError> {
        println!("[DURABILITY] unwrapped_search_session: Creating new BraveSearchSession");
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        with_config_key(&[Self::API_KEY_ENV_VAR], Err, |keys| {
            let api_key = keys.get(Self::API_KEY_ENV_VAR).unwrap().to_owned();
            let client = BraveSearchApi::new(api_key);
            Ok(BraveSearchSession::new(client, params))
        })
    }

    fn session_from_state(params: SearchParams, page_count: u32) -> Result<BraveSearchSession, SearchError> {
        println!("[DURABILITY] session_from_state: Creating BraveSearchSession from state, page_count: {}", page_count);
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        with_config_key(&[Self::API_KEY_ENV_VAR], Err, |keys| {
            let api_key = keys.get(Self::API_KEY_ENV_VAR).unwrap().to_owned();
            let client = BraveSearchApi::new(api_key);
            let session = BraveSearchSession::new(client, params);
            
            // Adjust session state to reflect the page count
            *session.current_offset.borrow_mut() = page_count;
            
            Ok(session)
        })
    }

    fn retry_search_params(original_params: &SearchParams, page_count: u32) -> SearchParams {
        println!("[DURABILITY] retry_search_params: Adjusting params for page_count: {}", page_count);
        // For Brave, we just return the original params
        // The offset is handled internally by the session state
        original_params.clone()
    }
}

type DurableBraveWebSearchComponent = DurableWebSearch<BraveWebSearchComponent>;

golem_web_search::export_web_search!(DurableBraveWebSearchComponent with_types_in golem_web_search);

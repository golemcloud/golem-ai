use golem_web_search::error::from_reqwest_error;
use golem_web_search::golem::web_search::web_search::SearchError;
use log::trace;
use reqwest::Method;
use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

const BASE_URL: &str = "https://api.search.brave.com/res/v1/web/search";

/// The Brave Search API client for web search.
pub struct BraveSearchApi {
    client: Client,
    pub api_key: String,
}

impl BraveSearchApi {
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .user_agent("Golem-Web-Search/1.0")
            .build()
            .expect("Failed to initialize HTTP client");

        Self { client, api_key }
    }

    pub fn search(&self, request: SearchRequest) -> Result<SearchResponse, SearchError> {
        trace!("Sending request to Brave Search API: {request:?}");

        let response = self
            .client
            .request(Method::GET, BASE_URL)
            .header("X-Subscription-Token", &self.api_key)
            .header("Accept", "application/json")
            .query(&[
                ("q", &request.query),
                ("count", &request.count.unwrap_or(10).to_string()),
                ("offset", &request.offset.unwrap_or(0).to_string()),
            ])
            .send()
            .map_err(|err| from_reqwest_error("Request failed", err))?;

        parse_response(response)
    }

    pub fn api_key(&self) -> &String {
        &self.api_key
    }
}

#[derive(Debug, Clone)]
pub struct SearchRequest {
    pub query: String,
    pub count: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: QueryInfo,
    pub web: Option<WebResults>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryInfo {
    pub original: String,
    pub more_results_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebResults {
    pub results: Vec<WebResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebResult {
    pub title: String,
    pub url: String,
    pub description: String,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
}

fn parse_response<T: DeserializeOwned + Debug>(response: Response) -> Result<T, SearchError> {
    let status = response.status();
    if status.is_success() {
        let body = response
            .json::<T>()
            .map_err(|err| from_reqwest_error("Failed to decode response body", err))?;

        trace!("Received response from Brave Search API: {body:?}");
        Ok(body)
    } else {
        // Try to parse error response
        match response.json::<ErrorResponse>() {
            Ok(error_body) => {
                trace!("Received {status} response from Brave Search API: {error_body:?}");

                let search_error = match status.as_u16() {
                    400 => SearchError::InvalidQuery,
                    401 => SearchError::BackendError("Invalid API key".to_string()),
                    403 => SearchError::BackendError("API key quota exceeded".to_string()),
                    429 => SearchError::RateLimited(60), // Default to 60 seconds
                    _ => SearchError::BackendError(format!(
                        "Request failed with {}: {}",
                        status, error_body.message
                    )),
                };

                Err(search_error)
            }
            Err(_) => {
                // Fallback for non-JSON error responses
                Err(SearchError::BackendError(format!(
                    "Request failed with status {status}"
                )))
            }
        }
    }
}

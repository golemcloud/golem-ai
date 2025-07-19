use golem_web_search::error::from_reqwest_error;
use golem_web_search::golem::web_search::web_search::SearchError;
use log::trace;
use reqwest::Method;
use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

const BASE_URL: &str = "https://api.tavily.com/search";

/// The Tavily Search API client for web search with deep document indexing.
pub struct TavilySearchApi {
    client: Client,
}

impl TavilySearchApi {
    pub fn new(_api_key: String) -> Self {
        let client = Client::builder()
            .user_agent("Golem-Web-Search/1.0")
            .build()
            .expect("Failed to initialize HTTP client");

        Self { client }
    }

    pub fn search(&self, request: SearchRequest) -> Result<SearchResponse, SearchError> {
        trace!("Sending request to Tavily Search API: {request:?}");

        let response = self
            .client
            .request(Method::POST, BASE_URL)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|err| from_reqwest_error("Request failed", err))?;

        parse_response(response)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub api_key: String,
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_depth: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_images: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_answer: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_raw_content: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub answer: Option<String>,
    pub query: String,
    pub response_time: f32,
    pub images: Option<Vec<String>>,
    pub results: Vec<SearchResult>,
    pub follow_up_questions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub content: String,
    pub raw_content: Option<String>,
    pub score: f32,
    pub published_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub detail: Option<String>,
}

fn parse_response<T: DeserializeOwned + Debug>(response: Response) -> Result<T, SearchError> {
    let status = response.status();
    if status.is_success() {
        let body = response
            .json::<T>()
            .map_err(|err| from_reqwest_error("Failed to decode response body", err))?;

        trace!("Received response from Tavily Search API: {body:?}");
        Ok(body)
    } else {
        // Try to parse error response
        match response.json::<ErrorResponse>() {
            Ok(error_body) => {
                trace!("Received {status} response from Tavily Search API: {error_body:?}");

                let search_error = match status.as_u16() {
                    400 => SearchError::InvalidQuery,
                    401 => SearchError::BackendError("Invalid API key".to_string()),
                    403 => SearchError::BackendError("API key quota exceeded".to_string()),
                    429 => SearchError::RateLimited(60), // Default to 60 seconds
                    _ => SearchError::BackendError(format!(
                        "Request failed with {}: {}",
                        status, error_body.error
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

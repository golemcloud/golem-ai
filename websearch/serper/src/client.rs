use golem_web_search::error::from_reqwest_error;
use golem_web_search::golem::web_search::web_search::SearchError;
use log::trace;
use reqwest::Method;
use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

const BASE_URL: &str = "https://google.serper.dev/search";

/// The Serper Search API client for Google-powered web search.
pub struct SerperSearchApi {
    api_key: String,
    client: Client,
}

impl SerperSearchApi {
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .user_agent("Golem-Web-Search/1.0")
            .build()
            .expect("Failed to initialize HTTP client");

        Self { api_key, client }
    }

    pub fn search(&self, request: SearchRequest) -> Result<SearchResponse, SearchError> {
        trace!("Sending request to Serper Search API: {request:?}");

        let response = self
            .client
            .request(Method::POST, BASE_URL)
            .header("X-API-KEY", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|err| from_reqwest_error("Request failed", err))?;

        parse_response(response)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub q: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gl: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hl: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub organic: Vec<SearchResult>,
    #[serde(rename = "searchParameters")]
    pub search_parameters: SearchParameters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub link: String,
    pub snippet: String,
    pub position: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchParameters {
    pub q: String,
    #[serde(rename = "type")]
    pub search_type: String,
    pub engine: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub message: String,
    pub error: Option<String>,
}

fn parse_response<T: DeserializeOwned + Debug>(response: Response) -> Result<T, SearchError> {
    let status = response.status();
    if status.is_success() {
        let body = response
            .json::<T>()
            .map_err(|err| from_reqwest_error("Failed to decode response body", err))?;

        trace!("Received response from Serper Search API: {body:?}");
        Ok(body)
    } else {
        // Try to parse error response
        match response.json::<ErrorResponse>() {
            Ok(error_body) => {
                trace!("Received {status} response from Serper Search API: {error_body:?}");

                let search_error = match status.as_u16() {
                    400 => SearchError::InvalidQuery,
                    401 => SearchError::BackendError("Invalid API key".to_string()),
                    403 => SearchError::BackendError("API access forbidden".to_string()),
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

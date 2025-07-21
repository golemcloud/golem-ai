use golem_web_search::error::from_reqwest_error;
use golem_web_search::golem::web_search::types::SearchError;
use log::trace;
use reqwest::{header, Client, Method, Response, StatusCode};
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://api.exa.ai/search";

pub struct ExaSearchApi {
    api_key: String,
    client: Client,
}

impl ExaSearchApi {
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .build()
            .expect("Failed to initialize HTTP client");
        Self { api_key, client }
    }

    pub fn search(&self, request: ExaSearchRequest) -> Result<ExaSearchResponse, SearchError> {
        trace!("Sending request to Exa Search API: {request:?}");

        let response: Response = self
            .client
            .request(Method::POST, BASE_URL)
            .header(header::ACCEPT, "application/json")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::USER_AGENT, "golem-websearch/1.0")
            .header("x-api-key", &self.api_key)
            .json(&request)
            .send()
            .map_err(|err| from_reqwest_error("Request failed", err))?;

        parse_response(response)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExaSearchRequest {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>, // "keyword", "neural", or "auto"
    #[serde(skip_serializing_if = "Option::is_none", rename = "numResults")]
    pub num_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "includeDomains")]
    pub include_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "excludeDomains")]
    pub exclude_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "startCrawlDate")]
    pub start_crawl_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "endCrawlDate")]
    pub end_crawl_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "startPublishedDate")]
    pub start_published_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "endPublishedDate")]
    pub end_published_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "includeText")]
    pub include_text: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "excludeText")]
    pub exclude_text: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<ExaSearchContents>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExaSearchContents {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlights: Option<ExaHighlightsOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<ExaSummaryOptions>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExaHighlightsOptions {
    #[serde(skip_serializing_if = "Option::is_none", rename = "numSentences")]
    pub num_sentences: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "highlightsPerUrl")]
    pub highlights_per_url: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExaSummaryOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExaSearchResponse {
    pub results: Vec<ExaResult>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "autopromptString")]
    pub autoprompt_string: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExaResult {
    pub id: String,
    pub url: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "publishedDate")]
    pub published_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlights: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "highlightScores")]
    pub highlight_scores: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

fn parse_response(response: Response) -> Result<ExaSearchResponse, SearchError> {
    let status = response.status();

    if status.is_success() {
        let response_text = response
            .text()
            .map_err(|err| from_reqwest_error("Failed to read response body", err))?;

        serde_json::from_str::<ExaSearchResponse>(&response_text)
            .map_err(|err| SearchError::BackendError(format!("Failed to parse response: {err}")))
    } else {
        let error_text = response
            .text()
            .map_err(|err| from_reqwest_error("Failed to read error response", err))?;

        match status {
            StatusCode::UNAUTHORIZED => {
                Err(SearchError::BackendError("Invalid API key".to_string()))
            }
            StatusCode::BAD_REQUEST => Err(SearchError::BackendError(format!(
                "Bad request: {error_text}"
            ))),
            StatusCode::TOO_MANY_REQUESTS => Err(SearchError::RateLimited(0)), // Reset time unknown
            StatusCode::INTERNAL_SERVER_ERROR => Err(SearchError::BackendError(format!(
                "Exa server error: {error_text}"
            ))),
            _ => Err(SearchError::BackendError(format!(
                "HTTP {status}: {error_text}"
            ))),
        }
    }
}

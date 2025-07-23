use golem_web_search::error::from_reqwest_error;
use golem_web_search::golem::web_search::web_search::SearchError;
use log::trace;
use reqwest::Url;
use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://www.googleapis.com/customsearch/v1";

/// Google Custom Search API client for web search.
pub struct GoogleSearchApi {
    client: Client,
    pub api_key: String,
    pub search_engine_id: String,
}

impl GoogleSearchApi {
    pub fn new(api_key: String, search_engine_id: String) -> Self {
        let client = Client::builder()
            .user_agent("Golem-Web-Search/1.0")
            .build()
            .expect("Failed to initialize HTTP client");

        Self {
            client,
            api_key,
            search_engine_id,
        }
    }

    pub fn search(&self, request: SearchRequest) -> Result<SearchResponse, SearchError> {
        trace!("Sending request to Google Custom Search API: {request:?}");

        let mut url = Url::parse(BASE_URL).expect("Invalid base URL");
        {
            let mut query_pairs = url.query_pairs_mut();
            query_pairs.append_pair("key", &self.api_key);
            query_pairs.append_pair("cx", &self.search_engine_id);
            query_pairs.append_pair("q", &urlencoding::encode(&request.query));
            if let Some(num) = request.max_results {
                query_pairs.append_pair("num", &num.to_string());
            }
            if let Some(start) = request.start {
                query_pairs.append_pair("start", &start.to_string());
            }
            if let Some(safe) = &request.safe {
                query_pairs.append_pair("safe", safe);
            }
            if let Some(lr) = &request.lr {
                query_pairs.append_pair("lr", lr);
            }
            if let Some(gl) = &request.gl {
                query_pairs.append_pair("gl", gl);
            }
            if let Some(date_restrict) = &request.date_restrict {
                query_pairs.append_pair("dateRestrict", date_restrict);
            }
            if let Some(site_search) = &request.site_search {
                query_pairs.append_pair("siteSearch", &urlencoding::encode(site_search));
            }
            if let Some(site_search_filter) = &request.site_search_filter {
                query_pairs.append_pair("siteSearchFilter", site_search_filter);
            }
            if request.img_type.is_some() || request.img_size.is_some() {
                query_pairs.append_pair("searchType", "image");
                if let Some(img_type) = &request.img_type {
                    query_pairs.append_pair("imgType", img_type);
                }
                if let Some(img_size) = &request.img_size {
                    query_pairs.append_pair("imgSize", img_size);
                }
            }
        }
        let response = self
            .client
            .request(Method::GET, url.as_str())
            .send()
            .map_err(|err| from_reqwest_error("Failed to send request", err))?;

        parse_response(response)
    }

    pub fn api_key(&self) -> &String {
        &self.api_key
    }

    pub fn search_engine_id(&self) -> &String {
        &self.search_engine_id
    }
}

#[derive(Debug, Clone)]
pub struct SearchRequest {
    pub query: String,
    pub max_results: Option<u32>,
    pub start: Option<u32>,
    pub safe: Option<String>,
    pub lr: Option<String>,
    pub gl: Option<String>,
    pub date_restrict: Option<String>,
    pub site_search: Option<String>,
    pub site_search_filter: Option<String>,
    pub img_type: Option<String>,
    pub img_size: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub response_time: f32,
    pub total_results: Option<u64>,
    pub results: Vec<SearchResult>,
    pub next_page: Option<NextPage>,
    pub previous_page: Option<PreviousPage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub content: String,
    pub published_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoogleApiResponse {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queries: Option<GoogleSearchQueries>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_information: Option<GoogleSearchInformation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<GoogleSearchItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoogleSearchQueries {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<Vec<GoogleQueryInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page: Option<Vec<NextPage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_page: Option<Vec<PreviousPage>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoogleQueryInfo {
    #[serde(rename = "searchTerms")]
    pub search_terms: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoogleSearchInformation {
    #[serde(rename = "searchTime")]
    pub search_time: f64,
    #[serde(rename = "totalResults")]
    pub total_results: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoogleSearchItem {
    pub title: String,
    pub link: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextPage {
    #[serde(rename = "startIndex")]
    pub start_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviousPage {
    #[serde(rename = "startIndex")]
    pub start_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ErrorResponse {
    pub error: ErrorResponseDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ErrorResponseDetails {
    pub code: u32,
    pub message: String,
}

fn parse_response(response: Response) -> Result<SearchResponse, SearchError> {
    let status = response.status();
    if status.is_success() {
        let google_response: GoogleApiResponse = response
            .json()
            .map_err(|err| from_reqwest_error("Failed to decode response body", err))?;

        trace!("Received response from Google Custom Search API: {google_response:?}");

        // Convert Google response
        let query = google_response
            .queries
            .as_ref()
            .and_then(|q| q.request.as_ref())
            .and_then(|r| r.first().map(|qi| qi.search_terms.clone()))
            .unwrap_or_default();

        let response_time = google_response
            .search_information
            .as_ref()
            .map(|info| info.search_time as f32)
            .unwrap_or(0.0);

        let total_results = google_response
            .search_information
            .and_then(|info| info.total_results.parse::<u64>().ok());

        let next_page = google_response
            .queries
            .as_ref()
            .and_then(|q| q.next_page.as_ref())
            .and_then(|np| np.first().cloned());

        let previous_page = google_response
            .queries
            .and_then(|q| q.previous_page)
            .and_then(|pp| pp.first().cloned());

        let results = google_response
            .items
            .unwrap_or_default()
            .into_iter()
            .map(|item| SearchResult {
                title: item.title,
                url: item.link,
                content: item.snippet,
                published_date: None, // Google doesn't provide this in basic search
            })
            .collect();

        Ok(SearchResponse {
            query,
            response_time,
            total_results,
            results,
            next_page,
            previous_page,
        })
    } else {
        // Try to parse error response
        match response.json::<ErrorResponse>() {
            Ok(error_body) => {
                trace!("Received {status} response from Google Custom Search API: {error_body:?}");

                let search_error = match error_body.error.code {
                    400 => SearchError::InvalidQuery,
                    401 => SearchError::BackendError("Invalid API key".to_string()),
                    403 => SearchError::BackendError("API key quota exceeded".to_string()),
                    429 => SearchError::RateLimited(60), // Default to 60 seconds
                    _ => SearchError::BackendError(format!(
                        "Request failed with {}: {}",
                        status, error_body.error.message
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

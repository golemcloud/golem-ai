use golem_search::config::{get_max_retries_config, get_timeout_config};
use golem_search::error::{from_reqwest_error, internal_error, search_error_from_status};
use golem_search::golem::search::types::SearchError;
use log::trace;
use reqwest::{Client, Method, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::fmt::Debug;
use std::time::Duration;

/// The OpenSearch API client for managing indices and performing search
/// Based on the OpenSearch REST API
#[derive(Clone)]
pub struct OpenSearchApi {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    username: Option<String>,
    password: Option<String>,
    max_retries: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenSearchSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mappings: Option<OpenSearchMappings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Map<String, Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenSearchMappings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenSearchQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggs: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _source: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OpenSearchSearchResponse {
    pub took: u32,
    pub timed_out: bool,
    pub hits: OpenSearchHits,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregations: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OpenSearchHits {
    pub total: OpenSearchTotal,
    pub max_score: Option<f64>,
    pub hits: Vec<OpenSearchHit>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OpenSearchTotal {
    pub value: u32,
    pub relation: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OpenSearchHit {
    #[serde(rename = "_index")]
    pub index: String,
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "_score")]
    pub score: Option<f64>,
    #[serde(rename = "_source")]
    pub source: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct OpenSearchBulkOperation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<OpenSearchBulkAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create: Option<OpenSearchBulkAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update: Option<OpenSearchBulkAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<OpenSearchBulkAction>,
}

#[derive(Debug, Serialize)]
pub struct OpenSearchBulkAction {
    #[serde(rename = "_index")]
    pub index: String,
    #[serde(rename = "_id")]
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OpenSearchBulkResponse {
    pub took: u32,
    pub errors: bool,
    pub items: Vec<Value>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OpenSearchIndexInfo {
    pub health: Option<String>,
    pub status: Option<String>,
    pub index: String,
    pub uuid: Option<String>,
    pub pri: Option<String>,
    pub rep: Option<String>,
    #[serde(rename = "docs.count")]
    pub docs_count: Option<String>,
    #[serde(rename = "docs.deleted")]
    pub docs_deleted: Option<String>,
    #[serde(rename = "store.size")]
    pub store_size: Option<String>,
    #[serde(rename = "pri.store.size")]
    pub pri_store_size: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OpenSearchErrorResponse {
    pub error: OpenSearchError,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OpenSearchError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OpenSearchScrollResponse {
    #[serde(rename = "_scroll_id")]
    pub scroll_id: String,
    pub took: u32,
    pub timed_out: bool,
    pub hits: OpenSearchHits,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregations: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct ScrollRequest {
    pub scroll: String,
    pub scroll_id: String,
}

impl OpenSearchApi {
    pub fn new(
        base_url: String,
        username: Option<String>,
        password: Option<String>,
        api_key: Option<String>,
    ) -> Self {
        let timeout_secs = get_timeout_config();
        let max_retries = get_max_retries_config();

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to initialize HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            username,
            password,
            max_retries,
        }
    }

    fn should_retry_error(&self, error: &reqwest::Error) -> bool {
        error.is_timeout() || error.is_request()
    }

    fn calculate_backoff_delay(attempt: u32, is_rate_limited: bool) -> Duration {
        let base_delay_ms = if is_rate_limited { 1000 } else { 200 }; // 1s for rate limit, 200ms for others
        let max_delay_ms = 30000; // 30 seconds max

        let delay_ms = std::cmp::min(max_delay_ms, base_delay_ms * (2_u64.pow(attempt)));

        Duration::from_millis(delay_ms)
    }

    fn execute_with_retry_sync<F>(&self, operation: F) -> Result<Response, SearchError>
    where
        F: Fn() -> Result<Response, reqwest::Error> + Send + Sync,
    {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            match operation() {
                Ok(response) => {
                    match response.status().as_u16() {
                        429 => {
                            // Rate limited - should retry with longer delay
                            if attempt < self.max_retries {
                                let delay = Self::calculate_backoff_delay(attempt, true);
                                trace!(
                                    "Rate limited (429), retrying in {:?} (attempt {}/{})",
                                    delay,
                                    attempt + 1,
                                    self.max_retries + 1
                                );
                                std::thread::sleep(delay);
                                continue;
                            } else {
                                return Ok(response);
                            }
                        }
                        502..=504 => {
                            // Server errors - should retry
                            if attempt < self.max_retries {
                                let delay = Self::calculate_backoff_delay(attempt, false);
                                trace!(
                                    "Server error ({}), retrying in {:?} (attempt {}/{})",
                                    response.status().as_u16(),
                                    delay,
                                    attempt + 1,
                                    self.max_retries + 1
                                );
                                std::thread::sleep(delay);
                                continue;
                            } else {
                                return Ok(response);
                            }
                        }
                        _ => return Ok(response),
                    }
                }
                Err(e) => {
                    last_error = Some(e);

                    if let Some(ref error) = last_error {
                        if self.should_retry_error(error) && attempt < self.max_retries {
                            let is_rate_limited = error.status().is_some_and(|s| s.as_u16() == 429);
                            let delay = Self::calculate_backoff_delay(attempt, is_rate_limited);

                            trace!(
                                "Request failed, retrying in {:?} (attempt {}/{}): {:?}",
                                delay,
                                attempt + 1,
                                self.max_retries + 1,
                                error
                            );
                            std::thread::sleep(delay);
                        } else if !self.should_retry_error(error) {
                            trace!("Request failed with non-retryable error: {error:?}");
                            break;
                        }
                    }
                }
            }
        }

        let error = last_error.unwrap();
        Err(internal_error(format!(
            "Request failed after {} attempts: {}",
            self.max_retries + 1,
            error
        )))
    }

    fn create_request(&self, method: Method, url: &str) -> RequestBuilder {
        let mut builder = self
            .client
            .request(method, url)
            .header("Content-Type", "application/json");

        // Add authentication
        if let Some(api_key) = &self.api_key {
            builder = builder.header("Authorization", format!("ApiKey {api_key}"));
        } else if let (Some(username), Some(password)) = (&self.username, &self.password) {
            builder = builder.basic_auth(username, Some(password));
        }

        builder
    }

    fn create_request_with_content_type(
        &self,
        method: Method,
        url: &str,
        content_type: &str,
    ) -> RequestBuilder {
        let mut builder = self
            .client
            .request(method, url)
            .header("Content-Type", content_type);

        // Add authentication
        if let Some(api_key) = &self.api_key {
            builder = builder.header("Authorization", format!("ApiKey {api_key}"));
        } else if let (Some(username), Some(password)) = (&self.username, &self.password) {
            builder = builder.basic_auth(username, Some(password));
        }

        builder
    }

    pub fn create_index(
        &self,
        index_name: &str,
        settings: Option<OpenSearchSettings>,
    ) -> Result<(), SearchError> {
        trace!("Creating index: {index_name}");

        let url = format!("{}/{}", self.base_url, index_name);

        let response = self.execute_with_retry_sync(|| {
            let mut request = self.create_request(Method::PUT, &url);

            if let Some(ref settings) = settings {
                request = request.json(settings);
            }

            request.send()
        })?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(search_error_from_status(response.status()))
        }
    }

    pub fn delete_index(&self, index_name: &str) -> Result<(), SearchError> {
        trace!("Deleting index: {index_name}");

        let url = format!("{}/{}", self.base_url, index_name);

        let response =
            self.execute_with_retry_sync(|| self.create_request(Method::DELETE, &url).send())?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(search_error_from_status(response.status()))
        }
    }

    pub fn list_indices(&self) -> Result<Vec<OpenSearchIndexInfo>, SearchError> {
        trace!("Listing indices");

        let url = format!("{}/_cat/indices?format=json", self.base_url);

        let response =
            self.execute_with_retry_sync(|| self.create_request(Method::GET, &url).send())?;

        parse_response(response)
    }

    pub fn index_document(
        &self,
        index_name: &str,
        id: &str,
        document: &Value,
    ) -> Result<(), SearchError> {
        trace!("Indexing document {id} in index: {index_name}");

        let url = format!("{}/{}/_doc/{}", self.base_url, index_name, id);

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::PUT, &url).json(document).send()
        })?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(search_error_from_status(response.status()))
        }
    }

    pub fn bulk_index(&self, operations: &str) -> Result<OpenSearchBulkResponse, SearchError> {
        trace!("Performing bulk index operation");

        let url = format!("{}/_bulk", self.base_url);

        let response = self.execute_with_retry_sync(|| {
            self.create_request_with_content_type(Method::POST, &url, "application/x-ndjson")
                .body(operations.to_string())
                .send()
        })?;

        parse_response(response)
    }

    pub fn delete_document(&self, index_name: &str, id: &str) -> Result<(), SearchError> {
        trace!("Deleting document {id} from index: {index_name}");

        let url = format!("{}/{}/_doc/{}", self.base_url, index_name, id);

        let response =
            self.execute_with_retry_sync(|| self.create_request(Method::DELETE, &url).send())?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(search_error_from_status(response.status()))
        }
    }

    pub fn get_document(&self, index_name: &str, id: &str) -> Result<Option<Value>, SearchError> {
        trace!("Getting document {id} from index: {index_name}");

        let url = format!("{}/{}/_doc/{}", self.base_url, index_name, id);

        let response =
            self.execute_with_retry_sync(|| self.create_request(Method::GET, &url).send())?;

        if response.status() == 404 {
            Ok(None)
        } else if response.status().is_success() {
            let doc: Value = parse_response(response)?;
            if let Some(source) = doc.get("_source") {
                Ok(Some(source.clone()))
            } else {
                Ok(None)
            }
        } else {
            Err(search_error_from_status(response.status()))
        }
    }

    pub fn search(
        &self,
        index_name: &str,
        query: &OpenSearchQuery,
    ) -> Result<OpenSearchSearchResponse, SearchError> {
        trace!("Searching index {index_name} with query: {query:?}");

        let url = format!("{}/{}/_search", self.base_url, index_name);

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, &url).json(query).send()
        })?;

        parse_response(response)
    }

    pub fn search_with_scroll(
        &self,
        index_name: &str,
        query: &OpenSearchQuery,
        scroll_timeout: &str,
    ) -> Result<OpenSearchScrollResponse, SearchError> {
        trace!("Searching index {index_name} with scroll, timeout: {scroll_timeout}");

        let url = format!(
            "{}/{}/_search?scroll={}",
            self.base_url, index_name, scroll_timeout
        );

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, &url).json(query).send()
        })?;

        parse_response(response)
    }

    pub fn scroll(
        &self,
        scroll_id: &str,
        scroll_timeout: &str,
    ) -> Result<OpenSearchScrollResponse, SearchError> {
        trace!("Scrolling with ID: {scroll_id} and timeout: {scroll_timeout}");

        let url = format!("{}/_search/scroll", self.base_url);

        let response = self.execute_with_retry_sync(|| {
            let scroll_request = ScrollRequest {
                scroll: scroll_timeout.to_string(),
                scroll_id: scroll_id.to_string(),
            };

            self.create_request(Method::POST, &url)
                .json(&scroll_request)
                .send()
        })?;

        parse_response(response)
    }

    pub fn clear_scroll(&self, scroll_id: &str) -> Result<(), SearchError> {
        trace!("Clearing scroll: {scroll_id}");

        let url = format!("{}/_search/scroll", self.base_url);
        let request_body = json!({
            "scroll_id": scroll_id
        });

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::DELETE, &url)
                .json(&request_body)
                .send()
        })?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(search_error_from_status(response.status()))
        }
    }

    pub fn get_mappings(&self, index_name: &str) -> Result<Value, SearchError> {
        trace!("Getting mappings for index: {index_name}");

        let url = format!("{}/{}/_mapping", self.base_url, index_name);

        let response =
            self.execute_with_retry_sync(|| self.create_request(Method::GET, &url).send())?;

        parse_response(response)
    }

    pub fn put_mappings(
        &self,
        index_name: &str,
        mappings: &OpenSearchMappings,
    ) -> Result<(), SearchError> {
        trace!("Putting mappings for index: {index_name}");

        let url = format!("{}/{}/_mapping", self.base_url, index_name);

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::PUT, &url).json(mappings).send()
        })?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(search_error_from_status(response.status()))
        }
    }
}

fn parse_response<T: DeserializeOwned + Debug>(response: Response) -> Result<T, SearchError> {
    let status = response.status();

    trace!("Received response from OpenSearch API: {response:?}");

    if status.is_success() {
        let body = response
            .json::<T>()
            .map_err(|err| from_reqwest_error("Failed to decode response body", err))?;

        trace!("Received response from OpenSearch API: {body:?}");

        Ok(body)
    } else {
        let error_body = response
            .text()
            .map_err(|err| from_reqwest_error("Failed to receive error response body", err))?;

        trace!("Received {status} response from OpenSearch API: {error_body:?}");

        Err(search_error_from_status(status))
    }
}

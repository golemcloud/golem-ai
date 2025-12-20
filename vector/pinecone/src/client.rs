use golem_vector::config::{get_max_retries_config, get_timeout_config};
use golem_vector::golem::vector::types::VectorError;
use log::trace;
use reqwest::{Client, Method, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::time::Duration;

/// The Pinecone Vector API client
/// based on https://docs.pinecone.io/reference/api/2025-04/
#[derive(Clone)]
pub struct PineconeClient {
    client: Client,
    api_key: String,
    control_plane_host: String,
    data_plane_host: Option<String>,
}

impl PineconeClient {
    pub fn new(api_key: String, environment: Option<String>) -> Self {
        let timeout_secs = get_timeout_config();

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to initialize HTTP client");

        let control_plane_host = match environment {
            Some(env) => format!("https://api.{}.pinecone.io", env),
            None => "https://api.pinecone.io".to_string(),
        };

        Self {
            client,
            api_key,
            control_plane_host,
            data_plane_host: None,
        }
    }

    fn create_request(&self, method: Method, url: &str) -> RequestBuilder {
        self.client
            .request(method, url)
            .header("Api-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("X-Pinecone-API-Version", "2025-04")
    }

    fn get_data_plane_url(&self, index_name: &str) -> Result<String, VectorError> {
        match &self.data_plane_host {
            Some(host) => Ok(host.clone()),
            None => {
                let index_info = self.describe_index(index_name)?;
                match index_info.host {
                    Some(host) => Ok(format!("https://{}", host)),
                    None => Err(VectorError::ConnectionError(
                        "No data plane host available for index".to_string(),
                    )),
                }
            }
        }
    }

    fn should_retry_error(&self, error: &reqwest::Error) -> bool {
        error.is_timeout() || error.is_request()
    }

    fn calculate_backoff_delay(attempt: u32, is_rate_limited: bool) -> Duration {
        let base_delay_ms = if is_rate_limited { 1000 } else { 200 };
        let max_delay_ms = 30000;

        let delay_ms = std::cmp::min(max_delay_ms, base_delay_ms * (2_u64.pow(attempt)));

        Duration::from_millis(delay_ms)
    }

    fn execute_with_retry_sync<F>(&self, operation: F) -> Result<Response, VectorError>
    where
        F: Fn() -> Result<Response, reqwest::Error> + Send + Sync,
    {
        let max_retries = get_max_retries_config();
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match operation() {
                Ok(response) => match response.status().as_u16() {
                    429 => {
                        if attempt < max_retries {
                            let delay = Self::calculate_backoff_delay(attempt, true);
                            trace!(
                                "Rate limited (attempt {}/{}), retrying in {:?}",
                                attempt + 1,
                                max_retries + 1,
                                delay
                            );
                            std::thread::sleep(delay);
                            continue;
                        } else {
                            return Err(VectorError::RateLimited(
                                "Rate limit exceeded after retries".to_string(),
                            ));
                        }
                    }
                    502..=504 => {
                        if attempt < max_retries {
                            let delay = Self::calculate_backoff_delay(attempt, false);
                            trace!(
                                "Server error {} (attempt {}/{}), retrying in {:?}",
                                response.status(),
                                attempt + 1,
                                max_retries + 1,
                                delay
                            );
                            std::thread::sleep(delay);
                            continue;
                        } else {
                            return Err(VectorError::ConnectionError(format!(
                                "Server error {} after {} attempts",
                                response.status(),
                                max_retries + 1
                            )));
                        }
                    }
                    _ => return Ok(response),
                },
                Err(e) => {
                    last_error = Some(e);

                    if let Some(ref error) = last_error {
                        if self.should_retry_error(error) && attempt < max_retries {
                            let is_rate_limited = error.status().is_some_and(|s| s.as_u16() == 429);
                            let delay = Self::calculate_backoff_delay(attempt, is_rate_limited);

                            trace!(
                                "Request failed (attempt {}/{}): {}. Retrying in {:?}",
                                attempt + 1,
                                max_retries + 1,
                                error,
                                delay
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
        Err(VectorError::ConnectionError(format!(
            "Request failed after {} attempts: {}",
            max_retries + 1,
            error
        )))
    }

    pub fn list_indexes(&self) -> Result<ListIndexesResponse, VectorError> {
        trace!("Listing indexes");

        let url = format!("{}/indexes", self.control_plane_host);

        let response =
            self.execute_with_retry_sync(|| self.create_request(Method::GET, &url).send())?;

        parse_response(response, "list_indexes")
    }

    pub fn create_index(&self, request: &CreateIndexRequest) -> Result<IndexModel, VectorError> {
        trace!("Creating index: {}", request.name);

        let url = format!("{}/indexes", self.control_plane_host);

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, &url).json(request).send()
        })?;

        parse_response(response, "create_index")
    }

    pub fn describe_index(&self, index_name: &str) -> Result<IndexModel, VectorError> {
        trace!("Describing index: {index_name}");

        let url = format!("{}/indexes/{}", self.control_plane_host, index_name);

        let response =
            self.execute_with_retry_sync(|| self.create_request(Method::GET, &url).send())?;

        parse_response(response, "describe_index")
    }

    pub fn delete_index(&self, index_name: &str) -> Result<(), VectorError> {
        trace!("Deleting index: {index_name}");

        let url = format!("{}/indexes/{}", self.control_plane_host, index_name);

        let response =
            self.execute_with_retry_sync(|| self.create_request(Method::DELETE, &url).send())?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_body = response.text().map_err(|e| {
                VectorError::ConnectionError(format!("Failed to read error response: {e}"))
            })?;
            Err(VectorError::ProviderError(format!(
                "Delete index failed: {error_body}"
            )))
        }
    }

    pub fn upsert_vectors(
        &self,
        index_name: &str,
        request: &UpsertRequest,
    ) -> Result<UpsertResponse, VectorError> {
        trace!(
            "Upserting {} vectors to index: {index_name}",
            request.vectors.len()
        );

        let data_plane_url = self.get_data_plane_url(index_name)?;
        let url = format!("{}/vectors/upsert", data_plane_url);

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, &url).json(request).send()
        })?;

        parse_response(response, "upsert_vectors")
    }

    pub fn query_vectors(
        &self,
        index_name: &str,
        request: &QueryRequest,
    ) -> Result<QueryResponse, VectorError> {
        trace!("Querying vectors in index: {index_name}");

        let data_plane_url = self.get_data_plane_url(index_name)?;
        let url = format!("{}/query", data_plane_url);

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, &url).json(request).send()
        })?;

        parse_response(response, "query_vectors")
    }

    pub fn fetch_vectors(
        &self,
        index_name: &str,
        request: &FetchRequest,
    ) -> Result<FetchResponse, VectorError> {
        trace!("Fetching vectors from index: {index_name}");

        let data_plane_url = self.get_data_plane_url(index_name)?;
        let mut url = format!("{}/vectors/fetch", data_plane_url);

        let mut params = Vec::new();
        for id in &request.ids {
            params.push(format!("ids={}", urlencoding::encode(id)));
        }
        if let Some(namespace) = &request.namespace {
            params.push(format!("namespace={}", urlencoding::encode(namespace)));
        }
        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        let response =
            self.execute_with_retry_sync(|| self.create_request(Method::GET, &url).send())?;

        parse_response(response, "fetch_vectors")
    }

    pub fn delete_vectors(
        &self,
        index_name: &str,
        request: &DeleteRequest,
    ) -> Result<(), VectorError> {
        trace!("Deleting vectors from index: {index_name}");

        let data_plane_url = self.get_data_plane_url(index_name)?;
        let url = format!("{}/vectors/delete", data_plane_url);

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, &url).json(request).send()
        })?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_body = response.text().map_err(|e| {
                VectorError::ConnectionError(format!("Failed to read error response: {e}"))
            })?;
            Err(VectorError::ProviderError(format!(
                "Delete vectors failed: {error_body}"
            )))
        }
    }

    pub fn describe_index_stats(
        &self,
        index_name: &str,
        request: &DescribeIndexStatsRequest,
    ) -> Result<DescribeIndexStatsResponse, VectorError> {
        trace!("Describing index stats for: {index_name}");

        let data_plane_url = self.get_data_plane_url(index_name)?;
        let url = format!("{}/describe_index_stats", data_plane_url);

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, &url).json(request).send()
        })?;

        parse_response(response, "describe_index_stats")
    }

    pub fn list_namespaces(&self, index_name: &str) -> Result<ListNamespacesResponse, VectorError> {
        trace!("Listing namespaces for index: {index_name}");

        let data_plane_url = self.get_data_plane_url(index_name)?;
        let url = format!("{}/namespaces", data_plane_url);

        let response =
            self.execute_with_retry_sync(|| self.create_request(Method::GET, &url).send())?;

        parse_response(response, "list_namespaces")
    }

    pub fn list_vector_ids(
        &self,
        index_name: &str,
        request: &ListVectorIdsRequest,
    ) -> Result<ListVectorIdsResponse, VectorError> {
        trace!("Listing vector IDs for index: {index_name}");

        let data_plane_url = self.get_data_plane_url(index_name)?;
        let mut url = format!("{}/vectors/list", data_plane_url);

        let mut params = Vec::new();
        if let Some(prefix) = &request.prefix {
            params.push(format!("prefix={}", urlencoding::encode(prefix)));
        }
        if let Some(limit) = request.limit {
            params.push(format!("limit={}", limit));
        }
        if let Some(pagination_token) = &request.pagination_token {
            params.push(format!(
                "paginationToken={}",
                urlencoding::encode(pagination_token)
            ));
        }
        if let Some(namespace) = &request.namespace {
            params.push(format!("namespace={}", urlencoding::encode(namespace)));
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        let response =
            self.execute_with_retry_sync(|| self.create_request(Method::GET, &url).send())?;

        parse_response(response, "list_vector_ids")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListIndexesResponse {
    pub indexes: Vec<IndexList>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexList {
    pub name: String,
    pub dimension: u32,
    pub metric: String,
    pub host: String,
    pub spec: IndexSpec,
    pub status: IndexStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIndexRequest {
    pub name: String,
    pub dimension: u32,
    pub metric: String,
    pub spec: IndexSpec,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deletion_protection: Option<DeletionProtection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexModel {
    pub name: String,
    pub dimension: u32,
    pub metric: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    pub spec: IndexSpec,
    pub status: IndexStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deletion_protection: Option<DeletionProtection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeletionProtection {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IndexSpec {
    Serverless(ServerlessSpec),
    Pod(PodSpec),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerlessSpec {
    pub serverless: ServerlessConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerlessConfig {
    pub cloud: String,
    pub region: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodSpec {
    pub pod: PodConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodConfig {
    pub environment: String,
    pub replicas: u32,
    pub shards: u32,
    pub pod_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pods: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_config: Option<MetadataConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_collection: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataConfig {
    pub indexed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStatus {
    pub ready: bool,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertRequest {
    pub vectors: Vec<Vector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vector {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sparse_values: Option<SparseValues>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseValues {
    pub indices: Vec<u32>,
    pub values: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertResponse {
    #[serde(rename = "upsertedCount")]
    pub upserted_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(rename = "topK")]
    pub top_k: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<HashMap<String, serde_json::Value>>,
    #[serde(rename = "includeValues")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_values: Option<bool>,
    #[serde(rename = "includeMetadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_metadata: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector: Option<Vec<f32>>,
    #[serde(rename = "sparseVector")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sparse_vector: Option<SparseValues>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub matches: Vec<ScoredVector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredVector {
    pub id: String,
    pub score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sparse_values: Option<SparseValues>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    #[serde(rename = "readUnits")]
    pub read_units: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchRequest {
    pub ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResponse {
    pub vectors: HashMap<String, Vector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ids: Option<Vec<String>>,
    #[serde(rename = "deleteAll")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_all: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescribeIndexStatsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescribeIndexStatsResponse {
    #[serde(rename = "namespaces")]
    pub namespaces: HashMap<String, NamespaceSummary>,
    pub dimension: u32,
    #[serde(rename = "indexFullness")]
    pub index_fullness: f32,
    #[serde(rename = "totalVectorCount")]
    pub total_vector_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NamespaceSummary {
    #[serde(rename = "vectorCount")]
    pub vector_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListNamespacesResponse {
    pub namespaces: Vec<NamespaceInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<NamespacePagination>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceInfo {
    pub name: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub record_count: u64,
}

fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{Deserialize, Error};

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNumber {
        String(String),
        Number(u64),
    }

    match StringOrNumber::deserialize(deserializer)? {
        StringOrNumber::String(s) => s.parse().map_err(D::Error::custom),
        StringOrNumber::Number(n) => Ok(n),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespacePagination {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListVectorIdsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListVectorIdsResponse {
    pub vectors: Vec<VectorId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorId {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PineconeErrorResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<PineconeError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PineconeError {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

// helper functions

fn from_pinecone_error_code(error_code: i32, message: &str, operation: &str) -> VectorError {
    match error_code {
        // auth and authorization errors (3-7)
        3 => VectorError::Unauthorized(format!("Forbidden for {}: {}", operation, message)),
        7 => VectorError::Unauthorized(format!("Unauthorized for {}: {}", operation, message)),

        // client errors (4xx range)
        400 => VectorError::InvalidParams(format!("Bad request for {}: {}", operation, message)),
        401 => VectorError::Unauthorized(format!(
            "Authentication failed for {}: {}",
            operation, message
        )),
        403 => {
            VectorError::Unauthorized(format!("Permission denied for {}: {}", operation, message))
        }
        404 => VectorError::NotFound(format!("Resource not found for {}: {}", operation, message)),
        409 => {
            VectorError::AlreadyExists(format!("Resource conflict for {}: {}", operation, message))
        }
        422 => {
            VectorError::InvalidParams(format!("Validation failed for {}: {}", operation, message))
        }
        429 => VectorError::RateLimited(format!(
            "Rate limit exceeded for {}: {}",
            operation, message
        )),

        // server errors (5xx range)
        500 => VectorError::ProviderError(format!(
            "Internal server error for {}: {}",
            operation, message
        )),
        502 => VectorError::ConnectionError(format!("Bad gateway for {}: {}", operation, message)),
        503 => VectorError::ConnectionError(format!(
            "Service unavailable for {}: {}",
            operation, message
        )),
        504 => {
            VectorError::ConnectionError(format!("Gateway timeout for {}: {}", operation, message))
        }

        // Pinecone-specific
        10000..=10999 => VectorError::InvalidParams(format!(
            "Pinecone validation error {} for {}: {}",
            error_code, operation, message
        )),
        11000..=11999 => VectorError::NotFound(format!(
            "Pinecone resource error {} for {}: {}",
            error_code, operation, message
        )),
        12000..=12999 => VectorError::AlreadyExists(format!(
            "Pinecone conflict error {} for {}: {}",
            error_code, operation, message
        )),
        13000..=13999 => VectorError::DimensionMismatch(format!(
            "Pinecone dimension error {} for {}: {}",
            error_code, operation, message
        )),
        14000..=14999 => VectorError::RateLimited(format!(
            "Pinecone rate limit error {} for {}: {}",
            error_code, operation, message
        )),
        15000..=15999 => VectorError::ProviderError(format!(
            "Pinecone index error {} for {}: {}",
            error_code, operation, message
        )),
        16000..=16999 => VectorError::InvalidParams(format!(
            "Pinecone vector operation error {} for {}: {}",
            error_code, operation, message
        )),
        17000..=17999 => VectorError::ConnectionError(format!(
            "Pinecone connection error {} for {}: {}",
            error_code, operation, message
        )),
        18000..=18999 => VectorError::ProviderError(format!(
            "Pinecone service error {} for {}: {}",
            error_code, operation, message
        )),

        _ => VectorError::ProviderError(format!(
            "Unknown Pinecone error code {} for {}: {}",
            error_code, operation, message
        )),
    }
}

fn from_http_status_code(status_code: u16, message: &str, operation: &str) -> VectorError {
    match status_code {
        400 => VectorError::InvalidParams(format!("Bad request for {}: {}", operation, message)),
        401 => VectorError::Unauthorized(format!(
            "Authentication failed for {}: {}",
            operation, message
        )),
        403 => {
            VectorError::Unauthorized(format!("Permission denied for {}: {}", operation, message))
        }
        404 => VectorError::NotFound(format!("Resource not found for {}: {}", operation, message)),
        409 => {
            VectorError::AlreadyExists(format!("Resource conflict for {}: {}", operation, message))
        }
        422 => {
            VectorError::InvalidParams(format!("Validation failed for {}: {}", operation, message))
        }
        429 => VectorError::RateLimited(format!(
            "Rate limit exceeded for {}: {}",
            operation, message
        )),
        500 => VectorError::ProviderError(format!(
            "Pinecone internal server error for {}: {}",
            operation, message
        )),
        502 => VectorError::ConnectionError(format!("Bad gateway for {}: {}", operation, message)),
        503 => VectorError::ConnectionError(format!(
            "Service unavailable for {}: {}",
            operation, message
        )),
        504 => {
            VectorError::ConnectionError(format!("Gateway timeout for {}: {}", operation, message))
        }
        505..=599 => VectorError::ProviderError(format!(
            "Server error {} for {}: {}",
            status_code, operation, message
        )),
        _ => VectorError::ProviderError(format!(
            "HTTP {} error for {}: {}",
            status_code, operation, message
        )),
    }
}

fn handle_pinecone_error(response: Response, operation: &str) -> VectorError {
    let status = response.status();

    if !status.is_success() {
        let error_body = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        trace!("HTTP error {status} for {operation}: {error_body:?}");

        if let Ok(error_response) = serde_json::from_str::<PineconeErrorResponse>(&error_body) {
            if let Some(code) = error_response.code {
                let message = error_response
                    .message
                    .as_deref()
                    .or_else(|| error_response.error.as_ref()?.message.as_deref())
                    .unwrap_or("No error message provided");
                return from_pinecone_error_code(code, message, operation);
            }

            if let Some(error) = &error_response.error {
                if let Some(code_str) = &error.code {
                    if let Ok(code) = code_str.parse::<i32>() {
                        let message = error
                            .message
                            .as_deref()
                            .or(error_response.message.as_deref())
                            .unwrap_or("No error message provided");
                        return from_pinecone_error_code(code, message, operation);
                    }
                }

                if let Some(message) = &error.message {
                    return from_http_status_code(status.as_u16(), message, operation);
                }
            }

            if let Some(message) = error_response.message {
                return from_http_status_code(status.as_u16(), &message, operation);
            }
        }

        return from_http_status_code(status.as_u16(), &error_body, operation);
    }

    VectorError::ProviderError(format!("Unexpected error state for {}", operation))
}

fn parse_response<T: DeserializeOwned + Debug>(
    response: Response,
    operation: &str,
) -> Result<T, VectorError> {
    let status = response.status();

    trace!("Received response from Pinecone API for {operation}: {response:?}");

    if status.is_success() {
        match response.json::<T>() {
            Ok(body) => {
                trace!("Successfully parsed Pinecone response for {operation}: {body:?}");
                Ok(body)
            }
            Err(parse_error) => {
                trace!("Failed to parse Pinecone response for {operation}: {parse_error}");
                Err(VectorError::ProviderError(format!(
                    "Failed to parse Pinecone response for {}: {}",
                    operation, parse_error
                )))
            }
        }
    } else {
        Err(handle_pinecone_error(response, operation))
    }
}

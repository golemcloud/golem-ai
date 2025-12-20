use golem_vector::config::{get_max_retries_config, get_timeout_config};
use golem_vector::golem::vector::types::VectorError;
use log::trace;
use reqwest::{Client, Method, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::time::Duration;

/// Qdrant Vector API client
/// based on https://qdrant.tech/documentation/
#[derive(Clone)]
pub struct QdrantClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl QdrantClient {
    pub fn new(url: String, api_key: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(get_timeout_config()))
            .build()
            .unwrap();

        Self {
            client,
            base_url: url.trim_end_matches('/').to_string(),
            api_key,
        }
    }

    fn create_request(&self, method: Method, endpoint: &str) -> RequestBuilder {
        let url = format!("{}{}", self.base_url, endpoint);
        let mut req = self.client.request(method, &url);

        if let Some(api_key) = &self.api_key {
            req = req.header("api-key", api_key);
        }

        req.header("Content-Type", "application/json")
    }

    fn should_retry_error(&self, error: &reqwest::Error) -> bool {
        if let Some(status) = error.status() {
            matches!(status.as_u16(), 429 | 500 | 502 | 503 | 504)
        } else {
            error.is_timeout()
        }
    }

    fn calculate_backoff_delay(attempt: u32, is_rate_limited: bool) -> Duration {
        let base_delay = if is_rate_limited { 1000 } else { 500 };
        let delay_ms = base_delay * 2_u64.pow(attempt);
        Duration::from_millis(delay_ms.min(30000))
    }

    fn execute_with_retry_sync<F>(&self, operation: F) -> Result<Response, VectorError>
    where
        F: Fn() -> Result<Response, reqwest::Error> + Send + Sync,
    {
        let max_retries = get_max_retries_config();

        for attempt in 0..=max_retries {
            match operation() {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(response);
                    } else if response.status().as_u16() == 429 && attempt < max_retries {
                        let delay = Self::calculate_backoff_delay(attempt, true);
                        std::thread::sleep(delay);
                        continue;
                    } else {
                        return Err(handle_qdrant_error(response, "operation"));
                    }
                }
                Err(e) => {
                    if attempt < max_retries && self.should_retry_error(&e) {
                        let is_rate_limited = e.status().is_some_and(|s| s.as_u16() == 429);
                        let delay = Self::calculate_backoff_delay(attempt, is_rate_limited);
                        std::thread::sleep(delay);
                        continue;
                    } else {
                        return Err(VectorError::ProviderError(format!("Request failed: {}", e)));
                    }
                }
            }
        }

        Err(VectorError::ProviderError(
            "Max retries exceeded".to_string(),
        ))
    }

    pub fn list_collections(&self) -> Result<ListCollectionsResponse, VectorError> {
        let request = || self.create_request(Method::GET, "/collections").send();

        let response = self.execute_with_retry_sync(request)?;
        parse_response(response, "list_collections")
    }

    pub fn create_collection(
        &self,
        request: &CreateCollectionRequest,
    ) -> Result<CreateCollectionResponse, VectorError> {
        let collection_name = &request.collection_name;
        let request_fn = || {
            self.create_request(Method::PUT, &format!("/collections/{}", collection_name))
                .json(&request.config)
                .send()
        };

        let response = self.execute_with_retry_sync(request_fn)?;
        parse_response(response, "create_collection")
    }

    pub fn get_collection(
        &self,
        collection_name: &str,
    ) -> Result<GetCollectionResponse, VectorError> {
        let request = || {
            self.create_request(Method::GET, &format!("/collections/{}", collection_name))
                .send()
        };

        let response = self.execute_with_retry_sync(request)?;
        parse_response(response, "get_collection")
    }

    pub fn delete_collection(
        &self,
        collection_name: &str,
    ) -> Result<DeleteCollectionResponse, VectorError> {
        let request = || {
            self.create_request(Method::DELETE, &format!("/collections/{}", collection_name))
                .send()
        };

        let response = self.execute_with_retry_sync(request)?;
        parse_response(response, "delete_collection")
    }

    pub fn collection_exists(&self, collection_name: &str) -> Result<bool, VectorError> {
        match self.get_collection(collection_name) {
            Ok(_) => Ok(true),
            Err(VectorError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    pub fn upsert_points(
        &self,
        collection_name: &str,
        request: &UpsertRequest,
    ) -> Result<UpsertResponse, VectorError> {
        let request_fn = || {
            self.create_request(
                Method::PUT,
                &format!("/collections/{}/points", collection_name),
            )
            .json(request)
            .send()
        };

        let response = self.execute_with_retry_sync(request_fn)?;
        parse_response(response, "upsert_points")
    }

    pub fn search_points(
        &self,
        collection_name: &str,
        request: &SearchRequest,
    ) -> Result<SearchResponse, VectorError> {
        let request_fn = || {
            self.create_request(
                Method::POST,
                &format!("/collections/{}/points/search", collection_name),
            )
            .json(request)
            .send()
        };

        let response = self.execute_with_retry_sync(request_fn)?;
        parse_response(response, "search_points")
    }

    pub fn get_points(
        &self,
        collection_name: &str,
        request: &GetPointsRequest,
    ) -> Result<GetPointsResponse, VectorError> {
        let request_fn = || {
            self.create_request(
                Method::POST,
                &format!("/collections/{}/points", collection_name),
            )
            .json(request)
            .send()
        };

        let response = self.execute_with_retry_sync(request_fn)?;
        parse_response(response, "get_points")
    }

    pub fn delete_points(
        &self,
        collection_name: &str,
        request: &DeletePointsRequest,
    ) -> Result<DeletePointsResponse, VectorError> {
        let request_fn = || {
            self.create_request(
                Method::POST,
                &format!("/collections/{}/points/delete", collection_name),
            )
            .json(request)
            .send()
        };

        let response = self.execute_with_retry_sync(request_fn)?;
        parse_response(response, "delete_points")
    }

    pub fn scroll_points(
        &self,
        collection_name: &str,
        request: &ScrollRequest,
    ) -> Result<ScrollResponse, VectorError> {
        let request_fn = || {
            self.create_request(
                Method::POST,
                &format!("/collections/{}/points/scroll", collection_name),
            )
            .json(request)
            .send()
        };

        let response = self.execute_with_retry_sync(request_fn)?;
        parse_response(response, "scroll_points")
    }

    pub fn count_points(
        &self,
        collection_name: &str,
        request: &CountRequest,
    ) -> Result<CountResponse, VectorError> {
        let request_fn = || {
            self.create_request(
                Method::POST,
                &format!("/collections/{}/points/count", collection_name),
            )
            .json(request)
            .send()
        };

        let response = self.execute_with_retry_sync(request_fn)?;
        parse_response(response, "count_points")
    }

    pub fn batch_search(
        &self,
        collection_name: &str,
        request: &BatchSearchRequest,
    ) -> Result<BatchSearchResponse, VectorError> {
        let request_fn = || {
            self.create_request(
                Method::POST,
                &format!("/collections/{}/points/search/batch", collection_name),
            )
            .json(request)
            .send()
        };

        let response = self.execute_with_retry_sync(request_fn)?;
        parse_response(response, "batch_search")
    }

    pub fn recommend_points(
        &self,
        collection_name: &str,
        request: &RecommendRequest,
    ) -> Result<RecommendResponse, VectorError> {
        let request_fn = || {
            self.create_request(
                Method::POST,
                &format!("/collections/{}/points/recommend", collection_name),
            )
            .json(request)
            .send()
        };

        let response = self.execute_with_retry_sync(request_fn)?;
        parse_response(response, "recommend_points")
    }

    pub fn discover_points(
        &self,
        collection_name: &str,
        request: &DiscoverRequest,
    ) -> Result<DiscoverResponse, VectorError> {
        let request_fn = || {
            self.create_request(
                Method::POST,
                &format!("/collections/{}/points/discover", collection_name),
            )
            .json(request)
            .send()
        };

        let response = self.execute_with_retry_sync(request_fn)?;
        parse_response(response, "discover_points")
    }

    pub fn create_field_index(
        &self,
        collection_name: &str,
        field_name: &str,
        field_type: &str,
    ) -> Result<(), VectorError> {
        trace!(
            "Creating index for field {} in collection: {}",
            field_name,
            collection_name
        );

        let request = serde_json::json!({
            "field_name": field_name,
            "field_schema": field_type
        });

        let response = self.execute_with_retry_sync(|| {
            self.create_request(
                Method::PUT,
                &format!("/collections/{}/index", collection_name),
            )
            .json(&request)
            .send()
        })?;

        parse_response::<serde_json::Value>(response, "create_field_index")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListCollectionsResponse {
    pub result: CollectionsResult,
    pub status: String,
    pub time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionsResult {
    pub collections: Vec<CollectionDescription>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionDescription {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCollectionRequest {
    pub collection_name: String,
    pub config: CollectionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<CollectionParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hnsw_config: Option<HnswConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wal_config: Option<WalConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimizer_config: Option<OptimizersConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantization_config: Option<QuantizationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vectors: Option<VectorConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shard_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_disk_payload: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vectors: Option<VectorConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shard_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sharding_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replication_factor: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_consistency_factor: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_fan_out_factor: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_disk_payload: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sparse_vectors: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VectorConfig {
    Single(VectorParams),
    Multiple(HashMap<String, VectorParams>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorParams {
    pub size: u32,
    pub distance: Distance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hnsw_config: Option<HnswConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantization_config: Option<QuantizationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_disk: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Distance {
    Cosine,
    Euclid,
    Dot,
    Manhattan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub m: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ef_construct: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_scan_threshold: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_indexing_threads: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_disk: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_m: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wal_capacity_mb: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wal_segments_ahead: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizersConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_threshold: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vacuum_min_vector_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_segment_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_segment_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memmap_threshold: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexing_threshold: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flush_interval_sec: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_optimization_threads: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum QuantizationConfig {
    Scalar(ScalarQuantization),
    Product(ProductQuantization),
    Binary(BinaryQuantization),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarQuantization {
    #[serde(rename = "type")]
    pub quantization_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantile: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_ram: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductQuantization {
    #[serde(rename = "type")]
    pub quantization_type: String,
    pub compression: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_ram: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryQuantization {
    #[serde(rename = "type")]
    pub quantization_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_ram: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCollectionResponse {
    pub result: bool,
    pub status: String,
    pub time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCollectionResponse {
    pub result: CollectionInfo,
    pub status: String,
    pub time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionInfo {
    pub status: String,
    pub optimizer_status: OptimizerStatus,
    pub vectors_count: Option<u64>,
    pub indexed_vectors_count: Option<u64>,
    pub points_count: Option<u64>,
    pub segments_count: u32,
    pub config: CollectionConfig,
    pub payload_schema: HashMap<String, PayloadFieldSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OptimizerStatus {
    String(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadFieldSchema {
    pub data_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteCollectionResponse {
    pub result: bool,
    pub status: String,
    pub time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertRequest {
    pub points: Vec<PointStruct>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ordering: Option<WriteOrdering>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointStruct {
    pub id: PointId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    pub vector: NamedVectors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PointId {
    Integer(u64),
    Uuid(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NamedVectors {
    Single(Vec<f32>),
    Multiple(HashMap<String, Vec<f32>>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteOrdering {
    #[serde(rename = "type")]
    pub ordering_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertResponse {
    pub result: UpdateResult,
    pub status: String,
    pub time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResult {
    pub operation_id: Option<u64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub vector: NamedVectorStruct,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<SearchParams>,
    pub limit: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_payload: Option<WithPayloadSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_vector: Option<WithVectorSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_threshold: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NamedVectorStruct {
    Default(Vec<f32>),
    Named { name: String, vector: Vec<f32> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub should: Option<Vec<Condition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub must: Option<Vec<Condition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub must_not: Option<Vec<Condition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Condition {
    Field(FieldCondition),
    HasId(HasIdCondition),
    Nested(Filter),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldCondition {
    pub key: String,
    #[serde(flatten)]
    pub condition: FieldConditionOneOf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldConditionOneOf {
    Match { r#match: MatchCondition },
    Range { range: RangeInterface },
    GeoBoundingBox { geo_bounding_box: GeoBoundingBox },
    GeoRadius { geo_radius: GeoRadius },
    GeoPolygon { geo_polygon: GeoPolygon },
    ValuesCount { values_count: ValuesCount },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchCondition {
    pub value: MatchValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MatchValue {
    String(String),
    Integer(i64),
    Boolean(bool),
    Strings(Vec<String>),
    Integers(Vec<i64>),
    Except(MatchExcept),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchExcept {
    pub except: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeInterface {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lt: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gt: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gte: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lte: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoBoundingBox {
    pub top_left: GeoPoint,
    pub bottom_right: GeoPoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoRadius {
    pub center: GeoPoint,
    pub radius: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoPolygon {
    pub exterior: GeoLineString,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interiors: Option<Vec<GeoLineString>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoPoint {
    pub lon: f64,
    pub lat: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLineString {
    pub points: Vec<GeoPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuesCount {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lt: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gt: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gte: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lte: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasIdCondition {
    pub has_id: Vec<PointId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hnsw_ef: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantization: Option<QuantizationSearchParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizationSearchParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rescore: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oversampling: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WithPayloadSelector {
    Bool(bool),
    Include(PayloadSelector),
    Exclude(PayloadSelectorExclude),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadSelector {
    pub include: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadSelectorExclude {
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WithVectorSelector {
    Bool(bool),
    Include(VectorSelector),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VectorSelector {
    Names(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub result: Vec<ScoredPoint>,
    pub status: String,
    pub time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredPoint {
    pub id: PointId,
    pub version: u64,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector: Option<NamedVectors>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPointsRequest {
    pub ids: Vec<PointId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_payload: Option<WithPayloadSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_vector: Option<WithVectorSelector>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPointsResponse {
    pub result: Vec<Record>,
    pub status: String,
    pub time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: PointId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector: Option<NamedVectors>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletePointsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points: Option<Vec<PointId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ordering: Option<WriteOrdering>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletePointsResponse {
    pub result: UpdateResult,
    pub status: String,
    pub time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<PointId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_payload: Option<WithPayloadSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_vector: Option<WithVectorSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_by: Option<OrderBy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBy {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<Direction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_from: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollResponse {
    pub result: ScrollResult,
    pub status: String,
    pub time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollResult {
    pub points: Vec<Record>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_offset: Option<PointId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountResponse {
    pub result: CountResult,
    pub status: String,
    pub time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountResult {
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSearchRequest {
    pub searches: Vec<SearchRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSearchResponse {
    pub result: Vec<Vec<ScoredPoint>>,
    pub status: String,
    pub time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendRequest {
    pub positive: Vec<RecommendExample>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative: Option<Vec<RecommendExample>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<SearchParams>,
    pub limit: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_payload: Option<WithPayloadSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_vector: Option<WithVectorSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_threshold: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub using: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lookup_from: Option<LookupLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RecommendExample {
    PointId(PointId),
    Vector(Vec<f32>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupLocation {
    pub collection: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendResponse {
    pub result: Vec<ScoredPoint>,
    pub status: String,
    pub time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverRequest {
    pub target: PointId,
    pub context: Vec<ContextPair>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<SearchParams>,
    pub limit: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_payload: Option<WithPayloadSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_vector: Option<WithVectorSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub using: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lookup_from: Option<LookupLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPair {
    pub positive: PointId,
    pub negative: PointId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverResponse {
    pub result: Vec<ScoredPoint>,
    pub status: String,
    pub time: f64,
}

// helper functions
fn from_qdrant_error_code(status: u16, message: &str) -> VectorError {
    match status {
        400 => VectorError::InvalidParams(message.to_string()),
        401 => VectorError::Unauthorized(message.to_string()),
        403 => VectorError::Unauthorized(message.to_string()),
        404 => VectorError::NotFound(message.to_string()),
        409 => VectorError::AlreadyExists(message.to_string()),
        422 => VectorError::InvalidParams(message.to_string()),
        429 => VectorError::RateLimited(message.to_string()),
        500..=599 => VectorError::ProviderError(format!("Server error ({}): {}", status, message)),
        _ => VectorError::ProviderError(format!("HTTP error ({}): {}", status, message)),
    }
}

fn handle_qdrant_error(response: Response, operation: &str) -> VectorError {
    let status = response.status().as_u16();
    let error_message = match response.text() {
        Ok(body) => {
            if let Ok(error_obj) = serde_json::from_str::<serde_json::Value>(&body) {
                error_obj
                    .get("status")
                    .and_then(|s| s.get("error"))
                    .and_then(|e| e.as_str())
                    .unwrap_or(&body)
                    .to_string()
            } else {
                body
            }
        }
        Err(_) => format!("HTTP {} error during {}", status, operation),
    };

    trace!("Qdrant API error: {} - {}", status, error_message);
    from_qdrant_error_code(status, &error_message)
}

fn parse_response<T: DeserializeOwned + Debug>(
    response: Response,
    operation: &str,
) -> Result<T, VectorError> {
    let status = response.status();

    if !status.is_success() {
        return Err(handle_qdrant_error(response, operation));
    }

    match response.text() {
        Ok(body) => {
            trace!("Qdrant API response for {}: {}", operation, body);
            serde_json::from_str(&body).map_err(|e| {
                VectorError::ProviderError(format!(
                    "Failed to parse response for {}: {}",
                    operation, e
                ))
            })
        }
        Err(e) => Err(VectorError::ProviderError(format!(
            "Failed to read response for {}: {}",
            operation, e
        ))),
    }
}

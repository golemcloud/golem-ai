use golem_vector::config::{get_max_retries_config, get_timeout_config};
use golem_vector::golem::vector::types::VectorError;
use log::trace;
use reqwest::{Client, Method, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::time::Duration;

/// Milvus Vector API client
/// based on https://milvus.io/docs
#[derive(Clone)]
pub struct MilvusClient {
    client: Client,
    base_url: String,
    token: Option<String>,
    database: String,
}

impl MilvusClient {
    pub fn new(uri: String, token: Option<String>, database: Option<String>) -> Self {
        let timeout_secs = get_timeout_config();

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to initialize HTTP client");

        let base_url = if uri.ends_with('/') {
            uri.trim_end_matches('/').to_string()
        } else {
            uri
        };

        Self {
            client,
            base_url,
            token,
            database: database.unwrap_or_else(|| "_default".to_string()),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn database(&self) -> &str {
        &self.database
    }

    fn create_request(&self, method: Method, endpoint: &str) -> RequestBuilder {
        let url = format!("{}{}", self.base_url, endpoint);
        let mut request = self
            .client
            .request(method, &url)
            .header("accept", "application/json")
            .header("content-type", "application/json");

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        request
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
                Ok(response) => return Ok(response),
                Err(error) => {
                    trace!("Request attempt {} failed: {}", attempt, error);
                    let _error_string = error.to_string();
                    last_error = Some(error);

                    if attempt < max_retries
                        && self.should_retry_error(last_error.as_ref().unwrap())
                    {
                        let is_rate_limited = last_error
                            .as_ref()
                            .and_then(|e| e.status())
                            .map(|s| s.as_u16() == 429)
                            .unwrap_or(false);

                        let delay = Self::calculate_backoff_delay(attempt, is_rate_limited);
                        trace!("Retrying in {:?}", delay);
                        std::thread::sleep(delay);
                    } else {
                        break;
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

    pub fn list_collections(&self) -> Result<ListCollectionsResponse, VectorError> {
        trace!("Listing collections");

        let request = ListCollectionsRequest {
            db_name: self.database.clone(),
        };

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, "/v2/vectordb/collections/list")
                .json(&request)
                .send()
        })?;

        parse_response(response, "list_collections")
    }

    pub fn create_collection(
        &self,
        request: &CreateCollectionRequest,
    ) -> Result<CreateCollectionResponse, VectorError> {
        trace!("Creating collection: {}", request.collection_name);

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, "/v2/vectordb/collections/create")
                .json(request)
                .send()
        })?;

        parse_response(response, "create_collection")
    }

    pub fn describe_collection(
        &self,
        collection_name: &str,
    ) -> Result<DescribeCollectionResponse, VectorError> {
        trace!("Describing collection: {collection_name}");

        let request = DescribeCollectionRequest {
            db_name: self.database.clone(),
            collection_name: collection_name.to_string(),
        };

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, "/v2/vectordb/collections/describe")
                .json(&request)
                .send()
        })?;

        parse_response(response, "describe_collection")
    }

    pub fn drop_collection(
        &self,
        collection_name: &str,
    ) -> Result<DropCollectionResponse, VectorError> {
        trace!("Dropping collection: {collection_name}");

        let request = DropCollectionRequest {
            db_name: self.database.clone(),
            collection_name: collection_name.to_string(),
        };

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, "/v2/vectordb/collections/drop")
                .json(&request)
                .send()
        })?;

        parse_response(response, "drop_collection")
    }

    pub fn has_collection(
        &self,
        collection_name: &str,
    ) -> Result<HasCollectionResponse, VectorError> {
        trace!("Checking if collection exists: {collection_name}");

        let request = HasCollectionRequest {
            db_name: self.database.clone(),
            collection_name: collection_name.to_string(),
        };

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, "/v2/vectordb/collections/has")
                .json(&request)
                .send()
        })?;

        parse_response(response, "has_collection")
    }

    pub fn load_collection(
        &self,
        collection_name: &str,
    ) -> Result<LoadCollectionResponse, VectorError> {
        trace!("Loading collection: {collection_name}");

        let request = LoadCollectionRequest {
            db_name: self.database.clone(),
            collection_name: collection_name.to_string(),
        };

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, "/v2/vectordb/collections/load")
                .json(&request)
                .send()
        })?;

        parse_response(response, "load_collection")
    }

    pub fn release_collection(
        &self,
        collection_name: &str,
    ) -> Result<ReleaseCollectionResponse, VectorError> {
        trace!("Releasing collection: {collection_name}");

        let request = ReleaseCollectionRequest {
            db_name: self.database.clone(),
            collection_name: collection_name.to_string(),
        };

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, "/v2/vectordb/collections/release")
                .json(&request)
                .send()
        })?;

        parse_response(response, "release_collection")
    }

    pub fn upsert(&self, request: &UpsertRequest) -> Result<UpsertResponse, VectorError> {
        trace!(
            "Upserting {} vectors to collection: {}",
            request.data.len(),
            request.collection_name
        );

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, "/v2/vectordb/entities/upsert")
                .json(request)
                .send()
        })?;

        parse_response(response, "upsert")
    }

    pub fn search(&self, request: &SearchRequest) -> Result<SearchResponse, VectorError> {
        trace!(
            "Searching vectors in collection: {}",
            request.collection_name
        );

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, "/v2/vectordb/entities/search")
                .json(request)
                .send()
        })?;

        parse_response(response, "search")
    }

    pub fn query(&self, request: &QueryRequest) -> Result<QueryResponse, VectorError> {
        trace!(
            "Querying vectors in collection: {}",
            request.collection_name
        );

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, "/v2/vectordb/entities/query")
                .json(request)
                .send()
        })?;

        parse_response(response, "query")
    }

    pub fn get(&self, request: &GetRequest) -> Result<GetResponse, VectorError> {
        trace!(
            "Getting vectors by IDs from collection: {}",
            request.collection_name
        );

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, "/v1/vector/get")
                .json(request)
                .send()
        })?;

        parse_response(response, "get")
    }

    pub fn delete(&self, request: &DeleteRequest) -> Result<DeleteResponse, VectorError> {
        trace!(
            "Deleting vectors from collection: {}",
            request.collection_name
        );

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, "/v1/vector/delete")
                .json(request)
                .send()
        })?;

        parse_response(response, "delete")
    }

    pub fn get_collection_stats(
        &self,
        collection_name: &str,
    ) -> Result<GetCollectionStatsResponse, VectorError> {
        trace!("Getting stats for collection: {collection_name}");

        let request = GetCollectionStatsRequest {
            db_name: self.database.clone(),
            collection_name: collection_name.to_string(),
        };

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, "/v2/vectordb/collections/get_stats")
                .json(&request)
                .send()
        })?;

        parse_response(response, "get_collection_stats")
    }

    pub fn create_partition(
        &self,
        collection_name: &str,
        partition_name: &str,
    ) -> Result<CreatePartitionResponse, VectorError> {
        trace!(
            "Creating partition: {} in collection: {}",
            partition_name,
            collection_name
        );

        let request = CreatePartitionRequest {
            db_name: self.database.clone(),
            collection_name: collection_name.to_string(),
            partition_name: partition_name.to_string(),
        };

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, "/v2/vectordb/partitions/create")
                .json(&request)
                .send()
        })?;

        parse_response(response, "create_partition")
    }

    pub fn drop_partition(
        &self,
        collection_name: &str,
        partition_name: &str,
    ) -> Result<DropPartitionResponse, VectorError> {
        trace!(
            "Dropping partition: {} from collection: {}",
            partition_name,
            collection_name
        );

        let request = DropPartitionRequest {
            db_name: self.database.clone(),
            collection_name: collection_name.to_string(),
            partition_name: partition_name.to_string(),
        };

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, "/v2/vectordb/partitions/drop")
                .json(&request)
                .send()
        })?;

        parse_response(response, "drop_partition")
    }

    pub fn list_partitions(
        &self,
        collection_name: &str,
    ) -> Result<ListPartitionsResponse, VectorError> {
        trace!("Listing partitions in collection: {}", collection_name);

        let request = ListPartitionsRequest {
            db_name: self.database.clone(),
            collection_name: collection_name.to_string(),
        };

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, "/v2/vectordb/partitions/list")
                .json(&request)
                .send()
        })?;

        parse_response(response, "list_partitions")
    }

    pub fn has_partition(
        &self,
        collection_name: &str,
        partition_name: &str,
    ) -> Result<HasPartitionResponse, VectorError> {
        trace!(
            "Checking if partition exists: {} in collection: {}",
            partition_name,
            collection_name
        );

        let request = HasPartitionRequest {
            db_name: self.database.clone(),
            collection_name: collection_name.to_string(),
            partition_name: partition_name.to_string(),
        };

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, "/v2/vectordb/partitions/has")
                .json(&request)
                .send()
        })?;

        parse_response(response, "has_partition")
    }

    pub fn load_partitions(
        &self,
        collection_name: &str,
        partition_names: Vec<String>,
    ) -> Result<LoadPartitionsResponse, VectorError> {
        trace!(
            "Loading partitions: {:?} in collection: {}",
            partition_names,
            collection_name
        );

        let request = LoadPartitionsRequest {
            db_name: self.database.clone(),
            collection_name: collection_name.to_string(),
            partition_names,
        };

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, "/v2/vectordb/partitions/load")
                .json(&request)
                .send()
        })?;

        parse_response(response, "load_partitions")
    }

    pub fn release_partitions(
        &self,
        collection_name: &str,
        partition_names: Vec<String>,
    ) -> Result<ReleasePartitionsResponse, VectorError> {
        trace!(
            "Releasing partitions: {:?} in collection: {}",
            partition_names,
            collection_name
        );

        let request = ReleasePartitionsRequest {
            db_name: self.database.clone(),
            collection_name: collection_name.to_string(),
            partition_names,
        };

        let response = self.execute_with_retry_sync(|| {
            self.create_request(Method::POST, "/v2/vectordb/partitions/release")
                .json(&request)
                .send()
        })?;

        parse_response(response, "release_partitions")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListCollectionsRequest {
    #[serde(rename = "dbName")]
    pub db_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListCollectionsResponse {
    pub code: i32,
    pub data: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCollectionRequest {
    #[serde(rename = "dbName")]
    pub db_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
    pub dimension: u32,
    #[serde(rename = "metricType", skip_serializing_if = "Option::is_none")]
    pub metric_type: Option<String>,
    #[serde(rename = "primaryField", skip_serializing_if = "Option::is_none")]
    pub primary_field: Option<String>,
    #[serde(rename = "vectorField", skip_serializing_if = "Option::is_none")]
    pub vector_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "enableDynamicField", skip_serializing_if = "Option::is_none")]
    pub enable_dynamic_field: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<CollectionSchema>,
    #[serde(rename = "indexParams", skip_serializing_if = "Option::is_none")]
    pub index_params: Option<Vec<IndexParam>>,
    #[serde(rename = "vectorFieldType", skip_serializing_if = "Option::is_none")]
    pub vector_field_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCollectionResponse {
    pub code: i32,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescribeCollectionRequest {
    #[serde(rename = "dbName")]
    pub db_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescribeCollectionResponse {
    pub code: i32,
    pub data: CollectionInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionInfo {
    #[serde(rename = "collectionName")]
    pub collection_name: String,
    pub description: Option<String>,
    pub fields: Vec<FieldSchema>,
    pub indexes: Vec<IndexInfo>,
    pub load: String,
    #[serde(rename = "shardsNum")]
    pub shards_num: i32,
    #[serde(rename = "enableDynamicField")]
    pub enable_dynamic_field: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropCollectionRequest {
    #[serde(rename = "dbName")]
    pub db_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropCollectionResponse {
    pub code: i32,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasCollectionRequest {
    #[serde(rename = "dbName")]
    pub db_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasCollectionResponse {
    pub code: i32,
    pub data: HasCollectionData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasCollectionData {
    pub has: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadCollectionRequest {
    #[serde(rename = "dbName")]
    pub db_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadCollectionResponse {
    pub code: i32,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseCollectionRequest {
    #[serde(rename = "dbName")]
    pub db_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseCollectionResponse {
    pub code: i32,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSchema {
    #[serde(rename = "autoId", skip_serializing_if = "Option::is_none")]
    pub auto_id: Option<bool>,
    #[serde(rename = "enableDynamicField", skip_serializing_if = "Option::is_none")]
    pub enable_dynamic_field: Option<bool>,
    pub fields: Vec<FieldSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchema {
    #[serde(rename = "name")]
    pub field_name: String,
    #[serde(rename = "type")]
    pub data_type: String,
    #[serde(rename = "primaryKey", skip_serializing_if = "Option::is_none")]
    pub is_primary: Option<bool>,
    #[serde(rename = "elementDataType", skip_serializing_if = "Option::is_none")]
    pub element_data_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "params", skip_serializing_if = "Option::is_none")]
    pub element_type_params: Option<Vec<HashMap<String, serde_json::Value>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertRequest {
    #[serde(rename = "dbName")]
    pub db_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
    pub data: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertRequest {
    #[serde(rename = "dbName")]
    pub db_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
    pub data: Vec<serde_json::Value>,
    #[serde(rename = "partitionName", skip_serializing_if = "Option::is_none")]
    pub partition_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertResponse {
    pub code: i32,
    pub data: UpsertData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertData {
    #[serde(rename = "upsertCount")]
    pub upsert_count: u32,
    #[serde(rename = "upsertIds")]
    pub upsert_ids: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    #[serde(rename = "dbName")]
    pub db_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<Vec<f32>>>,
    #[serde(rename = "sparseFloatVectors", skip_serializing_if = "Option::is_none")]
    pub sparse_float_vectors: Option<Vec<SparseFloatVector>>,
    #[serde(rename = "binaryVectors", skip_serializing_if = "Option::is_none")]
    pub binary_vectors: Option<Vec<Vec<u8>>>,
    #[serde(rename = "annsField")]
    pub anns_field: String,
    #[serde(rename = "metricType")]
    pub metric_type: String,
    pub limit: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(rename = "outputFields", skip_serializing_if = "Option::is_none")]
    pub output_fields: Option<Vec<String>>,
    #[serde(rename = "searchParams", skip_serializing_if = "Option::is_none")]
    pub search_params: Option<HashMap<String, serde_json::Value>>,
    #[serde(rename = "partitionNames", skip_serializing_if = "Option::is_none")]
    pub partition_names: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub code: i32,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseFloatVector {
    pub indices: Vec<u32>,
    pub values: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: serde_json::Value,
    pub distance: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    #[serde(rename = "dbName")]
    pub db_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ids: Option<Vec<serde_json::Value>>,
    #[serde(rename = "outputFields", skip_serializing_if = "Option::is_none")]
    pub output_fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(rename = "partitionNames", skip_serializing_if = "Option::is_none")]
    pub partition_names: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub code: i32,
    pub data: Vec<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetRequest {
    #[serde(rename = "collectionName")]
    pub collection_name: String,
    pub id: Vec<serde_json::Value>,
    #[serde(rename = "outputFields", skip_serializing_if = "Option::is_none")]
    pub output_fields: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetResponse {
    pub code: i32,
    pub data: Vec<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRequest {
    #[serde(rename = "collectionName")]
    pub collection_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(rename = "partitionNames", skip_serializing_if = "Option::is_none")]
    pub partition_names: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResponse {
    pub code: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    #[serde(rename = "fieldName")]
    pub field_name: String,
    #[serde(rename = "indexName")]
    pub index_name: String,
    #[serde(rename = "metricType")]
    pub metric_type: String,
    #[serde(rename = "indexType", skip_serializing_if = "Option::is_none")]
    pub index_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexParam {
    #[serde(rename = "fieldName")]
    pub field_name: String,
    #[serde(rename = "indexName")]
    pub index_name: String,
    #[serde(rename = "metricType", skip_serializing_if = "Option::is_none")]
    pub metric_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCollectionStatsRequest {
    #[serde(rename = "dbName")]
    pub db_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCollectionStatsResponse {
    pub code: i32,
    pub data: CollectionStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionStats {
    #[serde(rename = "rowCount")]
    pub row_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePartitionRequest {
    #[serde(rename = "dbName")]
    pub db_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
    #[serde(rename = "partitionName")]
    pub partition_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePartitionResponse {
    pub code: i32,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropPartitionRequest {
    #[serde(rename = "dbName")]
    pub db_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
    #[serde(rename = "partitionName")]
    pub partition_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropPartitionResponse {
    pub code: i32,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPartitionsRequest {
    #[serde(rename = "dbName")]
    pub db_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPartitionsResponse {
    pub code: i32,
    pub data: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasPartitionRequest {
    #[serde(rename = "dbName")]
    pub db_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
    #[serde(rename = "partitionName")]
    pub partition_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasPartitionResponse {
    pub code: i32,
    pub data: HasPartitionData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasPartitionData {
    pub has: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadPartitionsRequest {
    #[serde(rename = "dbName")]
    pub db_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
    #[serde(rename = "partitionNames")]
    pub partition_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadPartitionsResponse {
    pub code: i32,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleasePartitionsRequest {
    #[serde(rename = "dbName")]
    pub db_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
    #[serde(rename = "partitionNames")]
    pub partition_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleasePartitionsResponse {
    pub code: i32,
    pub data: serde_json::Value,
}

fn from_milvus_error_code(error_code: i32, message: &str) -> VectorError {
    match error_code {
        0 => VectorError::ProviderError(format!(
            "Unexpected success code with error message: {}",
            message
        )),

        // client errors (1-999)
        1 => VectorError::ProviderError(format!("Unexpected error: {}", message)),
        100 => VectorError::ProviderError(format!("Unexpected error: {}", message)),

        // collection related errors (700-799)
        700 => VectorError::NotFound(format!("Collection not found: {}", message)),
        701 => VectorError::NotFound(format!("Collection not loaded: {}", message)),
        800 => VectorError::AlreadyExists(format!("Collection already exists: {}", message)),

        // partition related errors (900-999)
        900 => VectorError::NotFound(format!("Partition not found: {}", message)),
        901 => VectorError::AlreadyExists(format!("Partition already exists: {}", message)),

        // parameter/validation errors (1100-1199)
        1100 => VectorError::InvalidParams(format!("Illegal argument: {}", message)),
        1101 => VectorError::InvalidParams(format!("Invalid collection name: {}", message)),
        1102 => VectorError::InvalidParams(format!("Invalid field name: {}", message)),
        1103 => VectorError::InvalidParams(format!("Invalid dimension: {}", message)),
        1104 => VectorError::InvalidParams(format!("Invalid index type: {}", message)),
        1105 => VectorError::InvalidParams(format!("Invalid parameter: {}", message)),
        1106 => VectorError::InvalidParams(format!("Invalid index parameter: {}", message)),

        // index related errors (1200-1299)
        1200 => VectorError::NotFound(format!("Index not found: {}", message)),
        1201 => VectorError::AlreadyExists(format!("Index already exists: {}", message)),

        // data type errors (1300-1399)
        1300 => VectorError::InvalidParams(format!("Data type mismatch: {}", message)),
        1301 => VectorError::InvalidParams(format!("Invalid data type: {}", message)),

        // search/query errors (1400-1499)
        1400 => VectorError::InvalidParams(format!("Search parameter error: {}", message)),
        1401 => VectorError::InvalidParams(format!("Invalid search expression: {}", message)),

        // insert/upsert errors (1500-1599)
        1500 => VectorError::InvalidParams(format!("Insert parameter error: {}", message)),
        1501 => VectorError::InvalidParams(format!("Invalid insert data: {}", message)),

        // server/internal errors (65536 +)
        65536..=i32::MAX => {
            VectorError::ProviderError(format!("Milvus internal error {}: {}", error_code, message))
        }

        _ => VectorError::ProviderError(format!(
            "Unknown Milvus error code {}: {}",
            error_code, message
        )),
    }
}

fn handle_milvus_error(response: Response, operation: &str) -> VectorError {
    let status = response.status();

    if !status.is_success() {
        let error_body = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        trace!("HTTP error {status} for {operation}: {error_body:?}");

        return match status.as_u16() {
            400 => {
                VectorError::InvalidParams(format!("Bad request for {}: {}", operation, error_body))
            }
            401 => VectorError::Unauthorized(format!("Authentication failed for {}", operation)),
            404 => VectorError::NotFound(format!(
                "Resource not found for {}: {}",
                operation, error_body
            )),
            409 => VectorError::AlreadyExists(format!(
                "Resource already exists for {}: {}",
                operation, error_body
            )),
            429 => VectorError::RateLimited(format!("Rate limit exceeded for {}", operation)),
            500..=599 => VectorError::ProviderError(format!(
                "Server error for {}: {}",
                operation, error_body
            )),
            _ => VectorError::ProviderError(format!(
                "HTTP {} for {}: {}",
                status.as_u16(),
                operation,
                error_body
            )),
        };
    }

    match response.text() {
        Ok(body) => {
            trace!("Response body for {operation}: {body}");

            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&body) {
                if let Some(code) = json_value.get("code").and_then(|c| c.as_i64()) {
                    if code != 0 {
                        let message = json_value
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("No error message provided");
                        return from_milvus_error_code(code as i32, message);
                    }
                }
            }

            VectorError::ProviderError(format!(
                "Failed to parse response for {}: {}",
                operation, body
            ))
        }
        Err(e) => {
            trace!("Failed to read response body for {operation}: {e}");
            VectorError::ProviderError(format!(
                "Failed to read response body for {}: {}",
                operation, e
            ))
        }
    }
}

//parsing function

fn parse_response<T: DeserializeOwned + Debug>(
    response: Response,
    operation: &str,
) -> Result<T, VectorError> {
    let status = response.status();

    trace!("Received response from Milvus API for {operation}: {response:?}");

    if status.is_success() {
        match response.text() {
            Ok(body) => {
                trace!("Received response body from Milvus API for {operation}: {body:?}");

                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(code) = json_value.get("code").and_then(|c| c.as_i64()) {
                        if code != 0 && code != 200 {
                            let message = json_value
                                .get("message")
                                .and_then(|m| m.as_str())
                                .unwrap_or("No error message provided");
                            return Err(from_milvus_error_code(code as i32, message));
                        }
                    }
                }

                match serde_json::from_str::<T>(&body) {
                    Ok(parsed) => {
                        trace!("Successfully parsed response for {operation}: {parsed:?}");
                        Ok(parsed)
                    }
                    Err(parse_error) => {
                        trace!("Failed to parse response for {operation}: {parse_error}");
                        Err(VectorError::ProviderError(format!(
                            "Failed to parse Milvus response for {}: {}",
                            operation, parse_error
                        )))
                    }
                }
            }
            Err(body_error) => {
                trace!("Failed to read response body for {operation}: {body_error}");
                Err(VectorError::ProviderError(format!(
                    "Failed to read Milvus response body for {}: {}",
                    operation, body_error
                )))
            }
        }
    } else {
        Err(handle_milvus_error(response, operation))
    }
}

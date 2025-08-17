//! Minimal synchronous Milvus REST client (v1 API).
//!
//! For native builds a blocking `reqwest` client is used. WebAssembly builds
//! return an `unsupported_feature` error because outbound HTTP is unavailable in
//! the sandboxed runtime.

use golem_vector::error::unsupported_feature;
use golem_vector::exports::golem::vector::types::{
    DistanceMetric, MetadataValue, VectorData, VectorError, VectorRecord,
};

use crate::conversion::{
    filter_expression_to_milvus, metadata_to_json_map, metric_to_milvus, vector_data_to_dense,
};

#[cfg(target_family = "wasm")]
pub struct MilvusClient;

#[cfg(target_family = "wasm")]
impl MilvusClient {
    pub fn new(_endpoint: String, _api_key: Option<String>) -> Self {
        MilvusClient
    }

    fn err<T>() -> Result<T, VectorError> {
        Err(unsupported_feature(
            "Milvus provider not available in wasm runtime",
        ))
    }

    pub fn create_collection(
        &self,
        _name: &str,
        _dimension: u32,
        _metric: DistanceMetric,
    ) -> Result<(), VectorError> {
        Self::err()
    }
    pub fn list_collections(&self) -> Result<Vec<String>, VectorError> {
        Self::err()
    }
    pub fn delete_collection(&self, _name: &str) -> Result<(), VectorError> {
        Self::err()
    }
    pub fn upsert_vectors(
        &self,
        _name: &str,
        _records: Vec<VectorRecord>,
    ) -> Result<(), VectorError> {
        Self::err()
    }
    pub fn query_vectors(
        &self,
        _name: &str,
        _query: Vec<f32>,
        _metric: DistanceMetric,
        _limit: u32,
        _expr: Option<String>,
    ) -> Result<Vec<(String, f32, Option<Vec<f32>>)>, VectorError> {
        Self::err()
    }

    pub fn get_vectors_by_ids(
        &self,
        _name: &str,
        _ids: Vec<String>,
    ) -> Result<Vec<VectorRecord>, VectorError> {
        Self::err()
    }
}

// ---------------------------- native impl ------------------------------
#[cfg(not(target_family = "wasm"))]
mod native {
    use super::*;
    use reqwest::blocking::{Client, Response};
    use serde::{Deserialize, Serialize, de::DeserializeOwned};
    use std::time::Duration;

    #[derive(Clone)]
    pub struct MilvusClient {
        http: Client,
        base_url: String,
        api_key: Option<String>,
        timeout: Duration,
        max_retries: u32,
    }

    impl MilvusClient {
        pub fn new(endpoint: String, api_key: Option<String>) -> Self {
            Self::new_with_config(endpoint, api_key, Duration::from_secs(30), 3)
        }

        pub fn new_with_config(
            endpoint: String, 
            api_key: Option<String>,
            timeout: Duration,
            max_retries: u32
        ) -> Self {
            let client = Client::builder()
                .timeout(timeout)
                .build()
                .expect("Failed to create HTTP client");
            
            Self {
                http: client,
                base_url: endpoint.trim_end_matches('/').to_string(),
                api_key,
                timeout,
                max_retries,
            }
        }

        // Helper method for consistent response handling
        fn handle_response<T: DeserializeOwned>(&self, resp: Response, operation: &str) -> Result<T, VectorError> {
            match resp.status() {
                status if status.is_success() => {
                    resp.json().map_err(|e| VectorError::ProviderError(
                        format!("Milvus {} response parsing error: {}", operation, e)
                    ))
                }
                status => {
                    let error_body = resp.text().unwrap_or_default();
                    Err(VectorError::ProviderError(
                        format!("Milvus {} error {}: {}", operation, status, error_body)
                    ))
                }
            }
        }

        // Helper method for retry logic
        fn with_retry<F, T>(&self, mut operation: F) -> Result<T, VectorError>
        where
            F: FnMut() -> Result<T, VectorError>,
        {
            for attempt in 0..=self.max_retries {
                match operation() {
                    Ok(result) => return Ok(result),
                    Err(e) if attempt == self.max_retries => return Err(e),
                    Err(_) => std::thread::sleep(Duration::from_millis(100 * (attempt + 1) as u64)),
                }
            }
            unreachable!()
        }

        // -------------------- collections ---------------------------
        pub fn create_collection(
            &self,
            name: &str,
            dimension: u32,
            metric: DistanceMetric,
        ) -> Result<(), VectorError> {
            #[derive(Serialize)]
            struct Payload<'a> {
                collection_name: &'a str,
                dimension: u32,
                metric_type: &'a str,
            }
            let url = format!("{}/v1/vector/collections", self.base_url);
            let resp = self
                .http
                .post(url)
                .json(&Payload {
                    collection_name: name,
                    dimension,
                    metric_type: metric_to_milvus(metric),
                })
                .send()
                .map_err(to_err)?;
            self.handle_response::<serde_json::Value>(resp, "create_collection")?;
            Ok(())
        }

        pub fn list_collections(&self) -> Result<Vec<String>, VectorError> {
            #[derive(Deserialize)]
            struct ListResp {
                data: Vec<CollectionItem>,
            }
            #[derive(Deserialize)]
            struct CollectionItem {
                collection_name: String,
            }
            let url = format!("{}/v1/vector/collections", self.base_url);
            let resp = self.http.get(url).send().map_err(to_err)?;
            let body: ListResp = self.handle_response(resp, "list_collections")?;
            Ok(body.data.into_iter().map(|c| c.collection_name).collect())
        }

        pub fn delete_collection(&self, name: &str) -> Result<(), VectorError> {
            let url = format!("{}/v1/vector/collections/{}", self.base_url, name);
            let resp = self.http.delete(url).send().map_err(to_err)?;
            self.handle_response::<serde_json::Value>(resp, "delete_collection")?;
            Ok(())
        }

        // -------------------- vectors ------------------------------
        pub fn upsert_vectors(
            &self,
            name: &str,
            records: Vec<VectorRecord>,
        ) -> Result<(), VectorError> {
            #[derive(Serialize)]
            struct VectorDataPayload<'a> {
                id: &'a str,
                vector: &'a [f32],
                metadata: Option<&'a serde_json::Value>,
            }
            let url = format!("{}/v1/vector/insert", self.base_url);
            let mut payloads = Vec::new();
            for rec in &records {
                let dense = vector_data_to_dense(rec.vector.clone())?;
                let meta_json = metadata_to_json_map(rec.metadata.clone());
                let meta_ref = meta_json.as_ref().map(|m| m as &serde_json::Value);
                payloads.push(VectorDataPayload {
                    id: &rec.id,
                    vector: &dense,
                    metadata: meta_ref,
                });
            }
            let resp = self.http.post(url).json(&payloads).send().map_err(to_err)?;
            self.handle_response::<serde_json::Value>(resp, "upsert_vectors")?;
            Ok(())
        }

        pub fn query_vectors(
            &self,
            name: &str,
            query: Vec<f32>,
            metric: DistanceMetric,
            limit: u32,
            expr: Option<String>,
        ) -> Result<Vec<(String, f32, Option<Vec<f32>>)>, VectorError> {
            #[derive(Serialize)]
            struct Query<'a> {
                collection_name: &'a str,
                vector: &'a [f32],
                top_k: u32,
                metric_type: &'a str,
                expr: Option<&'a str>,
            }
            #[derive(Deserialize)]
            struct Resp {
                data: Vec<ResultItem>,
            }
            #[derive(Deserialize)]
            struct ResultItem {
                id: String,
                distance: f32,
            }
            let url = format!("{}/v1/vector/search", self.base_url);
            let payload = Query {
                collection_name: name,
                vector: &query,
                top_k: limit,
                metric_type: metric_to_milvus(metric),
                expr: expr.as_deref(),
            };
            let resp = self.http.post(url).json(&payload).send().map_err(to_err)?;
            let body: Resp = self.handle_response(resp, "query_vectors")?;
            Ok(body
                .data
                .into_iter()
                .map(|r| (r.id, r.distance, None))
                .collect())
        }

        pub fn get_vectors_by_ids(
            &self,
            name: &str,
            ids: Vec<String>,
        ) -> Result<Vec<VectorRecord>, VectorError> {
            if ids.is_empty() {
                return Ok(Vec::new());
            }

            #[derive(Serialize)]
            struct GetReq<'a> {
                collection_name: &'a str,
                ids: &'a [String],
            }
            #[derive(Deserialize)]
            struct GetResp {
                data: Vec<GetItem>,
            }
            #[derive(Deserialize)]
            struct GetItem {
                id: String,
                vector: Vec<f32>,
                #[serde(default)]
                metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
            }

            let url = format!("{}/v1/vector/get", self.base_url);
            let payload = GetReq {
                collection_name: name,
                ids: &ids,
            };
            let resp = self.http.post(url).json(&payload).send().map_err(to_err)?;
            let body: GetResp = self.handle_response(resp, "get_vectors_by_ids")?;
            Ok(
                body
                    .data
                    .into_iter()
                    .map(|it| VectorRecord {
                        id: it.id,
                        vector: VectorData::Dense(it.vector),
                        metadata: it.metadata.map(|m| {
                            m.into_iter()
                                .map(|(k, v)| (k, MetadataValue::StringVal(v.to_string())))
                                .collect()
                        }),
                    })
                    .collect(),
            )
        }
    }

    fn to_err(e: impl std::fmt::Display) -> VectorError {
        VectorError::ProviderError(e.to_string())
    }
}

#[cfg(not(target_family = "wasm"))]
pub use native::MilvusClient;

//! Minimal synchronous Milvus REST client (v1 API).
//!
//! For native builds a blocking `reqwest` client is used. WebAssembly builds
//! return an `unsupported_feature` error because outbound HTTP is unavailable in
//! the sandboxed runtime.

use golem_vector::error::unsupported_feature;
use golem_vector::exports::golem::vector::types::{DistanceMetric, VectorError, VectorRecord};

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
}

// ---------------------------- native impl ------------------------------
#[cfg(not(target_family = "wasm"))]
mod native {
    use super::*;
    use reqwest::blocking::Client;
    use serde::{Deserialize, Serialize};

    #[derive(Clone)]
    pub struct MilvusClient {
        http: Client,
        base_url: String,
        api_key: Option<String>,
    }

    impl MilvusClient {
        pub fn new(endpoint: String, api_key: Option<String>) -> Self {
            Self {
                http: Client::new(),
                base_url: endpoint.trim_end_matches('/').to_string(),
                api_key,
            }
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
            if resp.status().is_success() {
                Ok(())
            } else {
                Err(VectorError::ProviderError(format!(
                    "Milvus error: {}",
                    resp.status()
                )))
            }
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
            if resp.status().is_success() {
                let body: ListResp = resp.json().map_err(to_err)?;
                Ok(body.data.into_iter().map(|c| c.collection_name).collect())
            } else {
                Err(VectorError::ProviderError(format!(
                    "Milvus error: {}",
                    resp.status()
                )))
            }
        }

        pub fn delete_collection(&self, name: &str) -> Result<(), VectorError> {
            let url = format!("{}/v1/vector/collections/{}", self.base_url, name);
            let resp = self.http.delete(url).send().map_err(to_err)?;
            if resp.status().is_success() {
                Ok(())
            } else {
                Err(VectorError::ProviderError(format!(
                    "Milvus error: {}",
                    resp.status()
                )))
            }
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
            if resp.status().is_success() {
                Ok(())
            } else {
                Err(VectorError::ProviderError(format!(
                    "Milvus error: {}",
                    resp.status()
                )))
            }
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
            if resp.status().is_success() {
                let body: Resp = resp.json().map_err(to_err)?;
                Ok(body
                    .data
                    .into_iter()
                    .map(|r| (r.id, r.distance, None))
                    .collect())
            } else {
                Err(VectorError::ProviderError(format!(
                    "Milvus error: {}",
                    resp.status()
                )))
            }
        }
    }

    fn to_err(e: impl std::fmt::Display) -> VectorError {
        VectorError::ProviderError(e.to_string())
    }
}

#[cfg(not(target_family = "wasm"))]
pub use native::MilvusClient;

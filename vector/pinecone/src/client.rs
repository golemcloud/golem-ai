//! Minimal synchronous REST client for the Pinecone API.
//!
//! This is **not** feature-complete but implements the few endpoints that the
//! current WIT component uses (`create_index`, `list_indexes`, `delete_index`,
//! `upsert_vectors`, `query`).  All requests are blocking and use the
//! [`reqwest`](https://docs.rs/reqwest) crate.

use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use golem_vector::exports::golem::vector::types::{
    FilterExpression, VectorData, VectorError, VectorRecord,
};

use crate::conversion::{metadata_to_json_map, metric_to_pinecone, vector_data_to_dense};

#[derive(Clone)]
pub struct PineconeApi {
    http: Client,
    /// Base URL of the Pinecone controller or index service, **without** trailing slash.
    base_url: String,
    api_key: Option<String>,
}

impl PineconeApi {
    pub fn new(base_url: impl Into<String>, api_key: Option<String>) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
        }
    }

    fn auth_headers(&self, headers: &mut reqwest::header::HeaderMap) {
        if let Some(key) = &self.api_key {
            headers.insert("Api-Key", key.parse().unwrap());
        }
    }

    // -------------------------- index management ---------------------------
    pub fn create_index(
        &self,
        name: &str,
        dimension: u32,
        metric: &str,
    ) -> Result<(), VectorError> {
        #[derive(Serialize)]
        struct Payload<'a> {
            name: &'a str,
            dimension: u32,
            metric: &'a str,
        }
        let url = format!("{}/databases", self.base_url);
        let mut req = self.http.post(url);
        if let Some(key) = &self.api_key {
            req = req.header("Api-Key", key);
        }
        let resp = req
            .json(&Payload {
                name,
                dimension,
                metric,
            })
            .send()
            .map_err(to_vector_error)?;
        match resp.status() {
            StatusCode::OK | StatusCode::CREATED => Ok(()),
            code => Err(VectorError::ProviderError(format!(
                "Pinecone error: {}",
                code
            ))),
        }
    }

    pub fn list_indexes(&self) -> Result<Vec<String>, VectorError> {
        let url = format!("{}/databases", self.base_url);
        let mut req = self.http.get(url);
        if let Some(key) = &self.api_key {
            req = req.header("Api-Key", key);
        }
        let resp = req.send().map_err(to_vector_error)?;
        match resp.status() {
            StatusCode::OK => {
                let list: Vec<String> = resp.json().map_err(to_vector_error)?;
                Ok(list)
            }
            code => Err(VectorError::ProviderError(format!(
                "Pinecone error: {}",
                code
            ))),
        }
    }

    pub fn delete_index(&self, name: &str) -> Result<(), VectorError> {
        let url = format!("{}/databases/{}", self.base_url, name);
        let mut req = self.http.delete(url);
        if let Some(key) = &self.api_key {
            req = req.header("Api-Key", key);
        }
        let resp = req.send().map_err(to_vector_error)?;
        match resp.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
            code => Err(VectorError::ProviderError(format!(
                "Pinecone error: {}",
                code
            ))),
        }
    }

    // ------------------------------ vectors --------------------------------

    pub fn upsert_vectors(
        &self,
        index_host: &str,
        vectors: Vec<VectorRecord>,
        namespace: Option<String>,
    ) -> Result<(), VectorError> {
        #[derive(Serialize)]
        struct Vector<'a> {
            id: &'a str,
            values: &'a [f32],
            #[serde(skip_serializing_if = "Option::is_none")]
            metadata: Option<&'a HashMap<String, serde_json::Value>>,
        }

        #[derive(Serialize)]
        struct Payload<'a> {
            vectors: Vec<Vector<'a>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            namespace: Option<String>,
        }

        let mut out_vecs = Vec::with_capacity(vectors.len());
        let mut payload_maps: Vec<HashMap<String, serde_json::Value>> =
            Vec::with_capacity(vectors.len());
        // We need to keep payload maps alive until serialization completes.
        for rec in &vectors {
            let dense = vector_data_to_dense(rec.vector.clone())?;
            let meta_map = metadata_to_json_map(rec.metadata.clone());
            payload_maps.push(meta_map.clone());
            out_vecs.push(Vector {
                id: &rec.id,
                values: &dense,
                metadata: if meta_map.is_empty() {
                    None
                } else {
                    Some(&meta_map)
                },
            });
        }

        let url = format!("{}/vectors/upsert", index_host.trim_end_matches('/'));
        let mut req = self.http.post(url);
        if let Some(key) = &self.api_key {
            req = req.header("Api-Key", key);
        }
        let payload = Payload {
            vectors: out_vecs,
            namespace,
        };
        let resp = req.json(&payload).send().map_err(to_vector_error)?;
        match resp.status() {
            StatusCode::OK | StatusCode::ACCEPTED => Ok(()),
            code => Err(VectorError::ProviderError(format!(
                "Pinecone upsert error: {}",
                code
            ))),
        }
    }

    pub fn query(
        &self,
        index_host: &str,
        vector: Vec<f32>,
        top_k: u32,
        namespace: Option<String>,
        filter: Option<serde_json::Value>,
        include_values: bool,
        include_metadata: bool,
    ) -> Result<Vec<QueryMatch>, VectorError> {
        #[derive(Serialize)]
        struct Payload<'a> {
            vector: &'a [f32],
            top_k: u32,
            #[serde(skip_serializing_if = "Option::is_none")]
            filter: Option<&'a serde_json::Value>,
            #[serde(skip_serializing_if = "Option::is_none")]
            namespace: Option<String>,
            #[serde(skip_serializing_if = "std::ops::Not::not")]
            include_values: bool,
            #[serde(skip_serializing_if = "std::ops::Not::not")]
            include_metadata: bool,
        }

        let url = format!("{}/query", index_host.trim_end_matches('/'));
        let mut req = self.http.post(url);
        if let Some(key) = &self.api_key {
            req = req.header("Api-Key", key);
        }
        let payload = Payload {
            vector: &vector,
            top_k,
            filter: filter.as_ref(),
            namespace,
            include_values,
            include_metadata,
        };
        let resp = req.json(&payload).send().map_err(to_vector_error)?;
        match resp.status() {
            StatusCode::OK => {
                let response: QueryResponse = resp.json().map_err(to_vector_error)?;
                Ok(response.matches)
            }
            code => Err(VectorError::ProviderError(format!(
                "Pinecone query error: {}",
                code
            ))),
        }
    }
}

// ---------------------------- DTOs ----------------------------

#[derive(Deserialize)]
struct QueryResponse {
    matches: Vec<QueryMatch>,
}

#[derive(Deserialize)]
pub struct QueryMatch {
    pub id: String,
    pub score: f32,
    #[serde(default)]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub values: Option<Vec<f32>>,
}

fn to_vector_error(e: impl std::fmt::Display) -> VectorError {
    VectorError::ProviderError(e.to_string())
}

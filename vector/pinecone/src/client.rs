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
<<<<<<< HEAD
<<<<<<< HEAD
use std::time::Duration;
=======
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da

use golem_vector::exports::golem::vector::types::{
    FilterExpression, VectorData, VectorError, VectorRecord,
};

use crate::conversion::{metadata_to_json_map, metric_to_pinecone, vector_data_to_dense};

#[derive(Clone)]
pub struct PineconeApi {
    http: Client,
<<<<<<< HEAD
    base_url: String,
    api_key: Option<String>,
=======
<<<<<<< HEAD
<<<<<<< HEAD
    controller_endpoint: String,
    index_host: Option<String>,
    api_key: String,
>>>>>>> 54db59b006712dd19266b3696202a3a95d62010a
    timeout: Duration,
    max_retries: u32,
}

impl PineconeApi {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        Self::new_with_config(base_url, api_key, Duration::from_secs(30), 3)
    }

    pub fn new_with_config(
        base_url: String,
        api_key: Option<String>,
        timeout: Duration,
        max_retries: u32,
    ) -> Self {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http: client,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            timeout,
            max_retries,
        }
    }

    pub fn health_check(&self) -> Result<bool, VectorError> {
        let url = format!("{}/describe_index_stats", self.base_url);
        let mut req = self.http.get(url);
        if let Some(key) = &self.api_key {
            req = req.header("Api-Key", key);
        }
        match req.send() {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
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

    fn auth_headers(&self, headers: &mut reqwest::header::HeaderMap) {
<<<<<<< HEAD
        if let Some(key) = &self.api_key {
            headers.insert("Api-Key", key.parse().unwrap());
        }
=======
        headers.insert("Api-Key", self.api_key.parse().unwrap());
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
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
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
>>>>>>> 54db59b006712dd19266b3696202a3a95d62010a
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

        let url = format!("{}/vectors/upsert", self.base_url);
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

<<<<<<< HEAD
<<<<<<< HEAD
    pub fn fetch_vectors(
        &self,
        ids: Vec<String>,
        namespace: Option<String>,
    ) -> Result<Vec<FetchVector>, VectorError> {
        #[derive(Deserialize)]
        struct FetchResponse {
            vectors: std::collections::HashMap<String, FetchVector>,
        }
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut url = format!("{}/vectors/fetch?", self.base_url);
        for (i, id) in ids.iter().enumerate() {
            if i > 0 {
                url.push('&');
            }
            url.push_str(&format!("ids[]={}", urlencoding::encode(id)));
        }
        if let Some(ns) = &namespace {
            url.push_str(&format!("&namespace={}", urlencoding::encode(ns)));
        }
        let mut req = self.http.get(url);
        if let Some(key) = &self.api_key {
            req = req.header("Api-Key", key);
        }
        let resp = req.send().map_err(to_vector_error)?;
        match resp.status() {
            StatusCode::OK => {
                let body: FetchResponse = resp.json().map_err(to_vector_error)?;
                Ok(body.vectors.into_iter().map(|(_, v)| v).collect())
            }
            StatusCode::NOT_FOUND => Ok(Vec::new()),
            code => Err(VectorError::ProviderError(format!(
                "Pinecone fetch error: {}",
                code
            ))),
        }
    }

=======
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
    pub fn query(
        &self,
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
            #[serde(skip_serializing_if = "crate::client::is_false")]
            include_values: bool,
            #[serde(skip_serializing_if = "crate::client::is_false")]
            include_metadata: bool,
        }

        let url = format!("{}/query", self.base_url);
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
<<<<<<< HEAD
<<<<<<< HEAD
pub struct FetchVector {
    pub id: String,
    #[serde(default)]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub values: Option<Vec<f32>>,
}

#[derive(Deserialize)]
=======
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
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

// Helper for serde skip_serializing_if to omit false booleans
pub(crate) fn is_false(b: &bool) -> bool { !*b }

//! Qdrant REST API client

use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
<<<<<<< HEAD
use std::time::Duration;
=======
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49

use golem_vector::exports::golem::vector::types::{
    FilterExpression, Metadata, VectorData, VectorError, VectorRecord,
};

#[derive(Clone)]
pub struct QdrantApi {
    http: Client,
    base_url: String,
    api_key: Option<String>,
<<<<<<< HEAD
    timeout: Duration,
    max_retries: u32,
=======
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
}

impl QdrantApi {
    pub fn new(base_url: impl Into<String>, api_key: Option<String>) -> Self {
<<<<<<< HEAD
        Self::new_with_config(base_url, api_key, Duration::from_secs(30), 3)
    }

    pub fn new_with_config(
        base_url: impl Into<String>, 
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
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
            timeout,
            max_retries,
        }
    }

    pub fn health_check(&self) -> Result<bool, VectorError> {
        let url = format!("{}/healthz", self.base_url);
        match self.http.get(url).send() {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
=======
        Self {
            http: Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
        }
    }

    fn auth_headers(&self, headers: &mut reqwest::header::HeaderMap) {
        if let Some(key) = &self.api_key {
            headers.insert("api-key", key.parse().unwrap());
        }
    }

<<<<<<< HEAD
    // Helper method for retry logic
    fn with_retry<F, T>(&self, operation: F) -> Result<T, VectorError>
    where
        F: Fn() -> Result<T, VectorError>,
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

=======
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
    // ------------------------- collections -----------------------------
    pub fn upsert_collection(
        &self,
        name: &str,
        dimension: u32,
        distance: &str,
    ) -> Result<CollectionDescription, VectorError> {
        #[derive(Serialize)]
        struct Payload<'a> {
            vectors: VectorParams<'a>,
        }
        #[derive(Serialize)]
        struct VectorParams<'a> {
            size: u32,
            distance: &'a str,
        }
        let url = format!("{}/collections/{}", self.base_url, name);
        let mut req = self.http.put(url);
        req = req.json(&Payload {
            vectors: VectorParams {
                size: dimension,
                distance,
            },
        });
        if let Some(key) = &self.api_key {
            req = req.header("api-key", key);
        }
        let resp = req.send().map_err(to_vector_error)?;
        match resp.status() {
            StatusCode::OK | StatusCode::CREATED => {
                let body: QdrantResponse<CollectionDescription> =
                    resp.json().map_err(to_vector_error)?;
                Ok(body.result)
            }
            code => Err(VectorError::ProviderError(format!(
                "Qdrant error: {}",
                code
            ))),
        }
    }

    pub fn list_collections(&self) -> Result<Vec<CollectionDescription>, VectorError> {
        let url = format!("{}/collections", self.base_url);
        let mut req = self.http.get(url);
        if let Some(key) = &self.api_key {
            req = req.header("api-key", key);
        }
        let resp = req.send().map_err(to_vector_error)?;
        let body: QdrantResponse<CollectionsList> = resp.json().map_err(to_vector_error)?;
        Ok(body.result.collections)
    }

    pub fn delete_collection(&self, name: &str) -> Result<(), VectorError> {
        let url = format!("{}/collections/{}", self.base_url, name);
        let mut req = self.http.delete(url);
        if let Some(key) = &self.api_key {
            req = req.header("api-key", key);
        }
        let resp = req.send().map_err(to_vector_error)?;
        match resp.status() {
            StatusCode::OK => Ok(()),
            code => Err(VectorError::ProviderError(format!(
                "Qdrant error: {}",
                code
            ))),
        }
    }

    // ------------------------- points ----------------------------------
    pub fn upsert_points(
        &self,
        collection: &str,
        points: Vec<QdrantPoint>,
        namespace: Option<String>,
    ) -> Result<(), VectorError> {
        #[derive(Serialize)]
        struct Payload {
            points: Vec<QdrantPoint>,
            #[serde(skip_serializing_if = "Option::is_none")]
            wait: Option<bool>,
        }
        let mut url = format!("{}/collections/{}/points", self.base_url, collection);
        if let Some(ns) = &namespace {
            url.push_str(&format!("?namespace={}", ns));
        }
        let mut req = self.http.put(url);
        if let Some(key) = &self.api_key {
            req = req.header("api-key", key);
        }
        let resp = req
            .json(&Payload {
                points,
                wait: Some(true),
            })
            .send()
            .map_err(to_vector_error)?;
        match resp.status() {
            StatusCode::OK | StatusCode::ACCEPTED => Ok(()),
            code => Err(VectorError::ProviderError(format!(
                "Qdrant error: {}",
                code
            ))),
        }
    }

    pub fn search(
        &self,
        collection: &str,
        vector: Vec<f32>,
        limit: u32,
        namespace: Option<String>,
        filter: Option<QdrantFilter>,
        with_vector: bool,
        with_payload: bool,
    ) -> Result<Vec<SearchResultOut>, VectorError> {
        #[derive(Serialize)]
        struct Payload<'a> {
            vector: &'a [f32],
            limit: u32,
            #[serde(skip_serializing_if = "Option::is_none")]
            filter: Option<&'a QdrantFilter>,
            #[serde(rename = "with_vectors", skip_serializing_if = "Option::is_none")]
            with_vectors: Option<bool>,
            #[serde(rename = "with_payload", skip_serializing_if = "Option::is_none")]
            with_payload: Option<bool>,
        }
        let mut url = format!("{}/collections/{}/points/search", self.base_url, collection);
        if let Some(ns) = &namespace {
            url.push_str(&format!("?namespace={}", ns));
        }
        let mut req = self.http.post(url);
        if let Some(key) = &self.api_key {
            req = req.header("api-key", key);
        }
        let payload = Payload {
            vector: &vector,
            limit,
            filter: filter.as_ref(),
            with_vectors: Some(with_vector),
            with_payload: Some(with_payload),
        };
        let resp = req.json(&payload).send().map_err(to_vector_error)?;
        match resp.status() {
            StatusCode::OK => {
                let body: QdrantResponse<Vec<SearchResultOut>> =
                    resp.json().map_err(to_vector_error)?;
                Ok(body.result)
            }
            code => Err(VectorError::ProviderError(format!(
                "Qdrant search error: {}",
                code
            ))),
        }
    }
}

// ----------------------------- DTOs ------------------------------
<<<<<<< HEAD
    /// Retrieve a single point by ID and namespace.
    pub fn get_point(
        &self,
        collection: &str,
        id: &str,
        namespace: Option<String>,
    ) -> Result<Option<PointOut>, VectorError> {
        let mut url = format!("{}/collections/{}/points/{}", self.base_url, collection, id);
        if let Some(ns) = &namespace {
            url.push_str(&format!("?namespace={}", ns));
        }
        let mut req = self.http.get(url);
        if let Some(key) = &self.api_key {
            req = req.header("api-key", key);
        }
        let resp = req.send().map_err(to_vector_error)?;
        match resp.status() {
            StatusCode::OK => {
                let body: QdrantResponse<PointOut> = resp.json().map_err(to_vector_error)?;
                Ok(Some(body.result))
            }
            StatusCode::NOT_FOUND => Ok(None),
            code => Err(VectorError::ProviderError(format!(
                "Qdrant get_point error: {}",
                code
            ))),
        }
    }

// ----------------------------- DTOs ------------------------------
=======
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
#[derive(Deserialize)]
struct QdrantResponse<T> {
    result: T,
    status: String,
    time: f64,
}

#[derive(Deserialize)]
<<<<<<< HEAD
pub struct PointOut {
    pub id: String,
    #[serde(default)]
    pub vector: Option<Vec<f32>>,
    #[serde(default)]
    pub payload: Option<HashMap<String, serde_json::Value>>, 
}


#[derive(Deserialize)]
=======
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
struct CollectionsList {
    collections: Vec<CollectionDescription>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct CollectionDescription {
    pub name: String,
    #[serde(default)]
    pub vectors_count: u64,
    #[serde(default)]
    pub indexed_vectors_count: u64,
    #[serde(default)]
    pub points_count: u64,
}

#[derive(Serialize)]
pub struct QdrantPoint {
    pub id: String,
    pub vector: Vec<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Serialize, Default)]
pub struct QdrantFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub must: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub should: Option<Vec<serde_json::Value>>,
    #[serde(rename = "must_not", skip_serializing_if = "Option::is_none")]
    pub must_not: Option<Vec<serde_json::Value>>,
}

#[derive(Deserialize)]
pub struct SearchResultOut {
    pub id: String,
    pub score: f32,
    #[serde(default)]
    pub payload: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub vector: Option<Vec<f32>>,
}

fn to_vector_error(e: impl std::fmt::Display) -> VectorError {
    VectorError::ProviderError(e.to_string())
}
<<<<<<< HEAD

fn to_vector_error_with_context(e: impl std::fmt::Display, operation: &str) -> VectorError {
    VectorError::ProviderError(format!("Qdrant {} error: {}", operation, e))
}
=======
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49

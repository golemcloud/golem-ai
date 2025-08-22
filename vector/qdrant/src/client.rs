//! Qdrant REST API client

use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use golem_vector::exports::golem::vector::types::{
    FilterExpression, Metadata, VectorData, VectorError, VectorRecord,
};

#[derive(Clone)]
pub struct QdrantApi {
    http: Client,
    base_url: String,
    api_key: Option<String>,
    timeout: Duration,
    max_retries: u32,
}

impl QdrantApi {
    pub fn new(base_url: impl Into<String>, api_key: Option<String>) -> Self {
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
        }
    }

    fn auth_headers(&self, headers: &mut reqwest::header::HeaderMap) {
        if let Some(key) = &self.api_key {
            headers.insert("api-key", key.parse().unwrap());
        }
    }

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
        let url = format!("{}/collections/{}/points", self.base_url, collection);
        // Emulate namespaces by writing a payload field `namespace`
        let points = if let Some(ns) = &namespace {
            let mut out = Vec::with_capacity(points.len());
            for mut p in points {
                let mut payload = p.payload.unwrap_or_default();
                payload.insert("namespace".to_string(), serde_json::Value::String(ns.clone()));
                p.payload = Some(payload);
                out.push(p);
            }
            out
        } else {
            points
        };

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
            #[serde(rename = "with_vector", skip_serializing_if = "Option::is_none")]
            with_vector: Option<bool>,
            #[serde(rename = "with_payload", skip_serializing_if = "Option::is_none")]
            with_payload: Option<bool>,
        }
        let url = format!("{}/collections/{}/points/search", self.base_url, collection);
        let mut req = self.http.post(url);
        if let Some(key) = &self.api_key {
            req = req.header("api-key", key);
        }
        // Merge namespace constraint into filter if provided
        let mut filter_owned = filter;
        if let Some(ns) = &namespace {
            let ns_cond = serde_json::json!({ "key": "namespace", "match": { "value": ns } });
            match &mut filter_owned {
                Some(f) => {
                    if let Some(must) = &mut f.must {
                        must.push(ns_cond);
                    } else {
                        f.must = Some(vec![ns_cond]);
                    }
                }
                None => {
                    filter_owned = Some(QdrantFilter {
                        must: Some(vec![ns_cond]),
                        should: None,
                        must_not: None,
                    });
                }
            }
        }

        let payload = Payload {
            vector: &vector,
            limit,
            filter: filter_owned.as_ref(),
            with_vector: Some(with_vector),
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
    
    /// Retrieve a single point by ID and namespace (namespace emulated via payload).
    pub fn get_point(
        &self,
        collection: &str,
        id: &str,
        namespace: Option<String>,
    ) -> Result<Option<PointOut>, VectorError> {
        let url = format!("{}/collections/{}/points/{}", self.base_url, collection, id);
        let mut req = self.http.get(url);
        if let Some(key) = &self.api_key {
            req = req.header("api-key", key);
        }
        let resp = req.send().map_err(to_vector_error)?;
        match resp.status() {
            StatusCode::OK => {
                let body: QdrantResponse<PointOut> = resp.json().map_err(to_vector_error)?;
                // If namespace provided, enforce it client-side by checking payload
                if let Some(ns) = namespace {
                    if let Some(payload) = &body.result.payload {
                        if let Some(val) = payload.get("namespace") {
                            if val == &serde_json::Value::String(ns) {
                                return Ok(Some(body.result));
                            } else {
                                return Ok(None);
                            }
                        }
                    }
                    return Ok(None);
                }
                Ok(Some(body.result))
            }
            StatusCode::NOT_FOUND => Ok(None),
            code => Err(VectorError::ProviderError(format!(
                "Qdrant get_point error: {}",
                code
            ))),
        }
    }

    /// Delete points by IDs.
    pub fn delete_points(
        &self,
        collection: &str,
        ids: Vec<String>,
        _namespace: Option<String>,
    ) -> Result<(), VectorError> {
        #[derive(Serialize)]
        struct Payload<'a> {
            points: &'a [String],
            #[serde(skip_serializing_if = "Option::is_none")]
            wait: Option<bool>,
        }
        let url = format!("{}/collections/{}/points/delete", self.base_url, collection);
        let mut req = self.http.post(url);
        if let Some(key) = &self.api_key {
            req = req.header("api-key", key);
        }
        let resp = req
            .json(&Payload { points: &ids, wait: Some(true) })
            .send()
            .map_err(to_vector_error)?;
        match resp.status() {
            StatusCode::OK | StatusCode::ACCEPTED => Ok(()),
            code => Err(VectorError::ProviderError(format!(
                "Qdrant delete_points error: {}",
                code
            ))),
        }
    }

    /// Delete points matching a filter; returns the number of points deleted (best-effort based on pre-count).
    pub fn delete_points_by_filter(
        &self,
        collection: &str,
        filter: Option<QdrantFilter>,
        namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        // Pre-count (best-effort)
        let count = self.count_points(collection, filter.clone(), namespace.clone())? as u32;

        #[derive(Serialize)]
        struct Payload<'a> {
            filter: &'a QdrantFilter,
            #[serde(skip_serializing_if = "Option::is_none")]
            wait: Option<bool>,
        }

        // Merge namespace into filter
        let mut filter_owned = filter.unwrap_or_default();
        if let Some(ns) = &namespace {
            let ns_cond = serde_json::json!({ "key": "namespace", "match": { "value": ns } });
            if let Some(must) = &mut filter_owned.must {
                must.push(ns_cond);
            } else {
                filter_owned.must = Some(vec![ns_cond]);
            }
        }

        let url = format!("{}/collections/{}/points/delete", self.base_url, collection);
        let mut req = self.http.post(url);
        if let Some(key) = &self.api_key {
            req = req.header("api-key", key);
        }
        let resp = req
            .json(&Payload { filter: &filter_owned, wait: Some(true) })
            .send()
            .map_err(to_vector_error)?;
        match resp.status() {
            StatusCode::OK | StatusCode::ACCEPTED => Ok(count),
            code => Err(VectorError::ProviderError(format!(
                "Qdrant delete_by_filter error: {}",
                code
            ))),
        }
    }

    /// Count points matching a filter (exact count).
    pub fn count_points(
        &self,
        collection: &str,
        filter: Option<QdrantFilter>,
        namespace: Option<String>,
    ) -> Result<u64, VectorError> {
        #[derive(Serialize)]
        struct Payload<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            filter: Option<&'a QdrantFilter>,
            exact: bool,
        }
        #[derive(Deserialize)]
        struct CountResult { count: u64 }

        let url = format!("{}/collections/{}/points/count", self.base_url, collection);
        let mut req = self.http.post(url);
        if let Some(key) = &self.api_key {
            req = req.header("api-key", key);
        }

        // Merge namespace into filter if provided
        let mut filter_owned = filter;
        if let Some(ns) = &namespace {
            let ns_cond = serde_json::json!({ "key": "namespace", "match": { "value": ns } });
            match &mut filter_owned {
                Some(f) => {
                    if let Some(must) = &mut f.must {
                        must.push(ns_cond);
                    } else {
                        f.must = Some(vec![ns_cond]);
                    }
                }
                None => {
                    filter_owned = Some(QdrantFilter { must: Some(vec![ns_cond]), should: None, must_not: None });
                }
            }
        }

        let resp = req
            .json(&Payload { filter: filter_owned.as_ref(), exact: true })
            .send()
            .map_err(to_vector_error)?;
        match resp.status() {
            StatusCode::OK => {
                let body: QdrantResponse<CountResult> = resp.json().map_err(to_vector_error)?;
                Ok(body.result.count)
            }
            code => Err(VectorError::ProviderError(format!(
                "Qdrant count_points error: {}",
                code
            ))),
        }
    }
}

// ----------------------------- DTOs ------------------------------
#[derive(Deserialize)]
struct QdrantResponse<T> {
    result: T,
    status: String,
    time: f64,
}

#[derive(Deserialize)]
pub struct PointOut {
    pub id: String,
    #[serde(default)]
    pub vector: Option<Vec<f32>>,
    #[serde(default)]
    pub payload: Option<HashMap<String, serde_json::Value>>, 
}


#[derive(Deserialize)]
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

#[derive(Serialize, Default, Clone)]
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

fn to_vector_error_with_context(e: impl std::fmt::Display, operation: &str) -> VectorError {
    VectorError::ProviderError(format!("Qdrant {} error: {}", operation, e))
}

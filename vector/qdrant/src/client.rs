//! For native builds we use `reqwest` HTTP client. WASM builds return
//! `unsupported_feature` as outbound network is disallowed.

use crate::conversion::{
    json_to_metadata_value, metadata_to_json_map, metric_to_qdrant,
    vector_data_to_dense,
};
use golem_vector::exports::golem::vector::types::{
    DistanceMetric, Metadata, VectorData, VectorError, VectorRecord,
};
use serde::{Deserialize, Serialize};

#[cfg(target_family = "wasm")]
pub struct QdrantClient;

#[cfg(target_family = "wasm")]
impl QdrantClient {
    pub fn new(_base_url: String, _api_key: Option<String>) -> Self {
        QdrantClient
    }
    fn err<T>() -> Result<T, VectorError> {
        Err(golem_vector::error::unsupported_feature(
            "Qdrant provider unavailable in wasm runtime",
        ))
    }
    pub fn create_collection(
        &self,
        _name: &str,
        _dim: u32,
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
        _collection: &str,
        _records: Vec<VectorRecord>,
    ) -> Result<(), VectorError> {
        Self::err()
    }
    pub fn query_vectors(
        &self,
        _collection: &str,
        _query: Vec<f32>,
        _limit: u32,
        _filter: Option<serde_json::Value>,
        _with_vectors: bool,
        _with_payload: bool,
    ) -> Result<Vec<(String, f32, Option<Vec<f32>>, Option<Metadata>)>, VectorError> {
        Self::err()
    }
    pub fn fetch_vectors(
        &self,
        _collection: &str,
        _ids: Vec<String>,
        _with_vectors: bool,
        _with_payload: bool,
    ) -> Result<Vec<VectorRecord>, VectorError> {
        Self::err()
    }
    pub fn delete_vectors(&self, _collection: &str, _ids: Vec<String>) -> Result<u32, VectorError> {
        Self::err()
    }
}

// ---------------------------- native impl ------------------------------
#[cfg(not(target_family = "wasm"))]
mod native {
    use super::*;
    use reqwest::{Client, Response};
    use serde_json::Map;
    use serde_json::{json, Value};
    use std::time::Duration;

    #[derive(Clone)]
    pub struct QdrantClient {
        http: Client,
        base_url: String,
        api_key: Option<String>,
    }

    impl QdrantClient {
        pub fn new(base_url: String, api_key: Option<String>) -> Self {
            let client = Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client");
            Self {
                http: client,
                base_url: base_url.trim_end_matches('/').to_string(),
                api_key,
            }
        }

        fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
            if let Some(key) = &self.api_key {
                req.header("api-key", key)
            } else {
                req
            }
        }

        fn handle<T: serde::de::DeserializeOwned>(
            &self,
            resp: Response,
            op: &str,
        ) -> Result<T, VectorError> {
            let status = resp.status();
            if status.is_success() {
                resp.json()
                    .map_err(|e| VectorError::ProviderError(format!("Qdrant {op} JSON error: {e}")))
            } else {
                let body = resp.text().unwrap_or_default();
                Err(golem_vector::error::provider_error(format!(
                    "Qdrant {op} HTTP {status}: {body}"
                )))
            }
        }

        // -------------------- collections ---------------------------
        pub fn create_collection(
            &self,
            name: &str,
            dim: u32,
            metric: DistanceMetric,
        ) -> Result<(), VectorError> {
            #[derive(Serialize)]
            struct VectorParams<'a> {
                size: u32,
                distance: &'a str,
            }
            #[derive(Serialize)]
            struct Body<'a> {
                vectors: VectorParams<'a>,
            }
            let body = Body {
                vectors: VectorParams {
                    size: dim,
                    distance: metric_to_qdrant(metric),
                },
            };
            let url = format!("{}/collections/{}", self.base_url, name);
            let resp = self
                .auth(self.http.put(url).json(&body))
                .send()
                .map_err(|e| golem_vector::error::from_reqwest_error("create_collection", e))?;
            let _: Value = self.handle(resp, "create_collection")?;
            Ok(())
        }

        pub fn list_collections(&self) -> Result<Vec<String>, VectorError> {
            #[derive(Deserialize)]
            struct Col {
                name: String,
            }
            #[derive(Deserialize)]
            struct Resp {
                result: Vec<Col>,
            }
            let url = format!("{}/collections", self.base_url);
            let r: Resp = self.handle(
                self.http
                    .get(url)
                    .send()
                    .map_err(|e| golem_vector::error::from_reqwest_error("list_collections", e))?,
                "list_collections",
            )?;
            Ok(r.result.into_iter().map(|c| c.name).collect())
        }

        pub fn delete_collection(&self, name: &str) -> Result<(), VectorError> {
            let url = format!("{}/collections/{}", self.base_url, name);
            let resp = self
                .http
                .delete(url)
                .send()
                .map_err(|e| golem_vector::error::from_reqwest_error("delete_collection", e))?;
            let _: Value = self.handle(resp, "delete_collection")?;
            Ok(())
        }

        // -------------------- vectors ------------------------------
        pub fn upsert_vectors(
            &self,
            collection: &str,
            records: Vec<VectorRecord>,
        ) -> Result<(), VectorError> {
            #[derive(Serialize)]
            struct Point {
                id: String,
                vector: Vec<f32>,
                #[serde(skip_serializing_if = "Option::is_none")]
                payload: Option<Map<String, Value>>,
            }
            #[derive(Serialize)]
            struct Body {
                points: Vec<Point>,
            }
            let mut points = Vec::with_capacity(records.len());
            for rec in &records {
                let vec = vector_data_to_dense(rec.vector.clone())?;
                let payload = metadata_to_json_map(rec.metadata.clone());
                points.push(Point {
                    id: rec.id.clone(),
                    vector: vec,
                    payload,
                });
            }
            let body = json!({"points": points, "wait": true});
            let url = format!("{}/collections/{}/points", self.base_url, collection);
            let resp = self
                .auth(self.http.put(url).json(&body))
                .send()
                .map_err(|e| golem_vector::error::from_reqwest_error("upsert_vectors", e))?;
            let _: Value = self.handle(resp, "upsert_vectors")?;
            Ok(())
        }

        pub fn query_vectors(
            &self,
            collection: &str,
            query: Vec<f32>,
            limit: u32,
            filter: Option<Value>,
            with_vectors: bool,
            with_payload: bool,
        ) -> Result<Vec<(String, f32, Option<Vec<f32>>, Option<Metadata>)>, VectorError> {
            #[derive(Serialize)]
            struct Body {
                vector: Vec<f32>,
                limit: u32,
                #[serde(skip_serializing_if = "Option::is_none")]
                filter: Option<Value>,
                with_vector: bool,
                with_payload: bool,
            }
            let body = Body {
                vector: query,
                limit,
                filter,
                with_vector: with_vectors,
                with_payload,
            };
            let url = format!("{}/collections/{}/points/search", self.base_url, collection);
            let resp = self
                .auth(self.http.post(url).json(&body))
                .send()
                .map_err(|e| golem_vector::error::from_reqwest_error("search", e))?;
            #[derive(Deserialize)]
            struct Pt {
                id: serde_json::Value,
                score: f32,
                vector: Option<Vec<f32>>,
                payload: Option<Map<String, Value>>,
            }
            #[derive(Deserialize)]
            struct Resp {
                result: Vec<Pt>,
            }
            let r: Resp = self.handle(resp, "search")?;
            Ok(r.result
                .into_iter()
                .map(|p| {
                    let id_str =
                        p.id.as_str()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| p.id.to_string());
                    let meta = p.payload.map(|map| {
                        map.into_iter()
                            .map(|(k, v)| (k, json_to_metadata_value(&v)))
                            .collect()
                    });
                    (id_str, p.score, p.vector, meta)
                })
                .collect())
        }

        pub fn fetch_vectors(
            &self,
            collection: &str,
            ids: Vec<String>,
            with_vectors: bool,
            with_payload: bool,
        ) -> Result<Vec<VectorRecord>, VectorError> {
            if ids.is_empty() {
                return Ok(Vec::new());
            }
            let url = format!("{}/collections/{}/points", self.base_url, collection);
            let body =
                json!({"ids": ids, "with_vector": with_vectors, "with_payload": with_payload});
            let resp = self
                .auth(self.http.post(url).json(&body))
                .send()
                .map_err(|e| golem_vector::error::from_reqwest_error("get_points", e))?;
            #[derive(Deserialize)]
            struct Pt {
                id: serde_json::Value,
                vector: Option<Vec<f32>>,
                payload: Option<Map<String, Value>>,
            }
            #[derive(Deserialize)]
            struct Resp {
                result: Vec<Pt>,
            }
            let r: Resp = self.handle(resp, "get_points")?;
            Ok(r.result
                .into_iter()
                .map(|p| {
                    let id_str =
                        p.id.as_str()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| p.id.to_string());
                    VectorRecord {
                        id: id_str,
                        vector: VectorData::Dense(p.vector.unwrap_or_default()),
                        metadata: p.payload.map(|map| {
                            map.into_iter()
                                .map(|(k, v)| (k, json_to_metadata_value(&v)))
                                .collect()
                        }),
                    }
                })
                .collect())
        }

        pub fn delete_vectors(
            &self,
            collection: &str,
            ids: Vec<String>,
        ) -> Result<u32, VectorError> {
            if ids.is_empty() {
                return Ok(0);
            }
            let url = format!("{}/collections/{}/points/delete", self.base_url, collection);
            let body = json!({"points": ids, "wait": true});
            let resp = self
                .auth(self.http.post(url).json(&body))
                .send()
                .map_err(|e| golem_vector::error::from_reqwest_error("delete_vectors", e))?;
            let _: Value = self.handle(resp, "delete_vectors")?;
            Ok(ids.len() as u32)
        }

        /// Scroll (list) vectors with optional filter & pagination.
        /// Returns (records, next_cursor)
        pub fn scroll_vectors(
            &self,
            collection: &str,
            filter: Option<Value>,
            limit: u32,
            cursor: Option<String>,
            with_vectors: bool,
            with_payload: bool,
        ) -> Result<(Vec<VectorRecord>, Option<String>), VectorError> {
            #[derive(Serialize)]
            struct Body<'a> {
                limit: u32,
                #[serde(skip_serializing_if = "Option::is_none")]
                filter: Option<&'a Value>,
                #[serde(skip_serializing_if = "Option::is_none")]
                offset: Option<serde_json::Value>,
                with_vector: bool,
                with_payload: bool,
            }
            let offset_val = cursor.map(|id| json!({"point_id": id}));
            let body = Body {
                limit,
                filter: filter.as_ref(),
                offset: offset_val,
                with_vector: with_vectors,
                with_payload,
            };
            let url = format!("{}/collections/{}/points/scroll", self.base_url, collection);
            let resp = self
                .auth(self.http.post(url).json(&body))
                .send()
                .map_err(|e| golem_vector::error::from_reqwest_error("scroll_vectors", e))?;
            #[derive(Deserialize)]
            struct Pt {
                id: serde_json::Value,
                vector: Option<Vec<f32>>,
                payload: Option<Map<String, Value>>,
            }
            #[derive(Deserialize)]
            struct Resp {
                result: ScrollResult,
            }
            #[derive(Deserialize)]
            struct ScrollResult {
                points: Vec<Pt>,
                #[serde(rename = "next_page_offset")]
                next: Option<serde_json::Value>,
            }
            let r: Resp = self.handle(resp, "scroll_vectors")?;
            let records = r
                .result
                .points
                .into_iter()
                .map(|p| {
                    let id_str = p
                        .id
                        .as_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| p.id.to_string());
                    VectorRecord {
                        id: id_str,
                        vector: VectorData::Dense(p.vector.unwrap_or_default()),
                        metadata: p.payload.map(|map| {
                            map.into_iter()
                                .map(|(k, v)| (k, json_to_metadata_value(&v)))
                                .collect()
                        }),
                    }
                })
                .collect();
            let next_cursor = r
                .result
                .next
                .and_then(|val| val.get("point_id").cloned())
                .and_then(|v| v.as_str().map(|s| s.to_string()));
            Ok((records, next_cursor))
        }

        /// Count vectors matching optional filter using /points/count
        pub fn count_vectors(
            &self,
            collection: &str,
            filter: Option<Value>,
        ) -> Result<u64, VectorError> {
            let url = format!("{}/collections/{}/points/count", self.base_url, collection);
            let body = json!({"filter": filter, "exact": true});
            #[derive(Deserialize)]
            struct Resp {
                result: CountResult,
            }
            #[derive(Deserialize)]
            struct CountResult {
                count: u64,
            }
            let resp = self
                .auth(self.http.post(url).json(&body))
                .send()
                .map_err(|e| golem_vector::error::from_reqwest_error("count_vectors", e))?;
            let r: Resp = self.handle(resp, "count_vectors")?;
            Ok(r.result.count)
        }
    }
}

#[cfg(not(target_family = "wasm"))]
pub use native::QdrantClient;

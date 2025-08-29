//! Minimal Pinecone REST client (Data Plane).
//!
//! The native build uses reqwest to interact with Pinecone. In wasm builds, we
//! return an `unsupported_feature` error because outbound HTTP is disallowed in
//! the sandboxed runtime.

use crate::conversion::{
    metadata_to_json_map, vector_data_to_dense,
};
use golem_vector::exports::golem::vector::types::{
    Metadata, VectorData, VectorError, VectorRecord,
};
use serde::{Deserialize, Serialize};

#[cfg(target_family = "wasm")]
pub struct PineconeClient;

#[cfg(target_family = "wasm")]
impl PineconeClient {
    pub fn new(_base_url: String, _api_key: String) -> Self {
        PineconeClient
    }
    fn err<T>() -> Result<T, VectorError> {
        Err(golem_vector::error::unsupported_feature(
            "Pinecone provider unavailable in WASM runtime",
        ))
    }
    pub fn upsert_vectors(
        &self,
        _namespace: &str,
        _records: Vec<VectorRecord>,
    ) -> Result<(), VectorError> {
        Self::err()
    }
    pub fn query_vectors(
        &self,
        _namespace: &str,
        _query: Vec<f32>,
        _limit: u32,
        _filter: Option<serde_json::Value>,
        _include_vectors: bool,
        _include_metadata: bool,
    ) -> Result<Vec<(String, f32, Option<Vec<f32>>, Option<Metadata>)>, VectorError> {
        Self::err()
    }
    pub fn fetch_vectors(
        &self,
        _namespace: &str,
        _ids: Vec<String>,
        _include_vectors: bool,
        _include_metadata: bool,
    ) -> Result<Vec<VectorRecord>, VectorError> {
        Self::err()
    }
    pub fn delete_vectors(&self, _namespace: &str, _ids: Vec<String>) -> Result<u32, VectorError> {
        Self::err()
    }
}

// ---------------------------- native impl ------------------------------
#[cfg(not(target_family = "wasm"))]
mod native {
    use super::*;
    use reqwest::{Client, Response};
    use serde_json::Value;
    use std::collections::HashMap;
    use std::time::Duration;

    #[derive(Clone)]
    pub struct PineconeClient {
        http: Client,
        base_url: String,
        api_key: String,
        timeout: Duration,
    }

    impl PineconeClient {
        pub fn new(base_url: String, api_key: String) -> Self {
            let client = Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client");
            Self {
                http: client,
                base_url: base_url.trim_end_matches('/').to_string(),
                api_key,
                timeout: Duration::from_secs(30),
            }
        }

        fn auth_header(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
            req.header("Api-Key", &self.api_key)
                .header("Accept", "application/json")
        }

        fn handle<T: serde::de::DeserializeOwned>(
            &self,
            resp: Response,
            op: &str,
        ) -> Result<T, VectorError> {
            let status = resp.status();
            if status.is_success() {
                resp.json().map_err(|e| {
                    VectorError::ProviderError(format!("Pinecone {op} JSON error: {e}"))
                })
            } else {
                let body = resp.text().unwrap_or_default();
                Err(golem_vector::error::provider_error(format!(
                    "Pinecone {op} HTTP {status}: {body}"
                )))
            }
        }

        // -------------------- vectors ------------------------------
        pub fn upsert_vectors(
            &self,
            namespace: &str,
            records: Vec<VectorRecord>,
        ) -> Result<(), VectorError> {
            if records.is_empty() {
                return Ok(());
            }
            #[derive(Serialize)]
            struct VecPayload {
                id: String,
                values: Vec<f32>,
                #[serde(skip_serializing_if = "Option::is_none")]
                metadata: Option<serde_json::Map<String, Value>>,
            }
            #[derive(Serialize)]
            struct Body {
                vectors: Vec<VecPayload>,
                namespace: String,
            }

            let mut vecs = Vec::with_capacity(records.len());
            for rec in &records {
                let values = vector_data_to_dense(rec.vector.clone())?;
                let meta = metadata_to_json_map(rec.metadata.clone());
                vecs.push(VecPayload {
                    id: rec.id.clone(),
                    values,
                    metadata: meta,
                });
            }
            let body = Body {
                vectors: vecs,
                namespace: namespace.to_string(),
            };
            let url = format!("{}/vectors/upsert", self.base_url);
            let resp = self
                .auth_header(self.http.post(url).json(&body))
                .send()
                .map_err(|e| golem_vector::error::from_reqwest_error("upsert", e))?;
            let _: serde_json::Value = self.handle(resp, "upsert_vectors")?;
            Ok(())
        }

        pub fn query_vectors(
            &self,
            namespace: &str,
            query: Vec<f32>,
            limit: u32,
            filter: Option<Value>,
            include_vectors: bool,
            include_metadata: bool,
        ) -> Result<Vec<(String, f32, Option<Vec<f32>>, Option<Metadata>)>, VectorError> {
            #[derive(Serialize)]
            struct Body {
                #[serde(rename = "vector")]
                query: Vec<f32>,
                topK: u32,
                #[serde(skip_serializing_if = "Option::is_none")]
                filter: Option<Value>,
                namespace: String,
                #[serde(rename = "includeValues", skip_serializing_if = "Option::is_none")]
                include_values: Option<bool>,
                #[serde(rename = "includeMetadata", skip_serializing_if = "Option::is_none")]
                include_metadata: Option<bool>,
            }
            let body = Body {
                query,
                topK: limit,
                filter,
                namespace: namespace.to_string(),
                include_values: Some(include_vectors),
                include_metadata: Some(include_metadata),
            };
            let url = format!("{}/query", self.base_url);
            let resp = self
                .auth_header(self.http.post(url).json(&body))
                .send()
                .map_err(|e| golem_vector::error::from_reqwest_error("query", e))?;
            #[derive(Deserialize)]
            struct Match {
                id: String,
                score: f32,
                #[serde(default)]
                values: Option<Vec<f32>>,
                #[serde(default)]
                metadata: Option<serde_json::Map<String, Value>>,
            }
            #[derive(Deserialize)]
            struct Resp {
                matches: Vec<Match>,
            }
            let r: Resp = self.handle(resp, "query_vectors")?;
            Ok(r.matches
                .into_iter()
                .map(|m| {
                    let meta = m.metadata.map(|map| {
                        map.into_iter()
                            .map(|(k, v)| (k, crate::conversion::json_to_metadata_value(&v)))
                            .collect()
                    });
                    (m.id, m.score, m.values, meta)
                })
                .collect())
        }

        pub fn fetch_vectors(
            &self,
            namespace: &str,
            ids: Vec<String>,
            include_vectors: bool,
            include_metadata: bool,
        ) -> Result<Vec<VectorRecord>, VectorError> {
            if ids.is_empty() {
                return Ok(Vec::new());
            }
            #[derive(Serialize)]
            struct Body {
                ids: Vec<String>,
                namespace: String,
                #[serde(rename = "includeValues")]
                include_values: bool,
                #[serde(rename = "includeMetadata")]
                include_metadata: bool,
            }
            let body = Body {
                ids: ids.clone(),
                namespace: namespace.to_string(),
                include_values: include_vectors,
                include_metadata,
            };
            let url = format!("{}/vectors/fetch", self.base_url);
            let resp = self
                .auth_header(self.http.post(url).json(&body))
                .send()
                .map_err(|e| golem_vector::error::from_reqwest_error("fetch", e))?;
            #[derive(Deserialize)]
            struct V {
                id: String,
                values: Option<Vec<f32>>,
                metadata: Option<serde_json::Map<String, Value>>,
            }
            #[derive(Deserialize)]
            struct Resp {
                vectors: HashMap<String, V>,
            }
            let r: Resp = self.handle(resp, "fetch_vectors")?;
            Ok(r.vectors.into_values().map(|v| VectorRecord {
                    id: v.id,
                    vector: VectorData::Dense(v.values.unwrap_or_default()),
                    metadata: v.metadata.map(|map| {
                        map.into_iter()
                            .map(|(k, v)| (k, crate::conversion::json_to_metadata_value(&v)))
                            .collect()
                    }),
                })
                .collect())
        }

        pub fn delete_vectors(
            &self,
            namespace: &str,
            ids: Vec<String>,
        ) -> Result<u32, VectorError> {
            if ids.is_empty() {
                return Ok(0);
            }
            #[derive(Serialize)]
            struct Body {
                ids: Vec<String>,
                namespace: String,
            }
            let body = Body {
                ids: ids.clone(),
                namespace: namespace.to_string(),
            };
            let url = format!("{}/vectors/delete", self.base_url);
            let resp = self
                .auth_header(self.http.post(url).json(&body))
                .send()
                .map_err(|e| golem_vector::error::from_reqwest_error("delete", e))?;
            let _: serde_json::Value = self.handle(resp, "delete_vectors")?;
            Ok(ids.len() as u32)
        }
    }
}

#[cfg(not(target_family = "wasm"))]
pub use native::PineconeClient;

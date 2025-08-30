use golem_vector::exports::golem::vector::types::{
    DistanceMetric, Metadata, MetadataKind, MetadataValue, VectorData, VectorError, VectorRecord,
};

use crate::conversion::{metadata_to_json_map, metric_to_milvus, vector_data_to_dense};

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
    use reqwest::{Client, Response};
    use serde::{de::DeserializeOwned, Deserialize, Serialize};
    use std::collections::HashMap;
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
            max_retries: u32,
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
        fn handle_response<T: DeserializeOwned>(
            &self,
            resp: Response,
            operation: &str,
        ) -> Result<T, VectorError> {
            match resp.status() {
                status if status.is_success() => resp.json().map_err(|e| {
                    VectorError::ProviderError(format!(
                        "Milvus {operation} response parsing error: {e}"
                    ))
                }),
                status => {
                    let error_body = resp.text().unwrap_or_default();
                    Err(VectorError::ProviderError(format!(
                        "Milvus {operation} error {status}: {error_body}"
                    )))
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
        /// Describe a collection and return (dimension, metric, vector_count)
        pub fn describe_collection(
            &self,
            name: &str,
        ) -> Result<(u32, DistanceMetric, u64), VectorError> {
            #[derive(Deserialize)]
            struct RespData {
                fields: Vec<Field>,
                #[serde(default)]
                indexes: Vec<Index>,
                #[serde(rename = "rowCount", default)]
                row_count: Option<u64>,
            }
            #[derive(Deserialize)]
            struct DescribeResp {
                data: RespData,
            }
            #[derive(Deserialize)]
            struct Field {
                #[serde(rename = "name")]
                name: String,
                #[serde(default)]
                params: Vec<Param>,
            }
            #[derive(Deserialize)]
            struct Param {
                key: String,
                value: String,
            }
            #[derive(Deserialize)]
            struct Index {
                #[serde(rename = "metricType", default)]
                metric_type: Option<String>,
            }

            fn metric_from_str(s: &str) -> DistanceMetric {
                match s.to_uppercase().as_str() {
                    "COSINE" => DistanceMetric::Cosine,
                    "IP" => DistanceMetric::DotProduct,
                    "L2" => DistanceMetric::Euclidean,
                    "HAMMING" => DistanceMetric::Hamming,
                    "JACCARD" => DistanceMetric::Jaccard,
                    _ => DistanceMetric::Cosine,
                }
            }

            let url = format!(
                "{}/v1/vector/collections/describe?collectionName={}",
                self.base_url, name
            );
            let resp = self.http.get(url).send().map_err(to_err)?;
            let body: DescribeResp = self.handle_response(resp, "describe_collection")?;

            // dimension: look for FloatVector field with dim param
            let mut dimension: u32 = 0;
            for field in &body.data.fields {
                for p in &field.params {
                    if p.key.eq_ignore_ascii_case("dim") {
                        if let Ok(d) = p.value.parse::<u32>() {
                            dimension = d;
                        }
                    }
                }
            }

            // metric: use first index metricType if available
            let metric = body
                .data
                .indexes
                .iter()
                .find_map(|idx| idx.metric_type.as_deref())
                .map(metric_from_str)
                .unwrap_or(DistanceMetric::Cosine);

            let count = body.data.row_count.unwrap_or(0);

            Ok((dimension, metric, count))
        }
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
        /// Delete vectors by IDs via Milvus REST `/v1/vector/delete` endpoint.
        pub fn delete_vectors(&self, name: &str, ids: Vec<String>) -> Result<u32, VectorError> {
            if ids.is_empty() {
                return Ok(0);
            }
            #[derive(Serialize)]
            struct DelReq<'a> {
                #[serde(rename = "collectionName")]
                collection_name: &'a str,
                #[serde(rename = "id")]
                id: &'a [String],
            }
            let url = format!("{}/v1/vector/delete", self.base_url);
            let payload = DelReq {
                collection_name: name,
                id: &ids,
            };
            let resp = self.http.post(url).json(&payload).send().map_err(to_err)?;
            // Successful deletion returns HTTP 200 with empty data.
            self.handle_response::<serde_json::Value>(resp, "delete_vectors")?;
            Ok(ids.len() as u32)
        }

        /// Low-level helper wrapping `/v1/vector/query` and returning raw JSON body.
        fn query_raw(&self, body: &serde_json::Value) -> Result<serde_json::Value, VectorError> {
            let url = format!("{}/v1/vector/query", self.base_url);
            let resp = self.http.post(url).json(body).send().map_err(to_err)?;
            self.handle_response(resp, "query_raw")
        }

        /// Query only `id` field, honoring filter / limit / offset.
        pub fn query_ids(
            &self,
            name: &str,
            expr: Option<String>,
            limit: u32,
            offset: u32,
        ) -> Result<Vec<String>, VectorError> {
            use serde_json::json;
            let body = json!({
                "collectionName": name,
                "outputFields": ["id"],
                "filter": expr,
                "limit": limit,
                "offset": offset
            });
            let json = self.query_raw(&body)?;
            let arr = json
                .get("data")
                .and_then(|d| d.as_array())
                .cloned()
                .unwrap_or_default();
            Ok(arr
                .into_iter()
                .filter_map(|v| v.get("id").and_then(|i| i.as_str()).map(|s| s.to_string()))
                .collect())
        }

        /// Count vectors matching optional filter by requesting `count(*)`.
        pub fn count_vectors(&self, name: &str, expr: Option<String>) -> Result<u64, VectorError> {
            use serde_json::json;
            let body = json!({
                "collectionName": name,
                "outputFields": ["count(*)"],
                "filter": expr,
                "limit": 0,
                "offset": 0
            });
            let json = self.query_raw(&body)?;
            json.get("data")
                .and_then(|d| d.get(0))
                .and_then(|obj| obj.get("count(*)"))
                .and_then(|c| c.as_u64())
                .ok_or_else(|| VectorError::ProviderError("Milvus count parse error".into()))
        }

        // -------------------- vectors ------------------------------
        pub fn upsert_vectors(
            &self,
            name: &str,
            records: Vec<VectorRecord>,
        ) -> Result<(), VectorError> {
            #[derive(Serialize)]
            struct VectorDataPayload {
                id: String,
                vector: Vec<f32>,
                metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
            }
            let url = format!("{}/v1/vector/insert", self.base_url);
            let mut payloads: Vec<VectorDataPayload> = Vec::new();
            for rec in records {
                let dense = vector_data_to_dense(rec.vector)?;
                let meta_json = metadata_to_json_map(rec.metadata);
                payloads.push(VectorDataPayload {
                    id: rec.id,
                    vector: dense,
                    metadata: meta_json,
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
            Ok(body
                .data
                .into_iter()
                .map(|it| VectorRecord {
                    id: it.id,
                    vector: VectorData::Dense(it.vector),
                    metadata: it.metadata.map(json_to_metadata),
                })
                .collect())
        }
    }

    // Convert a JSON object returned by Milvus into the new Metadata format
    fn json_to_metadata(map: HashMap<String, serde_json::Value>) -> Metadata {
        let mut next_id: u64 = 1;
        map.into_iter()
            .filter_map(|(k, v)| {
                json_to_kind(&v).map(|kind| {
                    let id = next_id;
                    next_id += 1;
                    (k, MetadataValue { id, kind })
                })
            })
            .collect()
    }

    fn json_to_kind(v: &serde_json::Value) -> Option<MetadataKind> {
        match v {
            serde_json::Value::String(s) => Some(MetadataKind::StringVal(s.clone())),
            serde_json::Value::Bool(b) => Some(MetadataKind::BoolVal(*b)),
            serde_json::Value::Number(n) => {
                if let Some(u) = n.as_u64() {
                    Some(MetadataKind::IntVal(u))
                } else if let Some(f) = n.as_f64() {
                    Some(MetadataKind::FloatVal(f))
                } else if let Some(i) = n.as_i64() {
                    if i >= 0 {
                        Some(MetadataKind::IntVal(i as u64))
                    } else {
                        Some(MetadataKind::FloatVal(i as f64))
                    }
                } else {
                    None
                }
            }
            // For arrays/objects/null, fall back to stringified JSON to avoid building reference graphs
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                Some(MetadataKind::StringVal(v.to_string()))
            }
            serde_json::Value::Null => None,
        }
    }

    fn to_err(e: impl std::fmt::Display) -> VectorError {
        VectorError::ProviderError(e.to_string())
    }
}

#[cfg(not(target_family = "wasm"))]
pub use native::MilvusClient;

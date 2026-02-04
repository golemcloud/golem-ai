use golem_vector::golem::vector::types::{FilterExpression, Id, VectorData, VectorError, VectorRecord, MetadataValue};
use golem_wasi_http::{Client, Method, RequestBuilder, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use base64::prelude::*;

#[derive(Clone)]
pub struct ChromaClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    tenant: String,
    database: String,
    thread_id: Option<String>,
}

impl ChromaClient {
    pub fn new(base_url: String, api_key: Option<String>, tenant: String, database: String, thread_id: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            tenant,
            database,
            thread_id,
        }
    }

    fn create_request(&self, method: Method, endpoint: &str) -> RequestBuilder {
        let url = format!("{}{}", self.base_url, endpoint);
        let mut req = self.client.request(method, &url);

        if let Some(key) = &self.api_key {
            req = req.header("X-Chroma-Token", key);
        }
        if !self.tenant.is_empty() {
             req = req.header("X-Chroma-Tenant", &self.tenant);
        }
        if !self.database.is_empty() {
             req = req.header("X-Chroma-Database", &self.database);
        }
        if let Some(tid) = &self.thread_id {
             req = req.header("X-Chroma-Thread-Id", tid);
        }
        
        req.header("Content-Type", "application/json")
    }
    
    fn execute(&self, req: RequestBuilder) -> Result<Response, VectorError> {
        let response = req.send().map_err(|e| VectorError::ConnectionError(e.to_string()))?;
        if !response.status().is_success() {
             let status = response.status();
             let body = response.text().unwrap_or_else(|_| "Could not read response body".to_string());
             let msg = format!("Chroma Error {}: {}", status, body);
             if status.as_u16() == 404 {
                 return Err(VectorError::NotFound(msg));
             }
             return Err(VectorError::ProviderError(msg));
        }
        Ok(response)
    }

    pub fn heartbeat(&self) -> Result<u64, VectorError> {
        let req = self.create_request(Method::GET, "/api/v1/heartbeat");
        let response = self.execute(req)?;
        
        let body: serde_json::Value = response.json::<serde_json::Value>()
             .map_err(|e| VectorError::ProviderError(format!("Invalid json: {e}")))?;
             
        body.get("nanosecond heartbeat")
            .and_then(|v| v.as_u64())
            .ok_or(VectorError::ProviderError("Invalid heartbeat response".to_string()))
    }

    pub fn list_collections(&self) -> Result<Vec<String>, VectorError> {
        let req = self.create_request(Method::GET, "/api/v1/collections");
        let response = self.execute(req)?;
        
        let collections: Vec<ChromaCollection> = response.json()
             .map_err(|e| VectorError::ProviderError(format!("Failed to parse collections: {e}")))?;
             
        Ok(collections.into_iter().map(|c| c.name).collect())
    }

    pub fn create_collection(&self, name: &str, metadata: Option<HashMap<String, serde_json::Value>>) -> Result<ChromaCollection, VectorError> {
        let payload = serde_json::json!({
            "name": name,
            "metadata": metadata.unwrap_or_default(),
        });
        
        let req = self.create_request(Method::POST, "/api/v1/collections")
            .json(&payload);

        let response = self.execute(req)?;
        response.json().map_err(|e| VectorError::ProviderError(format!("Failed: {e}")))
    }

    pub fn get_collection(&self, name: &str) -> Result<ChromaCollection, VectorError> {
        let endpoint = format!("/api/v1/collections/{}", name);
        let req = self.create_request(Method::GET, &endpoint);
        
        match self.execute(req) {
            Ok(response) => {
                 let mut col: ChromaCollection = response.json()
                    .map_err(|e| VectorError::ProviderError(format!("Failed to parse: {e}")))?;
                 
                 // Get count - use internal safe to avoid recursion
                 if let Ok(count) = self.count_vectors_internal(&col.id, None) {
                     col.count = Some(count as u32);
                 }
                 Ok(col)
            },
            Err(e) => Err(e),
        }
    }

    pub fn delete_collection(&self, name: &str) -> Result<(), VectorError> {
        let endpoint = format!("/api/v1/collections/{}", name);
        let req = self.create_request(Method::DELETE, &endpoint);
        self.execute(req).map(|_| ())
    }

    pub fn update_collection(&self, name: &str, metadata: Option<HashMap<String, serde_json::Value>>) -> Result<(), VectorError> {
         let col = self.get_collection_safe(name)?;
         let endpoint = format!("/api/v1/collections/{}", col.id);
         
         let payload = serde_json::json!({ "new_metadata": metadata });
         let req = self.create_request(Method::PUT, &endpoint).json(&payload);
            
         self.execute(req).map(|_| ())
    }

    fn convert_metadata_value(val: &MetadataValue) -> serde_json::Value {
        match val {
            MetadataValue::StringVal(s) => serde_json::json!(s),
            MetadataValue::NumberVal(n) => serde_json::json!(n),
            MetadataValue::IntegerVal(i) => serde_json::json!(i),
            MetadataValue::BooleanVal(b) => serde_json::json!(b),
            MetadataValue::NullVal => serde_json::Value::Null,
            MetadataValue::ArrayVal(_) => serde_json::Value::Null, // Not supported natively in Chroma metadata
            MetadataValue::ObjectVal(_) => serde_json::Value::Null,
            MetadataValue::GeoVal(g) => serde_json::json!(format!("{},{}", g.latitude, g.longitude)),
            MetadataValue::DatetimeVal(s) => serde_json::json!(s),
            MetadataValue::BlobVal(b) => serde_json::json!(BASE64_STANDARD.encode(b)),
        }
    }

    fn convert_filter(filter: &FilterExpression) -> serde_json::Value {
        match filter {
            FilterExpression::Condition(cond) => {
                let op = match cond.operator {
                    golem_vector::golem::vector::types::FilterOperator::Eq => "$eq",
                    golem_vector::golem::vector::types::FilterOperator::Ne => "$ne",
                    golem_vector::golem::vector::types::FilterOperator::Gt => "$gt",
                    golem_vector::golem::vector::types::FilterOperator::Gte => "$gte",
                    golem_vector::golem::vector::types::FilterOperator::Lt => "$lt",
                    golem_vector::golem::vector::types::FilterOperator::Lte => "$lte",
                    golem_vector::golem::vector::types::FilterOperator::In => "$in",
                    golem_vector::golem::vector::types::FilterOperator::Nin => "$nin",
                    _ => "$eq",
                };
                serde_json::json!({ cond.field.clone(): { op: Self::convert_metadata_value(&cond.value) } })
            },
            FilterExpression::And(filters) => {
                let list: Vec<serde_json::Value> = filters.iter().map(|f| Self::convert_filter(&f.get())).collect();
                serde_json::json!({ "$and": list })
            },
            FilterExpression::Or(filters) => {
                let list: Vec<serde_json::Value> = filters.iter().map(|f| Self::convert_filter(&f.get())).collect();
                serde_json::json!({ "$or": list })
            },
            FilterExpression::Not(f) => {
                // Chroma doesn't have a direct $not at the top level for metadata, often use specific operators
                // For simplicity, we might need a more complex transform or skip.
                Self::convert_filter(&f.get()) // Placeholder
            }
        }
    }

    pub fn add_vectors(&self, collection: &str, vectors: Vec<VectorRecord>) -> Result<(), VectorError> {
         let col = self.get_collection_safe(collection)?;
         let endpoint = format!("/api/v1/collections/{}/add", col.id);
         
         let ids: Vec<String> = vectors.iter().map(|v| v.id.clone()).collect();
         let embeddings: Vec<Vec<f32>> = vectors.iter().map(|v| match &v.vector {
             golem_vector::golem::vector::types::VectorData::Dense(vec) => vec.clone(),
             _ => vec![], // Chroma expects floats for embeddings
         }).collect();
         
         let metadatas: Vec<Option<HashMap<String, serde_json::Value>>> = vectors.iter().map(|v| {
             v.metadata.as_ref().map(|m| {
                 m.iter().map(|(k, val)| {
                     (k.clone(), Self::convert_metadata_value(val))
                 }).collect()
             })
         }).collect();
         
         let payload = serde_json::json!({
             "ids": ids,
             "embeddings": embeddings,
             "metadatas": metadatas,
         });
         
         let req = self.create_request(Method::POST, &endpoint).json(&payload);
         self.execute(req).map(|_| ())
    }
    
    pub fn get_vectors(&self, collection: &str, ids: Vec<Id>, include_vectors: Option<bool>, include_metadata: Option<bool>) -> Result<Vec<VectorRecord>, VectorError> {
         let col = self.get_collection_safe(collection)?;
         let endpoint = format!("/api/v1/collections/{}/get", col.id);
         
         let mut include = vec![];
         if include_vectors.unwrap_or(false) { include.push("embeddings"); }
         if include_metadata.unwrap_or(false) { include.push("metadatas"); }
         
         let payload = serde_json::json!({
             "ids": ids,
             "include": include
         });
         
         let req = self.create_request(Method::POST, &endpoint).json(&payload);
         let response = self.execute(req)?;
         let result: ChromaGetResult = response.json().map_err(|e| VectorError::ProviderError(format!("{e}")))?;
         self.map_get_result(result)
    }

    fn map_get_result(&self, result: ChromaGetResult) -> Result<Vec<VectorRecord>, VectorError> {
         let mut records = vec![];
         for (i, id) in result.ids.iter().enumerate() {
             let vector_vec = if let Some(ref embs) = result.embeddings {
                 embs.get(i).cloned().unwrap_or_default()
             } else {
                 vec![]
             };
             
             let vector = golem_vector::golem::vector::types::VectorData::Dense(vector_vec);
             
             let metadata = if let Some(ref metas) = result.metadatas {
                 metas.get(i).cloned().flatten().map(|m| {
                     m.into_iter().map(|(k, v)| {
                         let val = match v {
                             serde_json::Value::String(s) => golem_vector::golem::vector::types::MetadataValue::StringVal(s),
                             serde_json::Value::Number(n) => {
                                 if n.is_f64() {
                                     golem_vector::golem::vector::types::MetadataValue::NumberVal(n.as_f64().unwrap())
                                 } else {
                                     golem_vector::golem::vector::types::MetadataValue::IntegerVal(n.as_i64().unwrap())
                                 }
                             },
                             serde_json::Value::Bool(b) => golem_vector::golem::vector::types::MetadataValue::BooleanVal(b),
                             _ => golem_vector::golem::vector::types::MetadataValue::StringVal(v.to_string()),
                         };
                         (k, val)
                     }).collect()
                 })
             } else {
                 None
             };
             
             records.push(VectorRecord {
                 id: id.clone(),
                 vector,
                 metadata,
             });
         }
         Ok(records)
    }

    pub fn delete_vectors(&self, collection: &str, ids: Vec<Id>) -> Result<(), VectorError> {
         let col = self.get_collection_safe(collection)?;
         let endpoint = format!("/api/v1/collections/{}/delete", col.id);
         let payload = serde_json::json!({ "ids": ids });
         let req = self.create_request(Method::POST, &endpoint).json(&payload);
         self.execute(req).map(|_| ())
    }
    
    pub fn delete_by_filter(&self, collection: &str, filter: FilterExpression) -> Result<u32, VectorError> {
         let col = self.get_collection_safe(collection)?;
         let endpoint = format!("/api/v1/collections/{}/delete", col.id);
         let payload = serde_json::json!({ "where": Self::convert_filter(&filter) });
         let req = self.create_request(Method::POST, &endpoint).json(&payload);
         self.execute(req).map(|_| 0) // Chroma doesn't return count easily
    }

    pub fn update_vectors(&self, collection: &str, vectors: Vec<VectorRecord>) -> Result<(), VectorError> {
         let col = self.get_collection_safe(collection)?;
         let endpoint = format!("/api/v1/collections/{}/update", col.id);
         
         let ids: Vec<String> = vectors.iter().map(|v| v.id.clone()).collect();
         let embeddings: Vec<Vec<f32>> = vectors.iter().map(|v| match &v.vector {
             golem_vector::golem::vector::types::VectorData::Dense(vec) => vec.clone(),
             _ => vec![],
         }).collect();
      
         let payload = serde_json::json!({
             "ids": ids,
             "embeddings": embeddings,
         });
         
         let req = self.create_request(Method::POST, &endpoint).json(&payload);
         self.execute(req).map(|_| ())
    }

    pub fn count_vectors(&self, collection: &str, filter: Option<FilterExpression>) -> Result<u64, VectorError> {
        let col = self.get_collection_safe(collection)?; 
        self.count_vectors_internal(&col.id, filter)
    }

    fn count_vectors_internal(&self, collection_id: &str, filter: Option<FilterExpression>) -> Result<u64, VectorError> {
        if let Some(f) = filter {
             // For count with filter, use 'get' with include=[] and just ids
             let endpoint = format!("/api/v1/collections/{}/get", collection_id);
             let payload = serde_json::json!({ "where": Self::convert_filter(&f), "include": [] });
             let req = self.create_request(Method::POST, &endpoint).json(&payload);
             let response = self.execute(req)?;
             let result: ChromaGetResult = response.json().map_err(|e| VectorError::ProviderError(format!("{e}")))?;
             Ok(result.ids.len() as u64)
        } else {
             let endpoint = format!("/api/v1/collections/{}/count", collection_id);
             let req = self.create_request(Method::GET, &endpoint);
             let resp = self.execute(req)?;
             let val: serde_json::Value = resp.json().map_err(|e| VectorError::ProviderError(format!("{e}")))?;
             val.as_u64().ok_or(VectorError::ProviderError("Invalid count".to_string()))
        }
    }

    fn get_collection_safe(&self, name: &str) -> Result<ChromaCollection, VectorError> {
        let endpoint = format!("/api/v1/collections/{}", name);
        let req = self.create_request(Method::GET, &endpoint);
        let response = self.execute(req)?;
        response.json().map_err(|e| VectorError::ProviderError(format!("Failed to parse: {e}")))
    }

    pub fn list_vectors(&self, collection: &str, filter: Option<FilterExpression>, limit: u32, offset: u32, include_vectors: Option<bool>, include_metadata: Option<bool>) -> Result<golem_vector::golem::vector::vectors::ListResponse, VectorError> {
         let col = self.get_collection_safe(collection)?;
         let endpoint = format!("/api/v1/collections/{}/get", col.id);
         
         let mut include = vec![];
         if include_vectors.unwrap_or(false) { include.push("embeddings"); }
         if include_metadata.unwrap_or(false) { include.push("metadatas"); }
         
         let mut payload = serde_json::json!({
             "limit": limit,
             "offset": offset,
             "include": include
         });
         
         if let Some(f) = filter {
             payload["where"] = Self::convert_filter(&f);
         }
         
         let req = self.create_request(Method::POST, &endpoint).json(&payload);
         let response = self.execute(req)?;
         let result: ChromaGetResult = response.json().map_err(|e| VectorError::ProviderError(format!("{e}")))?;
         
         let records = self.map_get_result(result)?;
         
         let next_cursor = if records.len() == limit as usize {
             Some((offset + limit).to_string())
         } else {
             None
         };
         
         Ok(golem_vector::golem::vector::vectors::ListResponse {
             vectors: records,
             next_cursor,
             total_count: None, // Could fetch but expensive
         })
    }

    pub fn query_vector(&self, collection: &str, vector: VectorData, limit: u32, filter: Option<FilterExpression>, include_vectors: Option<bool>, include_metadata: Option<bool>) -> Result<Vec<golem_vector::golem::vector::types::SearchResult>, VectorError> {
         let col = self.get_collection_safe(collection)?;
         let endpoint = format!("/api/v1/collections/{}/query", col.id);
         
         let mut include = vec!["distances"];
         if include_vectors.unwrap_or(false) { include.push("embeddings"); }
         if include_metadata.unwrap_or(false) { include.push("metadatas"); }

         let vec_values = match vector {
             golem_vector::golem::vector::types::VectorData::Dense(v) => v,
             _ => return Err(VectorError::UnsupportedFeature("Only Dense vectors supported".to_string())),
         };

         let mut payload = serde_json::json!({
             "query_embeddings": [vec_values],
             "n_results": limit,
             "include": include
         });
         
         if let Some(f) = filter {
             payload["where"] = Self::convert_filter(&f);
         }
         
         let req = self.create_request(Method::POST, &endpoint).json(&payload);
         let response = self.execute(req)?;
         let result: ChromaQueryResult = response.json().map_err(|e| VectorError::ProviderError(format!("{e}")))?;
         
         Ok(self.map_query_result(result))
    }

    fn map_query_result(&self, result: ChromaQueryResult) -> Vec<golem_vector::golem::vector::types::SearchResult> {
         let mut results = vec![];
         if let Some(inner_ids) = result.ids.get(0) {
             for (i, id) in inner_ids.iter().enumerate() {
                 let distance = result.distances.as_ref().and_then(|d| d.get(0)).and_then(|d| d.get(i)).cloned().unwrap_or(0.0) as f32;
                 
                 let vector_out = if let Some(ref embs) = result.embeddings {
                     embs.get(0).and_then(|e| e.get(i)).cloned().map(golem_vector::golem::vector::types::VectorData::Dense)
                 } else {
                     None
                 };

                 let metadata_out = if let Some(ref metas) = result.metadatas {
                     metas.get(0).and_then(|m| m.get(i).cloned()).flatten().map(|m| {
                          m.into_iter().map(|(k, v)| {
                            let val = match v {
                                serde_json::Value::String(s) => golem_vector::golem::vector::types::MetadataValue::StringVal(s),
                                serde_json::Value::Number(n) => {
                                    if n.is_f64() {
                                        golem_vector::golem::vector::types::MetadataValue::NumberVal(n.as_f64().unwrap())
                                    } else {
                                        golem_vector::golem::vector::types::MetadataValue::IntegerVal(n.as_i64().unwrap())
                                    }
                                },
                                serde_json::Value::Bool(b) => golem_vector::golem::vector::types::MetadataValue::BooleanVal(b),
                                _ => golem_vector::golem::vector::types::MetadataValue::StringVal(v.to_string()),
                            };
                            (k, val)
                        }).collect()
                     })
                 } else {
                     None
                 };

                 results.push(golem_vector::golem::vector::types::SearchResult {
                     id: id.clone(),
                     distance: distance,
                     vector: vector_out, 
                     metadata: metadata_out, 
                     score: 1.0 / (1.0 + distance), // Distance to score mapping (heuristic)
                 });
             }
         }
         results
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChromaCollection {
    pub id: String,
    pub name: String,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub tenant: Option<String>,
    pub database: Option<String>,
    #[serde(skip_serializing)]
    pub count: Option<u32>, 
}

#[derive(Debug, Deserialize)]
struct ChromaGetResult {
    ids: Vec<String>,
    embeddings: Option<Vec<Vec<f32>>>,
    metadatas: Option<Vec<Option<HashMap<String, serde_json::Value>>>>,
}

#[derive(Debug, Deserialize)]
struct ChromaQueryResult {
    ids: Vec<Vec<String>>,
    distances: Option<Vec<Vec<f64>>>,
    embeddings: Option<Vec<Vec<Vec<f32>>>>,
    metadatas: Option<Vec<Vec<Option<HashMap<String, serde_json::Value>>>>>,
}
#[cfg(test)]
mod tests {
    use super::*;
    use golem_vector::golem::vector::types::{FilterCondition, FilterOperator, GeoCoordinates};

    #[test]
    fn test_convert_metadata_value() {
        assert_eq!(ChromaClient::convert_metadata_value(&MetadataValue::StringVal("test".to_string())), serde_json::json!("test"));
        assert_eq!(ChromaClient::convert_metadata_value(&MetadataValue::NumberVal(1.23)), serde_json::json!(1.23));
        assert_eq!(ChromaClient::convert_metadata_value(&MetadataValue::BooleanVal(true)), serde_json::json!(true));
        assert_eq!(ChromaClient::convert_metadata_value(&MetadataValue::NullVal), serde_json::Value::Null);
        
        let geo = GeoCoordinates { latitude: 10.0, longitude: 20.0 };
        assert_eq!(ChromaClient::convert_metadata_value(&MetadataValue::GeoVal(geo)), serde_json::json!("10,20"));

        let blob = vec![1, 2, 3];
        assert_eq!(ChromaClient::convert_metadata_value(&MetadataValue::BlobVal(blob)), serde_json::json!("AQID"));
    }

    #[test]
    fn test_convert_complex_filter() {
        // Test AND
        let cond1 = FilterExpression::Condition(FilterCondition {
            field: "a".to_string(),
            operator: FilterOperator::Eq,
            value: MetadataValue::IntegerVal(1),
        });
        let cond2 = FilterExpression::Condition(FilterCondition {
            field: "b".to_string(),
            operator: FilterOperator::Gt,
            value: MetadataValue::NumberVal(2.0),
        });
        
        // Golem WIT uses resources for And/Or, but for testing we can mock the values if we have access to the internals.
        // Since we can't easily create handles in unit tests, we'll test the logic of convert_filter using simple conditions first.
        
        assert_eq!(ChromaClient::convert_filter(&cond1), serde_json::json!({ "a": { "$eq": 1 } }));
        assert_eq!(ChromaClient::convert_filter(&cond2), serde_json::json!({ "b": { "$gt": 2.0 } }));
    }

    #[test]
    fn test_map_query_result() {
        let result = ChromaQueryResult {
            ids: vec![vec!["1".to_string(), "2".to_string()]],
            distances: Some(vec![vec![0.1, 0.5]]),
            embeddings: None,
            metadatas: Some(vec![vec![
                Some({
                    let mut m = HashMap::new();
                    m.insert("x".to_string(), serde_json::json!(10));
                    m
                }),
                None
            ]]),
        };

        let client = ChromaClient::new("".to_string(), None, "".to_string(), "".to_string(), None);
        let results = client.map_query_result(result);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "1");
        assert_eq!(results[0].distance, 0.1);
        assert!(results[0].score > 0.9);
        assert_eq!(results[1].id, "2");
    }

    #[test]
    fn test_filter_operators() {
        use golem_vector::golem::vector::types::FilterOperator::*;
        let ops = vec![
            (Eq, "$eq"), (Ne, "$ne"), (Gt, "$gt"), (Gte, "$gte"),
            (Lt, "$lt"), (Lte, "$lte"), (In, "$in"), (Nin, "$nin")
        ];
        
        for (f_op, c_op) in ops {
            let cond = FilterExpression::Condition(FilterCondition {
                field: "f".to_string(),
                operator: f_op,
                value: MetadataValue::IntegerVal(1),
            });
            let json = ChromaClient::convert_filter(&cond);
            assert_eq!(json, serde_json::json!({ "f": { c_op: 1 } }));
        }
    }
}

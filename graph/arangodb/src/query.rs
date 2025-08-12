use crate::{conversions, GraphArangoDbComponent, Transaction};
use golem_graph::golem::graph::{
    errors::GraphError,
    query::{
        Guest as QueryGuest, QueryExecutionResult, QueryOptions, QueryParameters, QueryResult,
    },
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct ArangoQueryResponse {
    pub result: Vec<Value>,
}

#[derive(Deserialize, Debug)]
pub struct ArangoDocument {
    #[serde(rename = "_id")]
    pub id: Option<String>,
    #[serde(rename = "_key")]
    pub key: Option<String>,
    #[serde(rename = "_from")]
    pub from: Option<String>,
    #[serde(rename = "_to")]
    pub to: Option<String>,
    #[serde(flatten)]
    pub properties: HashMap<String, Value>,
}

impl ArangoDocument {
    pub fn is_edge(&self) -> bool {
        self.from.is_some() && self.to.is_some()
    }

    pub fn is_vertex(&self) -> bool {
        self.id.is_some() && !self.is_edge()
    }

    pub fn extract_collection(&self) -> String {
        if let Some(id) = &self.id {
            id.split('/').next().unwrap_or_default().to_string()
        } else {
            String::new()
        }
    }
}

impl Transaction {
    pub fn execute_query(
        &self,
        query: String,
        parameters: Option<QueryParameters>,
        _options: Option<QueryOptions>,
    ) -> Result<QueryExecutionResult, GraphError> {
        let mut bind_vars = serde_json::Map::new();
        if let Some(p) = parameters {
            for (key, value) in p {
                bind_vars.insert(key, conversions::to_arango_value(value)?);
            }
        }

        let query_json = serde_json::json!({
            "query": query,
            "bindVars": bind_vars,
        });

        let response = self
            .api
            .execute_in_transaction(&self.transaction_id, query_json)?;

        let result_array = if let Some(array) = response.as_array() {
            array.clone()
        } else {
            let structured_response: Result<ArangoQueryResponse, _> =
                serde_json::from_value(response.clone());
            match structured_response {
                Ok(resp) => resp.result.into_iter().collect(),
                Err(_) => {
                    return Err(GraphError::InternalError(
                        "Unexpected AQL query response format".to_string(),
                    ));
                }
            }
        };

        let query_result_value = if result_array.is_empty() {
            QueryResult::Values(vec![])
        } else {
            self.parse_query_results(result_array)?
        };

        Ok(QueryExecutionResult {
            query_result_value,
            execution_time_ms: None,
            rows_affected: None,
            explanation: None,
            profile_data: None,
        })
    }

    fn parse_query_results(&self, result_array: Vec<Value>) -> Result<QueryResult, GraphError> {
        if let Some(first_value) = result_array.first() {
            if let Ok(first_doc) = serde_json::from_value::<ArangoDocument>(first_value.clone()) {
                if first_doc.is_edge() {
                    let mut edges = Vec::new();
                    for item in result_array {
                        if let Ok(doc) = serde_json::from_value::<ArangoDocument>(item) {
                            let collection = doc.extract_collection();
                            let mut doc_map = serde_json::Map::new();
                            if let Some(id) = doc.id {
                                doc_map.insert("_id".to_string(), Value::String(id));
                            }
                            if let Some(from) = doc.from {
                                doc_map.insert("_from".to_string(), Value::String(from));
                            }
                            if let Some(to) = doc.to {
                                doc_map.insert("_to".to_string(), Value::String(to));
                            }
                            for (key, value) in doc.properties {
                                doc_map.insert(key, value);
                            }
                            edges.push(crate::helpers::parse_edge_from_document(
                                &doc_map,
                                &collection,
                            )?);
                        }
                    }
                    return Ok(QueryResult::Edges(edges));
                } else if first_doc.is_vertex() {
                    let mut vertices = Vec::new();
                    for item in result_array {
                        if let Ok(doc) = serde_json::from_value::<ArangoDocument>(item) {
                            let collection = doc.extract_collection();
                            let mut doc_map = serde_json::Map::new();
                            if let Some(id) = doc.id {
                                doc_map.insert("_id".to_string(), Value::String(id));
                            }
                            if let Some(key) = doc.key {
                                doc_map.insert("_key".to_string(), Value::String(key));
                            }
                            for (key, value) in doc.properties {
                                doc_map.insert(key, value);
                            }
                            vertices.push(crate::helpers::parse_vertex_from_document(
                                &doc_map,
                                &collection,
                            )?);
                        }
                    }
                    return Ok(QueryResult::Vertices(vertices));
                }
            }

            if first_value.is_object() {
                let mut maps = Vec::new();
                for item in result_array {
                    if let Some(obj) = item.as_object() {
                        let mut map_row = Vec::new();
                        for (key, value) in obj {
                            map_row.push((
                                key.clone(),
                                conversions::from_arango_value(value.clone())?,
                            ));
                        }
                        maps.push(map_row);
                    }
                }
                return Ok(QueryResult::Maps(maps));
            }
        }

        let mut values = Vec::new();
        for item in result_array {
            values.push(conversions::from_arango_value(item)?);
        }
        Ok(QueryResult::Values(values))
    }
}

impl QueryGuest for GraphArangoDbComponent {
    fn execute_query(
        transaction: golem_graph::golem::graph::transactions::TransactionBorrow<'_>,
        query: String,
        parameters: Option<QueryParameters>,
        options: Option<QueryOptions>,
    ) -> Result<QueryExecutionResult, GraphError> {
        let tx: &Transaction = transaction.get();
        tx.execute_query(query, parameters, options)
    }
}

use crate::conversions;
use crate::{GraphJanusGraphComponent, Transaction};
use golem_graph::golem::graph::types::PropertyValue;
use golem_graph::golem::graph::{
    errors::GraphError,
    query::{Guest as QueryGuest, QueryExecutionResult, QueryParameters, QueryResult},
};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct GremlinQueryResponse {
    pub _result: Option<GremlinQueryResult>,
}

#[derive(Deserialize, Debug)]
pub struct GremlinQueryResult {
    pub _data: Option<GraphSONValue>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum GraphSONValue {
    Object(GraphSONObject),
    Array(Vec<Value>),
    Primitive(Value),
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct GraphSONObject {
    #[serde(rename = "@type")]
    pub object_type: Option<String>,
    #[serde(rename = "@value")]
    pub value: Option<Value>,
}

#[derive(Deserialize, Debug)]
pub struct GraphSONVertex {
    pub id: Option<Value>,
    pub label: Option<String>,
    pub properties: Option<HashMap<String, Vec<GraphSONProperty>>>,
    #[serde(rename = "outV")]
    pub out_v: Option<Value>,
    #[serde(rename = "inV")]
    pub in_v: Option<Value>,
}

#[derive(Deserialize, Debug)]
pub struct GraphSONProperty {
    #[allow(dead_code)]
    pub id: Option<String>,
    pub value: Option<Value>,
    #[serde(rename = "@value")]
    pub at_value: Option<GraphSONPropertyValue>,
}

#[derive(Deserialize, Debug)]
pub struct GraphSONPropertyValue {
    pub value: Option<Value>,
}

#[derive(Deserialize, Debug)]
pub struct GraphSONMap {
    #[serde(rename = "@type")]
    #[allow(dead_code)]
    pub map_type: Option<String>,
    #[serde(rename = "@value")]
    pub value: Option<Vec<Value>>,
}

#[derive(Deserialize, Debug)]
pub struct GraphSONList {
    #[serde(rename = "@type")]
    #[allow(dead_code)]
    pub list_type: Option<String>,
    #[serde(rename = "@value")]
    pub value: Option<Vec<Value>>,
}

fn to_bindings(parameters: QueryParameters) -> Result<Map<String, Value>, GraphError> {
    let mut bindings = Map::new();
    for (key, value) in parameters {
        let json_value = match value {
            PropertyValue::Float32Value(f) => json!(f),
            PropertyValue::Float64Value(f) => json!(f),
            PropertyValue::Int32(i) => json!(i),
            PropertyValue::Int64(i) => json!(i),
            PropertyValue::Boolean(b) => json!(b),
            PropertyValue::StringValue(s) => json!(s),
            _ => conversions::to_json_value(value)?,
        };

        bindings.insert(key, json_value);
    }
    Ok(bindings)
}

fn extract_result_data(response: &Value) -> Result<Option<&Value>, GraphError> {
    if response.is_array() || response.is_object() {
        Ok(Some(response))
    } else {
        Ok(None)
    }
}

fn parse_graphson_vertex(item: &Value) -> Result<Vec<(String, PropertyValue)>, GraphError> {
    if let Some(value_obj) = item.get("@value") {
        if let Ok(vertex) = serde_json::from_value::<GraphSONVertex>(value_obj.clone()) {
            let mut row = Vec::new();

            if let Some(id_val) = vertex.id {
                if let Ok(id_value) = conversions::from_gremlin_value(&id_val) {
                    row.push(("id".to_string(), id_value));
                }
            }

            if let Some(label) = vertex.label {
                row.push(("label".to_string(), PropertyValue::StringValue(label)));
            }

            if let Some(properties) = vertex.properties {
                for (prop_key, prop_array) in properties {
                    if let Some(first_prop) = prop_array.first() {
                        if let Some(prop_value) = &first_prop.value {
                            if let Ok(converted_value) = conversions::from_gremlin_value(prop_value)
                            {
                                row.push((prop_key, converted_value));
                                continue;
                            }
                        }

                        if let Some(at_value) = &first_prop.at_value {
                            if let Some(actual_value) = &at_value.value {
                                if let Ok(converted_value) =
                                    conversions::from_gremlin_value(actual_value)
                                {
                                    row.push((prop_key, converted_value));
                                }
                            }
                        }
                    }
                }
            }

            if let Some(from_vertex) = vertex.out_v {
                if let Ok(from_value) = conversions::from_gremlin_value(&from_vertex) {
                    row.push(("from".to_string(), from_value));
                }
            }
            if let Some(to_vertex) = vertex.in_v {
                if let Ok(to_value) = conversions::from_gremlin_value(&to_vertex) {
                    row.push(("to".to_string(), to_value));
                }
            }

            return Ok(row);
        }
    }

    Err(GraphError::InternalError(
        "Failed to parse GraphSON vertex/edge".to_string(),
    ))
}

fn parse_graphson_map(item: &Value) -> Result<Vec<(String, PropertyValue)>, GraphError> {
    if let Ok(graphson_map) = serde_json::from_value::<GraphSONMap>(item.clone()) {
        if let Some(map_array) = graphson_map.value {
            let mut row = Vec::new();
            let mut i = 0;

            while i + 1 < map_array.len() {
                if let (Some(key_val), Some(value_val)) = (map_array.get(i), map_array.get(i + 1)) {
                    if let Some(key_str) = key_val.as_str() {
                        let converted_value = if let Ok(graphson_list) =
                            serde_json::from_value::<GraphSONList>(value_val.clone())
                        {
                            if let Some(list_values) = graphson_list.value {
                                if let Some(first_value) = list_values.first() {
                                    conversions::from_gremlin_value(first_value)?
                                } else {
                                    i += 2;
                                    continue;
                                }
                            } else {
                                i += 2;
                                continue;
                            }
                        } else {
                            conversions::from_gremlin_value(value_val)?
                        };

                        row.push((key_str.to_string(), converted_value));
                    }
                }
                i += 2;
            }

            return Ok(row);
        }
    }

    Err(GraphError::InternalError(
        "Failed to parse GraphSON map".to_string(),
    ))
}

fn parse_plain_object(item: &Value) -> Result<Vec<(String, PropertyValue)>, GraphError> {
    if let Some(object_map) = item.as_object() {
        let mut row = Vec::new();

        for (key, gremlin_value) in object_map {
            let converted_value = if let Ok(graphson_list) =
                serde_json::from_value::<GraphSONList>(gremlin_value.clone())
            {
                if let Some(list_values) = graphson_list.value {
                    if let Some(first_value) = list_values.first() {
                        conversions::from_gremlin_value(first_value)?
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            } else if let Some(inner_array) = gremlin_value.as_array() {
                if let Some(actual_value) = inner_array.first() {
                    conversions::from_gremlin_value(actual_value)?
                } else {
                    continue;
                }
            } else {
                conversions::from_gremlin_value(gremlin_value)?
            };

            row.push((key.clone(), converted_value));
        }

        return Ok(row);
    }

    Err(GraphError::InternalError(
        "Expected object for plain map".to_string(),
    ))
}

fn parse_gremlin_response(response: Value) -> Result<QueryResult, GraphError> {
    let result_data = extract_result_data(&response)?.ok_or_else(|| {
        GraphError::InternalError("Invalid response structure from Gremlin".to_string())
    })?;

    let arr = if let Some(graphson_obj) = result_data.as_object() {
        if let Some(value_array) = graphson_obj.get("@value").and_then(|v| v.as_array()) {
            value_array
        } else {
            return Ok(QueryResult::Values(vec![]));
        }
    } else if let Some(direct_array) = result_data.as_array() {
        direct_array
    } else {
        return Ok(QueryResult::Values(vec![]));
    };

    if arr.is_empty() {
        return Ok(QueryResult::Values(vec![]));
    }

    let first_item = arr
        .first()
        .ok_or_else(|| GraphError::InternalError("Empty result array".to_string()))?;

    if !first_item.is_object() {
        let values = arr
            .iter()
            .map(conversions::from_gremlin_value)
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(QueryResult::Values(values));
    }

    let obj = first_item
        .as_object()
        .ok_or_else(|| GraphError::InternalError("Expected object in result array".to_string()))?;

    if obj.get("@type") == Some(&Value::String("g:Vertex".to_string()))
        || obj.get("@type") == Some(&Value::String("g:Edge".to_string()))
    {
        let mut maps = Vec::new();
        for item in arr {
            if let Ok(row) = parse_graphson_vertex(item) {
                if !row.is_empty() {
                    maps.push(row);
                }
            }
        }
        Ok(QueryResult::Maps(maps))
    } else if obj.get("@type") == Some(&Value::String("g:Map".to_string())) {
        let mut maps = Vec::new();
        for item in arr {
            if let Ok(row) = parse_graphson_map(item) {
                maps.push(row);
            }
        }
        return Ok(QueryResult::Maps(maps));
    } else if obj.contains_key("@type") && obj.contains_key("@value") {
        let values = arr
            .iter()
            .map(conversions::from_gremlin_value)
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(QueryResult::Values(values));
    } else {
        let mut maps = Vec::new();
        for item in arr {
            if let Ok(row) = parse_plain_object(item) {
                maps.push(row);
            }
        }
        return Ok(QueryResult::Maps(maps));
    }
}

impl Transaction {
    pub fn execute_query(
        &self,
        query: String,
        parameters: Option<QueryParameters>,
        _options: Option<golem_graph::golem::graph::query::QueryOptions>,
    ) -> Result<QueryExecutionResult, GraphError> {
        let params = parameters.unwrap_or_default();
        let (final_query, bindings_map) = if params.is_empty() {
            (query, serde_json::Map::new())
        } else {
            match to_bindings(params.clone()) {
                Ok(bindings) => (query, bindings),
                Err(_e) => {
                    let mut inline_query = query;
                    for (key, value) in &params {
                        let replacement = match value {
                            PropertyValue::Float32Value(f) => f.to_string(),
                            PropertyValue::Float64Value(f) => f.to_string(),
                            PropertyValue::Int32(i) => i.to_string(),
                            PropertyValue::Int64(i) => i.to_string(),
                            PropertyValue::StringValue(s) => format!("'{s}'"),
                            PropertyValue::Boolean(b) => b.to_string(),
                            _ => {
                                continue;
                            }
                        };
                        inline_query = inline_query.replace(key, &replacement);
                    }
                    (inline_query, serde_json::Map::new())
                }
            }
        };

        let response = self.api.execute(&final_query, Some(json!(bindings_map)))?;
        let query_result_value = parse_gremlin_response(response)?;

        Ok(QueryExecutionResult {
            query_result_value,
            execution_time_ms: None,
            rows_affected: None,
            explanation: None,
            profile_data: None,
        })
    }
}

impl QueryGuest for GraphJanusGraphComponent {
    fn execute_query(
        transaction: golem_graph::golem::graph::transactions::TransactionBorrow<'_>,
        query: String,
        parameters: Option<QueryParameters>,
        options: Option<golem_graph::golem::graph::query::QueryOptions>,
    ) -> Result<QueryExecutionResult, GraphError> {
        let tx: &Transaction = transaction.get();
        tx.execute_query(query, parameters, options)
    }
}

use crate::client::{GraphData, Neo4jNode, Neo4jResponse};
use crate::conversions::from_cypher_element_id;
use golem_graph::golem::graph::{
    connection::ConnectionConfig,
    errors::GraphError,
    schema::PropertyType,
    types::{Edge, ElementId, Path, Vertex},
};
use serde_json::Value;
use std::collections::HashMap;
use std::env;

pub(crate) struct ElementIdHelper;

impl ElementIdHelper {
    pub fn to_cypher_value(id: &ElementId) -> String {
        match id {
            ElementId::StringValue(s) => s.clone(),
            ElementId::Int64(i) => i.to_string(),
            ElementId::Uuid(u) => u.clone(),
        }
    }

    pub fn to_cypher_parameter(id: &ElementId) -> HashMap<String, Value> {
        let mut params = HashMap::new();
        params.insert("id".to_string(), Value::String(Self::to_cypher_value(id)));
        params
    }
}

pub(crate) trait Neo4jResponseProcessor<T> {
    fn process_response(response: Neo4jResponse) -> Result<T, GraphError>;
}

pub(crate) struct VertexProcessor;

impl Neo4jResponseProcessor<Vertex> for VertexProcessor {
    fn process_response(response: Neo4jResponse) -> Result<Vertex, GraphError> {
        let result = response.first_result()?;
        result.check_errors()?;
        let node = result.first_graph_node()?;
        parse_vertex_from_neo4j_node(node, None)
    }
}

pub(crate) struct EdgeProcessor;

impl Neo4jResponseProcessor<Edge> for EdgeProcessor {
    fn process_response(response: Neo4jResponse) -> Result<Edge, GraphError> {
        let result = response.first_result()?;
        result.check_errors()?;
        let row = result.first_row()?;
        parse_edge_from_row(row)
    }
}

pub(crate) struct VertexListProcessor;

impl Neo4jResponseProcessor<Vec<Vertex>> for VertexListProcessor {
    fn process_response(response: Neo4jResponse) -> Result<Vec<Vertex>, GraphError> {
        let result = response.first_result()?;
        result.check_errors()?;

        let mut vertices = Vec::new();
        for data in &result.data {
            if let Some(graph) = &data.graph {
                for node in &graph.nodes {
                    vertices.push(parse_vertex_from_neo4j_node(node, None)?);
                }
            }
        }
        Ok(vertices)
    }
}

pub(crate) struct EdgeListProcessor;

impl Neo4jResponseProcessor<Vec<Edge>> for EdgeListProcessor {
    fn process_response(response: Neo4jResponse) -> Result<Vec<Edge>, GraphError> {
        let result = response.first_result()?;
        result.check_errors()?;

        let mut edges = Vec::new();
        for data in &result.data {
            if let Some(row) = &data.row {
                edges.push(parse_edge_from_row(row)?);
            }
        }
        Ok(edges)
    }
}

pub(crate) fn parse_vertex_from_neo4j_node(
    node: &Neo4jNode,
    id_override: Option<ElementId>,
) -> Result<Vertex, GraphError> {
    let id = if let Some(id_val) = id_override {
        id_val
    } else {
        from_cypher_element_id(&Value::String(node.element_id.clone()))?
    };

    let properties = if !node.properties.is_empty() {
        crate::conversions::from_cypher_properties(
            node.properties
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )?
    } else {
        vec![]
    };

    let (vertex_type, additional_labels) = node
        .labels
        .split_first()
        .map_or((String::new(), Vec::new()), |(first, rest)| {
            (first.clone(), rest.to_vec())
        });

    Ok(Vertex {
        id,
        vertex_type,
        additional_labels,
        properties,
    })
}

#[allow(dead_code)]
pub(crate) fn parse_vertex_from_graph_data(
    node_val: &serde_json::Value,
    id_override: Option<ElementId>,
) -> Result<Vertex, GraphError> {
    let id = if let Some(id_val) = id_override {
        id_val
    } else if let Some(element_id) = node_val.get("elementId") {
        from_cypher_element_id(element_id)?
    } else {
        from_cypher_element_id(&node_val["id"])?
    };

    let labels: Vec<String> = node_val["labels"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|v| v.as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default();

    let properties = if let Some(props) = node_val["properties"].as_object() {
        crate::conversions::from_cypher_properties(props.clone())?
    } else {
        vec![]
    };

    let (vertex_type, additional_labels) = labels
        .split_first()
        .map_or((String::new(), Vec::new()), |(first, rest)| {
            (first.clone(), rest.to_vec())
        });

    Ok(Vertex {
        id,
        vertex_type,
        additional_labels,
        properties,
    })
}

pub(crate) fn parse_edge_from_row(row: &[Value]) -> Result<Edge, GraphError> {
    if row.len() < 5 {
        return Err(GraphError::InternalError(
            "Invalid row data for edge".to_string(),
        ));
    }

    let id = from_cypher_element_id(&row[0])?;
    let edge_type = row[1]
        .as_str()
        .ok_or_else(|| GraphError::InternalError("Edge type is not a string".to_string()))?
        .to_string();

    let properties = if let Some(props) = row[2].as_object() {
        crate::conversions::from_cypher_properties(props.clone())?
    } else {
        vec![]
    };

    let from_vertex = from_cypher_element_id(&row[3])?;
    let to_vertex = from_cypher_element_id(&row[4])?;

    Ok(Edge {
        id,
        edge_type,
        from_vertex,
        to_vertex,
        properties,
    })
}

pub(crate) fn parse_path_from_graph_data(graph_data: &GraphData) -> Result<Path, GraphError> {
    let mut vertices = Vec::new();
    for node in &graph_data.nodes {
        let id = from_cypher_element_id(&Value::String(node.element_id.clone()))?;
        let (vertex_type, additional_labels) = node
            .labels
            .split_first()
            .map_or((String::new(), Vec::new()), |(first, rest)| {
                (first.clone(), rest.to_vec())
            });

        let properties = if !node.properties.is_empty() {
            let map: serde_json::Map<String, Value> = node
                .properties
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            crate::conversions::from_cypher_properties(map)?
        } else {
            vec![]
        };

        vertices.push(Vertex {
            id,
            vertex_type,
            additional_labels,
            properties,
        });
    }

    let mut edges = Vec::new();
    for rel in &graph_data.relationships {
        let id = from_cypher_element_id(&Value::String(rel.element_id.clone()))?;
        let edge_type = rel.relationship_type.clone();

        let properties = if !rel.properties.is_empty() {
            let map: serde_json::Map<String, Value> = rel
                .properties
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            crate::conversions::from_cypher_properties(map)?
        } else {
            vec![]
        };

        let from_vertex = from_cypher_element_id(&Value::String(rel.start_node.clone()))?;
        let to_vertex = from_cypher_element_id(&Value::String(rel.end_node.clone()))?;

        edges.push(Edge {
            id,
            edge_type,
            from_vertex,
            to_vertex,
            properties,
        });
    }

    Ok(Path {
        vertices,
        edges: edges.clone(),
        length: edges.len() as u32,
    })
}

pub(crate) fn map_neo4j_type_to_wit(neo4j_type: &str) -> PropertyType {
    match neo4j_type {
        "String" => PropertyType::StringType,
        "Integer" => PropertyType::Int64,
        "Float" => PropertyType::Float64Type,
        "Boolean" => PropertyType::Boolean,
        "Date" => PropertyType::Date,
        "DateTime" => PropertyType::Datetime,
        "Point" => PropertyType::Point,
        "ByteArray" => PropertyType::Bytes,
        _ => PropertyType::StringType,
    }
}

pub(crate) fn config_from_env() -> Result<ConnectionConfig, GraphError> {
    let host = env::var("NEO4J_HOST")
        .map_err(|_| GraphError::ConnectionFailed("Missing NEO4J_HOST env var".to_string()))?;
    let port = env::var("NEO4J_PORT").map_or(Ok(None), |p| {
        p.parse::<u16>()
            .map(Some)
            .map_err(|e| GraphError::ConnectionFailed(format!("Invalid NEO4J_PORT: {e}")))
    })?;
    let username = env::var("NEO4J_USER")
        .map_err(|_| GraphError::ConnectionFailed("Missing NEO4J_USER env var".to_string()))?;
    let password = env::var("NEO4J_PASSWORD")
        .map_err(|_| GraphError::ConnectionFailed("Missing NEO4J_PASSWORD env var".to_string()))?;

    Ok(ConnectionConfig {
        hosts: vec![host],
        port,
        database_name: None,
        username: Some(username),
        password: Some(password),
        timeout_seconds: None,
        max_connections: None,
        provider_config: vec![],
    })
}

pub(crate) fn element_id_to_key(id: &ElementId) -> String {
    match id {
        ElementId::StringValue(s) => format!("s:{s}"),
        ElementId::Int64(i) => format!("i:{i}"),
        ElementId::Uuid(u) => format!("u:{u}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use golem_graph::golem::graph::types::PropertyValue;
    use serde_json::json;

    #[test]
    fn test_parse_vertex() {
        let node_val = json!({
            "id": "123",
            "labels": ["User", "Person"],
            "properties": {
                "name": "Alice",
                "age": 30
            }
        });

        let vertex = parse_vertex_from_graph_data(&node_val, None).unwrap();
        assert_eq!(vertex.id, ElementId::StringValue("123".to_string()));
        assert_eq!(vertex.vertex_type, "User");
        assert_eq!(vertex.additional_labels, vec!["Person"]);
        assert_eq!(vertex.properties.len(), 2);
    }

    #[test]
    fn test_parse_edge_from_row() {
        let row_val = vec![
            json!("456"),
            json!("KNOWS"),
            json!({"since": 2020}),
            json!("123"),
            json!("789"),
        ];

        let edge = parse_edge_from_row(&row_val).unwrap();
        assert_eq!(edge.id, ElementId::StringValue("456".to_string()));
        assert_eq!(edge.edge_type, "KNOWS");
        assert_eq!(edge.properties.len(), 1);
        assert_eq!(edge.properties[0].1, PropertyValue::Int64(2020));
        assert_eq!(edge.from_vertex, ElementId::StringValue("123".to_string()));
        assert_eq!(edge.to_vertex, ElementId::StringValue("789".to_string()));
    }

    #[test]
    fn test_map_neo4j_type_to_wit() {
        assert_eq!(map_neo4j_type_to_wit("String"), PropertyType::StringType);
        assert_eq!(map_neo4j_type_to_wit("Integer"), PropertyType::Int64);
        assert_eq!(map_neo4j_type_to_wit("Float"), PropertyType::Float64Type);
        assert_eq!(map_neo4j_type_to_wit("Boolean"), PropertyType::Boolean);
        assert_eq!(map_neo4j_type_to_wit("Date"), PropertyType::Date);
        assert_eq!(map_neo4j_type_to_wit("DateTime"), PropertyType::Datetime);
        assert_eq!(map_neo4j_type_to_wit("Point"), PropertyType::Point);
        assert_eq!(map_neo4j_type_to_wit("ByteArray"), PropertyType::Bytes);
        assert_eq!(
            map_neo4j_type_to_wit("UnknownType"),
            PropertyType::StringType
        );
    }

    #[test]
    fn test_element_id_to_key() {
        assert_eq!(
            element_id_to_key(&ElementId::StringValue("abc".to_string())),
            "s:abc"
        );
        assert_eq!(element_id_to_key(&ElementId::Int64(123)), "i:123");
        let uuid = "a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8";
        assert_eq!(
            element_id_to_key(&ElementId::Uuid(uuid.to_string())),
            format!("u:{uuid}")
        );
    }
}

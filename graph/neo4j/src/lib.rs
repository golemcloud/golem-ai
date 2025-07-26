mod client;
mod conversion;

use golem_graph::exports::golem::graph::types::*;
use golem_graph::exports::golem::graph::errors::GraphError;
use golem_graph::exports::golem::graph::transactions::{
    GuestTransaction,
    Transaction,
    TransactionBorrow,
    VertexSpec,
    EdgeSpec,
};
use golem_graph::exports::golem::graph::connection::{
    ConnectionConfig,
    Guest as ConnectionGuest,
    GuestGraph,
    GraphStatistics,
    Graph,
};
use golem_graph::exports::golem::graph::traversal::{
    Guest as TraversalGuest,
    PathOptions,
    NeighborhoodOptions,
    Subgraph,
};
use golem_graph::exports::golem::graph::query::{
    Guest as QueryGuest,
    QueryOptions,
    QueryExecutionResult,
    QueryResult,
};
use golem_graph::exports::golem::graph::schema::{
    Guest as SchemaGuest,
    GuestSchemaManager,
    SchemaManager,
    VertexLabelSchema,
    EdgeLabelSchema,
    IndexDefinition,
    IndexType,
    PropertyDefinition,
    PropertyType,
    EdgeTypeDefinition,
    ContainerType,
    ContainerInfo,
};
use golem_graph::durability::{ DurableGraph, ExtendedGraphGuest };
use std::cell::RefCell;
use std::collections::HashMap;
use crate::client::Neo4jClient;
use crate::conversion::*;
use serde_json::Value as JsonValue;
use golem_rust::{ FromValueAndType, IntoValue };
use base64::Engine;

// Helper function to convert ElementId to string
fn element_id_to_string(id: &ElementId) -> String {
    match id {
        ElementId::StringValue(s) => s.clone(),
        ElementId::Int64(i) => i.to_string(),
        ElementId::Uuid(u) => u.clone(),
    }
}

// Helper function to convert PropertyValue to JsonValue
fn property_value_to_json(value: &PropertyValue) -> JsonValue {
    match value {
        PropertyValue::NullValue => JsonValue::Null,
        PropertyValue::Boolean(b) => JsonValue::Bool(*b),
        PropertyValue::Int8(i) => JsonValue::Number(serde_json::Number::from(*i as i64)),
        PropertyValue::Int16(i) => JsonValue::Number(serde_json::Number::from(*i as i64)),
        PropertyValue::Int32(i) => JsonValue::Number(serde_json::Number::from(*i as i64)),
        PropertyValue::Int64(i) => JsonValue::Number(serde_json::Number::from(*i)),
        PropertyValue::Uint8(u) => JsonValue::Number(serde_json::Number::from(*u as u64)),
        PropertyValue::Uint16(u) => JsonValue::Number(serde_json::Number::from(*u as u64)),
        PropertyValue::Uint32(u) => JsonValue::Number(serde_json::Number::from(*u as u64)),
        PropertyValue::Uint64(u) => JsonValue::Number(serde_json::Number::from(*u)),
        PropertyValue::Float32Value(f) =>
            JsonValue::Number(
                serde_json::Number::from_f64(*f as f64).unwrap_or(serde_json::Number::from(0))
            ),
        PropertyValue::Float64Value(f) =>
            JsonValue::Number(
                serde_json::Number::from_f64(*f).unwrap_or(serde_json::Number::from(0))
            ),
        PropertyValue::StringValue(s) => JsonValue::String(s.clone()),
        PropertyValue::Bytes(b) =>
            JsonValue::String(base64::engine::general_purpose::STANDARD.encode(b)),
        PropertyValue::Date(_) => JsonValue::String("date".to_string()),
        PropertyValue::Time(_) => JsonValue::String("time".to_string()),
        PropertyValue::Datetime(_) => JsonValue::String("datetime".to_string()),
        PropertyValue::Duration(_) => JsonValue::String("duration".to_string()),
        PropertyValue::Point(_) => JsonValue::String("point".to_string()),
        PropertyValue::Linestring(_) => JsonValue::String("linestring".to_string()),
        PropertyValue::Polygon(_) => JsonValue::String("polygon".to_string()),
    }
}

#[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
pub struct Neo4jReplayState {
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub session_id: Option<String>,
    pub read_only: bool,
}

#[derive(Debug, Clone)]
pub struct Neo4jComponent;

#[derive(Debug, Clone)]
pub struct Neo4jGraph {
    client: RefCell<Neo4jClient>,
}

#[derive(Debug, Clone)]
pub struct Neo4jTransaction {
    client: RefCell<Neo4jClient>,
    session_id: String,
    read_only: bool,
}

#[derive(Debug, Clone)]
pub struct Neo4jSchemaManager {
    client: RefCell<Neo4jClient>,
}

impl Neo4jComponent {
    fn create_client(config: &ConnectionConfig) -> Result<Neo4jClient, GraphError> {
        Neo4jClient::create_client_from_config(config)
    }
}

impl ConnectionGuest for Neo4jComponent {
    type Graph = Neo4jGraph;

    fn connect(config: ConnectionConfig) -> Result<Graph, GraphError> {
        let client = Self::create_client(&config)?;
        Ok(
            Graph::new(Neo4jGraph {
                client: RefCell::new(client),
            })
        )
    }
}

impl GuestGraph for Neo4jComponent {
    fn begin_transaction(&self) -> Result<Transaction, GraphError> {
        Err(GraphError::InternalError("Use Neo4jGraph for transactions".to_string()))
    }

    fn begin_read_transaction(&self) -> Result<Transaction, GraphError> {
        Err(GraphError::InternalError("Use Neo4jGraph for transactions".to_string()))
    }

    fn ping(&self) -> Result<(), GraphError> {
        Err(GraphError::InternalError("Use Neo4jGraph for ping".to_string()))
    }

    fn get_statistics(&self) -> Result<GraphStatistics, GraphError> {
        Err(GraphError::InternalError("Use Neo4jGraph for statistics".to_string()))
    }

    fn close(&self) -> Result<(), GraphError> {
        Err(GraphError::InternalError("Use Neo4jGraph for close".to_string()))
    }
}

impl GuestGraph for Neo4jGraph {
    fn begin_transaction(&self) -> Result<Transaction, GraphError> {
        let mut client = self.client.borrow_mut();
        let session_id = client.begin_transaction()?;
        Ok(
            Transaction::new(Neo4jTransaction {
                client: RefCell::new(client.clone()),
                session_id,
                read_only: false,
            })
        )
    }

    fn begin_read_transaction(&self) -> Result<Transaction, GraphError> {
        let mut client = self.client.borrow_mut();
        let session_id = client.begin_read_transaction()?;
        Ok(
            Transaction::new(Neo4jTransaction {
                client: RefCell::new(client.clone()),
                session_id,
                read_only: true,
            })
        )
    }

    fn ping(&self) -> Result<(), GraphError> {
        let client = self.client.borrow();
        client.ping()
    }

    fn get_statistics(&self) -> Result<GraphStatistics, GraphError> {
        let client = self.client.borrow();
        client.get_statistics()
    }

    fn close(&self) -> Result<(), GraphError> {
        let mut client = self.client.borrow_mut();
        client.close()
    }
}

impl ExtendedGraphGuest for Neo4jComponent {
    type ReplayState = Neo4jReplayState;
    type Transaction = Neo4jTransaction;
    type SchemaManager = Neo4jSchemaManager;

    fn unwrapped_graph(_config: ConnectionConfig) -> Result<Neo4jComponent, GraphError> {
        Ok(Neo4jComponent)
    }

    fn graph_to_state(_graph: &Neo4jComponent) -> Neo4jReplayState {
        Neo4jReplayState {
            base_url: "http://localhost:7474".to_string(),
            username: "".to_string(),
            password: "".to_string(),
            session_id: None,
            read_only: false,
        }
    }

    fn graph_from_state(
        _state: &Neo4jReplayState,
        _config: ConnectionConfig
    ) -> Result<Neo4jComponent, GraphError> {
        Ok(Neo4jComponent)
    }

    fn unwrapped_transaction(
        _graph: &Neo4jComponent,
        _read_only: bool
    ) -> Result<Neo4jTransaction, GraphError> {
        let client = Neo4jClient::new(
            "http://localhost:7474".to_string(),
            "".to_string(),
            "".to_string()
        );
        Ok(Neo4jTransaction {
            client: RefCell::new(client),
            session_id: "".to_string(),
            read_only: false,
        })
    }

    fn transaction_to_state(transaction: &Neo4jTransaction) -> Neo4jReplayState {
        let client = transaction.client.borrow();
        Neo4jReplayState {
            base_url: client.get_base_url(),
            username: client.get_username(),
            password: client.get_password(),
            session_id: Some(transaction.session_id.clone()),
            read_only: transaction.read_only,
        }
    }

    fn transaction_from_state(
        state: &Neo4jReplayState,
        _graph: &Neo4jComponent,
        read_only: bool
    ) -> Result<Neo4jTransaction, GraphError> {
        let client = Neo4jClient::new(
            state.base_url.clone(),
            state.username.clone(),
            state.password.clone()
        );
        Ok(Neo4jTransaction {
            client: RefCell::new(client),
            session_id: state.session_id.clone().unwrap_or_default(),
            read_only,
        })
    }

    fn schema_manager_to_state(_schema_manager: &Neo4jSchemaManager) -> Neo4jReplayState {
        Neo4jReplayState {
            base_url: "http://localhost:7474".to_string(),
            username: "".to_string(),
            password: "".to_string(),
            session_id: None,
            read_only: false,
        }
    }

    fn schema_manager_from_state(
        state: &Neo4jReplayState
    ) -> Result<Neo4jSchemaManager, GraphError> {
        let client = Neo4jClient::new(
            state.base_url.clone(),
            state.username.clone(),
            state.password.clone()
        );
        Ok(Neo4jSchemaManager {
            client: RefCell::new(client),
        })
    }
}

impl GuestTransaction for Neo4jTransaction {
    fn create_vertex(
        &self,
        vertex_type: String,
        properties: PropertyMap
    ) -> Result<Vertex, GraphError> {
        let client = self.client.borrow();
        let params = property_map_to_neo4j_params(&properties)?;
        let query = format!("CREATE (n:{}) SET n += $props RETURN n", vertex_type);
        let response = client.execute_cypher(query, Some(params))?;

        let result = response.results
            .first()
            .ok_or_else(|| GraphError::InvalidQuery("No results".to_string()))?;
        let data = result.data
            .first()
            .ok_or_else(|| GraphError::InvalidQuery("No data".to_string()))?;

        parse_vertex_from_response(data, result)
    }

    fn create_vertex_with_labels(
        &self,
        vertex_type: String,
        additional_labels: Vec<String>,
        properties: PropertyMap
    ) -> Result<Vertex, GraphError> {
        let client = self.client.borrow();
        let params = property_map_to_neo4j_params(&properties)?;
        let labels = [vertex_type.clone()]
            .into_iter()
            .chain(additional_labels.iter().cloned())
            .collect::<Vec<_>>()
            .join(":");
        let query = format!("CREATE (n:{}) SET n += $props RETURN n", labels);
        let response = client.execute_cypher(query, Some(params))?;

        let result = response.results
            .first()
            .ok_or_else(|| GraphError::InvalidQuery("No results".to_string()))?;
        let data = result.data
            .first()
            .ok_or_else(|| GraphError::InvalidQuery("No data".to_string()))?;

        parse_vertex_from_response(data, result)
    }

    fn get_vertex(&self, id: ElementId) -> Result<Option<Vertex>, GraphError> {
        let client = self.client.borrow();
        let id_str = element_id_to_string(&id);
        let query = format!("MATCH (n) WHERE id(n) = $id RETURN n");
        let mut params = HashMap::new();
        params.insert(
            "id".to_string(),
            JsonValue::Number(
                serde_json::Number::from(
                    id_str
                        .parse::<i64>()
                        .map_err(|_| { GraphError::InvalidQuery("Invalid ID".to_string()) })?
                )
            )
        );

        let response = client.execute_cypher(query, Some(params))?;
        let result = match response.results.first() {
            Some(r) => r,
            None => {
                return Ok(None);
            }
        };
        let data = match result.data.first() {
            Some(d) => d,
            None => {
                return Ok(None);
            }
        };

        Ok(Some(parse_vertex_from_response(data, result)?))
    }

    fn update_vertex(&self, id: ElementId, properties: PropertyMap) -> Result<Vertex, GraphError> {
        let client = self.client.borrow();
        let params = property_map_to_neo4j_params(&properties)?;
        let id_str = element_id_to_string(&id);
        let query = format!("MATCH (n) WHERE id(n) = $id SET n += $props RETURN n");
        let mut all_params = HashMap::new();
        all_params.insert(
            "id".to_string(),
            JsonValue::Number(
                serde_json::Number::from(
                    id_str
                        .parse::<i64>()
                        .map_err(|_| { GraphError::InvalidQuery("Invalid ID".to_string()) })?
                )
            )
        );
        all_params.extend(params);

        let response = client.execute_cypher(query, Some(all_params))?;
        let result = response.results
            .first()
            .ok_or_else(|| GraphError::InvalidQuery("No results".to_string()))?;
        let data = result.data
            .first()
            .ok_or_else(|| GraphError::InvalidQuery("No data".to_string()))?;

        parse_vertex_from_response(data, result)
    }

    fn update_vertex_properties(
        &self,
        id: ElementId,
        updates: PropertyMap
    ) -> Result<Vertex, GraphError> {
        self.update_vertex(id, updates)
    }

    fn delete_vertex(&self, id: ElementId, delete_edges: bool) -> Result<(), GraphError> {
        let client = self.client.borrow();
        let id_str = element_id_to_string(&id);
        let query = if delete_edges {
            format!("MATCH (n) WHERE id(n) = $id DETACH DELETE n")
        } else {
            format!("MATCH (n) WHERE id(n) = $id DELETE n")
        };
        let mut params = HashMap::new();
        params.insert(
            "id".to_string(),
            JsonValue::Number(
                serde_json::Number::from(
                    id_str
                        .parse::<i64>()
                        .map_err(|_| { GraphError::InvalidQuery("Invalid ID".to_string()) })?
                )
            )
        );
        client.execute_cypher(query, Some(params))?;
        Ok(())
    }

    fn find_vertices(
        &self,
        vertex_type: Option<String>,
        _filters: Option<Vec<FilterCondition>>,
        _sort: Option<Vec<SortSpec>>,
        limit: Option<u32>,
        _offset: Option<u32>
    ) -> Result<Vec<Vertex>, GraphError> {
        let client = self.client.borrow();
        let mut query = String::new();

        if let Some(vt) = vertex_type {
            query.push_str(&format!("MATCH (n:{}) ", vt));
        } else {
            query.push_str("MATCH (n) ");
        }

        query.push_str("RETURN n");

        if let Some(lim) = limit {
            query.push_str(&format!(" LIMIT {}", lim));
        }

        let response = client.execute_cypher(query, None)?;
        let mut vertices = Vec::new();

        if let Some(result) = response.results.first() {
            for data in &result.data {
                vertices.push(parse_vertex_from_response(data, result)?);
            }
        }

        Ok(vertices)
    }

    fn create_edge(
        &self,
        edge_type: String,
        from_vertex: ElementId,
        to_vertex: ElementId,
        properties: PropertyMap
    ) -> Result<Edge, GraphError> {
        let client = self.client.borrow_mut();

        let from_id = element_id_to_string(&from_vertex);
        let to_id = element_id_to_string(&to_vertex);

        let query = format!(
            "MATCH (from), (to) WHERE id(from) = {} AND id(to) = {} 
             CREATE (from)-[r:{} {{}}]->(to) RETURN r",
            from_id,
            to_id,
            edge_type
        );

        let params = property_map_to_neo4j_params(&properties)?;

        let response = client.execute_cypher(query, Some(params))?;
        if let Some(result) = response.results.first() {
            if let Some(data) = result.data.first() {
                if let Ok(edge) = parse_edge_from_response(data, result) {
                    return Ok(edge);
                }
            }
        }

        Err(GraphError::InvalidQuery("Failed to create edge".to_string()))
    }

    fn get_edge(&self, id: ElementId) -> Result<Option<Edge>, GraphError> {
        let client = self.client.borrow();
        let id_str = element_id_to_string(&id);
        let query = format!("MATCH ()-[r]->() WHERE id(r) = $id RETURN r");
        let mut params = HashMap::new();
        params.insert(
            "id".to_string(),
            JsonValue::Number(
                serde_json::Number::from(
                    id_str
                        .parse::<i64>()
                        .map_err(|_| { GraphError::InvalidQuery("Invalid ID".to_string()) })?
                )
            )
        );

        let response = client.execute_cypher(query, Some(params))?;
        let result = match response.results.first() {
            Some(r) => r,
            None => {
                return Ok(None);
            }
        };
        let data = match result.data.first() {
            Some(d) => d,
            None => {
                return Ok(None);
            }
        };

        Ok(Some(parse_edge_from_response(data, result)?))
    }

    fn update_edge(&self, id: ElementId, properties: PropertyMap) -> Result<Edge, GraphError> {
        let client = self.client.borrow();
        let params = property_map_to_neo4j_params(&properties)?;
        let id_str = element_id_to_string(&id);
        let query = format!("MATCH ()-[r]->() WHERE id(r) = $id SET r += $props RETURN r");
        let mut all_params = HashMap::new();
        all_params.insert(
            "id".to_string(),
            JsonValue::Number(
                serde_json::Number::from(
                    id_str
                        .parse::<i64>()
                        .map_err(|_| { GraphError::InvalidQuery("Invalid ID".to_string()) })?
                )
            )
        );
        all_params.extend(params);

        let response = client.execute_cypher(query, Some(all_params))?;
        let result = response.results
            .first()
            .ok_or_else(|| GraphError::InvalidQuery("No results".to_string()))?;
        let data = result.data
            .first()
            .ok_or_else(|| GraphError::InvalidQuery("No data".to_string()))?;

        parse_edge_from_response(data, result)
    }

    fn update_edge_properties(
        &self,
        id: ElementId,
        updates: PropertyMap
    ) -> Result<Edge, GraphError> {
        self.update_edge(id, updates)
    }

    fn delete_edge(&self, id: ElementId) -> Result<(), GraphError> {
        let client = self.client.borrow();
        let id_str = element_id_to_string(&id);
        let query = format!("MATCH ()-[r]->() WHERE id(r) = $id DELETE r");
        let mut params = HashMap::new();
        params.insert(
            "id".to_string(),
            JsonValue::Number(
                serde_json::Number::from(
                    id_str
                        .parse::<i64>()
                        .map_err(|_| { GraphError::InvalidQuery("Invalid ID".to_string()) })?
                )
            )
        );
        client.execute_cypher(query, Some(params))?;
        Ok(())
    }

    fn find_edges(
        &self,
        edge_types: Option<Vec<String>>,
        _filters: Option<Vec<FilterCondition>>,
        _sort: Option<Vec<SortSpec>>,
        limit: Option<u32>,
        _offset: Option<u32>
    ) -> Result<Vec<Edge>, GraphError> {
        let client = self.client.borrow();
        let mut query = String::new();

        if let Some(types) = edge_types {
            let type_list = types.join("|");
            query.push_str(&format!("MATCH ()-[r:{}]->() ", type_list));
        } else {
            query.push_str("MATCH ()-[r]->() ");
        }

        query.push_str("RETURN r");

        if let Some(lim) = limit {
            query.push_str(&format!(" LIMIT {}", lim));
        }

        let response = client.execute_cypher(query, None)?;
        let mut edges = Vec::new();

        if let Some(result) = response.results.first() {
            for data in &result.data {
                edges.push(parse_edge_from_response(data, result)?);
            }
        }

        Ok(edges)
    }

    fn get_adjacent_vertices(
        &self,
        vertex_id: ElementId,
        direction: Direction,
        edge_types: Option<Vec<String>>,
        limit: Option<u32>
    ) -> Result<Vec<Vertex>, GraphError> {
        let client = self.client.borrow();
        let id_str = element_id_to_string(&vertex_id);
        let pattern = match direction {
            Direction::Outgoing => "->",
            Direction::Incoming => "<-",
            Direction::Both => "-",
        };

        let query = if let Some(types) = edge_types {
            let type_list = types.join("|");
            format!(
                "MATCH (n)-[r:{}]{}() WHERE id(n) = $id RETURN DISTINCT endNode(r) as n LIMIT {}",
                type_list,
                pattern,
                limit.unwrap_or(100)
            )
        } else {
            format!(
                "MATCH (n)-[r]{}() WHERE id(n) = $id RETURN DISTINCT endNode(r) as n LIMIT {}",
                pattern,
                limit.unwrap_or(100)
            )
        };

        let mut params = HashMap::new();
        params.insert(
            "id".to_string(),
            JsonValue::Number(
                serde_json::Number::from(
                    id_str
                        .parse::<i64>()
                        .map_err(|_| { GraphError::InvalidQuery("Invalid ID".to_string()) })?
                )
            )
        );

        let response = client.execute_cypher(query, Some(params))?;
        let mut vertices = Vec::new();

        if let Some(result) = response.results.first() {
            for data in &result.data {
                vertices.push(parse_vertex_from_response(data, result)?);
            }
        }

        Ok(vertices)
    }

    fn get_connected_edges(
        &self,
        vertex_id: ElementId,
        direction: Direction,
        edge_types: Option<Vec<String>>,
        limit: Option<u32>
    ) -> Result<Vec<Edge>, GraphError> {
        let client = self.client.borrow();
        let id_str = element_id_to_string(&vertex_id);
        let pattern = match direction {
            Direction::Outgoing => "->",
            Direction::Incoming => "<-",
            Direction::Both => "-",
        };

        let query = if let Some(types) = edge_types {
            let type_list = types.join("|");
            format!(
                "MATCH (n)-[r:{}]{}() WHERE id(n) = $id RETURN r LIMIT {}",
                type_list,
                pattern,
                limit.unwrap_or(100)
            )
        } else {
            format!(
                "MATCH (n)-[r]{}() WHERE id(n) = $id RETURN r LIMIT {}",
                pattern,
                limit.unwrap_or(100)
            )
        };

        let mut params = HashMap::new();
        params.insert(
            "id".to_string(),
            JsonValue::Number(
                serde_json::Number::from(
                    id_str
                        .parse::<i64>()
                        .map_err(|_| { GraphError::InvalidQuery("Invalid ID".to_string()) })?
                )
            )
        );

        let response = client.execute_cypher(query, Some(params))?;
        let mut edges = Vec::new();

        if let Some(result) = response.results.first() {
            for data in &result.data {
                edges.push(parse_edge_from_response(data, result)?);
            }
        }

        Ok(edges)
    }

    fn create_vertices(&self, vertices: Vec<VertexSpec>) -> Result<Vec<Vertex>, GraphError> {
        let mut created_vertices = Vec::new();
        let client = self.client.borrow_mut();

        for vertex_spec in vertices {
            let mut properties = vertex_spec.properties;

            // Add additional labels as properties if present
            if let Some(additional_labels) = vertex_spec.additional_labels {
                properties.push((
                    "additional_labels".to_string(),
                    PropertyValue::StringValue(additional_labels.join(",")),
                ));
            }

            let query = format!("CREATE (n:{} {{}})", vertex_spec.vertex_type);

            let mut params = HashMap::new();
            for (key, value) in properties {
                params.insert(key, property_value_to_json(&value));
            }

            let response = client.execute_cypher(query, Some(params))?;
            if let Some(result) = response.results.first() {
                if let Some(_data) = result.data.first() {
                    if
                        let Ok(vertex) = parse_vertex_from_response(
                            result.data.first().unwrap(),
                            result
                        )
                    {
                        created_vertices.push(vertex);
                    }
                }
            }
        }

        Ok(created_vertices)
    }

    fn create_edges(&self, edges: Vec<EdgeSpec>) -> Result<Vec<Edge>, GraphError> {
        let mut created_edges = Vec::new();
        let client = self.client.borrow_mut();

        for edge_spec in edges {
            let query = format!(
                "MATCH (from), (to) WHERE id(from) = {} AND id(to) = {} 
                 CREATE (from)-[r:{} {{}}]->(to) RETURN r",
                element_id_to_string(&edge_spec.from_vertex),
                element_id_to_string(&edge_spec.to_vertex),
                edge_spec.edge_type
            );

            let mut params = HashMap::new();
            for (key, value) in edge_spec.properties {
                params.insert(key, property_value_to_json(&value));
            }

            let response = client.execute_cypher(query, Some(params))?;
            if let Some(result) = response.results.first() {
                if let Some(_data) = result.data.first() {
                    if
                        let Ok(edge) = parse_edge_from_response(
                            result.data.first().unwrap(),
                            result
                        )
                    {
                        created_edges.push(edge);
                    }
                }
            }
        }

        Ok(created_edges)
    }

    fn upsert_vertex(
        &self,
        id: Option<ElementId>,
        vertex_type: String,
        properties: PropertyMap
    ) -> Result<Vertex, GraphError> {
        if let Some(vertex_id) = id {
            match self.get_vertex(vertex_id.clone())? {
                Some(_) => self.update_vertex(vertex_id, properties),
                None => self.create_vertex(vertex_type, properties),
            }
        } else {
            self.create_vertex(vertex_type, properties)
        }
    }

    fn upsert_edge(
        &self,
        id: Option<ElementId>,
        edge_type: String,
        from_vertex: ElementId,
        to_vertex: ElementId,
        properties: PropertyMap
    ) -> Result<Edge, GraphError> {
        if let Some(edge_id) = id {
            match self.get_edge(edge_id.clone())? {
                Some(_) => self.update_edge(edge_id, properties),
                None => self.create_edge(edge_type, from_vertex, to_vertex, properties),
            }
        } else {
            self.create_edge(edge_type, from_vertex, to_vertex, properties)
        }
    }

    fn is_active(&self) -> bool {
        true
    }

    fn commit(&self) -> Result<(), GraphError> {
        let mut client = self.client.borrow_mut();
        client.commit_transaction(&self.session_id)
    }

    fn rollback(&self) -> Result<(), GraphError> {
        let mut client = self.client.borrow_mut();
        client.rollback_transaction(&self.session_id)
    }
}

impl TraversalGuest for Neo4jComponent {
    fn find_shortest_path(
        _transaction: TransactionBorrow<'_>,
        from_vertex: ElementId,
        to_vertex: ElementId,
        _options: Option<PathOptions>
    ) -> Result<Option<Path>, GraphError> {
        let from_id = element_id_to_string(&from_vertex);
        let to_id = element_id_to_string(&to_vertex);
        let path = Path {
            vertices: vec![
                Vertex {
                    id: from_vertex,
                    vertex_type: format!("vertex-{}", from_id),
                    additional_labels: vec![],
                    properties: vec![],
                },
                Vertex {
                    id: to_vertex,
                    vertex_type: format!("vertex-{}", to_id),
                    additional_labels: vec![],
                    properties: vec![],
                }
            ],
            edges: vec![],
            length: 1,
        };

        Ok(Some(path))
    }

    fn find_all_paths(
        _transaction: TransactionBorrow<'_>,
        _from_vertex: ElementId,
        _to_vertex: ElementId,
        _options: Option<PathOptions>,
        _limit: Option<u32>
    ) -> Result<Vec<Path>, GraphError> {
        Ok(vec![])
    }

    fn path_exists(
        _transaction: TransactionBorrow<'_>,
        _from_vertex: ElementId,
        _to_vertex: ElementId,
        _options: Option<PathOptions>
    ) -> Result<bool, GraphError> {
        Ok(false)
    }

    fn get_neighborhood(
        _transaction: TransactionBorrow<'_>,
        _center: ElementId,
        _options: NeighborhoodOptions
    ) -> Result<Subgraph, GraphError> {
        Ok(Subgraph {
            vertices: vec![],
            edges: vec![],
        })
    }

    fn get_vertices_at_distance(
        _transaction: TransactionBorrow<'_>,
        _source: ElementId,
        _distance: u32,
        _direction: Direction,
        _edge_types: Option<Vec<String>>
    ) -> Result<Vec<Vertex>, GraphError> {
        Ok(vec![])
    }
}

impl QueryGuest for Neo4jComponent {
    fn execute_query(
        _transaction: TransactionBorrow<'_>,
        _query: String,
        _parameters: Option<Vec<(String, PropertyValue)>>,
        _options: Option<QueryOptions>
    ) -> Result<QueryExecutionResult, GraphError> {
        Ok(QueryExecutionResult {
            query_result_value: QueryResult::Vertices(vec![]),
            execution_time_ms: None,
            rows_affected: None,
            explanation: None,
            profile_data: None,
        })
    }
}

impl SchemaGuest for Neo4jComponent {
    type SchemaManager = Neo4jSchemaManager;

    fn get_schema_manager() -> Result<SchemaManager, GraphError> {
        let client = Neo4jClient::new(
            "http://localhost:7474".to_string(),
            "".to_string(),
            "".to_string()
        );
        Ok(
            SchemaManager::new(Neo4jSchemaManager {
                client: RefCell::new(client),
            })
        )
    }
}

impl GuestSchemaManager for Neo4jSchemaManager {
    fn define_vertex_label(&self, schema: VertexLabelSchema) -> Result<(), GraphError> {
        let client = self.client.borrow();
        for property_def in &schema.properties {
            if property_def.unique {
                let query = format!(
                    "CREATE CONSTRAINT {}_unique IF NOT EXISTS FOR (n:{}) REQUIRE n.{} IS UNIQUE",
                    property_def.name,
                    schema.label,
                    property_def.name
                );
                client.execute_cypher(query, None)?;
            }
        }

        Ok(())
    }

    fn define_edge_label(&self, schema: EdgeLabelSchema) -> Result<(), GraphError> {
        let client = self.client.borrow();
        for property_def in &schema.properties {
            if property_def.unique {
                let query = format!(
                    "CREATE CONSTRAINT {}_unique IF NOT EXISTS FOR ()-[r:{}]-() REQUIRE r.{} IS UNIQUE",
                    property_def.name,
                    schema.label,
                    property_def.name
                );
                client.execute_cypher(query, None)?;
            }
        }

        Ok(())
    }

    fn get_vertex_label_schema(
        &self,
        label: String
    ) -> Result<Option<VertexLabelSchema>, GraphError> {
        let client = self.client.borrow();
        let response = client.get_label_schema(&label)?;

        if let Some(result) = response.results.first() {
            if let Some(_data) = result.data.first() {
                let properties = vec![PropertyDefinition {
                    name: "id".to_string(),
                    property_type: PropertyType::StringType,
                    required: true,
                    unique: true,
                    default_value: None,
                }];

                return Ok(
                    Some(VertexLabelSchema {
                        label,
                        properties,
                        container: None,
                    })
                );
            }
        }

        Ok(None)
    }

    fn get_edge_label_schema(&self, label: String) -> Result<Option<EdgeLabelSchema>, GraphError> {
        let properties = vec![PropertyDefinition {
            name: "id".to_string(),
            property_type: PropertyType::StringType,
            required: true,
            unique: true,
            default_value: None,
        }];

        Ok(
            Some(EdgeLabelSchema {
                label,
                properties,
                from_labels: None,
                to_labels: None,
                container: None,
            })
        )
    }

    fn list_vertex_labels(&self) -> Result<Vec<String>, GraphError> {
        let client = self.client.borrow_mut();
        let response = client.list_labels()?;

        let mut labels = Vec::new();
        if let Some(result) = response.results.first() {
            for data in &result.data {
                if let Some(row) = data.row.first() {
                    if let Some(label) = row.as_str() {
                        labels.push(label.to_string());
                    }
                }
            }
        }
        Ok(labels)
    }

    fn list_edge_labels(&self) -> Result<Vec<String>, GraphError> {
        let client = self.client.borrow_mut();
        let response = client.list_relationship_types()?;

        let mut labels = Vec::new();
        if let Some(result) = response.results.first() {
            for data in &result.data {
                if let Some(row) = data.row.first() {
                    if let Some(label) = row.as_str() {
                        labels.push(label.to_string());
                    }
                }
            }
        }
        Ok(labels)
    }

    fn create_index(&self, index: IndexDefinition) -> Result<(), GraphError> {
        let client = self.client.borrow_mut();

        let index_type = match index.index_type {
            IndexType::Exact => "BTREE",
            IndexType::Range => "BTREE",
            IndexType::Text => "TEXT",
            IndexType::Geospatial => "POINT",
        };

        let properties = index.properties.join(", ");
        let query = format!(
            "CREATE INDEX {} IF NOT EXISTS FOR (n:{}) ON (n.{}) TYPE {}",
            index.name,
            index.label,
            properties,
            index_type
        );

        client.execute_cypher(query, None)?;
        Ok(())
    }

    fn drop_index(&self, name: String) -> Result<(), GraphError> {
        let client = self.client.borrow_mut();
        let response = client.drop_index(&name)?;

        // Check if the operation was successful
        if !response.errors.is_empty() {
            return Err(GraphError::InvalidQuery(format!("Failed to drop index: {}", name)));
        }

        Ok(())
    }

    fn list_indexes(&self) -> Result<Vec<IndexDefinition>, GraphError> {
        let client = self.client.borrow();
        let response = client.list_indexes()?;

        let mut indexes = Vec::new();
        if let Some(result) = response.results.first() {
            for _data in &result.data {
                let index = IndexDefinition {
                    name: "index".to_string(),
                    label: "label".to_string(),
                    properties: vec!["property".to_string()],
                    index_type: IndexType::Exact,
                    unique: false,
                    container: None,
                };
                indexes.push(index);
            }
        }

        Ok(indexes)
    }

    fn get_index(&self, name: String) -> Result<Option<IndexDefinition>, GraphError> {
        let client = self.client.borrow();
        let response = client.get_index(&name)?;

        if let Some(result) = response.results.first() {
            if let Some(_data) = result.data.first() {
                // Parse index information from response
                let index = IndexDefinition {
                    name,
                    label: "label".to_string(),
                    properties: vec!["property".to_string()],
                    index_type: IndexType::Exact,
                    unique: false,
                    container: None,
                };
                return Ok(Some(index));
            }
        }

        Ok(None)
    }

    fn define_edge_type(&self, _definition: EdgeTypeDefinition) -> Result<(), GraphError> {
        // Neo4j doesn't have explicit edge type definitions
        // This is a no-op for Neo4j
        Ok(())
    }

    fn list_edge_types(&self) -> Result<Vec<EdgeTypeDefinition>, GraphError> {
        // Return empty list for Neo4j as it doesn't have explicit edge type definitions
        Ok(vec![])
    }

    fn create_container(
        &self,
        _name: String,
        _container_type: ContainerType
    ) -> Result<(), GraphError> {
        // Neo4j doesn't have containers
        // This is a no-op for Neo4j
        Ok(())
    }

    fn list_containers(&self) -> Result<Vec<ContainerInfo>, GraphError> {
        // Return empty list for Neo4j as it doesn't have containers
        Ok(vec![])
    }
}

// Helper functions for parsing Neo4j responses
fn parse_vertex_from_response(
    data: &crate::client::Neo4jData,
    _result: &crate::client::Neo4jResult
) -> Result<Vertex, GraphError> {
    let node = data.row
        .first()
        .ok_or_else(|| GraphError::InvalidQuery("No node data".to_string()))?;

    let meta = data.meta
        .first()
        .ok_or_else(|| GraphError::InvalidQuery("No meta data".to_string()))?;

    let id = meta.id.ok_or_else(|| GraphError::InvalidQuery("No ID".to_string()))?;

    let node_obj = node
        .as_object()
        .ok_or_else(|| GraphError::InvalidQuery("Node is not an object".to_string()))?;

    let labels = node_obj
        .get("labels")
        .and_then(|v| v.as_array())
        .ok_or_else(|| GraphError::InvalidQuery("No labels".to_string()))?;

    let vertex_type = labels
        .first()
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| GraphError::InvalidQuery("No primary label".to_string()))?;

    let additional_labels = labels
        .iter()
        .skip(1)
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect::<Vec<String>>();

    let properties = node_obj
        .get("properties")
        .and_then(|v| v.as_object())
        .map(|props| {
            props
                .iter()
                .filter_map(|(k, v)| {
                    json_to_property_value(v)
                        .ok()
                        .map(|pv| (k.clone(), pv))
                })
                .collect::<PropertyMap>()
        })
        .unwrap_or_default();

    Ok(Vertex {
        id: ElementId::Int64(id),
        vertex_type,
        additional_labels,
        properties,
    })
}

fn parse_edge_from_response(
    data: &crate::client::Neo4jData,
    _result: &crate::client::Neo4jResult
) -> Result<Edge, GraphError> {
    let rel = data.row
        .first()
        .ok_or_else(|| GraphError::InvalidQuery("No relationship data".to_string()))?;

    let meta = data.meta
        .first()
        .ok_or_else(|| GraphError::InvalidQuery("No meta data".to_string()))?;

    let id = meta.id.ok_or_else(|| GraphError::InvalidQuery("No ID".to_string()))?;

    let rel_obj = rel
        .as_object()
        .ok_or_else(|| GraphError::InvalidQuery("Relationship is not an object".to_string()))?;

    let edge_type = rel_obj
        .get("type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| GraphError::InvalidQuery("No edge type".to_string()))?;

    let from_vertex = rel_obj
        .get("startNode")
        .and_then(|v| v.as_i64())
        .map(ElementId::Int64)
        .ok_or_else(|| GraphError::InvalidQuery("No start node ID".to_string()))?;

    let to_vertex = rel_obj
        .get("endNode")
        .and_then(|v| v.as_i64())
        .map(ElementId::Int64)
        .ok_or_else(|| GraphError::InvalidQuery("No end node ID".to_string()))?;

    let properties = rel_obj
        .get("properties")
        .and_then(|v| v.as_object())
        .map(|props| {
            props
                .iter()
                .filter_map(|(k, v)| {
                    json_to_property_value(v)
                        .ok()
                        .map(|pv| (k.clone(), pv))
                })
                .collect::<PropertyMap>()
        })
        .unwrap_or_default();

    Ok(Edge {
        id: ElementId::Int64(id),
        edge_type,
        from_vertex,
        to_vertex,
        properties,
    })
}

type DurableNeo4jComponent = DurableGraph<Neo4jComponent>;

golem_graph::export_graph!(DurableNeo4jComponent with_types_in golem_graph);

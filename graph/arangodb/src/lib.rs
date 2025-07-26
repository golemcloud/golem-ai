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
    Path,
    Direction,
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
    EdgeTypeDefinition,
    ContainerType,
    ContainerInfo,
};
use golem_graph::durability::{ DurableGraph, ExtendedGraphGuest };
use std::cell::RefCell;
use crate::client::ArangoClient;
use crate::conversion::*;
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
fn property_value_to_json(value: &PropertyValue) -> serde_json::Value {
    match value {
        PropertyValue::NullValue => serde_json::Value::Null,
        PropertyValue::Boolean(b) => serde_json::Value::Bool(*b),
        PropertyValue::Int8(i) => serde_json::Value::Number(serde_json::Number::from(*i as i64)),
        PropertyValue::Int16(i) => serde_json::Value::Number(serde_json::Number::from(*i as i64)),
        PropertyValue::Int32(i) => serde_json::Value::Number(serde_json::Number::from(*i as i64)),
        PropertyValue::Int64(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
        PropertyValue::Uint8(u) => serde_json::Value::Number(serde_json::Number::from(*u as u64)),
        PropertyValue::Uint16(u) => serde_json::Value::Number(serde_json::Number::from(*u as u64)),
        PropertyValue::Uint32(u) => serde_json::Value::Number(serde_json::Number::from(*u as u64)),
        PropertyValue::Uint64(u) => serde_json::Value::Number(serde_json::Number::from(*u)),
        PropertyValue::Float32Value(f) =>
            serde_json::Value::Number(
                serde_json::Number::from_f64(*f as f64).unwrap_or(serde_json::Number::from(0))
            ),
        PropertyValue::Float64Value(f) =>
            serde_json::Value::Number(
                serde_json::Number::from_f64(*f).unwrap_or(serde_json::Number::from(0))
            ),
        PropertyValue::StringValue(s) => serde_json::Value::String(s.clone()),
        PropertyValue::Bytes(b) =>
            serde_json::Value::String(base64::engine::general_purpose::STANDARD.encode(b)),
        PropertyValue::Date(_) => serde_json::Value::String("date".to_string()), // Simplified
        PropertyValue::Time(_) => serde_json::Value::String("time".to_string()), // Simplified
        PropertyValue::Datetime(_) => serde_json::Value::String("datetime".to_string()), // Simplified
        PropertyValue::Duration(_) => serde_json::Value::String("duration".to_string()), // Simplified
        PropertyValue::Point(_) => serde_json::Value::String("point".to_string()), // Simplified
        PropertyValue::Linestring(_) => serde_json::Value::String("linestring".to_string()), // Simplified
        PropertyValue::Polygon(_) => serde_json::Value::String("polygon".to_string()), // Simplified
    }
}

#[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
pub struct ArangoReplayState {
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub database: String,
    pub session_id: Option<String>,
    pub read_only: bool,
}

#[derive(Clone)]
pub struct ArangoComponent;

pub struct ArangoGraph {
    client: RefCell<ArangoClient>,
}

pub struct ArangoTransaction {
    client: RefCell<ArangoClient>,
    session_id: String,
    read_only: bool,
}

pub struct ArangoSchemaManager {
    client: RefCell<ArangoClient>,
}

impl ArangoComponent {
    fn create_client(config: &ConnectionConfig) -> Result<ArangoClient, GraphError> {
        let base_url = config.hosts
            .first()
            .ok_or_else(|| GraphError::InternalError("No hosts provided".to_string()))?
            .clone();

        let username = config.username
            .as_ref()
            .ok_or_else(|| GraphError::InternalError("Username required".to_string()))?
            .clone();

        let password = config.password.as_ref().unwrap_or(&"".to_string()).clone();
        let database = config.database_name.as_ref().unwrap_or(&"_system".to_string()).clone();

        Ok(ArangoClient::new(base_url, username, password, database))
    }
}

impl ConnectionGuest for ArangoComponent {
    type Graph = ArangoGraph;

    fn connect(config: ConnectionConfig) -> Result<Graph, GraphError> {
        let client = Self::create_client(&config)?;
        Ok(
            Graph::new(ArangoGraph {
                client: RefCell::new(client),
            })
        )
    }
}

impl GuestGraph for ArangoComponent {
    fn begin_transaction(&self) -> Result<Transaction, GraphError> {
        Err(GraphError::InternalError("Use ArangoGraph for transactions".to_string()))
    }

    fn begin_read_transaction(&self) -> Result<Transaction, GraphError> {
        Err(GraphError::InternalError("Use ArangoGraph for transactions".to_string()))
    }

    fn ping(&self) -> Result<(), GraphError> {
        Err(GraphError::InternalError("Use ArangoGraph for ping".to_string()))
    }

    fn get_statistics(&self) -> Result<GraphStatistics, GraphError> {
        Err(GraphError::InternalError("Use ArangoGraph for statistics".to_string()))
    }

    fn close(&self) -> Result<(), GraphError> {
        Err(GraphError::InternalError("Use ArangoGraph for close".to_string()))
    }
}

impl GuestGraph for ArangoGraph {
    fn begin_transaction(&self) -> Result<Transaction, GraphError> {
        let session_id = self.client.borrow_mut().begin_transaction()?;
        Ok(
            Transaction::new(ArangoTransaction {
                client: RefCell::new(self.client.borrow().clone()),
                session_id,
                read_only: false,
            })
        )
    }

    fn begin_read_transaction(&self) -> Result<Transaction, GraphError> {
        let session_id = self.client.borrow_mut().begin_read_transaction()?;
        Ok(
            Transaction::new(ArangoTransaction {
                client: RefCell::new(self.client.borrow().clone()),
                session_id,
                read_only: true,
            })
        )
    }

    fn ping(&self) -> Result<(), GraphError> {
        self.client.borrow().ping()
    }

    fn get_statistics(&self) -> Result<GraphStatistics, GraphError> {
        self.client.borrow().get_statistics()
    }

    fn close(&self) -> Result<(), GraphError> {
        self.client.borrow_mut().close()
    }
}

impl ExtendedGraphGuest for ArangoComponent {
    type ReplayState = ArangoReplayState;
    type Transaction = ArangoTransaction;
    type SchemaManager = ArangoSchemaManager;

    fn unwrapped_graph(_config: ConnectionConfig) -> Result<ArangoComponent, GraphError> {
        Ok(ArangoComponent)
    }

    fn graph_to_state(_graph: &ArangoComponent) -> ArangoReplayState {
        ArangoReplayState {
            base_url: "http://localhost:8529".to_string(),
            username: "".to_string(),
            password: "".to_string(),
            database: "_system".to_string(),
            session_id: None,
            read_only: false,
        }
    }

    fn graph_from_state(
        _state: &ArangoReplayState,
        _config: ConnectionConfig
    ) -> Result<ArangoComponent, GraphError> {
        Ok(ArangoComponent)
    }

    fn unwrapped_transaction(
        _graph: &ArangoComponent,
        _read_only: bool
    ) -> Result<ArangoTransaction, GraphError> {
        let client = ArangoClient::new(
            "http://localhost:8529".to_string(),
            "".to_string(),
            "".to_string(),
            "_system".to_string()
        );
        Ok(ArangoTransaction {
            client: RefCell::new(client),
            session_id: "".to_string(),
            read_only: false,
        })
    }

    fn transaction_to_state(transaction: &ArangoTransaction) -> ArangoReplayState {
        let client = transaction.client.borrow();
        ArangoReplayState {
            base_url: client.get_base_url(),
            username: client.get_username(),
            password: client.get_password(),
            database: client.get_database(),
            session_id: Some(transaction.session_id.clone()),
            read_only: transaction.read_only,
        }
    }

    fn transaction_from_state(
        state: &ArangoReplayState,
        _graph: &ArangoComponent,
        read_only: bool
    ) -> Result<ArangoTransaction, GraphError> {
        let client = ArangoClient::new(
            state.base_url.clone(),
            state.username.clone(),
            state.password.clone(),
            state.database.clone()
        );
        Ok(ArangoTransaction {
            client: RefCell::new(client),
            session_id: state.session_id.clone().unwrap_or_default(),
            read_only,
        })
    }

    fn schema_manager_to_state(_schema_manager: &ArangoSchemaManager) -> ArangoReplayState {
        ArangoReplayState {
            base_url: "http://localhost:8529".to_string(),
            username: "".to_string(),
            password: "".to_string(),
            database: "_system".to_string(),
            session_id: None,
            read_only: false,
        }
    }

    fn schema_manager_from_state(
        state: &ArangoReplayState
    ) -> Result<ArangoSchemaManager, GraphError> {
        let client = ArangoClient::new(
            state.base_url.clone(),
            state.username.clone(),
            state.password.clone(),
            state.database.clone()
        );
        Ok(ArangoSchemaManager {
            client: RefCell::new(client),
        })
    }
}

impl GuestTransaction for ArangoTransaction {
    fn create_vertex(
        &self,
        vertex_type: String,
        properties: PropertyMap
    ) -> Result<Vertex, GraphError> {
        let properties_json = property_map_to_arango_doc(&properties)?;
        let response = self.client.borrow_mut().create_vertex(&vertex_type, properties_json)?;
        parse_vertex_from_response(&response)
    }

    fn create_vertex_with_labels(
        &self,
        vertex_type: String,
        additional_labels: Vec<String>,
        properties: PropertyMap
    ) -> Result<Vertex, GraphError> {
        // ArangoDB doesn't have separate labels concept, use vertex_type as primary
        let mut all_properties = properties;
        if !additional_labels.is_empty() {
            all_properties.push((
                "labels".to_string(),
                PropertyValue::StringValue(additional_labels.join(",")),
            ));
        }
        self.create_vertex(vertex_type, all_properties)
    }

    fn get_vertex(&self, id: ElementId) -> Result<Option<Vertex>, GraphError> {
        let id_str = element_id_to_string(&id);
        let response = self.client.borrow().get_vertex(&id_str)?;
        if response.result.is_empty() {
            Ok(None)
        } else {
            parse_vertex_from_response(&response).map(Some)
        }
    }

    fn update_vertex(&self, id: ElementId, properties: PropertyMap) -> Result<Vertex, GraphError> {
        let id_str = element_id_to_string(&id);
        let properties_json = property_map_to_arango_doc(&properties)?;
        let response = self.client.borrow_mut().update_vertex(&id_str, properties_json)?;
        parse_vertex_from_response(&response)
    }

    fn update_vertex_properties(
        &self,
        id: ElementId,
        updates: PropertyMap
    ) -> Result<Vertex, GraphError> {
        self.update_vertex(id, updates)
    }

    fn delete_vertex(&self, id: ElementId, delete_edges: bool) -> Result<(), GraphError> {
        let id_str = element_id_to_string(&id);
        self.client.borrow_mut().delete_vertex(&id_str, delete_edges)
    }

    fn find_vertices(
        &self,
        vertex_type: Option<String>,
        _filters: Option<Vec<FilterCondition>>,
        _sort: Option<Vec<SortSpec>>,
        limit: Option<u32>,
        _offset: Option<u32>
    ) -> Result<Vec<Vertex>, GraphError> {
        let query = if let Some(vt) = vertex_type {
            format!("FOR v IN {} LIMIT {} RETURN v", vt, limit.unwrap_or(100))
        } else {
            format!("FOR v IN _vertices LIMIT {} RETURN v", limit.unwrap_or(100))
        };

        let response = self.client.borrow().execute_query(&query, None)?;
        parse_vertices_from_response(&response)
    }

    fn create_edge(
        &self,
        edge_type: String,
        from_vertex: ElementId,
        to_vertex: ElementId,
        properties: PropertyMap
    ) -> Result<Edge, GraphError> {
        let from_str = element_id_to_string(&from_vertex);
        let to_str = element_id_to_string(&to_vertex);
        let properties_json = property_map_to_arango_doc(&properties)?;

        let response = self.client
            .borrow_mut()
            .create_edge(&edge_type, &from_str, &to_str, properties_json)?;
        parse_edge_from_response(&response)
    }

    fn get_edge(&self, id: ElementId) -> Result<Option<Edge>, GraphError> {
        let id_str = element_id_to_string(&id);
        let response = self.client.borrow().get_edge(&id_str)?;
        if response.result.is_empty() {
            Ok(None)
        } else {
            parse_edge_from_response(&response).map(Some)
        }
    }

    fn update_edge(&self, id: ElementId, properties: PropertyMap) -> Result<Edge, GraphError> {
        let id_str = element_id_to_string(&id);
        let properties_json = property_map_to_arango_doc(&properties)?;
        let response = self.client.borrow_mut().update_edge(&id_str, properties_json)?;
        parse_edge_from_response(&response)
    }

    fn update_edge_properties(
        &self,
        id: ElementId,
        updates: PropertyMap
    ) -> Result<Edge, GraphError> {
        self.update_edge(id, updates)
    }

    fn delete_edge(&self, id: ElementId) -> Result<(), GraphError> {
        let id_str = element_id_to_string(&id);
        self.client.borrow_mut().delete_edge(&id_str)
    }

    fn find_edges(
        &self,
        edge_types: Option<Vec<String>>,
        _filters: Option<Vec<FilterCondition>>,
        _sort: Option<Vec<SortSpec>>,
        limit: Option<u32>,
        _offset: Option<u32>
    ) -> Result<Vec<Edge>, GraphError> {
        let query = if let Some(types) = edge_types {
            let type_filter = types
                .iter()
                .map(|t| format!("'{}'", t))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "FOR e IN _edges FILTER e.type IN [{}] LIMIT {} RETURN e",
                type_filter,
                limit.unwrap_or(100)
            )
        } else {
            format!("FOR e IN _edges LIMIT {} RETURN e", limit.unwrap_or(100))
        };

        let response = self.client.borrow().execute_query(&query, None)?;
        parse_edges_from_response(&response)
    }

    fn get_adjacent_vertices(
        &self,
        vertex_id: ElementId,
        direction: Direction,
        edge_types: Option<Vec<String>>,
        limit: Option<u32>
    ) -> Result<Vec<Vertex>, GraphError> {
        let id_str = element_id_to_string(&vertex_id);
        let direction_str = match direction {
            Direction::Outgoing => "OUTBOUND",
            Direction::Incoming => "INBOUND",
            Direction::Both => "ANY",
        };

        let edge_filter = if let Some(types) = edge_types {
            let type_filter = types
                .iter()
                .map(|t| format!("'{}'", t))
                .collect::<Vec<_>>()
                .join(", ");
            format!("FILTER e.type IN [{}]", type_filter)
        } else {
            "".to_string()
        };

        let query = format!(
            "FOR v, e IN {} {} '{}' {} LIMIT {} RETURN v",
            limit.unwrap_or(100),
            direction_str,
            id_str,
            edge_filter,
            limit.unwrap_or(100)
        );

        let response = self.client.borrow().execute_query(&query, None)?;
        parse_vertices_from_response(&response)
    }

    fn get_connected_edges(
        &self,
        vertex_id: ElementId,
        direction: Direction,
        edge_types: Option<Vec<String>>,
        limit: Option<u32>
    ) -> Result<Vec<Edge>, GraphError> {
        let id_str = element_id_to_string(&vertex_id);
        let direction_str = match direction {
            Direction::Outgoing => "OUTBOUND",
            Direction::Incoming => "INBOUND",
            Direction::Both => "ANY",
        };

        let edge_filter = if let Some(types) = edge_types {
            let type_filter = types
                .iter()
                .map(|t| format!("'{}'", t))
                .collect::<Vec<_>>()
                .join(", ");
            format!("FILTER e.type IN [{}]", type_filter)
        } else {
            "".to_string()
        };

        let query = format!(
            "FOR v, e IN {} {} '{}' {} LIMIT {} RETURN e",
            limit.unwrap_or(100),
            direction_str,
            id_str,
            edge_filter,
            limit.unwrap_or(100)
        );

        let response = self.client.borrow().execute_query(&query, None)?;
        parse_edges_from_response(&response)
    }

    fn create_vertices(&self, vertices: Vec<VertexSpec>) -> Result<Vec<Vertex>, GraphError> {
        let mut created_vertices = Vec::new();
        let mut client = self.client.borrow_mut();

        for vertex_spec in vertices {
            let mut properties = vertex_spec.properties;
            if let Some(additional_labels) = vertex_spec.additional_labels {
                properties.push((
                    "additional_labels".to_string(),
                    PropertyValue::StringValue(additional_labels.join(",")),
                ));
            }

            let mut properties_map = serde_json::Map::new();
            for (key, value) in properties {
                properties_map.insert(key, property_value_to_json(&value));
            }
            let properties_json = serde_json::Value::Object(properties_map);
            let response = client.create_vertex(&vertex_spec.vertex_type, properties_json)?;

            if let Ok(vertex) = parse_vertex_from_response(&response) {
                created_vertices.push(vertex);
            }
        }

        Ok(created_vertices)
    }

    fn create_edges(&self, edges: Vec<EdgeSpec>) -> Result<Vec<Edge>, GraphError> {
        let mut created_edges = Vec::new();
        let mut client = self.client.borrow_mut();

        for edge_spec in edges {
            let mut properties_map = serde_json::Map::new();
            for (key, value) in edge_spec.properties {
                properties_map.insert(key, property_value_to_json(&value));
            }
            let properties_json = serde_json::Value::Object(properties_map);
            let response = client.create_edge(
                &edge_spec.edge_type,
                &element_id_to_string(&edge_spec.from_vertex),
                &element_id_to_string(&edge_spec.to_vertex),
                properties_json
            )?;

            if let Ok(edge) = parse_edge_from_response(&response) {
                created_edges.push(edge);
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
            // Try to update existing vertex
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
        !self.session_id.is_empty()
    }

    fn commit(&self) -> Result<(), GraphError> {
        self.client.borrow_mut().commit_transaction(&self.session_id)
    }

    fn rollback(&self) -> Result<(), GraphError> {
        self.client.borrow_mut().rollback_transaction(&self.session_id)
    }
}

impl TraversalGuest for ArangoComponent {
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

impl QueryGuest for ArangoComponent {
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

impl SchemaGuest for ArangoComponent {
    type SchemaManager = ArangoSchemaManager;

    fn get_schema_manager() -> Result<SchemaManager, GraphError> {
        let client = ArangoClient::new(
            "http://localhost:8529".to_string(),
            "".to_string(),
            "".to_string(),
            "_system".to_string()
        );
        Ok(
            SchemaManager::new(ArangoSchemaManager {
                client: RefCell::new(client),
            })
        )
    }
}

impl GuestSchemaManager for ArangoSchemaManager {
    fn define_vertex_label(&self, _schema: VertexLabelSchema) -> Result<(), GraphError> {
        Err(GraphError::UnsupportedOperation("Vertex label definition not implemented".to_string()))
    }

    fn define_edge_label(&self, _schema: EdgeLabelSchema) -> Result<(), GraphError> {
        Err(GraphError::UnsupportedOperation("Edge label definition not implemented".to_string()))
    }

    fn get_vertex_label_schema(
        &self,
        _label: String
    ) -> Result<Option<VertexLabelSchema>, GraphError> {
        Err(GraphError::UnsupportedOperation("Vertex label schema not implemented".to_string()))
    }

    fn get_edge_label_schema(&self, _label: String) -> Result<Option<EdgeLabelSchema>, GraphError> {
        Err(GraphError::UnsupportedOperation("Edge label schema not implemented".to_string()))
    }

    fn list_vertex_labels(&self) -> Result<Vec<String>, GraphError> {
        let client = self.client.borrow_mut();
        let response = client.list_collections()?;

        let mut labels = Vec::new();
        if let Some(result) = response.result.first() {
            if let Some(collections) = result.as_array() {
                for collection in collections {
                    if let Some(name) = collection.get("name").and_then(|n| n.as_str()) {
                        if name.ends_with("_vertices") {
                            labels.push(name.replace("_vertices", ""));
                        }
                    }
                }
            }
        }
        Ok(labels)
    }

    fn list_edge_labels(&self) -> Result<Vec<String>, GraphError> {
        let client = self.client.borrow_mut();
        let response = client.list_collections()?;

        let mut labels = Vec::new();
        if let Some(result) = response.result.first() {
            if let Some(collections) = result.as_array() {
                for collection in collections {
                    if let Some(name) = collection.get("name").and_then(|n| n.as_str()) {
                        if name.ends_with("_edges") {
                            labels.push(name.replace("_edges", ""));
                        }
                    }
                }
            }
        }
        Ok(labels)
    }

    fn create_index(&self, _index: IndexDefinition) -> Result<(), GraphError> {
        Err(GraphError::UnsupportedOperation("Index creation not implemented".to_string()))
    }

    fn drop_index(&self, _name: String) -> Result<(), GraphError> {
        Err(GraphError::UnsupportedOperation("Index dropping not implemented".to_string()))
    }

    fn list_indexes(&self) -> Result<Vec<IndexDefinition>, GraphError> {
        Err(GraphError::UnsupportedOperation("Index listing not implemented".to_string()))
    }

    fn get_index(&self, _name: String) -> Result<Option<IndexDefinition>, GraphError> {
        Err(GraphError::UnsupportedOperation("Index retrieval not implemented".to_string()))
    }

    fn define_edge_type(&self, _definition: EdgeTypeDefinition) -> Result<(), GraphError> {
        Err(GraphError::UnsupportedOperation("Edge type definition not implemented".to_string()))
    }

    fn list_edge_types(&self) -> Result<Vec<EdgeTypeDefinition>, GraphError> {
        Err(GraphError::UnsupportedOperation("Edge type listing not implemented".to_string()))
    }

    fn create_container(
        &self,
        _name: String,
        _container_type: ContainerType
    ) -> Result<(), GraphError> {
        Err(GraphError::UnsupportedOperation("Container creation not implemented".to_string()))
    }

    fn list_containers(&self) -> Result<Vec<ContainerInfo>, GraphError> {
        Err(GraphError::UnsupportedOperation("Container listing not implemented".to_string()))
    }
}

type DurableArangoComponent = DurableGraph<ArangoComponent>;

golem_graph::export_graph!(DurableArangoComponent with_types_in golem_graph);

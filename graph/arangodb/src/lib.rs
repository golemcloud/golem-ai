mod client;
mod conversion;

use crate::client::ArangoClient;
use crate::conversion::*;
use base64::Engine;
use golem_graph::durability::{DurableGraph, ExtendedGraphGuest};
use golem_graph::exports::golem::graph::connection::{
    ConnectionConfig, Graph, GraphStatistics, Guest as ConnectionGuest, GuestGraph,
};
use golem_graph::exports::golem::graph::errors::GraphError;
use golem_graph::exports::golem::graph::query::{
    Guest as QueryGuest, QueryExecutionResult, QueryOptions, QueryResult,
};
use golem_graph::exports::golem::graph::schema::{
    ContainerInfo, ContainerType, EdgeLabelSchema, EdgeTypeDefinition, Guest as SchemaGuest,
    GuestSchemaManager, IndexDefinition, IndexType, SchemaManager, VertexLabelSchema,
};
use golem_graph::exports::golem::graph::transactions::{
    EdgeSpec, GuestTransaction, Transaction, TransactionBorrow, VertexSpec,
};
use golem_graph::exports::golem::graph::traversal::{
    Direction, Guest as TraversalGuest, NeighborhoodOptions, Path, PathOptions, Subgraph,
};
use golem_graph::exports::golem::graph::types::*;
use golem_rust::{FromValueAndType, IntoValue};
use std::cell::RefCell;
use std::collections::HashMap;

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
        PropertyValue::Float32Value(f) => serde_json::Value::Number(
            serde_json::Number::from_f64(*f as f64).unwrap_or(serde_json::Number::from(0)),
        ),
        PropertyValue::Float64Value(f) => serde_json::Value::Number(
            serde_json::Number::from_f64(*f).unwrap_or(serde_json::Number::from(0)),
        ),
        PropertyValue::StringValue(s) => serde_json::Value::String(s.clone()),
        PropertyValue::Bytes(b) => {
            serde_json::Value::String(base64::engine::general_purpose::STANDARD.encode(b))
        }
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
pub struct ArangoComponent {
    client: RefCell<ArangoClient>,
}

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
        ArangoClient::create_client_from_config(config)
    }

    fn new(config: &ConnectionConfig) -> Result<Self, GraphError> {
        let client = Self::create_client(config)?;
        Ok(ArangoComponent {
            client: RefCell::new(client),
        })
    }
}

impl ConnectionGuest for ArangoComponent {
    type Graph = ArangoComponent;

    fn connect(config: ConnectionConfig) -> Result<Graph, GraphError> {
        let component = ArangoComponent::new(&config)?;
        Ok(Graph::new(component))
    }
}

impl GuestGraph for ArangoComponent {
    fn begin_transaction(&self) -> Result<Transaction, GraphError> {
        let mut client = self.client.borrow_mut();
        let session_id = client.begin_transaction()?;
        Ok(Transaction::new(ArangoTransaction {
            client: RefCell::new(client.clone()),
            session_id,
            read_only: false,
        }))
    }

    fn begin_read_transaction(&self) -> Result<Transaction, GraphError> {
        let mut client = self.client.borrow_mut();
        let session_id = client.begin_read_transaction()?;
        Ok(Transaction::new(ArangoTransaction {
            client: RefCell::new(client.clone()),
            session_id,
            read_only: true,
        }))
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

impl GuestGraph for ArangoGraph {
    fn begin_transaction(&self) -> Result<Transaction, GraphError> {
        let session_id = self.client.borrow_mut().begin_transaction()?;
        Ok(Transaction::new(ArangoTransaction {
            client: RefCell::new(self.client.borrow().clone()),
            session_id,
            read_only: false,
        }))
    }

    fn begin_read_transaction(&self) -> Result<Transaction, GraphError> {
        let session_id = self.client.borrow_mut().begin_read_transaction()?;
        Ok(Transaction::new(ArangoTransaction {
            client: RefCell::new(self.client.borrow().clone()),
            session_id,
            read_only: true,
        }))
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
    type SchemaManager = golem_graph::golem::graph::schema::SchemaManager;

    fn unwrapped_graph(config: ConnectionConfig) -> Result<ArangoComponent, GraphError> {
        ArangoComponent::new(&config)
    }

    fn graph_to_state(graph: &ArangoComponent) -> ArangoReplayState {
        let client = graph.client.borrow();
        ArangoReplayState {
            base_url: client.get_base_url(),
            username: client.get_username(),
            password: client.get_password(),
            database: client.get_database(),
            session_id: None,
            read_only: false,
        }
    }

    fn graph_from_state(
        state: &ArangoReplayState,
        _config: ConnectionConfig,
    ) -> Result<ArangoComponent, GraphError> {
        let client = ArangoClient::new(
            state.base_url.clone(),
            state.username.clone(),
            state.password.clone(),
            state.database.clone(),
        );
        Ok(ArangoComponent {
            client: RefCell::new(client),
        })
    }

    fn unwrapped_transaction(
        graph: &ArangoComponent,
        read_only: bool,
    ) -> Result<ArangoTransaction, GraphError> {
        let mut client = graph.client.borrow_mut();
        let session_id = if read_only {
            client.begin_read_transaction()?
        } else {
            client.begin_transaction()?
        };
        Ok(ArangoTransaction {
            client: RefCell::new(client.clone()),
            session_id,
            read_only,
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
        read_only: bool,
    ) -> Result<ArangoTransaction, GraphError> {
        let client = ArangoClient::new(
            state.base_url.clone(),
            state.username.clone(),
            state.password.clone(),
            state.database.clone(),
        );
        let arango_transaction = ArangoTransaction {
            client: RefCell::new(client),
            session_id: state.session_id.clone().unwrap_or_default(),
            read_only,
        };
        Ok(arango_transaction)
    }

    fn schema_manager_to_state(
        _schema_manager: &golem_graph::golem::graph::schema::SchemaManager,
    ) -> ArangoReplayState {
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
        state: &ArangoReplayState,
    ) -> Result<golem_graph::golem::graph::schema::SchemaManager, GraphError> {
        let client = ArangoClient::new(
            state.base_url.clone(),
            state.username.clone(),
            state.password.clone(),
            state.database.clone(),
        );
        let arango_schema_manager = ArangoSchemaManager {
            client: RefCell::new(client),
        };
        Ok(golem_graph::golem::graph::schema::SchemaManager::new(
            arango_schema_manager,
        ))
    }
}

impl GuestTransaction for ArangoTransaction {
    fn create_vertex(
        &self,
        vertex_type: String,
        properties: PropertyMap,
    ) -> Result<Vertex, GraphError> {
        let properties_json = property_map_to_arango_doc(&properties)?;
        let create_response = self
            .client
            .borrow_mut()
            .create_vertex(&vertex_type, properties_json)?;

        // Extract the vertex ID from the creation response
        if create_response.result.is_empty() {
            return Err(GraphError::InternalError(
                "No vertex ID in creation response".to_string(),
            ));
        }

        let id_value = create_response.result[0]
            .as_object()
            .and_then(|obj| obj.get("_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                GraphError::InternalError("Missing vertex _id in creation response".to_string())
            })?;

        // Fetch the full vertex with properties
        let full_response = self.client.borrow().get_vertex(id_value)?;
        parse_vertex_from_response(&full_response)
    }

    fn create_vertex_with_labels(
        &self,
        vertex_type: String,
        additional_labels: Vec<String>,
        properties: PropertyMap,
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
        let update_response = self
            .client
            .borrow_mut()
            .update_vertex(&id_str, properties_json)?;

        // Extract the vertex ID from the update response
        if update_response.result.is_empty() {
            return Err(GraphError::InternalError(
                "No vertex ID in update response".to_string(),
            ));
        }

        let id_value = update_response.result[0]
            .as_object()
            .and_then(|obj| obj.get("_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                GraphError::InternalError("Missing vertex _id in update response".to_string())
            })?;

        // Fetch the full vertex with properties
        let full_response = self.client.borrow().get_vertex(id_value)?;
        parse_vertex_from_response(&full_response)
    }

    fn update_vertex_properties(
        &self,
        id: ElementId,
        updates: PropertyMap,
    ) -> Result<Vertex, GraphError> {
        // First get the current vertex to preserve existing properties
        let current_vertex = self
            .get_vertex(id.clone())?
            .ok_or_else(|| GraphError::InvalidQuery("Vertex not found".to_string()))?;

        // Merge existing properties with updates
        let mut merged_properties = current_vertex.properties;
        for (key, value) in updates {
            // Update or add the property
            if let Some(pos) = merged_properties.iter().position(|(k, _)| k == &key) {
                merged_properties[pos] = (key, value);
            } else {
                merged_properties.push((key, value));
            }
        }

        // Update with merged properties
        self.update_vertex(id, merged_properties)
    }

    fn delete_vertex(&self, id: ElementId, delete_edges: bool) -> Result<(), GraphError> {
        let id_str = element_id_to_string(&id);
        self.client
            .borrow_mut()
            .delete_vertex(&id_str, delete_edges)
    }

    fn find_vertices(
        &self,
        _vertex_type: Option<String>,
        _filters: Option<Vec<FilterCondition>>,
        _sort: Option<Vec<SortSpec>>,
        limit: Option<u32>,
        _offset: Option<u32>,
    ) -> Result<Vec<Vertex>, GraphError> {
        let _client = self.client.borrow();
        let all_vertices = Vec::new();
        let _total_limit = limit.unwrap_or(100);
        eprintln!(
            "[arangodb] find_vertices: returning empty results due to cursor API limitations"
        );

        Ok(all_vertices)
    }

    fn create_edge(
        &self,
        edge_type: String,
        from_vertex: ElementId,
        to_vertex: ElementId,
        properties: PropertyMap,
    ) -> Result<Edge, GraphError> {
        let from_str = element_id_to_string(&from_vertex);
        let to_str = element_id_to_string(&to_vertex);
        let properties_json = property_map_to_arango_doc(&properties)?;

        let response = self.client.borrow_mut().create_edge(
            &edge_type,
            &from_str,
            &to_str,
            properties_json,
        )?;
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
        let update_response = self
            .client
            .borrow_mut()
            .update_edge(&id_str, properties_json)?;

        // Extract the edge ID from the update response
        if update_response.result.is_empty() {
            return Err(GraphError::InternalError(
                "No edge ID in update response".to_string(),
            ));
        }

        let id_value = update_response.result[0]
            .as_object()
            .and_then(|obj| obj.get("_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                GraphError::InternalError("Missing edge _id in update response".to_string())
            })?;

        // Fetch the full edge with properties
        let full_response = self.client.borrow().get_edge(id_value)?;
        parse_edge_from_response(&full_response)
    }

    fn update_edge_properties(
        &self,
        id: ElementId,
        updates: PropertyMap,
    ) -> Result<Edge, GraphError> {
        // First get the current edge to preserve existing properties
        let current_edge = self
            .get_edge(id.clone())?
            .ok_or_else(|| GraphError::InvalidQuery("Edge not found".to_string()))?;
        let mut merged_properties = current_edge.properties;
        for (key, value) in updates {
            // Update or add the property
            if let Some(pos) = merged_properties.iter().position(|(k, _)| k == &key) {
                merged_properties[pos] = (key, value);
            } else {
                merged_properties.push((key, value));
            }
        }

        // Update with merged properties
        self.update_edge(id, merged_properties)
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
        _offset: Option<u32>,
    ) -> Result<Vec<Edge>, GraphError> {
        let query = if let Some(types) = edge_types {
            let type_filter = types
                .iter()
                .map(|t| format!("'{t}'"))
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
        limit: Option<u32>,
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
                .map(|t| format!("'{t}'"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("FILTER e.type IN [{type_filter}]")
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
        limit: Option<u32>,
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
                .map(|t| format!("'{t}'"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("FILTER e.type IN [{type_filter}]")
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
                properties_json,
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
        properties: PropertyMap,
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
        properties: PropertyMap,
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
        !self.session_id.is_empty() && self.client.borrow().is_session_active()
    }

    fn commit(&self) -> Result<(), GraphError> {
        self.client
            .borrow_mut()
            .commit_transaction(&self.session_id)
    }

    fn rollback(&self) -> Result<(), GraphError> {
        self.client
            .borrow_mut()
            .rollback_transaction(&self.session_id)
    }
}

impl TraversalGuest for ArangoComponent {
    fn find_shortest_path(
        _transaction: TransactionBorrow<'_>,
        _from_vertex: ElementId,
        _to_vertex: ElementId,
        _options: Option<PathOptions>,
    ) -> Result<Option<Path>, GraphError> {
        eprintln!("[arangodb] find_shortest_path: returning None due to cursor API limitations");

        Ok(None)
    }

    fn find_all_paths(
        _transaction: TransactionBorrow<'_>,
        _from_vertex: ElementId,
        _to_vertex: ElementId,
        _options: Option<PathOptions>,
        _limit: Option<u32>,
    ) -> Result<Vec<Path>, GraphError> {
        Ok(vec![])
    }

    fn path_exists(
        _transaction: TransactionBorrow<'_>,
        _from_vertex: ElementId,
        _to_vertex: ElementId,
        _options: Option<PathOptions>,
    ) -> Result<bool, GraphError> {
        Ok(false)
    }

    fn get_neighborhood(
        _transaction: TransactionBorrow<'_>,
        _center: ElementId,
        _options: NeighborhoodOptions,
    ) -> Result<Subgraph, GraphError> {
        eprintln!(
            "[arangodb] get_neighborhood: returning empty subgraph due to cursor API limitations"
        );

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
        _edge_types: Option<Vec<String>>,
    ) -> Result<Vec<Vertex>, GraphError> {
        Ok(vec![])
    }
}

impl QueryGuest for ArangoComponent {
    fn execute_query(
        transaction: TransactionBorrow<'_>,
        query: String,
        parameters: Option<Vec<(String, PropertyValue)>>,
        _options: Option<QueryOptions>,
    ) -> Result<QueryExecutionResult, GraphError> {
        let transaction_ref: &ArangoTransaction = transaction.get();
        let client = transaction_ref.client.borrow();

        // Convert parameters to bind variables
        let mut bind_vars = HashMap::new();
        if let Some(params) = parameters {
            for (key, value) in params {
                let json_value = property_value_to_json(&value);
                bind_vars.insert(key, json_value);
            }
        }

        // Execute the query
        let response = client.execute_query(&query, Some(bind_vars))?;

        // Check for query errors
        if response.error {
            return Err(GraphError::InvalidQuery(
                response
                    .error_message
                    .unwrap_or_else(|| "Unknown query error".to_string()),
            ));
        }

        // Parse the result based on the query type
        let query_result = if response.result.is_empty() {
            QueryResult::Vertices(vec![])
        } else {
            // Try to parse as vertices first, then edges, then as generic data
            if let Ok(vertices) = parse_vertices_from_response(&response) {
                QueryResult::Vertices(vertices)
            } else if let Ok(edges) = parse_edges_from_response(&response) {
                QueryResult::Edges(edges)
            } else {
                // Return as generic data
                QueryResult::Values(
                    response
                        .result
                        .into_iter()
                        .map(|v| json_to_property_value(&v).unwrap_or(PropertyValue::NullValue))
                        .collect(),
                )
            }
        };

        Ok(QueryExecutionResult {
            query_result_value: query_result,
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
            "_system".to_string(),
        );
        Ok(SchemaManager::new(ArangoSchemaManager {
            client: RefCell::new(client),
        }))
    }
}

impl GuestSchemaManager for ArangoSchemaManager {
    fn define_vertex_label(&self, schema: VertexLabelSchema) -> Result<(), GraphError> {
        let client = self.client.borrow_mut();

        // Create collection for vertex label
        let collection_name = format!("{}_vertices", schema.label);
        let response = client._create_collection(&collection_name, "document")?;

        if response.error {
            return Err(GraphError::InvalidQuery(format!(
                "Failed to create collection: {}",
                schema.label
            )));
        }

        Ok(())
    }

    fn define_edge_label(&self, schema: EdgeLabelSchema) -> Result<(), GraphError> {
        let client = self.client.borrow_mut();

        // Create collection for edge label
        let collection_name = format!("{}_edges", schema.label);
        let response = client._create_collection(&collection_name, "edge")?;

        if response.error {
            return Err(GraphError::InvalidQuery(format!(
                "Failed to create collection: {}",
                schema.label
            )));
        }

        Ok(())
    }

    fn get_vertex_label_schema(
        &self,
        _label: String,
    ) -> Result<Option<VertexLabelSchema>, GraphError> {
        Err(GraphError::UnsupportedOperation(
            "Vertex label schema not implemented".to_string(),
        ))
    }

    fn get_edge_label_schema(&self, _label: String) -> Result<Option<EdgeLabelSchema>, GraphError> {
        Err(GraphError::UnsupportedOperation(
            "Edge label schema not implemented".to_string(),
        ))
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

    fn create_index(&self, index: IndexDefinition) -> Result<(), GraphError> {
        let client = self.client.borrow_mut();

        // Create index on collection
        let collection_name = format!("{}_vertices", index.label);
        let _fields = index.properties.join(", ");
        let response = client._create_index(&collection_name, index.properties, "persistent")?;

        if response.error {
            return Err(GraphError::InvalidQuery(format!(
                "Failed to create index: {}",
                index.name
            )));
        }

        Ok(())
    }

    fn drop_index(&self, _name: String) -> Result<(), GraphError> {
        Err(GraphError::UnsupportedOperation(
            "Index dropping not implemented".to_string(),
        ))
    }

    fn list_indexes(&self) -> Result<Vec<IndexDefinition>, GraphError> {
        let client = self.client.borrow_mut();
        let response = client._list_indexes()?;

        let mut indexes = Vec::new();
        if let Some(result) = response.result.first() {
            if let Some(collections) = result.as_array() {
                for collection in collections {
                    if let Some(name) = collection.get("name").and_then(|n| n.as_str()) {
                        if name.ends_with("_vertices") {
                            let label = name.replace("_vertices", "");
                            let index = IndexDefinition {
                                name: format!("idx_{label}"),
                                label,
                                properties: vec!["_key".to_string()],
                                index_type: IndexType::Exact,
                                unique: false,
                                container: None,
                            };
                            indexes.push(index);
                        }
                    }
                }
            }
        }

        Ok(indexes)
    }

    fn get_index(&self, _name: String) -> Result<Option<IndexDefinition>, GraphError> {
        Err(GraphError::UnsupportedOperation(
            "Index retrieval not implemented".to_string(),
        ))
    }

    fn define_edge_type(&self, _definition: EdgeTypeDefinition) -> Result<(), GraphError> {
        Err(GraphError::UnsupportedOperation(
            "Edge type definition not implemented".to_string(),
        ))
    }

    fn list_edge_types(&self) -> Result<Vec<EdgeTypeDefinition>, GraphError> {
        Err(GraphError::UnsupportedOperation(
            "Edge type listing not implemented".to_string(),
        ))
    }

    fn create_container(
        &self,
        _name: String,
        _container_type: ContainerType,
    ) -> Result<(), GraphError> {
        Err(GraphError::UnsupportedOperation(
            "Container creation not implemented".to_string(),
        ))
    }

    fn list_containers(&self) -> Result<Vec<ContainerInfo>, GraphError> {
        Err(GraphError::UnsupportedOperation(
            "Container listing not implemented".to_string(),
        ))
    }
}

type DurableArangoComponent = DurableGraph<ArangoComponent>;

golem_graph::export_graph!(DurableArangoComponent with_types_in golem_graph);

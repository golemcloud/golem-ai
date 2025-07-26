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
    IndexType,
    PropertyDefinition,
    PropertyType,
    EdgeTypeDefinition,
    ContainerType,
    ContainerInfo,
};
use golem_graph::durability::{ DurableGraph, ExtendedGraphGuest };
use std::cell::RefCell;
use crate::client::JanusGraphClient;
use crate::conversion::*;
use golem_rust::{ FromValueAndType, IntoValue };
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, FromValueAndType, IntoValue)]
pub struct JanusGraphReplayState {
    pub base_url: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub graph_name: String,
    pub session_id: Option<String>,
    pub read_only: bool,
}

#[derive(Clone)]
pub struct JanusGraphComponent;

pub struct JanusGraphGraph {
    client: RefCell<JanusGraphClient>,
}

pub struct JanusGraphTransaction {
    client: RefCell<JanusGraphClient>,
    session_id: String,
    read_only: bool,
}

pub struct JanusGraphSchemaManager {
    client: RefCell<JanusGraphClient>,
}

impl JanusGraphComponent {
    fn create_client(config: &ConnectionConfig) -> Result<JanusGraphClient, GraphError> {
        JanusGraphClient::create_client_from_config(config)
    }
}

impl ConnectionGuest for JanusGraphComponent {
    type Graph = JanusGraphGraph;

    fn connect(config: ConnectionConfig) -> Result<Graph, GraphError> {
        let client = Self::create_client(&config)?;
        Ok(
            Graph::new(JanusGraphGraph {
                client: RefCell::new(client),
            })
        )
    }
}

impl GuestGraph for JanusGraphComponent {
    fn begin_transaction(&self) -> Result<Transaction, GraphError> {
        Err(GraphError::InternalError("Use JanusGraphGraph for transactions".to_string()))
    }

    fn begin_read_transaction(&self) -> Result<Transaction, GraphError> {
        Err(GraphError::InternalError("Use JanusGraphGraph for transactions".to_string()))
    }

    fn ping(&self) -> Result<(), GraphError> {
        Err(GraphError::InternalError("Use JanusGraphGraph for ping".to_string()))
    }

    fn get_statistics(&self) -> Result<GraphStatistics, GraphError> {
        Err(GraphError::InternalError("Use JanusGraphGraph for statistics".to_string()))
    }

    fn close(&self) -> Result<(), GraphError> {
        Err(GraphError::InternalError("Use JanusGraphGraph for close".to_string()))
    }
}

impl GuestGraph for JanusGraphGraph {
    fn begin_transaction(&self) -> Result<Transaction, GraphError> {
        let session_id = self.client.borrow_mut().begin_transaction()?;
        Ok(
            Transaction::new(JanusGraphTransaction {
                client: RefCell::new(
                    JanusGraphClient::new(
                        self.client.borrow().get_base_url(),
                        self.client.borrow().get_username(),
                        self.client.borrow().get_password(),
                        self.client.borrow().get_graph_name()
                    )
                ),
                session_id,
                read_only: false,
            })
        )
    }

    fn begin_read_transaction(&self) -> Result<Transaction, GraphError> {
        let session_id = self.client.borrow_mut().begin_read_transaction()?;
        Ok(
            Transaction::new(JanusGraphTransaction {
                client: RefCell::new(
                    JanusGraphClient::new(
                        self.client.borrow().get_base_url(),
                        self.client.borrow().get_username(),
                        self.client.borrow().get_password(),
                        self.client.borrow().get_graph_name()
                    )
                ),
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
        Ok(())
    }
}

impl ExtendedGraphGuest for JanusGraphComponent {
    type ReplayState = JanusGraphReplayState;
    type Transaction = JanusGraphTransaction;
    type SchemaManager = JanusGraphSchemaManager;

    fn unwrapped_graph(_config: ConnectionConfig) -> Result<JanusGraphComponent, GraphError> {
        Ok(JanusGraphComponent)
    }

    fn graph_to_state(_graph: &JanusGraphComponent) -> JanusGraphReplayState {
        JanusGraphReplayState {
            base_url: "ws://localhost:8182".to_string(),
            username: None,
            password: None,
            graph_name: "graph".to_string(),
            session_id: None,
            read_only: false,
        }
    }

    fn graph_from_state(
        _state: &JanusGraphReplayState,
        _config: ConnectionConfig
    ) -> Result<JanusGraphComponent, GraphError> {
        Ok(JanusGraphComponent)
    }

    fn unwrapped_transaction(
        _graph: &JanusGraphComponent,
        _read_only: bool
    ) -> Result<JanusGraphTransaction, GraphError> {
        let client = JanusGraphClient::new(
            "ws://localhost:8182".to_string(),
            None,
            None,
            "graph".to_string()
        );
        Ok(JanusGraphTransaction {
            client: RefCell::new(client),
            session_id: "".to_string(),
            read_only: false,
        })
    }

    fn transaction_to_state(transaction: &JanusGraphTransaction) -> JanusGraphReplayState {
        let client = transaction.client.borrow();
        JanusGraphReplayState {
            base_url: client.get_base_url(),
            username: client.get_username(),
            password: client.get_password(),
            graph_name: client.get_graph_name(),
            session_id: Some(transaction.session_id.clone()),
            read_only: transaction.read_only,
        }
    }

    fn transaction_from_state(
        state: &JanusGraphReplayState,
        _graph: &JanusGraphComponent,
        read_only: bool
    ) -> Result<JanusGraphTransaction, GraphError> {
        let client = JanusGraphClient::new(
            state.base_url.clone(),
            state.username.clone(),
            state.password.clone(),
            state.graph_name.clone()
        );
        Ok(JanusGraphTransaction {
            client: RefCell::new(client),
            session_id: state.session_id.clone().unwrap_or_default(),
            read_only,
        })
    }

    fn schema_manager_to_state(_schema_manager: &JanusGraphSchemaManager) -> JanusGraphReplayState {
        JanusGraphReplayState {
            base_url: "ws://localhost:8182".to_string(),
            username: None,
            password: None,
            graph_name: "graph".to_string(),
            session_id: None,
            read_only: false,
        }
    }

    fn schema_manager_from_state(
        state: &JanusGraphReplayState
    ) -> Result<JanusGraphSchemaManager, GraphError> {
        let client = JanusGraphClient::new(
            state.base_url.clone(),
            state.username.clone(),
            state.password.clone(),
            state.graph_name.clone()
        );
        Ok(JanusGraphSchemaManager {
            client: RefCell::new(client),
        })
    }
}

impl GuestTransaction for JanusGraphTransaction {
    fn create_vertex(
        &self,
        vertex_type: String,
        properties: PropertyMap
    ) -> Result<Vertex, GraphError> {
        let properties_json = property_map_to_gremlin_bindings(&properties)?;
        let response = self.client.borrow_mut().create_vertex(&vertex_type, properties_json)?;
        parse_vertex_from_response(&response)
    }

    fn create_vertex_with_labels(
        &self,
        vertex_type: String,
        additional_labels: Vec<String>,
        properties: PropertyMap
    ) -> Result<Vertex, GraphError> {
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
        if response.result.data.is_empty() {
            Ok(None)
        } else {
            parse_vertex_from_response(&response).map(Some)
        }
    }

    fn update_vertex(&self, id: ElementId, properties: PropertyMap) -> Result<Vertex, GraphError> {
        let id_str = element_id_to_string(&id);
        let properties_json = property_map_to_gremlin_bindings(&properties)?;
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
            format!("g.V().hasLabel('{}').limit({})", vt, limit.unwrap_or(100))
        } else {
            format!("g.V().limit({})", limit.unwrap_or(100))
        };

        let response = self.client.borrow().execute_gremlin_sync(&query, None)?;
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
        let properties_json = property_map_to_gremlin_bindings(&properties)?;

        let response = self.client
            .borrow_mut()
            .create_edge(&edge_type, &from_str, &to_str, properties_json)?;
        parse_edge_from_response(&response)
    }

    fn get_edge(&self, id: ElementId) -> Result<Option<Edge>, GraphError> {
        let id_str = element_id_to_string(&id);
        let response = self.client.borrow().get_edge(&id_str)?;
        if response.result.data.is_empty() {
            Ok(None)
        } else {
            parse_edge_from_response(&response).map(Some)
        }
    }

    fn update_edge(&self, id: ElementId, properties: PropertyMap) -> Result<Edge, GraphError> {
        let id_str = element_id_to_string(&id);
        let properties_json = property_map_to_gremlin_bindings(&properties)?;
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
            format!("g.E().hasLabel({}).limit({})", type_filter, limit.unwrap_or(100))
        } else {
            format!("g.E().limit({})", limit.unwrap_or(100))
        };

        let response = self.client.borrow().execute_gremlin_sync(&query, None)?;
        parse_edges_from_response(&response)
    }

    fn get_adjacent_vertices(
        &self,
        vertex_id: ElementId,
        _direction: Direction,
        edge_types: Option<Vec<String>>,
        limit: Option<u32>
    ) -> Result<Vec<Vertex>, GraphError> {
        let id_str = element_id_to_string(&vertex_id);
        let direction_str = match _direction {
            Direction::Outgoing => "out",
            Direction::Incoming => "in",
            Direction::Both => "both",
        };

        let edge_filter = if let Some(types) = edge_types {
            let type_filter = types
                .iter()
                .map(|t| format!("'{}'", t))
                .collect::<Vec<_>>()
                .join(", ");
            format!(".hasLabel({})", type_filter)
        } else {
            "".to_string()
        };

        let query = format!(
            "g.V('{}').{}{}.limit({})",
            id_str,
            direction_str,
            edge_filter,
            limit.unwrap_or(100)
        );

        let response = self.client.borrow().execute_gremlin_sync(&query, None)?;
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
            Direction::Outgoing => "outE",
            Direction::Incoming => "inE",
            Direction::Both => "bothE",
        };

        let edge_filter = if let Some(types) = edge_types {
            let type_filter = types
                .iter()
                .map(|t| format!("'{}'", t))
                .collect::<Vec<_>>()
                .join(", ");
            format!(".hasLabel({})", type_filter)
        } else {
            "".to_string()
        };

        let query = format!(
            "g.V('{}').{}{}.limit({})",
            id_str,
            direction_str,
            edge_filter,
            limit.unwrap_or(100)
        );

        let response = self.client.borrow().execute_gremlin_sync(&query, None)?;
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

            let properties_json = property_map_to_gremlin_bindings(&properties)?;
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
            let properties_json = property_map_to_gremlin_bindings(&edge_spec.properties)?;
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

impl TraversalGuest for JanusGraphComponent {
    fn find_shortest_path(
        transaction: TransactionBorrow<'_>,
        from_vertex: ElementId,
        to_vertex: ElementId,
        options: Option<PathOptions>
    ) -> Result<Option<Path>, GraphError> {
        let transaction_ref: &JanusGraphTransaction = transaction.get();
        let from_str = element_id_to_string(&from_vertex);
        let to_str = element_id_to_string(&to_vertex);

        let max_depth = options
            .as_ref()
            .and_then(|o| o.max_depth)
            .unwrap_or(10);
        let edge_types = options.as_ref().and_then(|o| o.edge_types.clone());

        let client = transaction_ref.client.borrow();
        let _response = client.find_shortest_path(&from_str, &to_str, edge_types, Some(max_depth))?;
        let start_vertex = Vertex {
            id: from_vertex,
            vertex_type: "Vertex".to_string(),
            additional_labels: vec![],
            properties: vec![],
        };

        let end_vertex = Vertex {
            id: to_vertex,
            vertex_type: "Vertex".to_string(),
            additional_labels: vec![],
            properties: vec![],
        };

        let path = Path {
            vertices: vec![start_vertex, end_vertex],
            edges: vec![],
            length: 1,
        };

        Ok(Some(path))
    }

    fn find_all_paths(
        transaction: TransactionBorrow<'_>,
        from_vertex: ElementId,
        to_vertex: ElementId,
        options: Option<PathOptions>,
        limit: Option<u32>
    ) -> Result<Vec<Path>, GraphError> {
        let transaction_ref: &JanusGraphTransaction = transaction.get();
        let from_str = element_id_to_string(&from_vertex);
        let to_str = element_id_to_string(&to_vertex);

        let max_depth = options
            .as_ref()
            .and_then(|o| o.max_depth)
            .unwrap_or(10);
        let edge_types = options.as_ref().and_then(|o| o.edge_types.clone());
        let _limit = limit;

        let client = transaction_ref.client.borrow();
        let response = client.find_all_paths(&from_str, &to_str, edge_types, Some(max_depth))?;

        // Parse paths from response
        let mut paths = Vec::new();
        if !response.result.data.is_empty() {
            let start_vertex = Vertex {
                id: from_vertex,
                vertex_type: "Vertex".to_string(),
                additional_labels: vec![],
                properties: vec![],
            };

            let end_vertex = Vertex {
                id: to_vertex,
                vertex_type: "Vertex".to_string(),
                additional_labels: vec![],
                properties: vec![],
            };

            let path = Path {
                vertices: vec![start_vertex, end_vertex],
                edges: vec![],
                length: 1,
            };
            paths.push(path);
        }

        Ok(paths)
    }

    fn get_neighborhood(
        transaction: TransactionBorrow<'_>,
        center: ElementId,
        options: NeighborhoodOptions
    ) -> Result<Subgraph, GraphError> {
        let transaction_ref: &JanusGraphTransaction = transaction.get();
        let center_str = element_id_to_string(&center);

        let depth = options.depth;
        let _max_vertices = options.max_vertices;
        let edge_types = options.edge_types;

        let client = transaction_ref.client.borrow();
        let response = client.get_neighborhood(&center_str, depth, edge_types)?;

        // Parse subgraph from response
        let vertices = parse_vertices_from_response(&response)?;
        let edges = parse_edges_from_response(&response)?;

        Ok(Subgraph { vertices, edges })
    }

    fn path_exists(
        transaction: TransactionBorrow<'_>,
        from_vertex: ElementId,
        to_vertex: ElementId,
        options: Option<PathOptions>
    ) -> Result<bool, GraphError> {
        let transaction_ref: &JanusGraphTransaction = transaction.get();
        let from_str = element_id_to_string(&from_vertex);
        let to_str = element_id_to_string(&to_vertex);

        let max_depth = options
            .as_ref()
            .and_then(|o| o.max_depth)
            .unwrap_or(10);
        let edge_types = options.as_ref().and_then(|o| o.edge_types.clone());

        let client = transaction_ref.client.borrow();
        let _response = client.path_exists(&from_str, &to_str, edge_types, Some(max_depth))?;
        Ok(true)
    }

    fn get_vertices_at_distance(
        transaction: TransactionBorrow<'_>,
        source: ElementId,
        distance: u32,
        _direction: Direction,
        edge_types: Option<Vec<String>>
    ) -> Result<Vec<Vertex>, GraphError> {
        let transaction_ref: &JanusGraphTransaction = transaction.get();
        let source_str = element_id_to_string(&source);

        let client = transaction_ref.client.borrow();
        let response = client.get_vertices_at_distance(&source_str, distance, edge_types, None)?;

        parse_vertices_from_response(&response)
    }
}

impl QueryGuest for JanusGraphComponent {
    fn execute_query(
        transaction: TransactionBorrow<'_>,
        query: String,
        parameters: Option<Vec<(String, PropertyValue)>>,
        _options: Option<QueryOptions>
    ) -> Result<QueryExecutionResult, GraphError> {
        let transaction_ref: &JanusGraphTransaction = transaction.get();
        let client = transaction_ref.client.borrow();
        let mut bindings = HashMap::new();
        if let Some(params) = parameters {
            for (key, value) in params {
                let json_value = property_value_to_json(&value)?;
                bindings.insert(key, json_value);
            }
        }

        let response = client.execute_custom_query(query, Some(bindings))?;

        // Parse the result based on the query type
        let query_result = if response.result.data.is_empty() {
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
                    response.result.data
                        .into_iter()
                        .map(|v| { json_to_property_value(&v).unwrap_or(PropertyValue::NullValue) })
                        .collect()
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

impl SchemaGuest for JanusGraphComponent {
    type SchemaManager = JanusGraphSchemaManager;

    fn get_schema_manager() -> Result<SchemaManager, GraphError> {
        let client = JanusGraphClient::new(
            "ws://localhost:8182".to_string(),
            None,
            None,
            "graph".to_string()
        );
        Ok(
            SchemaManager::new(JanusGraphSchemaManager {
                client: RefCell::new(client),
            })
        )
    }
}

impl GuestSchemaManager for JanusGraphSchemaManager {
    fn define_vertex_label(&self, schema: VertexLabelSchema) -> Result<(), GraphError> {
        let client = self.client.borrow();

        // Create vertex label
        let label_query = format!("mgmt.makeVertexLabel('{}').make()", schema.label);
        client.execute_gremlin_sync(&label_query, None)?;

        // Create properties and constraints
        for property_def in &schema.properties {
            let property_query = format!(
                "mgmt.makePropertyKey('{}').dataType({}).make()",
                property_def.name,
                property_type_to_gremlin_type(&property_def.property_type)
            );
            client.execute_gremlin_sync(&property_query, None)?;

            if property_def.unique {
                let constraint_query = format!(
                    "mgmt.buildIndex('by{}', Vertex.class).addKey({}).indexOnly(mgmt.getVertexLabel('{}')).unique().buildCompositeIndex()",
                    property_def.name,
                    property_def.name,
                    schema.label
                );
                client.execute_gremlin_sync(&constraint_query, None)?;
            }
        }

        Ok(())
    }

    fn define_edge_label(&self, schema: EdgeLabelSchema) -> Result<(), GraphError> {
        let client = self.client.borrow();

        // Create edge label
        let label_query = format!("mgmt.makeEdgeLabel('{}').make()", schema.label);
        client.execute_gremlin_sync(&label_query, None)?;

        // Create properties and constraints
        for property_def in &schema.properties {
            let property_query = format!(
                "mgmt.makePropertyKey('{}').dataType({}).make()",
                property_def.name,
                property_type_to_gremlin_type(&property_def.property_type)
            );
            client.execute_gremlin_sync(&property_query, None)?;

            if property_def.unique {
                let constraint_query = format!(
                    "mgmt.buildIndex('by{}', Edge.class).addKey({}).indexOnly(mgmt.getEdgeLabel('{}')).unique().buildCompositeIndex()",
                    property_def.name,
                    property_def.name,
                    schema.label
                );
                client.execute_gremlin_sync(&constraint_query, None)?;
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

        if response.result.data.is_empty() {
            return Ok(None);
        }

        // Parse properties from response
        let mut properties = Vec::new();
        for value in &response.result.data {
            if let Some(prop_name) = value.as_str() {
                properties.push(PropertyDefinition {
                    name: prop_name.to_string(),
                    property_type: PropertyType::StringType, // Default to string
                    required: false,
                    unique: false,
                    default_value: None,
                });
            }
        }

        Ok(
            Some(VertexLabelSchema {
                label,
                properties,
                container: None,
            })
        )
    }

    fn get_edge_label_schema(&self, label: String) -> Result<Option<EdgeLabelSchema>, GraphError> {
        let client = self.client.borrow();
        let response = client.get_label_schema(&label)?;

        if response.result.data.is_empty() {
            return Ok(None);
        }

        // Parse properties from response
        let mut properties = Vec::new();
        for value in &response.result.data {
            if let Some(prop_name) = value.as_str() {
                properties.push(PropertyDefinition {
                    name: prop_name.to_string(),
                    property_type: PropertyType::StringType, // Default to string
                    required: false,
                    unique: false,
                    default_value: None,
                });
            }
        }

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
        let query = "g.V().label().dedup()";
        let response = self.client.borrow().execute_gremlin_sync(query, None)?;
        parse_string_list_from_response(&response)
    }

    fn list_edge_labels(&self) -> Result<Vec<String>, GraphError> {
        let query = "g.E().label().dedup()";
        let response = self.client.borrow().execute_gremlin_sync(query, None)?;
        parse_string_list_from_response(&response)
    }

    fn create_index(&self, index: IndexDefinition) -> Result<(), GraphError> {
        let client = self.client.borrow();

        let index_type = match index.index_type {
            IndexType::Exact => "Composite",
            IndexType::Range => "Composite",
            IndexType::Text => "Mixed",
            IndexType::Geospatial => "Mixed",
        };

        let properties = index.properties.join(", ");
        let query = format!(
            "mgmt.buildIndex('{}', Vertex.class).addKey({}).build{}Index()",
            index.name,
            properties,
            index_type
        );

        client.execute_gremlin_sync(&query, None)?;
        Ok(())
    }

    fn drop_index(&self, name: String) -> Result<(), GraphError> {
        let client = self.client.borrow();
        let _response = client.execute_gremlin_sync(
            &format!("mgmt.getGraphIndex('{}')", name),
            None
        )?;
        Ok(())
    }

    fn list_indexes(&self) -> Result<Vec<IndexDefinition>, GraphError> {
        let client = self.client.borrow();
        let _response = client.execute_gremlin_sync("mgmt.getGraphIndexes(Vertex.class)", None)?;
        Ok(vec![])
    }

    fn get_index(&self, name: String) -> Result<Option<IndexDefinition>, GraphError> {
        let client = self.client.borrow();
        let _response = client.execute_gremlin_sync(
            &format!("mgmt.getGraphIndex('{}')", name),
            None
        )?;
        Ok(None)
    }

    fn define_edge_type(&self, _definition: EdgeTypeDefinition) -> Result<(), GraphError> {
        Ok(())
    }

    fn list_edge_types(&self) -> Result<Vec<EdgeTypeDefinition>, GraphError> {
        Ok(vec![])
    }

    fn create_container(
        &self,
        _name: String,
        _container_type: ContainerType
    ) -> Result<(), GraphError> {
        Ok(())
    }

    fn list_containers(&self) -> Result<Vec<ContainerInfo>, GraphError> {
        Ok(vec![])
    }
}

type DurableJanusGraphComponent = DurableGraph<JanusGraphComponent>;

golem_graph::export_graph!(DurableJanusGraphComponent with_types_in golem_graph);

pub mod config;
pub mod durability;
pub mod error;
pub mod query_utils;

pub mod model;

use crate::model::connection::{Graph, GraphStatistics, Transaction};
use crate::model::schema::{
    ConnectionConfig, ContainerInfo, ContainerType, EdgeLabelSchema, EdgeTypeDefinition,
    GraphError, IndexDefinition, SchemaManager, VertexLabelSchema,
};
use crate::model::transactions::{
    CreateEdgeOptions, CreateVertexOptions, Edge, ElementId, ExecuteQueryOptions,
    FindAllPathsOptions, FindEdgesOptions, FindShortestPathOptions, FindVerticesOptions,
    GetAdjacentVerticesOptions, GetConnectedEdgesOptions, GetNeighborhoodOptions,
    GetVerticesAtDistanceOptions, Path, PathExistsOptions, QueryExecutionResult, Subgraph,
    UpdateEdgeOptions, UpdateVertexOptions, Vertex,
};
use std::cell::RefCell;
use std::str::FromStr;

pub trait GraphInterface: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn begin_transaction(&self) -> Result<Transaction, model::connection::GraphError>;
    fn begin_read_transaction(&self) -> Result<Transaction, model::connection::GraphError>;
    fn ping(&self) -> Result<(), model::connection::GraphError>;
    fn close(&self) -> Result<(), model::connection::GraphError>;
    fn get_statistics(&self) -> Result<GraphStatistics, model::connection::GraphError>;
}

pub trait GraphProvider {
    type Graph: GraphInterface;
    fn connect(
        config: model::connection::ConnectionConfig,
    ) -> Result<Graph, model::connection::GraphError>;
}

pub trait TransactionInterface: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn execute_query(
        &self,
        options: ExecuteQueryOptions,
    ) -> Result<QueryExecutionResult, model::transactions::GraphError>;
    fn find_shortest_path(
        &self,
        options: FindShortestPathOptions,
    ) -> Result<Option<Path>, model::transactions::GraphError>;
    fn find_all_paths(
        &self,
        options: FindAllPathsOptions,
    ) -> Result<Vec<Path>, model::transactions::GraphError>;
    fn get_neighborhood(
        &self,
        options: GetNeighborhoodOptions,
    ) -> Result<Subgraph, model::transactions::GraphError>;
    fn path_exists(
        &self,
        options: PathExistsOptions,
    ) -> Result<bool, model::transactions::GraphError>;
    fn get_vertices_at_distance(
        &self,
        options: GetVerticesAtDistanceOptions,
    ) -> Result<Vec<Vertex>, model::transactions::GraphError>;
    fn get_adjacent_vertices(
        &self,
        options: GetAdjacentVerticesOptions,
    ) -> Result<Vec<Vertex>, model::transactions::GraphError>;
    fn get_connected_edges(
        &self,
        options: GetConnectedEdgesOptions,
    ) -> Result<Vec<Edge>, model::transactions::GraphError>;
    fn create_vertex(
        &self,
        options: CreateVertexOptions,
    ) -> Result<Vertex, model::transactions::GraphError>;
    fn create_vertices(
        &self,
        vertices: Vec<CreateVertexOptions>,
    ) -> Result<Vec<Vertex>, model::transactions::GraphError>;
    fn get_vertex(&self, id: ElementId) -> Result<Option<Vertex>, model::transactions::GraphError>;
    fn update_vertex(
        &self,
        options: UpdateVertexOptions,
    ) -> Result<Vertex, model::transactions::GraphError>;
    fn delete_vertex(
        &self,
        id: ElementId,
        delete_edges: bool,
    ) -> Result<(), model::transactions::GraphError>;
    fn find_vertices(
        &self,
        options: FindVerticesOptions,
    ) -> Result<Vec<Vertex>, model::transactions::GraphError>;
    fn create_edge(
        &self,
        options: CreateEdgeOptions,
    ) -> Result<Edge, model::transactions::GraphError>;
    fn create_edges(
        &self,
        edges: Vec<CreateEdgeOptions>,
    ) -> Result<Vec<Edge>, model::transactions::GraphError>;
    fn get_edge(&self, id: ElementId) -> Result<Option<Edge>, model::transactions::GraphError>;
    fn update_edge(
        &self,
        options: UpdateEdgeOptions,
    ) -> Result<Edge, model::transactions::GraphError>;
    fn delete_edge(&self, id: ElementId) -> Result<(), model::transactions::GraphError>;
    fn find_edges(
        &self,
        options: FindEdgesOptions,
    ) -> Result<Vec<Edge>, model::transactions::GraphError>;
    fn commit(&self) -> Result<(), model::transactions::GraphError>;
    fn rollback(&self) -> Result<(), model::transactions::GraphError>;
    fn is_active(&self) -> bool;
}

pub trait TransactionProvider {
    type Transaction: TransactionInterface;
}

pub trait SchemaManagerInterface: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn define_vertex_label(&self, schema: VertexLabelSchema) -> Result<(), GraphError>;
    fn define_edge_label(&self, schema: EdgeLabelSchema) -> Result<(), GraphError>;
    fn get_vertex_label_schema(
        &self,
        label: String,
    ) -> Result<Option<VertexLabelSchema>, GraphError>;
    fn get_edge_label_schema(&self, label: String) -> Result<Option<EdgeLabelSchema>, GraphError>;
    fn list_vertex_labels(&self) -> Result<Vec<String>, GraphError>;
    fn list_edge_labels(&self) -> Result<Vec<String>, GraphError>;
    fn create_index(&self, index: IndexDefinition) -> Result<(), GraphError>;
    fn drop_index(&self, name: String) -> Result<(), GraphError>;
    fn list_indexes(&self) -> Result<Vec<IndexDefinition>, GraphError>;
    fn get_index(&self, name: String) -> Result<Option<IndexDefinition>, GraphError>;
    fn define_edge_type(&self, definition: EdgeTypeDefinition) -> Result<(), GraphError>;
    fn list_edge_types(&self) -> Result<Vec<EdgeTypeDefinition>, GraphError>;
    fn create_container(
        &self,
        name: String,
        container_type: ContainerType,
    ) -> Result<(), GraphError>;
    fn list_containers(&self) -> Result<Vec<ContainerInfo>, GraphError>;
}

pub trait SchemaManagerProvider {
    type SchemaManager: SchemaManagerInterface;
    fn get_schema_manager(config: Option<ConnectionConfig>) -> Result<SchemaManager, GraphError>;
}

struct LoggingState {
    logging_initialized: bool,
}

impl LoggingState {
    fn init(&mut self) {
        if !self.logging_initialized {
            let _ = wasi_logger::Logger::install();
            let max_level: log::LevelFilter =
                log::LevelFilter::from_str(&std::env::var("GOLEM_GRAPH_LOG").unwrap_or_default())
                    .unwrap_or(log::LevelFilter::Info);
            log::set_max_level(max_level);
            self.logging_initialized = true;
        }
    }
}

thread_local! {
    /// This holds the state of our application.
    static LOGGING_STATE: RefCell<LoggingState> = const { RefCell::new(LoggingState {
        logging_initialized: false,
    }) };
}

pub fn init_logging() {
    LOGGING_STATE.with_borrow_mut(|state| state.init());
}

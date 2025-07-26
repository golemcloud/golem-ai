// Required traits for ExtendedGraphGuest
use crate::exports::golem::graph::connection::{
    ConnectionConfig,
    Guest as ConnectionGuest,
    GuestGraph,
};
use crate::exports::golem::graph::traversal::Guest as TraversalGuest;
use crate::exports::golem::graph::query::Guest as QueryGuest;
use crate::exports::golem::graph::schema::Guest as SchemaGuest;
use crate::exports::golem::graph::errors::GraphError;
use crate::exports::golem::graph::transactions::{ GuestTransaction, TransactionBorrow };
use crate::exports::golem::graph::schema::GuestSchemaManager;
use golem_rust::value_and_type::{ FromValueAndType, IntoValue };
use std::marker::PhantomData;

/// Wraps a graph implementation with custom durability
pub struct DurableGraph<Impl> {
    phantom: PhantomData<Impl>,
}

/// Trait to be implemented in addition to the graph `Guest` traits when wrapping with `DurableGraph`.
pub trait ExtendedGraphGuest: ConnectionGuest +
    GuestGraph +
    TraversalGuest +
    QueryGuest +
    SchemaGuest +
    Clone +
    'static
{
    type ReplayState: std::fmt::Debug + Clone + IntoValue + FromValueAndType;
    type Transaction: GuestTransaction;
    type SchemaManager: GuestSchemaManager;

    /// Creates an instance of the graph without wrapping it in a `Resource`
    fn unwrapped_graph(config: ConnectionConfig) -> Result<Self, GraphError> where Self: Sized;

    /// Used at the end of replay to go from replay to live mode
    fn graph_to_state(graph: &Self) -> Self::ReplayState;
    fn graph_from_state(
        state: &Self::ReplayState,
        config: ConnectionConfig
    ) -> Result<Self, GraphError>
        where Self: Sized;

    /// Creates an instance of the transaction without wrapping it in a `Resource`
    fn unwrapped_transaction(
        graph: &Self,
        read_only: bool
    ) -> Result<Self::Transaction, GraphError>;

    /// Used at the end of replay to go from replay to live mode for transactions
    fn transaction_to_state(transaction: &Self::Transaction) -> Self::ReplayState;
    fn transaction_from_state(
        state: &Self::ReplayState,
        graph: &Self,
        read_only: bool
    ) -> Result<Self::Transaction, GraphError>;

    fn schema_manager_to_state(
        schema_manager: &<Self as ExtendedGraphGuest>::SchemaManager
    ) -> Self::ReplayState;
    fn schema_manager_from_state(
        state: &Self::ReplayState
    ) -> Result<<Self as ExtendedGraphGuest>::SchemaManager, GraphError>;
}

/// When the durability feature flag is off, wrapping with `DurableGraph` is just a passthrough
#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use crate::durability::{ DurableGraph, ExtendedGraphGuest, TransactionBorrow };
    use crate::exports::golem::graph::connection::{
        ConnectionConfig,
        Guest as ConnectionGuest,
        Graph,
    };
    use crate::exports::golem::graph::transactions::{ Guest as TransactionGuest };
    use crate::exports::golem::graph::schema::{ Guest as SchemaGuest, SchemaManager };
    use crate::exports::golem::graph::query::{
        Guest as QueryGuest,
        QueryExecutionResult,
        QueryOptions,
    };
    use crate::exports::golem::graph::traversal::{
        Guest as TraversalGuest,
        Path,
        PathOptions,
        Subgraph,
        NeighborhoodOptions,
    };
    use crate::exports::golem::graph::types::{ ElementId, Direction, PropertyValue, Vertex };
    use crate::exports::golem::graph::errors::GraphError;

    impl<Impl: ExtendedGraphGuest> ConnectionGuest for DurableGraph<Impl> {
        type Graph = Impl;

        fn connect(config: ConnectionConfig) -> Result<Graph, GraphError> {
            let graph = Impl::unwrapped_graph(config)?;
            Ok(Graph::new(graph))
        }
    }

    impl<Impl: ExtendedGraphGuest> TransactionGuest for DurableGraph<Impl> {
        type Transaction = Impl::Transaction;
    }

    impl<Impl: ExtendedGraphGuest> SchemaGuest for DurableGraph<Impl> {
        type SchemaManager = <Impl as ExtendedGraphGuest>::SchemaManager;

        fn get_schema_manager() -> Result<SchemaManager, GraphError> {
            let schema_manager = Impl::get_schema_manager()?;
            Ok(schema_manager)
        }
    }

    impl<Impl: ExtendedGraphGuest> TraversalGuest for DurableGraph<Impl> {
        fn find_shortest_path(
            transaction: TransactionBorrow<'_>,
            from_vertex: ElementId,
            to_vertex: ElementId,
            options: Option<PathOptions>
        ) -> Result<Option<Path>, GraphError> {
            Impl::find_shortest_path(transaction, from_vertex, to_vertex, options)
        }

        fn find_all_paths(
            transaction: TransactionBorrow<'_>,
            from_vertex: ElementId,
            to_vertex: ElementId,
            options: Option<PathOptions>,
            limit: Option<u32>
        ) -> Result<Vec<Path>, GraphError> {
            Impl::find_all_paths(transaction, from_vertex, to_vertex, options, limit)
        }

        fn get_neighborhood(
            transaction: TransactionBorrow<'_>,
            center: ElementId,
            options: NeighborhoodOptions
        ) -> Result<Subgraph, GraphError> {
            Impl::get_neighborhood(transaction, center, options)
        }

        fn path_exists(
            transaction: TransactionBorrow<'_>,
            from_vertex: ElementId,
            to_vertex: ElementId,
            options: Option<PathOptions>
        ) -> Result<bool, GraphError> {
            Impl::path_exists(transaction, from_vertex, to_vertex, options)
        }

        fn get_vertices_at_distance(
            transaction: TransactionBorrow<'_>,
            source: ElementId,
            distance: u32,
            direction: Direction,
            edge_types: Option<Vec<String>>
        ) -> Result<Vec<Vertex>, GraphError> {
            Impl::get_vertices_at_distance(transaction, source, distance, direction, edge_types)
        }
    }

    impl<Impl: ExtendedGraphGuest> QueryGuest for DurableGraph<Impl> {
        fn execute_query(
            transaction: TransactionBorrow<'_>,
            query: String,
            parameters: Option<Vec<(String, PropertyValue)>>,
            options: Option<QueryOptions>
        ) -> Result<QueryExecutionResult, GraphError> {
            Impl::execute_query(transaction, query, parameters, options)
        }
    }
}

/// When the durability feature flag is on, wrapping with `DurableGraph` adds custom durability
/// on top of the provider-specific graph implementation using Golem's special host functions and
/// the `golem-rust` helper library.
///
/// There will be custom durability entries saved in the oplog, with the full graph request and configuration
/// stored as input, and the full response stored as output. To serialize these in a way it is
/// observable by oplog consumers, each relevant data type has to be converted to/from `ValueAndType`
/// which is implemented using the type classes and builder in the `golem-rust` library.
#[cfg(feature = "durability")]
mod durable_impl {
    use crate::durability::{ DurableGraph, ExtendedGraphGuest };
    use crate::exports::golem::graph::connection::{
        ConnectionConfig,
        Guest as ConnectionGuest,
        Graph,
        GuestGraph,
        GraphStatistics,
    };
    use crate::exports::golem::graph::transactions::{
        Guest as TransactionGuest,
        GuestTransaction,
        Transaction,
        TransactionBorrow,
    };
    use crate::exports::golem::graph::schema::{
        Guest as SchemaGuest,
        GuestSchemaManager,
        SchemaManager,
    };
    use crate::exports::golem::graph::query::{
        Guest as QueryGuest,
        QueryExecutionResult,
        QueryOptions,
    };
    use crate::exports::golem::graph::traversal::{
        Guest as TraversalGuest,
        Path,
        PathOptions,
        Subgraph,
        NeighborhoodOptions,
    };
    use crate::exports::golem::graph::types::{
        ElementId,
        Direction,
        PropertyValue,
        PropertyMap,
        Edge,
        Vertex,
        FilterCondition,
        SortSpec,
    };
    use crate::exports::golem::graph::errors::GraphError;
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    use golem_rust::durability::Durability;
    use golem_rust::{ with_persistence_level, PersistenceLevel };
    use golem_rust::value_and_type::{ FromValueAndType, IntoValue };
    use std::cell::RefCell;
    use crate::exports::golem::graph::transactions::{ VertexSpec, EdgeSpec };
    use crate::exports::golem::graph::schema::{
        VertexLabelSchema,
        EdgeLabelSchema,
        IndexDefinition,
        EdgeTypeDefinition,
        ContainerType,
        ContainerInfo,
    };

    #[derive(Debug, Clone, IntoValue)]
    struct ConnectInput {
        config: ConnectionConfig,
    }

    #[derive(Debug, Clone, IntoValue)]
    struct NoInput;

    #[derive(Debug, Clone, IntoValue)]
    struct NoOutput;

    #[derive(Debug, Clone, IntoValue)]
    struct CreateVertexInput {
        vertex_type: String,
        properties: PropertyMap,
    }

    impl FromValueAndType for NoInput {
        fn from_value_and_type(
            _: (golem_rust::value_and_type::Value, golem_rust::value_and_type::Type)
        ) -> Result<Self, String> {
            Ok(NoInput)
        }
        fn from_extractor<'a, 'b>(
            _: &'a impl golem_rust::value_and_type::WitValueExtractor<'a, 'b>
        ) -> Result<Self, String> {
            Ok(NoInput)
        }
    }

    impl FromValueAndType for NoOutput {
        fn from_value_and_type(
            _: (golem_rust::value_and_type::Value, golem_rust::value_and_type::Type)
        ) -> Result<Self, String> {
            Ok(NoOutput)
        }
        fn from_extractor<'a, 'b>(
            _: &'a impl golem_rust::value_and_type::WitValueExtractor<'a, 'b>
        ) -> Result<Self, String> {
            Ok(NoOutput)
        }
    }

    impl FromValueAndType for ConnectInput {
        fn from_value_and_type(
            _: (golem_rust::value_and_type::Value, golem_rust::value_and_type::Type)
        ) -> Result<Self, String> {
            Ok(ConnectInput {
                config: ConnectionConfig {
                    host: "localhost".to_string(),
                    port: Some(7687),
                    database_name: Some("neo4j".to_string()),
                    username: Some("neo4j".to_string()),
                    password: Some("password".to_string()),
                    timeout_seconds: Some(30),
                    max_connections: Some(10),
                    provider_config: vec![],
                },
            })
        }
        fn from_extractor<'a, 'b>(
            _: &'a impl golem_rust::value_and_type::WitValueExtractor<'a, 'b>
        ) -> Result<Self, String> {
            Ok(ConnectInput {
                config: ConnectionConfig {
                    host: "localhost".to_string(),
                    port: Some(7687),
                    database_name: Some("neo4j".to_string()),
                    username: Some("neo4j".to_string()),
                    password: Some("password".to_string()),
                    timeout_seconds: Some(30),
                    max_connections: Some(10),
                    provider_config: vec![],
                },
            })
        }
    }

    impl From<ConnectInput> for NoOutput {
        fn from(_: ConnectInput) -> Self {
            NoOutput
        }
    }

    impl From<NoInput> for NoOutput {
        fn from(_: NoInput) -> Self {
            NoOutput
        }
    }

    impl FromValueAndType for CreateVertexInput {
        fn from_value_and_type(
            _: (golem_rust::value_and_type::Value, golem_rust::value_and_type::Type)
        ) -> Result<Self, String> {
            Ok(CreateVertexInput {
                vertex_type: "".to_string(),
                properties: PropertyMap::new(),
            })
        }
        fn from_extractor<'a, 'b>(
            _: &'a impl golem_rust::value_and_type::WitValueExtractor<'a, 'b>
        ) -> Result<Self, String> {
            Ok(CreateVertexInput {
                vertex_type: "".to_string(),
                properties: PropertyMap::new(),
            })
        }
    }

    impl From<CreateVertexInput> for Vertex {
        fn from(input: CreateVertexInput) -> Self {
            Vertex {
                id: ElementId::Int64(0),
                vertex_type: input.vertex_type,
                additional_labels: vec![],
                properties: input.properties,
            }
        }
    }

    impl From<NoInput> for () {
        fn from(_: NoInput) -> Self {
            ()
        }
    }
    impl From<&GraphError> for GraphError {
        fn from(error: &GraphError) -> Self {
            error.clone()
        }
    }

    impl<Impl: ExtendedGraphGuest> ConnectionGuest for DurableGraph<Impl> {
        type Graph = DurableGraphInstance<Impl>;

        fn connect(config: ConnectionConfig) -> Result<Graph, GraphError> {
            let durability = Durability::<ConnectInput, GraphError>::new(
                "golem_graph",
                "connect",
                DurableFunctionType::WriteRemote
            );

            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::unwrapped_graph(config.clone())
                });

                match &result {
                    Ok(graph) => {
                        let _ = durability.persist(ConnectInput { config }, Ok(graph.clone()));
                        Ok(Graph::new(DurableGraphInstance::live(graph.clone(), config)))
                    }
                    Err(e) => {
                        let _ = durability.persist(ConnectInput { config }, Err(e.clone()));
                        Err(e.clone())
                    }
                }
            } else {
                match durability.replay::<NoOutput, GraphError>() {
                    Ok(_) => {
                        let graph = Impl::unwrapped_graph(config)?;
                        Ok(Graph::new(DurableGraphInstance::live(graph, config)))
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    impl<Impl: ExtendedGraphGuest> TransactionGuest for DurableGraph<Impl> {
        type Transaction = DurableTransactionInstance<Impl>;
    }

    impl<Impl: ExtendedGraphGuest> SchemaGuest for DurableGraph<Impl> {
        type SchemaManager = DurableSchemaManagerInstance<Impl>;

        fn get_schema_manager() -> Result<SchemaManager, GraphError> {
            let durability = Durability::<NoInput, GraphError>::new(
                "golem_graph_schema",
                "get_schema_manager",
                DurableFunctionType::WriteRemote
            );

            if durability.is_live() {
                let result = Impl::get_schema_manager();
                match &result {
                    Ok(schema_manager) => {
                        let _ = durability.persist(NoInput, Ok(schema_manager.clone()));
                        Ok(
                            SchemaManager::new(
                                DurableSchemaManagerInstance::new(schema_manager.clone())
                            )
                        )
                    }
                    Err(e) => {
                        let _ = durability.persist(NoInput, Err(e.clone()));
                        Err(e.clone())
                    }
                }
            } else {
                match durability.replay::<NoOutput, GraphError>() {
                    Ok(_) => {
                        let schema_manager = Impl::get_schema_manager()?;
                        Ok(SchemaManager::new(DurableSchemaManagerInstance::new(schema_manager)))
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    impl<Impl: ExtendedGraphGuest> TraversalGuest for DurableGraph<Impl> {
        fn find_shortest_path(
            transaction: TransactionBorrow<'_>,
            from_vertex: ElementId,
            to_vertex: ElementId,
            options: Option<PathOptions>
        ) -> Result<Option<Path>, GraphError> {
            Impl::find_shortest_path(transaction, from_vertex, to_vertex, options)
        }

        fn find_all_paths(
            transaction: TransactionBorrow<'_>,
            from_vertex: ElementId,
            to_vertex: ElementId,
            options: Option<PathOptions>,
            limit: Option<u32>
        ) -> Result<Vec<Path>, GraphError> {
            Impl::find_all_paths(transaction, from_vertex, to_vertex, options, limit)
        }

        fn get_neighborhood(
            transaction: TransactionBorrow<'_>,
            center: ElementId,
            options: NeighborhoodOptions
        ) -> Result<Subgraph, GraphError> {
            Impl::get_neighborhood(transaction, center, options)
        }

        fn path_exists(
            transaction: TransactionBorrow<'_>,
            from_vertex: ElementId,
            to_vertex: ElementId,
            options: Option<PathOptions>
        ) -> Result<bool, GraphError> {
            Impl::path_exists(transaction, from_vertex, to_vertex, options)
        }

        fn get_vertices_at_distance(
            transaction: TransactionBorrow<'_>,
            source: ElementId,
            distance: u32,
            direction: Direction,
            edge_types: Option<Vec<String>>
        ) -> Result<Vec<Vertex>, GraphError> {
            Impl::get_vertices_at_distance(transaction, source, distance, direction, edge_types)
        }
    }

    impl<Impl: ExtendedGraphGuest> QueryGuest for DurableGraph<Impl> {
        fn execute_query(
            transaction: TransactionBorrow<'_>,
            query: String,
            parameters: Option<Vec<(String, PropertyValue)>>,
            options: Option<QueryOptions>
        ) -> Result<QueryExecutionResult, GraphError> {
            Impl::execute_query(transaction, query, parameters, options)
        }
    }

    // Durable resource wrappers
    pub struct DurableGraphInstance<Impl: ExtendedGraphGuest> {
        graph: Impl,
        config: ConnectionConfig,
        state: Option<DurableGraphState<Impl>>,
    }

    impl<Impl: ExtendedGraphGuest> DurableGraphInstance<Impl> {
        fn live(graph: Impl, config: ConnectionConfig) -> Self {
            let graph_clone = graph.clone();
            Self {
                graph,
                config,
                state: Some(DurableGraphState::Live { graph: graph_clone }),
            }
        }
    }

    impl<Impl: ExtendedGraphGuest> Drop for DurableGraphInstance<Impl> {
        fn drop(&mut self) {
            if let Some(DurableGraphState::Live { graph }) = self.state.take() {
                with_persistence_level(PersistenceLevel::PersistNothing, move || drop(graph));
            }
        }
    }

    impl<Impl: ExtendedGraphGuest> GuestGraph for DurableGraphInstance<Impl> {
        fn begin_transaction(&self) -> Result<Transaction, GraphError> {
            let durability = Durability::<NoInput, GraphError>::new(
                "golem_graph_transaction",
                "begin_transaction",
                DurableFunctionType::WriteRemote
            );

            if durability.is_live() {
                let result = self.graph.begin_transaction();
                match &result {
                    Ok(transaction) => {
                        let _ = durability.persist(NoInput, Ok(transaction.clone()));
                        Ok(
                            Transaction::new(
                                DurableTransactionInstance::live(
                                    transaction.clone(),
                                    self.graph.clone(),
                                    false
                                )
                            )
                        )
                    }
                    Err(e) => {
                        let _ = durability.persist(NoInput, Err(e.clone()));
                        Err(e.clone())
                    }
                }
            } else {
                match durability.replay::<NoOutput, GraphError>() {
                    Ok(_) => {
                        let transaction = self.graph.begin_transaction()?;
                        Ok(
                            Transaction::new(
                                DurableTransactionInstance::live(
                                    transaction,
                                    self.graph.clone(),
                                    false
                                )
                            )
                        )
                    }
                    Err(e) => Err(e),
                }
            }
        }

        fn begin_read_transaction(&self) -> Result<Transaction, GraphError> {
            let durability = Durability::<NoInput, GraphError>::new(
                "golem_graph_transaction",
                "begin_read_transaction",
                DurableFunctionType::ReadRemote
            );

            if durability.is_live() {
                let result = self.graph.begin_read_transaction();
                match &result {
                    Ok(transaction) => {
                        let _ = durability.persist(NoInput, Ok(transaction.clone()));
                        Ok(
                            Transaction::new(
                                DurableTransactionInstance::live(
                                    transaction.clone(),
                                    self.graph.clone(),
                                    true
                                )
                            )
                        )
                    }
                    Err(e) => {
                        let _ = durability.persist(NoInput, Err(e.clone()));
                        Err(e.clone())
                    }
                }
            } else {
                match durability.replay::<NoOutput, GraphError>() {
                    Ok(_) => {
                        let transaction = self.graph.begin_read_transaction()?;
                        Ok(
                            Transaction::new(
                                DurableTransactionInstance::live(
                                    transaction,
                                    self.graph.clone(),
                                    true
                                )
                            )
                        )
                    }
                    Err(e) => Err(e),
                }
            }
        }

        fn ping(&self) -> Result<(), GraphError> {
            self.graph.ping()
        }

        fn get_statistics(&self) -> Result<GraphStatistics, GraphError> {
            self.graph.get_statistics()
        }

        fn close(&self) -> Result<(), GraphError> {
            self.graph.close()
        }
    }

    pub struct DurableTransactionInstance<Impl: ExtendedGraphGuest> {
        transaction: <Impl as ExtendedGraphGuest>::Transaction,
        graph: Impl,
        read_only: bool,
    }

    impl<Impl: ExtendedGraphGuest> DurableTransactionInstance<Impl> {
        fn live(
            transaction: <Impl as ExtendedGraphGuest>::Transaction,
            graph: Impl,
            read_only: bool
        ) -> Self {
            Self {
                transaction,
                graph,
                read_only,
            }
        }
    }

    impl<Impl: ExtendedGraphGuest> GuestTransaction for DurableTransactionInstance<Impl> {
        fn commit(&self) -> Result<(), GraphError> {
            let durability = Durability::<NoInput, GraphError>::new(
                "golem_graph_transaction",
                "commit",
                DurableFunctionType::WriteRemote
            );

            if durability.is_live() {
                let result = self.transaction.commit();
                match &result {
                    Ok(_) => {
                        let _ = durability.persist(NoInput, Ok(()));
                        result
                    }
                    Err(e) => {
                        let _ = durability.persist(NoInput, Err(e.clone()));
                        result
                    }
                }
            } else {
                match durability.replay::<(), GraphError>() {
                    Ok(_) => self.transaction.commit(),
                    Err(e) => Err(e),
                }
            }
        }

        fn rollback(&self) -> Result<(), GraphError> {
            let durability = Durability::<NoInput, GraphError>::new(
                "golem_graph_transaction",
                "rollback",
                DurableFunctionType::WriteRemote
            );

            if durability.is_live() {
                let result = self.transaction.rollback();
                match &result {
                    Ok(_) => {
                        let _ = durability.persist(NoInput, Ok(()));
                        result
                    }
                    Err(e) => {
                        let _ = durability.persist(NoInput, Err(e.clone()));
                        result
                    }
                }
            } else {
                match durability.replay::<(), GraphError>() {
                    Ok(_) => self.transaction.rollback(),
                    Err(e) => Err(e),
                }
            }
        }

        // Vertex operations
        fn create_vertex(
            &self,
            vertex_type: String,
            properties: PropertyMap
        ) -> Result<Vertex, GraphError> {
            let durability = Durability::<CreateVertexInput, GraphError>::new(
                "golem_graph_transaction",
                "create_vertex",
                DurableFunctionType::WriteRemote
            );

            if durability.is_live() {
                let result = self.transaction.create_vertex(
                    vertex_type.clone(),
                    properties.clone()
                );
                match &result {
                    Ok(vertex) => {
                        let _ = durability.persist(
                            CreateVertexInput { vertex_type, properties },
                            Ok(vertex.clone())
                        );
                        result
                    }
                    Err(e) => {
                        let _ = durability.persist(
                            CreateVertexInput { vertex_type, properties },
                            Err(e.clone())
                        );
                        result
                    }
                }
            } else {
                match durability.replay::<Vertex, GraphError>() {
                    Ok(vertex) => Ok(vertex),
                    Err(e) => Err(e),
                }
            }
        }

        fn create_vertex_with_labels(
            &self,
            vertex_type: String,
            additional_labels: Vec<String>,
            properties: PropertyMap
        ) -> Result<Vertex, GraphError> {
            self.transaction.create_vertex_with_labels(vertex_type, additional_labels, properties)
        }

        fn get_vertex(&self, id: ElementId) -> Result<Option<Vertex>, GraphError> {
            self.transaction.get_vertex(id)
        }

        fn update_vertex(
            &self,
            id: ElementId,
            properties: PropertyMap
        ) -> Result<Vertex, GraphError> {
            self.transaction.update_vertex(id, properties)
        }

        fn update_vertex_properties(
            &self,
            id: ElementId,
            updates: PropertyMap
        ) -> Result<Vertex, GraphError> {
            self.transaction.update_vertex_properties(id, updates)
        }

        fn delete_vertex(&self, id: ElementId, delete_edges: bool) -> Result<(), GraphError> {
            self.transaction.delete_vertex(id, delete_edges)
        }

        fn find_vertices(
            &self,
            vertex_type: Option<String>,
            filters: Option<Vec<FilterCondition>>,
            sort: Option<Vec<SortSpec>>,
            limit: Option<u32>,
            offset: Option<u32>
        ) -> Result<Vec<Vertex>, GraphError> {
            self.transaction.find_vertices(vertex_type, filters, sort, limit, offset)
        }

        // Edge operations
        fn create_edge(
            &self,
            edge_type: String,
            from_vertex: ElementId,
            to_vertex: ElementId,
            properties: PropertyMap
        ) -> Result<Edge, GraphError> {
            self.transaction.create_edge(edge_type, from_vertex, to_vertex, properties)
        }

        fn get_edge(&self, id: ElementId) -> Result<Option<Edge>, GraphError> {
            self.transaction.get_edge(id)
        }

        fn update_edge(&self, id: ElementId, properties: PropertyMap) -> Result<Edge, GraphError> {
            self.transaction.update_edge(id, properties)
        }

        fn update_edge_properties(
            &self,
            id: ElementId,
            updates: PropertyMap
        ) -> Result<Edge, GraphError> {
            self.transaction.update_edge_properties(id, updates)
        }

        fn delete_edge(&self, id: ElementId) -> Result<(), GraphError> {
            self.transaction.delete_edge(id)
        }

        fn find_edges(
            &self,
            edge_types: Option<Vec<String>>,
            filters: Option<Vec<FilterCondition>>,
            sort: Option<Vec<SortSpec>>,
            limit: Option<u32>,
            offset: Option<u32>
        ) -> Result<Vec<Edge>, GraphError> {
            self.transaction.find_edges(edge_types, filters, sort, limit, offset)
        }

        // Traversal operations
        fn get_adjacent_vertices(
            &self,
            vertex_id: ElementId,
            direction: Direction,
            edge_types: Option<Vec<String>>,
            limit: Option<u32>
        ) -> Result<Vec<Vertex>, GraphError> {
            self.transaction.get_adjacent_vertices(vertex_id, direction, edge_types, limit)
        }

        fn get_connected_edges(
            &self,
            vertex_id: ElementId,
            direction: Direction,
            edge_types: Option<Vec<String>>,
            limit: Option<u32>
        ) -> Result<Vec<Edge>, GraphError> {
            self.transaction.get_connected_edges(vertex_id, direction, edge_types, limit)
        }

        // Batch operations
        fn create_vertices(&self, vertices: Vec<VertexSpec>) -> Result<Vec<Vertex>, GraphError> {
            self.transaction.create_vertices(vertices)
        }

        fn create_edges(&self, edges: Vec<EdgeSpec>) -> Result<Vec<Edge>, GraphError> {
            self.transaction.create_edges(edges)
        }

        fn upsert_vertex(
            &self,
            id: Option<ElementId>,
            vertex_type: String,
            properties: PropertyMap
        ) -> Result<Vertex, GraphError> {
            self.transaction.upsert_vertex(id, vertex_type, properties)
        }

        fn upsert_edge(
            &self,
            id: Option<ElementId>,
            edge_type: String,
            from_vertex: ElementId,
            to_vertex: ElementId,
            properties: PropertyMap
        ) -> Result<Edge, GraphError> {
            self.transaction.upsert_edge(id, edge_type, from_vertex, to_vertex, properties)
        }

        fn is_active(&self) -> bool {
            self.transaction.is_active()
        }
    }

    pub struct DurableSchemaManagerInstance<Impl: ExtendedGraphGuest> {
        schema_manager: <Impl as ExtendedGraphGuest>::SchemaManager,
    }

    impl<Impl: ExtendedGraphGuest> DurableSchemaManagerInstance<Impl> {
        fn new(schema_manager: <Impl as ExtendedGraphGuest>::SchemaManager) -> Self {
            Self { schema_manager }
        }
    }

    impl<Impl: ExtendedGraphGuest> GuestSchemaManager for DurableSchemaManagerInstance<Impl> {
        fn define_vertex_label(&self, schema: VertexLabelSchema) -> Result<(), GraphError> {
            self.schema_manager.define_vertex_label(schema)
        }

        fn define_edge_label(&self, schema: EdgeLabelSchema) -> Result<(), GraphError> {
            self.schema_manager.define_edge_label(schema)
        }

        fn get_vertex_label_schema(
            &self,
            label: String
        ) -> Result<Option<VertexLabelSchema>, GraphError> {
            self.schema_manager.get_vertex_label_schema(label)
        }

        fn get_edge_label_schema(
            &self,
            label: String
        ) -> Result<Option<EdgeLabelSchema>, GraphError> {
            self.schema_manager.get_edge_label_schema(label)
        }

        fn list_vertex_labels(&self) -> Result<Vec<String>, GraphError> {
            self.schema_manager.list_vertex_labels()
        }

        fn list_edge_labels(&self) -> Result<Vec<String>, GraphError> {
            self.schema_manager.list_edge_labels()
        }

        fn create_index(&self, index: IndexDefinition) -> Result<(), GraphError> {
            self.schema_manager.create_index(index)
        }

        fn drop_index(&self, name: String) -> Result<(), GraphError> {
            self.schema_manager.drop_index(name)
        }

        fn list_indexes(&self) -> Result<Vec<IndexDefinition>, GraphError> {
            self.schema_manager.list_indexes()
        }

        fn get_index(&self, name: String) -> Result<Option<IndexDefinition>, GraphError> {
            self.schema_manager.get_index(name)
        }

        fn define_edge_type(&self, definition: EdgeTypeDefinition) -> Result<(), GraphError> {
            self.schema_manager.define_edge_type(definition)
        }

        fn list_edge_types(&self) -> Result<Vec<EdgeTypeDefinition>, GraphError> {
            self.schema_manager.list_edge_types()
        }

        fn create_container(
            &self,
            name: String,
            container_type: ContainerType
        ) -> Result<(), GraphError> {
            self.schema_manager.create_container(name, container_type)
        }

        fn list_containers(&self) -> Result<Vec<ContainerInfo>, GraphError> {
            self.schema_manager.list_containers()
        }
    }

    #[derive(Debug, Clone)]
    enum DurableGraphState<Impl: ExtendedGraphGuest> {
        Live {
            graph: Impl,
        },
        Replay {
            state: <Impl as ExtendedGraphGuest>::ReplayState,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;

    fn roundtrip_test<T: Debug + Clone + PartialEq + IntoValue + FromValueAndType>(value: T) {
        let encoded = value.clone().into_value();
        let decoded = T::from_value_and_type((
            encoded,
            golem_rust::value_and_type::Type::Unit,
        )).unwrap();
        assert_eq!(value, decoded);
    }

    // Mock implementations for testing
    #[derive(Debug, Clone, PartialEq)]
    struct MockGraph {
        id: String,
        config: ConnectionConfig,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct MockTransaction {
        id: String,
        read_only: bool,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct MockSchemaManager {
        id: String,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct MockReplayState {
        graph_id: String,
        transaction_id: Option<String>,
        schema_manager_id: Option<String>,
    }

    impl IntoValue for MockGraph {
        fn into_value(self) -> golem_rust::value_and_type::Value {
            golem_rust::value_and_type::Value::Unit
        }
    }

    impl FromValueAndType for MockGraph {
        fn from_value_and_type(
            _: (golem_rust::value_and_type::Value, golem_rust::value_and_type::Type)
        ) -> Result<Self, String> {
            Ok(MockGraph { id: "mock".to_string(), config: ConnectionConfig::default() })
        }
        fn from_extractor<'a, 'b>(
            _: &'a impl golem_rust::value_and_type::WitValueExtractor<'a, 'b>
        ) -> Result<Self, String> {
            Ok(MockGraph { id: "mock".to_string(), config: ConnectionConfig::default() })
        }
    }

    impl IntoValue for MockTransaction {
        fn into_value(self) -> golem_rust::value_and_type::Value {
            golem_rust::value_and_type::Value::Unit
        }
    }

    impl FromValueAndType for MockTransaction {
        fn from_value_and_type(
            _: (golem_rust::value_and_type::Value, golem_rust::value_and_type::Type)
        ) -> Result<Self, String> {
            Ok(MockTransaction { id: "mock".to_string(), read_only: false })
        }
        fn from_extractor<'a, 'b>(
            _: &'a impl golem_rust::value_and_type::WitValueExtractor<'a, 'b>
        ) -> Result<Self, String> {
            Ok(MockTransaction { id: "mock".to_string(), read_only: false })
        }
    }

    impl IntoValue for MockSchemaManager {
        fn into_value(self) -> golem_rust::value_and_type::Value {
            golem_rust::value_and_type::Value::Unit
        }
    }

    impl FromValueAndType for MockSchemaManager {
        fn from_value_and_type(
            _: (golem_rust::value_and_type::Value, golem_rust::value_and_type::Type)
        ) -> Result<Self, String> {
            Ok(MockSchemaManager { id: "mock".to_string() })
        }
        fn from_extractor<'a, 'b>(
            _: &'a impl golem_rust::value_and_type::WitValueExtractor<'a, 'b>
        ) -> Result<Self, String> {
            Ok(MockSchemaManager { id: "mock".to_string() })
        }
    }

    impl IntoValue for MockReplayState {
        fn into_value(self) -> golem_rust::value_and_type::Value {
            golem_rust::value_and_type::Value::Unit
        }
    }

    impl FromValueAndType for MockReplayState {
        fn from_value_and_type(
            _: (golem_rust::value_and_type::Value, golem_rust::value_and_type::Type)
        ) -> Result<Self, String> {
            Ok(MockReplayState {
                graph_id: "mock".to_string(),
                transaction_id: None,
                schema_manager_id: None,
            })
        }
        fn from_extractor<'a, 'b>(
            _: &'a impl golem_rust::value_and_type::WitValueExtractor<'a, 'b>
        ) -> Result<Self, String> {
            Ok(MockReplayState {
                graph_id: "mock".to_string(),
                transaction_id: None,
                schema_manager_id: None,
            })
        }
    }

    // Basic roundtrip tests
    #[test]
    fn mock_replay_state_roundtrip() {
        let state = MockReplayState {
            graph_id: "test".to_string(),
            transaction_id: Some("tx1".to_string()),
            schema_manager_id: Some("sm1".to_string()),
        };
        roundtrip_test(state);
    }

    #[test]
    fn mock_graph_roundtrip() {
        let graph = MockGraph {
            id: "test".to_string(),
            config: ConnectionConfig::default(),
        };
        roundtrip_test(graph);
    }

    #[test]
    fn mock_transaction_roundtrip() {
        let transaction = MockTransaction {
            id: "test".to_string(),
            read_only: false,
        };
        roundtrip_test(transaction);
    }

    #[test]
    fn mock_schema_manager_roundtrip() {
        let schema_manager = MockSchemaManager {
            id: "test".to_string(),
        };
        roundtrip_test(schema_manager);
    }
}

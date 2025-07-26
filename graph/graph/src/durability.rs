use crate::exports::golem::graph::connection::{
    ConnectionConfig, Guest as ConnectionGuest, GuestGraph,
};
use crate::exports::golem::graph::errors::GraphError;
use crate::exports::golem::graph::query::Guest as QueryGuest;
use crate::exports::golem::graph::schema::{
    ContainerInfo, ContainerType, EdgeLabelSchema, EdgeTypeDefinition, Guest as SchemaGuest,
    GuestSchemaManager, IndexDefinition, VertexLabelSchema,
};
use crate::exports::golem::graph::transactions::GuestTransaction;
use crate::exports::golem::graph::traversal::Guest as TraversalGuest;
use golem_rust::value_and_type::{FromValueAndType, IntoValueAndType};
use std::collections::HashMap;
use std::marker::PhantomData;

// In-memory storage for schema information
#[derive(Debug, Clone, Default)]
struct SchemaStorage {
    vertex_schemas: HashMap<String, VertexLabelSchema>,
    edge_schemas: HashMap<String, EdgeLabelSchema>,
    indexes: HashMap<String, IndexDefinition>,
    edge_types: Vec<EdgeTypeDefinition>,
    containers: HashMap<String, ContainerInfo>,
}

#[derive(Debug)]
pub struct DurableSchemaManager {
    storage: SchemaStorage,
}

impl Default for DurableSchemaManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DurableSchemaManager {
    pub fn new() -> Self {
        Self {
            storage: SchemaStorage::default(),
        }
    }
}

impl Clone for DurableSchemaManager {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage.clone(),
        }
    }
}

impl GuestSchemaManager for DurableSchemaManager {
    fn define_vertex_label(&self, schema: VertexLabelSchema) -> Result<(), GraphError> {
        let mut storage = self.storage.clone();
        storage.vertex_schemas.insert(schema.label.clone(), schema);
        Ok(())
    }

    fn define_edge_label(&self, schema: EdgeLabelSchema) -> Result<(), GraphError> {
        let mut storage = self.storage.clone();
        storage.edge_schemas.insert(schema.label.clone(), schema);
        Ok(())
    }

    fn get_vertex_label_schema(
        &self,
        label: String,
    ) -> Result<Option<VertexLabelSchema>, GraphError> {
        Ok(self.storage.vertex_schemas.get(&label).cloned())
    }

    fn get_edge_label_schema(&self, label: String) -> Result<Option<EdgeLabelSchema>, GraphError> {
        Ok(self.storage.edge_schemas.get(&label).cloned())
    }

    fn list_vertex_labels(&self) -> Result<Vec<String>, GraphError> {
        Ok(self.storage.vertex_schemas.keys().cloned().collect())
    }

    fn list_edge_labels(&self) -> Result<Vec<String>, GraphError> {
        Ok(self.storage.edge_schemas.keys().cloned().collect())
    }

    fn create_index(&self, index: IndexDefinition) -> Result<(), GraphError> {
        let mut storage = self.storage.clone();

        // Check if index already exists
        if storage.indexes.contains_key(&index.name) {
            return Err(GraphError::DuplicateElement(
                crate::exports::golem::graph::types::ElementId::StringValue(index.name),
            ));
        }

        storage.indexes.insert(index.name.clone(), index);
        Ok(())
    }

    fn drop_index(&self, name: String) -> Result<(), GraphError> {
        let mut storage = self.storage.clone();

        if storage.indexes.remove(&name).is_none() {
            return Err(GraphError::ElementNotFound(
                crate::exports::golem::graph::types::ElementId::StringValue(name),
            ));
        }

        Ok(())
    }

    fn list_indexes(&self) -> Result<Vec<IndexDefinition>, GraphError> {
        Ok(self.storage.indexes.values().cloned().collect())
    }

    fn get_index(&self, name: String) -> Result<Option<IndexDefinition>, GraphError> {
        Ok(self.storage.indexes.get(&name).cloned())
    }

    fn define_edge_type(&self, definition: EdgeTypeDefinition) -> Result<(), GraphError> {
        let mut storage = self.storage.clone();

        // Check if edge type already exists
        if storage
            .edge_types
            .iter()
            .any(|et| et.collection == definition.collection)
        {
            return Err(GraphError::DuplicateElement(
                crate::exports::golem::graph::types::ElementId::StringValue(definition.collection),
            ));
        }

        storage.edge_types.push(definition);
        Ok(())
    }

    fn list_edge_types(&self) -> Result<Vec<EdgeTypeDefinition>, GraphError> {
        Ok(self.storage.edge_types.clone())
    }

    fn create_container(
        &self,
        name: String,
        container_type: ContainerType,
    ) -> Result<(), GraphError> {
        let mut storage = self.storage.clone();

        // Check if container already exists
        if storage.containers.contains_key(&name) {
            return Err(GraphError::DuplicateElement(
                crate::exports::golem::graph::types::ElementId::StringValue(name),
            ));
        }

        let container_info = ContainerInfo {
            name: name.clone(),
            container_type,
            element_count: Some(0), // Start with 0 elements
        };

        storage.containers.insert(name, container_info);
        Ok(())
    }

    fn list_containers(&self) -> Result<Vec<ContainerInfo>, GraphError> {
        Ok(self.storage.containers.values().cloned().collect())
    }
}

/// Wraps a graph implementation with custom durability
pub struct DurableGraph<Impl> {
    phantom: PhantomData<Impl>,
}

/// Trait to be implemented in addition to the graph `Guest` traits when wrapping with `DurableGraph`.
pub trait ExtendedGraphGuest:
    ConnectionGuest + GuestGraph + TraversalGuest + QueryGuest + SchemaGuest + Clone + 'static
{
    type ReplayState: std::fmt::Debug + Clone + IntoValueAndType + FromValueAndType;
    type Transaction: GuestTransaction;
    type SchemaManager;

    /// Creates an instance of the graph without wrapping it in a `Resource`
    fn unwrapped_graph(config: ConnectionConfig) -> Result<Self, GraphError>
    where
        Self: Sized;

    /// Used at the end of replay to go from replay to live mode
    fn graph_to_state(graph: &Self) -> Self::ReplayState;
    fn graph_from_state(
        state: &Self::ReplayState,
        config: ConnectionConfig,
    ) -> Result<Self, GraphError>
    where
        Self: Sized;

    /// Creates an instance of the transaction without wrapping it in a `Resource`
    fn unwrapped_transaction(
        graph: &Self,
        read_only: bool,
    ) -> Result<Self::Transaction, GraphError>;

    /// Used at the end of replay to go from replay to live mode for transactions
    fn transaction_to_state(transaction: &Self::Transaction) -> Self::ReplayState;
    fn transaction_from_state(
        state: &Self::ReplayState,
        graph: &Self,
        read_only: bool,
    ) -> Result<Self::Transaction, GraphError>;

    fn schema_manager_to_state(
        schema_manager: &<Self as ExtendedGraphGuest>::SchemaManager,
    ) -> Self::ReplayState;
    fn schema_manager_from_state(
        state: &Self::ReplayState,
    ) -> Result<<Self as ExtendedGraphGuest>::SchemaManager, GraphError>;
}

/// When the durability feature flag is off, wrapping with `DurableGraph` is just a passthrough
#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use crate::durability::{DurableGraph, ExtendedGraphGuest, TransactionBorrow};
    use crate::exports::golem::graph::connection::{
        ConnectionConfig, Graph, Guest as ConnectionGuest,
    };
    use crate::exports::golem::graph::errors::GraphError;
    use crate::exports::golem::graph::query::{
        Guest as QueryGuest, QueryExecutionResult, QueryOptions,
    };
    use crate::exports::golem::graph::schema::{Guest as SchemaGuest, SchemaManager};
    use crate::exports::golem::graph::transactions::Guest as TransactionGuest;
    use crate::exports::golem::graph::traversal::{
        Guest as TraversalGuest, NeighborhoodOptions, Path, PathOptions, Subgraph,
    };
    use crate::exports::golem::graph::types::{Direction, ElementId, PropertyValue, Vertex};

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
        type SchemaManager = DurableSchemaManager;

        fn get_schema_manager() -> Result<Self::SchemaManager, GraphError> {
            Ok(DurableSchemaManager::new())
        }
    }

    impl<Impl: ExtendedGraphGuest> TraversalGuest for DurableGraph<Impl> {
        fn find_shortest_path(
            transaction: TransactionBorrow<'_>,
            from_vertex: ElementId,
            to_vertex: ElementId,
            options: Option<PathOptions>,
        ) -> Result<Option<Path>, GraphError> {
            Impl::find_shortest_path(transaction, from_vertex, to_vertex, options)
        }

        fn find_all_paths(
            transaction: TransactionBorrow<'_>,
            from_vertex: ElementId,
            to_vertex: ElementId,
            options: Option<PathOptions>,
            limit: Option<u32>,
        ) -> Result<Vec<Path>, GraphError> {
            Impl::find_all_paths(transaction, from_vertex, to_vertex, options, limit)
        }

        fn get_neighborhood(
            transaction: TransactionBorrow<'_>,
            center: ElementId,
            options: NeighborhoodOptions,
        ) -> Result<Subgraph, GraphError> {
            Impl::get_neighborhood(transaction, center, options)
        }

        fn path_exists(
            transaction: TransactionBorrow<'_>,
            from_vertex: ElementId,
            to_vertex: ElementId,
            options: Option<PathOptions>,
        ) -> Result<bool, GraphError> {
            Impl::path_exists(transaction, from_vertex, to_vertex, options)
        }

        fn get_vertices_at_distance(
            transaction: TransactionBorrow<'_>,
            source: ElementId,
            distance: u32,
            direction: Direction,
            edge_types: Option<Vec<String>>,
        ) -> Result<Vec<Vertex>, GraphError> {
            Impl::get_vertices_at_distance(transaction, source, distance, direction, edge_types)
        }
    }

    impl<Impl: ExtendedGraphGuest> QueryGuest for DurableGraph<Impl> {
        fn execute_query(
            transaction: TransactionBorrow<'_>,
            query: String,
            parameters: Option<Vec<(String, PropertyValue)>>,
            options: Option<QueryOptions>,
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
    use crate::durability::{DurableGraph, DurableSchemaManager, ExtendedGraphGuest};
    use crate::exports::golem::graph::connection::{
        ConnectionConfig, Graph, GraphStatistics, Guest as ConnectionGuest, GuestGraph,
    };
    use crate::exports::golem::graph::errors::GraphError;
    use crate::exports::golem::graph::query::{
        Guest as QueryGuest, QueryExecutionResult, QueryOptions,
    };
    use crate::exports::golem::graph::schema::{
        ContainerInfo, ContainerType, EdgeLabelSchema, EdgeTypeDefinition, IndexDefinition,
        VertexLabelSchema,
    };
    use crate::exports::golem::graph::schema::{
        Guest as SchemaGuest, GuestSchemaManager, SchemaManager,
    };
    use crate::exports::golem::graph::transactions::{EdgeSpec, VertexSpec};
    use crate::exports::golem::graph::transactions::{
        Guest as TransactionGuest, GuestTransaction, Transaction, TransactionBorrow,
    };
    use crate::exports::golem::graph::traversal::{
        Guest as TraversalGuest, NeighborhoodOptions, Path, PathOptions, Subgraph,
    };
    use crate::exports::golem::graph::types::{
        Direction, Edge, ElementId, FilterCondition, PropertyMap, PropertyValue, SortSpec, Vertex,
    };
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    use golem_rust::durability::Durability;
    use golem_rust::{with_persistence_level, PersistenceLevel};
    use golem_rust::{FromValueAndType, IntoValue};

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct ConnectInput {
        config: ConnectionConfig,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct NoInput;

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct NoOutput;

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct CreateVertexInput {
        vertex_type: String,
        properties: PropertyMap,
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
        fn from(_: NoInput) -> Self {}
    }
    impl From<&GraphError> for GraphError {
        fn from(error: &GraphError) -> Self {
            error.clone()
        }
    }
    impl<Impl: ExtendedGraphGuest<SchemaManager = SchemaManager>> SchemaGuest for DurableGraph<Impl> {
        type SchemaManager = DurableSchemaManager;

        fn get_schema_manager() -> std::result::Result<
            crate::golem::graph::schema::SchemaManager,
            crate::golem::graph::errors::GraphError,
        > {
            let durability = Durability::<NoInput, GraphError>::new(
                "golem_graph_schema",
                "get_schema_manager",
                DurableFunctionType::WriteRemote,
            );

            if durability.is_live() {
                let result: Result<DurableSchemaManager, GraphError> =
                    Ok(DurableSchemaManager::new());
                match &result {
                    Ok(_) => {
                        let _ = durability.persist(NoInput, Ok(NoInput));
                        Ok(crate::golem::graph::schema::SchemaManager::new(
                            result.unwrap(),
                        ))
                    }
                    Err(e) => {
                        let _ = durability.persist(NoInput, Err(e.clone()));
                        Err(e.clone())
                    }
                }
            } else {
                match durability.replay::<NoOutput, GraphError>() {
                    Ok(_) => Ok(crate::golem::graph::schema::SchemaManager::new(
                        DurableSchemaManager::new(),
                    )),
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
            options: Option<PathOptions>,
        ) -> Result<Option<Path>, GraphError> {
            Impl::find_shortest_path(transaction, from_vertex, to_vertex, options)
        }

        fn find_all_paths(
            transaction: TransactionBorrow<'_>,
            from_vertex: ElementId,
            to_vertex: ElementId,
            options: Option<PathOptions>,
            limit: Option<u32>,
        ) -> Result<Vec<Path>, GraphError> {
            Impl::find_all_paths(transaction, from_vertex, to_vertex, options, limit)
        }

        fn get_neighborhood(
            transaction: TransactionBorrow<'_>,
            center: ElementId,
            options: NeighborhoodOptions,
        ) -> Result<Subgraph, GraphError> {
            Impl::get_neighborhood(transaction, center, options)
        }

        fn path_exists(
            transaction: TransactionBorrow<'_>,
            from_vertex: ElementId,
            to_vertex: ElementId,
            options: Option<PathOptions>,
        ) -> Result<bool, GraphError> {
            Impl::path_exists(transaction, from_vertex, to_vertex, options)
        }

        fn get_vertices_at_distance(
            transaction: TransactionBorrow<'_>,
            source: ElementId,
            distance: u32,
            direction: Direction,
            edge_types: Option<Vec<String>>,
        ) -> Result<Vec<Vertex>, GraphError> {
            Impl::get_vertices_at_distance(transaction, source, distance, direction, edge_types)
        }
    }

    impl<Impl: ExtendedGraphGuest> QueryGuest for DurableGraph<Impl> {
        fn execute_query(
            transaction: TransactionBorrow<'_>,
            query: String,
            parameters: Option<Vec<(String, PropertyValue)>>,
            options: Option<QueryOptions>,
        ) -> Result<QueryExecutionResult, GraphError> {
            Impl::execute_query(transaction, query, parameters, options)
        }
    }
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
                DurableFunctionType::WriteRemote,
            );

            if durability.is_live() {
                let result = self.graph.begin_transaction();
                match result {
                    Ok(_) => {
                        let _ = durability.persist(NoInput, Ok(NoInput));
                        let transaction = Impl::unwrapped_transaction(&self.graph, false)?;
                        Ok(Transaction::new(DurableTransactionInstance::live(
                            transaction,
                            self.graph.clone(),
                            false,
                        )))
                    }
                    Err(e) => {
                        let _ = durability.persist(NoInput, Err(e.clone()));
                        Err(e.clone())
                    }
                }
            } else {
                match durability.replay::<NoOutput, GraphError>() {
                    Ok(_) => {
                        let transaction = Impl::unwrapped_transaction(&self.graph, false)?;
                        Ok(Transaction::new(DurableTransactionInstance::live(
                            transaction,
                            self.graph.clone(),
                            false,
                        )))
                    }
                    Err(e) => Err(e),
                }
            }
        }

        fn begin_read_transaction(&self) -> Result<Transaction, GraphError> {
            let durability = Durability::<NoInput, GraphError>::new(
                "golem_graph_transaction",
                "begin_read_transaction",
                DurableFunctionType::ReadRemote,
            );

            if durability.is_live() {
                let result = self.graph.begin_read_transaction();
                match result {
                    Ok(_) => {
                        let _ = durability.persist(NoInput, Ok(NoInput));
                        let transaction = Impl::unwrapped_transaction(&self.graph, true)?;
                        Ok(Transaction::new(DurableTransactionInstance::live(
                            transaction,
                            self.graph.clone(),
                            true,
                        )))
                    }
                    Err(e) => {
                        let _ = durability.persist(NoInput, Err(e.clone()));
                        Err(e.clone())
                    }
                }
            } else {
                match durability.replay::<NoOutput, GraphError>() {
                    Ok(_) => {
                        let transaction = Impl::unwrapped_transaction(&self.graph, true)?;
                        Ok(Transaction::new(DurableTransactionInstance::live(
                            transaction,
                            self.graph.clone(),
                            true,
                        )))
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
            read_only: bool,
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
                DurableFunctionType::WriteRemote,
            );

            if durability.is_live() {
                let result = self.transaction.commit();
                match &result {
                    Ok(_) => {
                        let _ = durability.persist(NoInput, Ok(NoInput));
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
                DurableFunctionType::WriteRemote,
            );

            if durability.is_live() {
                let result = self.transaction.rollback();
                match &result {
                    Ok(_) => {
                        let _ = durability.persist(NoInput, Ok(NoInput));
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
            properties: PropertyMap,
        ) -> Result<Vertex, GraphError> {
            let durability = Durability::<CreateVertexInput, GraphError>::new(
                "golem_graph_transaction",
                "create_vertex",
                DurableFunctionType::WriteRemote,
            );

            if durability.is_live() {
                let result = self
                    .transaction
                    .create_vertex(vertex_type.clone(), properties.clone());
                match &result {
                    Ok(_vertex) => {
                        let _ = durability.persist(
                            CreateVertexInput {
                                vertex_type: vertex_type.clone(),
                                properties: properties.clone(),
                            },
                            Ok(CreateVertexInput {
                                vertex_type: vertex_type.clone(),
                                properties: properties.clone(),
                            }),
                        );
                        result
                    }
                    Err(e) => {
                        let _ = durability.persist(
                            CreateVertexInput {
                                vertex_type: vertex_type.clone(),
                                properties: properties.clone(),
                            },
                            Err(e.clone()),
                        );
                        result
                    }
                }
            } else {
                match durability.replay::<Vertex, GraphError>() {
                    Ok(_vertex) => Ok(_vertex),
                    Err(e) => Err(e),
                }
            }
        }

        fn create_vertex_with_labels(
            &self,
            vertex_type: String,
            additional_labels: Vec<String>,
            properties: PropertyMap,
        ) -> Result<Vertex, GraphError> {
            self.transaction
                .create_vertex_with_labels(vertex_type, additional_labels, properties)
        }

        fn get_vertex(&self, id: ElementId) -> Result<Option<Vertex>, GraphError> {
            self.transaction.get_vertex(id)
        }

        fn update_vertex(
            &self,
            id: ElementId,
            properties: PropertyMap,
        ) -> Result<Vertex, GraphError> {
            self.transaction.update_vertex(id, properties)
        }

        fn update_vertex_properties(
            &self,
            id: ElementId,
            updates: PropertyMap,
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
            offset: Option<u32>,
        ) -> Result<Vec<Vertex>, GraphError> {
            self.transaction
                .find_vertices(vertex_type, filters, sort, limit, offset)
        }

        // Edge operations
        fn create_edge(
            &self,
            edge_type: String,
            from_vertex: ElementId,
            to_vertex: ElementId,
            properties: PropertyMap,
        ) -> Result<Edge, GraphError> {
            self.transaction
                .create_edge(edge_type, from_vertex, to_vertex, properties)
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
            updates: PropertyMap,
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
            offset: Option<u32>,
        ) -> Result<Vec<Edge>, GraphError> {
            self.transaction
                .find_edges(edge_types, filters, sort, limit, offset)
        }

        // Traversal operations
        fn get_adjacent_vertices(
            &self,
            vertex_id: ElementId,
            direction: Direction,
            edge_types: Option<Vec<String>>,
            limit: Option<u32>,
        ) -> Result<Vec<Vertex>, GraphError> {
            self.transaction
                .get_adjacent_vertices(vertex_id, direction, edge_types, limit)
        }

        fn get_connected_edges(
            &self,
            vertex_id: ElementId,
            direction: Direction,
            edge_types: Option<Vec<String>>,
            limit: Option<u32>,
        ) -> Result<Vec<Edge>, GraphError> {
            self.transaction
                .get_connected_edges(vertex_id, direction, edge_types, limit)
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
            properties: PropertyMap,
        ) -> Result<Vertex, GraphError> {
            self.transaction.upsert_vertex(id, vertex_type, properties)
        }

        fn upsert_edge(
            &self,
            id: Option<ElementId>,
            edge_type: String,
            from_vertex: ElementId,
            to_vertex: ElementId,
            properties: PropertyMap,
        ) -> Result<Edge, GraphError> {
            self.transaction
                .upsert_edge(id, edge_type, from_vertex, to_vertex, properties)
        }

        fn is_active(&self) -> bool {
            self.transaction.is_active()
        }
    }

    pub struct DurableSchemaManagerInstance<Impl: ExtendedGraphGuest> {
        schema_manager: DurableSchemaManager,
        _phantom: std::marker::PhantomData<Impl>,
    }

    impl<Impl: ExtendedGraphGuest> DurableSchemaManagerInstance<Impl> {
        fn new(schema_manager: DurableSchemaManager) -> Self {
            Self {
                schema_manager,
                _phantom: std::marker::PhantomData,
            }
        }
    }

    impl<Impl: ExtendedGraphGuest> GuestSchemaManager for DurableSchemaManagerInstance<Impl> {
        fn define_vertex_label(&self, schema: VertexLabelSchema) -> Result<(), GraphError> {
            GuestSchemaManager::define_vertex_label(&self.schema_manager, schema)
        }

        fn define_edge_label(&self, schema: EdgeLabelSchema) -> Result<(), GraphError> {
            GuestSchemaManager::define_edge_label(&self.schema_manager, schema)
        }

        fn get_vertex_label_schema(
            &self,
            label: String,
        ) -> Result<Option<VertexLabelSchema>, GraphError> {
            GuestSchemaManager::get_vertex_label_schema(&self.schema_manager, label)
        }

        fn get_edge_label_schema(
            &self,
            label: String,
        ) -> Result<Option<EdgeLabelSchema>, GraphError> {
            GuestSchemaManager::get_edge_label_schema(&self.schema_manager, label)
        }

        fn list_vertex_labels(&self) -> Result<Vec<String>, GraphError> {
            GuestSchemaManager::list_vertex_labels(&self.schema_manager)
        }

        fn list_edge_labels(&self) -> Result<Vec<String>, GraphError> {
            GuestSchemaManager::list_edge_labels(&self.schema_manager)
        }

        fn create_index(&self, index: IndexDefinition) -> Result<(), GraphError> {
            GuestSchemaManager::create_index(&self.schema_manager, index)
        }

        fn drop_index(&self, name: String) -> Result<(), GraphError> {
            GuestSchemaManager::drop_index(&self.schema_manager, name)
        }

        fn list_indexes(&self) -> Result<Vec<IndexDefinition>, GraphError> {
            GuestSchemaManager::list_indexes(&self.schema_manager)
        }

        fn get_index(&self, name: String) -> Result<Option<IndexDefinition>, GraphError> {
            GuestSchemaManager::get_index(&self.schema_manager, name)
        }

        fn define_edge_type(&self, definition: EdgeTypeDefinition) -> Result<(), GraphError> {
            GuestSchemaManager::define_edge_type(&self.schema_manager, definition)
        }

        fn list_edge_types(&self) -> Result<Vec<EdgeTypeDefinition>, GraphError> {
            GuestSchemaManager::list_edge_types(&self.schema_manager)
        }

        fn create_container(
            &self,
            name: String,
            container_type: ContainerType,
        ) -> Result<(), GraphError> {
            GuestSchemaManager::create_container(&self.schema_manager, name, container_type)
        }

        fn list_containers(&self) -> Result<Vec<ContainerInfo>, GraphError> {
            GuestSchemaManager::list_containers(&self.schema_manager)
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

    impl<Impl: ExtendedGraphGuest> ConnectionGuest for DurableGraph<Impl> {
        type Graph = DurableGraphInstance<Impl>;

        fn connect(config: ConnectionConfig) -> Result<Graph, GraphError> {
            let durability = Durability::<ConnectInput, GraphError>::new(
                "golem_graph",
                "connect",
                DurableFunctionType::WriteRemote,
            );

            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::unwrapped_graph(config.clone())
                });

                match &result {
                    Ok(graph) => {
                        let _ = durability.persist(
                            ConnectInput {
                                config: config.clone(),
                            },
                            Ok(ConnectInput {
                                config: config.clone(),
                            }),
                        );
                        Ok(Graph::new(DurableGraphInstance::live(
                            graph.clone(),
                            config,
                        )))
                    }
                    Err(e) => {
                        let _ = durability.persist(ConnectInput { config }, Err(e.clone()));
                        Err(e.clone())
                    }
                }
            } else {
                match durability.replay::<NoOutput, GraphError>() {
                    Ok(_) => {
                        let graph = Impl::unwrapped_graph(config.clone())?;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use golem_rust::value_and_type::{FromValueAndType, IntoValueAndType};
    use std::fmt::Debug;

    fn roundtrip_test<T: Debug + Clone + PartialEq + IntoValueAndType + FromValueAndType>(
        value: T,
    ) {
        let vnt = value.clone().into_value_and_type();
        let decoded = T::from_value_and_type(vnt).unwrap();
        assert_eq!(value, decoded);
    }
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

    // Basic tests
    #[test]
    fn mock_replay_state_roundtrip() {
        let state = MockReplayState {
            graph_id: "test".to_string(),
            transaction_id: Some("tx1".to_string()),
            schema_manager_id: Some("sm1".to_string()),
        };
        let cloned = state.clone();
        assert_eq!(state, cloned);
    }

    #[test]
    fn mock_graph_roundtrip() {
        let graph = MockGraph {
            id: "test".to_string(),
            config: ConnectionConfig {
                hosts: vec!["localhost".to_string()],
                port: Some(8529),
                database_name: Some("test".to_string()),
                timeout_seconds: Some(30),
                max_connections: Some(10),
                provider_config: vec![],
                username: Some("test".to_string()),
                password: Some("test".to_string()),
            },
        };
        let cloned = graph.clone();
        assert_eq!(graph, cloned);
    }

    #[test]
    fn mock_transaction_roundtrip() {
        let transaction = MockTransaction {
            id: "test".to_string(),
            read_only: false,
        };
        let cloned = transaction.clone();
        assert_eq!(transaction, cloned);
    }

    #[test]
    fn mock_schema_manager_roundtrip() {
        let schema_manager = MockSchemaManager {
            id: "test".to_string(),
        };
        let cloned = schema_manager.clone();
        assert_eq!(schema_manager, cloned);
    }
}

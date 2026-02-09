use crate::model::{
    connection::{self, ConnectionConfig},
    errors::GraphError,
    schema::SchemaManager,
    transactions::{self},
};
use crate::{GraphInterface, TransactionInterface};
use std::marker::PhantomData;

pub trait TransactionBorrowExt<'a, T> {
    fn get(&self) -> &'a T;
}

pub struct DurableGraph<Impl> {
    _phantom: PhantomData<Impl>,
}

pub trait ExtendedGuest: 'static
where
    Self::Graph: ProviderGraph + 'static,
{
    type Graph: GraphInterface;
    fn connect_internal(config: &ConnectionConfig) -> Result<Self::Graph, GraphError>;
}

pub trait ProviderGraph: GraphInterface {
    type Transaction: TransactionInterface;
}

/// When the durability feature flag is off, wrapping with `DurableGraph` is just a passthrough
#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use super::*;
    use crate::init_logging;

    impl<Impl: ExtendedGuest> connection::Guest for DurableGraph<Impl>
    where
        Impl::Graph: ProviderGraph + 'static,
    {
        type Graph = Impl::Graph;

        fn connect(config: ConnectionConfig) -> Result<connection::Graph, GraphError> {
            init_logging();
            let graph = Impl::connect_internal(&config)?;
            Ok(connection::Graph::new(graph))
        }
    }

    impl<Impl: ExtendedGuest + TransactionGuest> TransactionGuest for DurableGraph<Impl>
    where
        Impl::Graph: ProviderGraph + 'static,
    {
        type Transaction = Impl::Transaction;
    }

    impl<Impl: ExtendedGuest + SchemaGuest> SchemaGuest for DurableGraph<Impl>
    where
        Impl::Graph: ProviderGraph + 'static,
    {
        type SchemaManager = Impl::SchemaManager;

        fn get_schema_manager(
            config: Option<ConnectionConfig>,
        ) -> Result<SchemaManager, GraphError> {
            init_logging();
            Impl::get_schema_manager(config)
        }
    }
}

#[cfg(feature = "durability")]
mod durable_impl {
    use super::*;
    use crate::durability::transactions::CreateVertexOptions;
    use crate::model::connection::GraphStatistics;
    use crate::model::transactions::{
        CreateEdgeOptions, Edge, ElementId, ExecuteQueryOptions, FindAllPathsOptions,
        FindEdgesOptions, FindShortestPathOptions, FindVerticesOptions, GetAdjacentVerticesOptions,
        GetConnectedEdgesOptions, GetNeighborhoodOptions, GetVerticesAtDistanceOptions, Path,
        PathExistsOptions, QueryExecutionResult, Subgraph, UpdateEdgeOptions, UpdateVertexOptions,
        Vertex,
    };
    use crate::{
        init_logging, GraphInterface, GraphProvider, SchemaManagerProvider, TransactionInterface,
        TransactionProvider,
    };
    use golem_rust::bindings::golem::durability::durability::WrappedFunctionType;
    use golem_rust::durability::Durability;
    use golem_rust::{with_persistence_level, FromValueAndType, IntoValue, PersistenceLevel};

    #[derive(Debug, Clone, FromValueAndType, IntoValue)]
    pub(super) struct Unit;

    #[derive(Debug)]
    pub struct DurableGraphResource<G> {
        graph: G,
    }

    #[allow(dead_code)]
    #[derive(Debug)]
    pub struct DurableTransaction<T: TransactionInterface> {
        pub inner: T,
    }

    impl<T: TransactionInterface> DurableTransaction<T> {
        pub fn _new(inner: T) -> Self {
            Self { inner }
        }
    }

    impl<Impl: ExtendedGuest> GraphProvider for DurableGraph<Impl>
    where
        Impl::Graph: ProviderGraph + 'static,
    {
        type Graph = DurableGraphResource<Impl::Graph>;
        fn connect(config: ConnectionConfig) -> Result<connection::Graph, GraphError> {
            init_logging();
            let durability = Durability::<Unit, GraphError>::new(
                "golem_ai_graph",
                "connect",
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = Impl::connect_internal(&config);
                let persist_result = result.as_ref().map(|_| Unit).map_err(|e| e.clone());
                durability.persist(config.clone(), persist_result)?;
                result.map(|g| connection::Graph::new(DurableGraphResource::new(g)))
            } else {
                let _unit: Unit = durability.replay::<Unit, GraphError>()?;
                let graph = Impl::connect_internal(&config)?;
                Ok(connection::Graph::new(DurableGraphResource::new(graph)))
            }
        }
    }

    impl<Impl: ExtendedGuest + TransactionProvider> TransactionProvider for DurableGraph<Impl>
    where
        Impl::Graph: ProviderGraph + 'static,
    {
        type Transaction = Impl::Transaction;
    }

    impl<Impl: ExtendedGuest + SchemaManagerProvider> SchemaManagerProvider for DurableGraph<Impl>
    where
        Impl::Graph: ProviderGraph + 'static,
    {
        type SchemaManager = Impl::SchemaManager;

        fn get_schema_manager(
            config: Option<ConnectionConfig>,
        ) -> Result<SchemaManager, GraphError> {
            init_logging();
            Impl::get_schema_manager(config)
        }
    }

    impl<G: ProviderGraph + 'static> GraphInterface for DurableGraphResource<G> {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }

        fn begin_transaction(&self) -> Result<transactions::Transaction, GraphError> {
            init_logging();
            self.graph.begin_transaction()
        }

        fn begin_read_transaction(&self) -> Result<transactions::Transaction, GraphError> {
            init_logging();
            self.graph.begin_read_transaction()
        }

        fn ping(&self) -> Result<(), GraphError> {
            self.graph.ping()
        }

        fn close(&self) -> Result<(), GraphError> {
            init_logging();
            self.graph.close()
        }

        fn get_statistics(&self) -> Result<GraphStatistics, GraphError> {
            init_logging();
            self.graph.get_statistics()
        }
    }

    impl<G: GraphInterface> DurableGraphResource<G> {
        pub fn new(graph: G) -> Self {
            Self { graph }
        }
    }

    impl<T: TransactionInterface + 'static> TransactionInterface for DurableTransaction<T> {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }

        fn execute_query(
            &self,
            options: ExecuteQueryOptions,
        ) -> Result<QueryExecutionResult, GraphError> {
            init_logging();
            let durability: Durability<QueryExecutionResult, GraphError> = Durability::new(
                "golem_graph_transaction",
                "execute_query",
                WrappedFunctionType::WriteRemote,
            );

            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    self.execute_query(options.clone())
                });
                durability.persist(options, result)
            } else {
                durability.replay()
            }
        }

        fn find_shortest_path(
            &self,
            options: FindShortestPathOptions,
        ) -> Result<Option<Path>, GraphError> {
            init_logging();
            self.inner.find_shortest_path(options)
        }

        fn find_all_paths(&self, options: FindAllPathsOptions) -> Result<Vec<Path>, GraphError> {
            init_logging();
            self.inner.find_all_paths(options)
        }

        fn get_neighborhood(
            &self,
            options: GetNeighborhoodOptions,
        ) -> Result<Subgraph, GraphError> {
            init_logging();
            self.inner.get_neighborhood(options)
        }

        fn path_exists(&self, options: PathExistsOptions) -> Result<bool, GraphError> {
            init_logging();
            self.inner.path_exists(options)
        }

        fn get_vertices_at_distance(
            &self,
            options: GetVerticesAtDistanceOptions,
        ) -> Result<Vec<Vertex>, GraphError> {
            init_logging();
            self.inner.get_vertices_at_distance(options)
        }

        fn get_adjacent_vertices(
            &self,
            options: GetAdjacentVerticesOptions,
        ) -> Result<Vec<Vertex>, GraphError> {
            init_logging();
            self.inner.get_adjacent_vertices(options)
        }

        fn get_connected_edges(
            &self,
            option: GetConnectedEdgesOptions,
        ) -> Result<Vec<Edge>, GraphError> {
            init_logging();
            self.inner.get_connected_edges(option)
        }

        fn create_vertex(&self, options: CreateVertexOptions) -> Result<Vertex, GraphError> {
            init_logging();
            let durability: Durability<Vertex, GraphError> = Durability::new(
                "golem_graph_transaction",
                "create_vertex",
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    self.inner.create_vertex(options.clone())
                });
                durability.persist(options, result)
            } else {
                durability.replay()
            }
        }

        fn create_vertices(
            &self,
            vertices: Vec<CreateVertexOptions>,
        ) -> Result<Vec<Vertex>, GraphError> {
            init_logging();
            let durability: Durability<Vec<Vertex>, GraphError> = Durability::new(
                "golem_graph_transaction",
                "create_vertices",
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    self.inner.create_vertices(vertices.clone())
                });
                durability.persist(vertices, result)
            } else {
                durability.replay()
            }
        }

        fn get_vertex(&self, id: ElementId) -> Result<Option<Vertex>, GraphError> {
            init_logging();
            self.inner.get_vertex(id)
        }

        fn update_vertex(&self, options: UpdateVertexOptions) -> Result<Vertex, GraphError> {
            init_logging();
            let durability: Durability<Vertex, GraphError> = Durability::new(
                "golem_graph_transaction",
                "update_vertex",
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    self.inner.update_vertex(options.clone())
                });
                durability.persist(options, result)
            } else {
                durability.replay()
            }
        }

        fn delete_vertex(&self, id: ElementId, delete_edges: bool) -> Result<(), GraphError> {
            init_logging();
            let durability: Durability<Unit, GraphError> = Durability::new(
                "golem_graph_transaction",
                "delete_vertex",
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    self.inner.delete_vertex(id.clone(), delete_edges)
                });
                durability.persist((id, delete_edges), result.map(|_| Unit))?;
                Ok(())
            } else {
                durability.replay::<Unit, GraphError>()?;
                Ok(())
            }
        }

        fn find_vertices(&self, options: FindVerticesOptions) -> Result<Vec<Vertex>, GraphError> {
            init_logging();
            self.inner.find_vertices(options)
        }

        fn create_edge(&self, options: CreateEdgeOptions) -> Result<Edge, GraphError> {
            init_logging();
            let durability: Durability<Edge, GraphError> = Durability::new(
                "golem_graph_transaction",
                "create_edge",
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    self.inner.create_edge(options.clone())
                });
                durability.persist(options, result)
            } else {
                durability.replay()
            }
        }

        fn create_edges(&self, edges: Vec<CreateEdgeOptions>) -> Result<Vec<Edge>, GraphError> {
            init_logging();
            let durability: Durability<Vec<Edge>, GraphError> = Durability::new(
                "golem_graph_transaction",
                "create_edges",
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    self.inner.create_edges(edges.clone())
                });
                durability.persist(edges, result)
            } else {
                durability.replay()
            }
        }

        fn get_edge(&self, id: ElementId) -> Result<Option<Edge>, GraphError> {
            init_logging();
            self.inner.get_edge(id)
        }

        fn update_edge(&self, options: UpdateEdgeOptions) -> Result<Edge, GraphError> {
            init_logging();
            let durability: Durability<Edge, GraphError> = Durability::new(
                "golem_graph_transaction",
                "update_edge",
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    self.inner.update_edge(options.clone())
                });
                durability.persist(options, result)
            } else {
                durability.replay()
            }
        }

        fn delete_edge(&self, id: ElementId) -> Result<(), GraphError> {
            init_logging();
            let durability: Durability<Unit, GraphError> = Durability::new(
                "golem_graph_transaction",
                "delete_edge",
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    self.inner.delete_edge(id.clone())
                });
                durability.persist(id, result.map(|_| Unit))?;
                Ok(())
            } else {
                durability.replay::<Unit, GraphError>()?;
                Ok(())
            }
        }

        fn find_edges(&self, options: FindEdgesOptions) -> Result<Vec<Edge>, GraphError> {
            init_logging();
            self.inner.find_edges(options)
        }

        fn commit(&self) -> Result<(), GraphError> {
            init_logging();
            let durability = Durability::<Unit, GraphError>::new(
                "golem_graph_transaction",
                "commit",
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    self.inner.commit()
                });
                durability.persist(Unit, result.map(|_| Unit))?;
                Ok(())
            } else {
                durability.replay::<Unit, GraphError>()?;
                Ok(())
            }
        }

        fn rollback(&self) -> Result<(), GraphError> {
            init_logging();
            let durability = Durability::<Unit, GraphError>::new(
                "golem_graph_transaction",
                "rollback",
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    self.inner.rollback()
                });
                durability.persist(Unit, result.map(|_| Unit))?;
                Ok(())
            } else {
                durability.replay::<Unit, GraphError>()?;
                Ok(())
            }
        }

        fn is_active(&self) -> bool {
            self.inner.is_active()
        }
    }
}

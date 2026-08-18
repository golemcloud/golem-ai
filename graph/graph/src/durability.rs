use crate::model::{
    connection::{self, ConnectionConfig},
    errors::GraphError,
    schema::SchemaManager,
};
use crate::{GraphInterface, TransactionInterface};
use std::marker::PhantomData;

pub trait TransactionBorrowExt<'a, T> {
    fn get(&self) -> &'a T;
}

pub struct DurableGraph<Impl> {
    _phantom: PhantomData<Impl>,
}

#[allow(async_fn_in_trait)]
pub trait ExtendedGuest: 'static
where
    Self::Graph: ProviderGraph + 'static,
{
    type Graph: GraphInterface;
    async fn connect_internal(config: &ConnectionConfig) -> Result<Self::Graph, GraphError>;
}

pub trait ProviderGraph: GraphInterface {
    type Transaction: TransactionInterface;
}

/// When the `golem` feature flag is off, `DurableGraph<Impl>` is a transparent wrapper that
/// forwards every call to the inner provider without any oplog persistence.
#[cfg(not(feature = "golem"))]
mod passthrough_impl {
    use super::*;
    use crate::init_logging;
    use crate::{GraphProvider, SchemaManagerProvider, TransactionProvider};

    impl<Impl: ExtendedGuest> GraphProvider for DurableGraph<Impl>
    where
        Impl::Graph: ProviderGraph + 'static,
    {
        type Graph = Impl::Graph;

        async fn connect(config: ConnectionConfig) -> Result<connection::Graph, GraphError> {
            init_logging();
            let graph = Impl::connect_internal(&config).await?;
            Ok(connection::Graph::new(graph))
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

        async fn get_schema_manager(
            config: Option<ConnectionConfig>,
        ) -> Result<SchemaManager, GraphError> {
            init_logging();
            Impl::get_schema_manager(config).await
        }
    }
}

#[cfg(feature = "golem")]
mod durable_impl {
    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct NoInput;

    use super::*;
    use crate::model::connection::GraphStatistics;
    use crate::model::transactions;
    use crate::model::transactions::{
        CreateEdgeOptions, CreateVertexOptions, Edge, ElementId, ExecuteQueryOptions,
        FindAllPathsOptions, FindEdgesOptions, FindShortestPathOptions, FindVerticesOptions,
        GetAdjacentVerticesOptions, GetConnectedEdgesOptions, GetNeighborhoodOptions,
        GetVerticesAtDistanceOptions, Path, PathExistsOptions, QueryExecutionResult, Subgraph,
        UpdateEdgeOptions, UpdateVertexOptions, Vertex,
    };
    use crate::{
        init_logging, GraphInterface, GraphProvider, SchemaManagerProvider, TransactionInterface,
        TransactionProvider,
    };
    use async_trait::async_trait;
    use golem_rust::durability::{Durability, DurableFunctionType};
    use golem_rust::{FromSchema, IntoSchema};

    #[derive(Debug, Clone, FromSchema, IntoSchema)]
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
        async fn connect(config: ConnectionConfig) -> Result<connection::Graph, GraphError> {
            init_logging();
            // The opaque connection must be reconstructed during replay, so its provider-level
            // random and HTTP calls use their ordinary durability instead of a custom invocation.
            let graph = Impl::connect_internal(&config).await?;
            Ok(connection::Graph::new(DurableGraphResource::new(graph)))
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

        async fn get_schema_manager(
            config: Option<ConnectionConfig>,
        ) -> Result<SchemaManager, GraphError> {
            init_logging();
            Impl::get_schema_manager(config).await
        }
    }

    #[async_trait(?Send)]
    impl<G: ProviderGraph + 'static> GraphInterface for DurableGraphResource<G> {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }

        async fn begin_transaction(&self) -> Result<transactions::Transaction, GraphError> {
            init_logging();
            self.graph.begin_transaction().await
        }

        async fn begin_read_transaction(&self) -> Result<transactions::Transaction, GraphError> {
            init_logging();
            self.graph.begin_read_transaction().await
        }

        async fn ping(&self) -> Result<(), GraphError> {
            self.graph.ping().await
        }

        async fn close(&self) -> Result<(), GraphError> {
            init_logging();
            self.graph.close().await
        }

        async fn get_statistics(&self) -> Result<GraphStatistics, GraphError> {
            init_logging();
            self.graph.get_statistics().await
        }
    }

    impl<G: GraphInterface> DurableGraphResource<G> {
        pub fn new(graph: G) -> Self {
            Self { graph }
        }
    }

    #[async_trait(?Send)]
    impl<T: TransactionInterface + 'static> TransactionInterface for DurableTransaction<T> {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }

        async fn execute_query(
            &self,
            options: ExecuteQueryOptions,
        ) -> Result<QueryExecutionResult, GraphError> {
            init_logging();
            Durability::<QueryExecutionResult, GraphError>::new(
                "golem_graph_transaction",
                "execute_query",
                DurableFunctionType::WriteRemote,
                &options,
            )
            .run_async(|| self.inner.execute_query(options))
            .await
        }

        async fn find_shortest_path(
            &self,
            options: FindShortestPathOptions,
        ) -> Result<Option<Path>, GraphError> {
            init_logging();
            self.inner.find_shortest_path(options).await
        }

        async fn find_all_paths(
            &self,
            options: FindAllPathsOptions,
        ) -> Result<Vec<Path>, GraphError> {
            init_logging();
            self.inner.find_all_paths(options).await
        }

        async fn get_neighborhood(
            &self,
            options: GetNeighborhoodOptions,
        ) -> Result<Subgraph, GraphError> {
            init_logging();
            self.inner.get_neighborhood(options).await
        }

        async fn path_exists(&self, options: PathExistsOptions) -> Result<bool, GraphError> {
            init_logging();
            self.inner.path_exists(options).await
        }

        async fn get_vertices_at_distance(
            &self,
            options: GetVerticesAtDistanceOptions,
        ) -> Result<Vec<Vertex>, GraphError> {
            init_logging();
            self.inner.get_vertices_at_distance(options).await
        }

        async fn get_adjacent_vertices(
            &self,
            options: GetAdjacentVerticesOptions,
        ) -> Result<Vec<Vertex>, GraphError> {
            init_logging();
            self.inner.get_adjacent_vertices(options).await
        }

        async fn get_connected_edges(
            &self,
            option: GetConnectedEdgesOptions,
        ) -> Result<Vec<Edge>, GraphError> {
            init_logging();
            self.inner.get_connected_edges(option).await
        }

        async fn create_vertex(&self, options: CreateVertexOptions) -> Result<Vertex, GraphError> {
            init_logging();
            Durability::<Vertex, GraphError>::new(
                "golem_graph_transaction",
                "create_vertex",
                DurableFunctionType::WriteRemote,
                &options,
            )
            .run_async(|| self.inner.create_vertex(options))
            .await
        }

        async fn create_vertices(
            &self,
            vertices: Vec<CreateVertexOptions>,
        ) -> Result<Vec<Vertex>, GraphError> {
            init_logging();
            Durability::<Vec<Vertex>, GraphError>::new(
                "golem_graph_transaction",
                "create_vertices",
                DurableFunctionType::WriteRemote,
                &vertices,
            )
            .run_async(|| self.inner.create_vertices(vertices))
            .await
        }

        async fn get_vertex(&self, id: ElementId) -> Result<Option<Vertex>, GraphError> {
            init_logging();
            self.inner.get_vertex(id).await
        }

        async fn update_vertex(&self, options: UpdateVertexOptions) -> Result<Vertex, GraphError> {
            init_logging();
            Durability::<Vertex, GraphError>::new(
                "golem_graph_transaction",
                "update_vertex",
                DurableFunctionType::WriteRemote,
                &options,
            )
            .run_async(|| self.inner.update_vertex(options))
            .await
        }

        async fn delete_vertex(&self, id: ElementId, delete_edges: bool) -> Result<(), GraphError> {
            init_logging();
            let input = (id.clone(), delete_edges);
            Durability::<Unit, GraphError>::new(
                "golem_graph_transaction",
                "delete_vertex",
                DurableFunctionType::WriteRemote,
                &input,
            )
            .run_async(|| async {
                self.inner
                    .delete_vertex(id, delete_edges)
                    .await
                    .map(|_| Unit)
            })
            .await
            .map(|_| ())
        }

        async fn find_vertices(
            &self,
            options: FindVerticesOptions,
        ) -> Result<Vec<Vertex>, GraphError> {
            init_logging();
            self.inner.find_vertices(options).await
        }

        async fn create_edge(&self, options: CreateEdgeOptions) -> Result<Edge, GraphError> {
            init_logging();
            Durability::<Edge, GraphError>::new(
                "golem_graph_transaction",
                "create_edge",
                DurableFunctionType::WriteRemote,
                &options,
            )
            .run_async(|| self.inner.create_edge(options))
            .await
        }

        async fn create_edges(
            &self,
            edges: Vec<CreateEdgeOptions>,
        ) -> Result<Vec<Edge>, GraphError> {
            init_logging();
            Durability::<Vec<Edge>, GraphError>::new(
                "golem_graph_transaction",
                "create_edges",
                DurableFunctionType::WriteRemote,
                &edges,
            )
            .run_async(|| self.inner.create_edges(edges))
            .await
        }

        async fn get_edge(&self, id: ElementId) -> Result<Option<Edge>, GraphError> {
            init_logging();
            self.inner.get_edge(id).await
        }

        async fn update_edge(&self, options: UpdateEdgeOptions) -> Result<Edge, GraphError> {
            init_logging();
            Durability::<Edge, GraphError>::new(
                "golem_graph_transaction",
                "update_edge",
                DurableFunctionType::WriteRemote,
                &options,
            )
            .run_async(|| self.inner.update_edge(options))
            .await
        }

        async fn delete_edge(&self, id: ElementId) -> Result<(), GraphError> {
            init_logging();
            Durability::<Unit, GraphError>::new(
                "golem_graph_transaction",
                "delete_edge",
                DurableFunctionType::WriteRemote,
                &id,
            )
            .run_async(|| async { self.inner.delete_edge(id).await.map(|_| Unit) })
            .await
            .map(|_| ())
        }

        async fn find_edges(&self, options: FindEdgesOptions) -> Result<Vec<Edge>, GraphError> {
            init_logging();
            self.inner.find_edges(options).await
        }

        async fn commit(&self) -> Result<(), GraphError> {
            init_logging();
            Durability::<Unit, GraphError>::new(
                "golem_graph_transaction",
                "commit",
                DurableFunctionType::WriteRemote,
                &NoInput,
            )
            .run_async(|| async { self.inner.commit().await.map(|_| Unit) })
            .await
            .map(|_| ())
        }

        async fn rollback(&self) -> Result<(), GraphError> {
            init_logging();
            Durability::<Unit, GraphError>::new(
                "golem_graph_transaction",
                "rollback",
                DurableFunctionType::WriteRemote,
                &NoInput,
            )
            .run_async(|| async { self.inner.rollback().await.map(|_| Unit) })
            .await
            .map(|_| ())
        }

        fn is_active(&self) -> bool {
            self.inner.is_active()
        }
    }
}

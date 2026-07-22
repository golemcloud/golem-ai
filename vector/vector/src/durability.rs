use crate::model::types::VectorError;
use crate::{
    AnalyticsProvider, CollectionProvider, ConnectionProvider, FuncProvider, NamespacesProvider,
    SearchExtendedProvider, SearchProvider, VectorsProvider,
};
use std::marker::PhantomData;

pub struct DurableVector<Impl> {
    _phantom: PhantomData<Impl>,
}

/// Trait used by `DurableVector<Impl>` to implement durability.
///
/// All seven sub-traits (`ConnectionProvider`, `CollectionProvider`,
/// `VectorsProvider`, `SearchProvider`, `SearchExtendedProvider`,
/// `AnalyticsProvider`, `NamespacesProvider`) must agree on the same
/// `ProviderConfig` type so the durable wrapper can thread a single
/// `provider_config` value through every method on every trait.
#[allow(async_fn_in_trait)]
pub trait ExtendedVectorProvider:
    ConnectionProvider
    + CollectionProvider<ProviderConfig = <Self as ConnectionProvider>::ProviderConfig>
    + VectorsProvider<ProviderConfig = <Self as ConnectionProvider>::ProviderConfig>
    + SearchProvider<ProviderConfig = <Self as ConnectionProvider>::ProviderConfig>
    + SearchExtendedProvider<ProviderConfig = <Self as ConnectionProvider>::ProviderConfig>
    + AnalyticsProvider<ProviderConfig = <Self as ConnectionProvider>::ProviderConfig>
    + NamespacesProvider<ProviderConfig = <Self as ConnectionProvider>::ProviderConfig>
    + 'static
{
    async fn connect_internal(
        provider_config: <Self as ConnectionProvider>::ProviderConfig,
        endpoint: &str,
        credentials: &Option<crate::model::connection::Credentials>,
        timeout_ms: &Option<u32>,
        options: &Option<crate::model::types::Metadata>,
    ) -> Result<(), VectorError>;
}

impl<T: ExtendedVectorProvider> FuncProvider for T {
    type MetadataFunc = crate::model::types::MetadataValue;
    type FilterFunc = crate::model::types::FilterExpression;
}

impl<Impl: ExtendedVectorProvider> FuncProvider for DurableVector<Impl> {
    type MetadataFunc = crate::model::types::MetadataValue;
    type FilterFunc = crate::model::types::FilterExpression;
}

/// When the durability feature flag is off, `DurableVector<Impl>` is a transparent wrapper that
/// forwards every call to the inner provider without any oplog persistence.
#[cfg(not(feature = "golem"))]
mod passthrough_impl {
    use super::*;
    use crate::init_logging;

    impl<Impl: ExtendedVectorProvider> ConnectionProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        async fn connect(
            provider_config: Self::ProviderConfig,
            endpoint: String,
            credentials: Option<crate::model::connection::Credentials>,
            timeout_ms: Option<u32>,
            options: Option<crate::model::types::Metadata>,
        ) -> Result<(), VectorError> {
            init_logging();
            Impl::connect_internal(
                provider_config,
                &endpoint,
                &credentials,
                &timeout_ms,
                &options,
            )
            .await
        }

        async fn disconnect(provider_config: Self::ProviderConfig) -> Result<(), VectorError> {
            init_logging();
            Impl::disconnect(provider_config).await
        }

        async fn get_connection_status(
            provider_config: Self::ProviderConfig,
        ) -> Result<crate::model::connection::ConnectionStatus, VectorError> {
            init_logging();
            Impl::get_connection_status(provider_config).await
        }

        async fn test_connection(
            provider_config: Self::ProviderConfig,
            endpoint: String,
            credentials: Option<crate::model::connection::Credentials>,
            timeout_ms: Option<u32>,
            options: Option<crate::model::types::Metadata>,
        ) -> Result<bool, VectorError> {
            init_logging();
            Impl::test_connection(provider_config, endpoint, credentials, timeout_ms, options).await
        }
    }

    impl<Impl: ExtendedVectorProvider> CollectionProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        async fn upsert_collection(
            provider_config: Self::ProviderConfig,
            name: String,
            description: Option<String>,
            dimension: u32,
            metric: crate::model::types::DistanceMetric,
            index_config: Option<crate::model::collections::IndexConfig>,
            metadata: Option<crate::model::types::Metadata>,
        ) -> Result<crate::model::collections::CollectionInfo, VectorError> {
            init_logging();
            Impl::upsert_collection(
                provider_config,
                name,
                description,
                dimension,
                metric,
                index_config,
                metadata,
            )
            .await
        }

        async fn list_collections(
            provider_config: Self::ProviderConfig,
        ) -> Result<Vec<String>, VectorError> {
            init_logging();
            Impl::list_collections(provider_config).await
        }

        async fn get_collection(
            provider_config: Self::ProviderConfig,
            name: String,
        ) -> Result<crate::model::collections::CollectionInfo, VectorError> {
            init_logging();
            Impl::get_collection(provider_config, name).await
        }

        async fn update_collection(
            provider_config: Self::ProviderConfig,
            name: String,
            description: Option<String>,
            metadata: Option<crate::model::types::Metadata>,
        ) -> Result<crate::model::collections::CollectionInfo, VectorError> {
            init_logging();
            Impl::update_collection(provider_config, name, description, metadata).await
        }

        async fn delete_collection(
            provider_config: Self::ProviderConfig,
            name: String,
        ) -> Result<(), VectorError> {
            init_logging();
            Impl::delete_collection(provider_config, name).await
        }

        async fn collection_exists(
            provider_config: Self::ProviderConfig,
            name: String,
        ) -> Result<bool, VectorError> {
            init_logging();
            Impl::collection_exists(provider_config, name).await
        }
    }

    impl<Impl: ExtendedVectorProvider> VectorsProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        async fn upsert_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            vectors: Vec<crate::model::types::VectorRecord>,
            namespace: Option<String>,
        ) -> Result<crate::model::vectors::BatchResult, VectorError> {
            init_logging();
            Impl::upsert_vectors(provider_config, collection, vectors, namespace).await
        }

        async fn upsert_vector(
            provider_config: Self::ProviderConfig,
            collection: String,
            id: crate::model::types::Id,
            vector: crate::model::types::VectorData,
            metadata: Option<crate::model::types::Metadata>,
            namespace: Option<String>,
        ) -> Result<(), VectorError> {
            init_logging();
            Impl::upsert_vector(provider_config, collection, id, vector, metadata, namespace).await
        }

        async fn get_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            ids: Vec<crate::model::types::Id>,
            namespace: Option<String>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
        ) -> Result<Vec<crate::model::types::VectorRecord>, VectorError> {
            init_logging();
            Impl::get_vectors(
                provider_config,
                collection,
                ids,
                namespace,
                include_vectors,
                include_metadata,
            )
            .await
        }

        async fn get_vector(
            provider_config: Self::ProviderConfig,
            collection: String,
            id: crate::model::types::Id,
            namespace: Option<String>,
        ) -> Result<Option<crate::model::types::VectorRecord>, VectorError> {
            init_logging();
            Impl::get_vector(provider_config, collection, id, namespace).await
        }

        async fn update_vector(
            provider_config: Self::ProviderConfig,
            collection: String,
            id: crate::model::types::Id,
            vector: Option<crate::model::types::VectorData>,
            metadata: Option<crate::model::types::Metadata>,
            namespace: Option<String>,
            merge_metadata: Option<bool>,
        ) -> Result<(), VectorError> {
            init_logging();
            Impl::update_vector(
                provider_config,
                collection,
                id,
                vector,
                metadata,
                namespace,
                merge_metadata,
            )
            .await
        }

        async fn delete_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            ids: Vec<crate::model::types::Id>,
            namespace: Option<String>,
        ) -> Result<u32, VectorError> {
            init_logging();
            Impl::delete_vectors(provider_config, collection, ids, namespace).await
        }

        async fn delete_by_filter(
            provider_config: Self::ProviderConfig,
            collection: String,
            filter: crate::model::types::FilterExpression,
            namespace: Option<String>,
        ) -> Result<u32, VectorError> {
            init_logging();
            Impl::delete_by_filter(provider_config, collection, filter, namespace).await
        }

        async fn delete_namespace(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: String,
        ) -> Result<u32, VectorError> {
            init_logging();
            <Impl as VectorsProvider>::delete_namespace(provider_config, collection, namespace)
                .await
        }

        async fn list_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: Option<String>,
            filter: Option<crate::model::types::FilterExpression>,
            limit: Option<u32>,
            cursor: Option<String>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
        ) -> Result<crate::model::vectors::ListResponse, VectorError> {
            init_logging();
            Impl::list_vectors(
                provider_config,
                collection,
                namespace,
                filter,
                limit,
                cursor,
                include_vectors,
                include_metadata,
            )
            .await
        }

        async fn count_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            filter: Option<crate::model::types::FilterExpression>,
            namespace: Option<String>,
        ) -> Result<u64, VectorError> {
            init_logging();
            Impl::count_vectors(provider_config, collection, filter, namespace).await
        }
    }

    impl<Impl: ExtendedVectorProvider> SearchProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        async fn search_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            query: crate::model::search::SearchQuery,
            limit: u32,
            filter: Option<crate::model::types::FilterExpression>,
            namespace: Option<String>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
            min_score: Option<f32>,
            max_distance: Option<f32>,
            search_params: Option<Vec<(String, String)>>,
        ) -> Result<Vec<crate::model::types::SearchResult>, VectorError> {
            init_logging();
            Impl::search_vectors(
                provider_config,
                collection,
                query,
                limit,
                filter,
                namespace,
                include_vectors,
                include_metadata,
                min_score,
                max_distance,
                search_params,
            )
            .await
        }

        async fn find_similar(
            provider_config: Self::ProviderConfig,
            collection: String,
            vector: crate::model::types::VectorData,
            limit: u32,
            namespace: Option<String>,
        ) -> Result<Vec<crate::model::types::SearchResult>, VectorError> {
            init_logging();
            Impl::find_similar(provider_config, collection, vector, limit, namespace).await
        }

        async fn batch_search(
            provider_config: Self::ProviderConfig,
            collection: String,
            queries: Vec<crate::model::search::SearchQuery>,
            limit: u32,
            filter: Option<crate::model::types::FilterExpression>,
            namespace: Option<String>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
            search_params: Option<Vec<(String, String)>>,
        ) -> Result<Vec<Vec<crate::model::types::SearchResult>>, VectorError> {
            init_logging();
            Impl::batch_search(
                provider_config,
                collection,
                queries,
                limit,
                filter,
                namespace,
                include_vectors,
                include_metadata,
                search_params,
            )
            .await
        }
    }

    impl<Impl: ExtendedVectorProvider> SearchExtendedProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        async fn recommend_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            positive: Vec<crate::model::search_extended::RecommendationExample>,
            negative: Option<Vec<crate::model::search_extended::RecommendationExample>>,
            limit: u32,
            filter: Option<crate::model::types::FilterExpression>,
            namespace: Option<String>,
            strategy: Option<crate::model::search_extended::RecommendationStrategy>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
        ) -> Result<Vec<crate::model::types::SearchResult>, VectorError> {
            init_logging();
            Impl::recommend_vectors(
                provider_config,
                collection,
                positive,
                negative,
                limit,
                filter,
                namespace,
                strategy,
                include_vectors,
                include_metadata,
            )
            .await
        }

        async fn discover_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            target: Option<crate::model::search_extended::RecommendationExample>,
            context_pairs: Vec<crate::model::search_extended::ContextPair>,
            limit: u32,
            filter: Option<crate::model::types::FilterExpression>,
            namespace: Option<String>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
        ) -> Result<Vec<crate::model::types::SearchResult>, VectorError> {
            init_logging();
            Impl::discover_vectors(
                provider_config,
                collection,
                target,
                context_pairs,
                limit,
                filter,
                namespace,
                include_vectors,
                include_metadata,
            )
            .await
        }

        async fn search_groups(
            provider_config: Self::ProviderConfig,
            collection: String,
            query: crate::model::search::SearchQuery,
            group_by: String,
            group_size: u32,
            max_groups: u32,
            filter: Option<crate::model::types::FilterExpression>,
            namespace: Option<String>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
        ) -> Result<Vec<crate::model::search_extended::GroupedSearchResult>, VectorError> {
            init_logging();
            Impl::search_groups(
                provider_config,
                collection,
                query,
                group_by,
                group_size,
                max_groups,
                filter,
                namespace,
                include_vectors,
                include_metadata,
            )
            .await
        }

        async fn search_range(
            provider_config: Self::ProviderConfig,
            collection: String,
            vector: crate::model::types::VectorData,
            min_distance: Option<f32>,
            max_distance: f32,
            filter: Option<crate::model::types::FilterExpression>,
            namespace: Option<String>,
            limit: Option<u32>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
        ) -> Result<Vec<crate::model::types::SearchResult>, VectorError> {
            init_logging();
            Impl::search_range(
                provider_config,
                collection,
                vector,
                min_distance,
                max_distance,
                filter,
                namespace,
                limit,
                include_vectors,
                include_metadata,
            )
            .await
        }

        async fn search_text(
            provider_config: Self::ProviderConfig,
            collection: String,
            query_text: String,
            limit: u32,
            filter: Option<crate::model::types::FilterExpression>,
            namespace: Option<String>,
        ) -> Result<Vec<crate::model::types::SearchResult>, VectorError> {
            init_logging();
            Impl::search_text(
                provider_config,
                collection,
                query_text,
                limit,
                filter,
                namespace,
            )
            .await
        }
    }

    impl<Impl: ExtendedVectorProvider> AnalyticsProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        async fn get_collection_stats(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: Option<String>,
        ) -> Result<crate::model::analytics::CollectionStats, VectorError> {
            init_logging();
            Impl::get_collection_stats(provider_config, collection, namespace).await
        }

        async fn get_field_stats(
            provider_config: Self::ProviderConfig,
            collection: String,
            field: String,
            namespace: Option<String>,
        ) -> Result<crate::model::analytics::FieldStats, VectorError> {
            init_logging();
            Impl::get_field_stats(provider_config, collection, field, namespace).await
        }

        async fn get_field_distribution(
            provider_config: Self::ProviderConfig,
            collection: String,
            field: String,
            limit: Option<u32>,
            namespace: Option<String>,
        ) -> Result<Vec<(crate::model::types::MetadataValue, u64)>, VectorError> {
            init_logging();
            Impl::get_field_distribution(provider_config, collection, field, limit, namespace).await
        }
    }

    impl<Impl: ExtendedVectorProvider> NamespacesProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        async fn upsert_namespace(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: String,
            metadata: Option<crate::model::types::Metadata>,
        ) -> Result<crate::model::namespaces::NamespaceInfo, VectorError> {
            init_logging();
            Impl::upsert_namespace(provider_config, collection, namespace, metadata).await
        }

        async fn list_namespaces(
            provider_config: Self::ProviderConfig,
            collection: String,
        ) -> Result<Vec<crate::model::namespaces::NamespaceInfo>, VectorError> {
            init_logging();
            Impl::list_namespaces(provider_config, collection).await
        }

        async fn get_namespace(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: String,
        ) -> Result<crate::model::namespaces::NamespaceInfo, VectorError> {
            init_logging();
            Impl::get_namespace(provider_config, collection, namespace).await
        }

        async fn delete_namespace(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: String,
        ) -> Result<(), VectorError> {
            init_logging();
            <Impl as NamespacesProvider>::delete_namespace(provider_config, collection, namespace)
                .await
        }

        async fn namespace_exists(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: String,
        ) -> Result<bool, VectorError> {
            init_logging();
            Impl::namespace_exists(provider_config, collection, namespace).await
        }
    }
}

#[cfg(feature = "golem")]
mod durable_impl {
    use super::*;
    use crate::init_logging;
    use golem_rust::durability::Durability;
    use golem_rust::durability::DurableFunctionType;
    use golem_rust::{with_persistence_level_async, FromSchema, IntoSchema, PersistenceLevel};

    #[derive(Debug, Clone, FromSchema, IntoSchema)]
    pub(super) struct Unit;

    impl<Impl: ExtendedVectorProvider> ConnectionProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        async fn connect(
            provider_config: Self::ProviderConfig,
            endpoint: String,
            credentials: Option<crate::model::connection::Credentials>,
            timeout_ms: Option<u32>,
            options: Option<crate::model::types::Metadata>,
        ) -> Result<(), VectorError> {
            init_logging();
            let durability = Durability::<Unit, VectorError>::new(
                "golem_ai_vector",
                "connect",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::connect_internal(
                            provider_config,
                            &endpoint,
                            &credentials,
                            &timeout_ms,
                            &options,
                        )
                        .await
                    })
                    .await;
                // NOTE: `provider_config` deliberately not included in the persisted input,
                // because it can carry secrets (API keys etc.).
                durability.persist(
                    ConnectParams {
                        endpoint,
                        credentials,
                        timeout_ms,
                        options,
                    },
                    result.map(|_| Unit),
                )?;
                Ok(())
            } else {
                durability.replay::<Unit, VectorError>()?;
                Ok(())
            }
        }

        async fn disconnect(provider_config: Self::ProviderConfig) -> Result<(), VectorError> {
            init_logging();
            let durability = Durability::<Unit, VectorError>::new(
                "golem_ai_vector",
                "disconnect",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::disconnect(provider_config).await
                    })
                    .await;
                durability.persist(Unit, result.map(|_| Unit))?;
                Ok(())
            } else {
                durability.replay::<Unit, VectorError>()?;
                Ok(())
            }
        }

        async fn get_connection_status(
            provider_config: Self::ProviderConfig,
        ) -> Result<crate::model::connection::ConnectionStatus, VectorError> {
            init_logging();
            let durability: Durability<crate::model::connection::ConnectionStatus, VectorError> =
                Durability::new(
                    "golem_ai_vector",
                    "get_connection_status",
                    DurableFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::get_connection_status(provider_config).await
                    })
                    .await;
                durability.persist(Unit, result)
            } else {
                durability.replay()
            }
        }

        async fn test_connection(
            provider_config: Self::ProviderConfig,
            endpoint: String,
            credentials: Option<crate::model::connection::Credentials>,
            timeout_ms: Option<u32>,
            options: Option<crate::model::types::Metadata>,
        ) -> Result<bool, VectorError> {
            init_logging();
            let durability: Durability<bool, VectorError> = Durability::new(
                "golem_ai_vector",
                "test_connection",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::test_connection(
                            provider_config,
                            endpoint.clone(),
                            credentials.clone(),
                            timeout_ms,
                            options.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    ConnectParams {
                        endpoint,
                        credentials,
                        timeout_ms,
                        options,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }
    }

    impl<Impl: ExtendedVectorProvider> CollectionProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        async fn upsert_collection(
            provider_config: Self::ProviderConfig,
            name: String,
            description: Option<String>,
            dimension: u32,
            metric: crate::model::types::DistanceMetric,
            index_config: Option<crate::model::collections::IndexConfig>,
            metadata: Option<crate::model::types::Metadata>,
        ) -> Result<crate::model::collections::CollectionInfo, VectorError> {
            init_logging();
            let durability: Durability<crate::model::collections::CollectionInfo, VectorError> =
                Durability::new(
                    "golem_vector_collections",
                    "upsert_collection",
                    DurableFunctionType::WriteRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::upsert_collection(
                            provider_config,
                            name.clone(),
                            description.clone(),
                            dimension,
                            metric,
                            index_config.clone(),
                            metadata.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    UpsertCollectionParams {
                        name,
                        description,
                        dimension,
                        metric,
                        index_config,
                        metadata,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }

        async fn list_collections(
            provider_config: Self::ProviderConfig,
        ) -> Result<Vec<String>, VectorError> {
            init_logging();
            let durability: Durability<Vec<String>, VectorError> = Durability::new(
                "golem_vector_collections",
                "list_collections",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::list_collections(provider_config).await
                    })
                    .await;
                durability.persist(Unit, result)
            } else {
                durability.replay()
            }
        }

        async fn get_collection(
            provider_config: Self::ProviderConfig,
            name: String,
        ) -> Result<crate::model::collections::CollectionInfo, VectorError> {
            init_logging();
            let durability: Durability<crate::model::collections::CollectionInfo, VectorError> =
                Durability::new(
                    "golem_vector_collections",
                    "get_collection",
                    DurableFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::get_collection(provider_config, name.clone()).await
                    })
                    .await;
                durability.persist(name, result)
            } else {
                durability.replay()
            }
        }

        async fn update_collection(
            provider_config: Self::ProviderConfig,
            name: String,
            description: Option<String>,
            metadata: Option<crate::model::types::Metadata>,
        ) -> Result<crate::model::collections::CollectionInfo, VectorError> {
            init_logging();
            let durability: Durability<crate::model::collections::CollectionInfo, VectorError> =
                Durability::new(
                    "golem_vector_collections",
                    "update_collection",
                    DurableFunctionType::WriteRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::update_collection(
                            provider_config,
                            name.clone(),
                            description.clone(),
                            metadata.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    UpdateCollectionParams {
                        name,
                        description,
                        metadata,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }

        async fn delete_collection(
            provider_config: Self::ProviderConfig,
            name: String,
        ) -> Result<(), VectorError> {
            init_logging();
            let durability: Durability<Unit, VectorError> = Durability::new(
                "golem_vector_collections",
                "delete_collection",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::delete_collection(provider_config, name.clone()).await
                    })
                    .await;
                durability.persist(name, result.map(|_| Unit))?;
                Ok(())
            } else {
                durability.replay::<Unit, VectorError>()?;
                Ok(())
            }
        }

        async fn collection_exists(
            provider_config: Self::ProviderConfig,
            name: String,
        ) -> Result<bool, VectorError> {
            init_logging();
            let durability: Durability<bool, VectorError> = Durability::new(
                "golem_vector_collections",
                "collection_exists",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::collection_exists(provider_config, name.clone()).await
                    })
                    .await;
                durability.persist(name, result)
            } else {
                durability.replay()
            }
        }
    }

    impl<Impl: ExtendedVectorProvider> VectorsProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        async fn upsert_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            vectors: Vec<crate::model::types::VectorRecord>,
            namespace: Option<String>,
        ) -> Result<crate::model::vectors::BatchResult, VectorError> {
            init_logging();
            let durability: Durability<crate::model::vectors::BatchResult, VectorError> =
                Durability::new(
                    "golem_vector_vectors",
                    "upsert_vectors",
                    DurableFunctionType::WriteRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::upsert_vectors(
                            provider_config,
                            collection.clone(),
                            vectors.clone(),
                            namespace.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    UpsertVectorsParams {
                        collection,
                        vectors,
                        namespace,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }

        async fn upsert_vector(
            provider_config: Self::ProviderConfig,
            collection: String,
            id: crate::model::types::Id,
            vector: crate::model::types::VectorData,
            metadata: Option<crate::model::types::Metadata>,
            namespace: Option<String>,
        ) -> Result<(), VectorError> {
            init_logging();
            let durability: Durability<Unit, VectorError> = Durability::new(
                "golem_vector_vectors",
                "upsert_vector",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::upsert_vector(
                            provider_config,
                            collection.clone(),
                            id.clone(),
                            vector.clone(),
                            metadata.clone(),
                            namespace.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    UpsertVectorParams {
                        collection,
                        id,
                        vector,
                        metadata,
                        namespace,
                    },
                    result.map(|_| Unit),
                )?;
                Ok(())
            } else {
                durability.replay::<Unit, VectorError>()?;
                Ok(())
            }
        }

        async fn get_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            ids: Vec<crate::model::types::Id>,
            namespace: Option<String>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
        ) -> Result<Vec<crate::model::types::VectorRecord>, VectorError> {
            init_logging();
            let durability: Durability<Vec<crate::model::types::VectorRecord>, VectorError> =
                Durability::new(
                    "golem_vector_vectors",
                    "get_vectors",
                    DurableFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::get_vectors(
                            provider_config,
                            collection.clone(),
                            ids.clone(),
                            namespace.clone(),
                            include_vectors,
                            include_metadata,
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    GetVectorsParams {
                        collection,
                        ids,
                        namespace,
                        include_vectors,
                        include_metadata,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }

        async fn get_vector(
            provider_config: Self::ProviderConfig,
            collection: String,
            id: crate::model::types::Id,
            namespace: Option<String>,
        ) -> Result<Option<crate::model::types::VectorRecord>, VectorError> {
            init_logging();
            let durability: Durability<Option<crate::model::types::VectorRecord>, VectorError> =
                Durability::new(
                    "golem_vector_vectors",
                    "get_vector",
                    DurableFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::get_vector(
                            provider_config,
                            collection.clone(),
                            id.clone(),
                            namespace.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    GetVectorParams {
                        collection,
                        id,
                        namespace,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }

        async fn update_vector(
            provider_config: Self::ProviderConfig,
            collection: String,
            id: crate::model::types::Id,
            vector: Option<crate::model::types::VectorData>,
            metadata: Option<crate::model::types::Metadata>,
            namespace: Option<String>,
            merge_metadata: Option<bool>,
        ) -> Result<(), VectorError> {
            init_logging();
            let durability: Durability<Unit, VectorError> = Durability::new(
                "golem_vector_vectors",
                "update_vector",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::update_vector(
                            provider_config,
                            collection.clone(),
                            id.clone(),
                            vector.clone(),
                            metadata.clone(),
                            namespace.clone(),
                            merge_metadata,
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    UpdateVectorParams {
                        collection,
                        id,
                        vector,
                        metadata,
                        namespace,
                        merge_metadata,
                    },
                    result.map(|_| Unit),
                )?;
                Ok(())
            } else {
                durability.replay::<Unit, VectorError>()?;
                Ok(())
            }
        }

        async fn delete_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            ids: Vec<crate::model::types::Id>,
            namespace: Option<String>,
        ) -> Result<u32, VectorError> {
            init_logging();
            let durability: Durability<u32, VectorError> = Durability::new(
                "golem_vector_vectors",
                "delete_vectors",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::delete_vectors(
                            provider_config,
                            collection.clone(),
                            ids.clone(),
                            namespace.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    DeleteVectorsParams {
                        collection,
                        ids,
                        namespace,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }

        async fn delete_by_filter(
            provider_config: Self::ProviderConfig,
            collection: String,
            filter: crate::model::types::FilterExpression,
            namespace: Option<String>,
        ) -> Result<u32, VectorError> {
            init_logging();
            let durability: Durability<u32, VectorError> = Durability::new(
                "golem_vector_vectors",
                "delete_by_filter",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::delete_by_filter(
                            provider_config,
                            collection.clone(),
                            filter.clone(),
                            namespace.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    DeleteByFilterParams {
                        collection,
                        filter,
                        namespace,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }

        async fn delete_namespace(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: String,
        ) -> Result<u32, VectorError> {
            init_logging();
            let durability: Durability<u32, VectorError> = Durability::new(
                "golem_vector_vectors",
                "delete_namespace",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        <Impl as VectorsProvider>::delete_namespace(
                            provider_config,
                            collection.clone(),
                            namespace.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist((collection, namespace), result)
            } else {
                durability.replay()
            }
        }

        async fn list_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: Option<String>,
            filter: Option<crate::model::types::FilterExpression>,
            limit: Option<u32>,
            cursor: Option<String>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
        ) -> Result<crate::model::vectors::ListResponse, VectorError> {
            init_logging();
            let durability: Durability<crate::model::vectors::ListResponse, VectorError> =
                Durability::new(
                    "golem_vector_vectors",
                    "list_vectors",
                    DurableFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::list_vectors(
                            provider_config,
                            collection.clone(),
                            namespace.clone(),
                            filter.clone(),
                            limit,
                            cursor.clone(),
                            include_vectors,
                            include_metadata,
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    ListVectorsParams {
                        collection,
                        namespace,
                        filter,
                        limit,
                        cursor,
                        include_vectors,
                        include_metadata,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }

        async fn count_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            filter: Option<crate::model::types::FilterExpression>,
            namespace: Option<String>,
        ) -> Result<u64, VectorError> {
            init_logging();
            let durability: Durability<u64, VectorError> = Durability::new(
                "golem_vector_vectors",
                "count_vectors",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::count_vectors(
                            provider_config,
                            collection.clone(),
                            filter.clone(),
                            namespace.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    CountVectorsParams {
                        collection,
                        filter,
                        namespace,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }
    }

    impl<Impl: ExtendedVectorProvider> SearchProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        async fn search_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            query: crate::model::search::SearchQuery,
            limit: u32,
            filter: Option<crate::model::types::FilterExpression>,
            namespace: Option<String>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
            min_score: Option<f32>,
            max_distance: Option<f32>,
            search_params: Option<Vec<(String, String)>>,
        ) -> Result<Vec<crate::model::types::SearchResult>, VectorError> {
            init_logging();
            let durability: Durability<Vec<crate::model::types::SearchResult>, VectorError> =
                Durability::new(
                    "golem_vector_search",
                    "search_vectors",
                    DurableFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::search_vectors(
                            provider_config,
                            collection.clone(),
                            query.clone(),
                            limit,
                            filter.clone(),
                            namespace.clone(),
                            include_vectors,
                            include_metadata,
                            min_score,
                            max_distance,
                            search_params.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    SearchVectorsParams {
                        collection,
                        query,
                        limit,
                        filter,
                        namespace,
                        include_vectors,
                        include_metadata,
                        min_score,
                        max_distance,
                        search_params,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }

        async fn find_similar(
            provider_config: Self::ProviderConfig,
            collection: String,
            vector: crate::model::types::VectorData,
            limit: u32,
            namespace: Option<String>,
        ) -> Result<Vec<crate::model::types::SearchResult>, VectorError> {
            init_logging();
            let durability: Durability<Vec<crate::model::types::SearchResult>, VectorError> =
                Durability::new(
                    "golem_vector_search",
                    "find_similar",
                    DurableFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::find_similar(
                            provider_config,
                            collection.clone(),
                            vector.clone(),
                            limit,
                            namespace.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    FindSimilarParams {
                        collection,
                        vector,
                        limit,
                        namespace,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }

        async fn batch_search(
            provider_config: Self::ProviderConfig,
            collection: String,
            queries: Vec<crate::model::search::SearchQuery>,
            limit: u32,
            filter: Option<crate::model::types::FilterExpression>,
            namespace: Option<String>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
            search_params: Option<Vec<(String, String)>>,
        ) -> Result<Vec<Vec<crate::model::types::SearchResult>>, VectorError> {
            init_logging();
            let durability: Durability<Vec<Vec<crate::model::types::SearchResult>>, VectorError> =
                Durability::new(
                    "golem_vector_search",
                    "batch_search",
                    DurableFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::batch_search(
                            provider_config,
                            collection.clone(),
                            queries.clone(),
                            limit,
                            filter.clone(),
                            namespace.clone(),
                            include_vectors,
                            include_metadata,
                            search_params.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    BatchSearchParams {
                        collection,
                        queries,
                        limit,
                        filter,
                        namespace,
                        include_vectors,
                        include_metadata,
                        search_params,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }
    }

    impl<Impl: ExtendedVectorProvider> SearchExtendedProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        async fn recommend_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            positive: Vec<crate::model::search_extended::RecommendationExample>,
            negative: Option<Vec<crate::model::search_extended::RecommendationExample>>,
            limit: u32,
            filter: Option<crate::model::types::FilterExpression>,
            namespace: Option<String>,
            strategy: Option<crate::model::search_extended::RecommendationStrategy>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
        ) -> Result<Vec<crate::model::types::SearchResult>, VectorError> {
            init_logging();
            let durability: Durability<Vec<crate::model::types::SearchResult>, VectorError> =
                Durability::new(
                    "golem_vector_search_extended",
                    "recommend_vectors",
                    DurableFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::recommend_vectors(
                            provider_config,
                            collection.clone(),
                            positive.clone(),
                            negative.clone(),
                            limit,
                            filter.clone(),
                            namespace.clone(),
                            strategy,
                            include_vectors,
                            include_metadata,
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    RecommendVectorsParams {
                        collection,
                        positive,
                        negative,
                        limit,
                        filter,
                        namespace,
                        strategy,
                        include_vectors,
                        include_metadata,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }

        async fn discover_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            target: Option<crate::model::search_extended::RecommendationExample>,
            context_pairs: Vec<crate::model::search_extended::ContextPair>,
            limit: u32,
            filter: Option<crate::model::types::FilterExpression>,
            namespace: Option<String>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
        ) -> Result<Vec<crate::model::types::SearchResult>, VectorError> {
            init_logging();
            let durability: Durability<Vec<crate::model::types::SearchResult>, VectorError> =
                Durability::new(
                    "golem_vector_search_extended",
                    "discover_vectors",
                    DurableFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::discover_vectors(
                            provider_config,
                            collection.clone(),
                            target.clone(),
                            context_pairs.clone(),
                            limit,
                            filter.clone(),
                            namespace.clone(),
                            include_vectors,
                            include_metadata,
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    DiscoverVectorsParams {
                        collection,
                        target,
                        context_pairs,
                        limit,
                        filter,
                        namespace,
                        include_vectors,
                        include_metadata,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }

        async fn search_groups(
            provider_config: Self::ProviderConfig,
            collection: String,
            query: crate::model::search::SearchQuery,
            group_by: String,
            group_size: u32,
            max_groups: u32,
            filter: Option<crate::model::types::FilterExpression>,
            namespace: Option<String>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
        ) -> Result<Vec<crate::model::search_extended::GroupedSearchResult>, VectorError> {
            init_logging();
            let durability: Durability<
                Vec<crate::model::search_extended::GroupedSearchResult>,
                VectorError,
            > = Durability::new(
                "golem_vector_search_extended",
                "search_groups",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::search_groups(
                            provider_config,
                            collection.clone(),
                            query.clone(),
                            group_by.clone(),
                            group_size,
                            max_groups,
                            filter.clone(),
                            namespace.clone(),
                            include_vectors,
                            include_metadata,
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    SearchGroupsParams {
                        collection,
                        query,
                        group_by,
                        group_size,
                        max_groups,
                        filter,
                        namespace,
                        include_vectors,
                        include_metadata,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }

        async fn search_range(
            provider_config: Self::ProviderConfig,
            collection: String,
            vector: crate::model::types::VectorData,
            min_distance: Option<f32>,
            max_distance: f32,
            filter: Option<crate::model::types::FilterExpression>,
            namespace: Option<String>,
            limit: Option<u32>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
        ) -> Result<Vec<crate::model::types::SearchResult>, VectorError> {
            init_logging();
            let durability: Durability<Vec<crate::model::types::SearchResult>, VectorError> =
                Durability::new(
                    "golem_vector_search_extended",
                    "search_range",
                    DurableFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::search_range(
                            provider_config,
                            collection.clone(),
                            vector.clone(),
                            min_distance,
                            max_distance,
                            filter.clone(),
                            namespace.clone(),
                            limit,
                            include_vectors,
                            include_metadata,
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    SearchRangeParams {
                        collection,
                        vector,
                        min_distance,
                        max_distance,
                        filter,
                        namespace,
                        limit,
                        include_vectors,
                        include_metadata,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }

        async fn search_text(
            provider_config: Self::ProviderConfig,
            collection: String,
            query_text: String,
            limit: u32,
            filter: Option<crate::model::types::FilterExpression>,
            namespace: Option<String>,
        ) -> Result<Vec<crate::model::types::SearchResult>, VectorError> {
            init_logging();
            let durability: Durability<Vec<crate::model::types::SearchResult>, VectorError> =
                Durability::new(
                    "golem_vector_search_extended",
                    "search_text",
                    DurableFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::search_text(
                            provider_config,
                            collection.clone(),
                            query_text.clone(),
                            limit,
                            filter.clone(),
                            namespace.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    SearchTextParams {
                        collection,
                        query_text,
                        limit,
                        filter,
                        namespace,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }
    }

    impl<Impl: ExtendedVectorProvider> AnalyticsProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        async fn get_collection_stats(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: Option<String>,
        ) -> Result<crate::model::analytics::CollectionStats, VectorError> {
            init_logging();
            let durability: Durability<crate::model::analytics::CollectionStats, VectorError> =
                Durability::new(
                    "golem_vector_analytics",
                    "get_collection_stats",
                    DurableFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::get_collection_stats(
                            provider_config,
                            collection.clone(),
                            namespace.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist((collection, namespace), result)
            } else {
                durability.replay()
            }
        }

        async fn get_field_stats(
            provider_config: Self::ProviderConfig,
            collection: String,
            field: String,
            namespace: Option<String>,
        ) -> Result<crate::model::analytics::FieldStats, VectorError> {
            init_logging();
            let durability: Durability<crate::model::analytics::FieldStats, VectorError> =
                Durability::new(
                    "golem_vector_analytics",
                    "get_field_stats",
                    DurableFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::get_field_stats(
                            provider_config,
                            collection.clone(),
                            field.clone(),
                            namespace.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist((collection, field, namespace), result)
            } else {
                durability.replay()
            }
        }

        async fn get_field_distribution(
            provider_config: Self::ProviderConfig,
            collection: String,
            field: String,
            limit: Option<u32>,
            namespace: Option<String>,
        ) -> Result<Vec<(crate::model::types::MetadataValue, u64)>, VectorError> {
            init_logging();
            let durability: Durability<
                Vec<(crate::model::types::MetadataValue, u64)>,
                VectorError,
            > = Durability::new(
                "golem_vector_analytics",
                "get_field_distribution",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::get_field_distribution(
                            provider_config,
                            collection.clone(),
                            field.clone(),
                            limit,
                            namespace.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist(
                    GetFieldDistributionParams {
                        collection,
                        field,
                        limit,
                        namespace,
                    },
                    result,
                )
            } else {
                durability.replay()
            }
        }
    }

    impl<Impl: ExtendedVectorProvider> NamespacesProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        async fn upsert_namespace(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: String,
            metadata: Option<crate::model::types::Metadata>,
        ) -> Result<crate::model::namespaces::NamespaceInfo, VectorError> {
            init_logging();
            let durability: Durability<crate::model::namespaces::NamespaceInfo, VectorError> =
                Durability::new(
                    "golem_vector_namespaces",
                    "upsert_namespace",
                    DurableFunctionType::WriteRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::upsert_namespace(
                            provider_config,
                            collection.clone(),
                            namespace.clone(),
                            metadata.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist((collection, namespace, metadata), result)
            } else {
                durability.replay()
            }
        }

        async fn list_namespaces(
            provider_config: Self::ProviderConfig,
            collection: String,
        ) -> Result<Vec<crate::model::namespaces::NamespaceInfo>, VectorError> {
            init_logging();
            let durability: Durability<Vec<crate::model::namespaces::NamespaceInfo>, VectorError> =
                Durability::new(
                    "golem_vector_namespaces",
                    "list_namespaces",
                    DurableFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::list_namespaces(provider_config, collection.clone()).await
                    })
                    .await;
                durability.persist(collection, result)
            } else {
                durability.replay()
            }
        }

        async fn get_namespace(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: String,
        ) -> Result<crate::model::namespaces::NamespaceInfo, VectorError> {
            init_logging();
            let durability: Durability<crate::model::namespaces::NamespaceInfo, VectorError> =
                Durability::new(
                    "golem_vector_namespaces",
                    "get_namespace",
                    DurableFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::get_namespace(provider_config, collection.clone(), namespace.clone())
                            .await
                    })
                    .await;
                durability.persist((collection, namespace), result)
            } else {
                durability.replay()
            }
        }

        async fn delete_namespace(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: String,
        ) -> Result<(), VectorError> {
            init_logging();
            let durability: Durability<Unit, VectorError> = Durability::new(
                "golem_vector_namespaces",
                "delete_namespace",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        <Impl as NamespacesProvider>::delete_namespace(
                            provider_config,
                            collection.clone(),
                            namespace.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist((collection, namespace), result.map(|_| Unit))?;
                Ok(())
            } else {
                durability.replay::<Unit, VectorError>()?;
                Ok(())
            }
        }

        async fn namespace_exists(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: String,
        ) -> Result<bool, VectorError> {
            init_logging();
            let durability: Durability<bool, VectorError> = Durability::new(
                "golem_vector_namespaces",
                "namespace_exists",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async {
                        Impl::namespace_exists(
                            provider_config,
                            collection.clone(),
                            namespace.clone(),
                        )
                        .await
                    })
                    .await;
                durability.persist((collection, namespace), result)
            } else {
                durability.replay()
            }
        }
    }

    // Parameter structures for durability
    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct ConnectParams {
        endpoint: String,
        credentials: Option<crate::model::connection::Credentials>,
        timeout_ms: Option<u32>,
        options: Option<crate::model::types::Metadata>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct UpsertCollectionParams {
        name: String,
        description: Option<String>,
        dimension: u32,
        metric: crate::model::types::DistanceMetric,
        index_config: Option<crate::model::collections::IndexConfig>,
        metadata: Option<crate::model::types::Metadata>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct UpdateCollectionParams {
        name: String,
        description: Option<String>,
        metadata: Option<crate::model::types::Metadata>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct UpsertVectorsParams {
        collection: String,
        vectors: Vec<crate::model::types::VectorRecord>,
        namespace: Option<String>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct UpsertVectorParams {
        collection: String,
        id: crate::model::types::Id,
        vector: crate::model::types::VectorData,
        metadata: Option<crate::model::types::Metadata>,
        namespace: Option<String>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct UpdateVectorParams {
        collection: String,
        id: crate::model::types::Id,
        vector: Option<crate::model::types::VectorData>,
        metadata: Option<crate::model::types::Metadata>,
        namespace: Option<String>,
        merge_metadata: Option<bool>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct DeleteVectorsParams {
        collection: String,
        ids: Vec<crate::model::types::Id>,
        namespace: Option<String>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct DeleteByFilterParams {
        collection: String,
        filter: crate::model::types::FilterExpression,
        namespace: Option<String>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct SearchVectorsParams {
        collection: String,
        query: crate::model::search::SearchQuery,
        limit: u32,
        filter: Option<crate::model::types::FilterExpression>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
        min_score: Option<f32>,
        max_distance: Option<f32>,
        search_params: Option<Vec<(String, String)>>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct BatchSearchParams {
        collection: String,
        queries: Vec<crate::model::search::SearchQuery>,
        limit: u32,
        filter: Option<crate::model::types::FilterExpression>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
        search_params: Option<Vec<(String, String)>>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct GetVectorsParams {
        collection: String,
        ids: Vec<crate::model::types::Id>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct GetVectorParams {
        collection: String,
        id: crate::model::types::Id,
        namespace: Option<String>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct ListVectorsParams {
        collection: String,
        namespace: Option<String>,
        filter: Option<crate::model::types::FilterExpression>,
        limit: Option<u32>,
        cursor: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct CountVectorsParams {
        collection: String,
        filter: Option<crate::model::types::FilterExpression>,
        namespace: Option<String>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct FindSimilarParams {
        collection: String,
        vector: crate::model::types::VectorData,
        limit: u32,
        namespace: Option<String>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct RecommendVectorsParams {
        collection: String,
        positive: Vec<crate::model::search_extended::RecommendationExample>,
        negative: Option<Vec<crate::model::search_extended::RecommendationExample>>,
        limit: u32,
        filter: Option<crate::model::types::FilterExpression>,
        namespace: Option<String>,
        strategy: Option<crate::model::search_extended::RecommendationStrategy>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct DiscoverVectorsParams {
        collection: String,
        target: Option<crate::model::search_extended::RecommendationExample>,
        context_pairs: Vec<crate::model::search_extended::ContextPair>,
        limit: u32,
        filter: Option<crate::model::types::FilterExpression>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct SearchGroupsParams {
        collection: String,
        query: crate::model::search::SearchQuery,
        group_by: String,
        group_size: u32,
        max_groups: u32,
        filter: Option<crate::model::types::FilterExpression>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct SearchRangeParams {
        collection: String,
        vector: crate::model::types::VectorData,
        min_distance: Option<f32>,
        max_distance: f32,
        filter: Option<crate::model::types::FilterExpression>,
        namespace: Option<String>,
        limit: Option<u32>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct SearchTextParams {
        collection: String,
        query_text: String,
        limit: u32,
        filter: Option<crate::model::types::FilterExpression>,
        namespace: Option<String>,
    }

    #[derive(Debug, Clone, FromSchema, IntoSchema, PartialEq)]
    struct GetFieldDistributionParams {
        collection: String,
        field: String,
        limit: Option<u32>,
        namespace: Option<String>,
    }
}

#[cfg(all(test, feature = "golem"))]
mod tests {
    use crate::model::types::{
        DenseVector, DistanceMetric, FilterCondition, FilterExpression, FilterFunc, FilterOperator,
        Id, MetadataFunc, MetadataValue, SearchResult, VectorData, VectorError, VectorRecord,
    };
    use golem_rust::{FromSchema, IntoSchema};
    use std::fmt::Debug;

    fn roundtrip_test<T: Debug + Clone + PartialEq + IntoSchema + FromSchema>(value: T) {
        golem_rust::schema::try_into_schema_graph::<T>().unwrap();
        let schema_value = value.to_value();
        let extracted = T::from_value(&schema_value).unwrap();
        assert_eq!(value, extracted);
    }

    #[test]
    fn vector_error_roundtrip() {
        roundtrip_test(VectorError::NotFound("vector not found".to_string()));
        roundtrip_test(VectorError::AlreadyExists("collection exists".to_string()));
        roundtrip_test(VectorError::InvalidParams("invalid dimension".to_string()));
        roundtrip_test(VectorError::UnsupportedFeature(
            "feature not supported".to_string(),
        ));
        roundtrip_test(VectorError::DimensionMismatch(
            "dimension mismatch".to_string(),
        ));
        roundtrip_test(VectorError::InvalidVector(
            "invalid vector data".to_string(),
        ));
        roundtrip_test(VectorError::Unauthorized("access denied".to_string()));
        roundtrip_test(VectorError::RateLimited("too many requests".to_string()));
        roundtrip_test(VectorError::ProviderError("provider error".to_string()));
        roundtrip_test(VectorError::ConnectionError(
            "connection failed".to_string(),
        ));
    }

    #[test]
    fn vector_data_roundtrip() {
        let dense_vector: DenseVector = vec![1.0, 2.0, 3.0, 4.0];
        roundtrip_test(VectorData::Dense(dense_vector));

        let sparse_vector = crate::model::types::SparseVector {
            indices: vec![0, 2, 4],
            values: vec![1.0, 3.0, 5.0],
            total_dimensions: 10,
        };
        roundtrip_test(VectorData::Sparse(sparse_vector));
    }

    #[test]
    fn metadata_value_roundtrip() {
        roundtrip_test(MetadataValue::StringVal("test".to_string()));
        roundtrip_test(MetadataValue::NumberVal(42.5));
        roundtrip_test(MetadataValue::IntegerVal(123));
        roundtrip_test(MetadataValue::BooleanVal(true));
        roundtrip_test(MetadataValue::NullVal);
        roundtrip_test(MetadataValue::ObjectVal(vec![(
            "nested".to_string(),
            MetadataFunc::new(MetadataValue::ArrayVal(vec![MetadataFunc::new(
                MetadataValue::IntegerVal(7),
            )])),
        )]));
    }

    #[test]
    fn recursive_filter_expression_roundtrip() {
        let condition = FilterExpression::Condition(FilterCondition {
            field: "category".to_string(),
            operator: FilterOperator::Eq,
            value: MetadataValue::StringVal("electronics".to_string()),
        });
        roundtrip_test(FilterExpression::Not(FilterFunc::new(
            FilterExpression::And(vec![FilterFunc::new(condition)]),
        )));
    }

    #[test]
    fn filter_condition_roundtrip() {
        let condition = FilterCondition {
            field: "category".to_string(),
            operator: FilterOperator::Eq,
            value: MetadataValue::StringVal("electronics".to_string()),
        };
        roundtrip_test(condition);
    }

    #[test]
    fn vector_record_roundtrip() {
        let record = VectorRecord {
            id: "vec-123".to_string(),
            vector: VectorData::Dense(vec![1.0, 2.0, 3.0]),
            metadata: Some(vec![
                (
                    "category".to_string(),
                    MetadataValue::StringVal("test".to_string()),
                ),
                ("price".to_string(), MetadataValue::NumberVal(99.99)),
            ]),
        };
        roundtrip_test(record);
    }

    #[test]
    fn search_result_roundtrip() {
        let result = SearchResult {
            id: "result-456".to_string(),
            score: 0.95,
            distance: 0.05,
            vector: Some(VectorData::Dense(vec![0.1, 0.2, 0.3])),
            metadata: Some(vec![(
                "title".to_string(),
                MetadataValue::StringVal("Test Document".to_string()),
            )]),
        };
        roundtrip_test(result);
    }

    #[test]
    fn distance_metric_roundtrip() {
        roundtrip_test(DistanceMetric::Cosine);
        roundtrip_test(DistanceMetric::Euclidean);
        roundtrip_test(DistanceMetric::DotProduct);
        roundtrip_test(DistanceMetric::Manhattan);
        roundtrip_test(DistanceMetric::Hamming);
        roundtrip_test(DistanceMetric::Jaccard);
    }

    #[test]
    fn id_roundtrip() {
        let id: Id = "test-vector-id-123".to_string();
        roundtrip_test(id);
    }
}

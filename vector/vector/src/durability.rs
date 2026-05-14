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
    fn connect_internal(
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

        fn connect(
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
        }

        fn disconnect(provider_config: Self::ProviderConfig) -> Result<(), VectorError> {
            init_logging();
            Impl::disconnect(provider_config)
        }

        fn get_connection_status(
            provider_config: Self::ProviderConfig,
        ) -> Result<crate::model::connection::ConnectionStatus, VectorError> {
            init_logging();
            Impl::get_connection_status(provider_config)
        }

        fn test_connection(
            provider_config: Self::ProviderConfig,
            endpoint: String,
            credentials: Option<crate::model::connection::Credentials>,
            timeout_ms: Option<u32>,
            options: Option<crate::model::types::Metadata>,
        ) -> Result<bool, VectorError> {
            init_logging();
            Impl::test_connection(provider_config, endpoint, credentials, timeout_ms, options)
        }
    }

    impl<Impl: ExtendedVectorProvider> CollectionProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        fn upsert_collection(
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
        }

        fn list_collections(
            provider_config: Self::ProviderConfig,
        ) -> Result<Vec<String>, VectorError> {
            init_logging();
            Impl::list_collections(provider_config)
        }

        fn get_collection(
            provider_config: Self::ProviderConfig,
            name: String,
        ) -> Result<crate::model::collections::CollectionInfo, VectorError> {
            init_logging();
            Impl::get_collection(provider_config, name)
        }

        fn update_collection(
            provider_config: Self::ProviderConfig,
            name: String,
            description: Option<String>,
            metadata: Option<crate::model::types::Metadata>,
        ) -> Result<crate::model::collections::CollectionInfo, VectorError> {
            init_logging();
            Impl::update_collection(provider_config, name, description, metadata)
        }

        fn delete_collection(
            provider_config: Self::ProviderConfig,
            name: String,
        ) -> Result<(), VectorError> {
            init_logging();
            Impl::delete_collection(provider_config, name)
        }

        fn collection_exists(
            provider_config: Self::ProviderConfig,
            name: String,
        ) -> Result<bool, VectorError> {
            init_logging();
            Impl::collection_exists(provider_config, name)
        }
    }

    impl<Impl: ExtendedVectorProvider> VectorsProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        fn upsert_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            vectors: Vec<crate::model::types::VectorRecord>,
            namespace: Option<String>,
        ) -> Result<crate::model::vectors::BatchResult, VectorError> {
            init_logging();
            Impl::upsert_vectors(provider_config, collection, vectors, namespace)
        }

        fn upsert_vector(
            provider_config: Self::ProviderConfig,
            collection: String,
            id: crate::model::types::Id,
            vector: crate::model::types::VectorData,
            metadata: Option<crate::model::types::Metadata>,
            namespace: Option<String>,
        ) -> Result<(), VectorError> {
            init_logging();
            Impl::upsert_vector(provider_config, collection, id, vector, metadata, namespace)
        }

        fn get_vectors(
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
        }

        fn get_vector(
            provider_config: Self::ProviderConfig,
            collection: String,
            id: crate::model::types::Id,
            namespace: Option<String>,
        ) -> Result<Option<crate::model::types::VectorRecord>, VectorError> {
            init_logging();
            Impl::get_vector(provider_config, collection, id, namespace)
        }

        fn update_vector(
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
        }

        fn delete_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            ids: Vec<crate::model::types::Id>,
            namespace: Option<String>,
        ) -> Result<u32, VectorError> {
            init_logging();
            Impl::delete_vectors(provider_config, collection, ids, namespace)
        }

        fn delete_by_filter(
            provider_config: Self::ProviderConfig,
            collection: String,
            filter: crate::model::types::FilterExpression,
            namespace: Option<String>,
        ) -> Result<u32, VectorError> {
            init_logging();
            Impl::delete_by_filter(provider_config, collection, filter, namespace)
        }

        fn delete_namespace(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: String,
        ) -> Result<u32, VectorError> {
            init_logging();
            <Impl as VectorsProvider>::delete_namespace(provider_config, collection, namespace)
        }

        fn list_vectors(
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
        }

        fn count_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            filter: Option<crate::model::types::FilterExpression>,
            namespace: Option<String>,
        ) -> Result<u64, VectorError> {
            init_logging();
            Impl::count_vectors(provider_config, collection, filter, namespace)
        }
    }

    impl<Impl: ExtendedVectorProvider> SearchProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        fn search_vectors(
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
        }

        fn find_similar(
            provider_config: Self::ProviderConfig,
            collection: String,
            vector: crate::model::types::VectorData,
            limit: u32,
            namespace: Option<String>,
        ) -> Result<Vec<crate::model::types::SearchResult>, VectorError> {
            init_logging();
            Impl::find_similar(provider_config, collection, vector, limit, namespace)
        }

        fn batch_search(
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
        }
    }

    impl<Impl: ExtendedVectorProvider> SearchExtendedProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        fn recommend_vectors(
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
        }

        fn discover_vectors(
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
        }

        fn search_groups(
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
        }

        fn search_range(
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
        }

        fn search_text(
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
        }
    }

    impl<Impl: ExtendedVectorProvider> AnalyticsProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        fn get_collection_stats(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: Option<String>,
        ) -> Result<crate::model::analytics::CollectionStats, VectorError> {
            init_logging();
            Impl::get_collection_stats(provider_config, collection, namespace)
        }

        fn get_field_stats(
            provider_config: Self::ProviderConfig,
            collection: String,
            field: String,
            namespace: Option<String>,
        ) -> Result<crate::model::analytics::FieldStats, VectorError> {
            init_logging();
            Impl::get_field_stats(provider_config, collection, field, namespace)
        }

        fn get_field_distribution(
            provider_config: Self::ProviderConfig,
            collection: String,
            field: String,
            limit: Option<u32>,
            namespace: Option<String>,
        ) -> Result<Vec<(crate::model::types::MetadataValue, u64)>, VectorError> {
            init_logging();
            Impl::get_field_distribution(provider_config, collection, field, limit, namespace)
        }
    }

    impl<Impl: ExtendedVectorProvider> NamespacesProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        fn upsert_namespace(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: String,
            metadata: Option<crate::model::types::Metadata>,
        ) -> Result<crate::model::namespaces::NamespaceInfo, VectorError> {
            init_logging();
            Impl::upsert_namespace(provider_config, collection, namespace, metadata)
        }

        fn list_namespaces(
            provider_config: Self::ProviderConfig,
            collection: String,
        ) -> Result<Vec<crate::model::namespaces::NamespaceInfo>, VectorError> {
            init_logging();
            Impl::list_namespaces(provider_config, collection)
        }

        fn get_namespace(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: String,
        ) -> Result<crate::model::namespaces::NamespaceInfo, VectorError> {
            init_logging();
            Impl::get_namespace(provider_config, collection, namespace)
        }

        fn delete_namespace(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: String,
        ) -> Result<(), VectorError> {
            init_logging();
            <Impl as NamespacesProvider>::delete_namespace(provider_config, collection, namespace)
        }

        fn namespace_exists(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: String,
        ) -> Result<bool, VectorError> {
            init_logging();
            Impl::namespace_exists(provider_config, collection, namespace)
        }
    }
}

#[cfg(feature = "golem")]
mod durable_impl {
    use super::*;
    use crate::init_logging;
    use golem_rust::bindings::golem::durability::durability::WrappedFunctionType;
    use golem_rust::durability::Durability;
    use golem_rust::{with_persistence_level, FromValueAndType, IntoValue, PersistenceLevel};

    #[derive(Debug, Clone, FromValueAndType, IntoValue)]
    pub(super) struct Unit;

    impl<Impl: ExtendedVectorProvider> ConnectionProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        fn connect(
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
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::connect_internal(
                        provider_config,
                        &endpoint,
                        &credentials,
                        &timeout_ms,
                        &options,
                    )
                });
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

        fn disconnect(provider_config: Self::ProviderConfig) -> Result<(), VectorError> {
            init_logging();
            let durability = Durability::<Unit, VectorError>::new(
                "golem_ai_vector",
                "disconnect",
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::disconnect(provider_config)
                });
                durability.persist(Unit, result.map(|_| Unit))?;
                Ok(())
            } else {
                durability.replay::<Unit, VectorError>()?;
                Ok(())
            }
        }

        fn get_connection_status(
            provider_config: Self::ProviderConfig,
        ) -> Result<crate::model::connection::ConnectionStatus, VectorError> {
            init_logging();
            let durability: Durability<crate::model::connection::ConnectionStatus, VectorError> =
                Durability::new(
                    "golem_ai_vector",
                    "get_connection_status",
                    WrappedFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::get_connection_status(provider_config)
                });
                durability.persist(Unit, result)
            } else {
                durability.replay()
            }
        }

        fn test_connection(
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
                WrappedFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::test_connection(
                        provider_config,
                        endpoint.clone(),
                        credentials.clone(),
                        timeout_ms,
                        options.clone(),
                    )
                });
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

        fn upsert_collection(
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
                    WrappedFunctionType::WriteRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::upsert_collection(
                        provider_config,
                        name.clone(),
                        description.clone(),
                        dimension,
                        metric,
                        index_config.clone(),
                        metadata.clone(),
                    )
                });
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

        fn list_collections(
            provider_config: Self::ProviderConfig,
        ) -> Result<Vec<String>, VectorError> {
            init_logging();
            let durability: Durability<Vec<String>, VectorError> = Durability::new(
                "golem_vector_collections",
                "list_collections",
                WrappedFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::list_collections(provider_config)
                });
                durability.persist(Unit, result)
            } else {
                durability.replay()
            }
        }

        fn get_collection(
            provider_config: Self::ProviderConfig,
            name: String,
        ) -> Result<crate::model::collections::CollectionInfo, VectorError> {
            init_logging();
            let durability: Durability<crate::model::collections::CollectionInfo, VectorError> =
                Durability::new(
                    "golem_vector_collections",
                    "get_collection",
                    WrappedFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::get_collection(provider_config, name.clone())
                });
                durability.persist(name, result)
            } else {
                durability.replay()
            }
        }

        fn update_collection(
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
                    WrappedFunctionType::WriteRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::update_collection(
                        provider_config,
                        name.clone(),
                        description.clone(),
                        metadata.clone(),
                    )
                });
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

        fn delete_collection(
            provider_config: Self::ProviderConfig,
            name: String,
        ) -> Result<(), VectorError> {
            init_logging();
            let durability: Durability<Unit, VectorError> = Durability::new(
                "golem_vector_collections",
                "delete_collection",
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::delete_collection(provider_config, name.clone())
                });
                durability.persist(name, result.map(|_| Unit))?;
                Ok(())
            } else {
                durability.replay::<Unit, VectorError>()?;
                Ok(())
            }
        }

        fn collection_exists(
            provider_config: Self::ProviderConfig,
            name: String,
        ) -> Result<bool, VectorError> {
            init_logging();
            let durability: Durability<bool, VectorError> = Durability::new(
                "golem_vector_collections",
                "collection_exists",
                WrappedFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::collection_exists(provider_config, name.clone())
                });
                durability.persist(name, result)
            } else {
                durability.replay()
            }
        }
    }

    impl<Impl: ExtendedVectorProvider> VectorsProvider for DurableVector<Impl> {
        type ProviderConfig = <Impl as ConnectionProvider>::ProviderConfig;

        fn upsert_vectors(
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
                    WrappedFunctionType::WriteRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::upsert_vectors(
                        provider_config,
                        collection.clone(),
                        vectors.clone(),
                        namespace.clone(),
                    )
                });
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

        fn upsert_vector(
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
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::upsert_vector(
                        provider_config,
                        collection.clone(),
                        id.clone(),
                        vector.clone(),
                        metadata.clone(),
                        namespace.clone(),
                    )
                });
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

        fn get_vectors(
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
                    WrappedFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::get_vectors(
                        provider_config,
                        collection.clone(),
                        ids.clone(),
                        namespace.clone(),
                        include_vectors,
                        include_metadata,
                    )
                });
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

        fn get_vector(
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
                    WrappedFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::get_vector(
                        provider_config,
                        collection.clone(),
                        id.clone(),
                        namespace.clone(),
                    )
                });
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

        fn update_vector(
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
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::update_vector(
                        provider_config,
                        collection.clone(),
                        id.clone(),
                        vector.clone(),
                        metadata.clone(),
                        namespace.clone(),
                        merge_metadata,
                    )
                });
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

        fn delete_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            ids: Vec<crate::model::types::Id>,
            namespace: Option<String>,
        ) -> Result<u32, VectorError> {
            init_logging();
            let durability: Durability<u32, VectorError> = Durability::new(
                "golem_vector_vectors",
                "delete_vectors",
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::delete_vectors(
                        provider_config,
                        collection.clone(),
                        ids.clone(),
                        namespace.clone(),
                    )
                });
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

        fn delete_by_filter(
            provider_config: Self::ProviderConfig,
            collection: String,
            filter: crate::model::types::FilterExpression,
            namespace: Option<String>,
        ) -> Result<u32, VectorError> {
            init_logging();
            let durability: Durability<u32, VectorError> = Durability::new(
                "golem_vector_vectors",
                "delete_by_filter",
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::delete_by_filter(
                        provider_config,
                        collection.clone(),
                        filter.clone(),
                        namespace.clone(),
                    )
                });
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

        fn delete_namespace(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: String,
        ) -> Result<u32, VectorError> {
            init_logging();
            let durability: Durability<u32, VectorError> = Durability::new(
                "golem_vector_vectors",
                "delete_namespace",
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    <Impl as VectorsProvider>::delete_namespace(
                        provider_config,
                        collection.clone(),
                        namespace.clone(),
                    )
                });
                durability.persist((collection, namespace), result)
            } else {
                durability.replay()
            }
        }

        fn list_vectors(
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
                    WrappedFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
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
                });
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

        fn count_vectors(
            provider_config: Self::ProviderConfig,
            collection: String,
            filter: Option<crate::model::types::FilterExpression>,
            namespace: Option<String>,
        ) -> Result<u64, VectorError> {
            init_logging();
            let durability: Durability<u64, VectorError> = Durability::new(
                "golem_vector_vectors",
                "count_vectors",
                WrappedFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::count_vectors(
                        provider_config,
                        collection.clone(),
                        filter.clone(),
                        namespace.clone(),
                    )
                });
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

        fn search_vectors(
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
                    WrappedFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
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
                });
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

        fn find_similar(
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
                    WrappedFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::find_similar(
                        provider_config,
                        collection.clone(),
                        vector.clone(),
                        limit,
                        namespace.clone(),
                    )
                });
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

        fn batch_search(
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
                    WrappedFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
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
                });
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

        fn recommend_vectors(
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
                    WrappedFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
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
                });
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

        fn discover_vectors(
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
                    WrappedFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
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
                });
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

        fn search_groups(
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
                WrappedFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
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
                });
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

        fn search_range(
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
                    WrappedFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
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
                });
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

        fn search_text(
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
                    WrappedFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::search_text(
                        provider_config,
                        collection.clone(),
                        query_text.clone(),
                        limit,
                        filter.clone(),
                        namespace.clone(),
                    )
                });
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

        fn get_collection_stats(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: Option<String>,
        ) -> Result<crate::model::analytics::CollectionStats, VectorError> {
            init_logging();
            let durability: Durability<crate::model::analytics::CollectionStats, VectorError> =
                Durability::new(
                    "golem_vector_analytics",
                    "get_collection_stats",
                    WrappedFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::get_collection_stats(
                        provider_config,
                        collection.clone(),
                        namespace.clone(),
                    )
                });
                durability.persist((collection, namespace), result)
            } else {
                durability.replay()
            }
        }

        fn get_field_stats(
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
                    WrappedFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::get_field_stats(
                        provider_config,
                        collection.clone(),
                        field.clone(),
                        namespace.clone(),
                    )
                });
                durability.persist((collection, field, namespace), result)
            } else {
                durability.replay()
            }
        }

        fn get_field_distribution(
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
                WrappedFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::get_field_distribution(
                        provider_config,
                        collection.clone(),
                        field.clone(),
                        limit,
                        namespace.clone(),
                    )
                });
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

        fn upsert_namespace(
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
                    WrappedFunctionType::WriteRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::upsert_namespace(
                        provider_config,
                        collection.clone(),
                        namespace.clone(),
                        metadata.clone(),
                    )
                });
                durability.persist((collection, namespace, metadata), result)
            } else {
                durability.replay()
            }
        }

        fn list_namespaces(
            provider_config: Self::ProviderConfig,
            collection: String,
        ) -> Result<Vec<crate::model::namespaces::NamespaceInfo>, VectorError> {
            init_logging();
            let durability: Durability<Vec<crate::model::namespaces::NamespaceInfo>, VectorError> =
                Durability::new(
                    "golem_vector_namespaces",
                    "list_namespaces",
                    WrappedFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::list_namespaces(provider_config, collection.clone())
                });
                durability.persist(collection, result)
            } else {
                durability.replay()
            }
        }

        fn get_namespace(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: String,
        ) -> Result<crate::model::namespaces::NamespaceInfo, VectorError> {
            init_logging();
            let durability: Durability<crate::model::namespaces::NamespaceInfo, VectorError> =
                Durability::new(
                    "golem_vector_namespaces",
                    "get_namespace",
                    WrappedFunctionType::ReadRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::get_namespace(provider_config, collection.clone(), namespace.clone())
                });
                durability.persist((collection, namespace), result)
            } else {
                durability.replay()
            }
        }

        fn delete_namespace(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: String,
        ) -> Result<(), VectorError> {
            init_logging();
            let durability: Durability<Unit, VectorError> = Durability::new(
                "golem_vector_namespaces",
                "delete_namespace",
                WrappedFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    <Impl as NamespacesProvider>::delete_namespace(
                        provider_config,
                        collection.clone(),
                        namespace.clone(),
                    )
                });
                durability.persist((collection, namespace), result.map(|_| Unit))?;
                Ok(())
            } else {
                durability.replay::<Unit, VectorError>()?;
                Ok(())
            }
        }

        fn namespace_exists(
            provider_config: Self::ProviderConfig,
            collection: String,
            namespace: String,
        ) -> Result<bool, VectorError> {
            init_logging();
            let durability: Durability<bool, VectorError> = Durability::new(
                "golem_vector_namespaces",
                "namespace_exists",
                WrappedFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::namespace_exists(provider_config, collection.clone(), namespace.clone())
                });
                durability.persist((collection, namespace), result)
            } else {
                durability.replay()
            }
        }
    }

    // Parameter structures for durability
    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
    struct ConnectParams {
        endpoint: String,
        credentials: Option<crate::model::connection::Credentials>,
        timeout_ms: Option<u32>,
        options: Option<crate::model::types::Metadata>,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
    struct UpsertCollectionParams {
        name: String,
        description: Option<String>,
        dimension: u32,
        metric: crate::model::types::DistanceMetric,
        index_config: Option<crate::model::collections::IndexConfig>,
        metadata: Option<crate::model::types::Metadata>,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
    struct UpdateCollectionParams {
        name: String,
        description: Option<String>,
        metadata: Option<crate::model::types::Metadata>,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
    struct UpsertVectorsParams {
        collection: String,
        vectors: Vec<crate::model::types::VectorRecord>,
        namespace: Option<String>,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
    struct UpsertVectorParams {
        collection: String,
        id: crate::model::types::Id,
        vector: crate::model::types::VectorData,
        metadata: Option<crate::model::types::Metadata>,
        namespace: Option<String>,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
    struct UpdateVectorParams {
        collection: String,
        id: crate::model::types::Id,
        vector: Option<crate::model::types::VectorData>,
        metadata: Option<crate::model::types::Metadata>,
        namespace: Option<String>,
        merge_metadata: Option<bool>,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
    struct DeleteVectorsParams {
        collection: String,
        ids: Vec<crate::model::types::Id>,
        namespace: Option<String>,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
    struct DeleteByFilterParams {
        collection: String,
        filter: crate::model::types::FilterExpression,
        namespace: Option<String>,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
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

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
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

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
    struct GetVectorsParams {
        collection: String,
        ids: Vec<crate::model::types::Id>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
    struct GetVectorParams {
        collection: String,
        id: crate::model::types::Id,
        namespace: Option<String>,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
    struct ListVectorsParams {
        collection: String,
        namespace: Option<String>,
        filter: Option<crate::model::types::FilterExpression>,
        limit: Option<u32>,
        cursor: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
    struct CountVectorsParams {
        collection: String,
        filter: Option<crate::model::types::FilterExpression>,
        namespace: Option<String>,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
    struct FindSimilarParams {
        collection: String,
        vector: crate::model::types::VectorData,
        limit: u32,
        namespace: Option<String>,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
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

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
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

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
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

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
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

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
    struct SearchTextParams {
        collection: String,
        query_text: String,
        limit: u32,
        filter: Option<crate::model::types::FilterExpression>,
        namespace: Option<String>,
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue, PartialEq)]
    struct GetFieldDistributionParams {
        collection: String,
        field: String,
        limit: Option<u32>,
        namespace: Option<String>,
    }
}

#[cfg(test)]
mod tests {
    use crate::model::types::{
        DenseVector, DistanceMetric, FilterCondition, FilterOperator, Id, MetadataValue,
        SearchResult, VectorData, VectorError, VectorRecord,
    };
    use golem_rust::value_and_type::{FromValueAndType, IntoValueAndType};
    use std::fmt::Debug;

    fn roundtrip_test<T: Debug + Clone + PartialEq + IntoValueAndType + FromValueAndType>(
        value: T,
    ) {
        let vnt = value.clone().into_value_and_type();
        let extracted = T::from_value_and_type(vnt).unwrap();
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

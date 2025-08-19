//! Durability wrapper for vector providers.
//!
<<<<<<< HEAD
//! This follows the durability design used by other components:
//! * Today, `DurableVector` is a thin passthrough that calls the underlying provider
//!   implementation after initializing logging. This behavior is the same whether or not
//!   the `durability` feature is enabled (i.e., durability is currently a no-op).
//! * In the future, the `durability` feature may enable true op-log persistence & replay
//!   without breaking production code paths.
//!
//! Keeping the passthrough implementation complete ensures the shared `vector` crate
//! compiles cleanly and is production-safe while allowing a future drop-in durability
//! implementation without API changes.
=======
//! This follows the durability design used by the `search` component:
//! * When the `durability` feature flag is **off** (default), `DurableVector` is a thin
//!   passthrough that merely calls the underlying provider implementation after
//!   initializing logging.
//! * When the `durability` feature flag is **on**, compilation fails until the full
//!   durability logic (op-log persistence / replay) is implemented.
//!
//! Keeping the passthrough implementation complete ensures the shared `vector` crate
//! compiles cleanly today while still allowing provider crates to opt-into durability
//! later by enabling the feature.
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49

use crate::exports::golem::vector::collections::Guest as CollectionsGuest;
use crate::exports::golem::vector::connection::Guest as ConnectionGuest;
use crate::exports::golem::vector::namespaces::Guest as NamespacesGuest;
<<<<<<< HEAD
use crate::exports::golem::vector::analytics::Guest as AnalyticsGuest;
use crate::exports::golem::vector::search::Guest as SearchGuest;
use crate::exports::golem::vector::search_extended::Guest as SearchExtendedGuest;
use crate::exports::golem::vector::vectors::Guest as VectorsGuest;
use crate::exports::golem::vector::types::{FilterExpression, Metadata, VectorError, VectorRecord};
use crate::exports::golem::vector::vectors::BatchResult;
=======
use crate::exports::golem::vector::search::Guest as SearchGuest;
use crate::exports::golem::vector::search_extended::Guest as SearchExtendedGuest;
use crate::exports::golem::vector::types::{FilterExpression, Metadata, VectorError, VectorRecord};
use crate::exports::golem::vector::vectors::{BatchResult, ListResponse};
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
use crate::init_logging;
use std::marker::PhantomData;

/// Wraps a provider implementation with (future) durability support.
pub struct DurableVector<Impl> {
    _phantom: PhantomData<Impl>,
}

<<<<<<< HEAD
/// When the durability feature flag is off, wrapping with `DurableVector` is just a passthrough
#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use crate::durability::{DurableVector, ExtendedGuest};
    use crate::exports::golem::vector::analytics::Guest as AnalyticsGuest;
    use crate::exports::golem::vector::collections::Guest as CollectionsGuest;
    use crate::exports::golem::vector::connection::Guest as ConnectionGuest;
    use crate::exports::golem::vector::namespaces::Guest as NamespacesGuest;
    use crate::exports::golem::vector::search::Guest as SearchGuest;
    use crate::exports::golem::vector::types::{
        CollectionInfo, FilterExpression, SearchResult, VectorError, VectorIndex, VectorQuery,
        VectorRecord,
    };
    use crate::exports::golem::vector::vectors::Guest as VectorsGuest;

    impl<Impl: ExtendedGuest> VectorsGuest for DurableVector<Impl> {
        fn upsert_vectors(
            collection: String,
            vectors: Vec<VectorRecord>,
            namespace: Option<String>,
        ) -> Result<Vec<String>, VectorError> {
            crate::init_logging();
            Impl::upsert_vectors(collection, vectors, namespace)
        }
    }
}

=======
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
/// Providers must implement _all_ individual `Guest` traits plus `'static` to be wrapped.
pub trait ExtendedGuest:
    CollectionsGuest
    + VectorsGuest
    + SearchGuest
    + SearchExtendedGuest
    + NamespacesGuest
    + ConnectionGuest
<<<<<<< HEAD
    + AnalyticsGuest
=======
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
    + 'static
{
}

// --- Passthrough implementation ---------------------------------------------------------------
<<<<<<< HEAD
=======
#[cfg(not(feature = "durability"))]
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
mod passthrough_impl {
    use super::*;
    use crate::exports::golem::vector::collections::{CollectionInfo, IndexConfig};
    use crate::exports::golem::vector::connection::{ConnectionStatus, Credentials};
    use crate::exports::golem::vector::namespaces::NamespaceInfo;
<<<<<<< HEAD
    use crate::exports::golem::vector::types::SearchQuery as SearchQueryEnum;
=======
    use crate::exports::golem::vector::search::SearchQuery as SearchQueryEnum;
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
    use crate::exports::golem::vector::search_extended::{
        GroupedSearchResult, RecommendationExample, RecommendationStrategy,
    };
    use crate::exports::golem::vector::types::{DistanceMetric, SearchResult};
<<<<<<< HEAD
    use crate::exports::golem::vector::analytics::{CollectionStats, FieldStats};
    use crate::exports::golem::vector::types::MetadataValue;
=======
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49

    // ----- collections ------------------------------------------------------------------------
    impl<T: ExtendedGuest> CollectionsGuest for DurableVector<T> {
        fn upsert_collection(
            name: String,
            description: Option<String>,
            dimension: u32,
            metric: DistanceMetric,
            index_config: Option<IndexConfig>,
            metadata: Option<Metadata>,
        ) -> Result<CollectionInfo, VectorError> {
            init_logging();
            T::upsert_collection(name, description, dimension, metric, index_config, metadata)
        }

        fn list_collections() -> Result<Vec<CollectionInfo>, VectorError> {
            init_logging();
            T::list_collections()
        }

        fn get_collection(name: String) -> Result<CollectionInfo, VectorError> {
            init_logging();
            T::get_collection(name)
        }

        fn update_collection(
            name: String,
            description: Option<String>,
            metadata: Option<Metadata>,
        ) -> Result<CollectionInfo, VectorError> {
            init_logging();
            T::update_collection(name, description, metadata)
        }

        fn delete_collection(name: String) -> Result<(), VectorError> {
            init_logging();
            T::delete_collection(name)
        }

        fn collection_exists(name: String) -> Result<bool, VectorError> {
            init_logging();
            T::collection_exists(name)
        }
    }

<<<<<<< HEAD
    // ----- analytics -------------------------------------------------------------------------
    impl<T: ExtendedGuest> AnalyticsGuest for DurableVector<T> {
        fn get_collection_stats(
            collection: String,
            namespace: Option<String>,
        ) -> Result<CollectionStats, VectorError> {
            init_logging();
            T::get_collection_stats(collection, namespace)
        }

        fn get_field_stats(
            collection: String,
            field: String,
            namespace: Option<String>,
        ) -> Result<FieldStats, VectorError> {
            init_logging();
            T::get_field_stats(collection, field, namespace)
        }

        fn get_field_distribution(
            collection: String,
            field: String,
            limit: Option<u32>,
            namespace: Option<String>,
        ) -> Result<Vec<(MetadataValue, u64)>, VectorError> {
            init_logging();
            T::get_field_distribution(collection, field, limit, namespace)
        }
    }

    // ----- vectors ---------------------------------------------------------------------------
    
=======
    // ----- vectors ---------------------------------------------------------------------------
    use crate::exports::golem::vector::types::Id;
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
    use crate::exports::golem::vector::vectors::ListResponse as VListResponse;
    impl<T: ExtendedGuest> crate::exports::golem::vector::vectors::Guest for DurableVector<T> {
        fn upsert_vectors(
            collection: String,
            vectors: Vec<VectorRecord>,
            namespace: Option<String>,
        ) -> Result<BatchResult, VectorError> {
            init_logging();
            T::upsert_vectors(collection, vectors, namespace)
        }

        fn upsert_vector(
            collection: String,
            id: String,
            vector: crate::exports::golem::vector::types::VectorData,
            metadata: Option<Metadata>,
            namespace: Option<String>,
        ) -> Result<(), VectorError> {
            init_logging();
            T::upsert_vector(collection, id, vector, metadata, namespace)
        }

        fn get_vectors(
            collection: String,
            ids: Vec<String>,
            namespace: Option<String>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
        ) -> Result<Vec<VectorRecord>, VectorError> {
            init_logging();
            T::get_vectors(
                collection,
                ids,
                namespace,
                include_vectors,
                include_metadata,
            )
        }

        fn get_vector(
            collection: String,
            id: String,
            namespace: Option<String>,
        ) -> Result<Option<VectorRecord>, VectorError> {
            init_logging();
            T::get_vector(collection, id, namespace)
        }

        fn update_vector(
            collection: String,
            id: String,
            vector: Option<crate::exports::golem::vector::types::VectorData>,
            metadata: Option<Metadata>,
            namespace: Option<String>,
            merge_metadata: Option<bool>,
        ) -> Result<(), VectorError> {
            init_logging();
            T::update_vector(collection, id, vector, metadata, namespace, merge_metadata)
        }

        fn delete_vectors(
            collection: String,
            ids: Vec<String>,
            namespace: Option<String>,
        ) -> Result<u32, VectorError> {
            init_logging();
            T::delete_vectors(collection, ids, namespace)
        }

        fn delete_by_filter(
            collection: String,
            filter: FilterExpression,
            namespace: Option<String>,
        ) -> Result<u32, VectorError> {
            init_logging();
            T::delete_by_filter(collection, filter, namespace)
        }

<<<<<<< HEAD
=======
        fn delete_namespace(collection: String, namespace: String) -> Result<u32, VectorError> {
            init_logging();
            T::delete_namespace(collection, namespace)
        }

>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
        fn list_vectors(
            collection: String,
            namespace: Option<String>,
            filter: Option<FilterExpression>,
            limit: Option<u32>,
            cursor: Option<String>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
        ) -> Result<VListResponse, VectorError> {
            init_logging();
            T::list_vectors(
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
            collection: String,
            filter: Option<FilterExpression>,
            namespace: Option<String>,
        ) -> Result<u64, VectorError> {
            init_logging();
            T::count_vectors(collection, filter, namespace)
        }
    }

    // ----- search ----------------------------------------------------------------------------
    impl<T: ExtendedGuest> SearchGuest for DurableVector<T> {
        fn search_vectors(
            collection: String,
            query: SearchQueryEnum,
            limit: u32,
            filter: Option<FilterExpression>,
            namespace: Option<String>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
            min_score: Option<f32>,
            max_distance: Option<f32>,
            search_params: Option<Vec<(String, String)>>,
        ) -> Result<Vec<SearchResult>, VectorError> {
            init_logging();
            T::search_vectors(
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
            collection: String,
            vector: crate::exports::golem::vector::types::VectorData,
            limit: u32,
            namespace: Option<String>,
        ) -> Result<Vec<SearchResult>, VectorError> {
            init_logging();
            T::find_similar(collection, vector, limit, namespace)
        }

        fn batch_search(
            collection: String,
            queries: Vec<SearchQueryEnum>,
            limit: u32,
            filter: Option<FilterExpression>,
            namespace: Option<String>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
            search_params: Option<Vec<(String, String)>>,
        ) -> Result<Vec<Vec<SearchResult>>, VectorError> {
            init_logging();
            T::batch_search(
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

    // ----- search-extended -------------------------------------------------------------------
    impl<T: ExtendedGuest> SearchExtendedGuest for DurableVector<T> {
        fn recommend_vectors(
            collection: String,
            positive: Vec<RecommendationExample>,
            negative: Option<Vec<RecommendationExample>>,
            limit: u32,
            filter: Option<FilterExpression>,
            namespace: Option<String>,
            strategy: Option<RecommendationStrategy>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
        ) -> Result<Vec<SearchResult>, VectorError> {
            init_logging();
            T::recommend_vectors(
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
            collection: String,
            context_pairs: Vec<crate::exports::golem::vector::search_extended::ContextPair>,
            limit: u32,
            filter: Option<FilterExpression>,
            namespace: Option<String>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
        ) -> Result<Vec<SearchResult>, VectorError> {
            init_logging();
            T::discover_vectors(
                collection,
                context_pairs,
                limit,
                filter,
                namespace,
                include_vectors,
                include_metadata,
            )
        }

        fn search_groups(
            collection: String,
            query: SearchQueryEnum,
            group_by: String,
            group_size: u32,
            max_groups: u32,
            filter: Option<FilterExpression>,
            namespace: Option<String>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
        ) -> Result<Vec<GroupedSearchResult>, VectorError> {
            init_logging();
            T::search_groups(
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
            collection: String,
            vector: crate::exports::golem::vector::types::VectorData,
            min_distance: Option<f32>,
            max_distance: f32,
            filter: Option<FilterExpression>,
            namespace: Option<String>,
            limit: Option<u32>,
            include_vectors: Option<bool>,
            include_metadata: Option<bool>,
        ) -> Result<Vec<SearchResult>, VectorError> {
            init_logging();
            T::search_range(
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
            collection: String,
            query_text: String,
            limit: u32,
            filter: Option<FilterExpression>,
            namespace: Option<String>,
        ) -> Result<Vec<SearchResult>, VectorError> {
            init_logging();
            T::search_text(collection, query_text, limit, filter, namespace)
        }
    }

    // ----- namespaces ------------------------------------------------------------------------
    impl<T: ExtendedGuest> NamespacesGuest for DurableVector<T> {
        fn upsert_namespace(
            collection: String,
            namespace: String,
            metadata: Option<Metadata>,
        ) -> Result<NamespaceInfo, VectorError> {
            init_logging();
            T::upsert_namespace(collection, namespace, metadata)
        }

        fn list_namespaces(collection: String) -> Result<Vec<NamespaceInfo>, VectorError> {
            init_logging();
            T::list_namespaces(collection)
        }

        fn get_namespace(
            collection: String,
            namespace: String,
        ) -> Result<NamespaceInfo, VectorError> {
            init_logging();
            T::get_namespace(collection, namespace)
        }

        fn delete_namespace(collection: String, namespace: String) -> Result<(), VectorError> {
            init_logging();
            T::delete_namespace(collection, namespace)
        }

        fn namespace_exists(collection: String, namespace: String) -> Result<bool, VectorError> {
            init_logging();
            T::namespace_exists(collection, namespace)
        }
    }

    // ----- connection ------------------------------------------------------------------------
    impl<T: ExtendedGuest> ConnectionGuest for DurableVector<T> {
        fn connect(
            endpoint: String,
            credentials: Option<Credentials>,
            timeout_ms: Option<u32>,
            options: Option<Metadata>,
        ) -> Result<(), VectorError> {
            init_logging();
            T::connect(endpoint, credentials, timeout_ms, options)
        }

        fn disconnect() -> Result<(), VectorError> {
            init_logging();
            T::disconnect()
        }

        fn get_connection_status() -> Result<ConnectionStatus, VectorError> {
            init_logging();
            T::get_connection_status()
        }

        fn test_connection(
            endpoint: String,
            credentials: Option<Credentials>,
            timeout_ms: Option<u32>,
            options: Option<Metadata>,
        ) -> Result<bool, VectorError> {
            init_logging();
            T::test_connection(endpoint, credentials, timeout_ms, options)
        }
    }
}
<<<<<<< HEAD
=======

// --- Compile-time placeholder for future durability -------------------------------------------
#[cfg(feature = "durability")]
mod todo_impl {
    compile_error!("Full durability support for vector providers is not yet implemented – enable the feature once implemented.");
}
/*

            description: Option<String>,
            dimension: u32,
            metric: crate::exports::golem::vector::types::DistanceMetric,
            index_config: Option<crate::exports::golem::vector::collections::IndexConfig>,
            metadata: Option<crate::exports::golem::vector::types::Metadata>,
        ) -> Result<crate::exports::golem::vector::collections::CollectionInfo, VectorError> {
            T::upsert_collection(name, description, dimension, metric, index_config, metadata)
        }

        fn list_collections(
        ) -> Result<Vec<crate::exports::golem::vector::collections::CollectionInfo>, VectorError> {
            T::list_collections()
        }

        fn get_collection(
            name: String,
        ) -> Result<crate::exports::golem::vector::collections::CollectionInfo, VectorError> {
            T::get_collection(name)
        }

        fn update_collection(
            name: String,
            description: Option<String>,
            metadata: Option<crate::exports::golem::vector::types::Metadata>,
        ) -> Result<crate::exports::golem::vector::collections::CollectionInfo, VectorError> {
            T::update_collection(name, description, metadata)
        }

        fn delete_collection(name: String) -> Result<(), VectorError> {
            T::delete_collection(name)
        }

        fn collection_exists(name: String) -> Result<bool, VectorError> {
            T::collection_exists(name)
        }
    }
        fn upsert_vectors(
            collection: String,
            vectors: Vec<crate::exports::golem::vector::types::VectorRecord>,
            namespace: Option<String>,
        ) -> Result<crate::exports::golem::vector::vectors::BatchResult, VectorError> {
            T::upsert_vectors(collection, vectors, namespace)
        }

        fn get_vector(
            collection: String,
            id: String,
            namespace: Option<String>,
        ) -> Result<Option<crate::exports::golem::vector::types::VectorRecord>, VectorError> {
            T::get_vector(collection, id, namespace)
        }

        fn delete_vectors(
            collection: String,
            ids: Vec<String>,
            namespace: Option<String>,
        ) -> Result<u32, VectorError> {
            T::delete_vectors(collection, ids, namespace)
        }

        fn count_vectors(
            collection: String,
            filter: Option<crate::exports::golem::vector::types::FilterExpression>,
            namespace: Option<String>,
        ) -> Result<u64, VectorError> {
            T::count_vectors(collection, filter, namespace)
        }

    // Similar passthrough impls would follow for SearchGuest, SearchExtendedGuest, NamespacesGuest,
    // and ConnectionGuest, but are omitted here for brevity while durability is disabled.
}

// --- Compile-time placeholder when `durability` feature IS enabled -----------------------------
#[cfg(feature = "durability")]
mod todo_impl {
    // Intentionally fail compilation until full durability is implemented.
    compile_error!("Durability support for vector providers is not yet implemented");
}
*/
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49

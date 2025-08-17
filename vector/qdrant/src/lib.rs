//! Qdrant vector provider component (work-in-progress)
//!
//! This crate wires the synchronous [`QdrantApi`] REST client to the
//! Golem Vector WIT interfaces.  The full implementation will live
//! behind the `provider_impl` feature flag.  Until enabled, compilation
//! will fail with a clear message so that downstream workspaces are not
//! surprised by partial functionality.

mod bindings;
mod client;
mod conversion;

use crate::client::{QdrantApi, QdrantPoint};
use crate::conversion::*;
use golem_vector::durability::{DurableVector, ExtendedGuest};
use golem_vector::error::unsupported_feature;
use golem_vector::exports::golem::vector::collections::{CollectionInfo, IndexConfig};
use golem_vector::exports::golem::vector::connection::{Credentials, Guest as ConnectionGuest};
use golem_vector::exports::golem::vector::namespaces::{Guest as NamespacesGuest, NamespaceInfo};
use golem_vector::exports::golem::vector::search::{
    Guest as SearchGuest, SearchQuery as SearchQueryEnum,
};
use golem_vector::exports::golem::vector::search_extended::{
    GroupedSearchResult, Guest as SearchExtendedGuest, RecommendationExample,
    RecommendationStrategy,
};
use golem_vector::exports::golem::vector::types::*;
use golem_vector::exports::golem::vector::vectors::{
    BatchResult, Guest as VectorsGuest, ListResponse,
};
use golem_vector::init_logging;

struct QdrantComponent;

impl QdrantComponent {
    const ENDPOINT_ENV: &'static str = "QDRANT_ENDPOINT";
    const API_KEY_ENV: &'static str = "QDRANT_API_KEY";

    fn create_client() -> QdrantApi {
        // Endpoint defaults to localhost if env var not set
        let endpoint = std::env::var(Self::ENDPOINT_ENV)
            .unwrap_or_else(|_| "http://localhost:6333".to_string());
        let api_key = std::env::var(Self::API_KEY_ENV).ok();
        QdrantApi::new(endpoint, api_key)
    }
}

// -------------------- collections ---------------------------
impl golem_vector::exports::golem::vector::collections::Guest for QdrantComponent {
    fn upsert_collection(
        name: String,
        _description: Option<String>,
        dimension: u32,
        metric: DistanceMetric,
        _index_config: Option<IndexConfig>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        init_logging();
        let client = Self::create_client();
        let dist = metric_to_qdrant(metric);
        client
            .upsert_collection(&name, dimension, dist)
            .map(collection_desc_to_info)
    }

    fn list_collections() -> Result<Vec<CollectionInfo>, VectorError> {
        init_logging();
        let client = Self::create_client();
        client
            .list_collections()
            .map(|v| v.into_iter().map(collection_desc_to_info).collect())
    }

    fn get_collection(name: String) -> Result<CollectionInfo, VectorError> {
        init_logging();
        let client = Self::create_client();
        client
            .list_collections() // Qdrant lacks single collection endpoint V1
            .and_then(|list| {
                list.into_iter()
                    .find(|c| c.name == name)
                    .map(collection_desc_to_info)
                    .ok_or(VectorError::NotFound("Collection not found".into()))
            })
    }

    fn update_collection(
        _name: String,
        _description: Option<String>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        Err(unsupported_feature(
            "Update collection not supported by Qdrant",
        ))
    }

    fn delete_collection(name: String) -> Result<(), VectorError> {
        init_logging();
        let client = Self::create_client();
        client.delete_collection(&name)
    }

    fn collection_exists(name: String) -> Result<bool, VectorError> {
        Self::list_collections().map(|cols| cols.iter().any(|c| c.name == name))
    }
}

// -------------------- vectors -------------------------------
impl VectorsGuest for QdrantComponent {
    fn upsert_vectors(
        collection: String,
        vectors: Vec<VectorRecord>,
        namespace: Option<String>,
    ) -> Result<BatchResult, VectorError> {
        init_logging();
        let client = Self::create_client();
        let mut success = 0u32;
        let mut errors = Vec::new();
        let mut points: Vec<QdrantPoint> = Vec::with_capacity(vectors.len());
        for (idx, rec) in vectors.into_iter().enumerate() {
            match record_to_qdrant_point(rec) {
                Ok(p) => {
                    points.push(p);
                    success += 1;
                }
                Err(e) => errors.push((idx as u32, e)),
            }
        }
        if !points.is_empty() {
            client.upsert_points(&collection, points, namespace)?;
        }
        Ok(BatchResult {
            success_count: success,
            failure_count: errors.len() as u32,
            errors,
        })
    }

    fn upsert_vector(
        collection: String,
        id: String,
        vector: VectorData,
        metadata: Option<Metadata>,
        namespace: Option<String>,
    ) -> Result<(), VectorError> {
        let rec = VectorRecord {
            id,
            vector,
            metadata,
        };
        Self::upsert_vectors(collection, vec![rec], namespace).map(|_| ())
    }

    fn get_vectors(
        _collection: String,
        _ids: Vec<String>,
        _namespace: Option<String>,
        _include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
    ) -> Result<Vec<VectorRecord>, VectorError> {
        Err(unsupported_feature("Get vectors not implemented"))
    }

    fn get_vector(
        _collection: String,
        _id: String,
        _namespace: Option<String>,
    ) -> Result<Option<VectorRecord>, VectorError> {
        Err(unsupported_feature("Get vector not implemented"))
    }

    fn update_vector(
        _collection: String,
        _id: String,
        _vector: Option<VectorData>,
        _metadata: Option<Metadata>,
        _namespace: Option<String>,
        _merge_metadata: Option<bool>,
    ) -> Result<(), VectorError> {
        Err(unsupported_feature("Update vector not implemented"))
    }

    fn delete_vectors(
        _collection: String,
        _ids: Vec<String>,
        _namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        Err(unsupported_feature("Delete vectors not implemented"))
    }

    fn delete_by_filter(
        _collection: String,
        _filter: FilterExpression,
        _namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        Err(unsupported_feature("Delete by filter not implemented"))
    }

    fn list_vectors(
        _collection: String,
        _namespace: Option<String>,
        _filter: Option<FilterExpression>,
        _limit: Option<u32>,
        _cursor: Option<String>,
        _include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
    ) -> Result<ListResponse, VectorError> {
        Err(unsupported_feature("List vectors not implemented"))
    }

    fn count_vectors(
        _collection: String,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
    ) -> Result<u64, VectorError> {
        Err(unsupported_feature("Count vectors not implemented"))
    }
}

// -------------------- search --------------------------------
impl SearchGuest for QdrantComponent {
    fn search_vectors(
        collection: String,
        query: SearchQueryEnum,
        limit: u32,
        filter: Option<FilterExpression>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
        _min_score: Option<f32>,
        _max_distance: Option<f32>,
        _search_params: Option<Vec<(String, String)>>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        init_logging();
        let client = Self::create_client();
        let vector = match query {
            SearchQueryEnum::Vector(v) => vector_data_to_dense(v)?,
            _ => return Err(unsupported_feature("Only vector queries supported")),
        };
        let q_filter = filter_expression_to_qdrant(filter.clone());
        let results = client.search(
            &collection,
            vector,
            limit,
            namespace,
            q_filter,
            include_vectors.unwrap_or(false),
            include_metadata.unwrap_or(false),
        )?;
        Ok(results
            .into_iter()
            .map(|r| SearchResult {
                id: r.id,
                score: r.score,
                distance: 0.0,
                vector: r.vector.map(VectorData::Dense),
                metadata: r.payload.map(|p| {
                    p.into_iter()
                        .map(|(k, v)| (k, MetadataValue::StringVal(v.to_string())))
                        .collect()
                }),
            })
            .collect())
    }

    fn find_similar(
        collection: String,
        vector: VectorData,
        limit: u32,
        namespace: Option<String>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        Self::search_vectors(
            collection,
            SearchQueryEnum::Vector(vector),
            limit,
            None,
            namespace,
            Some(true),
            Some(true),
            None,
            None,
            None,
        )
    }

    fn batch_search(
        _collection: String,
        _queries: Vec<SearchQueryEnum>,
        _limit: u32,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
        _include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
        _search_params: Option<Vec<(String, String)>>,
    ) -> Result<Vec<Vec<SearchResult>>, VectorError> {
        Err(unsupported_feature("Batch search not implemented"))
    }
}

// -------------------- search extended ----------------------
impl SearchExtendedGuest for QdrantComponent {
    fn recommend_vectors(
        _collection: String,
        _positive: Vec<RecommendationExample>,
        _negative: Option<Vec<RecommendationExample>>,
        _limit: u32,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
        _strategy: Option<RecommendationStrategy>,
        _include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        Err(unsupported_feature("Recommend not implemented"))
    }

    fn discover_vectors(
        _collection: String,
        _context_pairs: Vec<golem_vector::exports::golem::vector::search_extended::ContextPair>,
        _limit: u32,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
        _include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        Err(unsupported_feature("Discover not implemented"))
    }

    fn search_groups(
        _collection: String,
        _query: SearchQueryEnum,
        _group_by: String,
        _group_size: u32,
        _max_groups: u32,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
        _include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
    ) -> Result<Vec<GroupedSearchResult>, VectorError> {
        Err(unsupported_feature("Group search not implemented"))
    }

    fn search_range(
        _collection: String,
        _vector: VectorData,
        _min_distance: Option<f32>,
        _max_distance: f32,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
        _limit: Option<u32>,
        _include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        Err(unsupported_feature("Range search not implemented"))
    }

    fn search_text(
        _collection: String,
        _query_text: String,
        _limit: u32,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        Err(unsupported_feature("Text search not supported"))
    }
}

// -------------------- namespaces ---------------------------
impl NamespacesGuest for QdrantComponent {
    fn upsert_namespace(
        _collection: String,
        _namespace: String,
        _metadata: Option<Metadata>,
    ) -> Result<NamespaceInfo, VectorError> {
        Err(unsupported_feature("Namespaces not supported by Qdrant"))
    }

    fn list_namespaces(_collection: String) -> Result<Vec<NamespaceInfo>, VectorError> {
        Err(unsupported_feature("Namespaces not supported by Qdrant"))
    }

    fn get_namespace(
        _collection: String,
        _namespace: String,
    ) -> Result<NamespaceInfo, VectorError> {
        Err(unsupported_feature("Namespaces not supported by Qdrant"))
    }

    fn delete_namespace(_collection: String, _namespace: String) -> Result<(), VectorError> {
        Err(unsupported_feature("Namespaces not supported by Qdrant"))
    }

    fn namespace_exists(_collection: String, _namespace: String) -> Result<bool, VectorError> {
        Err(unsupported_feature("Namespaces not supported by Qdrant"))
    }
}

// -------------------- connection ---------------------------
use golem_vector::exports::golem::vector::connection::{
    ConnectionStatus, Guest as ConnectionGuestImpl,
};

impl ConnectionGuestImpl for QdrantComponent {
    fn connect(
        _endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        _options: Option<Metadata>,
    ) -> Result<(), VectorError> {
        // Client is created on-demand; nothing to do.
        Ok(())
    }

    fn disconnect() -> Result<(), VectorError> {
        Ok(())
    }

    fn get_connection_status() -> Result<ConnectionStatus, VectorError> {
        Ok(ConnectionStatus {
            connected: true,
            provider: Some("qdrant".into()),
            endpoint: std::env::var(Self::ENDPOINT_ENV).ok(),
            last_activity: None,
        })
    }

    fn test_connection(
        endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        _options: Option<Metadata>,
    ) -> Result<bool, VectorError> {
        let client = QdrantApi::new(endpoint, None);
        // Attempt simple ping via list collections
        match client.list_collections() {
            Ok(_) => Ok(true),
            Err(e) => Err(e),
        }
    }
}

impl ExtendedGuest for QdrantComponent {}

type DurableQdrantComponent = DurableVector<QdrantComponent>;

golem_vector::export_vector!(DurableQdrantComponent with_types_in golem_vector);

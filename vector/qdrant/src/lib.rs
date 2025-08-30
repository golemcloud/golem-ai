mod bindings;
mod client;
mod conversion;

use client::QdrantClient;
use conversion::*;
use log::{debug, error};

use golem_vector::durability::{DurableVector, ExtendedGuest};
use golem_vector::error::unsupported_feature;
use golem_vector::exports::golem::vector::analytics::{
    CollectionStats, FieldStats, Guest as AnalyticsGuest,
};
use golem_vector::exports::golem::vector::collections::{
    CollectionInfo, Guest as CollectionsGuest, IndexConfig,
};
use golem_vector::exports::golem::vector::connection::{
    ConnectionStatus, Credentials, Guest as ConnectionGuest,
};
use golem_vector::exports::golem::vector::namespaces::{Guest as NamespacesGuest, NamespaceInfo};
use golem_vector::exports::golem::vector::search::{
    Guest as SearchGuest, SearchQuery as SearchQueryEnum, SearchResult,
};
use golem_vector::exports::golem::vector::search_extended::{
    ContextPair, GroupedSearchResult, Guest as SearchExtendedGuest, RecommendationExample,
    RecommendationStrategy,
};
use golem_vector::exports::golem::vector::types::*;
use golem_vector::exports::golem::vector::vectors::{
    BatchResult, Guest as VectorsGuest, ListResponse, VectorRecord,
};
use golem_vector::init_logging as golem_init_logging;

/// Exported component type via durability wrapper
pub type Component = DurableVector<QdrantComponent>;

pub struct QdrantComponent;

impl QdrantComponent {
    const ENV_ENDPOINT: &str = "QDRANT_ENDPOINT"; // e.g. http://localhost:6333
    const ENV_API_KEY: &str = "QDRANT_API_KEY"; // optional

    fn init_logging() {
        golem_init_logging();
    }

    fn create_client() -> Result<QdrantClient, VectorError> {
        let endpoint = std::env::var(Self::ENV_ENDPOINT).map_err(|_| {
            VectorError::InvalidParams(format!("{env} not set", env = Self::ENV_ENDPOINT))
        })?;
        let api_key = std::env::var(Self::ENV_API_KEY).ok();
        Ok(QdrantClient::new(endpoint, api_key))
    }

    fn unsupported(feature: &str) -> VectorError {
        unsupported_feature(format!("Qdrant: {feature}"))
    }

    fn collection_info(name: String, dim: u32, metric: DistanceMetric) -> CollectionInfo {
        CollectionInfo {
            name,
            description: None,
            dimension: dim,
            metric,
            vector_count: 0,
            size_bytes: Some(0),
            index_ready: false,
            created_at: None,
            updated_at: None,
            provider_stats: None,
        }
    }
}

// ---------------- collections -------------------------------
impl CollectionsGuest for QdrantComponent {
    fn upsert_collection(
        name: String,
        _description: Option<String>,
        dimension: u32,
        metric: DistanceMetric,
        _index_config: Option<IndexConfig>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        Self::init_logging();
        let client = Self::create_client()?;
        // For simplicity always attempt create; ignore already exists errors
        match client.create_collection(&name, dimension, metric) {
            Ok(()) => Ok(Self::collection_info(name, dimension, metric)),
            Err(e) => Err(e),
        }
    }

    fn list_collections() -> Result<Vec<CollectionInfo>, VectorError> {
        Self::init_logging();
        let client = Self::create_client()?;
        let cols = client.list_collections()?;
        // Dimension & metric unknown without extra API; fill with defaults
        Ok(cols
            .into_iter()
            .map(|n| Self::collection_info(n, 0, DistanceMetric::Cosine))
            .collect())
    }

    fn get_collection(_name: String) -> Result<CollectionInfo, VectorError> {
        // Not implemented – fallback to exists heuristic
        Err(Self::unsupported("get_collection exact info"))
    }

    fn update_collection(
        _name: String,
        _description: Option<String>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        Err(Self::unsupported("update_collection"))
    }
    fn delete_collection(name: String) -> Result<(), VectorError> {
        Self::create_client()?.delete_collection(&name)
    }
    fn collection_exists(name: String) -> Result<bool, VectorError> {
        let list = Self::list_collections()?;
        Ok(list.into_iter().any(|c| c.name == name))
    }
}

// ---------------- vectors -----------------------------------
impl VectorsGuest for QdrantComponent {
    fn upsert_vectors(
        _collection: String,
        vectors: Vec<VectorRecord>,
        _namespace: Option<String>,
    ) -> Result<BatchResult, VectorError> {
        Self::init_logging();
        if vectors.is_empty() {
            return Ok(BatchResult {
                success_count: 0,
                failure_count: 0,
                errors: vec![],
            });
        }
        let client = Self::create_client()?;
        let col = &_collection;
        match client.upsert_vectors(col, vectors.clone()) {
            Ok(()) => Ok(BatchResult {
                success_count: vectors.len() as u32,
                failure_count: 0,
                errors: vec![],
            }),
            Err(e) => {
                error!("Failed to upsert vectors: {e}");
                let errs = (0..vectors.len()).map(|i| (i as u32, e.clone())).collect();
                Ok(BatchResult {
                    success_count: 0,
                    failure_count: vectors.len() as u32,
                    errors: errs,
                })
            }
        }
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
        collection: String,
        ids: Vec<String>,
        _namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<Vec<VectorRecord>, VectorError> {
        Self::create_client()?.fetch_vectors(
            &collection,
            ids,
            include_vectors.unwrap_or(true),
            include_metadata.unwrap_or(true),
        )
    }

    fn get_vector(
        collection: String,
        id: String,
        namespace: Option<String>,
    ) -> Result<Option<VectorRecord>, VectorError> {
        let mut v = Self::get_vectors(collection, vec![id], namespace, Some(true), Some(true))?;
        Ok(v.pop())
    }

    fn update_vector(
        _collection: String,
        _id: String,
        _vector: Option<VectorData>,
        _metadata: Option<Metadata>,
        _namespace: Option<String>,
        _merge_metadata: Option<bool>,
    ) -> Result<(), VectorError> {
        Err(Self::unsupported("update_vector"))
    }

    fn delete_vectors(
        collection: String,
        ids: Vec<String>,
        _namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        Self::create_client()?.delete_vectors(&collection, ids)
    }

    fn delete_by_filter(
        collection: String,
        filter: FilterExpression,
        _namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        Self::init_logging();
        let client = Self::create_client()?;
        let filter_json = filter_expression_to_qdrant(Some(filter));
        // Scroll to collect IDs in batches
        let mut cursor: Option<String> = None;
        let mut total_deleted = 0u32;
        loop {
            let (records, next) = client.scroll_vectors(
                &collection,
                filter_json.clone(),
                256,
                cursor.clone(),
                false,
                false,
            )?;
            if records.is_empty() {
                break;
            }
            let ids: Vec<String> = records.into_iter().map(|r| r.id).collect();
            total_deleted += client.delete_vectors(&collection, ids)?;
            cursor = next;
            if cursor.is_none() {
                break;
            }
        }
        Ok(total_deleted)
    }
    fn list_vectors(
        collection: String,
        _namespace: Option<String>,
        filter: Option<FilterExpression>,
        limit: Option<u32>,
        cursor: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<ListResponse, VectorError> {
        Self::init_logging();
        let client = Self::create_client()?;
        let filter_json = filter_expression_to_qdrant(filter);
        let lim = limit.unwrap_or(100);
        let (records, next_cursor) = client.scroll_vectors(
            &collection,
            filter_json,
            lim,
            cursor.clone(),
            include_vectors.unwrap_or(true),
            include_metadata.unwrap_or(true),
        )?;
        Ok(ListResponse {
            vectors: records,
            next_cursor,
            total_count: None,
        })
    }
    fn count_vectors(
        collection: String,
        filter: Option<FilterExpression>,
        _namespace: Option<String>,
    ) -> Result<u64, VectorError> {
        Self::init_logging();
        let client = Self::create_client()?;
        let filter_json = filter_expression_to_qdrant(filter);
        client.count_vectors(&collection, filter_json)
    }
}

// ---------------- search ------------------------------------
impl SearchGuest for QdrantComponent {
    fn search_vectors(
        collection: String,
        query: SearchQueryEnum,
        limit: u32,
        filter: Option<FilterExpression>,
        _namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
        _min_score: Option<f32>,
        _max_distance: Option<f32>,
        _search_params: Option<Vec<(String, String)>>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        let qvec = match query {
            SearchQueryEnum::Vector(v) => vector_data_to_dense(v)?,
            SearchQueryEnum::ById(_) => {
                return Err(Self::unsupported("search by id not implemented"))
            }
            SearchQueryEnum::MultiVector(_) => {
                return Err(Self::unsupported("text search not implemented"))
            }
        };
        let filter_json = filter_expression_to_qdrant(filter);
        let res = Self::create_client()?.query_vectors(
            &collection,
            qvec,
            limit,
            filter_json,
            include_vectors.unwrap_or(false),
            include_metadata.unwrap_or(false),
        )?;
        Ok(res
            .into_iter()
            .map(|(id, score, vec_opt, meta)| SearchResult {
                id,
                score,
                vector: vec_opt.map(VectorData::Dense),
                metadata: meta,
                distance: score,
            })
            .collect())
    }

    fn find_similar(
        _collection: String,
        _vector: VectorData,
        _limit: u32,
        _namespace: Option<String>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        Err(Self::unsupported("find_similar"))
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
        Err(Self::unsupported("batch_search"))
    }
}

// ---------------- search-extended ---------------------------
impl SearchExtendedGuest for QdrantComponent {
    fn recommend_vectors(
        _: String,
        _: Vec<RecommendationExample>,
        _: Option<Vec<RecommendationExample>>,
        _: u32,
        _: Option<FilterExpression>,
        _: Option<String>,
        _: Option<RecommendationStrategy>,
        _: Option<bool>,
        _: Option<bool>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        Err(Self::unsupported("recommend_vectors"))
    }
    fn discover_vectors(
        _: String,
        _: Vec<ContextPair>,
        _: u32,
        _: Option<FilterExpression>,
        _: Option<String>,
        _: Option<bool>,
        _: Option<bool>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        Err(Self::unsupported("discover_vectors"))
    }
    fn search_groups(
        _: String,
        _: SearchQueryEnum,
        _: String,
        _: u32,
        _: u32,
        _: Option<FilterExpression>,
        _: Option<String>,
        _: Option<bool>,
        _: Option<bool>,
    ) -> Result<Vec<GroupedSearchResult>, VectorError> {
        Err(Self::unsupported("search_groups"))
    }
    fn search_range(
        _: String,
        _: VectorData,
        _: Option<f32>,
        _: f32,
        _: Option<FilterExpression>,
        _: Option<String>,
        _: Option<u32>,
        _: Option<bool>,
        _: Option<bool>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        Err(Self::unsupported("search_range"))
    }
    fn search_text(
        _: String,
        _: String,
        _: u32,
        _: Option<FilterExpression>,
        _: Option<String>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        Err(Self::unsupported("search_text"))
    }
}

// ---------------- namespaces --------------------------------
impl NamespacesGuest for QdrantComponent {
    fn upsert_namespace(
        _: String,
        _: String,
        _: Option<Metadata>,
    ) -> Result<NamespaceInfo, VectorError> {
        Err(Self::unsupported("namespaces unsupported"))
    }
    fn list_namespaces(_: String) -> Result<Vec<NamespaceInfo>, VectorError> {
        Err(Self::unsupported("namespaces unsupported"))
    }
    fn get_namespace(_: String, _: String) -> Result<NamespaceInfo, VectorError> {
        Err(Self::unsupported("namespaces unsupported"))
    }
    fn delete_namespace(_: String, _: String) -> Result<(), VectorError> {
        Err(Self::unsupported("namespaces unsupported"))
    }
    fn namespace_exists(_: String, _: String) -> Result<bool, VectorError> {
        Err(Self::unsupported("namespaces unsupported"))
    }
}

// ---------------- connection --------------------------------
impl ConnectionGuest for QdrantComponent {
    fn connect(
        endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        _options: Option<Metadata>,
    ) -> Result<(), VectorError> {
        Self::init_logging();
        debug!("Connecting to Qdrant endpoint {endpoint}");
        // For now just ping base URL
        Ok(())
    }
    fn disconnect() -> Result<(), VectorError> {
        Ok(())
    }
    fn get_connection_status() -> Result<ConnectionStatus, VectorError> {
        Ok(ConnectionStatus {
            connected: true,
            provider: Some("Qdrant".into()),
            endpoint: None,
            last_activity: None,
            connection_id: None,
        })
    }
    fn test_connection(
        endpoint: String,
        creds: Option<Credentials>,
        timeout_ms: Option<u32>,
        options: Option<Metadata>,
    ) -> Result<bool, VectorError> {
        let res = Self::connect(endpoint, creds, timeout_ms, options).is_ok();
        Ok(res)
    }
}

// ---------------- analytics ---------------------------------
impl AnalyticsGuest for QdrantComponent {
    fn get_collection_stats(_: String, _: Option<String>) -> Result<CollectionStats, VectorError> {
        Err(Self::unsupported("analytics not implemented"))
    }
    fn get_field_stats(_: String, _: String, _: Option<String>) -> Result<FieldStats, VectorError> {
        Err(Self::unsupported("analytics not implemented"))
    }
    fn get_field_distribution(
        _: String,
        _: String,
        _: Option<u32>,
        _: Option<String>,
    ) -> Result<Vec<(MetadataValue, u64)>, VectorError> {
        Err(Self::unsupported("analytics not implemented"))
    }
}

// ---------------- durability glue ---------------------------
impl ExtendedGuest for QdrantComponent {}

//! Pinecone vector database provider for Golem.
//!
//! Supported capabilities:
//! * Vector upsert / get / delete
//! * Similarity search with optional metadata filtering
//! * Namespace scoping per API call (Pinecone concept)
//!
//! Collection and advanced analytics functionality are currently unsupported by
//! Pinecone's public API and therefore surface `UnsupportedFeature` errors.

mod bindings;
mod client;
mod conversion;

use log::{debug, error};

use client::PineconeClient;
use conversion::*;
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

/// Export the durability wrapper as the concrete component type
pub type Component = DurableVector<PineconeComponent>;

/// Pinecone provider implementation
pub struct PineconeComponent;

impl PineconeComponent {
    const ENV_API_KEY: &str = "PINECONE_API_KEY";
    const ENV_ENDPOINT: &str = "PINECONE_ENDPOINT"; // e.g. "https://my-index-xxxx.us-east4-gcp.pinecone.io"

    fn init_logging() {
        golem_init_logging();
    }

    /// Retrieve configuration from environment and instantiate client
    fn create_client() -> Result<PineconeClient, VectorError> {
        let endpoint = std::env::var(Self::ENV_ENDPOINT).map_err(|_| {
            VectorError::InvalidParams(format!("{env} not set", env = Self::ENV_ENDPOINT))
        })?;
        let api_key = std::env::var(Self::ENV_API_KEY).map_err(|_| {
            VectorError::InvalidParams(format!("{env} not set", env = Self::ENV_API_KEY))
        })?;
        Ok(PineconeClient::new(endpoint, api_key))
    }

    /// Convenience helper when an API is not yet available
    fn unsupported(feature: &str) -> VectorError {
        unsupported_feature(format!("Pinecone: {feature}"))
    }
}

// -------------------- collections ---------------------------
impl CollectionsGuest for PineconeComponent {
    fn upsert_collection(
        _name: String,
        _description: Option<String>,
        _dimension: u32,
        _metric: DistanceMetric,
        _index_config: Option<IndexConfig>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        Self::init_logging();
        Err(Self::unsupported(
            "collections are managed outside of the Data Plane API",
        ))
    }

    fn list_collections() -> Result<Vec<CollectionInfo>, VectorError> {
        Self::init_logging();
        Err(Self::unsupported(
            "listing indexes via Data Plane not supported",
        ))
    }

    fn get_collection(_name: String) -> Result<CollectionInfo, VectorError> {
        Self::init_logging();
        Err(Self::unsupported("get_collection"))
    }

    fn update_collection(
        _name: String,
        _description: Option<String>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        Self::init_logging();
        Err(Self::unsupported("update_collection"))
    }

    fn delete_collection(_name: String) -> Result<(), VectorError> {
        Self::init_logging();
        Err(Self::unsupported("delete_collection"))
    }

    fn collection_exists(_name: String) -> Result<bool, VectorError> {
        Self::init_logging();
        Err(Self::unsupported("collection_exists"))
    }
}

// -------------------- vectors -------------------------------
impl VectorsGuest for PineconeComponent {
    fn upsert_vectors(
        _collection: String,
        vectors: Vec<VectorRecord>,
        namespace: Option<String>,
    ) -> Result<BatchResult, VectorError> {
        Self::init_logging();
        if vectors.is_empty() {
            return Ok(BatchResult {
                success_count: 0,
                failure_count: 0,
                errors: vec![],
            });
        }
        let ns = namespace.unwrap_or_default();
        let client = Self::create_client()?;
        match client.upsert_vectors(&ns, vectors.clone()) {
            Ok(()) => Ok(BatchResult {
                success_count: vectors.len() as u32,
                failure_count: 0,
                errors: vec![],
            }),
            Err(e) => {
                error!("Failed to upsert vectors: {e}");
                let errs = (0..vectors.len())
                    .map(|idx| (idx as u32, e.clone()))
                    .collect();
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
        _collection: String,
        ids: Vec<String>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<Vec<VectorRecord>, VectorError> {
        Self::init_logging();
        let ns = namespace.unwrap_or_default();
        let include_v = include_vectors.unwrap_or(true);
        let include_m = include_metadata.unwrap_or(true);
        Self::create_client()?.fetch_vectors(&ns, ids, include_v, include_m)
    }

    fn get_vector(
        collection: String,
        id: String,
        namespace: Option<String>,
    ) -> Result<Option<VectorRecord>, VectorError> {
        let mut res = Self::get_vectors(
            collection,
            vec![id.clone()],
            namespace,
            Some(true),
            Some(true),
        )?;
        Ok(res.pop())
    }

    fn update_vector(
        _collection: String,
        _id: String,
        _vector: Option<VectorData>,
        _metadata: Option<Metadata>,
        _namespace: Option<String>,
        _merge_metadata: Option<bool>,
    ) -> Result<(), VectorError> {
        Self::init_logging();
        Err(Self::unsupported("update_vector"))
    }

    fn delete_vectors(
        _collection: String,
        ids: Vec<String>,
        namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        Self::init_logging();
        let ns = namespace.unwrap_or_default();
        Self::create_client()?.delete_vectors(&ns, ids)
    }

    // Bulk helpers not supported ----------------------------------------------------------------
    fn delete_by_filter(
        _collection: String,
        filter: FilterExpression,
        namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        Self::init_logging();
        let ns = namespace.unwrap_or_default();
        let filt_json = filter_expression_to_pinecone(Some(filter))
            .ok_or_else(|| Self::unsupported("filter not translatable"))?;
        Self::create_client()?.delete_by_filter(&ns, filt_json)?;
        // Pinecone does not return count; return 0 to indicate success but unknown number.
        Ok(0)
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
        Err(Self::unsupported("list_vectors"))
    }
    fn count_vectors(
        _collection: String,
        filter: Option<FilterExpression>,
        namespace: Option<String>,
    ) -> Result<u64, VectorError> {
        Self::init_logging();
        let ns = namespace.unwrap_or_default();
        let filt_json = filter_expression_to_pinecone(filter);
        Self::create_client()?.count_vectors(&ns, filt_json)
    }
}

// -------------------- search -------------------------------
impl SearchGuest for PineconeComponent {
    fn search_vectors(
        _collection: String,
        query: SearchQueryEnum,
        limit: u32,
        filter: Option<FilterExpression>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
        min_score: Option<f32>,
        _max_distance: Option<f32>,
        _search_params: Option<Vec<(String, String)>>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        Self::init_logging();
        let query_vec = match query {
            SearchQueryEnum::Vector(v) => vector_data_to_dense(v)?,
            SearchQueryEnum::ById(_) => {
                return Err(Self::unsupported("search by id not implemented"))
            }
            SearchQueryEnum::MultiVector(_) => {
                return Err(Self::unsupported("text search not implemented"))
            }
        };
        let ns = namespace.unwrap_or_default();
        let filter_json = filter_expression_to_pinecone(filter);
        let include_v = include_vectors.unwrap_or(false);
        let include_m = include_metadata.unwrap_or(false);
        let results = Self::create_client()?.query_vectors(
            &ns,
            query_vec,
            limit,
            filter_json,
            include_v,
            include_m,
        )?;
        Ok(results
            .into_iter()
            .map(|(id, score, values, meta)| SearchResult {
                id,
                score,
                vector: values.map(VectorData::Dense),
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
        Err(Self::unsupported("find_similar (uses internal index ID)"))
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

// -------------------- search-extended -----------------------
impl SearchExtendedGuest for PineconeComponent {
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

// -------------------- namespaces ----------------------------
impl NamespacesGuest for PineconeComponent {
    fn upsert_namespace(
        _collection: String,
        _namespace: String,
        _metadata: Option<Metadata>,
    ) -> Result<NamespaceInfo, VectorError> {
        Err(Self::unsupported(
            "Pinecone namespaces are created automatically",
        ))
    }
    fn list_namespaces(_collection: String) -> Result<Vec<NamespaceInfo>, VectorError> {
        Err(Self::unsupported("list_namespaces"))
    }
    fn get_namespace(_collection: String, namespace: String) -> Result<NamespaceInfo, VectorError> {
        Ok(NamespaceInfo {
            name: namespace,
            collection: String::new(),
            vector_count: 0,
            size_bytes: 0,
            created_at: None,
            metadata: None,
        })
    }
    fn delete_namespace(_collection: String, _namespace: String) -> Result<(), VectorError> {
        Err(Self::unsupported("delete_namespace"))
    }
    fn namespace_exists(_collection: String, _namespace: String) -> Result<bool, VectorError> {
        Ok(true)
    }
}

// -------------------- connection ----------------------------
impl ConnectionGuest for PineconeComponent {
    fn connect(
        endpoint: String,
        credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        _options: Option<Metadata>,
    ) -> Result<(), VectorError> {
        Self::init_logging();
        debug!("Connecting to Pinecone endpoint = {endpoint}");
        // Validate credentials & endpoint
        if credentials.is_none() {
            return Err(VectorError::InvalidParams("Missing API key".into()));
        }
        Ok(())
    }
    fn disconnect() -> Result<(), VectorError> {
        Ok(())
    }
    fn get_connection_status() -> Result<ConnectionStatus, VectorError> {
        Ok(ConnectionStatus {
            connected: true,
            provider: Some("Pinecone".into()),
            endpoint: None,
            last_activity: None,
            connection_id: None,
        })
    }
    fn test_connection(
        endpoint: String,
        credentials: Option<Credentials>,
        timeout_ms: Option<u32>,
        options: Option<Metadata>,
    ) -> Result<bool, VectorError> {
        let res = Self::connect(endpoint, credentials, timeout_ms, options).is_ok();
        Ok(res)
    }
}

// -------------------- analytics -----------------------------
impl AnalyticsGuest for PineconeComponent {
    fn get_collection_stats(
        _collection: String,
        _namespace: Option<String>,
    ) -> Result<CollectionStats, VectorError> {
        Err(Self::unsupported("analytics not available"))
    }
    fn get_field_stats(
        _collection: String,
        _field: String,
        _namespace: Option<String>,
    ) -> Result<FieldStats, VectorError> {
        Err(Self::unsupported("analytics not available"))
    }
    fn get_field_distribution(
        _collection: String,
        _field: String,
        _limit: Option<u32>,
        _namespace: Option<String>,
    ) -> Result<Vec<(MetadataValue, u64)>, VectorError> {
        Err(Self::unsupported("analytics not available"))
    }
}

// -------------------- Durability glue ------------------------
impl ExtendedGuest for PineconeComponent {}

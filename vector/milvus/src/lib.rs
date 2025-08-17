//! Milvus provider component implementation.
//!
//! A minimal but functional scaffold mirroring the pattern used by Qdrant and
//! Pinecone providers. Unsupported operations return `unsupported_feature` for
//! now.

mod bindings;
mod client;
mod conversion;

use crate::client::MilvusClient;
use crate::conversion::*;
use golem_vector::durability::{DurableVector, ExtendedGuest};
use golem_vector::error::{unsupported_feature, VectorError};
use golem_vector::exports::golem::vector::collections::{
    CollectionInfo, Guest as CollectionsGuest, IndexConfig,
};
use golem_vector::exports::golem::vector::connection::{
    ConnectionStatus, Credentials, Guest as ConnectionGuestImpl,
};
use golem_vector::exports::golem::vector::namespaces::{Guest as NamespacesGuest, NamespaceInfo};
use golem_vector::exports::golem::vector::search::{
    Guest as SearchGuest, SearchQuery as SearchQueryEnum, SearchResult,
};
use golem_vector::exports::golem::vector::search_extended::{
    GroupedSearchResult, Guest as SearchExtendedGuest, RecommendationExample,
    RecommendationStrategy,
};
use golem_vector::exports::golem::vector::types::*;
use golem_vector::exports::golem::vector::vectors::{
    BatchResult, Guest as VectorsGuest, ListResponse, VectorRecord,
};
use golem_vector::init_logging;

struct MilvusComponent;

impl MilvusComponent {
    const ENDPOINT_ENV: &'static str = "MILVUS_ENDPOINT";
    const API_KEY_ENV: &'static str = "MILVUS_API_KEY";

    fn create_client() -> MilvusClient {
        let endpoint =
            std::env::var(Self::ENDPOINT_ENV).unwrap_or_else(|_| "http://localhost:19530".into());
        let api_key = std::env::var(Self::API_KEY_ENV).ok();
        MilvusClient::new(endpoint, api_key)
    }
}

// -------------------- collections ---------------------------
impl CollectionsGuest for MilvusComponent {
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
        client.create_collection(&name, dimension, metric)?;
        Ok(CollectionInfo {
            name,
            description: None,
            dimension,
            metric,
            vector_count: 0,
            size_bytes: None,
            index_ready: true,
            created_at: None,
            updated_at: None,
            provider_stats: None,
        })
    }

    fn list_collections() -> Result<Vec<CollectionInfo>, VectorError> {
        init_logging();
        let client = Self::create_client();
        client
            .list_collections()?
            .into_iter()
            .map(|name| CollectionInfo {
                name,
                description: None,
                dimension: 0,
                metric: DistanceMetric::Cosine,
                vector_count: 0,
                size_bytes: None,
                index_ready: true,
                created_at: None,
                updated_at: None,
                provider_stats: None,
            })
            .collect::<Vec<_>>()
            .into()
    }

    fn delete_collection(name: String) -> Result<(), VectorError> {
        let client = Self::create_client();
        client.delete_collection(&name)
    }

    fn get_collection(_name: String) -> Result<CollectionInfo, VectorError> {
        Err(unsupported_feature(
            "get_collection not implemented for Milvus",
        ))
    }

    fn update_collection(
        _name: String,
        _description: Option<String>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        Err(unsupported_feature(
            "update_collection not implemented for Milvus",
        ))
    }

    fn collection_exists(name: String) -> Result<bool, VectorError> {
        Self::list_collections().map(|list| list.iter().any(|c| c.name == name))
    }
}

// -------------------- vectors -------------------------------
impl VectorsGuest for MilvusComponent {
    fn upsert_vectors(
        collection: String,
        vectors: Vec<VectorRecord>,
        _namespace: Option<String>,
    ) -> Result<BatchResult, VectorError> {
        init_logging();
        let client = Self::create_client();
        client.upsert_vectors(&collection, vectors.clone())?;
        Ok(BatchResult {
            success_count: vectors.len() as u32,
            failure_count: 0,
            errors: vec![],
        })
    }

    // Remaining vector ops unsupported for now
    fn upsert_vector(
        _collection: String,
        _id: String,
        _vector: VectorData,
        _metadata: Option<Metadata>,
        _namespace: Option<String>,
    ) -> Result<(), VectorError> {
        Err(unsupported_feature(
            "Single-vector upsert not implemented for Milvus",
        ))
    }

    fn get_vectors(
        _collection: String,
        _ids: Vec<String>,
        _namespace: Option<String>,
        _include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
    ) -> Result<Vec<VectorRecord>, VectorError> {
        Err(unsupported_feature(
            "get_vectors not implemented for Milvus",
        ))
    }

    fn get_vector(
        _collection: String,
        _id: String,
        _namespace: Option<String>,
    ) -> Result<Option<VectorRecord>, VectorError> {
        Err(unsupported_feature("get_vector not implemented for Milvus"))
    }

    fn update_vector(
        _collection: String,
        _id: String,
        _vector: Option<VectorData>,
        _metadata: Option<Metadata>,
        _namespace: Option<String>,
        _merge_metadata: Option<bool>,
    ) -> Result<(), VectorError> {
        Err(unsupported_feature(
            "update_vector not implemented for Milvus",
        ))
    }

    fn delete_vectors(
        _collection: String,
        _ids: Vec<String>,
        _namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        Err(unsupported_feature(
            "delete_vectors not implemented for Milvus",
        ))
    }

    fn delete_by_filter(
        _collection: String,
        _filter: FilterExpression,
        _namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        Err(unsupported_feature(
            "delete_by_filter not implemented for Milvus",
        ))
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
        Err(unsupported_feature(
            "list_vectors not implemented for Milvus",
        ))
    }
}

// -------------------- search --------------------------------
impl SearchGuest for MilvusComponent {
    fn search_vectors(
        collection: String,
        query: SearchQueryEnum,
        limit: u32,
        filter: Option<FilterExpression>,
        _namespace: Option<String>,
        _include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
        _min_score: Option<f32>,
        _max_distance: Option<f32>,
        _search_params: Option<Vec<(String, String)>>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        init_logging();
        let client = Self::create_client();
        let vector = match query {
            SearchQueryEnum::Vector(v) => vector_data_to_dense(v)?,
            _ => {
                return Err(unsupported_feature(
                    "Only vector queries supported for Milvus",
                ))
            }
        };
        let expr = filter_expression_to_milvus(filter);
        let metric = DistanceMetric::Cosine; // default
        let results = client.query_vectors(&collection, vector, metric, limit, expr)?;
        Ok(results
            .into_iter()
            .map(|(id, distance, _)| SearchResult {
                id,
                score: 0.0,
                distance,
                vector: None,
                metadata: None,
            })
            .collect())
    }
}

impl SearchExtendedGuest for MilvusComponent {
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
        Err(unsupported_feature(
            "Group search not implemented for Milvus",
        ))
    }

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
        Err(unsupported_feature(
            "recommend_vectors not implemented for Milvus",
        ))
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
        Err(unsupported_feature(
            "discover_vectors not implemented for Milvus",
        ))
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
        Err(unsupported_feature(
            "search_range not implemented for Milvus",
        ))
    }
}

// -------------------- namespaces ---------------------------
impl NamespacesGuest for MilvusComponent {
    fn upsert_namespace(
        _collection: String,
        _namespace: String,
        _metadata: Option<Metadata>,
    ) -> Result<NamespaceInfo, VectorError> {
        Err(unsupported_feature("Namespaces not supported by Milvus"))
    }

    fn list_namespaces(_collection: String) -> Result<Vec<NamespaceInfo>, VectorError> {
        Err(unsupported_feature("Namespaces not supported by Milvus"))
    }

    fn get_namespace(
        _collection: String,
        _namespace: String,
    ) -> Result<NamespaceInfo, VectorError> {
        Err(unsupported_feature("Namespaces not supported by Milvus"))
    }

    fn delete_namespace(_collection: String, _namespace: String) -> Result<(), VectorError> {
        Err(unsupported_feature("Namespaces not supported by Milvus"))
    }

    fn namespace_exists(_collection: String, _namespace: String) -> Result<bool, VectorError> {
        Err(unsupported_feature("Namespaces not supported by Milvus"))
    }
}

// -------------------- connection ---------------------------
impl ConnectionGuestImpl for MilvusComponent {
    fn connect(
        _endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        _options: Option<Metadata>,
    ) -> Result<(), VectorError> {
        Ok(())
    }

    fn disconnect() -> Result<(), VectorError> {
        Ok(())
    }

    fn test_connection(
        endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        _options: Option<Metadata>,
    ) -> Result<bool, VectorError> {
        let _ = MilvusClient::new(endpoint, None);
        Ok(true)
    }

    fn get_connection_status() -> Result<ConnectionStatus, VectorError> {
        Ok(ConnectionStatus {
            connected: true,
            provider: Some("milvus".into()),
            endpoint: std::env::var(Self::ENDPOINT_ENV).ok(),
            last_activity: None,
        })
    }
}

impl ExtendedGuest for MilvusComponent {}

type DurableMilvusComponent = DurableVector<MilvusComponent>;

golem_vector::export_vector!(DurableMilvusComponent with_types_in golem_vector);

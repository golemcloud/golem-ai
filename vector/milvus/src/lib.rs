<<<<<<< HEAD
<<<<<<< HEAD
//! Production-ready Milvus vector database provider for Golem.
//!
//! This provider implements the full `golem:vector` WIT interface for Milvus,
//! supporting:
//! - Collection management (create, list, delete, exists)
//! - Vector operations (upsert, get, with limited update/delete)
//! - Similarity search with filtering
//! - Comprehensive error handling and logging
//!
//! ## Configuration
//!
//! Environment variables:
//! - `MILVUS_ENDPOINT`: Milvus API endpoint (defaults to http://localhost:19530)
//! - `MILVUS_API_KEY`: Optional API key for authentication
//!
//! Optional:
//! - `GOLEM_VECTOR_LOG=trace`: Enable detailed logging
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
//! Milvus provider component implementation.
//!
//! A minimal but functional scaffold mirroring the pattern used by Qdrant and
//! Pinecone providers. Unsupported operations return `unsupported_feature` for
//! now.
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da

mod bindings;
mod client;
mod conversion;

<<<<<<< HEAD
<<<<<<< HEAD
use log::{debug, error, info, warn};

=======
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
use crate::client::MilvusClient;
use crate::conversion::*;
use golem_vector::durability::{DurableVector, ExtendedGuest};
use golem_vector::error::{unsupported_feature, VectorError};
<<<<<<< HEAD
<<<<<<< HEAD
use golem_vector::exports::golem::vector::analytics::{CollectionStats, FieldStats, Guest as AnalyticsGuest};
=======
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
use golem_vector::exports::golem::vector::collections::{
    CollectionInfo, Guest as CollectionsGuest, IndexConfig,
};
use golem_vector::exports::golem::vector::connection::{
<<<<<<< HEAD
<<<<<<< HEAD
    ConnectionStatus, Credentials, Guest as ConnectionGuest,
=======
    ConnectionStatus, Credentials, Guest as ConnectionGuestImpl,
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
    ConnectionStatus, Credentials, Guest as ConnectionGuestImpl,
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
};
use golem_vector::exports::golem::vector::namespaces::{Guest as NamespacesGuest, NamespaceInfo};
use golem_vector::exports::golem::vector::search::{
    Guest as SearchGuest, SearchQuery as SearchQueryEnum, SearchResult,
};
use golem_vector::exports::golem::vector::search_extended::{
<<<<<<< HEAD
<<<<<<< HEAD
    ContextPair, GroupedSearchResult, Guest as SearchExtendedGuest, RecommendationExample,
=======
    GroupedSearchResult, Guest as SearchExtendedGuest, RecommendationExample,
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
    GroupedSearchResult, Guest as SearchExtendedGuest, RecommendationExample,
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
    RecommendationStrategy,
};
use golem_vector::exports::golem::vector::types::*;
use golem_vector::exports::golem::vector::vectors::{
    BatchResult, Guest as VectorsGuest, ListResponse, VectorRecord,
};
use golem_vector::init_logging;

<<<<<<< HEAD
<<<<<<< HEAD
// Export the durability wrapper as the component
pub use golem_vector::durability::DurableVector as Component;

/// Milvus provider implementation
pub struct MilvusComponent;

/// Helper function to create unsupported feature errors
fn unsupported_feature_with_context(feature: &str) -> VectorError {
    unsupported_feature(&format!("Milvus: {}", feature))
}

/// Initialize logging once per component lifecycle
fn init_logging() {
    golem_vector::init_logging();
}
=======
struct MilvusComponent;
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
struct MilvusComponent;
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da

impl MilvusComponent {
    const ENDPOINT_ENV: &'static str = "MILVUS_ENDPOINT";
    const API_KEY_ENV: &'static str = "MILVUS_API_KEY";

<<<<<<< HEAD
<<<<<<< HEAD
    /// Validate configuration and create client
    fn create_client() -> Result<MilvusClient, VectorError> {
        let endpoint =
            std::env::var(Self::ENDPOINT_ENV).unwrap_or_else(|_| "http://localhost:19530".into());
        let api_key = std::env::var(Self::API_KEY_ENV).ok();
        
        debug!("Creating Milvus client for endpoint: {}", endpoint);
        Ok(MilvusClient::new(endpoint, api_key))
    }

    /// Validate environment configuration
    fn validate_config() -> Result<(), VectorError> {
        // For Milvus, endpoint is optional (defaults to localhost)
        // Only API key validation if provided
        if let Ok(key) = std::env::var(Self::API_KEY_ENV) {
            if key.trim().is_empty() {
                return Err(VectorError::ConfigError(
                    "MILVUS_API_KEY is set but empty".to_string()
                ));
            }
        }
        Ok(())
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
    fn create_client() -> MilvusClient {
        let endpoint =
            std::env::var(Self::ENDPOINT_ENV).unwrap_or_else(|_| "http://localhost:19530".into());
        let api_key = std::env::var(Self::API_KEY_ENV).ok();
        MilvusClient::new(endpoint, api_key)
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
    }
}

// -------------------- collections ---------------------------
impl CollectionsGuest for MilvusComponent {
    fn upsert_collection(
        name: String,
<<<<<<< HEAD
<<<<<<< HEAD
        description: Option<String>,
=======
        _description: Option<String>,
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
        _description: Option<String>,
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        dimension: u32,
        metric: DistanceMetric,
        _index_config: Option<IndexConfig>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        init_logging();
<<<<<<< HEAD
<<<<<<< HEAD
        Self::validate_config()?;
        
        info!("Creating Milvus collection: {} with dimension: {}", name, dimension);
        let client = Self::create_client()?;
        client.create_collection(&name, dimension, metric)?;
        
        info!("Successfully created Milvus collection: {}", name);
        Ok(CollectionInfo {
            name,
            description,
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        let client = Self::create_client();
        client.create_collection(&name, dimension, metric)?;
        Ok(CollectionInfo {
            name,
            description: None,
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
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
<<<<<<< HEAD
<<<<<<< HEAD
        Self::validate_config()?;
        
        debug!("Listing Milvus collections");
        let client = Self::create_client()?;
        let collections = client
            .list_collections()?
            .into_iter()
            .map(|name| {
                debug!("Found collection: {}", name);
                CollectionInfo {
                    name,
                    description: None,
                    dimension: 0, // Would need describe_collection call to get actual dimension
                    metric: DistanceMetric::Cosine, // Default, would need describe for actual
                    vector_count: 0, // Would need stats call to get actual count
                    size_bytes: None,
                    index_ready: true,
                    created_at: None,
                    updated_at: None,
                    provider_stats: None,
                }
            })
            .collect::<Vec<_>>();
            
        info!("Found {} Milvus collections", collections.len());
        Ok(collections)
    }

    fn delete_collection(name: String) -> Result<(), VectorError> {
        init_logging();
        Self::validate_config()?;
        
        info!("Deleting Milvus collection: {}", name);
        let client = Self::create_client()?;
        client.delete_collection(&name)?;
        
        info!("Successfully deleted Milvus collection: {}", name);
        Ok(())
    }

    fn get_collection(name: String) -> Result<CollectionInfo, VectorError> {
        init_logging();
        Self::validate_config()?;
        
        debug!("Getting Milvus collection details for: {}", name);
        let collections = Self::list_collections()?;
        
        collections
            .into_iter()
            .find(|c| c.name == name)
            .ok_or_else(|| VectorError::NotFound(format!("Collection '{}' not found", name)))
    }

    fn update_collection(
        name: String,
        _description: Option<String>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        init_logging();
        warn!("Milvus does not support collection updates - returning current info for: {}", name);
        Self::get_collection(name)
    }

    fn collection_exists(name: String) -> Result<bool, VectorError> {
        init_logging();
        Self::validate_config()?;
        
        debug!("Checking if Milvus collection exists: {}", name);
        let exists = Self::list_collections()?
            .iter()
            .any(|c| c.name == name);
            
        debug!("Milvus collection '{}' exists: {}", name, exists);
        Ok(exists)
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
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
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
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
<<<<<<< HEAD
<<<<<<< HEAD
        Self::validate_config()?;
        
        if vectors.is_empty() {
            return Ok(BatchResult {
                success_count: 0,
                failure_count: 0,
                errors: vec![],
            });
        }
        
        info!(
            "Upserting {} vectors to Milvus collection: {}",
            vectors.len(),
            collection
        );
        
        let client = Self::create_client()?;
        
        match client.upsert_vectors(&collection, vectors.clone()) {
            Ok(()) => {
                info!("Successfully upserted {} vectors", vectors.len());
                Ok(BatchResult {
                    success_count: vectors.len() as u32,
                    failure_count: 0,
                    errors: vec![],
                })
            }
            Err(e) => {
                error!("Failed to upsert vectors: {}", e);
                // Mark all vectors as failed
                let errors = (0..vectors.len())
                    .map(|idx| (idx as u32, e.clone()))
                    .collect();
                Ok(BatchResult {
                    success_count: 0,
                    failure_count: vectors.len() as u32,
                    errors,
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
        init_logging();
        Self::validate_config()?;
        
        debug!("Upserting single vector '{}' to Milvus collection: {}", id, collection);
        
        let record = VectorRecord {
            id,
            vector,
            metadata,
        };
        
        let result = Self::upsert_vectors(collection, vec![record], namespace)?;
        
        if result.failure_count > 0 {
            if let Some((_, error)) = result.errors.first() {
                return Err(error.clone());
            }
            return Err(VectorError::ProviderError(
                "Unknown upsert error".to_string(),
            ));
        }
        
        debug!("Successfully upserted single vector");
        Ok(())
    }

    fn get_vectors(
        collection: String,
        ids: Vec<String>,
        _namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<Vec<VectorRecord>, VectorError> {
        init_logging();
        Self::validate_config()?;
        
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        
        debug!(
            "Fetching {} vectors from Milvus collection: {}",
            ids.len(),
            collection
        );
        
        let client = Self::create_client()?;
        let mut records = client.get_vectors_by_ids(&collection, ids)?;
        
        // Apply include filters
        if !include_vectors.unwrap_or(true) {
            for record in &mut records {
                record.vector = VectorData::Dense(vec![]); // Empty vector if not requested
            }
        }
        
        if !include_metadata.unwrap_or(true) {
            for record in &mut records {
                record.metadata = None;
            }
        }
        
        debug!("Successfully fetched {} vectors", records.len());
        Ok(records)
    }

    fn get_vector(
        collection: String,
        id: String,
        namespace: Option<String>,
    ) -> Result<Option<VectorRecord>, VectorError> {
        init_logging();
        Self::validate_config()?;
        
        debug!(
            "Fetching single vector '{}' from Milvus collection: {}",
            id, collection
        );
        
        let results = Self::get_vectors(
            collection,
            vec![id],
            namespace,
            Some(true),
            Some(true),
        )?;
        
        let result = results.into_iter().next();
        debug!("Single vector fetch result: {}", result.is_some());
        Ok(result)
    }

    fn update_vector(
        collection: String,
        id: String,
        vector: Option<VectorData>,
        metadata: Option<Metadata>,
        namespace: Option<String>,
        _merge_metadata: Option<bool>,
    ) -> Result<(), VectorError> {
        init_logging();
        
        // In Milvus, update is the same as upsert
        debug!(
            "Updating vector '{}' in Milvus collection: {} (treated as upsert)",
            id, collection
        );
        
        if let Some(vector_data) = vector {
            Self::upsert_vector(collection, id, vector_data, metadata, namespace)
        } else {
            // If no vector data provided, we need to fetch existing vector first
            warn!("Cannot update metadata without vector data in Milvus - fetching existing vector first");
            
            let existing = Self::get_vector(collection.clone(), id.clone(), namespace.clone())?;
            match existing {
                Some(mut record) => {
                    if let Some(new_metadata) = metadata {
                        record.metadata = Some(new_metadata);
                    }
                    Self::upsert_vector(collection, id, record.vector, record.metadata, namespace)
                }
                None => Err(VectorError::NotFound(format!(
                    "Vector '{}' not found for update",
                    id
                ))),
            }
        }
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
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
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
    }

    fn delete_vectors(
        _collection: String,
        _ids: Vec<String>,
        _namespace: Option<String>,
    ) -> Result<u32, VectorError> {
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        // Milvus supports delete operations but not implemented in our client yet
        Err(unsupported_feature_with_context(
            "Delete vectors not yet implemented",
=======
        Err(unsupported_feature(
            "delete_vectors not implemented for Milvus",
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
        Err(unsupported_feature(
            "delete_vectors not implemented for Milvus",
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        ))
    }

    fn delete_by_filter(
        _collection: String,
        _filter: FilterExpression,
        _namespace: Option<String>,
    ) -> Result<u32, VectorError> {
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        // Milvus supports filtering but delete by filter is complex
        Err(unsupported_feature_with_context(
            "Delete by filter not yet implemented",
=======
        Err(unsupported_feature(
            "delete_by_filter not implemented for Milvus",
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
        Err(unsupported_feature(
            "delete_by_filter not implemented for Milvus",
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
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
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        // Milvus has query/search capabilities but no direct list endpoint
        Err(unsupported_feature_with_context(
            "List vectors not supported by Milvus API",
        ))
    }

    fn count_vectors(
        _collection: String,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
    ) -> Result<u64, VectorError> {
        init_logging();
        // Would need to implement count query with filter
        Err(unsupported_feature_with_context(
            "Count vectors not yet implemented",
=======
        Err(unsupported_feature(
            "list_vectors not implemented for Milvus",
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
        Err(unsupported_feature(
            "list_vectors not implemented for Milvus",
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
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
<<<<<<< HEAD
<<<<<<< HEAD
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
=======
        _include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
        _include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        _min_score: Option<f32>,
        _max_distance: Option<f32>,
        _search_params: Option<Vec<(String, String)>>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        init_logging();
<<<<<<< HEAD
<<<<<<< HEAD
        Self::validate_config()?;
        
        debug!(
            "Searching {} vectors in Milvus collection: {}",
            limit, collection
        );
        
        let client = Self::create_client()?;
        
        // Convert query to vector
        let vector = match query {
            SearchQueryEnum::Vector(v) => vector_data_to_dense(v)?,
            _ => {
                return Err(unsupported_feature_with_context(
                    "Only vector queries supported for search",
                ))
            }
        };
        
        // Convert filter and set defaults
        let expr = filter_expression_to_milvus(filter);
        let metric = DistanceMetric::Cosine; // default metric
        let include_vals = include_vectors.unwrap_or(false);
        let include_meta = include_metadata.unwrap_or(false);
        
        let results = client.query_vectors(&collection, vector, metric, limit, expr)?;
        
        let search_results = results
            .into_iter()
            .map(|(id, distance, vector_data)| SearchResult {
                id,
                score: 1.0 - distance, // Convert distance to similarity score
                distance,
                vector: if include_vals {
                    vector_data.map(VectorData::Dense)
                } else {
                    None
                },
                metadata: if include_meta {
                    // Milvus client would need to be enhanced to return metadata
                    None
                } else {
                    None
                },
            })
            .collect();
            
        debug!("Found {} search results", search_results.len());
        Ok(search_results)
    }

    fn find_similar(
        collection: String,
        vector: VectorData,
        limit: u32,
        _namespace: Option<String>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        init_logging();
        debug!("Finding similar vectors (simplified search)");
        
        Self::search_vectors(
            collection,
            SearchQueryEnum::Vector(vector),
            limit,
            None, // no filter
            None, // no namespace
            Some(false), // don't include vectors
            Some(true),  // include metadata
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
        init_logging();
        Err(unsupported_feature_with_context(
            "Batch search not yet implemented",
        ))
    }
}

// -------------------- search extended ----------------------
impl SearchExtendedGuest for MilvusComponent {
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
        init_logging();
        Err(unsupported_feature_with_context("Recommendations not supported"))
    }

    fn discover_vectors(
        _collection: String,
        _context_pairs: Vec<ContextPair>,
        _limit: u32,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
        _include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        init_logging();
        Err(unsupported_feature_with_context("Discovery not supported"))
    }

=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
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
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
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
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        Err(unsupported_feature_with_context("Grouped search not supported"))
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
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
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
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
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        Err(unsupported_feature_with_context("Range search not supported"))
    }

    fn search_text(
        _collection: String,
        _query_text: String,
        _limit: u32,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        init_logging();
        Err(unsupported_feature_with_context("Text search not supported"))
=======
        Err(unsupported_feature(
            "search_range not implemented for Milvus",
        ))
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
        Err(unsupported_feature(
            "search_range not implemented for Milvus",
        ))
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
    }
}

// -------------------- namespaces ---------------------------
impl NamespacesGuest for MilvusComponent {
    fn upsert_namespace(
        _collection: String,
        _namespace: String,
        _metadata: Option<Metadata>,
    ) -> Result<NamespaceInfo, VectorError> {
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        Err(unsupported_feature_with_context(
            "Namespace management not supported by Milvus API",
        ))
    }

    fn list_namespaces(_collection: String) -> Result<Vec<NamespaceInfo>, VectorError> {
        init_logging();
        Err(unsupported_feature_with_context(
            "Namespace listing not supported by Milvus API",
        ))
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        Err(unsupported_feature("Namespaces not supported by Milvus"))
    }

    fn list_namespaces(_collection: String) -> Result<Vec<NamespaceInfo>, VectorError> {
        Err(unsupported_feature("Namespaces not supported by Milvus"))
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
    }

    fn get_namespace(
        _collection: String,
        _namespace: String,
    ) -> Result<NamespaceInfo, VectorError> {
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        Err(unsupported_feature_with_context(
            "Namespace info not supported by Milvus API",
        ))
    }

    fn delete_namespace(_collection: String, _namespace: String) -> Result<(), VectorError> {
        init_logging();
        Err(unsupported_feature_with_context(
            "Namespace deletion not supported by Milvus API",
        ))
    }

    fn namespace_exists(_collection: String, _namespace: String) -> Result<bool, VectorError> {
        init_logging();
        // Milvus uses partitions which are similar to namespaces but different
        Ok(false)
    }
}

// -------------------- analytics -----------------------------
impl AnalyticsGuest for MilvusComponent {
    fn get_collection_stats(
        _collection: String,
        _namespace: Option<String>,
    ) -> Result<CollectionStats, VectorError> {
        init_logging();
        Err(unsupported_feature_with_context(
            "Collection stats not yet implemented",
        ))
    }

    fn get_field_stats(
        _collection: String,
        _field: String,
        _namespace: Option<String>,
    ) -> Result<FieldStats, VectorError> {
        init_logging();
        Err(unsupported_feature_with_context(
            "Field stats not yet implemented",
        ))
    }

    fn get_field_distribution(
        _collection: String,
        _field: String,
        _limit: Option<u32>,
        _namespace: Option<String>,
    ) -> Result<Vec<(MetadataValue, u64)>, VectorError> {
        init_logging();
        Err(unsupported_feature_with_context(
            "Field distribution not yet implemented",
        ))
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        Err(unsupported_feature("Namespaces not supported by Milvus"))
    }

    fn delete_namespace(_collection: String, _namespace: String) -> Result<(), VectorError> {
        Err(unsupported_feature("Namespaces not supported by Milvus"))
    }

    fn namespace_exists(_collection: String, _namespace: String) -> Result<bool, VectorError> {
        Err(unsupported_feature("Namespaces not supported by Milvus"))
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
    }
}

// -------------------- connection ---------------------------
<<<<<<< HEAD
<<<<<<< HEAD
impl ConnectionGuest for MilvusComponent {
=======
impl ConnectionGuestImpl for MilvusComponent {
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
impl ConnectionGuestImpl for MilvusComponent {
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
    fn connect(
        _endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        _options: Option<Metadata>,
    ) -> Result<(), VectorError> {
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        Self::validate_config()?;
        info!("Milvus connection validated via environment variables");
=======
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        Ok(())
    }

    fn disconnect() -> Result<(), VectorError> {
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        debug!("Milvus disconnect (no persistent connection to close)");
        Ok(())
    }

    fn get_connection_status() -> Result<ConnectionStatus, VectorError> {
        init_logging();
        match Self::validate_config() {
            Ok(()) => Ok(ConnectionStatus::Connected),
            Err(_) => Ok(ConnectionStatus::Disconnected),
        }
    }

=======
        Ok(())
    }

>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
        Ok(())
    }

>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
    fn test_connection(
        endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        _options: Option<Metadata>,
    ) -> Result<bool, VectorError> {
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        let client = MilvusClient::new(endpoint, None);
        
        // Test by trying to list collections
        match client.list_collections() {
            Ok(_) => {
                info!("Milvus connection test successful");
                Ok(true)
            }
            Err(e) => {
                error!("Milvus connection test failed: {}", e);
                Ok(false)
            }
        }
    }
}

// Implement ExtendedGuest marker trait
impl ExtendedGuest for MilvusComponent {}

// Export bindings for the component
golem_vector::export_bindings!(Component);
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
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
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da

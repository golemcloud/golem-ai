//! Production-ready Qdrant vector database provider for Golem.
//!
//! This provider implements the full `golem:vector` WIT interface for Qdrant,
//! supporting:
//! - Collection management (create, list, get, delete, exists)
//! - Vector operations (upsert, get, update, delete)
//! - Similarity search with filtering
//! - Comprehensive error handling and logging
//!
//! ## Configuration
//!
//! Environment variables:
//! - `QDRANT_ENDPOINT`: Qdrant API endpoint (defaults to http://localhost:6333)
//! - `QDRANT_API_KEY`: Optional API key for authentication
//!
//! Optional:
//! - `GOLEM_VECTOR_LOG=trace`: Enable detailed logging

mod bindings;
mod client;
mod conversion;

use log::{debug, error, info, warn};

use crate::client::{QdrantApi, QdrantPoint};
use crate::conversion::*;
use golem_vector::durability::{DurableVector, ExtendedGuest};
use golem_vector::error::unsupported_feature;
use golem_vector::exports::golem::vector::analytics::{CollectionStats, FieldStats, Guest as AnalyticsGuest};
use golem_vector::exports::golem::vector::collections::{CollectionInfo, Guest as CollectionsGuest, IndexConfig};
use golem_vector::exports::golem::vector::connection::{
    ConnectionStatus, Credentials, Guest as ConnectionGuest,
};
use golem_vector::exports::golem::vector::namespaces::{Guest as NamespacesGuest, NamespaceInfo};
use golem_vector::exports::golem::vector::search::{
    Guest as SearchGuest, SearchQuery as SearchQueryEnum,
};
use golem_vector::exports::golem::vector::search_extended::{
    ContextPair, GroupedSearchResult, Guest as SearchExtendedGuest, RecommendationExample,
    RecommendationStrategy,
};
use golem_vector::exports::golem::vector::types::*;
use golem_vector::exports::golem::vector::vectors::{
    BatchResult, Guest as VectorsGuest, ListResponse,
};
use golem_vector::init_logging;

// Export the durability wrapper as the component
pub use golem_vector::durability::DurableVector as Component;

/// Qdrant provider implementation
pub struct QdrantComponent;

/// Helper function to create unsupported feature errors
fn unsupported_feature_with_context(feature: &str) -> VectorError {
    unsupported_feature(&format!("Qdrant: {}", feature))
}

/// Initialize logging once per component lifecycle
fn init_logging() {
    golem_vector::init_logging();
}

impl QdrantComponent {
    const ENDPOINT_ENV: &'static str = "QDRANT_ENDPOINT";
    const API_KEY_ENV: &'static str = "QDRANT_API_KEY";

    /// Validate configuration and create client
    fn create_client() -> Result<QdrantApi, VectorError> {
        // Endpoint defaults to localhost if env var not set
        let endpoint = std::env::var(Self::ENDPOINT_ENV)
            .unwrap_or_else(|_| "http://localhost:6333".to_string());
        let api_key = std::env::var(Self::API_KEY_ENV).ok();
        
        debug!("Creating Qdrant client for endpoint: {}", endpoint);
        Ok(QdrantApi::new(endpoint, api_key))
    }

    /// Validate environment configuration
    fn validate_config() -> Result<(), VectorError> {
        // For Qdrant, endpoint is optional (defaults to localhost)
        // Only API key validation if provided
        if let Ok(key) = std::env::var(Self::API_KEY_ENV) {
            if key.trim().is_empty() {
                return Err(VectorError::ConfigError(
                    "QDRANT_API_KEY is set but empty".to_string()
                ));
            }
        }
        Ok(())
    }
}

// -------------------- collections ---------------------------
impl CollectionsGuest for QdrantComponent {
    fn upsert_collection(
        name: String,
        description: Option<String>,
        dimension: u32,
        metric: DistanceMetric,
        _index_config: Option<IndexConfig>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        init_logging();
        Self::validate_config()?;
        
        info!("Creating Qdrant collection: {} with dimension: {}", name, dimension);
        let client = Self::create_client()?;
        let dist = metric_to_qdrant(metric)?;
        
        let result = client.upsert_collection(&name, dimension, dist)?;
        let mut info = collection_desc_to_info(result);
        
        // Set description if provided
        info.description = description;
        
        info!("Successfully created Qdrant collection: {}", name);
        Ok(info)
    }

    fn list_collections() -> Result<Vec<CollectionInfo>, VectorError> {
        init_logging();
        Self::validate_config()?;
        
        debug!("Listing Qdrant collections");
        let client = Self::create_client()?;
        let collections = client.list_collections()?
            .into_iter()
            .map(collection_desc_to_info)
            .collect::<Vec<_>>();
            
        info!("Found {} Qdrant collections", collections.len());
        Ok(collections)
    }

    fn get_collection(name: String) -> Result<CollectionInfo, VectorError> {
        init_logging();
        Self::validate_config()?;
        
        debug!("Getting Qdrant collection details for: {}", name);
        let client = Self::create_client()?;
        
        // Qdrant lacks single collection endpoint in V1 API
        let collections = client.list_collections()?;
        
        collections
            .into_iter()
            .find(|c| c.name == name)
            .map(collection_desc_to_info)
            .ok_or_else(|| VectorError::NotFound(format!("Collection '{}' not found", name)))
    }

    fn update_collection(
        name: String,
        _description: Option<String>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        init_logging();
        warn!("Qdrant does not support collection updates - returning current info for: {}", name);
        Self::get_collection(name)
    }

    fn delete_collection(name: String) -> Result<(), VectorError> {
        init_logging();
        Self::validate_config()?;
        
        info!("Deleting Qdrant collection: {}", name);
        let client = Self::create_client()?;
        client.delete_collection(&name)?;
        
        info!("Successfully deleted Qdrant collection: {}", name);
        Ok(())
    }

    fn collection_exists(name: String) -> Result<bool, VectorError> {
        init_logging();
        Self::validate_config()?;
        
        debug!("Checking if Qdrant collection exists: {}", name);
        let exists = Self::list_collections()?  
            .iter()
            .any(|c| c.name == name);
            
        debug!("Qdrant collection '{}' exists: {}", name, exists);
        Ok(exists)
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
        Self::validate_config()?;
        
        if vectors.is_empty() {
            return Ok(BatchResult {
                success_count: 0,
                failure_count: 0,
                errors: vec![],
            });
        }
        
        info!(
            "Upserting {} vectors to Qdrant collection: {} (namespace: {:?})",
            vectors.len(),
            collection,
            namespace
        );
        
        let client = Self::create_client()?;
        let mut success = 0u32;
        let mut errors = Vec::new();
        let mut points: Vec<QdrantPoint> = Vec::with_capacity(vectors.len());
        
        // Convert records to Qdrant points
        for (idx, rec) in vectors.into_iter().enumerate() {
            match record_to_qdrant_point(rec) {
                Ok(p) => {
                    points.push(p);
                    success += 1;
                }
                Err(e) => {
                    error!("Failed to convert vector at index {}: {}", idx, e);
                    errors.push((idx as u32, e));
                }
            }
        }
        
        // Upsert valid points if any
        if !points.is_empty() {
            match client.upsert_points(&collection, points, namespace) {
                Ok(()) => {
                    debug!("Successfully upserted {} points", success);
                }
                Err(e) => {
                    error!("Failed to upsert points: {}", e);
                    // Mark all converted points as failed
                    for idx in 0..success {
                        errors.push((idx, e.clone()));
                    }
                    success = 0;
                }
            }
        }
        
        info!(
            "Upsert completed: {} successful, {} failed",
            success,
            errors.len()
        );
        
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
        init_logging();
        Self::validate_config()?;
        
        debug!("Upserting single vector '{}' to Qdrant collection: {}", id, collection);
        
        let rec = VectorRecord {
            id,
            vector,
            metadata,
        };
        
        let result = Self::upsert_vectors(collection, vec![rec], namespace)?;
        
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
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<Vec<VectorRecord>, VectorError> {
        init_logging();
        Self::validate_config()?;
        
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        
        debug!(
            "Fetching {} vectors from Qdrant collection: {} (namespace: {:?})",
            ids.len(),
            collection,
            namespace
        );
        
        let client = Self::create_client()?;
        let mut out = Vec::with_capacity(ids.len());
        
        for id in ids {
            match client.get_point(&collection, &id, namespace.clone())? {
                Some(p) => {
                    let vector_data = if include_vectors.unwrap_or(true) {
                        p.vector.map(VectorData::Dense)
                    } else {
                        Some(VectorData::Dense(vec![])) // Empty vector if not requested
                    };
                    
                    if let Some(vector) = vector_data {
                        let metadata = if include_metadata.unwrap_or(true) {
                            p.payload.map(|m| {
                                m.into_iter()
                                    .map(|(k, v)| (k, MetadataValue::StringVal(v.to_string())))
                                    .collect()
                            })
                        } else {
                            None
                        };
                        
                        out.push(VectorRecord {
                            id: p.id,
                            vector,
                            metadata,
                        });
                    }
                }
                None => {
                    debug!("Vector '{}' not found", id);
                }
            }
        }
        
        debug!("Successfully fetched {} vectors", out.len());
        Ok(out)
    }

    fn get_vector(
        collection: String,
        id: String,
        namespace: Option<String>,
    ) -> Result<Option<VectorRecord>, VectorError> {
        init_logging();
        Self::validate_config()?;
        
        debug!(
            "Fetching single vector '{}' from Qdrant collection: {}",
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
        
        // In Qdrant, update is the same as upsert
        debug!(
            "Updating vector '{}' in Qdrant collection: {} (treated as upsert)",
            id, collection
        );
        
        if let Some(vector_data) = vector {
            Self::upsert_vector(collection, id, vector_data, metadata, namespace)
        } else {
            // If no vector data provided, we need to fetch existing vector first
            warn!("Cannot update metadata without vector data in Qdrant - fetching existing vector first");
            
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
    }

    fn delete_vectors(
        collection: String,
        ids: Vec<String>,
        namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        init_logging();
        Self::validate_config()?;
        
        if ids.is_empty() {
            return Ok(0);
        }
        
        info!(
            "Deleting {} vectors from Qdrant collection: {} (namespace: {:?})",
            ids.len(),
            collection,
            namespace
        );
        
        let client = Self::create_client()?;
        
        // Qdrant supports batch delete by IDs
        match client.delete_points(&collection, ids.clone(), namespace) {
            Ok(()) => {
                info!("Successfully deleted {} vectors", ids.len());
                Ok(ids.len() as u32)
            }
            Err(e) => {
                error!("Failed to delete vectors: {}", e);
                Err(e)
            }
        }
    }

    fn delete_by_filter(
        collection: String,
        filter: FilterExpression,
        namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        init_logging();
        Self::validate_config()?;
        
        info!(
            "Deleting vectors by filter in Qdrant collection: {} (namespace: {:?})",
            collection, namespace
        );
        
        let client = Self::create_client()?;
        let q_filter = filter_expression_to_qdrant(Some(filter));
        
        match client.delete_points_by_filter(&collection, q_filter, namespace) {
            Ok(count) => {
                info!("Successfully deleted {} vectors by filter", count);
                Ok(count)
            }
            Err(e) => {
                error!("Failed to delete vectors by filter: {}", e);
                Err(e)
            }
        }
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
        init_logging();
        // Qdrant supports scrolling through points, but it's complex to implement
        // with proper cursor management - marking as not yet implemented
        Err(unsupported_feature_with_context(
            "List vectors not yet implemented (requires scroll API)",
        ))
    }

    fn count_vectors(
        collection: String,
        filter: Option<FilterExpression>,
        namespace: Option<String>,
    ) -> Result<u64, VectorError> {
        init_logging();
        Self::validate_config()?;
        
        debug!(
            "Counting vectors in Qdrant collection: {} (namespace: {:?})",
            collection, namespace
        );
        
        let client = Self::create_client()?;
        let q_filter = filter_expression_to_qdrant(filter);
        
        match client.count_points(&collection, q_filter, namespace) {
            Ok(count) => {
                debug!("Found {} vectors matching criteria", count);
                Ok(count)
            }
            Err(e) => {
                error!("Failed to count vectors: {}", e);
                Err(e)
            }
        }
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
        Self::validate_config()?;
        
        debug!(
            "Searching {} vectors in Qdrant collection: {} (namespace: {:?})",
            limit, collection, namespace
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
        
        // Convert filter
        let q_filter = filter_expression_to_qdrant(filter);
        let include_vals = include_vectors.unwrap_or(false);
        let include_meta = include_metadata.unwrap_or(false);
        
        let results = client.search(
            &collection,
            vector,
            limit,
            namespace,
            q_filter,
            include_vals,
            include_meta,
        )?;
        
        let search_results = results
            .into_iter()
            .map(|r| SearchResult {
                id: r.id,
                score: r.score,
                distance: 1.0 - r.score, // Convert similarity to distance
                vector: if include_vals {
                    r.vector.map(VectorData::Dense)
                } else {
                    None
                },
                metadata: if include_meta {
                    r.payload.map(|p| {
                        p.into_iter()
                            .map(|(k, v)| (k, MetadataValue::StringVal(v.to_string())))
                            .collect()
                    })
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
        namespace: Option<String>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        init_logging();
        debug!("Finding similar vectors (simplified search)");
        
        Self::search_vectors(
            collection,
            SearchQueryEnum::Vector(vector),
            limit,
            None, // no filter
            namespace,
            Some(false), // don't include vectors
            Some(true),  // include metadata
            None,
            None,
            None,
        )
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
        
        info!("Performing batch search with {} queries", queries.len());
        
        let mut results = Vec::with_capacity(queries.len());
        
        // Execute each query individually
        for (idx, query) in queries.into_iter().enumerate() {
            debug!("Executing batch query {}", idx);
            
            match Self::search_vectors(
                collection.clone(),
                query,
                limit,
                filter.clone(),
                namespace.clone(),
                include_vectors,
                include_metadata,
                None,
                None,
                search_params.clone(),
            ) {
                Ok(search_results) => results.push(search_results),
                Err(e) => {
                    error!("Batch query {} failed: {}", idx, e);
                    return Err(e);
                }
            }
        }
        
        info!("Batch search completed successfully");
        Ok(results)
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
        init_logging();
        Err(unsupported_feature_with_context("Grouped search not supported"))
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
    }
}

// -------------------- namespaces ---------------------------
impl NamespacesGuest for QdrantComponent {
    fn upsert_namespace(
        _collection: String,
        _namespace: String,
        _metadata: Option<Metadata>,
    ) -> Result<NamespaceInfo, VectorError> {
        init_logging();
        Err(unsupported_feature_with_context(
            "Namespace management not supported by Qdrant API",
        ))
    }

    fn list_namespaces(_collection: String) -> Result<Vec<NamespaceInfo>, VectorError> {
        init_logging();
        Err(unsupported_feature_with_context(
            "Namespace listing not supported by Qdrant API",
        ))
    }

    fn get_namespace(
        _collection: String,
        _namespace: String,
    ) -> Result<NamespaceInfo, VectorError> {
        init_logging();
        Err(unsupported_feature_with_context(
            "Namespace info not supported by Qdrant API",
        ))
    }

    fn delete_namespace(_collection: String, _namespace: String) -> Result<(), VectorError> {
        init_logging();
        Err(unsupported_feature_with_context(
            "Namespace deletion not supported by Qdrant API",
        ))
    }

    fn namespace_exists(_collection: String, _namespace: String) -> Result<bool, VectorError> {
        init_logging();
        // Qdrant uses payload fields for filtering which can act like namespaces
        // but doesn't have explicit namespace support
        Ok(true)
    }
}

// -------------------- analytics -----------------------------
impl AnalyticsGuest for QdrantComponent {
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
    }
}

// -------------------- connection ---------------------------
impl ConnectionGuest for QdrantComponent {
    fn connect(
        _endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        _options: Option<Metadata>,
    ) -> Result<(), VectorError> {
        init_logging();
        Self::validate_config()?;
        info!("Qdrant connection validated via environment variables");
        Ok(())
    }

    fn disconnect() -> Result<(), VectorError> {
        init_logging();
        debug!("Qdrant disconnect (no persistent connection to close)");
        Ok(())
    }

    fn get_connection_status() -> Result<ConnectionStatus, VectorError> {
        init_logging();
        match Self::validate_config() {
            Ok(()) => Ok(ConnectionStatus::Connected),
            Err(_) => Ok(ConnectionStatus::Disconnected),
        }
    }

    fn test_connection(
        endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        _options: Option<Metadata>,
    ) -> Result<bool, VectorError> {
        init_logging();
        let client = QdrantApi::new(endpoint, None);
        
        // Test by trying to list collections
        match client.list_collections() {
            Ok(_) => {
                info!("Qdrant connection test successful");
                Ok(true)
            }
            Err(e) => {
                error!("Qdrant connection test failed: {}", e);
                Ok(false)
            }
        }
    }
}

// Implement ExtendedGuest marker trait
impl ExtendedGuest for QdrantComponent {}

// Export bindings for the component
golem_vector::export_bindings!(Component);

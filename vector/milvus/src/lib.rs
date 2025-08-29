//! Milvus vector database provider for Golem.
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

mod bindings;
mod client;
mod conversion;

use log::{debug, error, info, warn};

use crate::client::MilvusClient;
use crate::conversion::*;
use golem_vector::durability::DurableVector;
use golem_vector::durability::ExtendedGuest;
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
use golem_vector::exports::golem::vector::types::VectorError;
use golem_vector::exports::golem::vector::types::*;
use golem_vector::exports::golem::vector::vectors::{
    BatchResult, Guest as VectorsGuest, ListResponse, VectorRecord,
};
use golem_vector::init_logging as golem_init_logging;

// Export the durability wrapper as the concrete component type
pub type Component = DurableVector<MilvusComponent>;

/// Milvus provider implementation
pub struct MilvusComponent;

/// Helper function to create unsupported feature errors
fn unsupported_feature_with_context(feature: &str) -> VectorError {
    unsupported_feature(format!("Milvus: {feature}"))
}

/// Initialize logging once per component lifecycle
fn init_logging() {
    golem_init_logging();
}

impl MilvusComponent {
    const ENDPOINT_ENV: &'static str = "MILVUS_ENDPOINT";
    const API_KEY_ENV: &'static str = "MILVUS_API_KEY";

    /// Validate configuration and create client
    fn create_client() -> Result<MilvusClient, VectorError> {
        let endpoint =
            std::env::var(Self::ENDPOINT_ENV).unwrap_or_else(|_| "http://localhost:19530".into());
        let api_key = std::env::var(Self::API_KEY_ENV).ok();

        debug!("Creating Milvus client for endpoint: {endpoint}");
        Ok(MilvusClient::new(endpoint, api_key))
    }

    /// Validate environment configuration
    fn validate_config() -> Result<(), VectorError> {
        // For Milvus, endpoint is optional (defaults to localhost)
        // Only API key validation if provided
        if let Ok(key) = std::env::var(Self::API_KEY_ENV) {
            if key.trim().is_empty() {
                return Err(VectorError::InvalidParams(
                    "MILVUS_API_KEY is set but empty".to_string(),
                ));
            }
        }
        Ok(())
    }
}

// -------------------- collections ---------------------------
impl CollectionsGuest for MilvusComponent {
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

        info!(
            "Creating Milvus collection: {name} with dimension: {dimension}"
        );
        let client = Self::create_client()?;
        client.create_collection(&name, dimension, metric)?;

        info!("Successfully created Milvus collection: {name}");
        Ok(CollectionInfo {
            name,
            description,
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
        Self::validate_config()?;

        debug!("Listing Milvus collections");
        let client = Self::create_client()?;
        let collections = client
            .list_collections()?
            .into_iter()
            .map(|name| {
                debug!("Found collection: {name}");
                match client.describe_collection(&name) {
                    Ok((dimension, metric, count)) => {
                        let size_bytes = Some((dimension as u64) * (count as u64) * 4);
                        CollectionInfo {
                            name,
                            description: None,
                            dimension,
                            metric,
                            vector_count: count,
                            size_bytes,
                            index_ready: true,
                            created_at: None,
                            updated_at: None,
                            provider_stats: None,
                        }
                    }
                    Err(e) => {
                        warn!("Failed to describe Milvus collection {}: {}", name, e);
                        CollectionInfo {
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
                        }
                    }
                }
            })
            .collect::<Vec<_>>();

        info!("Found {} Milvus collections", collections.len());
        Ok(collections)
    }

    fn delete_collection(name: String) -> Result<(), VectorError> {
        init_logging();
        Self::validate_config()?;

        info!("Deleting Milvus collection: {name}");
        let client = Self::create_client()?;
        client.delete_collection(&name)?;

        info!("Successfully deleted Milvus collection: {name}");
        Ok(())
    }

    fn get_collection(name: String) -> Result<CollectionInfo, VectorError> {
        init_logging();
        Self::validate_config()?;

        debug!("Getting Milvus collection details for: {name}");
        let collections = Self::list_collections()?;

        collections
            .into_iter()
            .find(|c| c.name == name)
            .ok_or_else(|| VectorError::NotFound(format!("Collection '{name}' not found")))
    }

    fn update_collection(
        name: String,
        _description: Option<String>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        init_logging();
        warn!(
            "Milvus does not support collection updates - returning current info for: {name}"
        );
        Self::get_collection(name)
    }

    fn collection_exists(name: String) -> Result<bool, VectorError> {
        init_logging();
        Self::validate_config()?;

        debug!("Checking if Milvus collection exists: {name}");
        let exists = Self::list_collections()?.iter().any(|c| c.name == name);

        debug!("Milvus collection '{name}' exists: {exists}");
        Ok(exists)
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
                error!("Failed to upsert vectors: {e}");
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

        debug!(
            "Upserting single vector '{id}' to Milvus collection: {collection}"
        );

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
            "Fetching single vector '{id}' from Milvus collection: {collection}"
        );

        let results = Self::get_vectors(collection, vec![id], namespace, Some(true), Some(true))?;

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
            "Updating vector '{id}' in Milvus collection: {collection} (treated as upsert)"
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
                    "Vector '{id}' not found for update"
                ))),
            }
        }
    }

    fn delete_vectors(
        collection: String,
        ids: Vec<String>,
        _namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        init_logging();
        Self::validate_config()?;

        if ids.is_empty() {
            return Ok(0);
        }
        info!(
            "Deleting {} vectors from Milvus collection: {}",
            ids.len(),
            collection
        );

        let client = Self::create_client()?;
        match client.delete_vectors(&collection, ids) {
            Ok(count) => {
                info!("Successfully deleted {count} vectors");
                Ok(count)
            }
            Err(e) => {
                error!("Failed to delete vectors: {e}");
                Err(e)
            }
        }
    }

    fn delete_by_filter(
        collection: String,
        filter: FilterExpression,
        _namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        init_logging();
        Self::validate_config()?;

        let expr = filter_expression_to_milvus(Some(filter)).ok_or_else(|| {
            VectorError::InvalidParams("Unsupported or empty filter expression".into())
        })?;

        let client = Self::create_client()?;
        // Milvus REST allows maximum limit 16384 per query; iterate until all IDs deleted
        let mut offset: u32 = 0;
        let limit: u32 = 16384;
        let mut total_deleted: u32 = 0;
        loop {
            let ids = client.query_ids(&collection, Some(expr.clone()), limit, offset)?;
            if ids.is_empty() {
                break;
            }
            total_deleted += client.delete_vectors(&collection, ids.clone())? as u32;
            if ids.len() < limit as usize {
                break;
            }
            offset += limit;
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
        init_logging();
        Self::validate_config()?;

        let lim = limit.unwrap_or(100).min(16384);
        let offset = cursor
            .as_deref()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        let expr = filter_expression_to_milvus(filter);
        let client = Self::create_client()?;
        let ids = client.query_ids(&collection, expr, lim, offset)?;

        // Fetch records respecting include options
        let records = Self::get_vectors(
            collection.clone(),
            ids.clone(),
            None,
            include_vectors,
            include_metadata,
        )?;

        let next_cursor = if (ids.len() as u32) == lim {
            Some((offset + lim).to_string())
        } else {
            None
        };

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
        init_logging();
        Self::validate_config()?;
        let expr = filter_expression_to_milvus(filter);
        let client = Self::create_client()?;
        client.count_vectors(&collection, expr)
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
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
        _min_score: Option<f32>,
        _max_distance: Option<f32>,
        _search_params: Option<Vec<(String, String)>>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        init_logging();
        Self::validate_config()?;

        debug!(
            "Searching {limit} vectors in Milvus collection: {collection}"
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

        let search_results: Vec<SearchResult> = results
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
            None,        // no filter
            None,        // no namespace
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
        Err(unsupported_feature_with_context(
            "Recommendations not supported",
        ))
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
        Err(unsupported_feature_with_context(
            "Grouped search not supported",
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
        init_logging();
        Err(unsupported_feature_with_context(
            "Range search not supported",
        ))
    }

    fn search_text(
        _collection: String,
        _query_text: String,
        _limit: u32,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        init_logging();
        Err(unsupported_feature_with_context(
            "Text search not supported",
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
    }

    fn get_namespace(
        _collection: String,
        _namespace: String,
    ) -> Result<NamespaceInfo, VectorError> {
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
    }
}

// -------------------- connection ---------------------------
impl ConnectionGuest for MilvusComponent {
    fn connect(
        _endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        _options: Option<Metadata>,
    ) -> Result<(), VectorError> {
        init_logging();
        Self::validate_config()?;
        info!("Milvus connection validated via environment variables");
        Ok(())
    }

    fn disconnect() -> Result<(), VectorError> {
        init_logging();
        debug!("Milvus disconnect (no persistent connection to close)");
        Ok(())
    }

    fn get_connection_status() -> Result<ConnectionStatus, VectorError> {
        init_logging();
        let connected = Self::validate_config().is_ok();
        Ok(ConnectionStatus {
            connected,
            provider: Some("Milvus".to_string()),
            endpoint: std::env::var(Self::ENDPOINT_ENV).ok(),
            last_activity: None,
            connection_id: None,
        })
    }

    fn test_connection(
        endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        _options: Option<Metadata>,
    ) -> Result<bool, VectorError> {
        init_logging();
        let client = MilvusClient::new(endpoint, None);

        // Test by trying to list collections
        match client.list_collections() {
            Ok(_) => {
                info!("Milvus connection test successful");
                Ok(true)
            }
            Err(e) => {
                error!("Milvus connection test failed: {e}");
                Ok(false)
            }
        }
    }
}

// Implement ExtendedGuest marker trait
impl ExtendedGuest for MilvusComponent {}

// Export bindings for the component
golem_vector::export_vector!(Component with_types_in golem_vector);

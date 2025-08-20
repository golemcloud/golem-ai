<<<<<<< HEAD
<<<<<<< HEAD
//! Production-ready Pinecone vector database provider for Golem.
//!
//! This provider implements the full `golem:vector` WIT interface for Pinecone,
//! supporting:
//! - Index/collection management (create, list, delete, exists)
//! - Vector operations (upsert, get, update with limitations)
//! - Similarity search with filtering
//! - Namespace support
//! - Comprehensive error handling and logging
//!
//! ## Configuration
//!
//! Required environment variables:
//! - `PINECONE_CONTROLLER_ENDPOINT`: Controller API endpoint (e.g., https://controller.us-east1-gcp.pinecone.io)
//! - `PINECONE_INDEX_HOST`: Index host URL (e.g., https://my-index-abc123.svc.us-east1-gcp.pinecone.io)
//! - `PINECONE_API_KEY`: Pinecone API key
//!
//! Optional:
//! - `GOLEM_VECTOR_LOG=trace`: Enable detailed logging

use log::{debug, error, info, warn};
use std::env;

// Golem vector interface exports
use golem_vector::exports::golem::vector::analytics::Guest as AnalyticsGuest;
use golem_vector::exports::golem::vector::collections::{
    CollectionInfo, Guest as CollectionsGuest, IndexConfig,
};
use golem_vector::exports::golem::vector::connection::{
    ConnectionStatus, Credentials, Guest as ConnectionGuest,
};
use golem_vector::exports::golem::vector::namespaces::{Guest as NamespacesGuest, NamespaceInfo};
use golem_vector::exports::golem::vector::search::Guest as SearchGuest;
use golem_vector::exports::golem::vector::search_extended::{
    ContextPair, GroupedSearchResult, Guest as SearchExtendedGuest, RecommendationExample,
    RecommendationStrategy,
};
use golem_vector::exports::golem::vector::types::{
    DistanceMetric, FilterExpression, Metadata, MetadataValue, SearchQuery as SearchQueryEnum,
    SearchResult, VectorData, VectorError, VectorRecord,
};
use golem_vector::exports::golem::vector::vectors::{
    BatchResult, Guest as VectorsGuest, ListResponse,
};
use golem_vector::exports::golem::vector::analytics::{CollectionStats, FieldStats};

// Internal modules
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
//! Pinecone vector provider component (stub)
//!
//! This crate wires into the Golem Vector WIT interfaces but currently
//! contains only **placeholder** implementations that always return
//! `unsupported_feature` errors.  The goal is to keep the workspace
//! compiling until real Pinecone support is implemented.

<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
mod bindings;
mod client;
mod conversion;

<<<<<<< HEAD
<<<<<<< HEAD
use client::{PineconeApi, QueryMatch};
use conversion::{
    filter_expression_to_pinecone, metadata_to_json_map, metric_to_pinecone, vector_data_to_dense,
};

// Export the durability wrapper as the component
pub use golem_vector::durability::DurableVector as Component;
pub use golem_vector::durability::ExtendedGuest;

/// Pinecone provider implementation
pub struct PineconeComponent;

/// Helper function to create unsupported feature errors
fn unsupported_feature(feature: &str) -> VectorError {
    VectorError::NotSupported(format!("Pinecone: {}", feature))
}

/// Initialize logging once per component lifecycle
fn init_logging() {
    golem_vector::init_logging();
}

/// Convert JSON map to metadata (inverse of metadata_to_json_map)
fn json_map_to_metadata(
    map: std::collections::HashMap<String, serde_json::Value>,
) -> Option<Metadata> {
    if map.is_empty() {
        None
    } else {
        Some(
            map.into_iter()
                .filter_map(|(k, v)| {
                    let meta_val = match v {
                        serde_json::Value::String(s) => MetadataValue::StringVal(s),
                        serde_json::Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                MetadataValue::IntegerVal(i)
                            } else if let Some(f) = n.as_f64() {
                                MetadataValue::NumberVal(f)
                            } else {
                                return None;
                            }
                        }
                        serde_json::Value::Bool(b) => MetadataValue::BooleanVal(b),
                        serde_json::Value::Null => MetadataValue::NullVal,
                        _ => return None, // Skip complex types for now
                    };
                    Some((k, meta_val))
                })
                .collect(),
        )
    }
}

impl PineconeComponent {
    /// Validate required environment variables
    fn validate_config() -> Result<(), VectorError> {
        let missing_vars = ["PINECONE_CONTROLLER_ENDPOINT", "PINECONE_INDEX_HOST", "PINECONE_API_KEY"]
            .iter()
            .filter(|&var| env::var(var).is_err())
            .collect::<Vec<_>>();

        if !missing_vars.is_empty() {
            return Err(VectorError::ConfigError(format!(
                "Missing required environment variables: {}",
                missing_vars
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        Ok(())
    }

    /// Create controller API client
    fn controller_client() -> PineconeApi {
        let endpoint = env::var("PINECONE_CONTROLLER_ENDPOINT")
            .expect("PINECONE_CONTROLLER_ENDPOINT must be set");
        let api_key = env::var("PINECONE_API_KEY").ok();
        PineconeApi::new(endpoint, api_key)
    }

    /// Create index API client with host
    fn index_client() -> PineconeApi {
        let host = env::var("PINECONE_INDEX_HOST").expect("PINECONE_INDEX_HOST must be set");
        let api_key = env::var("PINECONE_API_KEY").ok();
        PineconeApi::new(host, api_key)
    }
}
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
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

struct PineconeComponent;
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da

// -------------------- collections ---------------------------
impl CollectionsGuest for PineconeComponent {
    fn upsert_collection(
<<<<<<< HEAD
<<<<<<< HEAD
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
            "Creating Pinecone index: {} with dimension: {}",
            name, dimension
        );
        let api = Self::controller_client();
        let metric_str = metric_to_pinecone(metric);

        api.create_index(&name, dimension, metric_str)?;

        info!("Successfully created Pinecone index: {}", name);
        Ok(CollectionInfo {
            name,
            description,
            dimension,
            metric,
            vector_count: 0,
            size_bytes: None,
            index_ready: true, // Pinecone indexes are ready immediately after creation
            created_at: None,  // Would need additional API call to get creation time
            updated_at: None,
            provider_stats: None,
        })
    }

    fn list_collections() -> Result<Vec<CollectionInfo>, VectorError> {
        init_logging();
        Self::validate_config()?;

        debug!("Listing Pinecone indexes");
        let api = Self::controller_client();
        let index_names = api.list_indexes()?;

        let collections = index_names
            .into_iter()
            .map(|name| {
                debug!("Found index: {}", name);
                CollectionInfo {
                    name,
                    description: None,
                    dimension: 0, // Would need describe_index call to get actual dimension
                    metric: DistanceMetric::Cosine, // Default, would need describe_index for actual
                    vector_count: 0, // Would need stats call to get actual count
                    size_bytes: None,
                    index_ready: true,
                    created_at: None,
                    updated_at: None,
                    provider_stats: None,
                }
            })
            .collect();

        info!("Found {} Pinecone indexes", collections.len());
        Ok(collections)
    }

    fn get_collection(name: String) -> Result<CollectionInfo, VectorError> {
        init_logging();
        Self::validate_config()?;

        debug!("Getting Pinecone index details for: {}", name);
        let collections = Self::list_collections()?;

        collections
            .into_iter()
            .find(|c| c.name == name)
            .ok_or_else(|| VectorError::NotFound(format!("Index '{}' not found", name)))
    }

    fn update_collection(
        name: String,
        _description: Option<String>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        init_logging();
        warn!(
            "Pinecone does not support index updates - returning current info for: {}",
            name
        );
        Self::get_collection(name)
    }

    fn delete_collection(name: String) -> Result<(), VectorError> {
        init_logging();
        Self::validate_config()?;

        info!("Deleting Pinecone index: {}", name);
        let api = Self::controller_client();
        api.delete_index(&name)?;

        info!("Successfully deleted Pinecone index: {}", name);
        Ok(())
    }

    fn collection_exists(name: String) -> Result<bool, VectorError> {
        init_logging();
        Self::validate_config()?;

        debug!("Checking if Pinecone index exists: {}", name);
        let exists = Self::list_collections()
            .map(|collections| collections.iter().any(|c| c.name == name))?;

        debug!("Pinecone index '{}' exists: {}", name, exists);
        Ok(exists)
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        _name: String,
        _description: Option<String>,
        _dimension: u32,
        _metric: DistanceMetric,
        _index_config: Option<IndexConfig>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        Err(unsupported_feature("Pinecone provider not implemented"))
    }

    fn list_collections() -> Result<Vec<CollectionInfo>, VectorError> {
        Err(unsupported_feature("Pinecone provider not implemented"))
    }

    fn get_collection(_name: String) -> Result<CollectionInfo, VectorError> {
        Err(unsupported_feature("Pinecone provider not implemented"))
    }

    fn update_collection(
        _name: String,
        _description: Option<String>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        Err(unsupported_feature("Pinecone provider not implemented"))
    }

    fn delete_collection(_name: String) -> Result<(), VectorError> {
        Err(unsupported_feature("Pinecone provider not implemented"))
    }

    fn collection_exists(_name: String) -> Result<bool, VectorError> {
        Err(unsupported_feature("Pinecone provider not implemented"))
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
    }
}

// -------------------- vectors -------------------------------
impl VectorsGuest for PineconeComponent {
    fn upsert_vectors(
<<<<<<< HEAD
<<<<<<< HEAD
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
            "Upserting {} vectors to Pinecone index: {} (namespace: {:?})",
            vectors.len(),
            collection,
            namespace
        );

        let api = Self::index_client();

        // Pinecone has batch size limits - handle in chunks if needed
        const MAX_BATCH_SIZE: usize = 1000;
        let mut total_success = 0u32;
        let mut total_errors = Vec::new();

        for (chunk_idx, chunk) in vectors.chunks(MAX_BATCH_SIZE).enumerate() {
            debug!(
                "Processing batch {} with {} vectors",
                chunk_idx,
                chunk.len()
            );

            match api.upsert_vectors(chunk.to_vec(), namespace.clone()) {
                Ok(()) => {
                    total_success += chunk.len() as u32;
                    debug!(
                        "Successfully upserted batch {} ({} vectors)",
                        chunk_idx,
                        chunk.len()
                        chunk.id
                    );
                }
                Err(e) => {
                    error!("Failed to upsert batch {}: {}", chunk_idx, e);
                    for (idx, _) in chunk.iter().enumerate() {
                        total_errors.push((chunk_idx * MAX_BATCH_SIZE + idx, e.clone()));
                    }
                }
            }
        }

        info!(
            "Upsert completed: {} successful, {} failed",
            total_success,
            total_errors.len()
        );

        Ok(BatchResult {
            success_count: total_success,
            failure_count: total_errors.len() as u32,
            errors: total_errors
                .into_iter()
                .map(|(idx, err)| (idx as u32, err))
                .collect(),
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

        debug!(
            "Upserting single vector '{}' to Pinecone index: {}",
            id, collection
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
            "Fetching {} vectors from Pinecone index: {} (namespace: {:?})",
            ids.len(),
            collection,
            namespace
        );

        let api = Self::index_client();
        let fetched = api.fetch_vectors(ids, namespace)?;

        let mut results = Vec::with_capacity(fetched.len());

        for fv in fetched {
            // Only include vectors if explicitly requested or not specified (default true)
            let vector_data = if include_vectors.unwrap_or(true) {
                fv.values.map(VectorData::Dense)
            } else {
                Some(VectorData::Dense(vec![])) // Empty vector if not requested
            };

            if let Some(vector) = vector_data {
                let metadata = if include_metadata.unwrap_or(true) {
                    fv.metadata.and_then(json_map_to_metadata)
                } else {
                    None
                };

                results.push(VectorRecord {
                    id: fv.id,
                    vector,
                    metadata,
                });
            }
        }

        debug!("Successfully fetched {} vectors", results.len());
        Ok(results)
    }

    fn get_vector(
        collection: String,
        id: String,
        namespace: Option<String>,
    ) -> Result<Option<VectorRecord>, VectorError> {
        init_logging();
        Self::validate_config()?;

        debug!(
            "Fetching single vector '{}' from Pinecone index: {}",
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

        // In Pinecone, update is the same as upsert
        debug!(
            "Updating vector '{}' in Pinecone index: {} (treated as upsert)",
            id, collection
        );

        if let Some(vector_data) = vector {
            Self::upsert_vector(collection, id, vector_data, metadata, namespace)
        } else {
            // If no vector data provided, we can only update metadata by fetching first
            warn!("Cannot update metadata without vector data in Pinecone - use upsert instead");
            Err(unsupported_feature(
                "Metadata-only updates not supported by Pinecone",
            ))
        }
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        _collection: String,
        _vectors: Vec<VectorRecord>,
        _namespace: Option<String>,
    ) -> Result<BatchResult, VectorError> {
        Err(unsupported_feature("Pinecone provider not implemented"))
    }

    fn upsert_vector(
        _collection: String,
        _id: String,
        _vector: VectorData,
        _metadata: Option<Metadata>,
        _namespace: Option<String>,
    ) -> Result<(), VectorError> {
        Err(unsupported_feature("Pinecone provider not implemented"))
    }

    fn get_vectors(
        _collection: String,
        _ids: Vec<String>,
        _namespace: Option<String>,
        _include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
    ) -> Result<Vec<VectorRecord>, VectorError> {
        Err(unsupported_feature("Pinecone provider not implemented"))
    }

    fn get_vector(
        _collection: String,
        _id: String,
        _namespace: Option<String>,
    ) -> Result<Option<VectorRecord>, VectorError> {
        Err(unsupported_feature("Pinecone provider not implemented"))
    }

    fn update_vector(
        _collection: String,
        _id: String,
        _vector: Option<VectorData>,
        _metadata: Option<Metadata>,
        _namespace: Option<String>,
        _merge_metadata: Option<bool>,
    ) -> Result<(), VectorError> {
        Err(unsupported_feature("Pinecone provider not implemented"))
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
        // Pinecone supports vector deletion but it's not implemented in our client yet
        Err(unsupported_feature(
            "Delete vectors not yet implemented for Pinecone",
        ))
=======
        Err(unsupported_feature("Pinecone provider not implemented"))
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
        Err(unsupported_feature("Pinecone provider not implemented"))
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
    }

    fn delete_by_filter(
        _collection: String,
        _filter: FilterExpression,
        _namespace: Option<String>,
    ) -> Result<u32, VectorError> {
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        // Pinecone supports filtering but delete by filter is complex
        Err(unsupported_feature(
            "Delete by filter not yet implemented for Pinecone",
        ))
=======
        Err(unsupported_feature("Pinecone provider not implemented"))
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
        Err(unsupported_feature("Pinecone provider not implemented"))
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
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
        // Pinecone doesn't have a direct list vectors endpoint
        Err(unsupported_feature(
            "List vectors not supported by Pinecone API",
        ))
=======
        Err(unsupported_feature("Pinecone provider not implemented"))
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
        Err(unsupported_feature("Pinecone provider not implemented"))
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
    }

    fn count_vectors(
        _collection: String,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
    ) -> Result<u64, VectorError> {
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        // Would need to use describe_index_stats for this
        Err(unsupported_feature(
            "Count vectors not yet implemented for Pinecone",
        ))
=======
        Err(unsupported_feature("Pinecone provider not implemented"))
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
        Err(unsupported_feature("Pinecone provider not implemented"))
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
    }
}

// -------------------- search --------------------------------
impl SearchGuest for PineconeComponent {
    fn search_vectors(
<<<<<<< HEAD
<<<<<<< HEAD
        collection: String,
        query: SearchQueryEnum,
        limit: u32,
        filter: Option<FilterExpression>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        _collection: String,
        _query: SearchQueryEnum,
        _limit: u32,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
        _include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        _min_score: Option<f32>,
        _max_distance: Option<f32>,
        _search_params: Option<Vec<(String, String)>>,
    ) -> Result<Vec<SearchResult>, VectorError> {
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        Self::validate_config()?;

        debug!(
            "Searching {} vectors in Pinecone index: {} (namespace: {:?})",
            limit, collection, namespace
        );

        let (api, host) = Self::index_client();

        // Convert query to vector
        let vector = match query {
            SearchQueryEnum::Vector(v) => vector_data_to_dense(v)?,
            _ => {
                return Err(unsupported_feature(
                    "Only vector queries supported for Pinecone",
                ))
            }
        };

        // Convert filter
        let filter_json = filter_expression_to_pinecone(filter)?;
        let include_values = include_vectors.unwrap_or(false);
        let include_meta = include_metadata.unwrap_or(false);

        let matches = api.query(
            vector,
            limit,
            namespace,
            filter_json,
            include_values,
            include_meta,
        )?;

        let results = matches
            .into_iter()
            .map(|m| SearchResult {
                id: m.id,
                score: m.score,
                distance: 1.0 - m.score, // Convert similarity to distance
                vector: if include_values {
                    m.values.map(VectorData::Dense)
                } else {
                    None
                },
                metadata: if include_meta {
                    m.metadata.and_then(json_map_to_metadata)
                } else {
                    None
                },
            })
            .collect();

        debug!("Found {} search results", results.len());
        Ok(results)
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
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        Err(unsupported_feature("Pinecone provider not implemented"))
    }

    fn find_similar(
        _collection: String,
        _vector: VectorData,
        _limit: u32,
        _namespace: Option<String>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        Err(unsupported_feature("Pinecone provider not implemented"))
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
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
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        Err(unsupported_feature(
            "Batch search not yet implemented for Pinecone",
        ))
    }
}

// -------------------- search-extended -----------------------
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        Err(unsupported_feature("Pinecone provider not implemented"))
    }
}

// -------------------- search extended ----------------------
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
impl SearchExtendedGuest for PineconeComponent {
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
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        Err(unsupported_feature("Recommendations not supported by Pinecone"))
=======
        Err(unsupported_feature("Pinecone provider not implemented"))
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
        Err(unsupported_feature("Pinecone provider not implemented"))
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
    }

    fn discover_vectors(
        _collection: String,
<<<<<<< HEAD
<<<<<<< HEAD
        _context_pairs: Vec<ContextPair>,
=======
        _context_pairs: Vec<golem_vector::exports::golem::vector::search_extended::ContextPair>,
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
        _context_pairs: Vec<golem_vector::exports::golem::vector::search_extended::ContextPair>,
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        _limit: u32,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
        _include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
    ) -> Result<Vec<SearchResult>, VectorError> {
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        Err(unsupported_feature("Discovery not supported by Pinecone"))
=======
        Err(unsupported_feature("Pinecone provider not implemented"))
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
        Err(unsupported_feature("Pinecone provider not implemented"))
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
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
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        Err(unsupported_feature("Grouped search not supported by Pinecone"))
=======
        Err(unsupported_feature("Pinecone provider not implemented"))
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
        Err(unsupported_feature("Pinecone provider not implemented"))
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
        Err(unsupported_feature("Range search not supported by Pinecone"))
=======
        Err(unsupported_feature("Pinecone provider not implemented"))
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
        Err(unsupported_feature("Pinecone provider not implemented"))
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
    }

    fn search_text(
        _collection: String,
        _query_text: String,
        _limit: u32,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
    ) -> Result<Vec<SearchResult>, VectorError> {
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        Err(unsupported_feature("Text search not supported by Pinecone"))
    }
}

// -------------------- namespaces ----------------------------
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        Err(unsupported_feature("Pinecone provider not implemented"))
    }
}

// -------------------- namespaces ---------------------------
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
impl NamespacesGuest for PineconeComponent {
    fn upsert_namespace(
        _collection: String,
        _namespace: String,
        _metadata: Option<Metadata>,
    ) -> Result<NamespaceInfo, VectorError> {
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        Err(unsupported_feature(
            "Namespace management not supported by Pinecone API",
        ))
    }

    fn list_namespaces(_collection: String) -> Result<Vec<NamespaceInfo>, VectorError> {
        init_logging();
        Err(unsupported_feature(
            "Namespace listing not supported by Pinecone API",
        ))
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        Err(unsupported_feature("Pinecone provider not implemented"))
    }

    fn list_namespaces(_collection: String) -> Result<Vec<NamespaceInfo>, VectorError> {
        Err(unsupported_feature("Pinecone provider not implemented"))
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
        Err(unsupported_feature(
            "Namespace info not supported by Pinecone API",
        ))
    }

    fn delete_namespace(_collection: String, _namespace: String) -> Result<(), VectorError> {
        init_logging();
        Err(unsupported_feature(
            "Namespace deletion not supported by Pinecone API",
        ))
    }

    fn namespace_exists(_collection: String, _namespace: String) -> Result<bool, VectorError> {
        init_logging();
        // Namespaces in Pinecone are implicit - they exist when vectors are added to them
        Ok(true)
    }
}

// -------------------- analytics -----------------------------
impl AnalyticsGuest for PineconeComponent {
    fn get_collection_stats(
        _collection: String,
        _namespace: Option<String>,
    ) -> Result<CollectionStats, VectorError> {
        init_logging();
        Err(unsupported_feature(
            "Collection stats not yet implemented for Pinecone",
        ))
    }

    fn get_field_stats(
        _collection: String,
        _field: String,
        _namespace: Option<String>,
    ) -> Result<FieldStats, VectorError> {
        init_logging();
        Err(unsupported_feature(
            "Field stats not yet implemented for Pinecone",
        ))
    }

    fn get_field_distribution(
        _collection: String,
        _field: String,
        _limit: Option<u32>,
        _namespace: Option<String>,
    ) -> Result<Vec<(MetadataValue, u64)>, VectorError> {
        init_logging();
        Err(unsupported_feature(
            "Field distribution not yet implemented for Pinecone",
        ))
    }
}

// -------------------- connection ----------------------------
impl ConnectionGuest for PineconeComponent {
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        Err(unsupported_feature("Pinecone provider not implemented"))
    }

    fn delete_namespace(_collection: String, _namespace: String) -> Result<(), VectorError> {
        Err(unsupported_feature("Pinecone provider not implemented"))
    }

    fn namespace_exists(_collection: String, _namespace: String) -> Result<bool, VectorError> {
        Err(unsupported_feature("Pinecone provider not implemented"))
    }
}

// -------------------- connection ---------------------------
impl ConnectionGuestImpl for PineconeComponent {
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
    fn connect(
        _endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        _options: Option<Metadata>,
    ) -> Result<(), VectorError> {
        init_logging();
<<<<<<< HEAD
<<<<<<< HEAD
        Self::validate_config()?;
        info!("Pinecone connection validated via environment variables");
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
        debug!("Pinecone disconnect (no persistent connection to close)");
=======
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        Ok(())
    }

    fn get_connection_status() -> Result<ConnectionStatus, VectorError> {
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        match Self::validate_config() {
            Ok(()) => Ok(ConnectionStatus::Connected),
            Err(_) => Ok(ConnectionStatus::Disconnected),
        }
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        Ok(ConnectionStatus {
            connected: false,
            provider: Some("pinecone".into()),
            endpoint: None,
            last_activity: None,
        })
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
    }

    fn test_connection(
        _endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        _options: Option<Metadata>,
    ) -> Result<bool, VectorError> {
<<<<<<< HEAD
<<<<<<< HEAD
        init_logging();
        // Test by trying to list indexes
        match Self::list_collections() {
            Ok(_) => {
                info!("Pinecone connection test successful");
                Ok(true)
            }
            Err(e) => {
                error!("Pinecone connection test failed: {}", e);
                Ok(false)
            }
        }
    }
}

// Implement ExtendedGuest marker trait
impl ExtendedGuest for PineconeComponent {}

// Export bindings for the component
golem_vector::export_bindings!(Component);
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        Err(unsupported_feature("Pinecone provider not implemented"))
    }
}

impl ExtendedGuest for PineconeComponent {}

type DurablePineconeComponent = DurableVector<PineconeComponent>;

golem_vector::export_vector!(DurablePineconeComponent with_types_in golem_vector);
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da

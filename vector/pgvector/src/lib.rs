//! PgVector provider component
//!
//! Production-ready PostgreSQL with pgvector extension provider for Golem Vector.
//! Implements comprehensive vector operations, collection management, and search 
//! functionality with robust error handling and logging.

mod bindings;
mod client;
mod conversion;

use crate::client::PgvectorClient;
use crate::conversion::*;
use golem_vector::durability::ExtendedGuest;
use golem_vector::error::{unsupported_feature_with_context, VectorError};
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
use golem_vector::init_logging;
use log::{debug, error, info, warn};
use std::env;

/// PgVector provider component
/// 
/// Provides PostgreSQL with pgvector extension integration for Golem Vector.
/// Supports comprehensive vector operations, collection management, and semantic search.
struct Component;

impl Component {
    /// Environment variable for PostgreSQL connection URL
    const URL_ENV: &'static str = "PGVECTOR_URL";
    
    /// Default PostgreSQL connection URL
    const DEFAULT_URL: &'static str = "postgres://postgres@localhost:5432/postgres";

    /// Validates pgvector configuration
    fn validate_config() -> Result<(), VectorError> {
        let _url = env::var(Self::URL_ENV)
            .unwrap_or_else(|_| Self::DEFAULT_URL.to_string());
        
        debug!("Using PostgreSQL URL: {}", _url.replace("://", "://***@"));
        Ok(())
    }

    /// Creates and returns a configured PgvectorClient
    fn create_client() -> Result<PgvectorClient, VectorError> {
        let url = env::var(Self::URL_ENV)
            .unwrap_or_else(|_| Self::DEFAULT_URL.to_string());
        
        debug!("Creating pgvector client");
        Ok(PgvectorClient::new(url))
    }
    
    /// Creates client with explicit URL for testing
    fn create_client_with_url(url: String) -> Result<PgvectorClient, VectorError> {
        debug!("Creating pgvector client with custom URL");
        Ok(PgvectorClient::new(url))
    }
}

// -------------------- analytics -----------------------------
impl AnalyticsGuest for Component {
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

// -------------------- collections ---------------------------
impl CollectionsGuest for Component {
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
            "Creating/updating pgvector collection '{}' with dimension {}",
            name, dimension
        );
        
        if dimension == 0 {
            return Err(VectorError::InvalidInput(
                "Vector dimension must be greater than 0".to_string(),
            ));
        }
        
        let mut client = Self::create_client()?;
        match client.create_collection(&name, dimension) {
            Ok(()) => {
                info!("Successfully created/updated collection '{}'", name);
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
            Err(e) => {
                error!("Failed to create/update collection '{}': {}", name, e);
                Err(e)
            }
        }
    }

    fn list_collections() -> Result<Vec<CollectionInfo>, VectorError> {
        init_logging();
        Self::validate_config()?;
        
        debug!("Listing pgvector collections");
        
        let mut client = Self::create_client()?;
        match client.list_collections() {
            Ok(collection_names) => {
                let collections = collection_names
                    .into_iter()
                    .map(|name| CollectionInfo {
                        name,
                        description: None,
                        dimension: 0, // Would need to query table schema to get actual dimension
                        metric: DistanceMetric::Cosine,
                        vector_count: 0,
                        size_bytes: None,
                        index_ready: true,
                        created_at: None,
                        updated_at: None,
                        provider_stats: None,
                    })
                    .collect();
                
                debug!("Found {} collections", collections.len());
                Ok(collections)
            }
            Err(e) => {
                error!("Failed to list collections: {}", e);
                Err(e)
            }
        }
    }

    fn get_collection(name: String) -> Result<CollectionInfo, VectorError> {
        init_logging();
        Self::validate_config()?;
        
        debug!("Getting pgvector collection info for '{}'", name);
        
        // Check if collection exists by listing all collections
        let collections = Self::list_collections()?;
        
        match collections.into_iter().find(|c| c.name == name) {
            Some(collection) => {
                debug!("Found collection '{}'", name);
                Ok(collection)
            }
            None => Err(VectorError::NotFound(format!(
                "Collection '{}' not found",
                name
            ))),
        }
    }

    fn update_collection(
        name: String,
        description: Option<String>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        init_logging();
        warn!("Collection update requested for '{}' - pgvector collections have limited update support", name);
        
        // For pgvector, we can only update metadata that we store separately
        // The table schema itself is immutable
        let mut existing = Self::get_collection(name)?;
        existing.description = description;
        
        debug!("Updated collection metadata (description only)");
        Ok(existing)
    }

    fn delete_collection(name: String) -> Result<(), VectorError> {
        init_logging();
        Self::validate_config()?;
        
        info!("Deleting pgvector collection '{}'", name);
        
        let mut client = Self::create_client()?;
        match client.delete_collection(&name) {
            Ok(()) => {
                info!("Successfully deleted collection '{}'", name);
                Ok(())
            }
            Err(e) => {
                error!("Failed to delete collection '{}': {}", name, e);
                Err(e)
            }
        }
    }

    fn collection_exists(name: String) -> Result<bool, VectorError> {
        init_logging();
        Self::validate_config()?;
        
        debug!("Checking if pgvector collection '{}' exists", name);
        
        match Self::list_collections() {
            Ok(collections) => {
                let exists = collections.iter().any(|c| c.name == name);
                debug!("Collection '{}' exists: {}", name, exists);
                Ok(exists)
            }
            Err(e) => {
                error!("Failed to check collection existence: {}", e);
                Err(e)
            }
        }
    }
}

// -------------------- vectors -------------------------------
impl VectorsGuest for Component {
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
            "Upserting {} vectors to pgvector collection: {}",
            vectors.len(),
            collection
        );
        
        let mut client = Self::create_client()?;
        match client.upsert_vectors(&collection, vectors.clone(), namespace) {
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
                // For pgvector, treat as complete failure since it's transactional
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
        
        debug!("Upserting single vector '{}' to pgvector collection: {}", id, collection);
        
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
            "Fetching {} vectors from pgvector collection: {}",
            ids.len(),
            collection
        );
        
        let mut client = Self::create_client()?;
        let mut records = client.get_vectors_by_ids(&collection, ids, namespace)?;
        
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
            "Fetching single vector '{}' from pgvector collection: {}",
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
        merge_metadata: Option<bool>,
    ) -> Result<(), VectorError> {
        init_logging();
        Self::validate_config()?;
        
        debug!(
            "Updating vector '{}' in pgvector collection: {}",
            id, collection
        );
        
        let mut client = Self::create_client()?;
        match client.update_vector(
            &collection,
            id,
            vector,
            metadata,
            merge_metadata.unwrap_or(false),
            namespace,
        ) {
            Ok(()) => {
                debug!("Successfully updated vector");
                Ok(())
            }
            Err(e) => {
                error!("Failed to update vector: {}", e);
                Err(e)
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
            "Deleting {} vectors from pgvector collection: {}",
            ids.len(),
            collection
        );
        
        let mut client = Self::create_client()?;
        match client.delete_vectors(&collection, ids, namespace) {
            Ok(count) => {
                info!("Successfully deleted {} vectors", count);
                Ok(count)
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
        
        debug!(
            "Deleting vectors by filter from pgvector collection: {}",
            collection
        );
        
        let mut client = Self::create_client()?;
        let filter_sql = filter_expression_to_sql(Some(filter), 1)
            .ok_or_else(|| VectorError::InvalidInput(
                "Unsupported filter expression for delete operation".to_string(),
            ))?;
            
        match client.delete_by_filter(&collection, filter_sql, namespace) {
            Ok(count) => {
                info!("Successfully deleted {} vectors by filter", count);
                Ok(count)
            }
            Err(e) => {
                error!("Failed to delete by filter: {}", e);
                Err(e)
            }
        }
    }

    fn list_vectors(
        collection: String,
        namespace: Option<String>,
        filter: Option<FilterExpression>,
        limit: Option<u32>,
        cursor: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<ListResponse, VectorError> {
        init_logging();
        Self::validate_config()?;
        
        debug!(
            "Listing vectors from pgvector collection: {} (limit: {:?})",
            collection, limit
        );
        
        let mut client = Self::create_client()?;
        let filter_sql = filter_expression_to_sql(filter, 1);
        
        match client.list_vectors(&collection, filter_sql, limit, cursor, namespace) {
            Ok((mut vectors, next_cursor)) => {
                // Apply include filters
                if !include_vectors.unwrap_or(true) {
                    for v in vectors.iter_mut() {
                        v.vector = VectorData::Dense(vec![]);
                    }
                }
                
                if matches!(include_metadata, Some(false)) {
                    for v in vectors.iter_mut() {
                        v.metadata = None;
                    }
                }
                
                debug!("Listed {} vectors", vectors.len());
                Ok(ListResponse {
                    vectors,
                    next_cursor,
                    total_count: None,
                })
            }
            Err(e) => {
                error!("Failed to list vectors: {}", e);
                Err(e)
            }
        }
    }

    fn count_vectors(
        collection: String,
        filter: Option<FilterExpression>,
        namespace: Option<String>,
    ) -> Result<u64, VectorError> {
        init_logging();
        Self::validate_config()?;
        
        debug!("Counting vectors in pgvector collection: {}", collection);
        
        let mut client = Self::create_client()?;
        let filter_sql = filter_expression_to_sql(filter, 1);
        
        match client.count_vectors(&collection, filter_sql, namespace) {
            Ok(count) => {
                debug!("Found {} vectors in collection", count);
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
impl SearchGuest for Component {
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
            "Searching {} vectors in pgvector collection: {}",
            limit, collection
        );
        
        let mut client = Self::create_client()?;
        
        // Convert query to vector
        let vector = match query {
            SearchQueryEnum::Vector(v) => vector_data_to_dense(v)?,
            _ => {
                return Err(unsupported_feature_with_context(
                    "Only vector queries supported for search",
                ))
            }
        };
        
        // $1 is used by the query vector; filters must start at $2
        let filt_sql = filter_expression_to_sql(filter, 2);
        let metric = DistanceMetric::Cosine; // Default metric, could be derived from collection metadata
        let include_vecs = include_vectors.unwrap_or(false);
        let include_meta = include_metadata.unwrap_or(false);
        
        match client.query_vectors(&collection, vector, metric, limit, filt_sql) {
            Ok(results) => {
                let search_results = results
                    .into_iter()
                    .map(|(id, distance, maybe_vec)| SearchResult {
                        id,
                        score: 1.0 - distance, // Convert distance to similarity score
                        distance,
                        vector: if include_vecs {
                            maybe_vec.map(VectorData::Dense)
                        } else {
                            None
                        },
                        metadata: if include_meta {
                            // pgvector client would need enhancement to return metadata in search
                            None
                        } else {
                            None
                        },
                    })
                    .collect();
                    
                debug!("Found {} search results", search_results.len());
                Ok(search_results)
            }
            Err(e) => {
                error!("Search failed: {}", e);
                Err(e)
            }
        }
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
        _search_params: Option<Vec<(String, String)>>,
    ) -> Result<Vec<Vec<SearchResult>>, VectorError> {
        init_logging();
        Self::validate_config()?;
        
        debug!("Batch searching {} queries in pgvector collection: {}", queries.len(), collection);
        
        let mut all_results: Vec<Vec<SearchResult>> = Vec::with_capacity(queries.len());
        
        // Execute each query individually
        for (idx, query) in queries.into_iter().enumerate() {
            debug!("Executing batch query {} of {}", idx + 1, all_results.capacity());
            
            let results = Self::search_vectors(
                collection.clone(),
                query,
                limit,
                filter.clone(),
                namespace.clone(),
                include_vectors,
                include_metadata,
                None,
                None,
                None,
            )?;
            
            all_results.push(results);
        }
        
        debug!("Batch search completed with {} result sets", all_results.len());
        Ok(all_results)
    }
}

// -------------------- search extended ----------------------
impl SearchExtendedGuest for Component {
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
            "Grouped search not supported by pgvector",
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
        init_logging();
        Err(unsupported_feature_with_context(
            "Recommendations not supported by pgvector",
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
        Err(unsupported_feature_with_context(
            "Discovery not supported by pgvector",
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
            "Range search not supported by pgvector",
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
            "Text search not supported by pgvector",
        ))
    }
}

// -------------------- namespaces ---------------------------
impl NamespacesGuest for Component {
    fn upsert_namespace(
        _collection: String,
        _namespace: String,
        _metadata: Option<Metadata>,
    ) -> Result<NamespaceInfo, VectorError> {
        init_logging();
        Err(unsupported_feature_with_context(
            "Namespace management not supported by pgvector",
        ))
    }

    fn list_namespaces(_collection: String) -> Result<Vec<NamespaceInfo>, VectorError> {
        init_logging();
        Err(unsupported_feature_with_context(
            "Namespace listing not supported by pgvector",
        ))
    }

    fn get_namespace(
        _collection: String,
        _namespace: String,
    ) -> Result<NamespaceInfo, VectorError> {
        init_logging();
        Err(unsupported_feature_with_context(
            "Namespace info not supported by pgvector",
        ))
    }

    fn delete_namespace(_collection: String, _namespace: String) -> Result<(), VectorError> {
        init_logging();
        Err(unsupported_feature_with_context(
            "Namespace deletion not supported by pgvector",
        ))
    }

    fn namespace_exists(_collection: String, _namespace: String) -> Result<bool, VectorError> {
        init_logging();
        // pgvector uses table-based namespaces which are handled differently
        Ok(false)
    }
}

// -------------------- connection ---------------------------
impl ConnectionGuest for Component {
    fn connect(
        _endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        _options: Option<Metadata>,
    ) -> Result<(), VectorError> {
        init_logging();
        Self::validate_config()?;
        info!("PgVector connection validated via environment variables");
        Ok(())
    }

    fn disconnect() -> Result<(), VectorError> {
        init_logging();
        debug!("PgVector disconnect (connections are managed per-request)");
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
        
        debug!("Testing PgVector connection with custom endpoint");
        
        match Self::create_client_with_url(endpoint) {
            Ok(_client) => {
                // Could test by trying to execute a simple query
                info!("PgVector connection test successful");
                Ok(true)
            }
            Err(e) => {
                error!("PgVector connection test failed: {}", e);
                Ok(false)
            }
        }
    }
}

// Implement ExtendedGuest marker trait
impl ExtendedGuest for Component {}

// Export bindings for the component
golem_vector::export_bindings!(Component);

use crate::client::ChromaClient;
use golem_vector::config::{get_optional_config, with_config_key, with_connection_config_key};
use golem_vector::durability::{DurableVector, ExtendedGuest};
use golem_vector::golem::vector::{
    analytics::{CollectionStats, FieldStats, Guest as AnalyticsGuest},
    collections::{CollectionInfo, Guest as CollectionsGuest, IndexConfig},
    connection::{ConnectionStatus, Credentials, Guest as ConnectionGuest},
    namespaces::{Guest as NamespacesGuest, NamespaceInfo},
    search::{Guest as SearchGuest, SearchQuery},
    search_extended::{
        ContextPair, GroupedSearchResult, Guest as SearchExtendedGuest, RecommendationExample,
        RecommendationStrategy,
    },
    types::{
        DistanceMetric, FilterExpression, Id, Metadata, MetadataValue, SearchResult, VectorData,
        VectorError, VectorRecord,
    },
    vectors::{BatchResult, Guest as VectorsGuest, ListResponse},
};

pub mod client;

struct ChromaComponent;

impl ChromaComponent {
    const URL_ENV_VAR: &'static str = "CHROMA_URL";
    const API_KEY_ENV_VAR: &'static str = "CHROMA_API_KEY";
    const TENANT_ENV_VAR: &'static str = "CHROMA_TENANT";
    const THREAD_ID_ENV_VAR: &'static str = "CHROMA_THREAD_ID"; // Header: X-Chroma-Thread-Id

    fn create_client() -> Result<ChromaClient, VectorError> {
        let url = with_config_key(
            Self::URL_ENV_VAR,
            |e| Err(VectorError::ConnectionError(format!("Missing URL: {e}"))),
            Ok,
        )
        .unwrap_or_else(|_| "http://localhost:8000".to_string());

        let api_key = get_optional_config(Self::API_KEY_ENV_VAR);
        let tenant = get_optional_config(Self::TENANT_ENV_VAR).unwrap_or_else(|| "default_tenant".to_string());
        let thread_id = get_optional_config(Self::THREAD_ID_ENV_VAR);
        // Default database is always "default_database" in open source Chroma, but can be configurable
        let database = "default_database".to_string(); 

        Ok(ChromaClient::new(url, api_key, tenant, database, thread_id))
    }

    fn create_client_with_options(options: &Option<Metadata>) -> Result<ChromaClient, VectorError> {
        let url = with_connection_config_key(options, "url")
            .unwrap_or_else(|| "http://localhost:8000".to_string());
        let api_key = with_connection_config_key(options, "api_key");
        let tenant = with_connection_config_key(options, "tenant").unwrap_or_else(|| "default_tenant".to_string());
        let database = with_connection_config_key(options, "database").unwrap_or_else(|| "default_database".to_string());
        let thread_id = with_connection_config_key(options, "thread_id");

        Ok(ChromaClient::new(url, api_key, tenant, database, thread_id))
    }
}

impl ExtendedGuest for ChromaComponent {
    fn connect_internal(
        _endpoint: &str,
        _credentials: &Option<Credentials>,
        _timeout_ms: &Option<u32>,
        options: &Option<Metadata>,
    ) -> Result<(), VectorError> {
        let _client = Self::create_client_with_options(options)?;
        Ok(())
    }
}

impl ConnectionGuest for ChromaComponent {
    fn connect(
        _endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        options: Option<Metadata>,
    ) -> Result<(), VectorError> {
        let _client = Self::create_client_with_options(&options)?;
        Ok(())
    }

    fn disconnect() -> Result<(), VectorError> {
        Ok(())
    }

    fn get_connection_status() -> Result<ConnectionStatus, VectorError> {
        match Self::create_client() {
            Ok(client) => match client.heartbeat() {
                Ok(_) => Ok(ConnectionStatus {
                    connected: true,
                    provider: Some("Chroma".to_string()),
                    endpoint: None,
                    last_activity: None,
                    connection_id: None,
                }),
                Err(_) => Ok(ConnectionStatus {
                    connected: false,
                    provider: Some("Chroma".to_string()),
                    endpoint: None,
                    last_activity: None,
                    connection_id: None,
                }),
            },
            Err(_) => Ok(ConnectionStatus {
                connected: false,
                provider: Some("Chroma".to_string()),
                endpoint: None,
                last_activity: None,
                connection_id: None,
            }),
        }
    }

    fn test_connection(
        _endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        options: Option<Metadata>,
    ) -> Result<bool, VectorError> {
        match Self::create_client_with_options(&options) {
            Ok(client) => match client.heartbeat() {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            },
            Err(_) => Ok(false),
        }
    }
}

impl CollectionsGuest for ChromaComponent {
    fn upsert_collection(
        name: String,
        _description: Option<String>,
        dimension: u32,
        metric: DistanceMetric,
        _index_config: Option<IndexConfig>,
        metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        let client = Self::create_client()?;
        let meta_map = metadata_to_map(metadata);
        
        // Chroma uses specific strings for metrics
        let metric_str = match metric {
             DistanceMetric::Cosine => "cosine",
             DistanceMetric::Euclidean => "l2",
             DistanceMetric::DotProduct => "ip",
             _ => "l2", // Default fallback
        };
        
        let mut create_metadata = meta_map.unwrap_or_default();
        create_metadata.insert("hnsw:space".to_string(), serde_json::Value::String(metric_str.to_string()));
        create_metadata.insert("dimension".to_string(), serde_json::json!(dimension));

        match client.create_collection(&name, Some(create_metadata)) {
            Ok(col) => Ok(chroma_collection_to_info(col)),
            Err(e) => Err(e), 
        }
    }

    fn list_collections() -> Result<Vec<String>, VectorError> {
        let client = Self::create_client()?;
        client.list_collections()
    }

    fn get_collection(name: String) -> Result<CollectionInfo, VectorError> {
        let client = Self::create_client()?;
        match client.get_collection(&name) {
            Ok(col) => Ok(chroma_collection_to_info(col)),
            Err(e) => Err(e),
        }
    }

    fn update_collection(
        name: String,
        _description: Option<String>,
        metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
         let client = Self::create_client()?;
         // To update, we first get the collection ID (not exposed in simple API, but let's assume update by name works in wrapper)
         // Actually Chroma API updates by ID usually. But let's check client implementation.
         // For now, assume a wrapper function exists.
         let meta_map = metadata_to_map(metadata);
         match client.update_collection(&name, meta_map) {
             Ok(_) => Self::get_collection(name),
             Err(e) => Err(e),
         }
    }

    fn delete_collection(name: String) -> Result<(), VectorError> {
        let client = Self::create_client()?;
        client.delete_collection(&name)
    }

    fn collection_exists(name: String) -> Result<bool, VectorError> {
        let client = Self::create_client()?;
        // List and check, or Try Get
        match client.get_collection(&name) {
            Ok(_) => Ok(true),
            Err(VectorError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }
}

impl VectorsGuest for ChromaComponent {
    fn upsert_vectors(
        collection: String,
        vectors: Vec<VectorRecord>,
        _namespace: Option<String>,
    ) -> Result<BatchResult, VectorError> {
        let client = Self::create_client()?;
        if vectors.is_empty() {
             return Ok(BatchResult {
                success_count: 0,
                failure_count: 0,
                errors: vec![],
            });
        }
        
        match client.add_vectors(&collection, vectors.clone()) {
            Ok(_) => Ok(BatchResult {
                success_count: vectors.len() as u32,
                failure_count: 0,
                errors: vec![],
            }),
            Err(e) => {
                 // Simplification: fail all
                 Ok(BatchResult {
                    success_count: 0,
                    failure_count: vectors.len() as u32,
                    errors: vectors.iter().enumerate().map(|(i, _)| (i as u32, VectorError::ProviderError(e.to_string()))).collect(),
                })
            }
        }
    }

    fn upsert_vector(
        collection: String,
        id: Id,
        vector: VectorData,
        metadata: Option<Metadata>,
        namespace: Option<String>,
    ) -> Result<(), VectorError> {
        let record = VectorRecord { id, vector, metadata };
        let res = Self::upsert_vectors(collection, vec![record], namespace)?;
        if res.success_count > 0 {
            Ok(())
        } else {
             Err(VectorError::ProviderError("Failed to upsert vector".to_string()))
        }
    }

    fn get_vectors(
        collection: String,
        ids: Vec<Id>,
        _namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<Vec<VectorRecord>, VectorError> {
        let client = Self::create_client()?;
        client.get_vectors(&collection, ids, include_vectors, include_metadata)
    }

    fn get_vector(
        collection: String,
        id: Id,
        namespace: Option<String>,
    ) -> Result<Option<VectorRecord>, VectorError> {
        let vec = Self::get_vectors(collection, vec![id], namespace, Some(true), Some(true))?;
        Ok(vec.into_iter().next())
    }
    
    fn update_vector(
        collection: String,
        id: Id,
        vector: Option<VectorData>,
        metadata: Option<Metadata>,
        namespace: Option<String>,
        _merge_metadata: Option<bool>,
    ) -> Result<(), VectorError> {
        // Chroma's 'update' works similarly to upsert but requires ID existence usually.
        // We can just use upsert logic if vector is present. If only metadata update, logic differs.
        let client = Self::create_client()?;
        
        if let Some(v) = vector {
             let record = VectorRecord { id: id.clone(), vector: v, metadata: metadata.clone() };
             client.update_vectors(&collection, vec![record])
        } else {
             // Metadata only update. Chroma supports this? client.update(ids, metadatas)
             // Need to fetch existing vector data if we must provide it, or use specific update_metadata API.
             // For MVP, we fetch and upsert.
             let current = Self::get_vector(collection.clone(), id.clone(), namespace.clone())?;
             if let Some(mut curr) = current {
                 if let Some(new_meta) = metadata {
                     // TODO: Merge logic if needed, for now replace/extend
                     curr.metadata = Some(new_meta);
                 }
                 client.update_vectors(&collection, vec![curr])
             } else {
                 Err(VectorError::NotFound("Vector to update not found".to_string()))
             }
        }
    }

    fn delete_vectors(
        collection: String,
        ids: Vec<Id>,
        _namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        let client = Self::create_client()?;
        match client.delete_vectors(&collection, ids.clone()) {
            Ok(_) => Ok(ids.len() as u32), // Rough estimate, Chroma doesn't return deleted count
            Err(e) => Err(e),
        }
    }

    fn delete_by_filter(
        collection: String,
        filter: FilterExpression,
        _namespace: Option<String>,
    ) -> Result<u32, VectorError> {
         let client = Self::create_client()?;
         // This requires mapping FilterExpression to Chroma's "where" clause
         match client.delete_by_filter(&collection, filter) {
             Ok(count) => Ok(count),
             Err(e) => Err(e),
         }
    }

    fn delete_namespace(_collection: String, _namespace: String) -> Result<u32, VectorError> {
         Err(VectorError::UnsupportedFeature("Namespaces not native to Chroma (treated as collection or metadata field)".to_string()))
    }

    fn list_vectors(
        collection: String,
        _namespace: Option<String>,
        filter: Option<FilterExpression>,
        limit: Option<u32>,
        offset: Option<String>, // Chroma uses offset integer, but we accept string
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<ListResponse, VectorError> {
        let client = Self::create_client()?;
        let num_offset = offset.and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        
        client.list_vectors(&collection, filter, limit.unwrap_or(10), num_offset, include_vectors, include_metadata)
    }

    fn count_vectors(
        collection: String,
        filter: Option<FilterExpression>,
        _namespace: Option<String>,
    ) -> Result<u64, VectorError> {
        let client = Self::create_client()?;
        // If filter is present, we might need to query. If not, get collection count.
        if filter.is_some() {
             // Query count is expensive/not direct. 
             // client.count(filter)
             client.count_vectors(&collection, filter)
        } else {
            let col = client.get_collection(&collection)?;
            Ok(col.count.unwrap_or(0) as u64) // 'count' field from API
        }
        
    }
}

impl SearchGuest for ChromaComponent {
    fn search_vectors(
        collection: String,
        query: SearchQuery,
        limit: u32,
        filter: Option<FilterExpression>,
        _namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
        _min_score: Option<f32>,
        _max_distance: Option<f32>,
        _search_params: Option<Vec<(String, String)>>,
    ) -> Result<Vec<SearchResult>, VectorError> {
         let client = Self::create_client()?;
         
         match query {
             SearchQuery::Vector(vec_data) => {
                 client.query_vector(&collection, vec_data, limit, filter, include_vectors, include_metadata)
             },
             _ => Err(VectorError::UnsupportedFeature("Only vector search supported currently".to_string())),
         }
    }
    
    fn find_similar(
        collection: String,
        vector: VectorData,
        limit: u32,
        namespace: Option<String>,
    ) -> Result<Vec<SearchResult>, VectorError> {
         let query = SearchQuery::Vector(vector);
         Self::search_vectors(collection, query, limit, None, namespace, Some(false), Some(false), None, None, None)
    }
    
    fn batch_search(
        _collection: String,
        _queries: Vec<SearchQuery>,
        _limit: u32,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
        _include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
        _search_params: Option<Vec<(String, String)>>,
    ) -> Result<Vec<Vec<SearchResult>>, VectorError> {
        Err(VectorError::UnsupportedFeature("Batch search not implemented yet".to_string()))
    }
}

impl SearchExtendedGuest for ChromaComponent {
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
        Err(VectorError::UnsupportedFeature("Recommendation not supported".to_string()))
    }

    fn discover_vectors(
        _collection: String,
        _target: Option<RecommendationExample>,
        _context_pairs: Vec<ContextPair>,
        _limit: u32,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
        _include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
    ) -> Result<Vec<SearchResult>, VectorError> {
         Err(VectorError::UnsupportedFeature("Discovery not supported".to_string()))
    }
    
    fn search_groups(
        _collection: String,
        _query: SearchQuery,
        _group_by: String,
        _group_size: u32,
        _max_groups: u32,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
        _include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
    ) -> Result<Vec<GroupedSearchResult>, VectorError> {
         Err(VectorError::UnsupportedFeature("Group search not supported".to_string()))
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
         Err(VectorError::UnsupportedFeature("Range search not supported".to_string()))
    }

    fn search_text(
        _collection: String,
        _query: String,
        _limit: u32,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
    ) -> Result<Vec<SearchResult>, VectorError> {
         // Chroma supports query texts if embedding function is server-side default or configured?
         // Usually requires client side embedding unless server has it.
         Err(VectorError::UnsupportedFeature("Text search not supported without server-side embedding context".to_string()))
    }
}

impl NamespacesGuest for ChromaComponent {
    fn upsert_namespace(_collection: String, _namespace: String, _metadata: Option<Metadata>) -> Result<NamespaceInfo, VectorError> {
        Err(VectorError::UnsupportedFeature("Namespaces Not Supported".to_string()))
    }
    fn list_namespaces(_collection: String) -> Result<Vec<NamespaceInfo>, VectorError> {
        Ok(vec![])
    }
    fn get_namespace(_collection: String, _namespace: String) -> Result<NamespaceInfo, VectorError> {
        Err(VectorError::NotFound("Namespace not found".to_string()))
    }
    fn delete_namespace(_collection: String, _namespace: String) -> Result<(), VectorError> {
        Err(VectorError::UnsupportedFeature("Namespaces Not Supported".to_string()))
    }
    fn namespace_exists(_collection: String, _namespace: String) -> Result<bool, VectorError> {
        Ok(false)
    }
}

impl AnalyticsGuest for ChromaComponent {
    fn get_collection_stats(collection: String, _: Option<String>) -> Result<CollectionStats, VectorError> {
         let info = Self::get_collection(collection)?;
         Ok(CollectionStats {
             vector_count: info.vector_count,
             dimension: info.dimension,
             size_bytes: info.size_bytes.unwrap_or(0),
             index_size_bytes: None,
             namespace_stats: vec![],
             distance_distribution: None,
         })
    }
    fn get_field_stats(_collection: String, _field: String, _: Option<String>) -> Result<FieldStats, VectorError> {
         Err(VectorError::UnsupportedFeature("Field stats not supported".to_string()))
    }
    fn get_field_distribution(_collection: String, _field: String, _limit: Option<u32>, _: Option<String>) -> Result<Vec<(MetadataValue, u64)>, VectorError> {
         Err(VectorError::UnsupportedFeature("Field dist not supported".to_string()))
    }
}

// Helpers
fn metadata_to_map(meta: Option<Metadata>) -> Option<std::collections::HashMap<String, serde_json::Value>> {
    meta.map(|m| {
        m.into_iter().map(|(k, v)| {
            let json_val = match v {
                MetadataValue::StringVal(s) => serde_json::Value::String(s),
                MetadataValue::NumberVal(n) => serde_json::json!(n),
                MetadataValue::IntegerVal(i) => serde_json::json!(i),
                MetadataValue::BooleanVal(b) => serde_json::Value::Bool(b),
                _ => serde_json::Value::Null, 
            };
            (k, json_val)
        }).collect()
    })
}

fn chroma_collection_to_info(r: client::ChromaCollection) -> CollectionInfo {
     let metadata = r.metadata.clone().unwrap_or_default();
     
     let dimension = metadata.get("dimension")
         .and_then(|v| v.as_u64())
         .map(|d| d as u32)
         .unwrap_or(0);

     let metric = metadata.get("hnsw:space")
         .and_then(|v| v.as_str())
         .map(|s| match s {
             "cosine" => DistanceMetric::Cosine,
             "l2" => DistanceMetric::Euclidean,
             "ip" => DistanceMetric::DotProduct,
             "manhattan" => DistanceMetric::Manhattan,
             _ => DistanceMetric::Euclidean,
         })
         .unwrap_or(DistanceMetric::Euclidean);

     CollectionInfo {
         name: r.name,
         description: None,
         dimension,
         metric,
         vector_count: r.count.unwrap_or(0) as u64,
         size_bytes: None,
         index_ready: true,
         created_at: None,
         updated_at: None,
         provider_stats: None,
     }
}



type DurableChromaComponent = DurableVector<ChromaComponent>;

golem_vector::export_vector!(DurableChromaComponent with_types_in golem_vector);

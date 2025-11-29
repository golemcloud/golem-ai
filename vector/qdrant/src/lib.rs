use crate::client::QdrantClient;
use crate::conversions::{
    collection_info_to_export_collection_info, create_batch_search_request,
    create_collection_config, create_count_request, create_delete_points_request,
    create_discover_request, create_get_points_request, create_recommend_request,
    create_scroll_request, create_search_request, records_to_vector_records,
    scored_points_to_search_results, vector_records_to_upsert_request,
};
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

mod client;
mod conversions;

struct QdrantComponent;

impl QdrantComponent {
    const URL_ENV_VAR: &'static str = "QDRANT_URL";
    const API_KEY_ENV_VAR: &'static str = "QDRANT_API_KEY";

    fn create_client() -> Result<QdrantClient, VectorError> {
        let url = with_config_key(
            Self::URL_ENV_VAR,
            |e| Err(VectorError::ConnectionError(format!("Missing URL: {e}"))),
            Ok,
        )
        .unwrap_or_else(|_| "http://localhost:6333".to_string());

        let api_key = get_optional_config(Self::API_KEY_ENV_VAR);

        Ok(QdrantClient::new(url, api_key))
    }

    fn create_client_with_options(options: &Option<Metadata>) -> Result<QdrantClient, VectorError> {
        let url = with_connection_config_key(options, "url")
            .unwrap_or_else(|| "http://localhost:6333".to_string());

        let api_key = with_connection_config_key(options, "api_key");

        Ok(QdrantClient::new(url, api_key))
    }

    fn metadata_indexes(
        client: &QdrantClient,
        collection_name: &str,
        vectors: &[VectorRecord],
    ) -> Result<(), VectorError> {
        use std::collections::HashSet;

        let mut attempted_fields: HashSet<(String, String)> = HashSet::new();

        for vector in vectors {
            if let Some(metadata) = &vector.metadata {
                for (field_name, field_value) in metadata {
                    let field_type = match field_value {
                        MetadataValue::StringVal(_) => "keyword",
                        MetadataValue::IntegerVal(_) => "integer",
                        MetadataValue::NumberVal(_) => "float",
                        MetadataValue::BooleanVal(_) => "bool",
                        MetadataValue::GeoVal(_) => "geo",
                        MetadataValue::ArrayVal(_)
                        | MetadataValue::ObjectVal(_)
                        | MetadataValue::DatetimeVal(_)
                        | MetadataValue::BlobVal(_)
                        | MetadataValue::NullVal => continue,
                    };

                    let field_key = (field_name.clone(), field_type.to_string());

                    if !attempted_fields.contains(&field_key) {
                        attempted_fields.insert(field_key);

                        if client
                            .create_field_index(collection_name, field_name, field_type)
                            .is_err()
                        {}
                    }
                }
            }
        }

        Ok(())
    }
}

impl ExtendedGuest for QdrantComponent {
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

impl ConnectionGuest for QdrantComponent {
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
            Ok(client) => match client.list_collections() {
                Ok(_) => Ok(ConnectionStatus {
                    connected: true,
                    provider: Some("Qdrant".to_string()),
                    endpoint: None,
                    last_activity: None,
                    connection_id: None,
                }),
                Err(_) => Ok(ConnectionStatus {
                    connected: false,
                    provider: Some("Qdrant".to_string()),
                    endpoint: None,
                    last_activity: None,
                    connection_id: None,
                }),
            },
            Err(_) => Ok(ConnectionStatus {
                connected: false,
                provider: Some("Qdrant".to_string()),
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
            Ok(client) => match client.list_collections() {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            },
            Err(_) => Ok(false),
        }
    }
}

impl CollectionsGuest for QdrantComponent {
    fn upsert_collection(
        name: String,
        _description: Option<String>,
        dimension: u32,
        metric: DistanceMetric,
        _index_config: Option<IndexConfig>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        let client = Self::create_client()?;

        let config = create_collection_config(dimension, metric);

        let create_request = client::CreateCollectionRequest {
            collection_name: name.clone(),
            config,
        };

        match client.create_collection(&create_request) {
            Ok(response) => {
                if response.result {
                    Self::get_collection(name)
                } else {
                    Err(VectorError::ProviderError(
                        "Failed to create collection".to_string(),
                    ))
                }
            }
            Err(e) => Err(e),
        }
    }

    fn list_collections() -> Result<Vec<String>, VectorError> {
        let client = Self::create_client()?;

        match client.list_collections() {
            Ok(response) => Ok(response
                .result
                .collections
                .into_iter()
                .map(|c| c.name)
                .collect()),
            Err(e) => Err(e),
        }
    }

    fn get_collection(name: String) -> Result<CollectionInfo, VectorError> {
        let client = Self::create_client()?;

        match client.get_collection(&name) {
            Ok(response) => collection_info_to_export_collection_info(&name, &response.result),
            Err(e) => Err(e),
        }
    }

    fn update_collection(
        name: String,
        _description: Option<String>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        Self::get_collection(name)
    }

    fn delete_collection(name: String) -> Result<(), VectorError> {
        let client = Self::create_client()?;

        match client.delete_collection(&name) {
            Ok(response) => {
                if response.result {
                    Ok(())
                } else {
                    Err(VectorError::ProviderError(
                        "Failed to delete collection".to_string(),
                    ))
                }
            }
            Err(e) => Err(e),
        }
    }

    fn collection_exists(name: String) -> Result<bool, VectorError> {
        let client = Self::create_client()?;
        client.collection_exists(&name)
    }
}

impl VectorsGuest for QdrantComponent {
    fn upsert_vectors(
        collection: String,
        vectors: Vec<VectorRecord>,
        _namespace: Option<String>,
    ) -> Result<BatchResult, VectorError> {
        let client = Self::create_client()?;

        if !vectors.is_empty() {
            Self::metadata_indexes(&client, &collection, &vectors)?;
        }

        let upsert_request = vector_records_to_upsert_request(&vectors)?;

        match client.upsert_points(&collection, &upsert_request) {
            Ok(response) => {
                if response.result.status == "acknowledged" || response.result.status == "completed"
                {
                    Ok(BatchResult {
                        success_count: vectors.len() as u32,
                        failure_count: 0,
                        errors: vec![],
                    })
                } else {
                    Ok(BatchResult {
                        success_count: 0,
                        failure_count: vectors.len() as u32,
                        errors: vectors
                            .into_iter()
                            .enumerate()
                            .map(|(i, _)| {
                                (
                                    i as u32,
                                    VectorError::ProviderError("Insert failed".to_string()),
                                )
                            })
                            .collect(),
                    })
                }
            }
            Err(e) => Err(e),
        }
    }

    fn upsert_vector(
        collection: String,
        id: Id,
        vector: VectorData,
        metadata: Option<Metadata>,
        namespace: Option<String>,
    ) -> Result<(), VectorError> {
        let record = VectorRecord {
            id,
            vector,
            metadata,
        };

        let result = Self::upsert_vectors(collection, vec![record], namespace)?;

        if result.success_count > 0 {
            Ok(())
        } else {
            Err(VectorError::ProviderError(
                "Failed to upsert vector".to_string(),
            ))
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

        let with_vector = include_vectors.unwrap_or(true);
        let with_payload = include_metadata.unwrap_or(true);

        let get_request = create_get_points_request(&ids, with_payload, with_vector)?;

        match client.get_points(&collection, &get_request) {
            Ok(response) => records_to_vector_records(&response.result),
            Err(e) => Err(e),
        }
    }

    fn get_vector(
        collection: String,
        id: Id,
        namespace: Option<String>,
    ) -> Result<Option<VectorRecord>, VectorError> {
        let vectors = Self::get_vectors(collection, vec![id], namespace, Some(true), Some(true))?;

        Ok(vectors.into_iter().next())
    }

    fn update_vector(
        collection: String,
        id: Id,
        vector: Option<VectorData>,
        metadata: Option<Metadata>,
        namespace: Option<String>,
        merge_metadata: Option<bool>,
    ) -> Result<(), VectorError> {
        if let Some(vector_data) = vector {
            Self::upsert_vector(collection, id, vector_data, metadata, namespace)
        } else {
            let current = Self::get_vector(collection.clone(), id.clone(), namespace.clone())?;
            if let Some(current_record) = current {
                let new_metadata = if merge_metadata.unwrap_or(false) {
                    if let (Some(current_meta), Some(new_meta)) =
                        (&current_record.metadata, &metadata)
                    {
                        let mut merged = current_meta.clone();
                        merged.extend(new_meta.clone());
                        Some(merged)
                    } else {
                        metadata.or(current_record.metadata)
                    }
                } else {
                    metadata.or(current_record.metadata)
                };

                Self::upsert_vector(
                    collection,
                    id,
                    current_record.vector,
                    new_metadata,
                    namespace,
                )
            } else {
                Err(VectorError::NotFound("Vector not found".to_string()))
            }
        }
    }

    fn delete_vectors(
        collection: String,
        ids: Vec<Id>,
        _namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        let client = Self::create_client()?;

        let delete_request = create_delete_points_request(Some(&ids), None)?;

        match client.delete_points(&collection, &delete_request) {
            Ok(response) => {
                if response.result.status == "acknowledged" || response.result.status == "completed"
                {
                    Ok(ids.len() as u32)
                } else {
                    Ok(0)
                }
            }
            Err(e) => Err(e),
        }
    }

    fn delete_by_filter(
        collection: String,
        filter: FilterExpression,
        _namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        let client = Self::create_client()?;

        let count = Self::count_vectors(collection.clone(), Some(filter.clone()), None)?;

        let delete_request = create_delete_points_request(None, Some(&filter))?;

        match client.delete_points(&collection, &delete_request) {
            Ok(response) => {
                if response.result.status == "acknowledged" || response.result.status == "completed"
                {
                    Ok(count as u32)
                } else {
                    Ok(0)
                }
            }
            Err(e) => Err(e),
        }
    }

    fn delete_namespace(_collection: String, _namespace: String) -> Result<u32, VectorError> {
        Err(VectorError::ProviderError(
            "Namespaces not supported in Qdrant".to_string(),
        ))
    }

    fn list_vectors(
        collection: String,
        _namespace: Option<String>,
        filter: Option<FilterExpression>,
        limit: Option<u32>,
        offset: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<ListResponse, VectorError> {
        let client = Self::create_client()?;

        let with_vector = include_vectors.unwrap_or(true);
        let with_payload = include_metadata.unwrap_or(true);

        let scroll_request = create_scroll_request(
            filter.as_ref(),
            limit,
            offset.as_ref(),
            with_payload,
            with_vector,
        )?;

        match client.scroll_points(&collection, &scroll_request) {
            Ok(response) => {
                let records = records_to_vector_records(&response.result.points)?;
                let next_offset = response.result.next_page_offset.map(|pid| match pid {
                    client::PointId::Integer(i) => (i as i64).to_string(),
                    client::PointId::Uuid(s) => s,
                });

                Ok(ListResponse {
                    vectors: records,
                    next_cursor: next_offset.map(|o| o.to_string()),
                    total_count: None,
                })
            }
            Err(e) => Err(e),
        }
    }

    fn count_vectors(
        collection: String,
        filter: Option<FilterExpression>,
        _namespace: Option<String>,
    ) -> Result<u64, VectorError> {
        let client = Self::create_client()?;

        let count_request = create_count_request(filter.as_ref())?;

        match client.count_points(&collection, &count_request) {
            Ok(response) => Ok(response.result.count),
            Err(e) => Err(e),
        }
    }
}

impl SearchGuest for QdrantComponent {
    fn search_vectors(
        collection: String,
        query: SearchQuery,
        limit: u32,
        filter: Option<FilterExpression>,
        _namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
        min_score: Option<f32>,
        _max_distance: Option<f32>,
        _search_params: Option<Vec<(String, String)>>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        let client = Self::create_client()?;

        let with_vector = include_vectors.unwrap_or(false);
        let with_payload = include_metadata.unwrap_or(true);

        let search_request = create_search_request(
            &query,
            limit,
            None,
            filter.as_ref(),
            with_payload,
            with_vector,
            min_score.map(|s| s as f64),
        )?;

        match client.search_points(&collection, &search_request) {
            Ok(response) => scored_points_to_search_results(&response.result),
            Err(e) => Err(e),
        }
    }

    fn find_similar(
        collection: String,
        vector: VectorData,
        limit: u32,
        namespace: Option<String>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        let query = SearchQuery::Vector(vector);
        Self::search_vectors(
            collection,
            query,
            limit,
            None,
            namespace,
            Some(false),
            Some(false),
            None,
            None,
            None,
        )
    }

    fn batch_search(
        collection: String,
        queries: Vec<SearchQuery>,
        limit: u32,
        filter: Option<FilterExpression>,
        _namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
        _search_params: Option<Vec<(String, String)>>,
    ) -> Result<Vec<Vec<SearchResult>>, VectorError> {
        let client = Self::create_client()?;

        let with_vector = include_vectors.unwrap_or(false);
        let with_payload = include_metadata.unwrap_or(true);

        let batch_request = create_batch_search_request(
            &queries,
            limit,
            filter.as_ref(),
            with_payload,
            with_vector,
        )?;

        match client.batch_search(&collection, &batch_request) {
            Ok(response) => {
                let mut results = Vec::new();
                for scored_points in response.result {
                    let search_results = scored_points_to_search_results(&scored_points)?;
                    results.push(search_results);
                }
                Ok(results)
            }
            Err(e) => Err(e),
        }
    }
}

impl SearchExtendedGuest for QdrantComponent {
    fn recommend_vectors(
        collection: String,
        positive: Vec<RecommendationExample>,
        negative: Option<Vec<RecommendationExample>>,
        limit: u32,
        filter: Option<FilterExpression>,
        _namespace: Option<String>,
        _strategy: Option<RecommendationStrategy>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        let client = Self::create_client()?;

        let with_vector = include_vectors.unwrap_or(false);
        let with_payload = include_metadata.unwrap_or(true);

        let recommend_request = create_recommend_request(
            &positive,
            negative.as_deref(),
            limit,
            filter.as_ref(),
            with_payload,
            with_vector,
        )?;

        match client.recommend_points(&collection, &recommend_request) {
            Ok(response) => scored_points_to_search_results(&response.result),
            Err(e) => Err(e),
        }
    }

    fn discover_vectors(
        collection: String,
        target: Option<RecommendationExample>,
        context_pairs: Vec<ContextPair>,
        limit: u32,
        filter: Option<FilterExpression>,
        _namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        let client = Self::create_client()?;

        let with_vector = include_vectors.unwrap_or(false);
        let with_payload = include_metadata.unwrap_or(true);

        let discover_request = create_discover_request(
            target.as_ref(),
            &context_pairs,
            limit,
            filter.as_ref(),
            with_payload,
            with_vector,
        )?;

        match client.discover_points(&collection, &discover_request) {
            Ok(response) => scored_points_to_search_results(&response.result),
            Err(e) => Err(e),
        }
    }

    fn search_groups(
        collection: String,
        query: SearchQuery,
        group_by: String,
        group_size: u32,
        max_groups: u32,
        filter: Option<FilterExpression>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<Vec<GroupedSearchResult>, VectorError> {
        let search_limit = (max_groups * group_size).max(1000);

        let search_results = Self::search_vectors(
            collection,
            query,
            search_limit,
            filter,
            namespace,
            include_vectors,
            include_metadata,
            None,
            None,
            None,
        )?;

        use std::collections::HashMap;
        let mut groups: HashMap<String, Vec<SearchResult>> = HashMap::new();

        for result in search_results {
            let group_key = if let Some(ref metadata) = result.metadata {
                metadata
                    .iter()
                    .find(|(key, _)| key == &group_by)
                    .map(|(_, value)| match value {
                        MetadataValue::StringVal(s) => s.clone(),
                        MetadataValue::IntegerVal(i) => i.to_string(),
                        MetadataValue::NumberVal(f) => f.to_string(),
                        MetadataValue::BooleanVal(b) => b.to_string(),
                        _ => "other".to_string(),
                    })
                    .unwrap_or_else(|| "null".to_string())
            } else {
                "null".to_string()
            };

            let group = groups.entry(group_key).or_default();
            if group.len() < group_size as usize {
                group.push(result);
            }
        }

        let mut grouped_results: Vec<GroupedSearchResult> = groups
            .into_iter()
            .take(max_groups as usize)
            .map(|(group_value, results)| {
                let count = results.len() as u32;
                GroupedSearchResult {
                    group_value: MetadataValue::StringVal(group_value),
                    results,
                    group_count: count,
                }
            })
            .collect();

        grouped_results.sort_by(|a, b| {
            let a_best_score = a.results.iter().map(|r| r.score).fold(0.0f32, f32::max);
            let b_best_score = b.results.iter().map(|r| r.score).fold(0.0f32, f32::max);
            b_best_score
                .partial_cmp(&a_best_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(grouped_results)
    }

    fn search_range(
        collection: String,
        vector: VectorData,
        min_distance: Option<f32>,
        max_distance: f32,
        filter: Option<FilterExpression>,
        namespace: Option<String>,
        limit: Option<u32>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        let query = SearchQuery::Vector(vector);
        Self::search_vectors(
            collection,
            query,
            limit.unwrap_or(10000),
            filter,
            namespace,
            include_vectors,
            include_metadata,
            min_distance,
            Some(max_distance),
            None,
        )
    }

    fn search_text(
        _collection: String,
        _query: String,
        _limit: u32,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        Err(VectorError::ProviderError(
            "Text search not directly supported in Qdrant".to_string(),
        ))
    }
}

impl AnalyticsGuest for QdrantComponent {
    fn get_collection_stats(
        collection: String,
        _namespace: Option<String>,
    ) -> Result<CollectionStats, VectorError> {
        let client = Self::create_client()?;

        match client.get_collection(&collection) {
            Ok(response) => {
                let info = response.result;

                Ok(CollectionStats {
                    vector_count: info.points_count.unwrap_or(0),
                    dimension: if let Some(vectors) = &info.config.vectors {
                        match vectors {
                            client::VectorConfig::Single(params) => params.size,
                            client::VectorConfig::Multiple(map) => {
                                map.values().next().map(|params| params.size).unwrap_or(0)
                            }
                        }
                    } else if let Some(params) = &info.config.params {
                        if let Some(vectors) = &params.vectors {
                            match vectors {
                                client::VectorConfig::Single(params) => params.size,
                                client::VectorConfig::Multiple(map) => {
                                    map.values().next().map(|params| params.size).unwrap_or(0)
                                }
                            }
                        } else {
                            0
                        }
                    } else {
                        0
                    },
                    size_bytes: 0,
                    index_size_bytes: None,
                    namespace_stats: vec![],
                    distance_distribution: None,
                })
            }
            Err(e) => Err(e),
        }
    }

    fn get_field_stats(
        _collection: String,
        _field_name: String,
        _namespace: Option<String>,
    ) -> Result<FieldStats, VectorError> {
        Err(VectorError::ProviderError(
            "Field stats not directly available in Qdrant".to_string(),
        ))
    }

    fn get_field_distribution(
        _collection: String,
        _field_name: String,
        _limit: Option<u32>,
        _namespace: Option<String>,
    ) -> Result<Vec<(MetadataValue, u64)>, VectorError> {
        Err(VectorError::ProviderError(
            "Field distribution not directly available in Qdrant".to_string(),
        ))
    }
}

impl NamespacesGuest for QdrantComponent {
    fn upsert_namespace(
        _collection: String,
        _name: String,
        _metadata: Option<Metadata>,
    ) -> Result<NamespaceInfo, VectorError> {
        Err(VectorError::ProviderError(
            "Namespaces not supported in Qdrant".to_string(),
        ))
    }

    fn list_namespaces(_collection: String) -> Result<Vec<NamespaceInfo>, VectorError> {
        Err(VectorError::ProviderError(
            "Namespaces not supported in Qdrant".to_string(),
        ))
    }

    fn get_namespace(
        _collection: String,
        _namespace: String,
    ) -> Result<NamespaceInfo, VectorError> {
        Err(VectorError::ProviderError(
            "Namespaces not supported in Qdrant".to_string(),
        ))
    }

    fn delete_namespace(_collection: String, _namespace: String) -> Result<(), VectorError> {
        Err(VectorError::ProviderError(
            "Namespaces not supported in Qdrant".to_string(),
        ))
    }

    fn namespace_exists(_collection: String, _namespace: String) -> Result<bool, VectorError> {
        Ok(false)
    }
}

type DurableQdrantComponent = DurableVector<QdrantComponent>;

golem_vector::export_vector!(DurableQdrantComponent with_types_in golem_vector);

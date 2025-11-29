use crate::client::PineconeClient;
use crate::conversions::{
    extract_dense_and_sparse_from_query, extract_prefix_from_filter,
    filter_expression_to_pinecone_filter, index_model_to_collection_info,
    pinecone_query_response_to_search_results, pinecone_vector_to_vector_record,
    vector_records_to_upsert_request,
};
use golem_vector::config::with_config_key;
use golem_vector::durability::{DurableVector, ExtendedGuest};
use golem_vector::golem::vector::{
    analytics::{CollectionStats, FieldStats, Guest as AnalyticsGuest, NamespaceStats},
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

struct PineconeComponent;

impl PineconeComponent {
    const API_KEY_ENV_VAR: &'static str = "PINECONE_API_KEY";
    const ENVIRONMENT_ENV_VAR: &'static str = "PINECONE_ENVIRONMENT";

    fn create_client() -> Result<PineconeClient, VectorError> {
        let api_key = with_config_key(
            Self::API_KEY_ENV_VAR,
            |e| {
                Err(VectorError::ConnectionError(format!(
                    "Missing API key: {e}"
                )))
            },
            Ok,
        )?;

        let environment = golem_vector::config::get_optional_config(Self::ENVIRONMENT_ENV_VAR);

        Ok(PineconeClient::new(api_key, environment))
    }
}

impl ExtendedGuest for PineconeComponent {
    fn connect_internal(
        _endpoint: &str,
        _credentials: &Option<Credentials>,
        _timeout_ms: &Option<u32>,
        _options: &Option<Metadata>,
    ) -> Result<(), VectorError> {
        let _client = Self::create_client()?;
        Ok(())
    }
}

impl ConnectionGuest for PineconeComponent {
    fn connect(
        _endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        _options: Option<Metadata>,
    ) -> Result<(), VectorError> {
        let _client = Self::create_client()?;
        Ok(())
    }

    fn disconnect() -> Result<(), VectorError> {
        Ok(())
    }

    fn get_connection_status() -> Result<ConnectionStatus, VectorError> {
        match Self::create_client() {
            Ok(_) => Ok(ConnectionStatus {
                connected: true,
                provider: Some("pinecone".to_string()),
                endpoint: Some("https://api.pinecone.io".to_string()),
                last_activity: None,
                connection_id: Some("pinecone-api".to_string()),
            }),
            Err(_) => Ok(ConnectionStatus {
                connected: false,
                provider: Some("pinecone".to_string()),
                endpoint: Some("https://api.pinecone.io".to_string()),
                last_activity: None,
                connection_id: Some("pinecone-api".to_string()),
            }),
        }
    }

    fn test_connection(
        _endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        _options: Option<Metadata>,
    ) -> Result<bool, VectorError> {
        match Self::create_client() {
            Ok(client) => match client.list_indexes() {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            },
            Err(_) => Ok(false),
        }
    }
}

impl CollectionsGuest for PineconeComponent {
    fn upsert_collection(
        name: String,
        _description: Option<String>,
        dimension: u32,
        metric: DistanceMetric,
        _index_config: Option<IndexConfig>,
        metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        let client = Self::create_client()?;

        let (cloud, region) = if let Some(meta) = &metadata {
            let cloud = meta
                .iter()
                .find(|(k, _)| k == "cloud")
                .and_then(|(_, v)| match v {
                    MetadataValue::StringVal(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| "aws".to_string());

            let region = meta
                .iter()
                .find(|(k, _)| k == "region")
                .and_then(|(_, v)| match v {
                    MetadataValue::StringVal(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| "us-east-1".to_string());

            (cloud, region)
        } else {
            ("aws".to_string(), "us-east-1".to_string())
        };

        let create_request = client::CreateIndexRequest {
            name: name.clone(),
            dimension,
            metric: conversions::distance_metric_to_string(&metric),
            spec: client::IndexSpec::Serverless(client::ServerlessSpec {
                serverless: client::ServerlessConfig { cloud, region },
            }),
            deletion_protection: Some(client::DeletionProtection::Disabled),
        };

        match client.create_index(&create_request) {
            Ok(index_model) => index_model_to_collection_info(&index_model),
            Err(e) => Err(e),
        }
    }

    fn list_collections() -> Result<Vec<String>, VectorError> {
        let client = Self::create_client()?;

        match client.list_indexes() {
            Ok(response) => Ok(response
                .indexes
                .iter()
                .map(|idx| idx.name.clone())
                .collect()),
            Err(e) => Err(e),
        }
    }

    fn get_collection(name: String) -> Result<CollectionInfo, VectorError> {
        let client = Self::create_client()?;

        match client.describe_index(&name) {
            Ok(index_model) => index_model_to_collection_info(&index_model),
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

        match client.delete_index(&name) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn collection_exists(name: String) -> Result<bool, VectorError> {
        match Self::get_collection(name) {
            Ok(_) => Ok(true),
            Err(VectorError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }
}

impl VectorsGuest for PineconeComponent {
    fn upsert_vectors(
        collection: String,
        vectors: Vec<VectorRecord>,
        namespace: Option<String>,
    ) -> Result<BatchResult, VectorError> {
        let client = Self::create_client()?;

        let upsert_request = vector_records_to_upsert_request(&vectors, namespace)?;

        match client.upsert_vectors(&collection, &upsert_request) {
            Ok(response) => Ok(BatchResult {
                success_count: response.upserted_count,
                failure_count: 0,
                errors: vec![],
            }),
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
        namespace: Option<String>,
        _include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
    ) -> Result<Vec<VectorRecord>, VectorError> {
        let client = Self::create_client()?;

        let fetch_request = client::FetchRequest { ids, namespace };

        match client.fetch_vectors(&collection, &fetch_request) {
            Ok(response) => {
                let mut records = Vec::new();
                for (id, vector) in response.vectors {
                    records.push(pinecone_vector_to_vector_record(&client::Vector {
                        id,
                        values: vector.values,
                        metadata: vector.metadata,
                        sparse_values: vector.sparse_values,
                    })?);
                }
                Ok(records)
            }
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
        _merge_metadata: Option<bool>,
    ) -> Result<(), VectorError> {
        if let Some(vector_data) = vector {
            Self::upsert_vector(collection, id, vector_data, metadata, namespace)
        } else {
            Err(VectorError::InvalidParams(
                "Vector data is required for update".to_string(),
            ))
        }
    }

    fn delete_vectors(
        collection: String,
        ids: Vec<Id>,
        namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        let client = Self::create_client()?;

        let delete_request = client::DeleteRequest {
            ids: Some(ids.clone()),
            delete_all: None,
            namespace,
            filter: None,
        };

        match client.delete_vectors(&collection, &delete_request) {
            Ok(_) => Ok(ids.len() as u32),
            Err(e) => Err(e),
        }
    }

    fn delete_by_filter(
        collection: String,
        filter: FilterExpression,
        namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        let client = Self::create_client()?;

        let pinecone_filter = filter_expression_to_pinecone_filter(&filter)?;

        let delete_request = client::DeleteRequest {
            ids: None,
            delete_all: None,
            namespace,
            filter: Some(pinecone_filter),
        };

        match client.delete_vectors(&collection, &delete_request) {
            Ok(_) => Ok(0),
            Err(e) => Err(e),
        }
    }

    fn delete_namespace(collection: String, namespace: String) -> Result<u32, VectorError> {
        let client = Self::create_client()?;

        let delete_request = client::DeleteRequest {
            ids: None,
            delete_all: Some(true),
            namespace: Some(namespace),
            filter: None,
        };

        match client.delete_vectors(&collection, &delete_request) {
            Ok(_) => Ok(0),
            Err(e) => Err(e),
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
        let client = Self::create_client()?;

        let prefix = if let Some(filter_expr) = &filter {
            extract_prefix_from_filter(filter_expr)
        } else {
            None
        };

        if filter.is_some() && prefix.is_none() {
            return Err(VectorError::UnsupportedFeature(
                "Filtering not supported in Pinecone list_vectors (serverless only supports list by prefix on 'id' field)".to_string()
            ));
        }

        let list_request = client::ListVectorIdsRequest {
            prefix,
            limit: limit.or(Some(100)),
            pagination_token: cursor,
            namespace: namespace.clone(),
        };

        match client.list_vector_ids(&collection, &list_request) {
            Ok(list_response) => {
                let vector_ids: Vec<String> =
                    list_response.vectors.iter().map(|v| v.id.clone()).collect();

                let mut vector_records = Vec::new();

                if !vector_ids.is_empty() {
                    if include_vectors.unwrap_or(false) || include_metadata.unwrap_or(false) {
                        const FETCH_BATCH_SIZE: usize = 100; // pinecone fetch limit

                        for chunk in vector_ids.chunks(FETCH_BATCH_SIZE) {
                            let fetch_request = client::FetchRequest {
                                ids: chunk.to_vec(),
                                namespace: namespace.clone(),
                            };

                            match client.fetch_vectors(&collection, &fetch_request) {
                                Ok(fetch_response) => {
                                    for (id, vector) in fetch_response.vectors {
                                        let record =
                                            pinecone_vector_to_vector_record(&client::Vector {
                                                id: id.clone(),
                                                values: if include_vectors.unwrap_or(false) {
                                                    vector.values
                                                } else {
                                                    None
                                                },
                                                metadata: if include_metadata.unwrap_or(false) {
                                                    vector.metadata
                                                } else {
                                                    None
                                                },
                                                sparse_values: if include_vectors.unwrap_or(false) {
                                                    vector.sparse_values
                                                } else {
                                                    None
                                                },
                                            })?;
                                        vector_records.push(record);
                                    }
                                }
                                Err(e) => {
                                    return Err(e);
                                }
                            }
                        }
                    } else {
                        for id in vector_ids {
                            vector_records.push(VectorRecord {
                                id,
                                vector: VectorData::Dense(vec![]),
                                metadata: None,
                            });
                        }
                    }
                }

                let next_cursor = list_response.pagination.and_then(|p| p.next);

                Ok(ListResponse {
                    vectors: vector_records,
                    next_cursor,
                    total_count: None,
                })
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("404") || error_msg.contains("not found") {
                    Err(VectorError::UnsupportedFeature(
                        "List vectors is only supported on Pinecone serverless indexes".to_string(),
                    ))
                } else {
                    Err(e)
                }
            }
        }
    }

    fn count_vectors(
        collection: String,
        _filter: Option<FilterExpression>,
        namespace: Option<String>,
    ) -> Result<u64, VectorError> {
        let client = Self::create_client()?;

        let stats_request = client::DescribeIndexStatsRequest { filter: None };

        match client.describe_index_stats(&collection, &stats_request) {
            Ok(stats) => {
                if let Some(ns) = namespace {
                    if let Some(namespace_stats) = stats.namespaces.get(&ns) {
                        Ok(namespace_stats.vector_count)
                    } else {
                        Ok(0)
                    }
                } else {
                    Ok(stats.total_vector_count)
                }
            }
            Err(e) => Err(e),
        }
    }
}

impl SearchGuest for PineconeComponent {
    fn search_vectors(
        collection: String,
        query: SearchQuery,
        limit: u32,
        filter: Option<FilterExpression>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
        min_score: Option<f32>,
        max_distance: Option<f32>,
        _search_params: Option<Vec<(String, String)>>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        let client = Self::create_client()?;

        let (query_vector, sparse_vector, query_id) = match &query {
            SearchQuery::Vector(_) => {
                let (dense, sparse) = extract_dense_and_sparse_from_query(&query);
                (dense, sparse, None)
            }
            SearchQuery::ById(id) => (None, None, Some(id.clone())),
            SearchQuery::MultiVector(_) => {
                let (dense, sparse) = extract_dense_and_sparse_from_query(&query);
                if dense.is_none() && sparse.is_none() {
                    return Err(VectorError::InvalidParams(
                        "Multi-vector query is empty".to_string(),
                    ));
                }
                (dense, sparse, None)
            }
        };

        let pinecone_filter = if let Some(filter_expr) = filter {
            Some(filter_expression_to_pinecone_filter(&filter_expr)?)
        } else {
            None
        };

        let query_request = client::QueryRequest {
            namespace,
            top_k: limit,
            filter: pinecone_filter,
            include_values: include_vectors,
            include_metadata,
            vector: query_vector,
            sparse_vector,
            id: query_id,
        };

        match client.query_vectors(&collection, &query_request) {
            Ok(response) => {
                let mut results = pinecone_query_response_to_search_results(response);

                if let Some(min_score_val) = min_score {
                    results.retain(|result| result.score >= min_score_val);
                }

                if let Some(max_distance_val) = max_distance {
                    results.retain(|result| result.distance <= max_distance_val);
                }

                Ok(results)
            }
            Err(e) => Err(e),
        }
    }

    fn find_similar(
        collection: String,
        vector: VectorData,
        limit: u32,
        namespace: Option<String>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        Self::search_vectors(
            collection,
            SearchQuery::Vector(vector),
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
        namespace: Option<String>,
        include_vectors: Option<bool>,
        include_metadata: Option<bool>,
        search_params: Option<Vec<(String, String)>>,
    ) -> Result<Vec<Vec<SearchResult>>, VectorError> {
        let mut results = Vec::new();

        for query in queries {
            let result = Self::search_vectors(
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
            )?;
            results.push(result);
        }

        Ok(results)
    }
}

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
        Err(VectorError::UnsupportedFeature(
            "Recommendation search not supported by Pinecone".to_string(),
        ))
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
        Err(VectorError::UnsupportedFeature(
            "Discovery search not supported by Pinecone".to_string(),
        ))
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
        Err(VectorError::UnsupportedFeature(
            "Grouped search not supported by Pinecone".to_string(),
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
        Err(VectorError::UnsupportedFeature(
            "Range search not supported by Pinecone".to_string(),
        ))
    }

    fn search_text(
        _collection: String,
        _query_text: String,
        _limit: u32,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        Err(VectorError::UnsupportedFeature(
            "Text search not supported by Pinecone".to_string(),
        ))
    }
}

impl AnalyticsGuest for PineconeComponent {
    fn get_collection_stats(
        collection: String,
        namespace: Option<String>,
    ) -> Result<CollectionStats, VectorError> {
        let client = Self::create_client()?;

        let stats_request = client::DescribeIndexStatsRequest { filter: None };

        match client.describe_index_stats(&collection, &stats_request) {
            Ok(stats) => {
                let namespace_stats = if let Some(ns) = namespace {
                    stats.namespaces.get(&ns).cloned().unwrap_or_default()
                } else {
                    client::NamespaceSummary {
                        vector_count: stats.total_vector_count,
                    }
                };

                let ns_stats: Vec<(String, NamespaceStats)> = stats
                    .namespaces
                    .iter()
                    .map(|(name, summary)| {
                        (
                            name.clone(),
                            NamespaceStats {
                                vector_count: summary.vector_count,
                                size_bytes: 0,
                            },
                        )
                    })
                    .collect();

                Ok(CollectionStats {
                    vector_count: namespace_stats.vector_count,
                    dimension: stats.dimension,
                    size_bytes: 0,
                    index_size_bytes: None,
                    namespace_stats: ns_stats,
                    distance_distribution: None,
                })
            }
            Err(e) => Err(e),
        }
    }

    fn get_field_stats(
        _collection: String,
        _field: String,
        _namespace: Option<String>,
    ) -> Result<FieldStats, VectorError> {
        Err(VectorError::UnsupportedFeature(
            "Field stats not supported by Pinecone".to_string(),
        ))
    }

    fn get_field_distribution(
        _collection: String,
        _field: String,
        _limit: Option<u32>,
        _namespace: Option<String>,
    ) -> Result<Vec<(MetadataValue, u64)>, VectorError> {
        Err(VectorError::UnsupportedFeature(
            "Field distribution not supported by Pinecone".to_string(),
        ))
    }
}

impl NamespacesGuest for PineconeComponent {
    fn upsert_namespace(
        collection: String,
        namespace: String,
        _metadata: Option<Metadata>,
    ) -> Result<NamespaceInfo, VectorError> {
        Ok(NamespaceInfo {
            name: namespace,
            collection,
            vector_count: 0,
            size_bytes: 0,
            created_at: None,
            metadata: None,
        })
    }

    fn list_namespaces(collection: String) -> Result<Vec<NamespaceInfo>, VectorError> {
        let client = Self::create_client()?;

        match client.list_namespaces(&collection) {
            Ok(namespaces) => {
                let mut namespace_infos = Vec::new();
                for ns in namespaces.namespaces {
                    namespace_infos.push(golem_vector::golem::vector::namespaces::NamespaceInfo {
                        name: ns.name,
                        collection: collection.clone(),
                        vector_count: ns.record_count,
                        size_bytes: 0,
                        created_at: None,
                        metadata: None,
                    });
                }
                Ok(namespace_infos)
            }
            Err(e) => Err(e),
        }
    }

    fn get_namespace(collection: String, namespace: String) -> Result<NamespaceInfo, VectorError> {
        let client = Self::create_client()?;

        let stats_request = client::DescribeIndexStatsRequest { filter: None };

        match client.describe_index_stats(&collection, &stats_request) {
            Ok(stats) => {
                if let Some(ns_stats) = stats.namespaces.get(&namespace) {
                    Ok(NamespaceInfo {
                        name: namespace,
                        collection,
                        vector_count: ns_stats.vector_count,
                        size_bytes: 0,
                        created_at: None,
                        metadata: None,
                    })
                } else {
                    Err(VectorError::NotFound(format!(
                        "Namespace {} not found",
                        namespace
                    )))
                }
            }
            Err(e) => Err(e),
        }
    }

    fn delete_namespace(collection: String, namespace: String) -> Result<(), VectorError> {
        <Self as VectorsGuest>::delete_namespace(collection, namespace)?;
        Ok(())
    }

    fn namespace_exists(collection: String, namespace: String) -> Result<bool, VectorError> {
        match Self::get_namespace(collection, namespace) {
            Ok(_) => Ok(true),
            Err(VectorError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }
}

type DurablePineconeComponent = DurableVector<PineconeComponent>;

golem_vector::export_vector!(DurablePineconeComponent with_types_in golem_vector);

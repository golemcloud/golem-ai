use crate::client::MilvusClient;
use crate::conversions::{
    collection_info_to_export_collection_info, collection_stats_to_export_stats,
    create_delete_request, create_get_request, create_query_request, create_search_request,
    distance_metric_to_string, milvus_entities_to_vector_records,
    milvus_search_results_to_search_results, vector_records_to_upsert_request, QueryRequestParams,
    SearchRequestParams,
};
use golem_ai_vector::durability::{DurableVector, ExtendedVectorProvider};
use golem_ai_vector::model::{
    analytics::{CollectionStats, FieldStats},
    collections::{CollectionInfo, IndexConfig},
    connection::{ConnectionStatus, Credentials},
    namespaces::NamespaceInfo,
    search::SearchQuery,
    search_extended::{
        ContextPair, GroupedSearchResult, RecommendationExample, RecommendationStrategy,
    },
    types::{
        DistanceMetric, FilterExpression, Id, Metadata, MetadataValue, SearchResult, VectorData,
        VectorError, VectorRecord,
    },
    vectors::{BatchResult, ListResponse},
};
use golem_ai_vector::{
    AnalyticsProvider, CollectionProvider, ConnectionProvider, NamespacesProvider,
    SearchExtendedProvider, SearchProvider, VectorsProvider,
};

mod client;
pub mod config;
mod conversions;

pub use crate::config::MilvusConfig;
#[cfg(feature = "golem")]
pub use crate::config::MilvusHostConfig;

pub struct Milvus;

impl ExtendedVectorProvider for Milvus {
    fn connect_internal(
        provider_config: <Self as ConnectionProvider>::ProviderConfig,
        _endpoint: &str,
        _credentials: &Option<Credentials>,
        _timeout_ms: &Option<u32>,
        _options: &Option<Metadata>,
    ) -> Result<(), VectorError> {
        let _client = MilvusClient::new(&provider_config);
        Ok(())
    }
}

impl ConnectionProvider for Milvus {
    type ProviderConfig = MilvusConfig;

    fn connect(
        provider_config: Self::ProviderConfig,
        _endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        _options: Option<Metadata>,
    ) -> Result<(), VectorError> {
        let _client = MilvusClient::new(&provider_config);
        Ok(())
    }

    fn disconnect(_provider_config: Self::ProviderConfig) -> Result<(), VectorError> {
        Ok(())
    }

    fn get_connection_status(
        provider_config: Self::ProviderConfig,
    ) -> Result<ConnectionStatus, VectorError> {
        let client = MilvusClient::new(&provider_config);
        match client.list_collections() {
            Ok(_) => Ok(ConnectionStatus {
                connected: true,
                provider: Some("milvus".to_string()),
                endpoint: Some(client.base_url().to_string()),
                last_activity: None,
                connection_id: Some("milvus-api".to_string()),
            }),
            Err(_) => Ok(ConnectionStatus {
                connected: false,
                provider: Some("milvus".to_string()),
                endpoint: Some(client.base_url().to_string()),
                last_activity: None,
                connection_id: Some("milvus-api".to_string()),
            }),
        }
    }

    fn test_connection(
        provider_config: Self::ProviderConfig,
        _endpoint: String,
        _credentials: Option<Credentials>,
        _timeout_ms: Option<u32>,
        _options: Option<Metadata>,
    ) -> Result<bool, VectorError> {
        let client = MilvusClient::new(&provider_config);
        match client.list_collections() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

impl CollectionProvider for Milvus {
    type ProviderConfig = MilvusConfig;

    fn upsert_collection(
        provider_config: Self::ProviderConfig,
        name: String,
        description: Option<String>,
        dimension: u32,
        metric: DistanceMetric,
        _index_config: Option<IndexConfig>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        let client = MilvusClient::new(&provider_config);

        let create_request = client::CreateCollectionRequest {
            db_name: client.database().to_string(),
            collection_name: name.clone(),
            dimension,
            metric_type: Some(distance_metric_to_string(&metric)),
            primary_field: Some("id".to_string()),
            vector_field: Some("vector".to_string()),
            description,
            enable_dynamic_field: Some(true),
            schema: None,
            index_params: None,
            vector_field_type: Some("FloatVector".to_string()),
        };

        match client.create_collection(&create_request) {
            Ok(_) => match client.load_collection(&name) {
                Ok(_) => match client.describe_collection(&name) {
                    Ok(response) => collection_info_to_export_collection_info(&response.data),
                    Err(e) => Err(e),
                },
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        }
    }

    fn list_collections(
        provider_config: Self::ProviderConfig,
    ) -> Result<Vec<String>, VectorError> {
        let client = MilvusClient::new(&provider_config);

        match client.list_collections() {
            Ok(response) => Ok(response.data),
            Err(e) => Err(e),
        }
    }

    fn get_collection(
        provider_config: Self::ProviderConfig,
        name: String,
    ) -> Result<CollectionInfo, VectorError> {
        let client = MilvusClient::new(&provider_config);

        match client.describe_collection(&name) {
            Ok(response) => collection_info_to_export_collection_info(&response.data),
            Err(e) => Err(e),
        }
    }

    fn update_collection(
        provider_config: Self::ProviderConfig,
        name: String,
        _description: Option<String>,
        _metadata: Option<Metadata>,
    ) -> Result<CollectionInfo, VectorError> {
        Self::get_collection(provider_config, name)
    }

    fn delete_collection(
        provider_config: Self::ProviderConfig,
        name: String,
    ) -> Result<(), VectorError> {
        let client = MilvusClient::new(&provider_config);

        let _ = client.release_collection(&name);

        match client.drop_collection(&name) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn collection_exists(
        provider_config: Self::ProviderConfig,
        name: String,
    ) -> Result<bool, VectorError> {
        let client = MilvusClient::new(&provider_config);

        match client.has_collection(&name) {
            Ok(response) => Ok(response.data.has),
            Err(_) => Ok(false),
        }
    }
}

impl VectorsProvider for Milvus {
    type ProviderConfig = MilvusConfig;

    fn upsert_vectors(
        provider_config: Self::ProviderConfig,
        collection: String,
        vectors: Vec<VectorRecord>,
        namespace: Option<String>,
    ) -> Result<BatchResult, VectorError> {
        let client = MilvusClient::new(&provider_config);

        let upsert_request = vector_records_to_upsert_request(
            &collection,
            client.database(),
            &vectors,
            namespace.as_deref(),
        )?;

        match client.upsert(&upsert_request) {
            Ok(response) => Ok(BatchResult {
                success_count: response.data.upsert_count,
                failure_count: 0,
                errors: vec![],
            }),
            Err(e) => Err(e),
        }
    }

    fn upsert_vector(
        provider_config: Self::ProviderConfig,
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

        let result = Self::upsert_vectors(provider_config, collection, vec![record], namespace)?;

        if result.success_count > 0 {
            Ok(())
        } else {
            Err(VectorError::ProviderError(
                "Failed to upsert vector".to_string(),
            ))
        }
    }

    fn get_vectors(
        provider_config: Self::ProviderConfig,
        collection: String,
        ids: Vec<Id>,
        _namespace: Option<String>,
        include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
    ) -> Result<Vec<VectorRecord>, VectorError> {
        let client = MilvusClient::new(&provider_config);

        let mut output_fields = Vec::new();
        if include_vectors.unwrap_or(true) {
            output_fields.push("vector".to_string());
        }

        let get_request = create_get_request(
            &collection,
            client.database(),
            &ids,
            if output_fields.is_empty() {
                None
            } else {
                Some(&output_fields)
            },
        );

        match client.get(&get_request) {
            Ok(response) => milvus_entities_to_vector_records(&response.data),
            Err(e) => Err(e),
        }
    }

    fn get_vector(
        provider_config: Self::ProviderConfig,
        collection: String,
        id: Id,
        namespace: Option<String>,
    ) -> Result<Option<VectorRecord>, VectorError> {
        let vectors = Self::get_vectors(
            provider_config,
            collection,
            vec![id],
            namespace,
            Some(true),
            Some(true),
        )?;
        Ok(vectors.into_iter().next())
    }

    fn update_vector(
        provider_config: Self::ProviderConfig,
        collection: String,
        id: Id,
        vector: Option<VectorData>,
        metadata: Option<Metadata>,
        namespace: Option<String>,
        _merge_metadata: Option<bool>,
    ) -> Result<(), VectorError> {
        if let Some(vector_data) = vector {
            Self::upsert_vector(provider_config, collection, id, vector_data, metadata, namespace)
        } else {
            Err(VectorError::InvalidParams(
                "Vector data is required for update".to_string(),
            ))
        }
    }

    fn delete_vectors(
        provider_config: Self::ProviderConfig,
        collection: String,
        ids: Vec<Id>,
        namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        let client = MilvusClient::new(&provider_config);

        let delete_request = create_delete_request(
            &collection,
            client.database(),
            Some(&ids),
            None,
            namespace.as_deref(),
        )?;

        match client.delete(&delete_request) {
            Ok(_response) => Ok(ids.len() as u32),
            Err(e) => Err(e),
        }
    }

    fn delete_by_filter(
        provider_config: Self::ProviderConfig,
        collection: String,
        filter: FilterExpression,
        namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        let client = MilvusClient::new(&provider_config);

        let delete_request = create_delete_request(
            &collection,
            client.database(),
            None,
            Some(&filter),
            namespace.as_deref(),
        )?;

        match client.delete(&delete_request) {
            Ok(_response) => Ok(0),
            Err(e) => Err(e),
        }
    }

    fn delete_namespace(
        _provider_config: Self::ProviderConfig,
        _collection: String,
        _namespace: String,
    ) -> Result<u32, VectorError> {
        Err(VectorError::UnsupportedFeature(
            "Milvus doesn't support namespaces like Pinecone".to_string(),
        ))
    }

    fn list_vectors(
        provider_config: Self::ProviderConfig,
        collection: String,
        namespace: Option<String>,
        filter: Option<FilterExpression>,
        limit: Option<u32>,
        _cursor: Option<String>,
        include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
    ) -> Result<ListResponse, VectorError> {
        let client = MilvusClient::new(&provider_config);

        let mut output_fields = vec!["id".to_string()];
        if include_vectors.unwrap_or(false) {
            output_fields.push("vector".to_string());
        }

        let query_request = create_query_request(QueryRequestParams {
            collection_name: &collection,
            db_name: client.database(),
            ids: None,
            filter: filter.as_ref(),
            output_fields: if output_fields.len() == 1 {
                None
            } else {
                Some(&output_fields)
            },
            limit,
            offset: None,
            partition_names: namespace.map(|ns| vec![ns]),
        })?;

        match client.query(&query_request) {
            Ok(response) => {
                let vector_records = milvus_entities_to_vector_records(&response.data)?;

                Ok(ListResponse {
                    vectors: vector_records,
                    next_cursor: None,
                    total_count: None,
                })
            }
            Err(e) => Err(e),
        }
    }

    fn count_vectors(
        provider_config: Self::ProviderConfig,
        collection: String,
        filter: Option<FilterExpression>,
        namespace: Option<String>,
    ) -> Result<u64, VectorError> {
        let client = MilvusClient::new(&provider_config);

        if filter.is_some() {
            let query_request = create_query_request(QueryRequestParams {
                collection_name: &collection,
                db_name: client.database(),
                ids: None,
                filter: filter.as_ref(),
                output_fields: Some(&["id".to_string()]),
                limit: None,
                offset: None,
                partition_names: namespace.map(|ns| vec![ns]),
            })?;

            match client.query(&query_request) {
                Ok(response) => Ok(response.data.len() as u64),
                Err(e) => Err(e),
            }
        } else {
            match client.get_collection_stats(&collection) {
                Ok(response) => Ok(response.data.row_count),
                Err(e) => Err(e),
            }
        }
    }
}

impl SearchProvider for Milvus {
    type ProviderConfig = MilvusConfig;

    fn search_vectors(
        provider_config: Self::ProviderConfig,
        collection: String,
        query: SearchQuery,
        limit: u32,
        filter: Option<FilterExpression>,
        namespace: Option<String>,
        include_vectors: Option<bool>,
        _include_metadata: Option<bool>,
        min_score: Option<f32>,
        max_distance: Option<f32>,
        _search_params: Option<Vec<(String, String)>>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        let client = MilvusClient::new(&provider_config);

        let mut output_fields = vec!["id".to_string()];
        if include_vectors.unwrap_or(false) {
            output_fields.push("vector".to_string());
        }

        let search_request = create_search_request(SearchRequestParams {
            collection_name: &collection,
            db_name: client.database(),
            query: &query,
            limit,
            filter: filter.as_ref(),
            output_fields: if output_fields.len() == 1 {
                None
            } else {
                Some(&output_fields)
            },
            anns_field: "vector",
            metric_type: "COSINE",
            partition_names: namespace.map(|ns| vec![ns]),
        })?;

        match client.search(&search_request) {
            Ok(response) => {
                let mut results = milvus_search_results_to_search_results(&response.data)?;

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
        provider_config: Self::ProviderConfig,
        collection: String,
        vector: VectorData,
        limit: u32,
        namespace: Option<String>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        Self::search_vectors(
            provider_config,
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
        provider_config: Self::ProviderConfig,
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
                provider_config.clone(),
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

impl SearchExtendedProvider for Milvus {
    type ProviderConfig = MilvusConfig;

    fn recommend_vectors(
        _provider_config: Self::ProviderConfig,
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
            "Recommendation search not supported by Milvus".to_string(),
        ))
    }

    fn discover_vectors(
        _provider_config: Self::ProviderConfig,
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
            "Discovery search not supported by Milvus".to_string(),
        ))
    }

    fn search_groups(
        _provider_config: Self::ProviderConfig,
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
            "Grouped search not supported by Milvus".to_string(),
        ))
    }

    fn search_range(
        _provider_config: Self::ProviderConfig,
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
            "Range search not supported by Milvus".to_string(),
        ))
    }

    fn search_text(
        _provider_config: Self::ProviderConfig,
        _collection: String,
        _query_text: String,
        _limit: u32,
        _filter: Option<FilterExpression>,
        _namespace: Option<String>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        Err(VectorError::UnsupportedFeature(
            "Text search not supported by Milvus".to_string(),
        ))
    }
}

impl AnalyticsProvider for Milvus {
    type ProviderConfig = MilvusConfig;

    fn get_collection_stats(
        provider_config: Self::ProviderConfig,
        collection: String,
        _namespace: Option<String>,
    ) -> Result<CollectionStats, VectorError> {
        let client = MilvusClient::new(&provider_config);

        match client.get_collection_stats(&collection) {
            Ok(response) => Ok(collection_stats_to_export_stats(&response.data)),
            Err(e) => Err(e),
        }
    }

    fn get_field_stats(
        _provider_config: Self::ProviderConfig,
        _collection: String,
        _field: String,
        _namespace: Option<String>,
    ) -> Result<FieldStats, VectorError> {
        Err(VectorError::UnsupportedFeature(
            "Field stats not supported by Milvus".to_string(),
        ))
    }

    fn get_field_distribution(
        _provider_config: Self::ProviderConfig,
        _collection: String,
        _field: String,
        _limit: Option<u32>,
        _namespace: Option<String>,
    ) -> Result<Vec<(MetadataValue, u64)>, VectorError> {
        Err(VectorError::UnsupportedFeature(
            "Field distribution not supported by Milvus".to_string(),
        ))
    }
}

impl NamespacesProvider for Milvus {
    type ProviderConfig = MilvusConfig;

    fn upsert_namespace(
        provider_config: Self::ProviderConfig,
        collection: String,
        namespace: String,
        _metadata: Option<Metadata>,
    ) -> Result<NamespaceInfo, VectorError> {
        let client = MilvusClient::new(&provider_config);

        match client.has_partition(&collection, &namespace) {
            Ok(response) => {
                if response.data.has {
                    Ok(NamespaceInfo {
                        name: namespace,
                        collection,
                        created_at: None,
                        vector_count: 0,
                        size_bytes: 0,
                        metadata: None,
                    })
                } else {
                    match client.create_partition(&collection, &namespace) {
                        Ok(_) => {
                            let _ = client.load_partitions(&collection, vec![namespace.clone()]);

                            Ok(NamespaceInfo {
                                name: namespace,
                                collection,
                                created_at: None,
                                vector_count: 0,
                                size_bytes: 0,
                                metadata: None,
                            })
                        }
                        Err(e) => Err(e),
                    }
                }
            }
            Err(e) => Err(e),
        }
    }

    fn list_namespaces(
        provider_config: Self::ProviderConfig,
        collection: String,
    ) -> Result<Vec<NamespaceInfo>, VectorError> {
        let client = MilvusClient::new(&provider_config);

        match client.list_partitions(&collection) {
            Ok(response) => {
                let namespaces = response
                    .data
                    .into_iter()
                    .map(|partition_name| NamespaceInfo {
                        name: partition_name,
                        collection: collection.clone(),
                        created_at: None,
                        vector_count: 0,
                        size_bytes: 0,
                        metadata: None,
                    })
                    .collect();
                Ok(namespaces)
            }
            Err(e) => Err(e),
        }
    }

    fn get_namespace(
        provider_config: Self::ProviderConfig,
        collection: String,
        namespace: String,
    ) -> Result<NamespaceInfo, VectorError> {
        let client = MilvusClient::new(&provider_config);

        match client.has_partition(&collection, &namespace) {
            Ok(response) => {
                if response.code == 0 && response.data.has {
                    Ok(NamespaceInfo {
                        name: namespace,
                        collection,
                        created_at: None,
                        vector_count: 0,
                        size_bytes: 0,
                        metadata: None,
                    })
                } else {
                    Err(VectorError::NotFound(format!(
                        "Partition {} not found in collection {}",
                        namespace, collection
                    )))
                }
            }
            Err(e) => Err(e),
        }
    }

    fn delete_namespace(
        provider_config: Self::ProviderConfig,
        collection: String,
        namespace: String,
    ) -> Result<(), VectorError> {
        let client = MilvusClient::new(&provider_config);

        let _ = client.release_partitions(&collection, vec![namespace.clone()]);

        match client.drop_partition(&collection, &namespace) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn namespace_exists(
        provider_config: Self::ProviderConfig,
        collection: String,
        namespace: String,
    ) -> Result<bool, VectorError> {
        let client = MilvusClient::new(&provider_config);

        match client.has_partition(&collection, &namespace) {
            Ok(response) => Ok(response.data.has),
            Err(_) => Ok(false),
        }
    }
}

pub type DurableMilvus = DurableVector<Milvus>;

use golem_ai_vector::model::collections::{DistanceMetric, IndexConfig};
use golem_ai_vector::model::connection;
use golem_ai_vector::model::search::{SearchQuery, VectorData};
use golem_ai_vector::model::search_extended::{
    ContextPair, RecommendationExample, RecommendationStrategy,
};
use golem_ai_vector::model::types::{
    FilterCondition, FilterExpression, FilterOperator, MetadataValue,
};
use golem_ai_vector::model::vectors::VectorRecord;
use golem_ai_vector::{
    AnalyticsProvider, CollectionProvider, ConnectionProvider, NamespacesProvider,
    SearchExtendedProvider, SearchProvider, VectorsProvider,
};
use golem_rust::{agent_definition, agent_implementation};

#[cfg(feature = "milvus")]
type Provider = golem_ai_vector_milvus::DurableMilvus;
#[cfg(feature = "pinecone")]
type Provider = golem_ai_vector_pinecone::DurablePinecone;
#[cfg(feature = "qdrant")]
type Provider = golem_ai_vector_qdrant::DurableQdrant;
#[cfg(feature = "pgvector")]
type Provider = golem_ai_vector_pgvector::DurablePgVector;

#[cfg(feature = "milvus")]
const PROVIDER: &str = "milvus";
#[cfg(feature = "pinecone")]
const PROVIDER: &str = "pinecone";
#[cfg(feature = "qdrant")]
const PROVIDER: &str = "qdrant";
#[cfg(feature = "pgvector")]
const PROVIDER: &str = "pgvector";

#[cfg(feature = "milvus")]
const TEST_ENDPOINT: &str = "http://127.0.0.1:19530";
#[cfg(feature = "milvus")]
const TEST_DATABASE: &str = "default";

#[cfg(feature = "pinecone")]
const TEST_ENDPOINT: &str = "https://your-index.pinecone.io";
#[cfg(feature = "pinecone")]
const TEST_API_KEY: &str = "your-api-key";

#[cfg(feature = "qdrant")]
const TEST_ENDPOINT: &str = "http://127.0.0.1:6333";
#[cfg(feature = "qdrant")]
const TEST_API_KEY: &str = "";

#[cfg(feature = "pgvector")]
const TEST_ENDPOINT: &str = "postgresql://postgres:mysecretpassword@localhost:3000/mydatabase";
#[cfg(feature = "pgvector")]
const TEST_DATABASE: &str = "postgres";

fn get_test_endpoint() -> String {
    std::env::var("VECTOR_TEST_ENDPOINT").unwrap_or_else(|_| TEST_ENDPOINT.to_string())
}

fn get_test_credentials() -> Option<connection::Credentials> {
    #[cfg(feature = "pinecone")]
    {
        let api_key =
            std::env::var("PINECONE_API_KEY").unwrap_or_else(|_| TEST_API_KEY.to_string());
        if !api_key.is_empty() && api_key != "your-api-key" {
            return Some(connection::Credentials::ApiKey(api_key));
        }
    }

    #[cfg(feature = "qdrant")]
    {
        let api_key =
            std::env::var("QDRANT_API_KEY").unwrap_or_else(|_| TEST_API_KEY.to_string());
        if !api_key.is_empty() {
            return Some(connection::Credentials::ApiKey(api_key));
        }
    }

    None
}

fn get_test_options() -> Option<connection::Metadata> {
    #[cfg(feature = "milvus")]
    {
        let database =
            std::env::var("MILVUS_DATABASE").unwrap_or_else(|_| TEST_DATABASE.to_string());
        return Some(vec![(
            "database".to_string(),
            MetadataValue::StringVal(database),
        )]);
    }

    #[cfg(feature = "pgvector")]
    {
        return Some(vec![(
            "connection_string".to_string(),
            MetadataValue::StringVal(get_test_endpoint()),
        )]);
    }

    #[cfg(not(any(feature = "milvus", feature = "pgvector")))]
    None
}

type DenseVector = Vec<f32>;

fn create_test_vector(id: &str, dimensions: u32) -> VectorRecord {
    let vector_data = (0..dimensions)
        .map(|i| (i as f32) * 0.1 + 1.0)
        .collect::<Vec<f32>>();

    let metadata = vec![
        (
            "name".to_string(),
            MetadataValue::StringVal(format!("vector_{}", id)),
        ),
        (
            "category".to_string(),
            MetadataValue::StringVal("test".to_string()),
        ),
        (
            "index".to_string(),
            MetadataValue::IntegerVal(id.parse::<i64>().unwrap_or(0)),
        ),
        (
            "active".to_string(),
            MetadataValue::BooleanVal(true),
        ),
        (
            "score".to_string(),
            MetadataValue::NumberVal(0.85),
        ),
    ];

    VectorRecord {
        id: id.to_string(),
        vector: VectorData::Dense(vector_data),
        metadata: Some(metadata),
    }
}

fn create_query_vector(dimensions: u32) -> DenseVector {
    (0..dimensions)
        .map(|i| (i as f32) * 0.1 + 0.5)
        .collect::<Vec<f32>>()
}

#[agent_definition]
pub trait VectorTest {
    fn new(name: String) -> Self;
    fn test1(&self) -> String;
    fn test2(&self) -> String;
    fn test3(&self) -> String;
    fn test4(&self) -> String;
    fn test5(&self) -> String;
    fn test6(&self) -> String;
    fn test7(&self) -> String;
}

struct VectorTestImpl {
    _name: String,
}

#[agent_implementation]
impl VectorTest for VectorTestImpl {
    fn new(name: String) -> Self {
        Self { _name: name }
    }

    fn test1(&self) -> String {
        println!(
            "Starting test1: Basic connection and collection operations with {}",
            PROVIDER
        );
        let mut results = Vec::new();

        let endpoint = get_test_endpoint();
        let credentials = get_test_credentials();
        let options = get_test_options();

        println!("Connecting to vector database at: {}", endpoint);

        match Provider::connect(endpoint.clone(), credentials.clone(), Some(5000), options) {
            Ok(_) => results.push("✓ Successfully connected to vector database".to_string()),
            Err(error) => return format!("✗ Connection failed: {:?}", error),
        }

        match Provider::get_connection_status() {
            Ok(status) => {
                results.push(format!(
                    "✓ Connection status: connected={}, provider={:?}",
                    status.connected, status.provider
                ));
            }
            Err(error) => return format!("✗ Failed to get connection status: {:?}", error),
        };

        let collection_name = "testcollection1".to_string();
        let index_config = IndexConfig {
            index_type: None,
            parameters: vec![],
        };

        let collection_metadata = vec![
            (
                "description".to_string(),
                MetadataValue::StringVal(
                    "Test collection for basic operations".to_string(),
                ),
            ),
            (
                "created_by".to_string(),
                MetadataValue::StringVal("test1".to_string()),
            ),
        ];

        match Provider::upsert_collection(
            collection_name.clone(),
            Some("Test collection for basic operations".to_string()),
            128,
            DistanceMetric::Cosine,
            Some(index_config),
            Some(collection_metadata),
        ) {
            Ok(info) => {
                results.push(format!("✓ Created collection: {}", info.name));
                std::thread::sleep(std::time::Duration::from_secs(4));
            }
            Err(error) => return format!("✗ Collection creation failed: {:?}", error),
        };

        match Provider::list_collections() {
            Ok(list) => {
                results.push(format!("✓ Listed {} collections", list.len()));
            }
            Err(error) => return format!("✗ Failed to list collections: {:?}", error),
        };

        match Provider::collection_exists(collection_name.clone()) {
            Ok(exists) => {
                results.push(format!("✓ Collection exists check: {}", exists));
            }
            Err(error) => return format!("✗ Failed to check collection existence: {:?}", error),
        };

        match Provider::test_connection(
            endpoint,
            get_test_credentials(),
            Some(5000),
            get_test_options(),
        ) {
            Ok(result) => {
                results.push(format!("✓ Connection test result: {}", result));
            }
            Err(error) => return format!("✗ Connection test failed: {:?}", error),
        };

        match Provider::disconnect() {
            Ok(_) => results.push("✓ Successfully disconnected".to_string()),
            Err(error) => results.push(format!("⚠ Disconnect failed: {:?}", error)),
        };

        results.join("\n")
    }

    fn test2(&self) -> String {
        println!(
            "Starting test2: Vector CRUD operations with {}",
            PROVIDER
        );
        let mut results = Vec::new();

        let endpoint = get_test_endpoint();
        let credentials = get_test_credentials();
        let options = get_test_options();

        match Provider::connect(endpoint, credentials, Some(5000), options) {
            Ok(_) => results.push("✓ Connected to vector database".to_string()),
            Err(error) => return format!("✗ Connection failed: {:?}", error),
        }

        let collection_name = "testcollection2".to_string();
        let dimensions = 64;

        let index_config = IndexConfig {
            index_type: None,
            parameters: vec![],
        };

        match Provider::upsert_collection(
            collection_name.clone(),
            None,
            dimensions,
            DistanceMetric::Cosine,
            Some(index_config),
            None,
        ) {
            Ok(_) => results.push(format!(
                "✓ Created collection with {} dimensions",
                dimensions
            )),
            Err(error) => return format!("✗ Collection creation failed: {:?}", error),
        }

        let test_vectors = vec![
            create_test_vector("1", dimensions),
            create_test_vector("2", dimensions),
            create_test_vector("3", dimensions),
        ];

        match Provider::upsert_vectors(collection_name.clone(), test_vectors, None) {
            Ok(result) => {
                results.push(format!("✓ Upserted {} vectors", result.success_count));
                std::thread::sleep(std::time::Duration::from_secs(4));
            }
            Err(error) => return format!("✗ Vector upsert failed: {:?}", error),
        };

        let vector_ids = vec!["1".to_string(), "2".to_string(), "3".to_string()];
        match Provider::get_vectors(
            collection_name.clone(),
            vector_ids,
            None,
            Some(true),
            Some(true),
        ) {
            Ok(vectors) => {
                results.push(format!("✓ Retrieved {} vectors", vectors.len()));
            }
            Err(error) => return format!("✗ Vector retrieval failed: {:?}", error),
        };

        match Provider::get_vector(collection_name.clone(), "1".to_string(), None) {
            Ok(Some(vector)) => {
                results.push(format!("✓ Retrieved single vector: {}", vector.id));
            }
            Ok(None) => return "✗ Vector not found".to_string(),
            Err(error) => return format!("✗ Single vector retrieval failed: {:?}", error),
        };

        let updated_metadata = vec![
            (
                "name".to_string(),
                MetadataValue::StringVal("updated_vector_1".to_string()),
            ),
            (
                "updated".to_string(),
                MetadataValue::BooleanVal(true),
            ),
        ];

        let update_vector = VectorData::Dense(
            (0..dimensions)
                .map(|i| (i as f32) * 0.05 + 2.0)
                .collect(),
        );

        match Provider::update_vector(
            collection_name.clone(),
            "1".to_string(),
            Some(update_vector),
            Some(updated_metadata),
            None,
            Some(true),
        ) {
            Ok(_) => results.push("✓ Vector updated successfully".to_string()),
            Err(error) => return format!("✗ Vector update failed: {:?}", error),
        }

        match Provider::count_vectors(collection_name.clone(), None, None) {
            Ok(count) => {
                results.push(format!("✓ Vector count: {}", count));
            }
            Err(error) => return format!("✗ Vector count failed: {:?}", error),
        };

        match Provider::delete_vectors(
            collection_name.clone(),
            vec!["3".to_string()],
            None,
        ) {
            Ok(count) => {
                results.push(format!("✓ Deleted {} vectors", count));
            }
            Err(error) => return format!("✗ Vector deletion failed: {:?}", error),
        };

        let _ = Provider::disconnect();

        results.join("\n")
    }

    fn test3(&self) -> String {
        println!(
            "Starting test3: Similarity search operations with {}",
            PROVIDER
        );
        let mut results = Vec::new();

        let endpoint = get_test_endpoint();
        let credentials = get_test_credentials();
        let options = get_test_options();

        match Provider::connect(endpoint, credentials, Some(5000), options) {
            Ok(_) => results.push("✓ Connected to vector database".to_string()),
            Err(error) => return format!("✗ Connection failed: {:?}", error),
        }

        let collection_name = "testcollection3".to_string();
        let dimensions = 32;

        let index_config = IndexConfig {
            index_type: None,
            parameters: vec![],
        };

        match Provider::upsert_collection(
            collection_name.clone(),
            None,
            dimensions,
            DistanceMetric::Cosine,
            Some(index_config),
            None,
        ) {
            Ok(_) => results.push(format!(
                "✓ Created collection with {} dimensions",
                dimensions
            )),
            Err(error) => return format!("✗ Collection creation failed: {:?}", error),
        }

        let test_vectors = (1..=10)
            .map(|i| create_test_vector(&i.to_string(), dimensions))
            .collect::<Vec<_>>();

        match Provider::upsert_vectors(collection_name.clone(), test_vectors, None) {
            Ok(_) => {
                results.push("✓ Inserted 10 test vectors".to_string());
                std::thread::sleep(std::time::Duration::from_secs(4));
            }
            Err(error) => return format!("✗ Vector upsert failed: {:?}", error),
        }

        let query_vector = create_query_vector(dimensions);
        let search_query = SearchQuery::Vector(VectorData::Dense(query_vector));

        match Provider::search_vectors(
            collection_name.clone(),
            search_query,
            5,
            None,
            None,
            Some(true),
            None,
            None,
            None,
            None,
        ) {
            Ok(search_results) => {
                results.push(format!(
                    "✓ Similarity search returned {} results",
                    search_results.len()
                ));
            }
            Err(error) => return format!("✗ Search failed: {:?}", error),
        };

        let query_vector2 = create_query_vector(dimensions);
        match Provider::find_similar(
            collection_name.clone(),
            VectorData::Dense(query_vector2),
            3,
            None,
        ) {
            Ok(similar_results) => {
                results.push(format!(
                    "✓ Find similar returned {} results",
                    similar_results.len()
                ));
            }
            Err(error) => return format!("✗ Find similar failed: {:?}", error),
        };

        let batch_queries = vec![
            SearchQuery::Vector(VectorData::Dense(create_query_vector(dimensions))),
            SearchQuery::Vector(VectorData::Dense(create_query_vector(dimensions))),
        ];

        match Provider::batch_search(
            collection_name.clone(),
            batch_queries,
            3,
            None,
            None,
            Some(true),
            None,
            None,
        ) {
            Ok(batch_results) => {
                results.push(format!(
                    "✓ Batch search completed {} queries",
                    batch_results.len()
                ));
            }
            Err(error) => return format!("✗ Batch search failed: {:?}", error),
        };

        let _ = Provider::disconnect();

        results.join("\n")
    }

    fn test4(&self) -> String {
        println!(
            "Starting test4: Advanced search and filtering with {}",
            PROVIDER
        );
        let mut results = Vec::new();

        let endpoint = get_test_endpoint();
        let credentials = get_test_credentials();
        let options = get_test_options();

        match Provider::connect(endpoint, credentials, Some(5000), options) {
            Ok(_) => results.push("✓ Connected to vector database".to_string()),
            Err(error) => return format!("✗ Connection failed: {:?}", error),
        }

        let collection_name = "testcollection4".to_string();
        let dimensions = 64;

        let index_config = IndexConfig {
            index_type: None,
            parameters: vec![],
        };

        match Provider::upsert_collection(
            collection_name.clone(),
            None,
            dimensions,
            DistanceMetric::Euclidean,
            Some(index_config),
            None,
        ) {
            Ok(_) => results.push(format!(
                "✓ Created collection with {} dimensions (Euclidean metric)",
                dimensions
            )),
            Err(error) => return format!("✗ Collection creation failed: {:?}", error),
        }

        let mut test_vectors = Vec::new();
        for i in 1..=20 {
            let category = if i <= 10 {
                "category_a"
            } else {
                "category_b"
            };
            let score = (i as f64) * 0.05;

            let metadata = vec![
                (
                    "name".to_string(),
                    MetadataValue::StringVal(format!("vector_{}", i)),
                ),
                (
                    "category".to_string(),
                    MetadataValue::StringVal(category.to_string()),
                ),
                ("score".to_string(), MetadataValue::NumberVal(score)),
                ("index".to_string(), MetadataValue::IntegerVal(i as i64)),
                (
                    "active".to_string(),
                    MetadataValue::BooleanVal(i % 2 == 0),
                ),
            ];

            test_vectors.push(VectorRecord {
                id: i.to_string(),
                vector: VectorData::Dense(create_query_vector(dimensions)),
                metadata: Some(metadata),
            });
        }

        match Provider::upsert_vectors(collection_name.clone(), test_vectors, None) {
            Ok(_) => {
                results.push("✓ Inserted 20 vectors with metadata".to_string());
                std::thread::sleep(std::time::Duration::from_secs(4));
            }
            Err(error) => return format!("✗ Vector upsert failed: {:?}", error),
        }

        let filter = FilterExpression::Condition(FilterCondition {
            field: "category".to_string(),
            operator: FilterOperator::Eq,
            value: MetadataValue::StringVal("category_a".to_string()),
        });

        match Provider::search_vectors(
            collection_name.clone(),
            SearchQuery::Vector(VectorData::Dense(create_query_vector(dimensions))),
            5,
            Some(filter),
            None,
            Some(true),
            None,
            None,
            None,
            None,
        ) {
            Ok(filtered_results) => {
                results.push(format!(
                    "✓ Filtered search (category_a) returned {} results",
                    filtered_results.len()
                ));
            }
            Err(error) => return format!("✗ Filtered search failed: {:?}", error),
        };

        let list_filter = if PROVIDER == "pinecone" {
            FilterExpression::Condition(FilterCondition {
                field: "id".to_string(),
                operator: FilterOperator::Contains,
                value: MetadataValue::StringVal("1".to_string()),
            })
        } else {
            FilterExpression::Condition(FilterCondition {
                field: "active".to_string(),
                operator: FilterOperator::Eq,
                value: MetadataValue::BooleanVal(true),
            })
        };

        match Provider::list_vectors(
            collection_name.clone(),
            None,
            Some(list_filter),
            Some(10),
            None,
            Some(true),
            None,
        ) {
            Ok(list_response) => {
                let filter_desc = if PROVIDER == "pinecone" {
                    "ID contains '1'"
                } else {
                    "active=true"
                };
                results.push(format!(
                    "✓ List vectors ({}) found {} vectors",
                    filter_desc,
                    list_response.vectors.len()
                ));
            }
            Err(error) => return format!("✗ List vectors failed: {:?}", error),
        };

        let delete_filter = FilterExpression::Condition(FilterCondition {
            field: "index".to_string(),
            operator: FilterOperator::Gt,
            value: MetadataValue::IntegerVal(15),
        });

        match Provider::delete_by_filter(collection_name.clone(), delete_filter, None) {
            Ok(count) => {
                results.push(format!(
                    "✓ Deleted {} vectors by filter (index > 15)",
                    count
                ));
            }
            Err(error) => return format!("✗ Delete by filter failed: {:?}", error),
        };

        let _ = Provider::disconnect();

        results.join("\n")
    }

    fn test5(&self) -> String {
        println!(
            "Starting test5: Extended search capabilities with {}",
            PROVIDER
        );
        let mut results = Vec::new();

        let endpoint = get_test_endpoint();
        let credentials = get_test_credentials();
        let options = get_test_options();

        match Provider::connect(endpoint, credentials, Some(5000), options) {
            Ok(_) => results.push("✓ Connected to vector database".to_string()),
            Err(error) => return format!("✗ Connection failed: {:?}", error),
        }

        let collection_name = "testcollection5".to_string();
        let dimensions = 128;

        let index_config = IndexConfig {
            index_type: None,
            parameters: vec![],
        };

        match Provider::upsert_collection(
            collection_name.clone(),
            None,
            dimensions,
            DistanceMetric::DotProduct,
            Some(index_config),
            None,
        ) {
            Ok(_) => results.push(format!(
                "✓ Created collection with {} dimensions (DotProduct metric)",
                dimensions
            )),
            Err(error) => return format!("✗ Collection creation failed: {:?}", error),
        }

        let test_vectors = (1..=15)
            .map(|i| create_test_vector(&i.to_string(), dimensions))
            .collect::<Vec<_>>();

        match Provider::upsert_vectors(collection_name.clone(), test_vectors, None) {
            Ok(_) => {
                results.push("✓ Inserted 15 test vectors".to_string());
                std::thread::sleep(std::time::Duration::from_secs(4));
            }
            Err(error) => return format!("✗ Vector upsert failed: {:?}", error),
        }

        let positive_examples = vec![
            RecommendationExample::VectorId("1".to_string()),
            RecommendationExample::VectorId("2".to_string()),
        ];
        let negative_examples = vec![RecommendationExample::VectorId("10".to_string())];

        match Provider::recommend_vectors(
            collection_name.clone(),
            positive_examples,
            Some(negative_examples),
            5,
            None,
            None,
            Some(RecommendationStrategy::AverageVector),
            Some(true),
            None,
        ) {
            Ok(recommendation_results) => {
                results.push(format!(
                    "✓ Recommendation search found {} results",
                    recommendation_results.len()
                ));
            }
            Err(error) => {
                results.push(format!(
                    "⚠ Recommendation search not supported: {:?}",
                    error
                ));
            }
        }

        let context_pairs = vec![ContextPair {
            positive: RecommendationExample::VectorId("1".to_string()),
            negative: RecommendationExample::VectorId("5".to_string()),
        }];

        match Provider::discover_vectors(
            collection_name.clone(),
            None,
            context_pairs,
            5,
            None,
            None,
            Some(true),
            None,
        ) {
            Ok(discovery_results) => {
                results.push(format!(
                    "✓ Discovery search found {} results",
                    discovery_results.len()
                ));
            }
            Err(error) => {
                results.push(format!(
                    "⚠ Discovery search not supported: {:?}",
                    error
                ));
            }
        }

        let query_vector = VectorData::Dense(create_query_vector(dimensions));

        match Provider::search_range(
            collection_name.clone(),
            query_vector,
            Some(0.1),
            0.8,
            None,
            None,
            Some(10),
            Some(true),
            None,
        ) {
            Ok(range_results) => {
                results.push(format!(
                    "✓ Range search found {} results",
                    range_results.len()
                ));
            }
            Err(error) => {
                results.push(format!("⚠ Range search not supported: {:?}", error));
            }
        }

        match Provider::search_text(
            collection_name.clone(),
            "test query".to_string(),
            5,
            None,
            None,
        ) {
            Ok(text_results) => {
                results.push(format!(
                    "✓ Text search found {} results",
                    text_results.len()
                ));
            }
            Err(error) => {
                results.push(format!("⚠ Text search not supported: {:?}", error));
            }
        }

        let _ = Provider::disconnect();

        results.join("\n")
    }

    fn test6(&self) -> String {
        println!(
            "Starting test6: Namespace operations with {}",
            PROVIDER
        );
        let mut results = Vec::new();

        let endpoint = get_test_endpoint();
        let credentials = get_test_credentials();
        let options = get_test_options();

        match Provider::connect(endpoint, credentials, Some(5000), options) {
            Ok(_) => results.push("✓ Connected to vector database".to_string()),
            Err(error) => return format!("✗ Connection failed: {:?}", error),
        }

        let collection_name = "testcollection6".to_string();
        let dimensions = 64;

        let index_config = IndexConfig {
            index_type: None,
            parameters: vec![],
        };

        match Provider::upsert_collection(
            collection_name.clone(),
            None,
            dimensions,
            DistanceMetric::Cosine,
            Some(index_config),
            None,
        ) {
            Ok(_) => results.push(format!(
                "✓ Created collection with {} dimensions",
                dimensions
            )),
            Err(error) => return format!("✗ Collection creation failed: {:?}", error),
        }

        let namespace_name = "test_namespace".to_string();
        let namespace_metadata = vec![(
            "description".to_string(),
            MetadataValue::StringVal("Test namespace".to_string()),
        )];

        match Provider::upsert_namespace(
            collection_name.clone(),
            namespace_name.clone(),
            Some(namespace_metadata),
        ) {
            Ok(namespace_info) => {
                results.push(format!("✓ Created namespace: {}", namespace_info.name));
                std::thread::sleep(std::time::Duration::from_secs(10));
            }
            Err(error) => {
                results.push(format!(
                    "⚠ Namespace creation not supported: {:?}",
                    error
                ));
            }
        }

        match Provider::list_namespaces(collection_name.clone()) {
            Ok(namespace_list) => {
                results.push(format!("✓ Listed {} namespaces", namespace_list.len()));
            }
            Err(error) => {
                results.push(format!(
                    "⚠ Namespace listing not supported: {:?}",
                    error
                ));
            }
        }

        let test_vectors = vec![
            create_test_vector("101", dimensions),
            create_test_vector("102", dimensions),
        ];

        match Provider::upsert_vectors(
            collection_name.clone(),
            test_vectors,
            Some(namespace_name.clone()),
        ) {
            Ok(batch_result) => {
                results.push(format!(
                    "✓ Inserted {} vectors into namespace",
                    batch_result.success_count
                ));
                std::thread::sleep(std::time::Duration::from_secs(10));
            }
            Err(error) => {
                results.push(format!(
                    "⚠ Namespace vector insertion failed: {:?}",
                    error
                ));
            }
        }

        match Provider::search_vectors(
            collection_name.clone(),
            SearchQuery::Vector(VectorData::Dense(create_query_vector(dimensions))),
            5,
            None,
            Some(namespace_name.clone()),
            Some(true),
            None,
            None,
            None,
            None,
        ) {
            Ok(search_results) => {
                results.push(format!(
                    "✓ Namespace search found {} results",
                    search_results.len()
                ));
            }
            Err(error) => {
                results.push(format!("⚠ Namespace search failed: {:?}", error));
            }
        }

        match Provider::namespace_exists(collection_name.clone(), namespace_name.clone()) {
            Ok(exists) => {
                results.push(format!("✓ Namespace exists check: {}", exists));
            }
            Err(error) => {
                results.push(format!(
                    "⚠ Namespace existence check failed: {:?}",
                    error
                ));
            }
        }

        let _ = Provider::disconnect();

        results.join("\n")
    }

    fn test7(&self) -> String {
        println!(
            "Starting test7: Analytics and statistics with {}",
            PROVIDER
        );
        let mut results = Vec::new();

        let endpoint = get_test_endpoint();
        let credentials = get_test_credentials();
        let options = get_test_options();

        match Provider::connect(endpoint, credentials, Some(5000), options) {
            Ok(_) => results.push("✓ Connected to vector database".to_string()),
            Err(error) => return format!("✗ Connection failed: {:?}", error),
        }

        let collection_name = "testcollection7".to_string();
        let dimensions = 64;

        let index_config = IndexConfig {
            index_type: None,
            parameters: vec![],
        };

        match Provider::upsert_collection(
            collection_name.clone(),
            None,
            dimensions,
            DistanceMetric::Cosine,
            Some(index_config),
            None,
        ) {
            Ok(_) => results.push(format!(
                "✓ Created collection with {} dimensions",
                dimensions
            )),
            Err(error) => return format!("✗ Collection creation failed: {:?}", error),
        }

        let test_vectors = (1..=50)
            .map(|i| {
                let category = match i % 3 {
                    0 => "category_a",
                    1 => "category_b",
                    _ => "category_c",
                };

                let metadata = vec![
                    (
                        "name".to_string(),
                        MetadataValue::StringVal(format!("vector_{}", i)),
                    ),
                    (
                        "category".to_string(),
                        MetadataValue::StringVal(category.to_string()),
                    ),
                    (
                        "score".to_string(),
                        MetadataValue::NumberVal((i as f64) * 0.02),
                    ),
                    (
                        "index".to_string(),
                        MetadataValue::IntegerVal(i as i64),
                    ),
                ];

                VectorRecord {
                    id: i.to_string(),
                    vector: VectorData::Dense(create_query_vector(dimensions)),
                    metadata: Some(metadata),
                }
            })
            .collect::<Vec<_>>();

        match Provider::upsert_vectors(collection_name.clone(), test_vectors, None) {
            Ok(_) => {
                results.push("✓ Inserted 50 test vectors with metadata".to_string());
                std::thread::sleep(std::time::Duration::from_secs(4));
            }
            Err(error) => return format!("✗ Vector upsert failed: {:?}", error),
        }

        match Provider::get_collection_stats(collection_name.clone(), None) {
            Ok(stats) => {
                results.push(format!(
                    "✓ Collection stats: {} vectors, {} dimensions",
                    stats.vector_count, stats.dimension
                ));
            }
            Err(error) => {
                results.push(format!(
                    "⚠ Collection stats not supported: {:?}",
                    error
                ));
            }
        }

        match Provider::get_field_stats(collection_name.clone(), "category".to_string(), None) {
            Ok(field_stats) => {
                results.push(format!(
                    "✓ Field stats for 'category': {} unique values",
                    field_stats.unique_values
                ));
            }
            Err(error) => {
                results.push(format!("⚠ Field stats not supported: {:?}", error));
            }
        }

        match Provider::get_field_distribution(
            collection_name.clone(),
            "category".to_string(),
            None,
            None,
        ) {
            Ok(distribution) => {
                results.push(format!(
                    "✓ Field distribution: {} different values found",
                    distribution.len()
                ));
            }
            Err(error) => {
                results.push(format!(
                    "⚠ Field distribution not supported: {:?}",
                    error
                ));
            }
        }

        match Provider::get_collection(collection_name.clone()) {
            Ok(collection_info) => {
                results.push(format!(
                    "✓ Collection info: {}, {} dimensions, {} metric",
                    collection_info.name,
                    collection_info.dimension,
                    match collection_info.metric {
                        DistanceMetric::Cosine => "cosine",
                        DistanceMetric::Euclidean => "euclidean",
                        DistanceMetric::DotProduct => "dot_product",
                        DistanceMetric::Manhattan => "manhattan",
                        DistanceMetric::Hamming => "hamming",
                        DistanceMetric::Jaccard => "jaccard",
                    }
                ));
            }
            Err(error) => {
                results.push(format!(
                    "⚠ Collection info retrieval failed: {:?}",
                    error
                ));
            }
        }

        match Provider::delete_collection(collection_name.clone()) {
            Ok(_) => {
                results.push("✓ Collection deleted successfully".to_string());
            }
            Err(error) => {
                results.push(format!("⚠ Collection deletion failed: {:?}", error));
            }
        }

        let _ = Provider::disconnect();

        results.join("\n")
    }
}

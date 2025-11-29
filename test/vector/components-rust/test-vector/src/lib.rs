#[allow(static_mut_refs)]
mod bindings;

use crate::bindings::exports::test::vector_exports::test_vector_api::*;
use crate::bindings::golem::vector::{
    connection::{self},
    types::{
        DenseVector, VectorData, VectorRecord, Metadata, MetadataValue, DistanceMetric, 
        FilterExpression, FilterCondition, FilterOperator
    },
    collections::{self, IndexConfig},
    vectors::{self},
    search::{self, SearchQuery},
    search_extended::{self, RecommendationExample, RecommendationStrategy, ContextPair},
    analytics::{self},
    namespaces::{self},
};

struct Component;

#[cfg(feature = "milvus")]
const PROVIDER: &'static str = "milvus";
#[cfg(feature = "pinecone")]
const PROVIDER: &'static str = "pinecone";
#[cfg(feature = "qdrant")]
const PROVIDER: &'static str = "qdrant";
#[cfg(feature = "pgvector")]
const PROVIDER: &'static str = "pgvector";

const DEFAULT_TEST_HOST: &'static str = "127.0.0.1";

// Milvus configuration
#[cfg(feature = "milvus")]
const TEST_ENDPOINT: &'static str = "http://127.0.0.1:19530";
#[cfg(feature = "milvus")]
const TEST_DATABASE: &'static str = "default";

// Pinecone configuration
#[cfg(feature = "pinecone")]
const TEST_ENDPOINT: &'static str = "https://your-index.pinecone.io";
#[cfg(feature = "pinecone")]
const TEST_API_KEY: &'static str = "your-api-key";

// Qdrant configuration
#[cfg(feature = "qdrant")]
const TEST_ENDPOINT: &'static str = "http://127.0.0.1:6333";
#[cfg(feature = "qdrant")]
const TEST_API_KEY: &'static str = "";

// PgVector configuration
#[cfg(feature = "pgvector")]
const TEST_ENDPOINT: &'static str = "postgresql://postgres:mysecretpassword@localhost:3000/mydatabase
";
#[cfg(feature = "pgvector")]
const TEST_DATABASE: &'static str = "postgres";

fn get_test_endpoint() -> String {
    std::env::var("VECTOR_TEST_ENDPOINT").unwrap_or_else(|_| TEST_ENDPOINT.to_string())
}

fn get_test_credentials() -> Option<crate::bindings::golem::vector::connection::Credentials> {
    #[cfg(feature = "pinecone")]
    {
        let api_key = std::env::var("PINECONE_API_KEY").unwrap_or_else(|_| TEST_API_KEY.to_string());
        if !api_key.is_empty() && api_key != "your-api-key" {
            return Some(crate::bindings::golem::vector::connection::Credentials::ApiKey(api_key));
        }
    }
    
    #[cfg(feature = "qdrant")]
    {
        let api_key = std::env::var("QDRANT_API_KEY").unwrap_or_else(|_| TEST_API_KEY.to_string());
        if !api_key.is_empty() {
            return Some(crate::bindings::golem::vector::connection::Credentials::ApiKey(api_key));
        }
    }
    
    None
}

fn get_test_options() -> Option<Metadata> {
    #[cfg(feature = "milvus")]
    {
        let database = std::env::var("MILVUS_DATABASE").unwrap_or_else(|_| TEST_DATABASE.to_string());
        return Some(vec![
            ("database".to_string(), MetadataValue::StringVal(database)),
        ]);
    }
    
    #[cfg(feature = "pgvector")]
    {
        return Some(vec![
            ("connection_string".to_string(), MetadataValue::StringVal(get_test_endpoint())),
        ]);
    }
    
    #[cfg(not(any(feature = "milvus", feature = "pgvector")))]
    None
}

fn create_test_vector(id: &str, dimensions: u32) -> VectorRecord {
    let vector_data = (0..dimensions)
        .map(|i| (i as f32) * 0.1 + 1.0)
        .collect::<Vec<f32>>();
    
    let metadata = vec![
        ("name".to_string(), MetadataValue::StringVal(format!("vector_{}", id))),
        ("category".to_string(), MetadataValue::StringVal("test".to_string())),
        ("index".to_string(), MetadataValue::IntegerVal(id.parse::<i64>().unwrap_or(0))),
        ("active".to_string(), MetadataValue::BooleanVal(true)),
        ("score".to_string(), MetadataValue::NumberVal(0.85)),
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

impl Guest for Component {
    /// test1 demonstrates basic connection and collection operations
    fn test1() -> String {
        println!("Starting test1: Basic connection and collection operations with {}", PROVIDER);
        let mut results = Vec::new();
        
        let endpoint = get_test_endpoint();
        let credentials = get_test_credentials();
        let options = get_test_options();
        
        println!("Connecting to vector database at: {}", endpoint);
        
        match connection::connect(&endpoint, credentials.as_ref(), Some(5000), options) {
            Ok(_) => results.push("✓ Successfully connected to vector database".to_string()),
            Err(error) => return format!("✗ Connection failed: {:?}", error),
        }
        
        let status = match connection::get_connection_status() {
            Ok(status) => {
                results.push(format!("✓ Connection status: connected={}, provider={:?}", 
                             status.connected, status.provider));
                status
            },
            Err(error) => return format!("✗ Failed to get connection status: {:?}", error),
        };
        
        let collection_name = "testcollection1".to_string();
        let index_config = IndexConfig {
            index_type: None,
            parameters: vec![],
        };
        
        let collection_metadata = vec![
            ("description".to_string(), MetadataValue::StringVal("Test collection for basic operations".to_string())),
            ("created_by".to_string(), MetadataValue::StringVal("test1".to_string())),
        ];
        
        let collection_info = match collections::upsert_collection(
            &collection_name,
            Some("Test collection for basic operations"),
            128,
            DistanceMetric::Cosine,
            Some(&index_config),
            Some(collection_metadata)
        ) {
            Ok(info) => {
                results.push(format!("✓ Created collection: {}", info.name));
                std::thread::sleep(std::time::Duration::from_secs(4));
                info
            },
            Err(error) => return format!("✗ Collection creation failed: {:?}", error),
        };
        
        let collections_list = match collections::list_collections() {
            Ok(list) => {
                results.push(format!("✓ Listed {} collections", list.len()));
                list
            },
            Err(error) => return format!("✗ Failed to list collections: {:?}", error),
        };
        
        let exists = match collections::collection_exists(&collection_name) {
            Ok(exists) => {
                results.push(format!("✓ Collection exists check: {}", exists));
                exists
            },
            Err(error) => return format!("✗ Failed to check collection existence: {:?}", error),
        };
        
        let test_result = match connection::test_connection(
            &endpoint,
            credentials.as_ref(),
            Some(5000),
            get_test_options()
        ) {
            Ok(result) => {
                results.push(format!("✓ Connection test result: {}", result));
                result
            },
            Err(error) => return format!("✗ Connection test failed: {:?}", error),
        };
        
        match connection::disconnect() {
            Ok(_) => results.push("✓ Successfully disconnected".to_string()),
            Err(error) => results.push(format!("⚠ Disconnect failed: {:?}", error)),
        };
        
        results.join("\n")
    }

    /// test2 demonstrates vector CRUD operations
    fn test2() -> String {
        println!("Starting test2: Vector CRUD operations with {}", PROVIDER);
        let mut results = Vec::new();
        
        let endpoint = get_test_endpoint();
        let credentials = get_test_credentials();
        let options = get_test_options();
        
        match connection::connect(&endpoint, credentials.as_ref(), Some(5000), options) {
            Ok(_) => results.push("✓ Connected to vector database".to_string()),
            Err(error) => return format!("✗ Connection failed: {:?}", error),
        }
        
        let collection_name = "testcollection2".to_string();
        let dimensions = 64;
        
        let index_config = IndexConfig {
            index_type: None,
            parameters: vec![],
        };
        
        match collections::upsert_collection(&collection_name, None, dimensions, DistanceMetric::Cosine, Some(&index_config), None) {
            Ok(_) => results.push(format!("✓ Created collection with {} dimensions", dimensions)),
            Err(error) => return format!("✗ Collection creation failed: {:?}", error),
        }
        
        let test_vectors = vec![
            create_test_vector("1", dimensions),
            create_test_vector("2", dimensions),
            create_test_vector("3", dimensions),
        ];
        
        let batch_result = match vectors::upsert_vectors(
            &collection_name,
            test_vectors,
            None
        ) {
            Ok(result) => {
                results.push(format!("✓ Upserted {} vectors", result.success_count));
                std::thread::sleep(std::time::Duration::from_secs(4));
                result
            },
            Err(error) => return format!("✗ Vector upsert failed: {:?}", error),
        };
        
        let vector_ids = vec!["1".to_string(), "2".to_string(), "3".to_string()];
        let retrieved_vectors = match vectors::get_vectors(
            &collection_name,
            &vector_ids,
            None,
            Some(true),
            Some(true)
        ) {
            Ok(vectors) => {
                results.push(format!("✓ Retrieved {} vectors", vectors.len()));
                vectors
            },
            Err(error) => return format!("✗ Vector retrieval failed: {:?}", error),
        };
        
        let _single_vector = match vectors::get_vector(
            &collection_name,
            &"1".to_string(),
            None
        ) {
            Ok(Some(vector)) => {
                results.push(format!("✓ Retrieved single vector: {}", vector.id));
                vector
            },
            Ok(None) => return "✗ Vector not found".to_string(),
            Err(error) => return format!("✗ Single vector retrieval failed: {:?}", error),
        };
        
        let updated_metadata = vec![
            ("name".to_string(), MetadataValue::StringVal("updated_vector_1".to_string())),
            ("updated".to_string(), MetadataValue::BooleanVal(true)),
        ];

         let update_vector = VectorData::Dense((0..dimensions)
            .map(|i| (i as f32) * 0.05 + 2.0)
            .collect());
        
        match vectors::update_vector(
            &collection_name,
            &"1".to_string(),
            Some(&update_vector),
            Some(updated_metadata),
            None,
            Some(true)
        ) {
            Ok(_) => results.push("✓ Vector updated successfully".to_string()),
            Err(error) => return format!("✗ Vector update failed: {:?}", error),
        }
        
        let count = match vectors::count_vectors(&collection_name, None, None) {
            Ok(count) => {
                results.push(format!("✓ Vector count: {}", count));
                count
            },
            Err(error) => return format!("✗ Vector count failed: {:?}", error),
        };
        
        let deleted_count = match vectors::delete_vectors(
            &collection_name,
            &vec!["3".to_string()],
            None
        ) {
            Ok(count) => {
                results.push(format!("✓ Deleted {} vectors", count));
                count
            },
            Err(error) => return format!("✗ Vector deletion failed: {:?}", error),
        };
        
        let _ = connection::disconnect();
        
        results.join("\n")
    }

    /// test3 demonstrates similarity search operations
    fn test3() -> String {
        println!("Starting test3: Similarity search operations with {}", PROVIDER);
        let mut results = Vec::new();
        
        let endpoint = get_test_endpoint();
        let credentials = get_test_credentials();
        let options = get_test_options();
        
        match connection::connect(&endpoint, credentials.as_ref(), Some(5000), options) {
            Ok(_) => results.push("✓ Connected to vector database".to_string()),
            Err(error) => return format!("✗ Connection failed: {:?}", error),
        }
        
        let collection_name = "testcollection3".to_string();
        let dimensions = 32;
        
        let index_config = IndexConfig {
            index_type: None,
            parameters: vec![],
        };
        
        match collections::upsert_collection(&collection_name, None, dimensions, DistanceMetric::Cosine, Some(&index_config), None) {
            Ok(_) => results.push(format!("✓ Created collection with {} dimensions", dimensions)),
            Err(error) => return format!("✗ Collection creation failed: {:?}", error),
        }
        
        let test_vectors = (1..=10)
            .map(|i| create_test_vector(&i.to_string(), dimensions))
            .collect::<Vec<_>>();
        
        match vectors::upsert_vectors(&collection_name, test_vectors, None) {
            Ok(_) => {
                results.push("✓ Inserted 10 test vectors".to_string());
                std::thread::sleep(std::time::Duration::from_secs(4));
            },
            Err(error) => return format!("✗ Vector upsert failed: {:?}", error),
        }
        
        let query_vector = create_query_vector(dimensions);
        let search_query = SearchQuery::Vector(VectorData::Dense(query_vector));
        
        let search_results = match search::search_vectors(
            &collection_name,
            &search_query,
            5,
            None,
            None,
            Some(true),
            None,
            None,
            None,
            None
        ) {
            Ok(search_results) => {
                results.push(format!("✓ Similarity search returned {} results", search_results.len()));
                search_results
            },
            Err(error) => return format!("✗ Search failed: {:?}", error),
        };
        
        let query_vector2 = create_query_vector(dimensions);
        let similar_results = match search::find_similar(
            &collection_name,
            &VectorData::Dense(query_vector2),
            3,
            None
        ) {
            Ok(similar_results) => {
                results.push(format!("✓ Find similar returned {} results", similar_results.len()));
                similar_results
            },
            Err(error) => return format!("✗ Find similar failed: {:?}", error),
        };
        
        let batch_queries = vec![
            SearchQuery::Vector(VectorData::Dense(create_query_vector(dimensions))),
            SearchQuery::Vector(VectorData::Dense(create_query_vector(dimensions))),
        ];
        
        let batch_results = match search::batch_search(
            &collection_name,
            &batch_queries,
            3,
            None,
            None,
            Some(true),
            None,
            None
        ) {
            Ok(batch_results) => {
                results.push(format!("✓ Batch search completed {} queries", batch_results.len()));
                batch_results
            },
            Err(error) => return format!("✗ Batch search failed: {:?}", error),
        };
        
        let _ = connection::disconnect();
        
        results.join("\n")
    }

    /// test4 demonstrates advanced search and filtering operations
    fn test4() -> String {
        println!("Starting test4: Advanced search and filtering with {}", PROVIDER);
        let mut results = Vec::new();
        
        let endpoint = get_test_endpoint();
        let credentials = get_test_credentials();
        let options = get_test_options();
        
        match connection::connect(&endpoint, credentials.as_ref(), Some(5000), options) {
            Ok(_) => results.push("✓ Connected to vector database".to_string()),
            Err(error) => return format!("✗ Connection failed: {:?}", error),
        }
        
        let collection_name = "testcollection4".to_string();
        let dimensions = 64;
        
        let index_config = IndexConfig {
            index_type: None,
            parameters: vec![],
        };
        
        match collections::upsert_collection(&collection_name, None, dimensions, DistanceMetric::Euclidean, Some(&index_config), None) {
            Ok(_) => results.push(format!("✓ Created collection with {} dimensions (Euclidean metric)", dimensions)),
            Err(error) => return format!("✗ Collection creation failed: {:?}", error),
        }
        
        let mut test_vectors = Vec::new();
        for i in 1..=20 {
            let category = if i <= 10 { "category_a" } else { "category_b" };
            let score = (i as f64) * 0.05;
            
            let metadata = vec![
                ("name".to_string(), MetadataValue::StringVal(format!("vector_{}", i))),
                ("category".to_string(), MetadataValue::StringVal(category.to_string())),
                ("score".to_string(), MetadataValue::NumberVal(score)),
                ("index".to_string(), MetadataValue::IntegerVal(i as i64)),
                ("active".to_string(), MetadataValue::BooleanVal(i % 2 == 0)),
            ];
            
            test_vectors.push(VectorRecord {
                id: i.to_string(),
                vector: VectorData::Dense(create_query_vector(dimensions)),
                metadata: Some(metadata),
            });
        }
        
        match vectors::upsert_vectors(&collection_name, test_vectors, None) {
            Ok(_) => {
                results.push("✓ Inserted 20 vectors with metadata".to_string());
                std::thread::sleep(std::time::Duration::from_secs(4));
            },
            Err(error) => return format!("✗ Vector upsert failed: {:?}", error),
        }
        
        let filter = FilterExpression::Condition(FilterCondition {
            field: "category".to_string(),
            operator: FilterOperator::Eq,
            value: MetadataValue::StringVal("category_a".to_string()),
        });
        
        let filtered_search = match search::search_vectors(
            &collection_name,
            &SearchQuery::Vector(VectorData::Dense(create_query_vector(dimensions))),
            5,
            Some(filter),
            None,
            Some(true),
            None,
            None,
            None,
            None
        ) {
            Ok(filtered_results) => {
                results.push(format!("✓ Filtered search (category_a) returned {} results", filtered_results.len()));
                filtered_results
            },
            Err(error) => return format!("✗ Filtered search failed: {:?}", error),
        };
        
        let list_filter = if PROVIDER == "pinecone" {
            // Pinecone only supports prefix filtering on ID field
            FilterExpression::Condition(FilterCondition {
                field: "id".to_string(),
                operator: FilterOperator::Contains,
                value: MetadataValue::StringVal("1".to_string()),
            })
        } else {
            // Other providers support metadata filtering
            FilterExpression::Condition(FilterCondition {
                field: "active".to_string(),
                operator: FilterOperator::Eq,
                value: MetadataValue::BooleanVal(true),
            })
        };
        
        let list_response = match vectors::list_vectors(
            &collection_name,
            None,
            Some(list_filter),
            Some(10),
            None,
            Some(true),
            None
        ) {
            Ok(list_response) => {
                let filter_desc = if PROVIDER == "pinecone" {
                    "ID contains '1'"
                } else {
                    "active=true"
                };
                results.push(format!("✓ List vectors ({}) found {} vectors", filter_desc, list_response.vectors.len()));
                list_response
            },
            Err(error) => return format!("✗ List vectors failed: {:?}", error),
        };
        
        let delete_filter = FilterExpression::Condition(FilterCondition {
            field: "index".to_string(),
            operator: FilterOperator::Gt,
            value: MetadataValue::IntegerVal(15),
        });
        
        let deleted_count = match vectors::delete_by_filter(
            &collection_name,
            delete_filter,
            None
        ) {
            Ok(count) => {
                results.push(format!("✓ Deleted {} vectors by filter (index > 15)", count));
                count
            },
            Err(error) => return format!("✗ Delete by filter failed: {:?}", error),
        };
        
        let _ = connection::disconnect();
        
        results.join("\n")
    }

    /// test5 demonstrates extended search capabilities (recommendation, discovery, etc.)
    fn test5() -> String {
        println!("Starting test5: Extended search capabilities with {}", PROVIDER);
        let mut results = Vec::new();
        
        let endpoint = get_test_endpoint();
        let credentials = get_test_credentials();
        let options = get_test_options();
        
        match connection::connect(&endpoint, credentials.as_ref(), Some(5000), options) {
            Ok(_) => results.push("✓ Connected to vector database".to_string()),
            Err(error) => return format!("✗ Connection failed: {:?}", error),
        }
        
        let collection_name = "testcollection5".to_string();
        let dimensions = 128;
        
        let index_config = IndexConfig {
            index_type: None,
            parameters: vec![],
        };
        
        match collections::upsert_collection(&collection_name, None, dimensions, DistanceMetric::DotProduct, Some(&index_config), None) {
            Ok(_) => results.push(format!("✓ Created collection with {} dimensions (DotProduct metric)", dimensions)),
            Err(error) => return format!("✗ Collection creation failed: {:?}", error),
        }
        
        let test_vectors = (1..=15)
            .map(|i| create_test_vector(&i.to_string(), dimensions))
            .collect::<Vec<_>>();
        
        match vectors::upsert_vectors(&collection_name, test_vectors, None) {
            Ok(_) => {
                results.push("✓ Inserted 15 test vectors".to_string());
                std::thread::sleep(std::time::Duration::from_secs(4));
            },
            Err(error) => return format!("✗ Vector upsert failed: {:?}", error),
        }
        
        // recommendation-based search ( not  supported by all providers)
        let positive_examples = vec![
            RecommendationExample::VectorId("1".to_string()),
            RecommendationExample::VectorId("2".to_string()),
        ];
        let negative_examples = vec![
            RecommendationExample::VectorId("10".to_string()),
        ];
        
        match search_extended::recommend_vectors(
            &collection_name,
            &positive_examples,
            Some(&negative_examples),
            5,
            None,
            None,
            Some(RecommendationStrategy::AverageVector),
            Some(true),
            None
        ) {
            Ok(recommendation_results) => {
                results.push(format!("✓ Recommendation search found {} results", recommendation_results.len()));
            },
            Err(error) => {
                results.push(format!("⚠ Recommendation search not supported: {:?}", error));
            }
        }
        
        // discovery/context search ( not supported by all providers)
        let context_pairs = vec![
            ContextPair {
                positive: RecommendationExample::VectorId("1".to_string()),
                negative: RecommendationExample::VectorId("5".to_string()),
            },
        ];
        
        match search_extended::discover_vectors(
            &collection_name,
            None,
            &context_pairs,
            5,
            None,
            None,
            Some(true),
            None
        ) {
            Ok(discovery_results) => {
                results.push(format!("✓ Discovery search found {} results", discovery_results.len()));
            },
            Err(error) => {
                results.push(format!("⚠ Discovery search not supported: {:?}", error));
            }
        }
        
        // range search (not supported by all providers)
        let query_vector = VectorData::Dense(create_query_vector(dimensions));
        
        match search_extended::search_range(
            &collection_name,
            &query_vector,
            Some(0.1),
            0.8,
            None,
            None,
            Some(10),
            Some(true),
            None
        ) {
            Ok(range_results) => {
                results.push(format!("✓ Range search found {} results", range_results.len()));
            },
            Err(error) => {
                results.push(format!("⚠ Range search not supported: {:?}", error));
            }
        }
        
        // text search (not supported by all providers)
        match search_extended::search_text(
            &collection_name,
            "test query",
            5,
            None,
            None
        ) {
            Ok(text_results) => {
                results.push(format!("✓ Text search found {} results", text_results.len()));
            },
            Err(error) => {
                results.push(format!("⚠ Text search not supported: {:?}", error));
            }
        }
        
        let _ = connection::disconnect();
        
        results.join("\n")
    }

    /// test6 demonstrates namespace operations
    fn test6() -> String {
        println!("Starting test6: Namespace operations with {}", PROVIDER);
        let mut results = Vec::new();
        
        let endpoint = get_test_endpoint();
        let credentials = get_test_credentials();
        let options = get_test_options();
        
        match connection::connect(&endpoint, credentials.as_ref(), Some(5000), options) {
            Ok(_) => results.push("✓ Connected to vector database".to_string()),
            Err(error) => return format!("✗ Connection failed: {:?}", error),
        }
        
        let collection_name = "testcollection6".to_string();
        let dimensions = 64;
        
        let index_config = IndexConfig {
            index_type: None,
            parameters: vec![],
        };
        
        match collections::upsert_collection(&collection_name, None, dimensions, DistanceMetric::Cosine, Some(&index_config), None) {
            Ok(_) => results.push(format!("✓ Created collection with {} dimensions", dimensions)),
            Err(error) => return format!("✗ Collection creation failed: {:?}", error),
        }
        
        // namespace operations ( not supported by all providers)
        let namespace_name = "test_namespace".to_string();
        let namespace_metadata = vec![
            ("description".to_string(), MetadataValue::StringVal("Test namespace".to_string())),
        ];
        
        match namespaces::upsert_namespace(
            &collection_name,
            &namespace_name,
            Some(namespace_metadata)
        ) {
            Ok(namespace_info) => {
                results.push(format!("✓ Created namespace: {}", namespace_info.name));
                std::thread::sleep(std::time::Duration::from_secs(10));

            },
            Err(error) => {
                results.push(format!("⚠ Namespace creation not supported: {:?}", error));
            }
        }
        
        match namespaces::list_namespaces(&collection_name) {
            Ok(namespace_list) => {
                results.push(format!("✓ Listed {} namespaces", namespace_list.len()));
            },
            Err(error) => {
                results.push(format!("⚠ Namespace listing not supported: {:?}", error));
            }
        }
        
        let test_vectors = vec![
            create_test_vector("101", dimensions),
            create_test_vector("102", dimensions),
        ];
        
        match vectors::upsert_vectors(
            &collection_name,
            test_vectors,
            Some(&namespace_name)
        ) {
            Ok(batch_result) => {
                results.push(format!("✓ Inserted {} vectors into namespace", batch_result.success_count));
                std::thread::sleep(std::time::Duration::from_secs(10));
            },
            Err(error) => {
                results.push(format!("⚠ Namespace vector insertion failed: {:?}", error));
            }
        }
        
        match search::search_vectors(
            &collection_name,
            &SearchQuery::Vector(VectorData::Dense(create_query_vector(dimensions))),
            5,
            None,
            Some(&namespace_name),
            Some(true),
            None,
            None,
            None,
            None
        ) {
            Ok(search_results) => {
                results.push(format!("✓ Namespace search found {} results", search_results.len()));
            },
            Err(error) => {
                results.push(format!("⚠ Namespace search failed: {:?}", error));
            }
        }
        
        match namespaces::namespace_exists(&collection_name, &namespace_name) {
            Ok(exists) => {
                results.push(format!("✓ Namespace exists check: {}", exists));
            },
            Err(error) => {
                results.push(format!("⚠ Namespace existence check failed: {:?}", error));
            }
        }
        
        let _ = connection::disconnect();
        
        results.join("\n")
    }

    /// test7 demonstrates analytics and statistics operations
    fn test7() -> String {
        println!("Starting test7: Analytics and statistics with {}", PROVIDER);
        let mut results = Vec::new();
        
        let endpoint = get_test_endpoint();
        let credentials = get_test_credentials();
        let options = get_test_options();
        
        match connection::connect(&endpoint, credentials.as_ref(), Some(5000), options) {
            Ok(_) => results.push("✓ Connected to vector database".to_string()),
            Err(error) => return format!("✗ Connection failed: {:?}", error),
        }
        
        let collection_name = "testcollection7".to_string();
        let dimensions = 64;
        
        let index_config = IndexConfig {
            index_type: None,
            parameters: vec![],
        };
        
        match collections::upsert_collection(&collection_name, None, dimensions, DistanceMetric::Cosine, Some(&index_config), None) {
            Ok(_) => results.push(format!("✓ Created collection with {} dimensions", dimensions)),
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
                    ("name".to_string(), MetadataValue::StringVal(format!("vector_{}", i))),
                    ("category".to_string(), MetadataValue::StringVal(category.to_string())),
                    ("score".to_string(), MetadataValue::NumberVal((i as f64) * 0.02)),
                    ("index".to_string(), MetadataValue::IntegerVal(i as i64)),
                ];
                
                VectorRecord {
                    id: i.to_string(),
                    vector: VectorData::Dense(create_query_vector(dimensions)),
                    metadata: Some(metadata),
                }
            })
            .collect::<Vec<_>>();
        
        match vectors::upsert_vectors(&collection_name, test_vectors, None) {
            Ok(_) => {
                results.push("✓ Inserted 50 test vectors with metadata".to_string());
                std::thread::sleep(std::time::Duration::from_secs(4));
            },
            Err(error) => return format!("✗ Vector upsert failed: {:?}", error),
        }

        match analytics::get_collection_stats(&collection_name, None) {
            Ok(stats) => {
                results.push(format!(
                    "✓ Collection stats: {} vectors, {} dimensions", 
                    stats.vector_count, stats.dimension
                ));
            },
            Err(error) => {
                results.push(format!("⚠ Collection stats not supported: {:?}", error));
            }
        }
        
        match analytics::get_field_stats(
            &collection_name,
            "category",
            None
        ) {
            Ok(field_stats) => {
                results.push(format!(
                    "✓ Field stats for 'category': {} unique values", 
                    field_stats.unique_values
                ));
            },
            Err(error) => {
                results.push(format!("⚠ Field stats not supported: {:?}", error));
            }
        }
        
        match analytics::get_field_distribution(
            &collection_name,
            "category",
            None,
            None
        ) {
            Ok(distribution) => {
                results.push(format!(
                    "✓ Field distribution: {} different values found", 
                    distribution.len()
                ));
            },
            Err(error) => {
                results.push(format!("⚠ Field distribution not supported: {:?}", error));
            }
        }
        
        match collections::get_collection(&collection_name.clone()) {
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
            },
            Err(error) => {
                results.push(format!("⚠ Collection info retrieval failed: {:?}", error));
            }
        }
        
        match collections::delete_collection(&collection_name) {
            Ok(_) => {
                results.push("✓ Collection deleted successfully".to_string());
            },
            Err(error) => {
                results.push(format!("⚠ Collection deletion failed: {:?}", error));
            }
        }
        
        let _ = connection::disconnect();
        
        results.join("\n")
    }
}

bindings::export!(Component with_types_in bindings);

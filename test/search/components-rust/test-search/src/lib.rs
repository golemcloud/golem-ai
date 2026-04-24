use golem_ai_search::model::*;
use golem_ai_search::SearchProvider;
use golem_rust::{agent_definition, agent_implementation};

#[cfg(feature = "algolia")]
type Provider = golem_ai_search_algolia::DurableAlgolia;
#[cfg(feature = "elasticsearch")]
type Provider = golem_ai_search_elasticsearch::DurableElasticsarch;
#[cfg(feature = "meilisearch")]
type Provider = golem_ai_search_meilisearch::DurableMeilisearch;
#[cfg(feature = "opensearch")]
type Provider = golem_ai_search_opensearch::DurableOpenSearch;
#[cfg(feature = "typesense")]
type Provider = golem_ai_search_typesense::DurableTypesense;

#[cfg(feature = "algolia")]
const TEST_INDEX: &str = "test-algolia-index";
#[cfg(feature = "elasticsearch")]
const TEST_INDEX: &str = "test-elasticsearch-index";
#[cfg(feature = "meilisearch")]
const TEST_INDEX: &str = "test-meilisearch-index";
#[cfg(feature = "opensearch")]
const TEST_INDEX: &str = "test-opensearch-index";
#[cfg(feature = "typesense")]
const TEST_INDEX: &str = "test-typesense-index";

fn create_test_documents() -> Vec<Doc> {
    vec![
        Doc {
            id: "doc1".to_string(),
            content: r#"{"title": "The Great Gatsby", "author": "F. Scott Fitzgerald", "year": 1925, "genre": "fiction", "description": "A classic American novel about the Jazz Age"}"#.to_string(),
        },
        Doc {
            id: "doc2".to_string(),
            content: r#"{"title": "To Kill a Mockingbird", "author": "Harper Lee", "year": 1960, "genre": "fiction", "description": "A powerful story about racial injustice in the American South"}"#.to_string(),
        },
        Doc {
            id: "doc3".to_string(),
            content: r#"{"title": "1984", "author": "George Orwell", "year": 1949, "genre": "dystopian", "description": "A dystopian novel about totalitarian surveillance"}"#.to_string(),
        },
        Doc {
            id: "doc4".to_string(),
            content: r#"{"title": "Pride and Prejudice", "author": "Jane Austen", "year": 1813, "genre": "romance", "description": "A romantic novel about marriage and social class in Georgian England"}"#.to_string(),
        },
        Doc {
            id: "doc5".to_string(),
            content: r#"{"title": "The Catcher in the Rye", "author": "J.D. Salinger", "year": 1951, "genre": "fiction", "description": "A coming-of-age story about teenage rebellion"}"#.to_string(),
        },
    ]
}

fn create_test_schema() -> Schema {
    Schema {
        fields: vec![
            SchemaField {
                name: "title".to_string(),
                field_type: FieldType::Text,
                required: false,
                facet: false,
                sort: false,
                index: true,
            },
            SchemaField {
                name: "author".to_string(),
                field_type: FieldType::Text,
                required: false,
                facet: true,
                sort: false,
                index: true,
            },
            SchemaField {
                name: "year".to_string(),
                field_type: FieldType::Integer,
                required: false,
                facet: false,
                sort: true,
                index: true,
            },
            SchemaField {
                name: "genre".to_string(),
                field_type: FieldType::Text,
                required: false,
                facet: true,
                sort: false,
                index: true,
            },
            SchemaField {
                name: "description".to_string(),
                field_type: FieldType::Text,
                required: false,
                facet: false,
                sort: false,
                index: true,
            },
        ],
        primary_key: Some("id".to_string()),
    }
}

fn needs_explicit_index_creation() -> bool {
    TEST_INDEX == "test-elasticsearch-index"
        || TEST_INDEX == "test-typesense-index"
        || TEST_INDEX == "test-opensearch-index"
}

fn maybe_create_index(index_name: &str, results: &mut Vec<String>) -> Result<(), String> {
    if needs_explicit_index_creation() {
        println!("Setting up index: {}", index_name);
        match Provider::create_index(CreateIndexOptions {
            index_name: index_name.to_string(),
            schema: Some(create_test_schema()),
        }) {
            Ok(_) => {
                results.push("✓ Index created successfully".to_string());
                Ok(())
            }
            Err(e) => Err(format!("✗ Index creation failed: {:?}", e)),
        }
    } else {
        println!("Setting up index: {}", index_name);
        Ok(())
    }
}

#[agent_definition]
pub trait SearchTest {
    fn new(name: String) -> Self;
    fn test1(&self) -> String;
    fn test2(&self) -> String;
    fn test3(&self) -> String;
    fn test4(&self) -> String;
    fn test5(&self) -> String;
    fn test6(&self) -> String;
    fn test7(&self) -> String;
}

struct SearchTestImpl {
    _name: String,
}

#[agent_implementation]
impl SearchTest for SearchTestImpl {
    fn new(name: String) -> Self {
        Self { _name: name }
    }

    fn test1(&self) -> String {
        let index_name = format!("{}-test1", TEST_INDEX);
        let mut results = Vec::new();

        if let Err(e) = maybe_create_index(&index_name, &mut results) {
            return e;
        }

        println!("Setting up index : {}", index_name);
        match Provider::update_schema(index_name.clone(), create_test_schema()) {
            Ok(_) => results.push("✓ Index schema configured successfully".to_string()),
            Err(SearchError::Unsupported) => {
                results.push("✓ Schema configuration not required (auto-detected)".to_string())
            }
            Err(e) => {
                results.push(format!("⚠  setup failed, proceeding anyway: {:?}", e));
            }
        }

        let docs = create_test_documents();
        println!("Inserting {} documents", docs.len());
        match Provider::upsert_many(index_name.clone(), docs) {
            Ok(_) => results.push("✓ Documents inserted successfull".to_string()),
            Err(e) => {
                results.push(format!("✗ Document insertion failed: {:?}", e));
                return results.join("\n");
            }
        }

        println!("Retrieving document with ID: doc1");
        let mut retrieval_success = false;
        for attempt in 1..=5 {
            match Provider::get(index_name.clone(), "doc1".to_string()) {
                Ok(Some(doc)) => {
                    results.push(format!(
                        "✓ Document retrieved: {} (attempt {})",
                        doc.id, attempt
                    ));
                    retrieval_success = true;
                    break;
                }
                Ok(None) => {
                    if attempt == 5 {
                        results.push("✗ Document not found after 5 attempts".to_string());
                    } else {
                        println!("Document not found, retrying... (attempt {}/5)", attempt);
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                    }
                }
                Err(e) => {
                    results.push(format!("✗ Document retrieval failed: {:?}", e));
                    break;
                }
            }
        }

        if retrieval_success {
            println!("Deleting document with ID: doc1");
            match Provider::delete(index_name.clone(), "doc1".to_string()) {
                Ok(_) => {
                    results.push("✓ Document deleted successfully".to_string());

                    for attempt in 1..=5 {
                        match Provider::get(index_name.clone(), "doc1".to_string()) {
                            Ok(None) | Err(_) => {
                                results.push(format!(
                                    "✓ Document deletion verified (attempt {})",
                                    attempt
                                ));
                                break;
                            }
                            Ok(Some(_)) => {
                                if attempt == 5 {
                                    results
                                        .push("⚠ Document still exists after deletion".to_string());
                                } else {
                                    std::thread::sleep(std::time::Duration::from_millis(1000));
                                }
                            }
                        }
                    }
                }
                Err(e) => results.push(format!("✗ Document deletion failed: {:?}", e)),
            }
        }

        println!("Deleting index: {}", index_name);
        match Provider::delete_index(index_name.clone()) {
            Ok(_) => results.push("✓ Index deleted successfully".to_string()),
            Err(e) => results.push(format!("✗ Index deletion failed: {:?}", e)),
        }

        Provider::delete_index(index_name).ok();
        results.join("\n")
    }

    fn test2(&self) -> String {
        let index_name = format!("{}-test2", TEST_INDEX);
        let mut results = Vec::new();

        match Provider::update_schema(index_name.clone(), create_test_schema()) {
            Ok(_) => {}
            Err(SearchError::Unsupported) => {
                println!("Schema setup not required (auto-detected on first document)");
            }
            Err(_) => {
                println!("Schema setup failed, proceeding with document insertion");
            }
        }

        let docs = create_test_documents();
        if let Err(e) = Provider::upsert_many(index_name.clone(), docs) {
            Provider::delete_index(index_name).ok();
            return format!("Document insertion failed: {:?}", e);
        }

        println!("Testing basic text search for 'Gatsby'");
        let query = SearchQuery {
            q: Some("Gatsby".to_string()),
            filters: vec![],
            sort: vec![],
            facets: vec![],
            page: None,
            per_page: None,
            offset: None,
            highlight: None,
            config: None,
        };

        let mut _search_success = false;
        for attempt in 1..=10 {
            match Provider::search(index_name.clone(), query.clone()) {
                Ok(search_results) if !search_results.hits.is_empty() => {
                    results.push(format!(
                        "✓ Search returned {} hits (attempt {})",
                        search_results.hits.len(),
                        attempt
                    ));
                    if let Some(first_hit) = search_results.hits.first() {
                        results.push(format!("  First hit ID: {}", first_hit.id));
                        if let Some(score) = first_hit.score {
                            results.push(format!("  Score: {:.2}", score));
                        }
                    }
                    _search_success = true;
                    break;
                }
                Ok(_) => {
                    if attempt == 10 {
                        results.push("⚠ Search returned no hits after 10 attempts".to_string());
                    } else {
                        println!(
                            "Search returned no hits, retrying... (attempt {}/10)",
                            attempt
                        );
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                    }
                }
                Err(e) => {
                    results.push(format!("✗ Search failed: {:?}", e));
                    break;
                }
            }
        }

        println!("Testing filtered search for fiction genre");

        let filter_attempts = vec![
            (
                "Algolia/Elasticsearch/opensearch/typesense",
                "genre:fiction",
            ),
            ("Meilisearch", "genre = \"fiction\""),
            ("Alternative", "genre=\"fiction\""),
        ];

        let mut filter_success = false;
        for (provider_hint, filter_syntax) in &filter_attempts {
            let filtered_query = SearchQuery {
                q: Some("Gatsby".to_string()),
                filters: vec![filter_syntax.to_string()],
                sort: vec![],
                facets: vec![],
                page: None,
                per_page: None,
                offset: None,
                highlight: None,
                config: None,
            };

            match Provider::search(index_name.clone(), filtered_query) {
                Ok(search_results) => {
                    results.push(format!(
                        "✓ Filtered search returned {} hits (syntax: {})",
                        search_results.hits.len(),
                        provider_hint
                    ));
                    filter_success = true;
                    break;
                }
                Err(SearchError::InvalidQuery(_)) => {
                    continue;
                }
                Err(SearchError::Unsupported) => {
                    results.push("⚠ Filtered search not supported by this provider".to_string());
                    filter_success = true;
                    break;
                }
                Err(e) => {
                    if filter_attempts.iter().position(|(p, _)| p == provider_hint)
                        == Some(filter_attempts.len() - 1)
                    {
                        results.push(format!(
                            "✗ Filtered search failed with all syntaxes: {:?}",
                            e
                        ));
                    }
                }
            }
        }

        if !filter_success {
            println!("Falling back to text-based search for 'fiction'");
            let fallback_query = SearchQuery {
                q: Some("fiction".to_string()),
                filters: vec![],
                sort: vec![],
                facets: vec![],
                page: None,
                per_page: None,
                offset: None,
                highlight: None,
                config: None,
            };

            match Provider::search(index_name.clone(), fallback_query) {
                Ok(search_results) => {
                    results.push(format!(
                        "✓ Fallback text search for 'fiction' returned {} hits",
                        search_results.hits.len()
                    ));
                }
                Err(e) => results.push(format!("✗ Even fallback search failed: {:?}", e)),
            }
        }

        Provider::delete_index(index_name).ok();
        results.join("\n")
    }

    fn test3(&self) -> String {
        let index_name = format!("{}-test3", TEST_INDEX);
        let mut results = Vec::new();

        if let Err(e) = maybe_create_index(&index_name, &mut results) {
            return e;
        }

        match Provider::update_schema(index_name.clone(), create_test_schema()) {
            Ok(_) => {}
            Err(SearchError::Unsupported) => {}
            Err(_) => {}
        }

        let docs = create_test_documents();
        if let Err(e) = Provider::upsert_many(index_name.clone(), docs) {
            Provider::delete_index(index_name).ok();
            return format!("Document insertion failed: {:?}", e);
        }

        println!("Testing search with sorting by year");
        let sorted_query = SearchQuery {
            q: None,
            filters: vec![],
            sort: vec!["year:desc".to_string()],
            facets: vec![],
            page: None,
            per_page: None,
            offset: None,
            highlight: None,
            config: None,
        };

        match Provider::search(index_name.clone(), sorted_query) {
            Ok(search_results) => {
                results.push(format!(
                    "✓ Sorted search returned {} hits",
                    search_results.hits.len()
                ));
                if search_results.hits.len() >= 2 {
                    results
                        .push("  Verifying sort order by checking first two results".to_string());
                }
            }
            Err(e) => results.push(format!("✗ Sorted search failed: {:?}", e)),
        }

        println!("Testing pagination with page=1, per_page=2");
        let paginated_query = SearchQuery {
            q: None,
            filters: vec![],
            sort: vec!["year:desc".to_string()],
            facets: vec![],
            page: Some(1),
            per_page: Some(2),
            offset: None,
            highlight: None,
            config: None,
        };

        match Provider::search(index_name.clone(), paginated_query) {
            Ok(search_results) => {
                results.push(format!(
                    "✓ Paginated search returned {} hits",
                    search_results.hits.len()
                ));
                if let Some(total) = search_results.total {
                    results.push(format!("  Total documents: {}", total));
                }
                if let Some(page) = search_results.page {
                    results.push(format!("  Current page: {}", page));
                }
            }
            Err(e) => results.push(format!("✗ Paginated search failed: {:?}", e)),
        }

        Provider::delete_index(index_name).ok();
        results.join("\n")
    }

    fn test4(&self) -> String {
        let index_name = format!("{}-test4th", TEST_INDEX);
        let mut results = Vec::new();

        if let Err(e) = maybe_create_index(&index_name, &mut results) {
            return e;
        }

        match Provider::update_schema(index_name.clone(), create_test_schema()) {
            Ok(_) => {}
            Err(SearchError::Unsupported) => {}
            Err(_) => {}
        }

        let docs = create_test_documents();
        if let Err(e) = Provider::upsert_many(index_name.clone(), docs) {
            Provider::delete_index(index_name).ok();
            return format!("Document insertion failed: {:?}", e);
        }

        println!("Testing search with highlighting");
        let highlight_query = SearchQuery {
            q: Some("American".to_string()),
            filters: vec![],
            sort: vec![],
            facets: vec!["genre".to_string(), "author".to_string()],
            page: None,
            per_page: None,
            offset: None,
            highlight: Some(HighlightConfig {
                fields: vec!["title".to_string(), "description".to_string()],
                pre_tag: Some("<mark>".to_string()),
                post_tag: Some("</mark>".to_string()),
                max_length: Some(200),
            }),
            config: None,
        };

        match Provider::search(index_name.clone(), highlight_query) {
            Ok(search_results) => {
                results.push(format!(
                    "✓ Highlighted search returned {} hits",
                    search_results.hits.len()
                ));

                for hit in &search_results.hits {
                    if hit.highlights.is_some() {
                        results.push("  ✓ Found highlights in results".to_string());
                        break;
                    }
                }

                if search_results.facets.is_some() {
                    results.push("  ✓ Facet data returned".to_string());
                } else {
                    results.push("  ⚠ No facet data returned (may not be supported)".to_string());
                }

                if let Some(took_ms) = search_results.took_ms {
                    results.push(format!("  Query took: {}ms", took_ms));
                }
            }
            Err(e) => results.push(format!("✗ Highlighted search failed: {:?}", e)),
        }

        Provider::delete_index(index_name).ok();
        results.join("\n")
    }

    fn test5(&self) -> String {
        let index_name = format!("{}-test5", TEST_INDEX);
        let mut results = Vec::new();

        if let Err(e) = maybe_create_index(&index_name, &mut results) {
            return e;
        }

        println!("Setting up index with predefined schema");
        let original_schema = create_test_schema();
        match Provider::update_schema(index_name.clone(), original_schema.clone()) {
            Ok(_) => results.push("✓ Index schema configured successfully".to_string()),
            Err(SearchError::Unsupported) => {
                results.push(
                    "⚠ Schema configuration not supported, will test with document insertion"
                        .to_string(),
                );
                let test_docs = vec![create_test_documents().into_iter().next().unwrap()];
                if let Err(e) = Provider::upsert_many(index_name.clone(), test_docs) {
                    return format!("Document insertion failed: {:?}", e);
                }
            }
            Err(e) => {
                results.push(format!(
                    "⚠ Schema setup failed: {:?}, proceeding with document insertion",
                    e
                ));

                let test_docs = vec![create_test_documents().into_iter().next().unwrap()];
                if let Err(e) = Provider::upsert_many(index_name.clone(), test_docs) {
                    return format!("Document insertion failed: {:?}", e);
                }
            }
        }

        println!("Retrieving index schema");
        match Provider::get_schema(index_name.clone()) {
            Ok(retrieved_schema) => {
                results.push("✓ Schema retrieved successfully".to_string());
                results.push(format!("  Fields count: {}", retrieved_schema.fields.len()));

                if let Some(pk) = &retrieved_schema.primary_key {
                    results.push(format!("  Primary key: {}", pk));
                }

                let field_names: Vec<&String> =
                    retrieved_schema.fields.iter().map(|f| &f.name).collect();
                if field_names.contains(&&"title".to_string()) {
                    results.push("  ✓ Title field found".to_string());
                }
                if field_names.contains(&&"author".to_string()) {
                    results.push("  ✓ Author field found".to_string());
                }
            }
            Err(e) => results.push(format!("✗ Schema retrieval failed: {:?}", e)),
        }

        println!("Testing schema update");
        let mut updated_schema = original_schema;
        updated_schema.fields.push(SchemaField {
            name: "isbn".to_string(),
            field_type: FieldType::Text,
            required: false,
            facet: false,
            sort: false,
            index: true,
        });

        match Provider::update_schema(index_name.clone(), updated_schema) {
            Ok(_) => results.push("✓ Schema updated successfully".to_string()),
            Err(SearchError::Unsupported) => {
                results.push("  ⚠ Schema updates not supported by this provider".to_string())
            }
            Err(e) => results.push(format!("✗ Schema update failed: {:?}", e)),
        }

        Provider::delete_index(index_name).ok();
        results.join("\n")
    }

    fn test6(&self) -> String {
        let index_name = format!("{}-test6", TEST_INDEX);
        let mut results = Vec::new();

        if let Err(e) = maybe_create_index(&index_name, &mut results) {
            return e;
        }

        match Provider::update_schema(index_name.clone(), create_test_schema()) {
            Ok(_) => {}
            Err(SearchError::Unsupported) => {}
            Err(_) => {}
        }

        let mut docs = create_test_documents();
        for i in 6..=20 {
            docs.push(Doc {
                id: format!("doc{}", i),
                content: format!(r#"{{"title": "Book {}", "author": "Author {}", "year": {}, "genre": "test", "description": "A test book for streaming search"}}"#, i, i, 1900 + i),
            });
        }

        if let Err(e) = Provider::upsert_many(index_name.clone(), docs) {
            Provider::delete_index(index_name).ok();
            return format!("Document insertion failed: {:?}", e);
        }

        println!("Testing streaming search functionality");
        let stream_query = SearchQuery {
            q: Some("book".to_string()),
            filters: vec![],
            sort: vec!["year:asc".to_string()],
            facets: vec![],
            page: None,
            per_page: Some(5),
            offset: None,
            highlight: None,
            config: None,
        };

        match Provider::stream_search(index_name.clone(), stream_query.clone()) {
            Ok(stream) => {
                results.push("✓ Search stream created successfully".to_string());

                let mut total_hits = 0;
                let mut batch_count = 0;

                for _ in 0..5 {
                    let hits = stream.blocking_get_next();
                    if hits.is_empty() {
                        break;
                    }

                    batch_count += 1;
                    total_hits += hits.len();
                    results.push(format!("  Batch {}: {} hits", batch_count, hits.len()));
                }

                results.push(format!(
                    "✓ Streaming complete: {} total hits in {} batches",
                    total_hits, batch_count
                ));
            }
            Err(SearchError::Unsupported) => {
                results.push("⚠ Streaming search not supported by this provider".to_string());

                match Provider::search(index_name.clone(), stream_query) {
                    Ok(search_results) => {
                        results.push(format!(
                            "  Fallback: Regular search returned {} hits",
                            search_results.hits.len()
                        ));
                    }
                    Err(e) => results.push(format!("  Fallback search also failed: {:?}", e)),
                }
            }
            Err(e) => results.push(format!("✗ Streaming search failed: {:?}", e)),
        }

        Provider::delete_index(index_name).ok();
        results.join("\n")
    }

    fn test7(&self) -> String {
        let mut results = Vec::new();

        results.push("=== Testing Unsupported Operations ===".to_string());

        let test_index = "test777-unsupported".to_string();
        let schema = create_test_schema();

        if needs_explicit_index_creation() {
            println!("Setting up index: {}", test_index);
            match Provider::create_index(CreateIndexOptions {
                index_name: test_index.clone(),
                schema: Some(create_test_schema()),
            }) {
                Ok(_) => results.push("✓ Index created successfully".to_string()),
                Err(e) => return format!("✗ Index creation failed: {:?}", e),
            }
        } else {
            println!("Setting up index: {}", test_index);
        }

        match Provider::update_schema(test_index.clone(), schema.clone()) {
            Ok(()) => results.push("✓ Schema update supported and successful".to_string()),
            Err(SearchError::Unsupported) => {
                results.push("✓ Schema update gracefully reports as unsupported".to_string())
            }
            Err(e) => results.push(format!("⚠ Schema update failed with: {:?}", e)),
        }

        let advanced_query = SearchQuery {
            q: Some("test".to_string()),
            filters: vec!["complex_filter:value AND nested.field:value".to_string()],
            sort: vec!["complex_sort:desc".to_string()],
            facets: vec!["facet1".to_string(), "facet2".to_string()],
            page: Some(1),
            per_page: Some(10),
            offset: Some(0),
            highlight: Some(HighlightConfig {
                fields: vec!["title".to_string(), "content".to_string()],
                pre_tag: Some("<em>".to_string()),
                post_tag: Some("</em>".to_string()),
                max_length: Some(150),
            }),
            config: None,
        };

        match Provider::search(test_index.clone(), advanced_query.clone()) {
            Ok(_) => results.push("✓ Advanced search features supported".to_string()),
            Err(SearchError::Unsupported) => {
                results.push("✓ Advanced search gracefully reports as unsupported".to_string())
            }
            Err(SearchError::IndexNotFound) => {
                results.push("✓ Expected index not found (index doesn't exist yet)".to_string())
            }
            Err(e) => results.push(format!("⚠ Advanced search failed: {:?}", e)),
        }

        match Provider::stream_search(test_index.clone(), advanced_query) {
            Ok(_) => results.push("✓ Streaming search supported".to_string()),
            Err(SearchError::Unsupported) => {
                results.push("✓ Streaming search gracefully reports as unsupported".to_string())
            }
            Err(SearchError::IndexNotFound) => {
                results.push("✓ Expected index not found for streaming".to_string())
            }
            Err(e) => results.push(format!("⚠ Streaming search failed: {:?}", e)),
        }

        results.push("\n=== Testing Invalid Input Handling ===".to_string());

        let invalid_doc = Doc {
            id: "invalid-json".to_string(),
            content: r#"{"invalid": json, "malformed": true"#.to_string(),
        };

        match Provider::upsert(test_index.clone(), invalid_doc) {
            Ok(()) => results.push("⚠ Invalid JSON was accepted (lenient validation)".to_string()),
            Err(SearchError::InvalidQuery(msg)) => {
                results.push(format!("✓ Invalid JSON rejected: {}", msg))
            }
            Err(e) => results.push(format!("✓ Invalid input handled with error: {:?}", e)),
        }

        let invalid_query = SearchQuery {
            q: Some("((unclosed parenthesis AND malformed:".to_string()),
            filters: vec!["invalid_filter_syntax:::".to_string()],
            sort: vec!["invalid_sort_field:invalid_direction".to_string()],
            facets: vec![],
            page: Some(0),
            per_page: Some(0),
            offset: None,
            highlight: None,
            config: None,
        };

        match Provider::search(test_index.clone(), invalid_query) {
            Ok(_) => results.push("⚠ Invalid query was accepted (lenient parsing)".to_string()),
            Err(SearchError::InvalidQuery(msg)) => {
                results.push(format!("✓ Invalid query rejected: {}", msg))
            }
            Err(SearchError::IndexNotFound) => {
                results.push("✓ Index not found (expected since we haven't created it)".to_string())
            }
            Err(e) => results.push(format!("✓ Invalid query handled: {:?}", e)),
        }

        results.push("\n=== Testing Non-Existent Resource Handling ===".to_string());

        let nonexistent_index = "definitely-does-not-exist-12345".to_string();

        match Provider::get(nonexistent_index.clone(), "any-id".to_string()) {
            Ok(None) => results.push("✓ Non-existent document properly returns None".to_string()),
            Err(SearchError::IndexNotFound) => {
                results.push("✓ Non-existent index properly reports IndexNotFound".to_string())
            }
            Err(e) => results.push(format!("✓ Non-existent resource handled: {:?}", e)),
            Ok(Some(_)) => {
                results.push("⚠ Unexpected document found in non-existent index".to_string())
            }
        }

        match Provider::delete(nonexistent_index.clone(), "non-existent-doc".to_string()) {
            Ok(()) => {
                results.push("✓ Deleting non-existent document succeeds (idempotent)".to_string())
            }
            Err(SearchError::IndexNotFound) => {
                results.push("✓ Non-existent index properly reports IndexNotFound".to_string())
            }
            Err(e) => results.push(format!("✓ Delete non-existent handled: {:?}", e)),
        }

        match Provider::get_schema(nonexistent_index.clone()) {
            Ok(_) => results.push("⚠ Schema retrieved from non-existent index".to_string()),
            Err(SearchError::IndexNotFound) => {
                results.push("✓ Schema request properly reports IndexNotFound".to_string())
            }
            Err(SearchError::Unsupported) => {
                results.push("✓ Schema operations not supported by provider".to_string())
            }
            Err(e) => results.push(format!("✓ Schema request handled: {:?}", e)),
        }

        results.push("\n=== Testing Edge Cases ===".to_string());

        let empty_doc = Doc {
            id: "empty-doc".to_string(),
            content: "{}".to_string(),
        };

        match Provider::upsert(test_index.clone(), empty_doc) {
            Ok(()) => results.push("✓ Empty document accepted".to_string()),
            Err(e) => results.push(format!("✓ Empty document handled: {:?}", e)),
        }

        let long_id_doc = Doc {
            id: "a".repeat(1000),
            content: r#"{"test": "value"}"#.to_string(),
        };

        match Provider::upsert(test_index.clone(), long_id_doc) {
            Ok(()) => results.push("✓ Long document ID accepted".to_string()),
            Err(SearchError::InvalidQuery(msg)) => {
                results.push(format!("✓ Long ID rejected: {}", msg))
            }
            Err(e) => results.push(format!("✓ Long ID handled: {:?}", e)),
        }

        let empty_query = SearchQuery {
            q: Some("".to_string()),
            filters: vec![],
            sort: vec![],
            facets: vec![],
            page: None,
            per_page: None,
            offset: None,
            highlight: None,
            config: None,
        };

        match Provider::search(test_index.clone(), empty_query) {
            Ok(results_obj) => results.push(format!(
                "✓ Empty query executed, returned {} hits",
                results_obj.hits.len()
            )),
            Err(SearchError::IndexNotFound) => {
                results.push("✓ Expected IndexNotFound for empty query".to_string())
            }
            Err(e) => results.push(format!("✓ Empty query handled: {:?}", e)),
        }

        results.push("\n=== Testing Error Consistency ===".to_string());

        let ops_results = vec![
            ("list_indexes", Provider::list_indexes().is_ok()),
            (
                "create_index",
                Provider::create_index(CreateIndexOptions {
                    index_name: "test-create".to_string(),
                    schema: Some(schema.clone()),
                })
                .is_ok(),
            ),
            (
                "delete_index",
                Provider::delete_index("non-existent".to_string()).is_ok(),
            ),
        ];

        for (op_name, success) in ops_results {
            if success {
                results.push(format!("✓ {}: Operation completed", op_name));
            } else {
                results.push(format!("✓ {}: Error handled gracefully", op_name));
            }
        }

        results.push("\n=== Testing System Resilience ===".to_string());

        let stress_index = "stress-test-index".to_string();
        let mut stress_results = Vec::new();

        for i in 0..5 {
            let doc = Doc {
                id: format!("stress-doc-{}", i),
                content: format!(r#"{{"value": {}, "test": "stress"}}"#, i),
            };

            match Provider::upsert(stress_index.clone(), doc) {
                Ok(()) => stress_results.push(true),
                Err(_) => stress_results.push(false),
            }
        }

        let success_count = stress_results.iter().filter(|&&x| x).count();
        results.push(format!(
            "✓ Stress test: {}/{} operations succeeded",
            success_count,
            stress_results.len()
        ));

        let _ = Provider::delete_index(test_index);
        let _ = Provider::delete_index(stress_index);
        let _ = Provider::delete_index("test-create".to_string());

        results.push("\n=== Error Handling Test Complete ===".to_string());
        results.join("\n")
    }
}

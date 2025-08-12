#[allow(static_mut_refs)]
mod bindings;

use std::collections::HashSet;
use crate::bindings::exports::test::graph_exports::test_graph_api::*;
use crate::bindings::golem::graph::{
    connection::{self, ConnectionConfig, connect},
    schema::{self},
    query::{self, QueryResult, QueryOptions},
    transactions::{self, VertexSpec, EdgeSpec},
    traversal::{self, NeighborhoodOptions, PathOptions},
    types::{PropertyValue, Direction},
    errors::GraphError,
};

struct Component;

#[cfg(feature = "arangodb")]
const PROVIDER: &'static str = "arangodb";
#[cfg(feature = "janusgraph")]
const PROVIDER: &'static str = "janusgraph";
#[cfg(feature = "neo4j")]
const PROVIDER: &'static str = "neo4j";

const DEFAULT_TEST_HOST: &'static str = "127.0.0.1";

#[cfg(feature = "arangodb")]
const TEST_DATABASE: &'static str = "test";
#[cfg(feature = "arangodb")]
const TEST_PORT: u16 = 8529;
#[cfg(feature = "arangodb")]
const TEST_USERNAME: &'static str = "root";
#[cfg(feature = "arangodb")]
const TEST_PASSWORD: &'static str = "test";

#[cfg(feature = "janusgraph")]
const TEST_DATABASE: &'static str = "janusgraph";
#[cfg(feature = "janusgraph")]
const TEST_PORT: u16 = 8182;
#[cfg(feature = "janusgraph")]
const TEST_USERNAME: &'static str = "";
#[cfg(feature = "janusgraph")]
const TEST_PASSWORD: &'static str = "";

#[cfg(feature = "neo4j")]
const TEST_DATABASE: &'static str = "neo4j";
#[cfg(feature = "neo4j")]
const TEST_PORT: u16 = 7474;
#[cfg(feature = "neo4j")]
const TEST_USERNAME: &'static str = "neo4j";
#[cfg(feature = "neo4j")]
const TEST_PASSWORD: &'static str = "password";

fn get_test_host() -> String {
    std::env::var("GRAPH_TEST_HOST").unwrap_or_else(|_| DEFAULT_TEST_HOST.to_string())
}

// Helper function to ensure required collections exist for ArangoDB tests
#[cfg(feature = "arangodb")]
fn ensure_arangodb_collections(graph_connection: &crate::bindings::golem::graph::connection::Graph) -> Result<(), String> {
    use crate::bindings::golem::graph::schema::{self, ContainerType};
    
    println!("Setting up ArangoDB collections for testing...");
    
    let schema_manager = match schema::get_schema_manager(None) {
        Ok(manager) => manager,
        Err(error) => return Err(format!("Failed to get schema manager: {:?}", error)),
    };

    let required_collections = vec![
        ("Person", ContainerType::VertexContainer),
        ("TempUser", ContainerType::VertexContainer),
        ("Company", ContainerType::VertexContainer),
        ("Employee", ContainerType::VertexContainer),
        ("Node", ContainerType::VertexContainer),
        ("Product", ContainerType::VertexContainer),
        ("User", ContainerType::VertexContainer),
        ("KNOWS", ContainerType::EdgeContainer),
        ("WORKS_FOR", ContainerType::EdgeContainer),
        ("CONNECTS", ContainerType::EdgeContainer),
        ("FOLLOWS", ContainerType::EdgeContainer),
    ];

    let existing_containers = match schema_manager.list_containers() {
        Ok(containers) => containers,
        Err(error) => {
            println!("ERROR: Could not list existing containers: {:?}", error);
            vec![] 
        }
    };

    let existing_names: HashSet<String> = existing_containers
        .iter()
        .map(|c| c.name.clone())
        .collect();

    for (name, container_type) in required_collections {
        if existing_names.contains(name) {
            println!("Collection '{}' already exists", name);
            continue;
        }

        match schema_manager.create_container(name, container_type) {
            Ok(_) => println!("Collection '{}' created successfully", name),
            Err(error) => {
                println!("ERROR: Could not create collection '{}': {:?}", name, error);
            }
        }
    }

    println!("ArangoDB collection setup completed");
    Ok(())
}

#[cfg(not(feature = "arangodb"))]
fn ensure_arangodb_collections(_graph_connection: &crate::bindings::golem::graph::connection::Graph) -> Result<(), String> {
    Ok(())
}

impl Guest for Component {
    /// test1 demonstrates basic vertex creation and retrieval operations
    fn test1() -> String {
        println!("Starting test1: Basic vertex operations with {}", PROVIDER);
        
        let config = ConnectionConfig {
            hosts: vec![get_test_host()],
            port: Some(TEST_PORT),
            database_name: Some(TEST_DATABASE.to_string()),
            username: if TEST_USERNAME.is_empty() { None } else { Some(TEST_USERNAME.to_string()) },
            password: if TEST_PASSWORD.is_empty() { None } else { Some(TEST_PASSWORD.to_string()) },
            timeout_seconds: None,  
            max_connections: None,  
            provider_config: vec![],
        };

        println!("Connecting to graph database...");
        let graph_connection = match connection::connect(&config) {
            Ok(conn) => conn,
            Err(error) => {
                return format!("ERROR: Connection failed please ensure you are connected: {:?}", error);
            }
        };
        if let Err(error) = ensure_arangodb_collections(&graph_connection) {
            println!("ERROR: Collection setup failed: {}", error);
        }

        println!("Beginning transaction...");
        let transaction = match graph_connection.begin_transaction() {
            Ok(tx) => tx,
            Err(error) => {
                return format!("ERROR: Transaction creation failed: {:?}", error);
            }
        };

        let properties = vec![
            ("name".to_string(), PropertyValue::StringValue("Alice".to_string())),
            ("age".to_string(), PropertyValue::Int32(30)),
            ("active".to_string(), PropertyValue::Boolean(true)),
        ];

        println!("Creating vertex...");
        let vertex = match transaction.create_vertex("Person", &properties) {
            Ok(v) => v,
            Err(error) => return format!("ERROR: Vertex creation failed: {:?}", error),
        };

        println!("Created vertex with ID: {:?}", vertex.id);

        // Retrieving the vertex by ID
        let retrieved_vertex = match transaction.get_vertex(&vertex.id.clone()) {
            Ok(Some(v)) => v,
            Ok(None) => return "ERROR: Vertex not found after creation".to_string(),
            Err(error) => return format!("ERROR: Vertex retrieval failed: {:?}", error),
        };

        // Committing the transaction
        match transaction.commit() {
            Ok(_) => println!("Transaction committed successfully"),
            Err(error) => return format!("ERROR: Commit failed: {:?}", error),
        };

        let _ = graph_connection.close();

        format!(
            "SUCCESS [{}]: Created and retrieved vertex of type '{}' with ID {:?} and {} properties",
            PROVIDER,
            retrieved_vertex.vertex_type,
            retrieved_vertex.id,
            retrieved_vertex.properties.len()
        )
    }

    /// test2 demonstrates edge creation and relationship operations
    fn test2() -> String {
        println!("Starting test2: Edge operations with {}", PROVIDER);
        let mut results = Vec::new();
        
        let config = ConnectionConfig {
            hosts: vec![get_test_host()],
            port: Some(TEST_PORT),
            database_name: Some(TEST_DATABASE.to_string()),
            username: if TEST_USERNAME.is_empty() { None } else { Some(TEST_USERNAME.to_string()) },
            password: if TEST_PASSWORD.is_empty() { None } else { Some(TEST_PASSWORD.to_string()) },
            timeout_seconds: None,  
            max_connections: None,  
            provider_config: vec![],
        };

        let graph_connection = match connect(&config) {
            Ok(conn) => conn,
            Err(error) => {
                return format!("ERROR: Connection failed: {:?}", error);
            }
        };

        if let Err(error) = ensure_arangodb_collections(&graph_connection) {
            println!("ERROR: Collection setup failed: {}", error);
        } 

        let transaction = match graph_connection.begin_transaction() {
            Ok(tx) => tx,
            Err(error) => {
                return format!("ERROR: Transaction creation failed: {:?}", error);
            }
        };

        // Creating two vertices
        let person1_props = vec![
            ("name".to_string(), PropertyValue::StringValue("Bob".to_string())),
            ("age".to_string(), PropertyValue::Int32(25)),
        ];

        let person2_props = vec![
            ("name".to_string(), PropertyValue::StringValue("Carol".to_string())),
            ("age".to_string(), PropertyValue::Int32(28)),
        ];

        let vertex1 = match transaction.create_vertex("Person", &person1_props) {
            Ok(v) => v,
            Err(error) => return format!("ERROR: First vertex creation failed: {:?}", error),
        };

        let vertex2 = match transaction.create_vertex("Person", &person2_props) {
            Ok(v) => v,
            Err(error) => return format!("ERROR: Second vertex creation failed: {:?}", error),
        };

        let edge_props = vec![
            ("relationship".to_string(), PropertyValue::StringValue("FRIEND".to_string())),
            ("since".to_string(), PropertyValue::StringValue("2020-01-01".to_string())),
            ("weight".to_string(), PropertyValue::Float32Value(0.8)),
        ];

        let edge = match transaction.create_edge(
            "KNOWS",
            &vertex1.id.clone(),
            &vertex2.id.clone(),
            &edge_props,
        ) {
            Ok(e) => {
                println!("INFO: Successfully created edge: {:?} -> {:?} (type: {})", e.from_vertex, e.to_vertex, e.edge_type);
                e
            },
            Err(error) => return format!("ERROR: Edge creation failed: {:?}", error),
        };

        // Retrieve adjacent vertices
        let adjacent_vertices = match transaction.get_adjacent_vertices(
            &vertex1.id.clone(),
            Direction::Outgoing,
            Some(&["KNOWS".to_string()]),
            Some(10),
        ) {
            Ok(vertices) => {
                println!("INFO: Successfully found {} adjacent vertices using get_adjacent_vertices", vertices.len());
                results.push("Standard get_adjacent_vertices API succeeded".to_string());
                vertices
            },
            Err(error) => {
                println!("ERROR: get_adjacent_vertices failed: {:?}", error);
                results.push("get_adjacent_vertices failed".to_string());
                return format!("ERROR: Adjacent vertices retrieval failed: {:?} | Provider: {} | Edge: {:?} -> {:?} | Results: {:?}", 
                    error, PROVIDER, vertex1.id, vertex2.id, results);
            }
        };

        match transaction.commit() {
            Ok(_) => (),
            Err(error) => return format!("ERROR: Commit failed: {:?}", error),
        };

        let _ = graph_connection.close();

        format!(
            "SUCCESS [{}]: Created edge of type '{}' between vertices. Found {} adjacent vertices | Provider-specific handling: {:?}",
            PROVIDER,
            edge.edge_type,
            adjacent_vertices.len(),
            results
        )
    }

    /// test3 demonstrates transaction rollback and error handling
    fn test3() -> String {
        
        let config = ConnectionConfig {
            hosts: vec![get_test_host()],
            port: Some(TEST_PORT),
            database_name: Some(TEST_DATABASE.to_string()),
            username: if TEST_USERNAME.is_empty() { None } else { Some(TEST_USERNAME.to_string()) },
            password: if TEST_PASSWORD.is_empty() { None } else { Some(TEST_PASSWORD.to_string()) },
            timeout_seconds: None, 
            max_connections: None, 
            provider_config: vec![],
        };

        let graph_connection = match connect(&config) {
            Ok(conn) => conn,
            Err(error) => {
                return format!("ERROR: Connection failed: {:?}", error);
            }
        };

        if let Err(error) = ensure_arangodb_collections(&graph_connection) {
            println!("ERROR: Collection setup failed: {}", error);
        }

        let transaction = match graph_connection.begin_transaction() {
            Ok(tx) => tx,
            Err(error) => {
                return format!("ERROR: Transaction creation failed: {:?}", error);
            }
        };

        // Creating a vertex
        let properties = vec![
            ("name".to_string(), PropertyValue::StringValue("TestUser".to_string())),
            ("temp".to_string(), PropertyValue::Boolean(true)),
        ];

        let vertex = match transaction.create_vertex("TempUser", &properties) {
            Ok(v) => v,
            Err(error) => return format!("ERROR: Vertex creation failed: {:?}", error),
        };

        let is_active_before = transaction.is_active();
        
        // Intentionally rolling-back the transaction
        match transaction.rollback() {
            Ok(_) => println!("Transaction rolled back successfully"),
            Err(error) => return format!("ERROR: Rollback failed: {:?}", error),
        };

        let is_active_after = transaction.is_active();

        let _ = graph_connection.close();

        format!(
            "SUCCESS [{}]: Transaction test completed. Active before rollback: {}, after rollback: {}. Vertex ID was: {:?}",
            PROVIDER,
            is_active_before,
            is_active_after,
            vertex.id
        )
    }

    /// test4 demonstrates batch operations for creating multiple vertices and edges
    fn test4() -> String {
        println!("Starting test4: Batch operations with {}", PROVIDER);
        let mut results = Vec::new();
        
        let config = ConnectionConfig {
            hosts: vec![get_test_host()],
            port: Some(TEST_PORT),
            database_name: Some(TEST_DATABASE.to_string()),
            username: if TEST_USERNAME.is_empty() { None } else { Some(TEST_USERNAME.to_string()) },
            password: if TEST_PASSWORD.is_empty() { None } else { Some(TEST_PASSWORD.to_string()) },
            timeout_seconds: None,  
            max_connections: None,  
            provider_config: vec![],
        };

        let graph_connection = match connect(&config) {
            Ok(conn) => conn,
            Err(error) => {
                return format!("ERROR: Connection failed: {:?}", error);
            }
        };

        if let Err(error) = ensure_arangodb_collections(&graph_connection) {
            println!("ERROR: Collection setup failed: {}", error);
        }

        let transaction = match graph_connection.begin_transaction() {
            Ok(tx) => tx,
            Err(error) => {
                return format!("ERROR: Transaction creation failed: {:?}", error);
            }
        };

        // Creating multiple vertices in a batch
        let vertex_specs = vec![
            transactions::VertexSpec {
                vertex_type: "Company".to_string(),
                additional_labels: None,
                properties: vec![
                    ("name".to_string(), PropertyValue::StringValue("TechCorp".to_string())),
                    ("founded".to_string(), PropertyValue::Int32(2010)),
                ],
            },
            transactions::VertexSpec {
                vertex_type: "Company".to_string(),
                additional_labels: None,
                properties: vec![
                    ("name".to_string(), PropertyValue::StringValue("DataInc".to_string())),
                    ("founded".to_string(), PropertyValue::Int32(2015)),
                ],
            },
            transactions::VertexSpec {
                vertex_type: "Employee".to_string(),
                additional_labels: Some(vec!["Person".to_string()]),
                properties: vec![
                    ("name".to_string(), PropertyValue::StringValue("John".to_string())),
                    ("role".to_string(), PropertyValue::StringValue("Developer".to_string())),
                ],
            },
        ];

        let vertices = match transaction.create_vertices(&vertex_specs) {
            Ok(v) => {
                results.push("Standard batch vertex creation succeeded".to_string());
                v
            },
            Err(error) => {
                results.push("Batch vertex creation failed".to_string());
                return format!("ERROR: Batch vertex creation failed: {:?} | Results: {:?}", error, results);
            }
        };

        // Creating edges between the vertices
        if vertices.len() >= 3 {
            let edge_specs = vec![
                transactions::EdgeSpec {
                    edge_type: "WORKS_FOR".to_string(),
                    from_vertex: vertices[2].id.clone(),
                    to_vertex: vertices[0].id.clone(), 
                    properties: vec![
                        ("start_date".to_string(), PropertyValue::StringValue("2022-01-01".to_string())),
                        ("position".to_string(), PropertyValue::StringValue("Senior Developer".to_string())),
                    ],
                },
            ];

            let edges = match transaction.create_edges(&edge_specs) {
                Ok(e) => {
                    results.push("Standard batch edge creation succeeded".to_string());
                    e
                },
                Err(error) => {
                    results.push("Batch edge creation failed".to_string());
                    return format!("ERROR: Batch edge creation failed: {:?} | Results: {:?}", error, results);
                }
            };

            match transaction.commit() {
                Ok(_) => (),
                Err(error) => return format!("ERROR: Commit failed: {:?}", error),
            };

            let _ = graph_connection.close();

            format!(
                "SUCCESS [{}]: Created {} vertices and {} edges in batch operations | Provider-specific handling: {:?}",
                PROVIDER,
                vertices.len(),
                edges.len(),
                results
            )
        } else {
            format!("ERROR: Expected at least 3 vertices, got {} | Results: {:?}", vertices.len(), results)
        }
    }

    /// test5 demonstrates graph traversal and pathfinding operations
    fn test5() -> String {
        println!("Starting test5: Traversal operations with {}", PROVIDER);
        let mut results = Vec::new();
        
        let config = ConnectionConfig {
            hosts: vec![get_test_host()],
            port: Some(TEST_PORT),
            database_name: Some(TEST_DATABASE.to_string()),
            username: if TEST_USERNAME.is_empty() { None } else { Some(TEST_USERNAME.to_string()) },
            password: if TEST_PASSWORD.is_empty() { None } else { Some(TEST_PASSWORD.to_string()) },
            timeout_seconds: None, 
            max_connections: None, 
            provider_config: vec![],
        };

        let graph_connection = match connect(&config) {
            Ok(conn) => conn,
            Err(error) => return format!("ERROR: Connection failed: {:?}", error),
        };

        if let Err(error) = ensure_arangodb_collections(&graph_connection) {
            println!("ERROR: Collection setup failed: {}", error);
        }

        let transaction = match graph_connection.begin_transaction() {
            Ok(tx) => tx,
            Err(error) => return format!("ERROR: Transaction creation failed: {:?}", error),
        };

        // Create a small network: A -> B -> C
        let vertex_a = match transaction.create_vertex("Node", &vec![
            ("name".to_string(), PropertyValue::StringValue("A".to_string())),
        ]) {
            Ok(v) => v,
            Err(error) => return format!("ERROR: Vertex A creation failed: {:?}", error),
        };

        let vertex_b = match transaction.create_vertex("Node", &vec![
            ("name".to_string(), PropertyValue::StringValue("B".to_string())),
        ]) {
            Ok(v) => v,
            Err(error) => return format!("ERROR: Vertex B creation failed: {:?}", error),
        };

        let vertex_c = match transaction.create_vertex("Node", &vec![
            ("name".to_string(), PropertyValue::StringValue("C".to_string())),
        ]) {
            Ok(v) => v,
            Err(error) => return format!("ERROR: Vertex C creation failed: {:?}", error),
        };

        // Creating edges
        let _ = transaction.create_edge("CONNECTS", &vertex_a.id.clone(), &vertex_b.id.clone(), &vec![]);
        let _ = transaction.create_edge("CONNECTS", &vertex_b.id.clone(), &vertex_c.id.clone(), &vec![]);

        // Testing neighborhood exploration
        let neighborhood = match traversal::get_neighborhood(
            &transaction,
            &vertex_b.id.clone(),
            &traversal::NeighborhoodOptions {
                depth: 1,
                direction: Direction::Both,
                edge_types: Some(vec!["CONNECTS".to_string()]),
                max_vertices: Some(10),
            },
        ) {
            Ok(subgraph) => {
                results.push("Standard neighborhood exploration succeeded".to_string());
                subgraph
            },
            Err(error) => {
                results.push("Neighborhood exploration failed".to_string());
                return format!("ERROR: Neighborhood exploration failed: {:?} | Results: {:?}", error, results);
            }
        };

        // Test pathfinding
        let path_exists_result = match traversal::path_exists(
            &transaction,
            &vertex_a.id.clone(),
            &vertex_c.id.clone(),
            Some(&traversal::PathOptions {
                max_depth: Some(3),
                edge_types: Some(vec!["CONNECTS".to_string()]),
                vertex_types: None,
                vertex_filters: None,
                edge_filters: None,
            }),
        ) {
            Ok(exists) => {
                results.push("Standard path existence check succeeded".to_string());
                exists
            },
            Err(error) => {
                results.push("Path existence check failed".to_string());
                return format!("ERROR: Path existence check failed: {:?} | Results: {:?}", error, results);
            }
        };

        match transaction.commit() {
            Ok(_) => (),
            Err(error) => return format!("ERROR: Commit failed: {:?}", error),
        };

        let _ = graph_connection.close();

        format!(
            "SUCCESS [{}]: Traversal test completed. Neighborhood has {} vertices and {} edges. Path from A to C exists: {} | Provider-specific handling: {:?}",
            PROVIDER,
            neighborhood.vertices.len(),
            neighborhood.edges.len(),
            path_exists_result,
            results
        )
    }

    /// test6 demonstrates query operations using database-specific query languages
    fn test6() -> String {
        println!("Starting test6: Query operations with {}", PROVIDER);
        let mut results = Vec::new();
        
        let config = ConnectionConfig {
            hosts: vec![get_test_host()],
            port: Some(TEST_PORT),
            database_name: Some(TEST_DATABASE.to_string()),
            username: if TEST_USERNAME.is_empty() { None } else { Some(TEST_USERNAME.to_string()) },
            password: if TEST_PASSWORD.is_empty() { None } else { Some(TEST_PASSWORD.to_string()) },
            timeout_seconds: None,  
            max_connections: None, 
            provider_config: vec![],
        };

        let graph_connection = match connect(&config) {
            Ok(conn) => conn,
            Err(error) => return format!("ERROR: Connection failed: {:?}", error),
        };
        if let Err(error) = ensure_arangodb_collections(&graph_connection) {
            println!("ERROR: Collection setup failed: {}", error);
        }

        let transaction = match graph_connection.begin_transaction() {
            Ok(tx) => tx,
            Err(error) => return format!("ERROR: Transaction creation failed: {:?}", error),
        };

        // Creating some test data first
        let _ = transaction.create_vertex("Product", &vec![
            ("name".to_string(), PropertyValue::StringValue("Widget".to_string())),
            ("price".to_string(), PropertyValue::Float32Value(19.99)),
        ]);

        let _ = transaction.create_vertex("Product", &vec![
            ("name".to_string(), PropertyValue::StringValue("Gadget".to_string())),
            ("price".to_string(), PropertyValue::Float32Value(29.99)),
        ]);

        // Execute a provider-specific query
        let (query_string, parameters) = match PROVIDER {
            "neo4j" => {
                results.push("Using Neo4j Cypher query with parameters".to_string());
                ("MATCH (p:Product) WHERE p.price > $min_price RETURN p".to_string(),
                 vec![("min_price".to_string(), PropertyValue::Float32Value(15.0))])
            },
            "arangodb" => {
                results.push("Using ArangoDB AQL query with parameters".to_string());
                ("FOR p IN Product FILTER p.price > @min_price RETURN p".to_string(),
                 vec![("min_price".to_string(), PropertyValue::Float32Value(15.0))])
            },
            "janusgraph" => {
                results.push("Using JanusGraph Gremlin query with parameters".to_string());
                ("g.V().hasLabel('Product').has('price', gt(min_price))".to_string(),
                 vec![("min_price".to_string(), PropertyValue::Float32Value(15.0))])
            },
            _ => {
                results.push("Using generic SQL-like query".to_string());
                ("SELECT * FROM Product WHERE price > 15.0".to_string(), vec![])
            }
        };

        let query_result = match query::execute_query(
            &transaction,
            &query_string,
            if parameters.is_empty() { None } else { Some(&parameters) },
            Some(query::QueryOptions {
                timeout_seconds: Some(30),
                max_results: Some(100),
                explain: false,
                profile: false,
            }),
        ) {
            Ok(result) => {
                results.push("Standard query execution succeeded".to_string());
                result
            },
            Err(error) => {
                results.push("Query execution failed".to_string());
                return format!("ERROR: Query execution failed: {:?} | Results: {:?}", error, results);
            }
        };

        match transaction.commit() {
            Ok(_) => (),
            Err(error) => return format!("ERROR: Commit failed: {:?}", error),
        };

        let _ = graph_connection.close();

        let result_count = match &query_result.query_result_value {
            QueryResult::Vertices(vertices) => vertices.len(),
            QueryResult::Maps(maps) => maps.len(),
            QueryResult::Values(values) => values.len(),
            _ => 0,
        };

        format!(
            "SUCCESS [{}]: Query executed successfully. Found {} results. Execution time: {:?}ms | Provider-specific handling: {:?}",
            PROVIDER,
            result_count,
            query_result.execution_time_ms,
            results
        )
    }

    /// test7 demonstrates schema management operations
    fn test7() -> String {
        println!("Starting test7: Schema operations with {}", PROVIDER);
        
        // Testing schema manager creation
        let schema_manager = match schema::get_schema_manager(None) {
            Ok(manager) => manager,
            Err(error) => {
                return format!("ERROR: Schema manager creation failed: {:?}", error);
            }
        };

        let mut vertex_count = 0;
        let mut edge_count = 0;
        let mut index_count = 0;

        match schema_manager.list_vertex_labels() {
            Ok(labels) => {
                vertex_count = labels.len();
                println!("Found {} vertex labels", vertex_count);
            }
            Err(error) => {
                println!("ERROR: Could not list vertex labels: {:?}", error);
            }
        }

        match schema_manager.list_edge_labels() {
            Ok(labels) => {
                edge_count = labels.len();
                println!("Found {} edge labels", edge_count);
            }
            Err(error) => {
                println!("ERROR: Could not list edge labels: {:?}", error);
            }
        }

        // Trying to list indexes
        match schema_manager.list_indexes() {
            Ok(idx_list) => {
                index_count = idx_list.len();
                println!("Found {} indexes", index_count);
            }
            Err(error) => {
                println!("ERROR: Could not list indexes: {:?}", error);
            }
        }

        format!(
            "SUCCESS {} Schema operations completed. Found {} vertex labels, {} edge labels, and {} indexes",
            PROVIDER,
            vertex_count,
            edge_count,
            index_count
        )
    }
}

bindings::export!(Component with_types_in bindings);


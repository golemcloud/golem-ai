#[allow(static_mut_refs)]
mod bindings;

use golem_rust::atomically;
use crate::bindings::test::helper_client::test_helper_client::TestHelperApi;
use crate::bindings::exports::test::graph_exports::test_graph_api::*;
use crate::bindings::golem::graph::connection::{ connect, ConnectionConfig };
use crate::bindings::golem::graph::types::{
    PropertyValue,
    Direction,
    ComparisonOperator,
    FilterCondition,
};
use crate::bindings::golem::graph::schema::{
    get_schema_manager,
    PropertyType,
    PropertyDefinition,
    VertexLabelSchema,
    IndexType,
    IndexDefinition,
};
use crate::bindings::golem::graph::query::execute_query;
use crate::bindings::golem::graph::traversal::{
    find_shortest_path,
    get_neighborhood,
    NeighborhoodOptions,
};
use crate::bindings::golem::graph::errors::GraphError;

struct Component;

#[cfg(feature = "neo4j")]
const PROVIDER: &'static str = "neo4j";
#[cfg(feature = "arangodb")]
const PROVIDER: &'static str = "arangodb";
#[cfg(feature = "janusgraph")]
const PROVIDER: &'static str = "janusgraph";

impl Guest for Component {
    /// test1: Basic CRUD operations - create, read, update, delete vertices and edges
    fn test1() -> String {
        let config = get_connection_config();

        let graph = match connect(&config) {
            Ok(g) => g,
            Err(e) => {
                return format!("Connection failed: {:?}", e);
            }
        };

        let mut output = String::new();
        output.push_str(&format!("Connected to {} successfully!\n\n", PROVIDER));

        let transaction = match graph.begin_transaction() {
            Ok(tx) => tx,
            Err(e) => {
                return format!("Failed to begin transaction: {:?}", e);
            }
        };

        // Create vertices
        let person_props = vec![
            ("name".to_string(), PropertyValue::StringValue("Alice".to_string())),
            ("age".to_string(), PropertyValue::Int32(30))
        ];

        let company_props = vec![
            ("name".to_string(), PropertyValue::StringValue("TechCorp".to_string())),
            ("industry".to_string(), PropertyValue::StringValue("Technology".to_string()))
        ];

        let alice = match transaction.create_vertex("Person", &person_props) {
            Ok(v) => {
                output.push_str(
                    &format!("Created vertex: {} ({})\n", v.vertex_type, format_element_id(&v.id))
                );
                v
            }
            Err(e) => {
                return format!("Failed to create Alice vertex: {:?}", e);
            }
        };

        let techcorp = match transaction.create_vertex("Company", &company_props) {
            Ok(v) => {
                output.push_str(
                    &format!("Created vertex: {} ({})\n", v.vertex_type, format_element_id(&v.id))
                );
                v
            }
            Err(e) => {
                return format!("Failed to create TechCorp vertex: {:?}", e);
            }
        };

        // Create edge
        let works_props = vec![
            ("since".to_string(), PropertyValue::StringValue("2020".to_string())),
            ("position".to_string(), PropertyValue::StringValue("Developer".to_string()))
        ];

        let _works_edge = match
            transaction.create_edge(
                "WORKS_FOR",
                &alice.id.clone(),
                &techcorp.id.clone(),
                &works_props
            )
        {
            Ok(e) => {
                output.push_str(
                    &format!("Created edge: {} ({})\n", e.edge_type, format_element_id(&e.id))
                );
                e
            }
            Err(e) => {
                return format!("Failed to create WORKS_FOR edge: {:?}", e);
            }
        };

        // Read operations
        match transaction.get_vertex(&alice.id.clone()) {
            Ok(Some(vertex)) => {
                output.push_str(
                    &format!("Retrieved Alice: {} properties\n", vertex.properties.len())
                );
            }
            Ok(None) => output.push_str("Alice vertex not found\n"),
            Err(e) => {
                return format!("Failed to get Alice vertex: {:?}", e);
            }
        }

        // Update vertex
        let updated_props = vec![
            ("name".to_string(), PropertyValue::StringValue("Alice Smith".to_string())),
            ("age".to_string(), PropertyValue::Int32(31)),
            ("city".to_string(), PropertyValue::StringValue("San Francisco".to_string()))
        ];

        match transaction.update_vertex(&alice.id.clone(), &updated_props) {
            Ok(_) => output.push_str("Updated Alice vertex successfully\n"),
            Err(e) => {
                return format!("Failed to update Alice vertex: {:?}", e);
            }
        }

        // Commit transaction
        match transaction.commit() {
            Ok(_) => output.push_str("Transaction committed successfully\n"),
            Err(e) => {
                return format!("Failed to commit transaction: {:?}", e);
            }
        }

        output.push_str(&format!("\nBasic CRUD operations completed with {} provider\n", PROVIDER));
        output
    }

    /// test2: Transaction lifecycle with crash simulation and recovery
    fn test2() -> String {
        let config = get_connection_config();

        let graph = match connect(&config) {
            Ok(g) => g,
            Err(e) => {
                return format!("Connection failed: {:?}", e);
            }
        };

        let mut output = String::new();
        output.push_str(&format!("Testing transaction lifecycle with {} provider\n\n", PROVIDER));

        let name = std::env::var("GOLEM_WORKER_NAME").unwrap();
        let mut round = 0;

        let transaction = match graph.begin_transaction() {
            Ok(tx) => {
                output.push_str("Transaction started successfully\n");
                tx
            }
            Err(e) => {
                return format!("Failed to begin transaction: {:?}", e);
            }
        };

        let props1 = vec![
            ("name".to_string(), PropertyValue::StringValue("Node1".to_string())),
            ("value".to_string(), PropertyValue::Int32(100))
        ];

        let vertex1 = match transaction.create_vertex("TestNode", &props1) {
            Ok(v) => {
                output.push_str(&format!("Created first vertex: {}\n", format_element_id(&v.id)));
                v
            }
            Err(e) => {
                return format!("Failed to create first vertex: {:?}", e);
            }
        };

        round += 1;

        if round == 1 {
            atomically(|| {
                let client = TestHelperApi::new(&name);
                let answer = client.blocking_inc_and_get();
                if answer == 1 {
                    panic!("Simulating crash during transaction")
                }
            });
        }

        let props2 = vec![
            ("name".to_string(), PropertyValue::StringValue("Node2".to_string())),
            ("value".to_string(), PropertyValue::Int32(200))
        ];

        let vertex2 = match transaction.create_vertex("TestNode", &props2) {
            Ok(v) => {
                output.push_str(
                    &format!("Created second vertex after recovery: {}\n", format_element_id(&v.id))
                );
                v
            }
            Err(e) => {
                return format!("Failed to create second vertex: {:?}", e);
            }
        };

        let edge_props = vec![(
            "relationship".to_string(),
            PropertyValue::StringValue("connects".to_string()),
        )];

        match transaction.create_edge("CONNECTS", &vertex1.id, &vertex2.id, &edge_props) {
            Ok(e) => output.push_str(&format!("Created edge: {}\n", format_element_id(&e.id))),
            Err(e) => {
                return format!("Failed to create edge: {:?}", e);
            }
        }

        if transaction.is_active() {
            output.push_str("Transaction is still active before commit\n");
        }

        match transaction.commit() {
            Ok(_) => {
                output.push_str("Transaction committed successfully after crash recovery\n");
                if !transaction.is_active() {
                    output.push_str("Transaction is no longer active after commit\n");
                }
            }
            Err(e) => {
                return format!("Failed to commit transaction: {:?}", e);
            }
        }

        output.push_str(
            &format!("\nTransaction lifecycle test completed with {} provider\n", PROVIDER)
        );
        output
    }

    /// test3: Schema operations - type definitions, indexes, constraints
    fn test3() -> String {
        let config = get_connection_config();

        let graph = match connect(&config) {
            Ok(g) => g,
            Err(e) => {
                return format!("Connection failed: {:?}", e);
            }
        };

        let mut output = String::new();
        output.push_str(&format!("Testing schema operations with {} provider\n\n", PROVIDER));

        let schema_manager = match get_schema_manager() {
            Ok(sm) => sm,
            Err(e) => {
                output.push_str(
                    &format!("Schema operations not supported by {}: {:?}\n", PROVIDER, e)
                );
                return output;
            }
        };

        let user_schema = VertexLabelSchema {
            label: "User".to_string(),
            properties: vec![
                PropertyDefinition {
                    name: "username".to_string(),
                    property_type: PropertyType::StringType,
                    required: true,
                    unique: true,
                    default_value: None,
                },
                PropertyDefinition {
                    name: "email".to_string(),
                    property_type: PropertyType::StringType,
                    required: true,
                    unique: true,
                    default_value: None,
                },
                PropertyDefinition {
                    name: "age".to_string(),
                    property_type: PropertyType::Int32,
                    required: false,
                    unique: false,
                    default_value: Some(PropertyValue::Int32(0)),
                }
            ],
            container: None,
        };

        match schema_manager.define_vertex_label(&user_schema) {
            Ok(_) => output.push_str("Defined User vertex label schema\n"),
            Err(e) => output.push_str(&format!("Failed to define User schema: {:?}\n", e)),
        }

        let username_index = IndexDefinition {
            name: "idx_username".to_string(),
            label: "User".to_string(),
            properties: vec!["username".to_string()],
            index_type: IndexType::Exact,
            unique: true,
            container: None,
        };

        match schema_manager.create_index(&username_index) {
            Ok(_) => output.push_str("Created username index\n"),
            Err(e) => output.push_str(&format!("Failed to create username index: {:?}\n", e)),
        }

        match schema_manager.list_vertex_labels() {
            Ok(labels) => {
                output.push_str(&format!("Vertex labels: {}\n", labels.join(", ")));
            }
            Err(e) => output.push_str(&format!("Failed to list vertex labels: {:?}\n", e)),
        }

        match schema_manager.list_indexes() {
            Ok(indexes) => {
                output.push_str(&format!("Found {} indexes\n", indexes.len()));
                for idx in indexes {
                    output.push_str(
                        &format!("  - {}: {:?} on {}\n", idx.name, idx.index_type, idx.label)
                    );
                }
            }
            Err(e) => output.push_str(&format!("Failed to list indexes: {:?}\n", e)),
        }

        output.push_str(
            &format!("\nSchema operations test completed with {} provider\n", PROVIDER)
        );
        output
    }

    /// test4: Query execution with various complexity levels
    fn test4() -> String {
        let config = get_connection_config();

        let graph = match connect(&config) {
            Ok(g) => g,
            Err(e) => {
                return format!("Connection failed: {:?}", e);
            }
        };

        let mut output = String::new();
        output.push_str(&format!("Testing query execution with {} provider\n\n", PROVIDER));

        let transaction = match graph.begin_transaction() {
            Ok(tx) => tx,
            Err(e) => {
                return format!("Failed to begin transaction: {:?}", e);
            }
        };

        let props = vec![
            ("name".to_string(), PropertyValue::StringValue("QueryTest".to_string())),
            ("score".to_string(), PropertyValue::Int32(95))
        ];

        match transaction.create_vertex("TestEntity", &props) {
            Ok(v) =>
                output.push_str(&format!("Created test vertex: {}\n", format_element_id(&v.id))),
            Err(e) => {
                return format!("Failed to create test vertex: {:?}", e);
            }
        }

        let filter_condition = crate::bindings::golem::graph::types::FilterCondition {
            property: "score".to_string(),
            operator: crate::bindings::golem::graph::types::ComparisonOperator::GreaterThan,
            value: PropertyValue::Int32(90),
        };

        match
            transaction.find_vertices(
                Some("TestEntity"),
                Some(&vec![filter_condition]),
                None,
                Some(100),
                None
            )
        {
            Ok(vertices) => {
                output.push_str(&format!("Query executed successfully\n"));
                output.push_str(&format!("Found {} vertices with score > 90\n", vertices.len()));

                for vertex in vertices.iter().take(5) {
                    if let Some(name) = vertex.properties.iter().find(|(k, _)| k == "name") {
                        if let Some(score) = vertex.properties.iter().find(|(k, _)| k == "score") {
                            output.push_str(&format!("  - {:?}: {:?}\n", name.1, score.1));
                        }
                    }
                }
            }
            Err(e) => output.push_str(&format!("Query execution failed: {:?}\n", e)),
        }

        match transaction.commit() {
            Ok(_) => output.push_str("Query test transaction committed\n"),
            Err(e) => {
                return format!("Failed to commit query test transaction: {:?}", e);
            }
        }

        output.push_str(&format!("\nQuery execution test completed with {} provider\n", PROVIDER));
        output
    }

    /// test5: Traversal operations - pathfinding, neighborhood exploration
    fn test5() -> String {
        let config = get_connection_config();

        let graph = match connect(&config) {
            Ok(g) => g,
            Err(e) => {
                return format!("Connection failed: {:?}", e);
            }
        };

        let mut output = String::new();
        output.push_str(&format!("Testing traversal operations with {} provider\n\n", PROVIDER));

        let transaction = match graph.begin_transaction() {
            Ok(tx) => tx,
            Err(e) => {
                return format!("Failed to begin transaction: {:?}", e);
            }
        };

        let node_a = match
            transaction.create_vertex(
                "Node",
                &vec![("name".to_string(), PropertyValue::StringValue("A".to_string()))]
            )
        {
            Ok(v) => v,
            Err(e) => {
                return format!("Failed to create node A: {:?}", e);
            }
        };

        let node_b = match
            transaction.create_vertex(
                "Node",
                &vec![("name".to_string(), PropertyValue::StringValue("B".to_string()))]
            )
        {
            Ok(v) => v,
            Err(e) => {
                return format!("Failed to create node B: {:?}", e);
            }
        };

        let node_c = match
            transaction.create_vertex(
                "Node",
                &vec![("name".to_string(), PropertyValue::StringValue("C".to_string()))]
            )
        {
            Ok(v) => v,
            Err(e) => {
                return format!("Failed to create node C: {:?}", e);
            }
        };

        match transaction.create_edge("CONNECTS", &node_a.id.clone(), &node_b.id.clone(), &vec![]) {
            Ok(_) => output.push_str("Created edge A->B\n"),
            Err(e) => {
                return format!("Failed to create edge A->B: {:?}", e);
            }
        }

        match transaction.create_edge("CONNECTS", &node_b.id.clone(), &node_c.id.clone(), &vec![]) {
            Ok(_) => output.push_str("Created edge B->C\n"),
            Err(e) => {
                return format!("Failed to create edge B->C: {:?}", e);
            }
        }

        match find_shortest_path(&transaction, &node_a.id.clone(), &node_c.id.clone(), None) {
            Ok(Some(path)) => {
                output.push_str(&format!("Found shortest path A->C: length {}\n", path.length));
                output.push_str(
                    &format!(
                        "Path has {} vertices and {} edges\n",
                        path.vertices.len(),
                        path.edges.len()
                    )
                );
            }
            Ok(None) => output.push_str("No path found from A to C\n"),
            Err(e) => output.push_str(&format!("Pathfinding failed: {:?}\n", e)),
        }

        let neighborhood_opts = NeighborhoodOptions {
            depth: 2,
            direction: Direction::Outgoing,
            edge_types: Some(vec!["CONNECTS".to_string()]),
            max_vertices: Some(10),
        };

        match get_neighborhood(&transaction, &node_a.id.clone(), &neighborhood_opts) {
            Ok(subgraph) => {
                output.push_str(
                    &format!(
                        "Neighborhood of A: {} vertices, {} edges\n",
                        subgraph.vertices.len(),
                        subgraph.edges.len()
                    )
                );
            }
            Err(e) => output.push_str(&format!("Neighborhood exploration failed: {:?}\n", e)),
        }

        match transaction.get_adjacent_vertices(&node_b.id, Direction::Both, None, None) {
            Ok(adjacent) => {
                output.push_str(&format!("Node B has {} adjacent vertices\n", adjacent.len()));
            }
            Err(e) => output.push_str(&format!("Failed to get adjacent vertices: {:?}\n", e)),
        }

        match transaction.commit() {
            Ok(_) => output.push_str("Traversal test transaction committed\n"),
            Err(e) => {
                return format!("Failed to commit traversal test transaction: {:?}", e);
            }
        }

        output.push_str(
            &format!("\nTraversal operations test completed with {} provider\n", PROVIDER)
        );
        output
    }

    /// test6: Batch operations and upserts
    fn test6() -> String {
        let config = get_connection_config();

        let graph = match connect(&config) {
            Ok(g) => g,
            Err(e) => {
                return format!("Connection failed: {:?}", e);
            }
        };

        let mut output = String::new();
        output.push_str(&format!("Testing batch operations with {} provider\n\n", PROVIDER));

        let transaction = match graph.begin_transaction() {
            Ok(tx) => tx,
            Err(e) => {
                return format!("Failed to begin transaction: {:?}", e);
            }
        };

        let vertex_specs = vec![
            crate::bindings::golem::graph::transactions::VertexSpec {
                vertex_type: "BatchNode".to_string(),
                additional_labels: None,
                properties: vec![
                    ("name".to_string(), PropertyValue::StringValue("Batch1".to_string())),
                    ("value".to_string(), PropertyValue::Int32(10))
                ],
            },
            crate::bindings::golem::graph::transactions::VertexSpec {
                vertex_type: "BatchNode".to_string(),
                additional_labels: None,
                properties: vec![
                    ("name".to_string(), PropertyValue::StringValue("Batch2".to_string())),
                    ("value".to_string(), PropertyValue::Int32(20))
                ],
            },
            crate::bindings::golem::graph::transactions::VertexSpec {
                vertex_type: "BatchNode".to_string(),
                additional_labels: None,
                properties: vec![
                    ("name".to_string(), PropertyValue::StringValue("Batch3".to_string())),
                    ("value".to_string(), PropertyValue::Int32(30))
                ],
            }
        ];

        let created_vertices = match transaction.create_vertices(&vertex_specs) {
            Ok(vertices) => {
                output.push_str(&format!("Batch created {} vertices\n", vertices.len()));
                vertices
            }
            Err(e) => {
                return format!("Failed to batch create vertices: {:?}", e);
            }
        };

        if created_vertices.len() >= 2 {
            let edge_specs = vec![
                crate::bindings::golem::graph::transactions::EdgeSpec {
                    edge_type: "BATCH_CONNECTS".to_string(),
                    from_vertex: created_vertices[0].id.clone(),
                    to_vertex: created_vertices[1].id.clone(),
                    properties: vec![("weight".to_string(), PropertyValue::Float32Value(1.0))],
                },
                crate::bindings::golem::graph::transactions::EdgeSpec {
                    edge_type: "BATCH_CONNECTS".to_string(),
                    from_vertex: created_vertices[1].id.clone(),
                    to_vertex: created_vertices[2].id.clone(),
                    properties: vec![("weight".to_string(), PropertyValue::Float32Value(2.0))],
                }
            ];

            match transaction.create_edges(&edge_specs) {
                Ok(edges) => output.push_str(&format!("Batch created {} edges\n", edges.len())),
                Err(e) => {
                    return format!("Failed to batch create edges: {:?}", e);
                }
            }
        }

        let upsert_props = vec![
            ("name".to_string(), PropertyValue::StringValue("UpsertTest".to_string())),
            ("value".to_string(), PropertyValue::Int32(999))
        ];

        match transaction.upsert_vertex(None, "UpsertNode", &upsert_props.clone()) {
            Ok(_) => output.push_str("Upserted vertex (created new)\n"),
            Err(e) => {
                return format!("Failed to upsert vertex: {:?}", e);
            }
        }

        let filters = vec![FilterCondition {
            property: "value".to_string(),
            operator: ComparisonOperator::GreaterThan,
            value: PropertyValue::Int32(15),
        }];

        match transaction.find_vertices(Some("BatchNode"), Some(&filters), None, None, None) {
            Ok(found_vertices) => {
                output.push_str(
                    &format!("Found {} vertices with value > 15\n", found_vertices.len())
                );
            }
            Err(e) => {
                return format!("Failed to find vertices: {:?}", e);
            }
        }

        match transaction.commit() {
            Ok(_) => output.push_str("Batch operations transaction committed\n"),
            Err(e) => {
                return format!("Failed to commit batch operations transaction: {:?}", e);
            }
        }

        output.push_str(&format!("\nBatch operations test completed with {} provider\n", PROVIDER));
        output
    }

    /// test7: Error handling for unsupported operations
    fn test7() -> String {
        let config = get_connection_config();

        let graph = match connect(&config) {
            Ok(g) => g,
            Err(e) => {
                return format!("Connection failed: {:?}", e);
            }
        };

        let mut output = String::new();
        output.push_str(&format!("Testing error handling with {} provider\n\n", PROVIDER));

        let transaction = match graph.begin_transaction() {
            Ok(tx) => tx,
            Err(e) => {
                return format!("Failed to begin transaction: {:?}", e);
            }
        };

        let invalid_query = "THIS IS NOT A VALID QUERY SYNTAX!!!";
        match execute_query(&transaction, invalid_query, None, None) {
            Ok(_) => output.push_str("WARNING: Invalid query unexpectedly succeeded\n"),
            Err(GraphError::InvalidQuery(msg)) => {
                output.push_str(&format!("Correctly caught invalid query error: {}\n", msg));
            }
            Err(e) =>
                output.push_str(&format!("Invalid query returned different error: {:?}\n", e)),
        }

        let fake_id = crate::bindings::golem::graph::types::ElementId::StringValue(
            "non-existent-id".to_string()
        );
        match transaction.get_vertex(&fake_id.clone()) {
            Ok(None) => output.push_str("Correctly returned None for non-existent vertex\n"),
            Ok(Some(_)) => output.push_str("WARNING: Non-existent vertex unexpectedly found\n"),
            Err(GraphError::ElementNotFound(_)) => {
                output.push_str("Correctly caught element not found error\n");
            }
            Err(e) =>
                output.push_str(
                    &format!("Non-existent vertex returned different error: {:?}\n", e)
                ),
        }

        let vertex_with_complex_props = vec![
            ("name".to_string(), PropertyValue::StringValue("Test".to_string())),
            (
                "complex_data".to_string(),
                PropertyValue::StringValue("complex_data_value".to_string()),
            )
        ];

        match transaction.create_vertex("ComplexTest", &vertex_with_complex_props) {
            Ok(_) => output.push_str("Complex property types supported\n"),
            Err(GraphError::InvalidPropertyType(msg)) => {
                output.push_str(&format!("Complex property type not supported: {}\n", msg));
            }
            Err(e) => output.push_str(&format!("Complex property creation failed: {:?}\n", e)),
        }

        let user1_props = vec![
            ("username".to_string(), PropertyValue::StringValue("duplicate_user".to_string())),
            ("email".to_string(), PropertyValue::StringValue("user@test.com".to_string()))
        ];

        let user2_props = vec![
            ("username".to_string(), PropertyValue::StringValue("duplicate_user".to_string())),
            ("email".to_string(), PropertyValue::StringValue("user2@test.com".to_string()))
        ];

        match transaction.create_vertex("User", &user1_props) {
            Ok(_) => output.push_str("Created first user\n"),
            Err(e) => output.push_str(&format!("Failed to create first user: {:?}\n", e)),
        }

        match transaction.create_vertex("User", &user2_props) {
            Ok(_) =>
                output.push_str(
                    "WARNING: Duplicate user creation succeeded (no constraints enforced)\n"
                ),
            Err(GraphError::ConstraintViolation(msg)) => {
                output.push_str(&format!("Correctly caught constraint violation: {}\n", msg));
            }
            Err(GraphError::DuplicateElement(_)) => {
                output.push_str("Correctly caught duplicate element error\n");
            }
            Err(e) =>
                output.push_str(
                    &format!("Duplicate user creation failed with different error: {:?}\n", e)
                ),
        }

        match transaction.commit() {
            Ok(_) => output.push_str("Error handling test transaction committed\n"),
            Err(e) => {
                return format!("Failed to commit error handling test transaction: {:?}", e);
            }
        }

        output.push_str(&format!("\nError handling test completed with {} provider\n", PROVIDER));
        output
    }

    /// test8: Connection management and configuration with durability verification
    fn test8() -> String {
        let mut output = String::new();
        output.push_str(
            &format!("Testing connection management and durability with {} provider\n\n", PROVIDER)
        );

        let config = get_connection_config();
        output.push_str(&format!("Connection config created for {}\n", PROVIDER));

        let graph = match connect(&config) {
            Ok(g) => {
                output.push_str("Successfully connected to graph database\n");
                g
            }
            Err(e) => {
                return format!("Connection failed: {:?}", e);
            }
        };

        match graph.ping() {
            Ok(_) => output.push_str("Connection health check passed\n"),
            Err(e) => {
                return format!("Connection health check failed: {:?}", e);
            }
        }

        let transaction = match graph.begin_transaction() {
            Ok(tx) => {
                output.push_str("Transaction started for durability test\n");
                tx
            }
            Err(e) => {
                return format!("Failed to begin durability test transaction: {:?}", e);
            }
        };

        let persistent_props = vec![
            ("name".to_string(), PropertyValue::StringValue("DurabilityTest".to_string())),
            (
                "created_at".to_string(),
                PropertyValue::StringValue(
                    std::time::SystemTime
                        ::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        .to_string()
                ),
            ),
            ("test_id".to_string(), PropertyValue::StringValue("durability_test_node".to_string()))
        ];

        let _persistent_vertex = match
            transaction.create_vertex("PersistentNode", &persistent_props)
        {
            Ok(v) => {
                output.push_str(
                    &format!("Created persistent vertex: {}\n", format_element_id(&v.id))
                );
                v
            }
            Err(e) => {
                return format!("Failed to create persistent vertex: {:?}", e);
            }
        };

        // Commit to ensure data is persisted
        match transaction.commit() {
            Ok(_) => output.push_str("Durability test data committed successfully\n"),
            Err(e) => {
                return format!("Failed to commit durability test data: {:?}", e);
            }
        }

        // Simulate disconnection by dropping the graph connection
        drop(graph);
        output.push_str("Disconnected from graph database\n");

        // Wait a moment to simulate network interruption
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Reconnect to test durability
        let graph_reconnected = match connect(&config) {
            Ok(g) => {
                output.push_str("Successfully reconnected to graph database\n");
                g
            }
            Err(e) => {
                return format!("Failed to reconnect: {:?}", e);
            }
        };

        // Verify that our persistent data survived
        let verify_transaction = match graph_reconnected.begin_transaction() {
            Ok(tx) => tx,
            Err(e) => {
                return format!("Failed to begin verification transaction: {:?}", e);
            }
        };

        // Try to find our persistent vertex
        let filters = vec![FilterCondition {
            property: "test_id".to_string(),
            operator: ComparisonOperator::Equal,
            value: PropertyValue::StringValue("durability_test_node".to_string()),
        }];

        match
            verify_transaction.find_vertices(
                Some("PersistentNode"),
                Some(&filters),
                None,
                None,
                None
            )
        {
            Ok(found_vertices) => {
                if found_vertices.is_empty() {
                    output.push_str("WARNING: Persistent data not found after reconnection\n");
                } else {
                    output.push_str(
                        &format!(
                            "SUCCESS: Found {} persistent vertices after reconnection\n",
                            found_vertices.len()
                        )
                    );
                    for vertex in found_vertices {
                        output.push_str(
                            &format!(
                                "  Verified vertex: {} with {} properties\n",
                                format_element_id(&vertex.id),
                                vertex.properties.len()
                            )
                        );
                    }
                }
            }
            Err(e) => output.push_str(&format!("Failed to verify persistent data: {:?}\n", e)),
        }

        // Test connection configuration options
        output.push_str("\nConnection Configuration Details:\n");
        output.push_str(&format!("  Provider: {}\n", PROVIDER));

        // Test connection statistics (if supported)
        match graph_reconnected.get_statistics() {
            Ok(stats) => {
                output.push_str(&format!("  Graph statistics: {:?}\n", stats));
            }
            Err(e) => output.push_str(&format!("  Statistics not available: {:?}\n", e)),
        }

        // Test transaction isolation levels (if supported)
        let isolation_transaction = match graph_reconnected.begin_transaction() {
            Ok(tx) => tx,
            Err(e) => {
                return format!("Failed to begin isolation test transaction: {:?}", e);
            }
        };

        // Clean up verification transaction
        match verify_transaction.commit() {
            Ok(_) => output.push_str("Verification transaction committed\n"),
            Err(e) =>
                output.push_str(&format!("Failed to commit verification transaction: {:?}\n", e)),
        }

        // Test concurrent connection handling
        output.push_str("\nTesting concurrent operations:\n");
        let concurrent_props = vec![
            ("name".to_string(), PropertyValue::StringValue("ConcurrentTest".to_string())),
            ("thread_id".to_string(), PropertyValue::StringValue("main".to_string()))
        ];

        match isolation_transaction.create_vertex("ConcurrentNode", &concurrent_props) {
            Ok(v) =>
                output.push_str(
                    &format!("Created concurrent test vertex: {}\n", format_element_id(&v.id))
                ),
            Err(e) => output.push_str(&format!("Failed to create concurrent vertex: {:?}\n", e)),
        }

        // Test connection timeout and retry behavior
        output.push_str("Testing connection resilience...\n");

        // Attempt multiple rapid operations to test connection stability
        for i in 0..3 {
            let rapid_props = vec![
                ("name".to_string(), PropertyValue::StringValue(format!("RapidTest{}", i))),
                ("iteration".to_string(), PropertyValue::Int32(i))
            ];

            match isolation_transaction.create_vertex("RapidNode", &rapid_props) {
                Ok(v) =>
                    output.push_str(
                        &format!("Rapid operation {} successful: {}\n", i, format_element_id(&v.id))
                    ),
                Err(e) => output.push_str(&format!("Rapid operation {} failed: {:?}\n", i, e)),
            }
        }

        // Final cleanup and commit
        match isolation_transaction.commit() {
            Ok(_) => output.push_str("All connection management tests committed successfully\n"),
            Err(e) => {
                return format!("Failed to commit connection management tests: {:?}", e);
            }
        }

        // Test graceful disconnection
        match graph_reconnected.close() {
            Ok(_) => output.push_str("Connection closed gracefully\n"),
            Err(e) => output.push_str(&format!("Connection close failed: {:?}\n", e)),
        }

        output.push_str(
            &format!("\nConnection management and durability test completed with {} provider\n", PROVIDER)
        );
        output
    }
}

fn get_connection_config() -> ConnectionConfig {
    // Try to get configuration from environment variables first
    let host = std::env
        ::var(format!("GOLEM_{}_HOST", PROVIDER.to_uppercase()))
        .unwrap_or_else(|_| "localhost".to_string());

    let port = std::env
        ::var(format!("GOLEM_{}_PORT", PROVIDER.to_uppercase()))
        .ok()
        .and_then(|p| p.parse::<u16>().ok());

    let username = std::env
        ::var(format!("GOLEM_{}_USER", PROVIDER.to_uppercase()))
        .ok()
        .or_else(|| {
            match PROVIDER {
                "neo4j" => Some("neo4j".to_string()),
                "arangodb" => Some("root".to_string()),
                "janusgraph" => Some("".to_string()),
                _ => Some("test_user".to_string()),
            }
        });

    let password = std::env
        ::var(format!("GOLEM_{}_PASSWORD", PROVIDER.to_uppercase()))
        .ok()
        .or_else(|| Some("test_password".to_string()));

    let database_name = std::env
        ::var(format!("GOLEM_{}_DATABASE", PROVIDER.to_uppercase()))
        .ok()
        .or_else(|| Some("test_graph".to_string()));

    ConnectionConfig {
        hosts: vec![host],
        port,
        database_name,
        username,
        password,
        timeout_seconds: Some(30),
        max_connections: Some(10),
        provider_config: vec![("provider".to_string(), PROVIDER.to_string())],
    }
}

fn format_element_id(id: &crate::bindings::golem::graph::types::ElementId) -> String {
    match id {
        crate::bindings::golem::graph::types::ElementId::StringValue(s) => s.clone(),
        crate::bindings::golem::graph::types::ElementId::Int64(i) => i.to_string(),
        crate::bindings::golem::graph::types::ElementId::Uuid(u) => u.clone(),
    }
}

bindings::export!(Component with_types_in bindings);

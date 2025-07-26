mod client;
mod connection;
mod conversions;
mod helpers;
mod query;
mod schema;
mod transaction;
mod traversal;

use client::ArangoDbApi;
use golem_graph::config::with_config_key;
use golem_graph::durability::{DurableGraph, ExtendedGuest};
use golem_graph::golem::graph::{
    connection::ConnectionConfig, errors::GraphError, transactions::Guest as TransactionGuest,
};
use std::sync::Arc;

pub struct GraphArangoDbComponent;

pub struct Graph {
    api: Arc<ArangoDbApi>,
}

pub struct Transaction {
    api: Arc<ArangoDbApi>,
    transaction_id: String,
}

pub struct SchemaManager {
    graph: Arc<Graph>,
}

impl ExtendedGuest for GraphArangoDbComponent {
    type Graph = Graph;
    fn connect_internal(config: &ConnectionConfig) -> Result<Graph, GraphError> {
        let host = with_config_key(config, "ARANGO_HOST")
            .or_else(|| with_config_key(config, "ARANGODB_HOST"))
            .or_else(|| config.hosts.first().cloned())
            .ok_or_else(|| GraphError::ConnectionFailed("Missing host".to_string()))?;

        let port = with_config_key(config, "ARANGO_PORT")
            .or_else(|| with_config_key(config, "ARANGODB_PORT"))
            .and_then(|p| p.parse().ok())
            .or(config.port)
            .unwrap_or(8529);

        let username = with_config_key(config, "ARANGO_USER")
            .or_else(|| with_config_key(config, "ARANGODB_USER"))
            .or_else(|| config.username.clone())
            .ok_or_else(|| GraphError::ConnectionFailed("Missing username".to_string()))?;

        let password = with_config_key(config, "ARANGO_PASSWORD")
            .or_else(|| with_config_key(config, "ARANGODB_PASSWORD"))
            .or_else(|| config.password.clone())
            .ok_or_else(|| GraphError::ConnectionFailed("Missing password".to_string()))?;

        let database_name = with_config_key(config, "ARANGO_DATABASE")
            .or_else(|| with_config_key(config, "ARANGODB_DATABASE"))
            .or_else(|| config.database_name.clone())
            .unwrap_or_else(|| "_system".to_string());

        let api = ArangoDbApi::new(&host, port, &username, &password, &database_name);
        Ok(Graph::new(api))
    }
}

impl TransactionGuest for GraphArangoDbComponent {
    type Transaction = Transaction;
}

impl Graph {
    fn new(api: ArangoDbApi) -> Self {
        Self { api: Arc::new(api) }
    }
}

impl Transaction {
    fn new(api: Arc<ArangoDbApi>, transaction_id: String) -> Self {
        Self {
            api,
            transaction_id,
        }
    }
}

type DurableGraphArangoDbComponent = DurableGraph<GraphArangoDbComponent>;

golem_graph::export_graph!(DurableGraphArangoDbComponent with_types_in golem_graph);

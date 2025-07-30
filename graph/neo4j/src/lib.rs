mod client;
mod connection;
mod conversions;
mod helpers;
mod query;
mod schema;
mod transaction;
mod traversal;

use client::Neo4jApi;
use golem_graph::config::with_config_key;
use golem_graph::durability::{DurableGraph, ExtendedGuest};
use golem_graph::golem::graph::{
    connection::ConnectionConfig, errors::GraphError, transactions::Guest as TransactionGuest,
};
use std::sync::Arc;

pub struct GraphNeo4jComponent;

pub struct Graph {
    api: Arc<Neo4jApi>,
}

pub struct Transaction {
    api: Arc<Neo4jApi>,
    transaction_url: String,
    state: std::sync::RwLock<TransactionState>,
}

#[derive(Debug, Clone, PartialEq)]
enum TransactionState {
    Active,
    Committed,
    RolledBack,
}

pub struct SchemaManager {
    graph: Arc<Graph>,
}

impl ExtendedGuest for GraphNeo4jComponent {
    type Graph = Graph;
    fn connect_internal(config: &ConnectionConfig) -> Result<Graph, GraphError> {
        let host = with_config_key(config, "NEO4J_HOST")
            .or_else(|| config.hosts.first().cloned())
            .ok_or_else(|| GraphError::ConnectionFailed("Missing host".to_string()))?;

        let port = with_config_key(config, "NEO4J_PORT")
            .and_then(|p| p.parse().ok())
            .or(config.port)
            .unwrap_or(7687);

        let username = with_config_key(config, "NEO4J_USER")
            .or_else(|| config.username.clone())
            .ok_or_else(|| GraphError::ConnectionFailed("Missing username".to_string()))?;

        let password = with_config_key(config, "NEO4J_PASSWORD")
            .or_else(|| config.password.clone())
            .ok_or_else(|| GraphError::ConnectionFailed("Missing password".to_string()))?;

        let api = Neo4jApi::new(&host, port, "neo4j", &username, &password);
        Ok(Graph::new(api))
    }
}

impl TransactionGuest for GraphNeo4jComponent {
    type Transaction = Transaction;
}

impl Graph {
    fn new(api: Neo4jApi) -> Self {
        Self { api: Arc::new(api) }
    }

    pub(crate) fn begin_transaction(&self) -> Result<Transaction, GraphError> {
        let tx_url = self.api.begin_transaction()?;
        Ok(Transaction::new(self.api.clone(), tx_url))
    }
}

impl Transaction {
    fn new(api: Arc<Neo4jApi>, transaction_url: String) -> Self {
        Self {
            api,
            transaction_url,
            state: std::sync::RwLock::new(TransactionState::Active),
        }
    }
}

type DurableGraphNeo4jComponent = DurableGraph<GraphNeo4jComponent>;

golem_graph::export_graph!(DurableGraphNeo4jComponent with_types_in golem_graph);

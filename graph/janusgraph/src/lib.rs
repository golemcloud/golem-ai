mod client;
mod connection;
mod conversions;
mod helpers;
mod query;
mod query_utils;
mod schema;
mod transaction;
mod traversal;

use client::JanusGraphApi;
use golem_graph::config::with_config_key;
use golem_graph::durability::{DurableGraph, ExtendedGuest};
use golem_graph::golem::graph::{
    connection::ConnectionConfig, errors::GraphError, transactions::Guest as TransactionGuest,
};
use std::sync::Arc;

pub struct GraphJanusGraphComponent;

pub struct Graph {
    pub api: Arc<JanusGraphApi>,
}

pub struct Transaction {
    api: Arc<JanusGraphApi>,
    state: std::sync::RwLock<TransactionState>,
}

#[derive(Debug, Clone, PartialEq)]
enum TransactionState {
    Active,
    Committed,
    RolledBack,
}

pub struct SchemaManager {
    pub graph: Arc<Graph>,
}

impl ExtendedGuest for GraphJanusGraphComponent {
    type Graph = Graph;
    fn connect_internal(config: &ConnectionConfig) -> Result<Graph, GraphError> {
        let host = with_config_key(config, "JANUSGRAPH_HOST")
            .or_else(|| config.hosts.first().cloned())
            .ok_or_else(|| GraphError::ConnectionFailed("Missing host".to_string()))?;

        let port = with_config_key(config, "JANUSGRAPH_PORT")
            .and_then(|p| p.parse().ok())
            .or(config.port)
            .unwrap_or(8182); // Default Gremlin Server port

        let username =
            with_config_key(config, "JANUSGRAPH_USER").or_else(|| config.username.clone());

        let password =
            with_config_key(config, "JANUSGRAPH_PASSWORD").or_else(|| config.password.clone());

        let api = JanusGraphApi::new(&host, port, username.as_deref(), password.as_deref())?;
        api.execute("g.tx().open()", None)?;
        Ok(Graph::new(api))
    }
}

impl TransactionGuest for GraphJanusGraphComponent {
    type Transaction = Transaction;
}

impl Graph {
    fn new(api: JanusGraphApi) -> Self {
        Self { api: Arc::new(api) }
    }
}

impl Transaction {
    fn new(api: Arc<JanusGraphApi>) -> Self {
        Self {
            api,
            state: std::sync::RwLock::new(TransactionState::Active),
        }
    }
}

type DurableGraphJanusGraphComponent = DurableGraph<GraphJanusGraphComponent>;

golem_graph::export_graph!(DurableGraphJanusGraphComponent with_types_in golem_graph);

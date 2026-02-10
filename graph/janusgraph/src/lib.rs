mod client;
mod connection;
mod conversions;
mod helpers;
mod query_utils;
mod schema;
mod transaction;

use client::JanusGraphApi;
use golem_ai_graph::config::with_config_key;
use golem_ai_graph::durability::{DurableGraph, ExtendedGuest};
use golem_ai_graph::model::{connection::ConnectionConfig, errors::GraphError};
use golem_ai_graph::TransactionProvider;
use std::sync::Arc;

pub struct JanusGraph;

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

impl ExtendedGuest for JanusGraph {
    type Graph = Graph;
    fn connect_internal(config: &ConnectionConfig) -> Result<Graph, GraphError> {
        let host = with_config_key(config, "JANUSGRAPH_HOST")
            .or_else(|| {
                config
                    .hosts
                    .as_ref()
                    .and_then(|hosts| hosts.first())
                    .cloned()
            })
            .ok_or_else(|| GraphError::ConnectionFailed("Missing host".to_string()))?;

        let port = with_config_key(config, "JANUSGRAPH_PORT")
            .and_then(|p| p.parse().ok())
            .or(config.port)
            .unwrap_or(8182);
        let username =
            with_config_key(config, "JANUSGRAPH_USER").or_else(|| config.username.clone());

        let password =
            with_config_key(config, "JANUSGRAPH_PASSWORD").or_else(|| config.password.clone());

        let api = JanusGraphApi::new(&host, port, username.as_deref(), password.as_deref())?;
        api.execute("g.tx().open()", None)?;
        Ok(Graph::new(api))
    }
}

impl TransactionProvider for JanusGraph {
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

pub type DurableJanusGraph = DurableGraph<JanusGraph>;

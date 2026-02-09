use crate::client::{Neo4jStatement, Neo4jStatements};
use crate::{Graph, Transaction};
use golem_graph::{
    durability::ProviderGraph,
    model::{
        connection::GraphStatistics, errors::GraphError,
        transactions::Transaction as TransactionResource,
    },
    GraphInterface,
};
use std::collections::HashMap;

impl ProviderGraph for Graph {
    type Transaction = Transaction;
}

impl GraphInterface for Graph {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn begin_transaction(&self) -> Result<TransactionResource, GraphError> {
        let transaction_url = self.api.begin_transaction()?;
        let transaction = Transaction::new(self.api.clone(), transaction_url);
        Ok(TransactionResource::new(transaction))
    }

    fn begin_read_transaction(&self) -> Result<TransactionResource, GraphError> {
        let transaction_url = self.api.begin_transaction()?;
        let transaction = Transaction::new(self.api.clone(), transaction_url);
        Ok(TransactionResource::new(transaction))
    }

    fn ping(&self) -> Result<(), GraphError> {
        let transaction_url = self.api.begin_transaction()?;
        self.api.rollback_transaction(&transaction_url)
    }

    fn close(&self) -> Result<(), GraphError> {
        Ok(())
    }

    fn get_statistics(&self) -> Result<GraphStatistics, GraphError> {
        let transaction_url = self.api.begin_transaction()?;

        let node_count_stmt = Neo4jStatement::with_row_only(
            "MATCH (n) RETURN count(n) as nodeCount".to_string(),
            HashMap::new(),
        );
        let node_count_resp = self.api.execute_typed_transaction(
            &transaction_url,
            &Neo4jStatements::single(node_count_stmt),
        )?;

        let node_count = node_count_resp
            .first_result()?
            .first_row()?
            .first()
            .and_then(|v| v.as_u64());

        let rel_count_stmt = Neo4jStatement::with_row_only(
            "MATCH ()-[r]->() RETURN count(r) as relCount".to_string(),
            HashMap::new(),
        );
        let rel_count_resp = self.api.execute_typed_transaction(
            &transaction_url,
            &Neo4jStatements::single(rel_count_stmt),
        )?;

        let rel_count = rel_count_resp
            .first_result()?
            .first_row()?
            .first()
            .and_then(|v| v.as_u64());

        self.api.rollback_transaction(&transaction_url)?;

        Ok(GraphStatistics {
            vertex_count: node_count,
            edge_count: rel_count,
            label_count: None,
            property_count: None,
        })
    }
}

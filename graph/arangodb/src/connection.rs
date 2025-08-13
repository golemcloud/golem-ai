use crate::{Graph, Transaction};
use golem_graph::{
    durability::ProviderGraph,
    golem::graph::{
        connection::{GraphStatistics, GuestGraph},
        errors::GraphError,
        transactions::Transaction as TransactionResource,
    },
};

impl ProviderGraph for Graph {
    type Transaction = Transaction;
}

impl GuestGraph for Graph {
    fn begin_transaction(&self) -> Result<TransactionResource, GraphError> {
        let transaction_id = self.api.begin_dynamic_transaction(false)?;
        let transaction = Transaction::new(self.api.clone(), transaction_id);
        Ok(TransactionResource::new(transaction))
    }

    fn begin_read_transaction(&self) -> Result<TransactionResource, GraphError> {
        let transaction_id = self.api.begin_dynamic_transaction(true)?;
        let transaction = Transaction::new(self.api.clone(), transaction_id);
        Ok(TransactionResource::new(transaction))
    }

    fn ping(&self) -> Result<(), GraphError> {
        self.api.ping()
    }

    fn close(&self) -> Result<(), GraphError> {
        Ok(())
    }

    fn get_statistics(&self) -> Result<GraphStatistics, GraphError> {
        let stats = self.api.get_database_statistics()?;

        Ok(GraphStatistics {
            vertex_count: Some(stats.vertex_count),
            edge_count: Some(stats.edge_count),
            label_count: None, // ArangoDB doesn't have a direct concept of "labels" count
            property_count: None,
        })
    }
}

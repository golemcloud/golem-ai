use crate::{Graph, Transaction};
use async_trait::async_trait;
use golem_ai_graph::{
    durability::ProviderGraph,
    model::{
        connection::GraphStatistics, errors::GraphError,
        transactions::Transaction as TransactionResource,
    },
    GraphInterface,
};

impl ProviderGraph for Graph {
    type Transaction = Transaction;
}

#[async_trait(?Send)]
impl GraphInterface for Graph {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    async fn begin_transaction(&self) -> Result<TransactionResource, GraphError> {
        let transaction_id = self.api.begin_dynamic_transaction(false).await?;
        let transaction = Transaction::new(self.api.clone(), transaction_id);
        Ok(TransactionResource::new(transaction))
    }

    async fn begin_read_transaction(&self) -> Result<TransactionResource, GraphError> {
        let transaction_id = self.api.begin_dynamic_transaction(true).await?;
        let transaction = Transaction::new(self.api.clone(), transaction_id);
        Ok(TransactionResource::new(transaction))
    }

    async fn ping(&self) -> Result<(), GraphError> {
        self.api.ping().await
    }

    async fn close(&self) -> Result<(), GraphError> {
        Ok(())
    }

    async fn get_statistics(&self) -> Result<GraphStatistics, GraphError> {
        let stats = self.api.get_database_statistics().await?;

        Ok(GraphStatistics {
            vertex_count: Some(stats.vertex_count),
            edge_count: Some(stats.edge_count),
            label_count: None, // ArangoDB doesn't have a direct concept of "labels" count
            property_count: None,
        })
    }
}

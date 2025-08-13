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
        self.api.execute("g.tx().open()", None)?;
        let transaction = Transaction::new(self.api.clone());
        Ok(TransactionResource::new(transaction))
    }

    fn begin_read_transaction(&self) -> Result<TransactionResource, GraphError> {
        self.begin_transaction()
    }

    fn ping(&self) -> Result<(), GraphError> {
        self.api.execute("1+1", None)?;
        Ok(())
    }

    fn close(&self) -> Result<(), GraphError> {
        Ok(())
    }

    fn get_statistics(&self) -> Result<GraphStatistics, GraphError> {
        let vertex_count_res = self.api.execute("g.V().count()", None)?;
        let edge_count_res = self.api.execute("g.E().count()", None)?;

        fn extract_count(val: &serde_json::Value) -> Option<u64> {
            // The client now returns the data directly
            if let Some(list) = val.get("@value").and_then(|v| v.as_array()) {
                list.first()
            } else if let Some(arr) = val.as_array() {
                arr.first()
            } else {
                Some(val)
            }
            .and_then(|v| {
                if let Some(n) = v.as_u64() {
                    Some(n)
                } else if let Some(obj) = v.as_object() {
                    if let Some(val) = obj.get("@value") {
                        val.as_u64()
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
        }

        let vertex_count = extract_count(&vertex_count_res);
        let edge_count = extract_count(&edge_count_res);

        Ok(GraphStatistics {
            vertex_count,
            edge_count,
            label_count: None,
            property_count: None,
        })
    }
}

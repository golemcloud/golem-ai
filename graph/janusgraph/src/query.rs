use crate::conversions;
use crate::{GraphJanusGraphComponent, Transaction};
use golem_ai_graph::model::types::PropertyValue;
use golem_ai_graph::model::{
    errors::GraphError,
    query::{Guest as QueryGuest, QueryExecutionResult, QueryParameters, QueryResult},
};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::HashMap;



impl Transaction {

}

impl QueryGuest for GraphJanusGraphComponent {
    fn execute_query(
        transaction: golem_ai_graph::model::transactions::TransactionBorrow<'_>,
        query: String,
        parameters: Option<QueryParameters>,
        options: Option<golem_ai_graph::model::query::QueryOptions>,
    ) -> Result<QueryExecutionResult, GraphError> {
        let tx: &Transaction = transaction.get();
        tx.execute_query(query, parameters, options)
    }
}

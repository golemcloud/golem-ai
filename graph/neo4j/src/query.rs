use crate::client::{Neo4jStatement, Neo4jStatements};
use crate::conversions;
use crate::{GraphNeo4jComponent, Transaction};
use golem_graph::golem::graph::{
    errors::GraphError,
    query::{Guest as QueryGuest, QueryExecutionResult, QueryOptions, QueryParameters},
};
use std::collections::HashMap;

impl Transaction {
    pub fn execute_query(
        &self,
        query: String,
        parameters: Option<QueryParameters>,
        _options: Option<QueryOptions>,
    ) -> Result<QueryExecutionResult, GraphError> {
        let mut params = HashMap::new();
        if let Some(p) = parameters {
            for (key, value) in p {
                params.insert(key, conversions::to_json_value(value)?);
            }
        }

        let statement = Neo4jStatement::new(query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        let result = response.first_result()?;
        result.check_errors()?;

        let columns: Vec<String> = result.columns.clone().unwrap_or_default();
        let mut rows = Vec::new();

        for data_item in &result.data {
            if let Some(row_data) = &data_item.row {
                rows.push(row_data.clone());
            }
        }

        let query_result_value = if columns.len() == 1 {
            let mut values = Vec::new();
            for row in rows {
                if let Some(val) = row.first() {
                    values.push(conversions::from_json_value(val.clone())?);
                }
            }
            golem_graph::golem::graph::query::QueryResult::Values(values)
        } else {
            let mut maps = Vec::new();
            for row in rows {
                let mut map_row = Vec::new();
                for (i, col_name) in columns.iter().enumerate() {
                    if let Some(val) = row.get(i) {
                        map_row
                            .push((col_name.clone(), conversions::from_json_value(val.clone())?));
                    }
                }
                maps.push(map_row);
            }
            golem_graph::golem::graph::query::QueryResult::Maps(maps)
        };

        Ok(QueryExecutionResult {
            query_result_value,
            execution_time_ms: None,
            rows_affected: None,
            explanation: None,
            profile_data: None,
        })
    }

    pub(crate) fn execute_schema_query_and_extract_string_list(
        &self,
        query: &str,
    ) -> Result<Vec<String>, GraphError> {
        let statement = Neo4jStatement::with_row_only(query.to_string(), HashMap::new());
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        let result = response.first_result()?;
        result.check_errors()?;

        let mut items = Vec::new();
        for data_item in &result.data {
            if let Some(row) = &data_item.row {
                if let Some(value) = row.first().and_then(|v| v.as_str()) {
                    items.push(value.to_string());
                }
            }
        }
        Ok(items)
    }
}

impl QueryGuest for GraphNeo4jComponent {
    fn execute_query(
        transaction: golem_graph::golem::graph::transactions::TransactionBorrow<'_>,
        query: String,
        parameters: Option<QueryParameters>,
        options: Option<QueryOptions>,
    ) -> Result<QueryExecutionResult, GraphError> {
        let tx: &Transaction = transaction.get();
        tx.execute_query(query, parameters, options)
    }
}

use crate::conversions::{
    distance_metric_to_pgvector_operator, string_to_distance_metric, string_value_to_db_value,
};
use golem_rust::bindings::golem::rdbms::postgres::{DbConnection, DbResult, DbValue};
use golem_vector::config::get_max_retries_config;
use golem_vector::golem::vector::types::VectorError;
use log::trace;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;

/// PostgreSQL Vector (pgvector) client using Golem RDBMS interface
#[derive(Clone)]
pub struct PgVectorClient {
    connection_string: String,
}

impl PgVectorClient {
    pub fn new(connection_string: String) -> Self {
        Self { connection_string }
    }

    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }

    fn get_connection(&self) -> Result<DbConnection, VectorError> {
        match DbConnection::open(&self.connection_string) {
            Ok(conn) => Ok(conn),
            Err(e) => Err(VectorError::ConnectionError(format!(
                "Failed to connect to PostgreSQL: {:?}",
                e
            ))),
        }
    }

    fn execute_sql(&self, sql: &str, params: Vec<DbValue>) -> Result<u64, VectorError> {
        let conn = self.get_connection()?;
        trace!("Executing SQL: {} with {} params", sql, params.len());

        match conn.execute(sql, params) {
            Ok(rows_affected) => Ok(rows_affected),
            Err(e) => Err(VectorError::ProviderError(format!(
                "SQL execution failed: {:?}",
                e
            ))),
        }
    }

    fn query_sql(&self, sql: &str, params: Vec<DbValue>) -> Result<DbResult, VectorError> {
        let conn = self.get_connection()?;
        trace!("Querying SQL: {} with {} params", sql, params.len());

        match conn.query(sql, params) {
            Ok(result) => Ok(result),
            Err(e) => Err(VectorError::ProviderError(format!(
                "SQL query failed: {:?}",
                e
            ))),
        }
    }

    fn db_value_to_string(&self, value: &DbValue) -> Option<String> {
        match value {
            DbValue::Text(val) => Some(val.clone()),
            DbValue::Varchar(val) => Some(val.clone()),
            DbValue::Bpchar(val) => Some(val.clone()),
            DbValue::Int4(val) => Some(val.to_string()),
            DbValue::Int8(val) => Some(val.to_string()),
            DbValue::Float4(val) => Some(val.to_string()),
            DbValue::Float8(val) => Some(val.to_string()),
            DbValue::Boolean(val) => Some(val.to_string()),
            DbValue::Date(val) => Some(format!("{:?}", val)),
            DbValue::Time(val) => Some(format!("{:?}", val)),
            DbValue::Timestamp(val) => Some(format!("{:?}", val)),
            DbValue::Timestamptz(val) => Some(format!("{:?}", val)),
            DbValue::Uuid(val) => Some(format!("{:?}", val)),
            DbValue::Json(val) => Some(val.clone()),
            DbValue::Jsonb(val) => Some(val.clone()),
            DbValue::Null => None,
            _ => Some(format!("{:?}", value)),
        }
    }

    fn db_value_to_f32(&self, value: &DbValue) -> Option<f32> {
        match value {
            DbValue::Float4(val) => Some(*val),
            DbValue::Float8(val) => Some(*val as f32),
            DbValue::Int4(val) => Some(*val as f32),
            DbValue::Text(val) | DbValue::Varchar(val) => val.parse().ok(),
            _ => None,
        }
    }

    fn parse_pgvector_string(&self, s: &str) -> Result<Vec<f32>, VectorError> {
        let trimmed = s.trim_start_matches('[').trim_end_matches(']');
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        trimmed
            .split(',')
            .map(|part| {
                part.trim().parse::<f32>().map_err(|e| {
                    VectorError::ProviderError(format!(
                        "Failed to parse vector component '{}': {}",
                        part, e
                    ))
                })
            })
            .collect()
    }

    fn get_table_column_types(
        &self,
        table_name: &str,
    ) -> Result<HashMap<String, String>, VectorError> {
        let sql = r#"
            SELECT column_name::text, data_type::text 
            FROM information_schema.columns 
            WHERE table_name = $1 AND table_schema = 'public'
        "#;

        let params = vec![DbValue::Text(table_name.to_string())];
        let result = self.query_sql(sql, params)?;

        let mut column_types = HashMap::new();
        for row in &result.rows {
            if row.values.len() >= 2 {
                let column_name = match &row.values[0] {
                    DbValue::Text(name) => name.clone(),
                    _ => continue,
                };
                let data_type = match &row.values[1] {
                    DbValue::Text(dtype) => dtype.clone(),
                    _ => continue,
                };
                column_types.insert(column_name, data_type);
            }
        }

        Ok(column_types)
    }

    fn execute_with_retry<F, T>(&self, operation: F) -> Result<T, VectorError>
    where
        F: Fn() -> Result<T, VectorError>,
    {
        let max_retries = get_max_retries_config();
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match operation() {
                Ok(result) => {
                    trace!("SQL operation succeeded on attempt {}", attempt + 1);
                    return Ok(result);
                }
                Err(e) => {
                    trace!("SQL operation failed on attempt {}: {}", attempt + 1, e);
                    last_error = Some(e);

                    if attempt < max_retries {
                        std::thread::sleep(std::time::Duration::from_millis(
                            100 * (2_u64.pow(attempt)),
                        ));
                        continue;
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| VectorError::ProviderError("Max retries exceeded".to_string())))
    }

    pub fn enable_extension(&self) -> Result<(), VectorError> {
        self.execute_with_retry(|| {
            self.execute_sql("CREATE EXTENSION IF NOT EXISTS vector", vec![])?;
            Ok(())
        })
    }

    pub fn create_table(
        &self,
        request: &CreateTableRequest,
    ) -> Result<CreateTableResponse, VectorError> {
        self.execute_with_retry(|| {
            let mut sql = format!("CREATE TABLE IF NOT EXISTS \"{}\" (", request.table_name);
            sql.push_str("id VARCHAR PRIMARY KEY, ");

            if let Some(dimension) = request.dimension {
                sql.push_str(&format!("embedding vector({}), ", dimension));
            } else {
                sql.push_str("embedding vector, ");
            }

            for (column_name, column_type) in &request.metadata_columns {
                sql.push_str(&format!("\"{}\" {}, ", column_name, column_type));
            }

            sql = sql.trim_end_matches(", ").to_string();
            sql.push(')');

            self.execute_sql(&sql, vec![])?;

            Ok(CreateTableResponse {
                table_name: request.table_name.clone(),
            })
        })
    }

    pub fn drop_table(&self, table_name: &str) -> Result<DropTableResponse, VectorError> {
        self.execute_with_retry(|| {
            let sql = format!("DROP TABLE IF EXISTS \"{}\"", table_name);
            self.execute_sql(&sql, vec![])?;

            Ok(DropTableResponse {
                table_name: table_name.to_string(),
            })
        })
    }

    pub fn table_exists(&self, table_name: &str) -> Result<TableExistsResponse, VectorError> {
        self.execute_with_retry(|| {
            let sql = "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = $1)";
            let params = vec![DbValue::Text(table_name.to_string())];
            let result = self.query_sql(sql, params)?;

            let exists = if let Some(row) = result.rows.first() {
                if let Some(DbValue::Boolean(val)) = row.values.first() {
                    *val
                } else {
                    false
                }
            } else {
                false
            };

            Ok(TableExistsResponse { exists })
        })
    }

    pub fn upsert_vectors(
        &self,
        request: &UpsertVectorsRequest,
    ) -> Result<UpsertVectorsResponse, VectorError> {
        self.execute_with_retry(|| {
            let mut column_types = self.get_table_column_types(&request.table_name)?;
            let mut upserted_count = 0;

            for vector in &request.vectors {
                let embedding_str = format!(
                    "[{}]",
                    vector
                        .embedding
                        .iter()
                        .map(|f| f.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                );

                let mut columns = vec!["id".to_string(), "embedding".to_string()];
                let mut placeholders = vec!["$1".to_string(), "$2::vector".to_string()];
                let mut params = vec![
                    DbValue::Text(vector.id.clone()),
                    DbValue::Text(embedding_str),
                ];
                let mut update_clauses = vec!["embedding = EXCLUDED.embedding".to_string()];

                let mut missing_columns: Vec<(String, String)> = Vec::new();
                for (key, value) in &vector.metadata {
                    if !column_types.contains_key(key) {
                        let inferred_type = if value.parse::<i64>().is_ok() {
                            "BIGINT"
                        } else if value.parse::<f64>().is_ok() {
                            "DOUBLE PRECISION"
                        } else if value.eq_ignore_ascii_case("true")
                            || value.eq_ignore_ascii_case("false")
                        {
                            "BOOLEAN"
                        } else {
                            "TEXT"
                        };

                        missing_columns.push((key.clone(), inferred_type.to_string()));
                    }
                }

                if !missing_columns.is_empty() {
                    let mut alters = Vec::new();
                    for (col, ty) in &missing_columns {
                        alters.push(format!("ADD COLUMN \"{}\" {}", col, ty));
                    }

                    let alter_sql = format!(
                        "ALTER TABLE \"{}\" {}",
                        request.table_name,
                        alters.join(", ")
                    );
                    self.execute_sql(&alter_sql, vec![])?;

                    let refreshed = self.get_table_column_types(&request.table_name)?;
                    for (k, v) in refreshed {
                        column_types.insert(k, v);
                    }
                }

                let mut param_index = 3;
                for (key, value) in &vector.metadata {
                    if let Some(column_type) = column_types.get(key) {
                        columns.push(format!("\"{}\"", key));
                        placeholders.push(format!("${}", param_index));

                        let db_value = string_value_to_db_value(value, column_type)?;

                        params.push(db_value);
                        update_clauses.push(format!("\"{}\" = EXCLUDED.\"{}\"", key, key));
                        param_index += 1;
                    }
                }

                let sql = format!(
                    "INSERT INTO \"{}\" ({}) VALUES ({}) ON CONFLICT (id) DO UPDATE SET {}",
                    request.table_name,
                    columns.join(", "),
                    placeholders.join(", "),
                    update_clauses.join(", ")
                );

                self.execute_sql(&sql, params)?;
                upserted_count += 1;
            }

            Ok(UpsertVectorsResponse {
                upserted_count: upserted_count as u32,
            })
        })
    }

    pub fn search_vectors(&self, request: &SearchRequest) -> Result<SearchResponse, VectorError> {
        self.execute_with_retry(|| {
            let operator = distance_metric_to_pgvector_operator(&string_to_distance_metric(
                &request.distance_metric,
            ));

            let select_columns: Vec<String> = request
                .select_columns
                .iter()
                .map(|col| {
                    if col == "embedding" {
                        "embedding::text as embedding".to_string()
                    } else {
                        col.clone()
                    }
                })
                .collect();

            let mut sql = format!(
                "SELECT {}, embedding {} $1::vector as distance FROM \"{}\"",
                select_columns.join(", "),
                operator,
                request.table_name
            );

            let query_vector_str = format!(
                "[{}]",
                request
                    .query_vector
                    .iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );

            let mut params = vec![DbValue::Text(query_vector_str)];

            if !request.filters.is_empty() {
                let column_types = self.get_table_column_types(&request.table_name)?;

                if let Some(where_clause) = request.filters.get("where_clause") {
                    let column_exists = column_types
                        .keys()
                        .any(|col| where_clause.contains(&format!("\"{}\"", col)));

                    if column_exists {
                        sql.push_str(&format!(" WHERE {}", where_clause));
                    }
                } else {
                    let mut filter_conditions = Vec::new();
                    for (key, value) in &request.filters {
                        if column_types.contains_key(key) {
                            params.push(DbValue::Text(value.clone()));
                            filter_conditions.push(format!("\"{}\" = ${}", key, params.len()));
                        }
                    }

                    if !filter_conditions.is_empty() {
                        sql.push_str(&format!(" WHERE {}", filter_conditions.join(" AND ")));
                    }
                }
            }

            sql.push_str(&format!(
                " ORDER BY embedding {} $1::vector LIMIT {}",
                operator, request.limit
            ));

            let result = self.query_sql(&sql, params)?;

            let mut results = Vec::new();
            for row in &result.rows {
                let mut id = String::new();
                let mut distance = 0.0f32;
                let mut embedding = Vec::new();
                let mut metadata = HashMap::new();

                for (i, column) in result.columns.iter().enumerate() {
                    if let Some(value) = row.values.get(i) {
                        match column.name.as_str() {
                            "id" => {
                                if let Some(s) = self.db_value_to_string(value) {
                                    id = s;
                                }
                            }
                            "distance" => {
                                if let Some(f) = self.db_value_to_f32(value) {
                                    distance = f;
                                }
                            }
                            "embedding" => {
                                if let Some(s) = self.db_value_to_string(value) {
                                    if let Ok(parsed) = self.parse_pgvector_string(&s) {
                                        embedding = parsed;
                                    }
                                }
                            }
                            _ => {
                                if let Some(s) = self.db_value_to_string(value) {
                                    metadata.insert(column.name.clone(), s);
                                }
                            }
                        }
                    }
                }

                results.push(SearchResult {
                    id,
                    embedding,
                    distance,
                    metadata,
                });
            }

            Ok(SearchResponse { results })
        })
    }

    pub fn get_vectors(
        &self,
        request: &GetVectorsRequest,
    ) -> Result<GetVectorsResponse, VectorError> {
        self.execute_with_retry(|| {
            let columns = if request.select_columns.is_empty() {
                vec!["id".to_string(), "embedding::text as embedding".to_string()]
            } else {
                let mut cols = Vec::new();
                for col in &request.select_columns {
                    if col == "embedding" {
                        cols.push("embedding::text as embedding".to_string());
                    } else {
                        cols.push(col.clone());
                    }
                }
                cols
            };

            let placeholders: Vec<String> =
                (1..=request.ids.len()).map(|i| format!("${}", i)).collect();

            let sql = format!(
                "SELECT {} FROM \"{}\" WHERE id IN ({})",
                columns.join(", "),
                request.table_name,
                placeholders.join(", ")
            );

            let params: Vec<DbValue> = request
                .ids
                .iter()
                .map(|id| DbValue::Text(id.clone()))
                .collect();
            let result = self.query_sql(&sql, params)?;

            let mut results = Vec::new();
            for row in &result.rows {
                let mut id = String::new();
                let mut embedding = Vec::new();
                let mut metadata = HashMap::new();

                for (i, column) in result.columns.iter().enumerate() {
                    if let Some(value) = row.values.get(i) {
                        match column.name.as_str() {
                            "id" => {
                                if let Some(s) = self.db_value_to_string(value) {
                                    id = s;
                                }
                            }
                            "embedding" => {
                                if let Some(s) = self.db_value_to_string(value) {
                                    if let Ok(parsed) = self.parse_pgvector_string(&s) {
                                        embedding = parsed;
                                    }
                                }
                            }
                            _ => {
                                if let Some(s) = self.db_value_to_string(value) {
                                    metadata.insert(column.name.clone(), s);
                                }
                            }
                        }
                    }
                }

                results.push(VectorResult {
                    id,
                    embedding,
                    metadata,
                });
            }

            Ok(GetVectorsResponse { results })
        })
    }

    pub fn delete_vectors(
        &self,
        request: &DeleteVectorsRequest,
    ) -> Result<DeleteVectorsResponse, VectorError> {
        self.execute_with_retry(|| {
            let placeholders: Vec<String> =
                (1..=request.ids.len()).map(|i| format!("${}", i)).collect();

            let sql = format!(
                "DELETE FROM \"{}\" WHERE id IN ({})",
                request.table_name,
                placeholders.join(", ")
            );

            let params: Vec<DbValue> = request
                .ids
                .iter()
                .map(|id| DbValue::Text(id.clone()))
                .collect();
            let rows_affected = self.execute_sql(&sql, params)?;

            Ok(DeleteVectorsResponse {
                deleted_count: rows_affected as u32,
            })
        })
    }

    pub fn count_vectors(&self, table_name: &str) -> Result<CountVectorsResponse, VectorError> {
        self.execute_with_retry(|| {
            let sql = format!("SELECT COUNT(*) as count FROM \"{}\"", table_name);
            let result = self.query_sql(&sql, vec![])?;

            let count = if let Some(row) = result.rows.first() {
                if let Some(value) = row.values.first() {
                    match value {
                        DbValue::Int8(val) => *val as u64,
                        DbValue::Int4(val) => *val as u64,
                        _ => 0,
                    }
                } else {
                    0
                }
            } else {
                0
            };

            Ok(CountVectorsResponse { count })
        })
    }

    pub fn delete_by_filter(
        &self,
        request: &DeleteByFilterRequest,
    ) -> Result<DeleteByFilterResponse, VectorError> {
        self.execute_with_retry(|| {
            let column_types = self.get_table_column_types(&request.table_name)?;

            let mut sql = format!("DELETE FROM \"{}\"", request.table_name);
            let mut params = Vec::new();

            if !request.filters.is_empty() {
                if let Some(where_clause) = request.filters.get("where_clause") {
                    let column_exists = column_types
                        .keys()
                        .any(|col| where_clause.contains(&format!("\"{}\"", col)));

                    if column_exists {
                        sql.push_str(&format!(" WHERE {}", where_clause));
                    } else {
                        return Ok(DeleteByFilterResponse { deleted_count: 0 });
                    }
                } else {
                    let mut filter_conditions = Vec::new();
                    for (key, value) in &request.filters {
                        if column_types.contains_key(key) {
                            params.push(DbValue::Text(value.clone()));
                            filter_conditions.push(format!("\"{}\" = ${}", key, params.len()));
                        }
                    }

                    if filter_conditions.is_empty() {
                        return Ok(DeleteByFilterResponse { deleted_count: 0 });
                    }

                    sql.push_str(&format!(" WHERE {}", filter_conditions.join(" AND ")));
                }
            } else {
                return Err(VectorError::InvalidParams(
                    "No filter provided for delete operation".to_string(),
                ));
            }

            let rows_affected = self.execute_sql(&sql, params)?;

            Ok(DeleteByFilterResponse {
                deleted_count: rows_affected as u32,
            })
        })
    }

    pub fn list_vectors(
        &self,
        request: &ListVectorsRequest,
    ) -> Result<ListVectorsResponse, VectorError> {
        self.execute_with_retry(|| {
            let column_types = self.get_table_column_types(&request.table_name)?;

            let columns = if request.select_columns.is_empty() {
                vec!["id".to_string(), "embedding::text as embedding".to_string()]
            } else {
                let mut cols = Vec::new();
                for col in &request.select_columns {
                    if col == "embedding" {
                        cols.push("embedding::text as embedding".to_string());
                    } else {
                        cols.push(col.clone());
                    }
                }
                cols
            };

            let mut sql = format!(
                "SELECT {} FROM \"{}\"",
                columns.join(", "),
                request.table_name
            );
            let mut params = Vec::new();

            if !request.filters.is_empty() {
                if let Some(where_clause) = request.filters.get("where_clause") {
                    let column_exists = column_types
                        .keys()
                        .any(|col| where_clause.contains(&format!("\"{}\"", col)));

                    if column_exists {
                        sql.push_str(&format!(" WHERE {}", where_clause));
                    }
                } else {
                    let mut filter_conditions = Vec::new();
                    for (key, value) in &request.filters {
                        if column_types.contains_key(key) {
                            params.push(DbValue::Text(value.clone()));
                            filter_conditions.push(format!("\"{}\" = ${}", key, params.len()));
                        }
                    }

                    if !filter_conditions.is_empty() {
                        sql.push_str(&format!(" WHERE {}", filter_conditions.join(" AND ")));
                    }
                }
            }

            sql.push_str(&format!(" ORDER BY id LIMIT {}", request.limit));
            if let Some(offset) = request.offset {
                sql.push_str(&format!(" OFFSET {}", offset));
            }

            let result = self.query_sql(&sql, params)?;

            let mut vectors = Vec::new();
            for row in &result.rows {
                let mut id = String::new();
                let mut embedding = Vec::new();
                let mut metadata = HashMap::new();

                for (i, column) in result.columns.iter().enumerate() {
                    if let Some(value) = row.values.get(i) {
                        match column.name.as_str() {
                            "id" => {
                                if let Some(s) = self.db_value_to_string(value) {
                                    id = s;
                                }
                            }
                            "embedding" => {
                                if let Some(s) = self.db_value_to_string(value) {
                                    if let Ok(parsed) = self.parse_pgvector_string(&s) {
                                        embedding = parsed;
                                    }
                                }
                            }
                            _ => {
                                if let Some(s) = self.db_value_to_string(value) {
                                    metadata.insert(column.name.clone(), s);
                                }
                            }
                        }
                    }
                }

                vectors.push(VectorResult {
                    id,
                    embedding,
                    metadata,
                });
            }

            let has_more = vectors.len() == request.limit as usize;
            let next_cursor = if has_more {
                vectors.last().map(|v| v.id.clone())
            } else {
                None
            };

            Ok(ListVectorsResponse {
                vectors,
                cursor: next_cursor,
            })
        })
    }

    pub fn search_range(
        &self,
        request: &SearchRangeRequest,
    ) -> Result<SearchRangeResponse, VectorError> {
        self.execute_with_retry(|| {
            let operator = distance_metric_to_pgvector_operator(&string_to_distance_metric(
                &request.distance_metric,
            ));
            let column_types = self.get_table_column_types(&request.table_name)?;

            let select_columns: Vec<String> = request
                .select_columns
                .iter()
                .map(|col| {
                    if col == "embedding" {
                        "embedding::text as embedding".to_string()
                    } else {
                        col.clone()
                    }
                })
                .collect();

            let mut sql = format!(
                "SELECT {}, embedding {} $1::vector as distance FROM \"{}\"",
                select_columns.join(", "),
                operator,
                request.table_name
            );

            let query_vector_str = format!(
                "[{}]",
                request
                    .query_vector
                    .iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );

            let mut params = vec![DbValue::Text(query_vector_str)];
            let mut where_conditions = Vec::new();

            if let Some(min_dist) = request.min_distance {
                where_conditions.push(format!("embedding {} $1::vector >= {}", operator, min_dist));
            }
            where_conditions.push(format!(
                "embedding {} $1::vector <= {}",
                operator, request.max_distance
            ));

            if !request.filters.is_empty() {
                if let Some(where_clause) = request.filters.get("where_clause") {
                    let column_exists = column_types
                        .keys()
                        .any(|col| where_clause.contains(&format!("\"{}\"", col)));

                    if column_exists {
                        where_conditions.push(format!("({})", where_clause));
                    }
                } else {
                    for (key, value) in &request.filters {
                        if column_types.contains_key(key) {
                            params.push(DbValue::Text(value.clone()));
                            where_conditions.push(format!("\"{}\" = ${}", key, params.len()));
                        }
                    }
                }
            }

            if !where_conditions.is_empty() {
                sql.push_str(&format!(" WHERE {}", where_conditions.join(" AND ")));
            }

            sql.push_str(&format!(" ORDER BY embedding {} $1::vector", operator));
            if let Some(limit) = request.limit {
                sql.push_str(&format!(" LIMIT {}", limit));
            }

            let result = self.query_sql(&sql, params)?;

            let mut results = Vec::new();
            for row in &result.rows {
                let mut id = String::new();
                let mut distance = 0.0f32;
                let mut embedding = Vec::new();
                let mut metadata = HashMap::new();

                for (i, column) in result.columns.iter().enumerate() {
                    if let Some(value) = row.values.get(i) {
                        match column.name.as_str() {
                            "id" => {
                                if let Some(s) = self.db_value_to_string(value) {
                                    id = s;
                                }
                            }
                            "distance" => {
                                if let Some(f) = self.db_value_to_f32(value) {
                                    distance = f;
                                }
                            }
                            "embedding" => {
                                if let Some(s) = self.db_value_to_string(value) {
                                    if let Ok(parsed) = self.parse_pgvector_string(&s) {
                                        embedding = parsed;
                                    }
                                }
                            }
                            _ => {
                                if let Some(s) = self.db_value_to_string(value) {
                                    metadata.insert(column.name.clone(), s);
                                }
                            }
                        }
                    }
                }

                results.push(SearchResult {
                    id,
                    embedding,
                    distance,
                    metadata,
                });
            }

            Ok(SearchRangeResponse { results })
        })
    }

    pub fn get_field_stats(
        &self,
        request: &FieldStatsRequest,
    ) -> Result<FieldStatsResponse, VectorError> {
        self.execute_with_retry(|| {
            let column_types = self.get_table_column_types(&request.table_name)?;
            if !column_types.contains_key(&request.field_name) {
                return Err(VectorError::InvalidParams(format!("Field '{}' does not exist", request.field_name)));
            }
            let field_type = column_types.get(&request.field_name).unwrap();
            let sql = match field_type.to_uppercase().as_str() {
                "INTEGER" | "BIGINT" | "INT4" | "INT8" => {
                    format!(
                        "SELECT COUNT(*) as count, MIN(\"{}\"::bigint) as min_val, MAX(\"{}\"::bigint) as max_val, AVG(\"{}\"::bigint) as avg_val FROM \"{}\" WHERE \"{}\" IS NOT NULL",
                        request.field_name, request.field_name, request.field_name, request.table_name, request.field_name
                    )
                },
                "REAL" | "DOUBLE PRECISION" | "FLOAT4" | "FLOAT8" | "NUMERIC" => {
                    format!(
                        "SELECT COUNT(*) as count, MIN(\"{}\"::double precision) as min_val, MAX(\"{}\"::double precision) as max_val, AVG(\"{}\"::double precision) as avg_val FROM \"{}\" WHERE \"{}\" IS NOT NULL",
                        request.field_name, request.field_name, request.field_name, request.table_name, request.field_name
                    )
                },
                _ => {
                    format!(
                        "SELECT COUNT(*) as count, COUNT(DISTINCT \"{}\") as unique_count FROM \"{}\" WHERE \"{}\" IS NOT NULL",
                        request.field_name, request.table_name, request.field_name
                    )
                }
            };
            let result = self.query_sql(&sql, vec![])?;
            if let Some(row) = result.rows.first() {
                let count = match row.values.first() {
                    Some(DbValue::Int8(val)) => *val as u64,
                    Some(DbValue::Int4(val)) => *val as u64,
                    _ => 0,
                };
                let (min_val, max_val, avg_val, unique_count) = if field_type.to_uppercase().contains("INT") || field_type.to_uppercase().contains("FLOAT") || field_type.to_uppercase().contains("REAL") || field_type.to_uppercase().contains("NUMERIC") {
                    let min_val = row.values.get(1).and_then(|v| self.db_value_to_f32(v)).map(|f| f as f64);
                    let max_val = row.values.get(2).and_then(|v| self.db_value_to_f32(v)).map(|f| f as f64);
                    let avg_val = row.values.get(3).and_then(|v| self.db_value_to_f32(v)).map(|f| f as f64);
                    (min_val, max_val, avg_val, None)
                } else {
                    let unique_count = match row.values.get(1) {
                        Some(DbValue::Int8(val)) => Some(*val as u64),
                        Some(DbValue::Int4(val)) => Some(*val as u64),
                        _ => None,
                    };
                    (None, None, None, unique_count)
                };
                Ok(FieldStatsResponse {
                    field_name: request.field_name.clone(),
                    field_type: field_type.clone(),
                    count,
                    unique_count,
                    min_value: min_val,
                    max_value: max_val,
                    avg_value: avg_val,
                })
            } else {
                Err(VectorError::ProviderError("Failed to get field statistics".to_string()))
            }
        })
    }

    pub fn get_field_distribution(
        &self,
        request: &FieldDistributionRequest,
    ) -> Result<FieldDistributionResponse, VectorError> {
        self.execute_with_retry(|| {
            let column_types = self.get_table_column_types(&request.table_name)?;
            if !column_types.contains_key(&request.field_name) {
                return Err(VectorError::InvalidParams(format!("Field '{}' does not exist", request.field_name)));
            }
            let sql = format!(
                "SELECT \"{}\"::text as value, COUNT(*) as count FROM \"{}\" WHERE \"{}\" IS NOT NULL GROUP BY \"{}\" ORDER BY count DESC LIMIT {}",
                request.field_name, request.table_name, request.field_name, request.field_name, request.limit
            );
            let result = self.query_sql(&sql, vec![])?;
            let mut distribution = Vec::new();
            for row in &result.rows {
                if row.values.len() >= 2 {
                    let value = match &row.values[0] {
                        DbValue::Text(val) => val.clone(),
                        _ => continue,
                    };
                    let count = match &row.values[1] {
                        DbValue::Int8(val) => *val as u64,
                        DbValue::Int4(val) => *val as u64,
                        _ => continue,
                    };
                    distribution.push((value, count));
                }
            }
            Ok(FieldDistributionResponse {
                field_name: request.field_name.clone(),
                distribution,
            })
        })
    }

    pub fn list_tables(&self) -> Result<ListTablesResponse, VectorError> {
        self.execute_with_retry(|| {
            let sql = "SELECT table_name::text FROM information_schema.tables WHERE table_schema = 'public'";
            let result = self.query_sql(sql, vec![])?;
            let tables: Vec<String> = result.rows.iter()
                .filter_map(|row| {
                    if let Some(value) = row.values.first() {
                        match value {
                            DbValue::Text(table_name) => Some(table_name.clone()),
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
                .collect();
            Ok(ListTablesResponse { tables })
        })
    }

    pub fn describe_table(&self, table_name: &str) -> Result<DescribeTableResponse, VectorError> {
        self.execute_with_retry(|| {
            let sql = r#"
                SELECT 
                    a.attname::text as column_name, 
                    CASE 
                        WHEN t.typname = 'vector' THEN 
                            CASE 
                                WHEN a.atttypmod > 0 THEN 'vector(' || a.atttypmod || ')'
                                ELSE 'vector'
                            END
                        ELSE pg_catalog.format_type(a.atttypid, a.atttypmod)::text
                    END as data_type,
                    NOT a.attnotnull as is_nullable
                FROM pg_attribute a
                JOIN pg_class c ON a.attrelid = c.oid
                JOIN pg_namespace n ON c.relnamespace = n.oid
                LEFT JOIN pg_type t ON a.atttypid = t.oid
                WHERE c.relname = $1 
                    AND n.nspname = 'public'
                    AND a.attnum > 0 
                    AND NOT a.attisdropped
                ORDER BY a.attnum
            "#;

            let params = vec![DbValue::Text(table_name.to_string())];
            let result = self.query_sql(sql, params)?;

            let columns: Vec<TableColumn> = result
                .rows
                .iter()
                .map(|row| {
                    let name = if !row.values.is_empty() {
                        match &row.values[0] {
                            DbValue::Text(name) => name.clone(),
                            _ => String::new(),
                        }
                    } else {
                        String::new()
                    };

                    let data_type = if row.values.len() > 1 {
                        match &row.values[1] {
                            DbValue::Text(dtype) => dtype.clone(),
                            _ => String::new(),
                        }
                    } else {
                        String::new()
                    };

                    let nullable = if row.values.len() > 2 {
                        match &row.values[2] {
                            DbValue::Boolean(is_null) => *is_null,
                            _ => false,
                        }
                    } else {
                        false
                    };

                    TableColumn {
                        name,
                        data_type,
                        nullable,
                    }
                })
                .collect();

            Ok(DescribeTableResponse {
                table_name: table_name.to_string(),
                columns,
            })
        })
    }
}

//req/res structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTableRequest {
    pub table_name: String,
    pub dimension: Option<u32>,
    pub metadata_columns: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTableResponse {
    pub table_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropTableResponse {
    pub table_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableExistsResponse {
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorData {
    pub id: String,
    pub embedding: Vec<f32>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertVectorsRequest {
    pub table_name: String,
    pub vectors: Vec<VectorData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertVectorsResponse {
    pub upserted_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub table_name: String,
    pub query_vector: Vec<f32>,
    pub distance_metric: String,
    pub limit: i32,
    pub filters: HashMap<String, String>,
    pub select_columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub embedding: Vec<f32>,
    pub distance: f32,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetVectorsRequest {
    pub table_name: String,
    pub ids: Vec<String>,
    pub select_columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorResult {
    pub id: String,
    pub embedding: Vec<f32>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetVectorsResponse {
    pub results: Vec<VectorResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteVectorsRequest {
    pub table_name: String,
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteVectorsResponse {
    pub deleted_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountVectorsResponse {
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTablesResponse {
    pub tables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableColumn {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescribeTableResponse {
    pub table_name: String,
    pub columns: Vec<TableColumn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteByFilterRequest {
    pub table_name: String,
    pub filters: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteByFilterResponse {
    pub deleted_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListVectorsRequest {
    pub table_name: String,
    pub filters: HashMap<String, String>,
    pub limit: u32,
    pub offset: Option<u64>,
    pub select_columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListVectorsResponse {
    pub vectors: Vec<VectorResult>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRangeRequest {
    pub table_name: String,
    pub query_vector: Vec<f32>,
    pub distance_metric: String,
    pub min_distance: Option<f32>,
    pub max_distance: f32,
    pub filters: HashMap<String, String>,
    pub select_columns: Vec<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRangeResponse {
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldStatsRequest {
    pub table_name: String,
    pub field_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldStatsResponse {
    pub field_name: String,
    pub field_type: String,
    pub count: u64,
    pub unique_count: Option<u64>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub avg_value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDistributionRequest {
    pub table_name: String,
    pub field_name: String,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDistributionResponse {
    pub field_name: String,
    pub distribution: Vec<(String, u64)>,
}

// helper functions

impl Default for PgVectorClient {
    fn default() -> Self {
        Self::new("postgres://postgres@localhost:5432/postgres".to_string())
    }
}

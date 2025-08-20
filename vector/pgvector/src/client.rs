//! Synchronous client for interacting with a `pgvector`-enabled PostgreSQL
//! database.
//!
//! The implementation supports two build targets:
//! 1. `wasm32-wasi` (default when compiling to WebAssembly components) – all
//!    methods return `unsupported_feature` because direct TCP connections are
//!    unavailable in the sandboxed runtime.
//! 2. Native targets (e.g. `x86_64-unknown-linux-gnu`) – the client establishes
//!    a blocking connection using the [`postgres`](https://docs.rs/postgres)
//!    crate and executes simple SQL statements.
//!
//! Runtime selection happens via `cfg(target_family = "wasm")`.

use golem_vector::error::unsupported_feature;
<<<<<<< HEAD
<<<<<<< HEAD
use golem_vector::exports::golem::vector::types::{
    DistanceMetric, Metadata, VectorData, VectorError, VectorRecord,
};
use crate::conversion::{metric_to_pgvector, vector_data_to_dense, metadata_to_json_map, json_object_to_metadata};
=======
use golem_vector::exports::golem::vector::types::{DistanceMetric, VectorError, VectorRecord};

use crate::conversion::{filter_expression_to_sql, metric_to_pgvector, vector_data_to_dense};
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
use golem_vector::exports::golem::vector::types::{DistanceMetric, VectorError, VectorRecord};

use crate::conversion::{filter_expression_to_sql, metric_to_pgvector, vector_data_to_dense};
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da

#[cfg(target_family = "wasm")]
pub struct PgvectorClient;

#[cfg(target_family = "wasm")]
impl PgvectorClient {
    pub fn new(_database_url: String) -> Result<Self, VectorError> {
        Ok(PgvectorClient)
    }

    fn err<T>() -> Result<T, VectorError> {
        Err(unsupported_feature(
            "pgvector provider not available in wasm runtime",
        ))
    }

    pub fn create_collection(&self, _name: &str, _dimension: u32) -> Result<(), VectorError> {
        Self::err()
    }

    pub fn list_collections(&self) -> Result<Vec<String>, VectorError> {
        Self::err()
    }

    pub fn delete_collection(&self, _name: &str) -> Result<(), VectorError> {
        Self::err()
    }

    pub fn upsert_vectors(
        &self,
        _table: &str,
        _records: Vec<VectorRecord>,
        _namespace: Option<String>,
    ) -> Result<(), VectorError> {
        Self::err()
    }

    pub fn query_vectors(
        &self,
        _table: &str,
        _query: Vec<f32>,
        _metric: DistanceMetric,
        _limit: u32,
<<<<<<< HEAD
<<<<<<< HEAD
        _filter_sql: Option<(String, Vec<String>)>,
    ) -> Result<Vec<(String, f32, Option<Vec<f32>>)>, VectorError> {
        Self::err()
    }

    pub fn get_vectors_by_ids(
        &self,
        _table: &str,
        _ids: Vec<String>,
        _namespace: Option<String>,
    ) -> Result<Vec<VectorRecord>, VectorError> {
        Self::err()
    }

    pub fn update_vector(
        &self,
        _table: &str,
        _id: String,
        _vector: Option<VectorData>,
        _metadata: Option<Metadata>,
        _merge_metadata: bool,
        _namespace: Option<String>,
    ) -> Result<(), VectorError> {
        Self::err()
    }

    pub fn delete_vectors(
        &self,
        _table: &str,
        _ids: Vec<String>,
        _namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        Self::err()
    }

    pub fn delete_by_filter(
        &self,
        _table: &str,
        _filter_sql: (String, Vec<String>),
        _namespace: Option<String>,
    ) -> Result<u32, VectorError> {
        Self::err()
    }

    pub fn list_vectors(
        &self,
        _table: &str,
        _filter_sql: Option<(String, Vec<String>)>,
        _limit: Option<u32>,
        _cursor: Option<String>,
        _namespace: Option<String>,
    ) -> Result<(Vec<VectorRecord>, Option<String>), VectorError> {
        Self::err()
    }

    pub fn count_vectors(
        &self,
        _table: &str,
        _filter_sql: Option<(String, Vec<String>)>,
        _namespace: Option<String>,
    ) -> Result<u64, VectorError> {
        Self::err()
    }
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
        _filter_sql: Option<(String, Vec<serde_json::Value>)>,
    ) -> Result<Vec<(String, f32, Option<Vec<f32>>)>, VectorError> {
        Self::err()
    }
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
}

// -----------------------------------------------------------------------------
// Native build – real SQL implementation
// -----------------------------------------------------------------------------
#[cfg(not(target_family = "wasm"))]
mod native {
    use super::*;
    use postgres::{Client, NoTls, Row};
    use serde_json::Value;
<<<<<<< HEAD
<<<<<<< HEAD
    use std::time::Duration;
    use r2d2_postgres::{PostgresConnectionManager, r2d2::Pool};

    pub struct PgvectorClient {
        pool: Pool<PostgresConnectionManager<NoTls>>,
        timeout: Duration,
        max_retries: u32,
    }

    impl PgvectorClient {
        pub fn new(database_url: String) -> Result<Self, VectorError> {
            Self::new_with_pool(database_url, 10, Duration::from_secs(30), 3)
        }

        pub fn new_with_pool(
            database_url: String,
            max_connections: u32,
            timeout: Duration,
            max_retries: u32,
        ) -> Result<Self, VectorError> {
            let manager = PostgresConnectionManager::new(
                database_url.parse().map_err(|e| VectorError::ConfigurationError(
                    format!("Invalid database URL: {}", e)
                ))?,
                NoTls,
            );
            let pool = Pool::builder()
                .max_size(max_connections)
                .connection_timeout(timeout)
                .build(manager)
                .map_err(|e| VectorError::ProviderError(format!("Pool creation error: {}", e)))?;
            Ok(Self {
                pool,
                timeout,
                max_retries,
            })
        }

        // Helper method for retry logic
        fn with_retry<F, T>(&self, mut operation: F) -> Result<T, VectorError>
        where
            F: FnMut() -> Result<T, VectorError>,
        {
            for attempt in 0..=self.max_retries {
                match operation() {
                    Ok(result) => return Ok(result),
                    Err(e) if attempt == self.max_retries => return Err(e),
                    Err(_) => std::thread::sleep(Duration::from_millis(100 * (attempt + 1) as u64)),
                }
            }
            unreachable!()
        }

        // Helper to get a connection from the pool
        fn get_connection(&self) -> Result<r2d2_postgres::r2d2::PooledConnection<PostgresConnectionManager<NoTls>>, VectorError> {
            self.pool.get().map_err(|e| VectorError::ProviderError(
                format!("Failed to get database connection: {}", e)
            ))
        }

        pub fn create_collection(&self, name: &str, dimension: u32) -> Result<(), VectorError> {
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da

    pub struct PgvectorClient {
        client: Client,
    }

    impl PgvectorClient {
        pub fn new(database_url: String) -> Self {
            let client =
                Client::connect(&database_url, NoTls).expect("Failed to connect to Postgres");
            Self { client }
        }

        pub fn create_collection(&mut self, name: &str, dimension: u32) -> Result<(), VectorError> {
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
            let sql = format!(
                "CREATE TABLE IF NOT EXISTS {} (id TEXT PRIMARY KEY, embedding vector({}), metadata JSONB)",
                name, dimension
            );
<<<<<<< HEAD
<<<<<<< HEAD
            let mut conn = self.get_connection()?;
            conn.execute(sql.as_str(), &[]).map_err(to_err)?;
=======
            self.client.execute(sql.as_str(), &[]).map_err(to_err)?;
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
            self.client.execute(sql.as_str(), &[]).map_err(to_err)?;
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
            // index for faster similarity search
            let idx_sql = format!(
                "CREATE INDEX IF NOT EXISTS {}_embedding_idx ON {} USING ivfflat (embedding)",
                name, name
            );
<<<<<<< HEAD
<<<<<<< HEAD
            conn.execute(idx_sql.as_str(), &[]).map_err(to_err)?;
            Ok(())
        }

        pub fn list_collections(&self) -> Result<Vec<String>, VectorError> {
            let mut conn = self.get_connection()?;
            let rows = conn
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
            self.client.execute(idx_sql.as_str(), &[]).map_err(to_err)?;
            Ok(())
        }

        pub fn list_collections(&mut self) -> Result<Vec<String>, VectorError> {
            let rows = self
                .client
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
                .query(
                    "SELECT table_name FROM information_schema.tables WHERE table_schema='public'",
                    &[],
                )
                .map_err(to_err)?;
            Ok(rows.iter().map(|r| r.get::<_, String>(0)).collect())
        }

<<<<<<< HEAD
<<<<<<< HEAD
        pub fn delete_collection(&self, name: &str) -> Result<(), VectorError> {
            let sql = format!("DROP TABLE IF EXISTS {}", name);
            let mut conn = self.get_connection()?;
            conn.execute(sql.as_str(), &[]).map_err(to_err)?;
=======
        pub fn delete_collection(&mut self, name: &str) -> Result<(), VectorError> {
            let sql = format!("DROP TABLE IF EXISTS {}", name);
            self.client.execute(sql.as_str(), &[]).map_err(to_err)?;
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
        pub fn delete_collection(&mut self, name: &str) -> Result<(), VectorError> {
            let sql = format!("DROP TABLE IF EXISTS {}", name);
            self.client.execute(sql.as_str(), &[]).map_err(to_err)?;
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
            Ok(())
        }

        pub fn upsert_vectors(
<<<<<<< HEAD
<<<<<<< HEAD
            &self,
=======
            &mut self,
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
            &mut self,
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
            table: &str,
            records: Vec<VectorRecord>,
            _namespace: Option<String>,
        ) -> Result<(), VectorError> {
            let sql = format!(
<<<<<<< HEAD
<<<<<<< HEAD
                "INSERT INTO {} (id, embedding, metadata) VALUES ($1, $2::vector, $3) ON CONFLICT (id) DO UPDATE SET embedding = EXCLUDED.embedding, metadata = EXCLUDED.metadata",
                table
            );
            let mut conn = self.get_connection()?;
            for rec in records {
                let dense = vector_data_to_dense(rec.vector)?;
                let meta = rec
                    .metadata
                    .map(|m| serde_json::Value::Object(metadata_to_json_map(Some(m))));
<<<<<<< HEAD
                let dense_text = to_vector_text(&dense);
                conn.execute(&sql, &[&rec.id, &dense_text, &meta])
=======
                conn.execute(&sql, &[&rec.id, &dense, &meta])
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
                "INSERT INTO {} (id, embedding, metadata) VALUES ($1, $2, $3) ON CONFLICT (id) DO UPDATE SET embedding = EXCLUDED.embedding, metadata = EXCLUDED.metadata",
                table
            );
            for rec in records {
                let dense = vector_data_to_dense(rec.vector)?;
                let meta = rec.metadata.map(|m| {
                    serde_json::Value::Object(
                        m.into_iter()
                            .map(|(k, v)| (k, serde_json::Value::String(format!("{}", v))))
                            .collect(),
                    )
                });
                self.client
                    .execute(&sql, &[&rec.id, &dense, &meta])
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
>>>>>>> 54db59b006712dd19266b3696202a3a95d62010a
                    .map_err(to_err)?;
            }
            Ok(())
        }

        pub fn query_vectors(
<<<<<<< HEAD
<<<<<<< HEAD
            &self,
=======
            &mut self,
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
            &mut self,
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
            table: &str,
            query: Vec<f32>,
            metric: DistanceMetric,
            limit: u32,
<<<<<<< HEAD
<<<<<<< HEAD
            filter_sql: Option<(String, Vec<String>)>,
        ) -> Result<Vec<(String, f32, Option<Vec<f32>>)>, VectorError> {
            let op = metric_to_pgvector(metric);
            let mut sql = format!(
                "SELECT id, embedding {} $1::vector AS distance, embedding::text FROM {}",
                op, table
            );
            // Hold filter values so parameter references remain valid during query execution
            let mut held_filter_values: Vec<String> = Vec::new();
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
            filter_sql: Option<(String, Vec<Value>)>,
        ) -> Result<Vec<(String, f32, Option<Vec<f32>>)>, VectorError> {
            let op = metric_to_pgvector(metric);
            let mut sql = format!(
                "SELECT id, embedding {} $1 AS distance, embedding FROM {}",
                op, table
            );
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
            let mut params: Vec<&(dyn postgres::types::ToSql + Sync)> = Vec::new();
            let query_text = to_vector_text(&query);
            params.push(&query_text);
            if let Some((filter, values)) = filter_sql {
                sql.push_str(" WHERE ");
                sql.push_str(&filter);
<<<<<<< HEAD
<<<<<<< HEAD
                held_filter_values = values;
            }
            for v in &held_filter_values {
                params.push(v);
=======
                for v in &values {
                    params.push(v);
                }
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
                for v in &values {
                    params.push(v);
                }
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
            }
            sql.push_str(" ORDER BY distance ASC LIMIT ");
            sql.push_str(&limit.to_string());
<<<<<<< HEAD
            let mut conn = self.get_connection()?;
            let rows = conn.query(sql.as_str(), &params).map_err(to_err)?;
=======
            let rows = self.client.query(sql.as_str(), &params).map_err(to_err)?;
<<<<<<< HEAD
<<<<<<< HEAD
>>>>>>> 54db59b006712dd19266b3696202a3a95d62010a
            let mut out: Vec<(String, f32, Option<Vec<f32>>)> = Vec::with_capacity(rows.len());
            for row in rows.into_iter() {
                let id: String = row.get(0);
                let dist: f32 = row.get(1);
                let embedding_text: Option<String> = row.get(2);
                let maybe_vec = parse_vector_text_opt(embedding_text);
                out.push((id, dist, maybe_vec));
            }
            Ok(out)
        }

        pub fn get_vectors_by_ids(
            &self,
            table: &str,
            ids: Vec<String>,
            _namespace: Option<String>,
        ) -> Result<Vec<VectorRecord>, VectorError> {
            if ids.is_empty() {
                return Ok(Vec::new());
            }
            // Select id, embedding, metadata for provided IDs
            let sql = format!(
                "SELECT id, embedding::text, metadata FROM {} WHERE id = ANY($1)",
                table
            );
            let mut conn = self.get_connection()?;
            let rows = conn.query(sql.as_str(), &[&ids]).map_err(to_err)?;
            let mut out = Vec::with_capacity(rows.len());
            for row in rows {
                let id: String = row.get(0);
                let embedding_text: Option<String> = row.get(1);
                let metadata_json: Option<serde_json::Value> = row.get(2);
                if let Some(vec) = parse_vector_text_opt(embedding_text) {
                    let metadata = metadata_json.and_then(|v| match v {
                        serde_json::Value::Object(map) => Some(json_object_to_metadata(map)),
                        _ => None,
                    });
                    out.push(VectorRecord {
                        id,
                        vector: VectorData::Dense(vec),
                        metadata,
                    });
                }
            }
            Ok(out)
        }

        pub fn update_vector(
            &self,
            table: &str,
            id: String,
            vector: Option<VectorData>,
            metadata: Option<Metadata>,
            merge_metadata: bool,
            _namespace: Option<String>,
        ) -> Result<(), VectorError> {
            // Prepare optional params
            let dense_opt: Option<Vec<f32>> = match vector {
                Some(v) => Some(vector_data_to_dense(v)?),
                None => None,
            };
            let meta_json: Option<Value> = metadata
                .map(|m| Value::Object(metadata_to_json_map(Some(m))));

            let sql = format!(
                "UPDATE {} SET embedding = CASE WHEN $2 IS NULL THEN embedding ELSE $2::vector END, metadata = CASE WHEN $3 IS NULL THEN metadata ELSE CASE WHEN $4 THEN COALESCE(metadata, '{{}}'::jsonb) || $3 ELSE $3 END END WHERE id = $1",
                table
            );
            let merge_flag = merge_metadata;
            let mut conn = self.get_connection()?;
            // Convert optional vector to optional text for ::vector cast
            let dense_opt_text: Option<String> = dense_opt.as_ref().map(|v| to_vector_text(v));
            conn.execute(&sql, &[&id, &dense_opt_text, &meta_json, &merge_flag])
                .map_err(to_err)?;
            Ok(())
        }

        pub fn delete_vectors(
            &self,
            table: &str,
            ids: Vec<String>,
            _namespace: Option<String>,
        ) -> Result<u32, VectorError> {
            if ids.is_empty() {
                return Ok(0);
            }
            let sql = format!("DELETE FROM {} WHERE id = ANY($1)", table);
            let mut conn = self.get_connection()?;
            let n = conn.execute(sql.as_str(), &[&ids]).map_err(to_err)?;
            Ok(n as u32)
        }

        pub fn delete_by_filter(
            &self,
            table: &str,
            filter_sql: (String, Vec<String>),
            _namespace: Option<String>,
        ) -> Result<u32, VectorError> {
            let (where_sql, values) = filter_sql;
            let mut sql = format!("DELETE FROM {} WHERE {}", table, where_sql);
            // Build params vec
            let mut params: Vec<&(dyn postgres::types::ToSql + Sync)> = Vec::new();
            for v in &values {
                params.push(v);
            }
            let mut conn = self.get_connection()?;
            let n = conn.execute(sql.as_str(), &params).map_err(to_err)?;
            Ok(n as u32)
        }

        pub fn list_vectors(
            &self,
            table: &str,
            filter_sql: Option<(String, Vec<String>)>,
            limit: Option<u32>,
            cursor: Option<String>,
            _namespace: Option<String>,
        ) -> Result<(Vec<VectorRecord>, Option<String>), VectorError> {
            let limit = limit.unwrap_or(100);
            let offset: i64 = cursor
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);

            let mut sql = format!("SELECT id, embedding::text, metadata FROM {}", table);
            // Hold filter values so parameter references remain valid during query execution
            let mut held_filter_values: Vec<String> = Vec::new();
            let mut params: Vec<&(dyn postgres::types::ToSql + Sync)> = Vec::new();
            if let Some((where_sql, values)) = filter_sql {
                sql.push_str(" WHERE ");
                sql.push_str(&where_sql);
                held_filter_values = values;
            }
            sql.push_str(" ORDER BY id ASC LIMIT ");
            sql.push_str(&limit.to_string());
            sql.push_str(" OFFSET ");
            sql.push_str(&offset.to_string());
            for v in &held_filter_values {
                params.push(v);
            }

            let mut conn = self.get_connection()?;
            let rows = conn.query(sql.as_str(), &params).map_err(to_err)?;
            let mut out = Vec::with_capacity(rows.len());
            for row in rows.iter() {
                let id: String = row.get(0);
                let embedding_text: Option<String> = row.get(1);
                let metadata_json: Option<Value> = row.get(2);
                if let Some(vec) = parse_vector_text_opt(embedding_text) {
                    let metadata = metadata_json.and_then(|v| match v {
                        Value::Object(map) => Some(json_object_to_metadata(map)),
                        _ => None,
                    });
                    out.push(VectorRecord {
                        id,
                        vector: VectorData::Dense(vec),
                        metadata,
                    });
                }
            }
            let next_cursor = if out.len() as u32 == limit {
                Some((offset + limit as i64).to_string())
            } else {
                None
            };
            Ok((out, next_cursor))
        }

        pub fn count_vectors(
            &self,
            table: &str,
            filter_sql: Option<(String, Vec<String>)>,
            _namespace: Option<String>,
        ) -> Result<u64, VectorError> {
            let mut sql = format!("SELECT COUNT(*) FROM {}", table);
            // Hold filter values so parameter references remain valid during query execution
            let mut held_filter_values: Vec<String> = Vec::new();
            let mut params: Vec<&(dyn postgres::types::ToSql + Sync)> = Vec::new();
            if let Some((where_sql, values)) = filter_sql {
                sql.push_str(" WHERE ");
                sql.push_str(&where_sql);
                held_filter_values = values;
            }
            for v in &held_filter_values {
                params.push(v);
            }
            let mut conn = self.get_connection()?;
            let row = conn.query_one(sql.as_str(), &params).map_err(to_err)?;
            let count: i64 = row.get(0);
            Ok(count as u64)
        }
    }

    fn parse_vector_text_opt(s: Option<String>) -> Option<Vec<f32>> {
        s.as_deref().and_then(parse_vector_text)
    }

    fn parse_vector_text(s: &str) -> Option<Vec<f32>> {
        let s = s.trim();
        let inner = s.strip_prefix('[')?.strip_suffix(']')?;
        if inner.trim().is_empty() {
            return Some(Vec::new());
        }
        let mut out = Vec::new();
        for part in inner.split(',') {
            match part.trim().parse::<f32>() {
                Ok(v) => out.push(v),
                Err(_) => return None,
            }
        }
        Some(out)
=======
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
            Ok(rows.into_iter().map(row_to_tuple).collect())
        }
    }

    fn row_to_tuple(row: Row) -> (String, f32, Option<Vec<f32>>) {
        let id: String = row.get(0);
        let dist: f32 = row.get(1);
        let embedding: Option<Vec<f32>> = row.get(2);
        (id, dist, embedding)
<<<<<<< HEAD
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
=======
>>>>>>> 99fae2e2b91a5f023d76b6603d8b38164ebb18da
    }

    fn to_vector_text(v: &[f32]) -> String {
        let mut s = String::with_capacity(v.len() * 6 + 2);
        s.push('[');
        for (i, val) in v.iter().enumerate() {
            if i > 0 { s.push_str(", "); }
            s.push_str(&val.to_string());
        }
        s.push(']');
        s
    }

    fn to_err(e: impl std::fmt::Display) -> VectorError {
        VectorError::ProviderError(e.to_string())
    }
}

#[cfg(not(target_family = "wasm"))]
pub use native::PgvectorClient;

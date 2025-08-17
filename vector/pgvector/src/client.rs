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
use golem_vector::exports::golem::vector::types::{DistanceMetric, VectorError, VectorRecord};

use crate::conversion::{filter_expression_to_sql, metric_to_pgvector, vector_data_to_dense};

#[cfg(target_family = "wasm")]
pub struct PgvectorClient;

#[cfg(target_family = "wasm")]
impl PgvectorClient {
    pub fn new(_database_url: String) -> Self {
        PgvectorClient
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
        _filter_sql: Option<(String, Vec<serde_json::Value>)>,
    ) -> Result<Vec<(String, f32, Option<Vec<f32>>)>, VectorError> {
        Self::err()
    }
}

// -----------------------------------------------------------------------------
// Native build – real SQL implementation
// -----------------------------------------------------------------------------
#[cfg(not(target_family = "wasm"))]
mod native {
    use super::*;
    use postgres::{Client, NoTls, Row};
    use serde_json::Value;

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
            let sql = format!(
                "CREATE TABLE IF NOT EXISTS {} (id TEXT PRIMARY KEY, embedding vector({}), metadata JSONB)",
                name, dimension
            );
            self.client.execute(sql.as_str(), &[]).map_err(to_err)?;
            // index for faster similarity search
            let idx_sql = format!(
                "CREATE INDEX IF NOT EXISTS {}_embedding_idx ON {} USING ivfflat (embedding)",
                name, name
            );
            self.client.execute(idx_sql.as_str(), &[]).map_err(to_err)?;
            Ok(())
        }

        pub fn list_collections(&mut self) -> Result<Vec<String>, VectorError> {
            let rows = self
                .client
                .query(
                    "SELECT table_name FROM information_schema.tables WHERE table_schema='public'",
                    &[],
                )
                .map_err(to_err)?;
            Ok(rows.iter().map(|r| r.get::<_, String>(0)).collect())
        }

        pub fn delete_collection(&mut self, name: &str) -> Result<(), VectorError> {
            let sql = format!("DROP TABLE IF EXISTS {}", name);
            self.client.execute(sql.as_str(), &[]).map_err(to_err)?;
            Ok(())
        }

        pub fn upsert_vectors(
            &mut self,
            table: &str,
            records: Vec<VectorRecord>,
            _namespace: Option<String>,
        ) -> Result<(), VectorError> {
            let sql = format!(
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
                    .map_err(to_err)?;
            }
            Ok(())
        }

        pub fn query_vectors(
            &mut self,
            table: &str,
            query: Vec<f32>,
            metric: DistanceMetric,
            limit: u32,
            filter_sql: Option<(String, Vec<Value>)>,
        ) -> Result<Vec<(String, f32, Option<Vec<f32>>)>, VectorError> {
            let op = metric_to_pgvector(metric);
            let mut sql = format!(
                "SELECT id, embedding {} $1 AS distance, embedding FROM {}",
                op, table
            );
            let mut params: Vec<&(dyn postgres::types::ToSql + Sync)> = Vec::new();
            params.push(&query);
            if let Some((filter, values)) = filter_sql {
                sql.push_str(" WHERE ");
                sql.push_str(&filter);
                for v in &values {
                    params.push(v);
                }
            }
            sql.push_str(" ORDER BY distance ASC LIMIT ");
            sql.push_str(&limit.to_string());
            let rows = self.client.query(sql.as_str(), &params).map_err(to_err)?;
            Ok(rows.into_iter().map(row_to_tuple).collect())
        }
    }

    fn row_to_tuple(row: Row) -> (String, f32, Option<Vec<f32>>) {
        let id: String = row.get(0);
        let dist: f32 = row.get(1);
        let embedding: Option<Vec<f32>> = row.get(2);
        (id, dist, embedding)
    }

    fn to_err(e: impl std::fmt::Display) -> VectorError {
        VectorError::ProviderError(e.to_string())
    }
}

#[cfg(not(target_family = "wasm"))]
pub use native::PgvectorClient;

//! Conversions and helpers for the pgvector provider.
//!
//! * Convert WIT `VectorData` → Postgres array (Vec<f32>)
//! * Translate filter expressions into SQL `WHERE` fragments (very limited)
//! * Map WIT `DistanceMetric` to pgvector operator / function names

use golem_vector::error::invalid_vector;
use golem_vector::exports::golem::vector::types::{
    DistanceMetric, FilterCondition, FilterExpression, FilterOperator, Metadata, MetadataValue,
    VectorData, VectorError,
};
use serde_json::{json, Value};
use std::collections::HashMap;

// -----------------------------------------------------------------------------
// Metric mapping
// -----------------------------------------------------------------------------

pub fn metric_to_pgvector(metric: DistanceMetric) -> &'static str {
    match metric {
        DistanceMetric::Cosine => "<=>", // cosine distance operator in pgvector
        DistanceMetric::Euclidean => "<->", // l2 distance operator
        DistanceMetric::Dot => "<#>",    // negative inner product
        _ => "<->",
    }
}

// -----------------------------------------------------------------------------
// Vector conversion
// -----------------------------------------------------------------------------

pub fn vector_data_to_dense(v: VectorData) -> Result<Vec<f32>, VectorError> {
    match v {
        VectorData::Dense(d) => Ok(d),
        _ => Err(invalid_vector("pgvector supports only dense vectors")),
    }
}

// -----------------------------------------------------------------------------
// Metadata handling – stored as JSONB columns when available
// -----------------------------------------------------------------------------

fn metadata_value_to_json(v: MetadataValue) -> Value {
    match v {
        MetadataValue::StringVal(s) => Value::String(s),
        MetadataValue::NumberVal(n) => Value::from(n),
        MetadataValue::IntegerVal(i) => Value::from(i),
        MetadataValue::BooleanVal(b) => Value::from(b),
        MetadataValue::NullVal => Value::Null,
        MetadataValue::ArrayVal(arr) => {
            Value::Array(arr.into_iter().map(metadata_value_to_json).collect())
        }
        MetadataValue::ObjectVal(obj) => Value::Object(
            obj.into_iter()
                .map(|(k, v)| (k, metadata_value_to_json(v)))
                .collect(),
        ),
        MetadataValue::GeoVal(coords) => json!({ "lat": coords.latitude, "lon": coords.longitude }),
        MetadataValue::DatetimeVal(dt) => Value::String(dt),
        MetadataValue::BlobVal(b) => Value::String(base64::encode(b)),
    }
}

pub fn metadata_to_json_map(meta: Option<Metadata>) -> HashMap<String, Value> {
    meta.map(|m| {
        m.into_iter()
            .map(|(k, v)| (k, metadata_value_to_json(v)))
            .collect::<HashMap<_, _>>()
    })
    .unwrap_or_default()
}

// -----------------------------------------------------------------------------
// VERY small subset of filter → SQL translation
// -----------------------------------------------------------------------------

/// Translate `FilterExpression` into SQL fragment and parameter list.
/// Returns `(sql, values)` where `values` are JSON-encoded.
/// Unsupported constructs yield `None`.
pub fn filter_expression_to_sql(expr: Option<FilterExpression>) -> Option<(String, Vec<Value>)> {
    fn cond_to_sql(cond: &FilterCondition, idx: usize) -> Option<(String, Value)> {
        let placeholder = format!("${}", idx);
        let field = format!("metadata->>'{}'", cond.field); // assumes JSONB metadata column
        match cond.operator {
            FilterOperator::Eq => Some((
                format!("{} = {}", field, placeholder),
                metadata_value_to_json(cond.value.clone()),
            )),
            FilterOperator::Gt => Some((
                format!("{}::numeric > {}", field, placeholder),
                metadata_value_to_json(cond.value.clone()),
            )),
            FilterOperator::Gte => Some((
                format!("{}::numeric >= {}", field, placeholder),
                metadata_value_to_json(cond.value.clone()),
            )),
            FilterOperator::Lt => Some((
                format!("{}::numeric < {}", field, placeholder),
                metadata_value_to_json(cond.value.clone()),
            )),
            FilterOperator::Lte => Some((
                format!("{}::numeric <= {}", field, placeholder),
                metadata_value_to_json(cond.value.clone()),
            )),
            _ => None,
        }
    }

    fn walk(expr: &FilterExpression, sql_parts: &mut Vec<String>, params: &mut Vec<Value>) {
        match expr {
            FilterExpression::Condition(c) => {
                if let Some((sql, val)) = cond_to_sql(c, params.len() + 1) {
                    sql_parts.push(sql);
                    params.push(val);
                }
            }
            FilterExpression::And(list) => {
                for e in list {
                    walk(e, sql_parts, params);
                }
            }
            _ => {
                // OR/NOT currently unsupported for simplicity
            }
        }
    }

    let expr = expr?;
    let mut parts = Vec::new();
    let mut params = Vec::new();
    walk(&expr, &mut parts, &mut params);
    if parts.is_empty() {
        None
    } else {
        Some((parts.join(" AND "), params))
    }
}

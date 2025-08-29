//! Conversions and helpers for the pgvector provider.
//!
//! * Convert WIT `VectorData` → Postgres vector literal (Vec<f32>)
//! * Translate filter expressions into SQL `WHERE` fragments
//! * Map WIT `DistanceMetric` to pgvector operator / function names

use golem_vector::conversion_errors::{validate_vector_dimension, ConversionError};
use golem_vector::exports::golem::vector::types::{
    DistanceMetric, FilterExpression, Metadata, MetadataKind, MetadataValue, VectorData,
    VectorError,
};
use serde_json::Value;

// -----------------------------------------------------------------------------
// Metric mapping
// -----------------------------------------------------------------------------

pub fn metric_to_pgvector(metric: DistanceMetric) -> &'static str {
    match metric {
        DistanceMetric::Cosine => "<=>", // cosine distance operator in pgvector
        DistanceMetric::Euclidean => "<->", // l2 distance operator
        DistanceMetric::DotProduct => "<#>", // negative inner product
        _ => "<->",
    }
}

// -----------------------------------------------------------------------------
// Vector conversion
// -----------------------------------------------------------------------------
/// Convert VectorData to dense f32 vector for Postgres/pgvector with validation
pub fn vector_data_to_dense(data: VectorData) -> Result<Vec<f32>, VectorError> {
    match data {
        VectorData::Dense(values) => {
            validate_vector_dimension(&values, None)?;
            Ok(values)
        }
        // Non-dense representations are not supported by this provider currently
        _ => Err(ConversionError::UnsupportedMetric {
            metric: "non-dense vector representation".to_string(),
            provider: "Pgvector".to_string(),
        }
        .into()),
    }
}

// -----------------------------------------------------------------------------
// Metadata handling – stored as JSONB columns when available
// -----------------------------------------------------------------------------

fn metadata_value_to_json(v: MetadataValue) -> Value {
    match v.kind {
        MetadataKind::StringVal(s) => Value::String(s),
        MetadataKind::IntVal(i) => Value::from(i),
        MetadataKind::FloatVal(n) => Value::from(n),
        MetadataKind::BoolVal(b) => Value::from(b),
        // Complex kinds reference other metadata values by ID. Without a resolver
        // context, we emit placeholders to keep behavior defined.
        MetadataKind::ArrayVal(_ids) => Value::Array(Vec::new()),
        MetadataKind::ObjectVal(_fields) => Value::Object(serde_json::Map::new()),
    }
}

pub fn metadata_to_json_map(meta: Option<Metadata>) -> serde_json::Map<String, Value> {
    meta.map(|m| {
        m.into_iter()
            .map(|(k, v)| (k, metadata_value_to_json(v)))
            .collect::<serde_json::Map<_, _>>()
    })
    .unwrap_or_default()
}

// -----------------------------------------------------------------------------
// VERY small subset of filter → SQL translation
// -----------------------------------------------------------------------------

/// Translate an optional `FilterExpression` into an optional SQL fragment and parameter list.
/// Returns `Some((sql, values))` where `values` are strings to bind, or `None` if no usable filter.
pub fn filter_expression_to_sql(
    expr: Option<FilterExpression>,
    start_param_index: usize,
) -> Option<(String, Vec<String>)> {
    use golem_vector::exports::golem::vector::types::{
        CompareCondition, EqualsCondition, FilterKind, InListCondition,
    };

    let expr = expr?;
    match expr.kind {
        FilterKind::Equals(EqualsCondition { key, value }) => {
            let sql = format!("metadata->>'{key}' = ${start_param_index}::text");
            Some((sql, vec![value]))
        }
        FilterKind::NotEquals(EqualsCondition { key, value }) => {
            let sql = format!("metadata->>'{key}' <> ${start_param_index}::text");
            Some((sql, vec![value]))
        }
        FilterKind::GreaterThan(CompareCondition { key, number }) => {
            let sql = format!(
                "(metadata->>'{key}')::numeric > ${start_param_index}::numeric"
            );
            Some((sql, vec![number.to_string()]))
        }
        FilterKind::LessThan(CompareCondition { key, number }) => {
            let sql = format!(
                "(metadata->>'{key}')::numeric < ${start_param_index}::numeric"
            );
            Some((sql, vec![number.to_string()]))
        }
        FilterKind::InList(InListCondition { key, values }) => {
            if values.is_empty() {
                return None;
            }
            let mut placeholders = Vec::with_capacity(values.len());
            let mut params = Vec::with_capacity(values.len());
            let mut cur = start_param_index;
            for v in values.into_iter() {
                placeholders.push(format!("${cur}::text"));
                params.push(v);
                cur += 1;
            }
            let sql = format!("metadata->>'{}' IN ({})", key, placeholders.join(", "));
            Some((sql, params))
        }
        // Logical operators reference other filter IDs; without a resolver context
        // we cannot translate them here.
        FilterKind::And(_) | FilterKind::Or(_) | FilterKind::Not(_) => None,
    }
}

// -----------------------------------------------------------------------------
// JSON -> Metadata helpers
// -----------------------------------------------------------------------------

pub fn json_to_metadata_value(v: &Value) -> MetadataValue {
    match v {
        Value::String(s) => MetadataValue {
            id: 0,
            kind: MetadataKind::StringVal(s.clone()),
        },
        Value::Number(n) => {
            if let Some(i) = n.as_u64() {
                MetadataValue {
                    id: 0,
                    kind: MetadataKind::IntVal(i),
                }
            } else if let Some(f) = n.as_f64() {
                MetadataValue {
                    id: 0,
                    kind: MetadataKind::FloatVal(f),
                }
            } else {
                MetadataValue {
                    id: 0,
                    kind: MetadataKind::StringVal(n.to_string()),
                }
            }
        }
        Value::Bool(b) => MetadataValue {
            id: 0,
            kind: MetadataKind::BoolVal(*b),
        },
        Value::Null => MetadataValue {
            id: 0,
            kind: MetadataKind::StringVal("null".to_string()),
        },
        // Without an arena to allocate referenced values, emit placeholders
        Value::Array(_arr) => MetadataValue {
            id: 0,
            kind: MetadataKind::ArrayVal(Vec::new()),
        },
        Value::Object(_map) => MetadataValue {
            id: 0,
            kind: MetadataKind::ObjectVal(Vec::new()),
        },
    }
}

pub fn json_object_to_metadata(map: serde_json::Map<String, Value>) -> Metadata {
    map.into_iter()
        .map(|(k, v)| (k, json_to_metadata_value(&v)))
        .collect()
}

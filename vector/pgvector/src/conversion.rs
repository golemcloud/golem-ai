//! Conversions and helpers for the pgvector provider.
//!
//! * Convert WIT `VectorData` → Postgres vector literal (Vec<f32>)
//! * Translate filter expressions into SQL `WHERE` fragments
//! * Map WIT `DistanceMetric` to pgvector operator / function names

use golem_vector::exports::golem::vector::types::{
    DistanceMetric,
    FilterExpression,
    FilterOperator,
    FilterValue,
    Metadata,
    MetadataValue,
    VectorData,
    VectorError,
};
use golem_vector::conversion_errors::{ConversionError, validate_vector_dimension};
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
        },
        VectorData::Sparse { .. } => Err(ConversionError::UnsupportedMetric {
            metric: "sparse vectors".to_string(),
            provider: "Pgvector".to_string(),
        }.into()),
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
        MetadataValue::GeoVal(coords) => {
            let mut map = serde_json::Map::new();
            map.insert("lat".into(), Value::from(coords.latitude));
            map.insert("lon".into(), Value::from(coords.longitude));
            Value::Object(map)
        }
        MetadataValue::DatetimeVal(dt) => Value::String(dt),
        MetadataValue::BlobVal(b) => Value::String(base64::encode(b)),
    }
}

pub fn metadata_to_json_map(meta: Option<Metadata>) -> serde_json::Map<String, Value> {
    meta
        .map(|m| {
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
    use golem_vector::exports::golem::vector::types::FilterCondition;

    fn value_to_param(v: &FilterValue) -> Option<String> {
        match v {
            FilterValue::StringVal(s) => Some(s.clone()),
            FilterValue::NumberVal(n) => Some(n.to_string()),
            FilterValue::IntegerVal(i) => Some(i.to_string()),
            FilterValue::BooleanVal(b) => Some(if *b { "true".into() } else { "false".into() }),
            FilterValue::ArrayVal(_) | FilterValue::ListVal(_) | FilterValue::ObjectVal(_) => None,
            FilterValue::NullVal => None,
        }
    }

    fn cond_to_sql(cond: &FilterCondition, idx: usize) -> Option<(String, Vec<String>, usize)> {
        let field_text = format!("metadata->>'{}'", cond.field);
        match cond.operator {
            FilterOperator::Eq => {
                let val = value_to_param(&cond.value)?;
                let sql = format!("{} = ${}::text", field_text, idx);
                Some((sql, vec![val], idx + 1))
            }
            FilterOperator::Gt | FilterOperator::Gte | FilterOperator::Lt | FilterOperator::Lte => {
                let val = value_to_param(&cond.value)?;
                let op = match cond.operator {
                    FilterOperator::Gt => ">",
                    FilterOperator::Gte => ">=",
                    FilterOperator::Lt => "<",
                    FilterOperator::Lte => "<=",
                    _ => unreachable!(),
                };
                let sql = format!("({})::numeric {} ${}::numeric", field_text, op, idx);
                Some((sql, vec![val], idx + 1))
            }
            FilterOperator::In | FilterOperator::Nin => {
                if let FilterValue::ListVal(list) = &cond.value {
                    let mut vals: Vec<String> = Vec::new();
                    let mut phs: Vec<String> = Vec::new();
                    let mut cur = idx;
                    for v in list {
                        if let Some(s) = value_to_param(v) {
                            vals.push(s);
                            phs.push(format!("${}::text", cur));
                            cur += 1;
                        }
                    }
                    if vals.is_empty() { return None; }
                    let op = if matches!(cond.operator, FilterOperator::In) { "IN" } else { "NOT IN" };
                    let sql = format!("{} {} ({})", field_text, op, phs.join(", "));
                    Some((sql, vals, cur))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn build_sql(expr: &FilterExpression, mut idx: usize) -> Option<(String, Vec<String>, usize)> {
        match expr {
            FilterExpression::Condition(c) => cond_to_sql(c, idx),
            FilterExpression::And(list) => {
                let mut parts: Vec<String> = Vec::new();
                let mut params: Vec<String> = Vec::new();
                let mut cur = idx;
                for e in list {
                    if let Some((sql, vals, next)) = build_sql(e, cur) {
                        parts.push(sql);
                        params.extend(vals);
                        cur = next;
                    }
                }
                if parts.is_empty() { None } else { Some((parts.join(" AND "), params, cur)) }
            }
            FilterExpression::Or(list) => {
                let mut parts: Vec<String> = Vec::new();
                let mut params: Vec<String> = Vec::new();
                let mut cur = idx;
                for e in list {
                    if let Some((sql, vals, next)) = build_sql(e, cur) {
                        parts.push(sql);
                        params.extend(vals);
                        cur = next;
                    }
                }
                if parts.is_empty() { None } else { Some((format!("({})", parts.join(" OR ")), params, cur)) }
            }
            FilterExpression::Not(inner) => {
                let (sql, vals, next) = build_sql(inner, idx)?;
                Some((format!("NOT ({})", sql), vals, next))
            }
        }
    }

    let expr = expr?;
    let (sql, params, _) = build_sql(&expr, start_param_index)?;
    if sql.trim().is_empty() { None } else { Some((sql, params)) }
}

// -----------------------------------------------------------------------------
// JSON -> Metadata helpers
// -----------------------------------------------------------------------------

pub fn json_to_metadata_value(v: &Value) -> MetadataValue {
    match v {
        Value::String(s) => MetadataValue::StringVal(s.clone()),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                MetadataValue::IntegerVal(i)
            } else if let Some(f) = n.as_f64() {
                MetadataValue::NumberVal(f)
            } else {
                MetadataValue::StringVal(n.to_string())
            }
        }
        Value::Bool(b) => MetadataValue::BooleanVal(*b),
        Value::Null => MetadataValue::NullVal,
        Value::Array(arr) => MetadataValue::ArrayVal(arr.iter().map(json_to_metadata_value).collect()),
        Value::Object(map) => MetadataValue::ObjectVal(
            map.iter().map(|(k, v)| (k.clone(), json_to_metadata_value(v))).collect(),
        ),
    }
}

pub fn json_object_to_metadata(map: serde_json::Map<String, Value>) -> Metadata {
    map.into_iter()
        .map(|(k, v)| (k, json_to_metadata_value(&v)))
        .collect()
}

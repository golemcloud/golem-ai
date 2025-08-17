//! Conversions and helpers for the pgvector provider.
//!
//! * Convert WIT `VectorData` → Postgres array (Vec<f32>)
//! * Translate filter expressions into SQL `WHERE` fragments (very limited)
//! * Map WIT `DistanceMetric` to pgvector operator / function names

use golem_vector::error::invalid_vector;
use golem_vector::exports::golem::vector::types::{
    DistanceMetric, FilterExpression, FilterOperator, FilterValue, Metadata, MetadataValue,
    VectorData, VectorError,
};
use golem_vector::conversion_errors::{ConversionError, validate_vector_dimension, validate_filter_depth};
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
        MetadataValue::FloatVal(n) => Value::from(n),
        MetadataValue::IntVal(i) => Value::from(i),
        MetadataValue::BoolVal(b) => Value::from(b),
        MetadataValue::ArrayVal(arr) => {
            Value::Array(arr.into_iter().map(metadata_value_to_json).collect())
        }
        MetadataValue::ObjectVal(obj) => Value::Object(
            obj.into_iter()
                .map(|(k, v)| (k, metadata_value_to_json(v)))
                .collect(),
        ),
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

/// Translate `FilterExpression` into SQL fragment and parameter list.
/// Returns `(sql, values)` where `values` are JSON-encoded.
/// Convert FilterExpression to SQL WHERE clause with parameters and validation
pub fn filter_expression_to_sql(
    expr: FilterExpression,
    start_param_index: usize,
) -> Result<(String, Vec<String>), VectorError> {
    // Validate filter depth for SQL complexity
    validate_filter_depth(&expr, 0, 6, "Pgvector", |e| {
        match e {
            FilterExpression::And(exprs) | FilterExpression::Or(exprs) => exprs.iter().collect(),
            FilterExpression::Not(inner) => vec![inner.as_ref()],
            _ => vec![],
                    _ => j.to_string(),
                }
            }
        }
    }

    fn cond_to_sql(cond: &FilterCondition, idx: usize) -> Option<(String, String)> {
        let placeholder = format!("${}", idx);
        let field_text = format!("metadata->>'{}'", cond.field); // JSONB -> text
        match cond.operator {
            // Compare as text explicitly to avoid type ambiguity
            FilterOperator::Eq => Some((
                format!("{} = {}::text", field_text, placeholder),
                value_to_string(&cond.value),
            )),
            // Numeric comparisons – cast both sides to numeric and also cast the param
            FilterOperator::Gt => Some((
    let mut params = Vec::new();
    let sql = build_sql_condition(&expr, start_param_index, &mut params)?;
    if sql.trim().is_empty() {
        return Err(ConversionError::FilterTranslation("Generated empty SQL condition".to_string()).into());
    }
    Ok((sql, params))
}

fn build_sql_condition(
    expr: &FilterExpression,
    start_param_index: usize,
    params: &mut Vec<String>,
) -> Result<String, VectorError> {
    match expr {
        FilterExpression::Condition(c) => {
            let placeholder = format!("${}", start_param_index);
            let field_text = format!("metadata->>'{}'", c.field); // JSONB -> text
            match c.operator {
                // Compare as text explicitly to avoid type ambiguity
                FilterOperator::Eq => Some((
                    format!("{} = {}::text", field_text, placeholder),
                    value_to_string(&c.value),
                )),
                // Numeric comparisons – cast both sides to numeric and also cast the param
                FilterOperator::Gt => Some((
                    format!("({})::numeric > {}::numeric", field_text, placeholder),
                    value_to_string(&c.value),
                )),
                FilterOperator::Gte => Some((
                    format!("({})::numeric >= {}::numeric", field_text, placeholder),
                    value_to_string(&c.value),
                )),
                FilterOperator::Lt => Some((
                    format!("({})::numeric < {}::numeric", field_text, placeholder),
                    value_to_string(&c.value),
                )),
                FilterOperator::Lte => Some((
                    format!("({})::numeric <= {}::numeric", field_text, placeholder),
                    value_to_string(&c.value),
                )),
                _ => None,
            }
        }
        FilterExpression::And(list) => {
            let mut parts = Vec::new();
            for e in list {
                let sql = build_sql_condition(e, start_param_index, params)?;
                parts.push(sql);
            }
            Some((parts.join(" AND "), String::new()))
        }
        _ => None,
    }
    .ok_or(ConversionError::UnsupportedFilterOperator(c.operator.to_string()))?
}

fn value_to_string(v: &FilterValue) -> String {
    match v {
        FilterValue::StringVal(s) => s.clone(),
        FilterValue::NumberVal(n) => n.to_string(),
        FilterValue::IntegerVal(i) => i.to_string(),
        FilterValue::BooleanVal(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        FilterValue::ListVal(list) => {
            let items: Vec<String> = list.iter().map(|v| {
                let j = metadata_value_to_json(v.clone());
                match j {
                    Value::String(s) => s,
                    _ => j.to_string(),
                }
            }).collect();
            format!("[{}]", items.join(", "))
        }
        FilterValue::ArrayVal(arr) => {
            let items: Vec<String> = arr.iter().map(|v| {
                let j = metadata_value_to_json(v.clone());
                match j {
                    Value::String(s) => s,
                    _ => j.to_string(),
                }
            }).collect();
            format!("[{}]", items.join(", "))
        }
    if parts.is_empty() {
        None
    } else {
        Some((parts.join(" AND "), params))
    }
}

// -----------------------------------------------------------------------------
// JSON -> Metadata helpers
// -----------------------------------------------------------------------------

pub fn json_to_metadata_value(v: &Value) -> MetadataValue {
    match v {
        Value::String(s) => MetadataValue::StringVal(s.clone()),
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                MetadataValue::IntVal(u)
            } else if let Some(f) = n.as_f64() {
                MetadataValue::FloatVal(f)
            } else {
                // Fallback to string representation
                MetadataValue::StringVal(n.to_string())
            }
        }
        Value::Bool(b) => MetadataValue::BoolVal(*b),
        Value::Null => MetadataValue::StringVal("null".into()),
        Value::Array(arr) => {
            MetadataValue::ArrayVal(arr.iter().map(json_to_metadata_value).collect())
        }
        Value::Object(map) => MetadataValue::ObjectVal(
            map.iter()
                .map(|(k, v)| (k.clone(), json_to_metadata_value(v)))
                .collect(),
        ),
    }
}

pub fn json_object_to_metadata(map: serde_json::Map<String, Value>) -> Metadata {
    map.into_iter()
        .map(|(k, v)| (k, json_to_metadata_value(&v)))
        .collect()
}

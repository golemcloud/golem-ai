//! Conversion helpers for Milvus provider.

use golem_vector::exports::golem::vector::types::{
    DistanceMetric, FilterExpression, Metadata, MetadataValue, VectorData, VectorError,
};
use golem_vector::conversion_errors::{validate_vector_dimension, ConversionError};
use serde_json::Value;
use std::collections::HashMap;

/// Convert VectorData to dense f32 vector for Milvus with validation
pub fn vector_data_to_dense(data: VectorData) -> Result<Vec<f32>, VectorError> {
    match data {
        VectorData::Dense(values) => {
            validate_vector_dimension(&values, None)?;
            Ok(values)
        },
        VectorData::Sparse { .. } => Err(ConversionError::UnsupportedMetric {
            metric: "sparse vectors".to_string(),
            provider: "Milvus".to_string(),
        }.into()),
    }
}

/// Converts metadata into Milvus JSON map (payload).
pub fn metadata_to_json_map(metadata: Option<Metadata>) -> Option<HashMap<String, Value>> {
    metadata.map(|m| {
        m.into_iter()
            .map(|(k, v)| (k, metadata_value_to_json(v)))
            .collect()
    })
}

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
        MetadataValue::GeoVal(_) | MetadataValue::DatetimeVal(_) | MetadataValue::BlobVal(_) => {
            Value::Null // unsupported types for now
        }
    }
}

/// Convert distance metric to Milvus metric type
pub fn metric_to_milvus(metric: DistanceMetric) -> &'static str {
    match metric {
        DistanceMetric::Cosine => "COSINE",
        DistanceMetric::Euclidean => "L2",
        DistanceMetric::DotProduct => "IP",
        DistanceMetric::Manhattan => "L1",
    }
}

/// Translate a `FilterExpression` into a Milvus boolean expression string.
///
/// Supported:
/// - Comparisons: `eq`, `gt`, `gte`, `lt`, `lte`
/// - Membership: `in`
/// - Boolean: `and`, `or`, `not` (nested combinations are handled with parentheses)
///
/// Unsupported constructs are skipped; if nothing remains, returns `None`.
pub fn filter_expression_to_milvus(expr: Option<FilterExpression>) -> Option<String> {
    let expr = expr?;
    build_expr(&expr)
}

fn build_expr(expr: &FilterExpression) -> Option<String> {
    use golem_vector::exports::golem::vector::types::{FilterCondition, FilterOperator};
    match expr {
        FilterExpression::Condition(FilterCondition { field, operator, value }) => {
            match operator {
                FilterOperator::Eq => literal(value).map(|v| format!("{} == {}", field, v)),
                FilterOperator::Gt => literal(value).map(|v| format!("{} > {}", field, v)),
                FilterOperator::Gte => literal(value).map(|v| format!("{} >= {}", field, v)),
                FilterOperator::Lt => literal(value).map(|v| format!("{} < {}", field, v)),
                FilterOperator::Lte => literal(value).map(|v| format!("{} <= {}", field, v)),
                FilterOperator::In => list_literal(value).map(|v| format!("{} in {}", field, v)),
                FilterOperator::Nin | FilterOperator::NotIn => list_literal(value).map(|v| format!("!({} in {})", field, v)),
                _ => None,
            }
        }
        FilterExpression::And(list) => {
            if list.is_empty() { return None; }
            let mut parts: Vec<String> = Vec::new();
            for e in list {
                if let Some(part) = build_expr(e) {
                    let formatted = if part.contains(" || ") { format!("({})", part) } else { part };
                    parts.push(formatted);
                }
            }
            if parts.is_empty() { None } else { Some(parts.join(" && ")) }
        }
        FilterExpression::Or(list) => {
            if list.is_empty() { return None; }
            let mut parts: Vec<String> = Vec::new();
            for e in list {
                if let Some(part) = build_expr(e) {
                    let formatted = if part.contains(" && ") { format!("({})", part) } else { part };
                    parts.push(formatted);
                }
            }
            if parts.is_empty() { None } else { Some(parts.join(" || ")) }
        }
        FilterExpression::Not(inner) => build_expr(inner).map(|s| format!("!({})", s)),
    }
}

fn literal(v: &MetadataValue) -> Option<String> {
    match v {
        MetadataValue::StringVal(s) => Some(format!("\"{}\"", s.replace('"', "\\\""))),
        MetadataValue::NumberVal(n) => Some(n.to_string()),
        MetadataValue::IntegerVal(i) => Some(i.to_string()),
        MetadataValue::BooleanVal(b) => Some(b.to_string()),
        _ => None,
    }
}

fn list_literal(v: &MetadataValue) -> Option<String> {
    match v {
        MetadataValue::ArrayVal(arr) => {
            if arr.is_empty() {
                return None;
            }
            let mut items = Vec::new();
            for item in arr {
                if let Some(s) = literal(item) { items.push(s); }
            }
            if items.is_empty() { None } else { Some(format!("[{}]", items.join(", "))) }
        }
        _ => None,
    }
}

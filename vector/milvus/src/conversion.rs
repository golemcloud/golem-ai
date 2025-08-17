//! Conversion helpers for Milvus provider.

use golem_vector::error::{invalid_vector, unsupported_feature, VectorError};
use golem_vector::exports::golem::vector::types::{
    DistanceMetric, FilterExpression, Metadata, MetadataValue, VectorData,
};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Converts `VectorData` into a dense `Vec<f32>` compatible with Milvus.
pub fn vector_data_to_dense(v: VectorData) -> Result<Vec<f32>, VectorError> {
    match v {
        VectorData::Dense(d) => Ok(d),
        _ => Err(invalid_vector(
            "Milvus currently supports only dense vectors",
        )),
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

/// Maps distance metric to Milvus metric string.
pub fn metric_to_milvus(metric: DistanceMetric) -> &'static str {
    match metric {
        DistanceMetric::Cosine => "COSINE",
        DistanceMetric::Euclidean => "L2",
        DistanceMetric::DotProduct => "IP",
        _ => "COSINE",
    }
}

/// Translate a limited `FilterExpression` into Milvus boolean expression string.
///
/// Supported: AND of simple equality comparisons.
pub fn filter_expression_to_milvus(expr: Option<FilterExpression>) -> Option<String> {
    expr.map(|e| build_expr(&e)).filter(|s| !s.is_empty())
}

fn build_expr(expr: &FilterExpression) -> String {
    use golem_vector::exports::golem::vector::types::{FilterCondition, FilterOperator};
    match expr {
        FilterExpression::Condition(FilterCondition {
            field,
            operator,
            value,
        }) => {
            match operator {
                FilterOperator::Eq => format!("{} == {}", field, literal(value)),
                FilterOperator::In => format!("{} in {}", field, list_literal(value)),
                _ => "".into(), // unsupported
            }
        }
        FilterExpression::And(list) => list
            .iter()
            .map(build_expr)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" && "),
        _ => "".into(), // other combinators unsupported for now
    }
}

fn literal(v: &MetadataValue) -> String {
    match v {
        MetadataValue::StringVal(s) => format!("\"{}\"", s.replace('\"', "\\\"")),
        MetadataValue::NumberVal(n) => n.to_string(),
        MetadataValue::IntegerVal(i) => i.to_string(),
        MetadataValue::BooleanVal(b) => b.to_string(),
        _ => "null".into(),
    }
}

fn list_literal(v: &MetadataValue) -> String {
    match v {
        MetadataValue::ArrayVal(arr) => {
            let items: Vec<String> = arr.iter().map(literal).collect();
            format!("[{}]", items.join(", "))
        }
        _ => "[]".into(),
    }
}

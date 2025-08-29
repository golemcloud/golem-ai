//! Conversion helpers for Milvus provider.

use golem_vector::conversion_errors::{validate_vector_dimension, ConversionError};
use golem_vector::exports::golem::vector::types::{
    DistanceMetric, FilterExpression, FilterKind, Metadata, MetadataKind, MetadataValue,
    VectorData, VectorError,
};
use serde_json::Value;
use std::collections::HashMap;

/// Convert VectorData to dense f32 vector for Milvus with validation
pub fn vector_data_to_dense(data: VectorData) -> Result<Vec<f32>, VectorError> {
    match data {
        VectorData::Dense(values) => {
            validate_vector_dimension(&values, None)?;
            Ok(values)
        }
        // Only dense vectors are supported by this client path currently
        _ => Err(ConversionError::UnsupportedMetric {
            metric: "non-dense vectors".to_string(),
            provider: "Milvus".to_string(),
        }
        .into()),
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
    match v.kind {
        MetadataKind::StringVal(s) => Value::String(s),
        MetadataKind::IntVal(i) => Value::from(i),
        MetadataKind::FloatVal(f) => Value::from(f),
        MetadataKind::BoolVal(b) => Value::from(b),
        // Complex kinds are not serialized into Milvus payload for now
        MetadataKind::ArrayVal(_) | MetadataKind::ObjectVal(_) => Value::Null,
    }
}

/// Convert distance metric to Milvus metric type
pub fn metric_to_milvus(metric: DistanceMetric) -> &'static str {
    match metric {
        DistanceMetric::Cosine => "COSINE",
        DistanceMetric::Euclidean => "L2",
        DistanceMetric::DotProduct => "IP",
        DistanceMetric::Manhattan => "L1",
        DistanceMetric::Hamming => "HAMMING",
        DistanceMetric::Jaccard => "JACCARD",
    }
}

/// Translate a `FilterExpression` (new WIT shape) into a Milvus boolean expression string.
/// Only simple forms are supported; complex logical references (and/or/not) are skipped.
pub fn filter_expression_to_milvus(expr: Option<FilterExpression>) -> Option<String> {
    let expr = expr?;
    match &expr.kind {
        FilterKind::Equals(cond) => Some(format!("{} == {}", cond.key, quote_str(&cond.value))),
        FilterKind::NotEquals(cond) => Some(format!("{} != {}", cond.key, quote_str(&cond.value))),
        FilterKind::GreaterThan(cond) => Some(format!("{} > {}", cond.key, cond.number)),
        FilterKind::LessThan(cond) => Some(format!("{} < {}", cond.key, cond.number)),
        FilterKind::InList(cond) => {
            if cond.values.is_empty() {
                return None;
            }
            let vals = cond
                .values
                .iter()
                .map(|s| quote_str(s))
                .collect::<Vec<_>>()
                .join(", ");
            Some(format!("{} in [{}]", cond.key, vals))
        }
        FilterKind::And(_) | FilterKind::Or(_) | FilterKind::Not(_) => None,
    }
}

fn quote_str(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\\\""))
}

use golem_vector::conversion_errors::{validate_vector_dimension, ConversionError};
use golem_vector::exports::golem::vector::types::{
    DistanceMetric, FilterExpression, FilterKind, Metadata, MetadataKind, MetadataValue,
    VectorData, VectorError,
};
use serde_json::{Map, Value};

// -----------------------------------------------------------------------------
// Metric mapping
// -----------------------------------------------------------------------------
/// Map WIT metric enum to Pinecone REST API string literal
pub fn metric_to_pinecone(metric: DistanceMetric) -> &'static str {
    match metric {
        DistanceMetric::Cosine => "cosine",
        DistanceMetric::DotProduct => "dotproduct",
        DistanceMetric::Euclidean => "euclidean",
        // Pinecone does not support others; treat unknown as cosine.
        _ => "cosine",
    }
}

// -----------------------------------------------------------------------------
// Vector conversion
// -----------------------------------------------------------------------------
/// Convert `VectorData` to a dense `Vec<f32>` acceptable by Pinecone
pub fn vector_data_to_dense(data: VectorData) -> Result<Vec<f32>, VectorError> {
    match data {
        VectorData::Dense(values) => {
            validate_vector_dimension(&values, None)?;
            Ok(values)
        }
        _ => Err(ConversionError::UnsupportedMetric {
            metric: "non-dense vector representation".into(),
            provider: "Pinecone".into(),
        }
        .into()),
    }
}

// -----------------------------------------------------------------------------
// Metadata mapping helpers
// -----------------------------------------------------------------------------
fn metadata_value_to_json(v: MetadataValue) -> Value {
    match v.kind {
        MetadataKind::StringVal(s) => Value::String(s),
        MetadataKind::IntVal(i) => Value::from(i),
        MetadataKind::FloatVal(f) => Value::from(f),
        MetadataKind::BoolVal(b) => Value::from(b),
        // Complex types are collapsed to null for now
        MetadataKind::ArrayVal(_) | MetadataKind::ObjectVal(_) => Value::Null,
    }
}

/// Convert metadata into JSON object expected by Pinecone REST API.
pub fn metadata_to_json_map(metadata: Option<Metadata>) -> Option<Map<String, Value>> {
    metadata.map(|m| {
        m.into_iter()
            .map(|(k, v)| (k, metadata_value_to_json(v)))
            .collect::<Map<_, _>>()
    })
}

// -----------------------------------------------------------------------------
// Filter → JSON translation
// -----------------------------------------------------------------------------
/// Translate optional `FilterExpression` into optional JSON object for Pinecone.
/// Only a subset of comparisons is supported.
pub fn filter_expression_to_pinecone(expr: Option<FilterExpression>) -> Option<Value> {
    let expr = expr?;
    match expr.kind {
        FilterKind::Equals(cond) => Some(json!({ cond.key: cond.value })),
        FilterKind::NotEquals(cond) => Some(json!({ cond.key: { "$ne": cond.value } })),
        FilterKind::GreaterThan(cond) => Some(json!({ cond.key: { "$gt": cond.number } })),
        FilterKind::LessThan(cond) => Some(json!({ cond.key: { "$lt": cond.number } })),
        FilterKind::InList(cond) => {
            if cond.values.is_empty() {
                None
            } else {
                Some(json!({ cond.key: { "$in": cond.values } }))
            }
        }
        // Unsupported logical forms currently
        FilterKind::And(_) | FilterKind::Or(_) | FilterKind::Not(_) => None,
    }
}

// -----------------------------------------------------------------------------
// JSON -> Metadata helpers
// -----------------------------------------------------------------------------

/// Convert `serde_json::Value` into WIT `MetadataValue` (very shallow mapping).
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
            kind: MetadataKind::StringVal("null".into()),
        },
        Value::Array(_) => MetadataValue {
            id: 0,
            kind: MetadataKind::ArrayVal(Vec::new()),
        },
        Value::Object(_) => MetadataValue {
            id: 0,
            kind: MetadataKind::ObjectVal(Vec::new()),
        },
    }
}

pub fn json_object_to_metadata(map: Map<String, Value>) -> Metadata {
    map.into_iter()
        .map(|(k, v)| (k, json_to_metadata_value(&v)))
        .collect()
}

// Re-export macro for `json!`
#[allow(unused_imports)]
use serde_json::json;

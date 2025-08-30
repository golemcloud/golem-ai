use golem_vector::conversion_errors::{validate_vector_dimension, ConversionError};
use golem_vector::exports::golem::vector::types::{
    DistanceMetric, FilterExpression, FilterKind, Metadata, MetadataKind, MetadataValue,
    VectorData, VectorError,
};
use serde_json::{json, Map, Value};

// -----------------------------------------------------------------------------
// Metric mapping
// -----------------------------------------------------------------------------
/// Map WIT metric enum to Qdrant distance string
pub fn metric_to_qdrant(metric: DistanceMetric) -> &'static str {
    match metric {
        DistanceMetric::Cosine => "Cosine",
        DistanceMetric::DotProduct => "Dot",
        DistanceMetric::Euclidean => "Euclid",
        _ => "Cosine",
    }
}

// -----------------------------------------------------------------------------
// Vector conversion
// -----------------------------------------------------------------------------
/// Convert `VectorData` to dense vector for Qdrant
pub fn vector_data_to_dense(data: VectorData) -> Result<Vec<f32>, VectorError> {
    match data {
        VectorData::Dense(vals) => {
            validate_vector_dimension(&vals, None)?;
            Ok(vals)
        }
        _ => Err(ConversionError::UnsupportedMetric {
            metric: "non-dense vector".into(),
            provider: "Qdrant".into(),
        }
        .into()),
    }
}

// -----------------------------------------------------------------------------
// Metadata helpers
// -----------------------------------------------------------------------------
fn metadata_value_to_json(v: MetadataValue) -> Value {
    match v.kind {
        MetadataKind::StringVal(s) => Value::String(s),
        MetadataKind::IntVal(i) => Value::from(i),
        MetadataKind::FloatVal(f) => Value::from(f),
        MetadataKind::BoolVal(b) => Value::from(b),
        MetadataKind::ArrayVal(_) | MetadataKind::ObjectVal(_) => Value::Null,
    }
}

pub fn metadata_to_json_map(meta: Option<Metadata>) -> Option<Map<String, Value>> {
    meta.map(|m| {
        m.into_iter()
            .map(|(k, v)| (k, metadata_value_to_json(v)))
            .collect()
    })
}

// -----------------------------------------------------------------------------
// Filter translation
// -----------------------------------------------------------------------------
/// Translate optional `FilterExpression` to Qdrant filter JSON (`filter` object)
pub fn filter_expression_to_qdrant(expr: Option<FilterExpression>) -> Option<Value> {
    let expr = expr?;
    let cond = match expr.kind {
        FilterKind::Equals(c) => json!({"key": c.key, "match": {"value": c.value}}),
        FilterKind::NotEquals(c) => json!({"key": c.key, "match": {"not": {"value": c.value}}}),
        FilterKind::GreaterThan(c) => json!({"key": c.key, "range": {"gt": c.number}}),
        FilterKind::LessThan(c) => json!({"key": c.key, "range": {"lt": c.number}}),
        FilterKind::InList(c) => {
            if c.values.is_empty() {
                return None;
            }
            json!({"key": c.key, "match": {"any": c.values}})
        }
        _ => return None,
    };
    Some(json!({"must": [cond]}))
}

// -----------------------------------------------------------------------------
// JSON -> Metadata mapping
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

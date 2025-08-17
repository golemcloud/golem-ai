//! Data conversions required by the Pinecone provider.
//!
//! This module supplies helpers for:
//! * mapping `DistanceMetric` → Pinecone metric string
//! * validating that vectors are dense f32 arrays
//! * turning `Metadata` / `MetadataValue` into JSON maps
//! * converting high-level `FilterExpression` into the JSON filter that
//!   Pinecone understands (limited subset)

use golem_vector::error::invalid_vector;
use golem_vector::exports::golem::vector::types::{
    DistanceMetric, FilterCondition, FilterExpression, FilterOperator, Metadata, MetadataValue,
    VectorData, VectorError,
};
use serde_json::{json, Value};
use std::collections::HashMap;

// -----------------------------------------------------------------------------
// Metrics & vectors
// -----------------------------------------------------------------------------

pub fn metric_to_pinecone(metric: DistanceMetric) -> &'static str {
    match metric {
        DistanceMetric::Cosine => "cosine",
        DistanceMetric::Dot => "dotproduct",
        DistanceMetric::Euclidean => "euclidean",
        _ => "cosine", // fallback
    }
}

pub fn vector_data_to_dense(v: VectorData) -> Result<Vec<f32>, VectorError> {
    match v {
        VectorData::Dense(d) => Ok(d),
        _ => Err(invalid_vector("Pinecone supports only dense vectors")),
    }
}

// -----------------------------------------------------------------------------
// Metadata → JSON
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

/// Convert optional metadata into a JSON map acceptable by Pinecone.
/// Returns an empty map if `None`.
pub fn metadata_to_json_map(meta: Option<Metadata>) -> HashMap<String, Value> {
    meta.map(|m| {
        m.into_iter()
            .map(|(k, v)| (k, metadata_value_to_json(v)))
            .collect::<HashMap<_, _>>()
    })
    .unwrap_or_default()
}

// -----------------------------------------------------------------------------
// Filter translation (limited subset)
// -----------------------------------------------------------------------------

/// Convert a high-level `FilterExpression` into Pinecone filter JSON.
/// Only a subset of operators is supported: `eq`, comparisons (`gt`, `gte`,
/// `lt`, `lte`) and membership (`in`).  Unsupported or deeply-nested constructs
/// fall back to `None` so the caller can return an error.
pub fn filter_expression_to_pinecone(expr: Option<FilterExpression>) -> Option<Value> {
    fn cond_to_json(cond: &FilterCondition) -> Option<Value> {
        let key = &cond.field;
        match cond.operator {
            FilterOperator::Eq => {
                Some(json!({ key: { "$eq": metadata_value_to_json(cond.value.clone()) } }))
            }
            FilterOperator::Gt => {
                Some(json!({ key: { "$gt": metadata_value_to_json(cond.value.clone()) } }))
            }
            FilterOperator::Gte => {
                Some(json!({ key: { "$gte": metadata_value_to_json(cond.value.clone()) } }))
            }
            FilterOperator::Lt => {
                Some(json!({ key: { "$lt": metadata_value_to_json(cond.value.clone()) } }))
            }
            FilterOperator::Lte => {
                Some(json!({ key: { "$lte": metadata_value_to_json(cond.value.clone()) } }))
            }
            FilterOperator::In => {
                Some(json!({ key: { "$in": metadata_value_to_json(cond.value.clone()) } }))
            }
            _ => None, // unsupported
        }
    }

    fn merge_objects(mut a: Value, b: Value) -> Value {
        if let (Value::Object(ref mut map_a), Value::Object(map_b)) = (&mut a, b) {
            for (k, v) in map_b {
                map_a.insert(k, v);
            }
        }
        a
    }

    fn walk(expr: &FilterExpression, acc: &mut Value) {
        match expr {
            FilterExpression::Condition(c) => {
                if let Some(j) = cond_to_json(c) {
                    *acc = merge_objects(acc.take(), j);
                }
            }
            FilterExpression::And(list) | FilterExpression::Or(list) => {
                for e in list {
                    walk(e, acc);
                }
            }
            FilterExpression::Not(_) => {
                // Pinecone does not support NOT directly – skip.
            }
        }
    }

    let mut root = Value::Object(serde_json::Map::new());
    let expr = expr?; // early return if None
    walk(&expr, &mut root);
    if root.as_object().map_or(true, |m| m.is_empty()) {
        None
    } else {
        Some(root)
    }
}

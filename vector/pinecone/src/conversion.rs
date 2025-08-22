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
use golem_vector::conversion_errors::{ConversionError, validate_vector_dimension, validate_filter_depth};
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
        VectorData::Dense(d) => {
            validate_vector_dimension(&d, None)?;
            Ok(d)
        },
        VectorData::Sparse { .. } => Err(ConversionError::UnsupportedMetric {
            metric: "sparse vectors".to_string(),
            provider: "Pinecone".to_string(),
        }.into()),
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
/// will return an error.
pub fn filter_expression_to_pinecone(expr: Option<FilterExpression>) -> Result<Option<Value>, VectorError> {
    if let Some(expr) = expr {
        // Validate filter depth (Pinecone has reasonable nesting limits)
        validate_filter_depth(&expr, 0, 8, "Pinecone", |e| {
            match e {
                FilterExpression::And(exprs) | FilterExpression::Or(exprs) => exprs.iter().collect(),
                FilterExpression::Not(inner) => vec![inner.as_ref()],
                _ => vec![],
            }
        })?;
        
        let result = convert_filter_expression(&expr)?;
        Ok(Some(result))
    } else {
        Ok(None)
    }
}

fn convert_filter_expression(expr: &FilterExpression) -> Result<Value, VectorError> {
    fn cond_to_json(cond: &FilterCondition) -> Result<Option<Value>, VectorError> {
        let key = &cond.field;
        match cond.operator {
            FilterOperator::Eq => {
                Ok(Some(json!({ key: { "$eq": metadata_value_to_json(cond.value.clone()) } })))
            }
            FilterOperator::Gt => {
                // Validate that value is numeric
                let val_json = metadata_value_to_json(cond.value.clone());
                if !val_json.is_number() {
                    return Err(ConversionError::ValidationFailed(
                        format!("GT operator requires numeric value, got {:?}", cond.value)
                    ).into());
                }
                Ok(Some(json!({ key: { "$gt": val_json } }))
            }
            FilterOperator::Gte => {
                // Validate that value is numeric
                let val_json = metadata_value_to_json(cond.value.clone());
                if !val_json.is_number() {
                    return Err(ConversionError::ValidationFailed(
                        format!("GTE operator requires numeric value, got {:?}", cond.value)
                    ).into());
                }
                Ok(Some(json!({ key: { "$gte": val_json } }))
            }
            FilterOperator::Lt => {
                // Validate that value is numeric
                let val_json = metadata_value_to_json(cond.value.clone());
                if !val_json.is_number() {
                    return Err(ConversionError::ValidationFailed(
                        format!("LT operator requires numeric value, got {:?}", cond.value)
                    ).into());
                }
                Ok(Some(json!({ key: { "$lt": val_json } }))
            }
            FilterOperator::Lte => {
                // Validate that value is numeric
                let val_json = metadata_value_to_json(cond.value.clone());
                if !val_json.is_number() {
                    return Err(ConversionError::ValidationFailed(
                        format!("LTE operator requires numeric value, got {:?}", cond.value)
                    ).into());
                }
                Ok(Some(json!({ key: { "$lte": val_json } }))
            }
            FilterOperator::In => {
                // Validate that value is an array
                match &cond.value {
                    MetadataValue::ArrayVal(_) => {
                        Ok(Some(json!({ key: { "$in": metadata_value_to_json(cond.value.clone()) } }))
                    },
                    _ => {
                        Err(ConversionError::ValidationFailed(
                            format!("IN operator requires array value, got {:?}", cond.value)
                        ).into())
                    }
                }
            }
            _ => {
                Err(ConversionError::UnsupportedFilterOperator {
                    operator: format!("{:?}", cond.operator),
                    provider: "Pinecone".to_string(),
                }.into())
            }
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

    fn walk(expr: &FilterExpression, acc: &mut Value) -> Result<(), VectorError> {
        match expr {
            FilterExpression::Condition(c) => {
                if let Some(j) = cond_to_json(c)? {
                    *acc = merge_objects(acc.take(), j);
                }
            }
            FilterExpression::And(list) => {
                if list.is_empty() {
                    return Err(ConversionError::FilterTranslation("AND expression cannot be empty".to_string()).into());
                }
                for e in list {
                    walk(e, acc)?;
                }
            }
            FilterExpression::Or(list) => {
                if list.is_empty() {
                    return Err(ConversionError::FilterTranslation("OR expression cannot be empty".to_string()).into());
                }
                for e in list {
                    walk(e, acc)?;
                }
            }
            FilterExpression::Not(_) => {
                // Pinecone does not support NOT directly – skip.
            }
        }
        Ok(())
    }

    let mut root = Value::Object(serde_json::Map::new());
    walk(expr, &mut root)?;
    
    if root.as_object().map_or(true, |m| m.is_empty()) {
        Err(ConversionError::FilterTranslation("Filter expression resulted in empty filter".to_string()).into())
    } else {
        Ok(root)
    }
}

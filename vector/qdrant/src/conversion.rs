use crate::client::{CollectionDescription, QdrantFilter, QdrantPoint};
use golem_vector::error::invalid_vector;
use golem_vector::exports::golem::vector::collections::CollectionInfo;
use golem_vector::exports::golem::vector::types::{
    DistanceMetric, FilterExpression, FilterOperator, FilterValue, Metadata, MetadataValue,
    VectorData, VectorError, VectorRecord,
};
use golem_vector::conversion_errors::{ConversionError, validate_vector_dimension, validate_filter_depth};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Convert VectorData to dense f32 vector for Qdrant with validation
pub fn vector_data_to_dense(data: VectorData) -> Result<Vec<f32>, VectorError> {
    match data {
        VectorData::Dense(values) => {
            validate_vector_dimension(&values, None)?;
            Ok(values)
        },
        VectorData::Sparse { .. } => Err(ConversionError::UnsupportedMetric {
            metric: "sparse vectors".to_string(),
            provider: "Qdrant".to_string(),
        }.into()),
    }
}

/// Converts optional metadata into Qdrant payload map.
pub fn metadata_to_payload(metadata: Option<Metadata>) -> Option<HashMap<String, Value>> {
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

/// Convert distance metric to Qdrant distance string with validation
pub fn metric_to_qdrant(metric: DistanceMetric) -> Result<&'static str, VectorError> {
    match metric {
        DistanceMetric::Cosine => Ok("Cosine"),
        DistanceMetric::Euclidean => Ok("Euclid"),
        DistanceMetric::DotProduct => Ok("Dot"),
        DistanceMetric::Manhattan => Err(ConversionError::UnsupportedMetric {
            metric: "Manhattan".to_string(),
            provider: "Qdrant".to_string(),
        }.into()),
    }
}

/// Converts a `VectorRecord` into a `QdrantPoint`.
pub fn record_to_qdrant_point(rec: VectorRecord) -> Result<QdrantPoint, VectorError> {
    let vector = vector_data_to_dense(rec.vector)?;
    let payload = metadata_to_payload(rec.metadata);
    Ok(QdrantPoint {
        id: rec.id,
        vector,
        payload,
    })
}

/// Convert FilterExpression to Qdrant filter JSON with validation
pub fn filter_expression_to_qdrant(expr: FilterExpression) -> Result<serde_json::Value, VectorError> {
    // Validate filter depth (Qdrant supports deep nesting but let's set a reasonable limit)
    validate_filter_depth(&expr, 0, 10, "Qdrant", |e| {
        match e {
            FilterExpression::And(exprs) | FilterExpression::Or(exprs) => exprs.iter().collect(),
            FilterExpression::Not(inner) => vec![inner.as_ref()],
            _ => vec![],
        }
    })?;
    
    convert_filter_expression(&expr)
}

fn convert_filter_expression(expr: &FilterExpression) -> Result<serde_json::Value, VectorError> {
    use golem_vector::exports::golem::vector::types::{FilterCondition, FilterOperator};

    fn cond_to_json(cond: &FilterCondition) -> Option<Value> {
        let key = &cond.field;
        match cond.operator {
            FilterOperator::Eq => Some(
                json!({ "key": key, "match": { "value": metadata_value_to_json(cond.value.clone()) } }),
            ),
            FilterOperator::Gt | FilterOperator::Gte | FilterOperator::Lt | FilterOperator::Lte => {
                // Validate numeric operations only work with numeric values
                let val_json = metadata_value_to_json(cond.value.clone());
                if !val_json.is_number() {
                    return None; // Invalid operation for non-numeric value
                }
                let op_str = match cond.operator {
                    FilterOperator::Gt => "gt",
                    FilterOperator::Gte => "gte",
                    FilterOperator::Lt => "lt",
                    FilterOperator::Lte => "lte",
                    _ => unreachable!(),
                };
                Some(
                    json!({ "key": key, "range": { op_str: metadata_value_to_json(cond.value.clone()) } }),
                )
            }
            FilterOperator::In => {
                if let FilterValue::ListVal(list) = &cond.value {
                    if list.is_empty() {
                        return None; // Empty IN list is invalid
                    }
                    let values: Vec<Value> = list.iter().map(|v| metadata_value_to_json(v.clone())).collect();
                    Some(json!({ "key": key, "match": { "any": values } }))
                } else {
                    None
                }
            },
            FilterOperator::Nin => Some(
                json!({ "key": key, "match": { "not_any": metadata_value_to_json(cond.value.clone()) } }),
            ),
            _ => None, // Unsupported operator
        }
    }

    fn walk(
        expr: &FilterExpression,
        must: &mut Vec<Value>,
        should: &mut Vec<Value>,
        must_not: &mut Vec<Value>,
    ) {
        match expr {
            FilterExpression::Condition(cond) => {
                if let Some(j) = cond_to_json(cond) {
                    must.push(j);
                }
            }
            FilterExpression::And(conditions) => {
                for e in conditions {
                    walk(e, must, should, must_not);
                }
            }
            FilterExpression::Or(list) => {
                for e in list {
                    let mut inner_should = Vec::new();
                    walk(e, &mut inner_should, &mut Vec::new(), &mut Vec::new());
                    if !inner_should.is_empty() {
                        should.extend(inner_should);
                    }
                }
            }
            FilterExpression::Not(inner) => {
                let mut temp = Vec::new();
                walk(inner, &mut temp, &mut Vec::new(), &mut Vec::new());
                if !temp.is_empty() {
                    must_not.extend(temp);
                }
            }
        }
    }

    match expr {
        FilterExpression::And(conditions) => {
            if conditions.is_empty() {
                return Err(ConversionError::FilterTranslation("AND expression cannot be empty".to_string()).into());
            }
            let must: Vec<Value> = conditions
                .iter()
                .filter_map(|e| convert_filter_expression(e).ok())
                .collect();
            if must.is_empty() {
                return Err(ConversionError::FilterTranslation("No valid conditions in AND expression".to_string()).into());
            }
            Ok(json!({ "must": must }))
        }
        FilterExpression::Or(conditions) => {
            if conditions.is_empty() {
                return Err(ConversionError::FilterTranslation("OR expression cannot be empty".to_string()).into());
            }
            let should: Vec<Value> = conditions
                .iter()
                .filter_map(|e| convert_filter_expression(e).ok())
                .collect();
            if should.is_empty() {
                return Err(ConversionError::FilterTranslation("No valid conditions in OR expression".to_string()).into());
            }
            Ok(json!({ "should": should }))
        }
        FilterExpression::Not(inner) => {
            let inner_filter = convert_filter_expression(inner)?;
            Ok(json!({ "must_not": [inner_filter] }))
        }
        FilterExpression::Condition(cond) => {
            if let Some(json_cond) = cond_to_json(cond) {
                Ok(json_cond)
            } else {
                Err(ConversionError::UnsupportedFilterOperator {
                    operator: format!("{:?}", cond.operator),
                    provider: "Qdrant".to_string(),
                }.into())
            }
        }
    }
}

/// Converts Qdrant collection description to WIT `CollectionInfo`.
pub fn collection_desc_to_info(desc: CollectionDescription) -> CollectionInfo {
    CollectionInfo {
        name: desc.name,
        description: None,
        dimension: 0,
        metric: DistanceMetric::Cosine,
        vector_count: desc.points_count,
        size_bytes: None,
        index_ready: true,
        created_at: None,
        updated_at: None,
        provider_stats: None,
    }
}

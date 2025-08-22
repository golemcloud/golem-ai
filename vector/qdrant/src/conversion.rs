use crate::client::{CollectionDescription, QdrantFilter, QdrantPoint};
use golem_vector::exports::golem::vector::collections::CollectionInfo;
use golem_vector::exports::golem::vector::types::{
    DistanceMetric, FilterExpression, FilterOperator, FilterValue, Metadata, MetadataValue,
    VectorData, VectorError, VectorRecord,
};
use golem_vector::conversion_errors::{ConversionError, validate_vector_dimension};
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

/// Convert an optional `FilterExpression` into an optional `QdrantFilter`.
/// Returns `None` if the input is `None` or if the expression cannot be translated.
pub fn filter_expression_to_qdrant(expr: Option<FilterExpression>) -> Option<QdrantFilter> {
    use golem_vector::exports::golem::vector::types::FilterCondition;

    fn cond_to_json(cond: &FilterCondition) -> Option<Value> {
        let key = &cond.field;
        match cond.operator {
            FilterOperator::Eq => Some(json!({
                "key": key,
                "match": { "value": metadata_value_to_json(cond.value.clone()) }
            })),
            FilterOperator::Gt | FilterOperator::Gte | FilterOperator::Lt | FilterOperator::Lte => {
                // Numeric comparisons require numeric values
                let val_json = metadata_value_to_json(cond.value.clone());
                if !val_json.is_number() { return None; }
                let op_str = match cond.operator {
                    FilterOperator::Gt => "gt",
                    FilterOperator::Gte => "gte",
                    FilterOperator::Lt => "lt",
                    FilterOperator::Lte => "lte",
                    _ => unreachable!(),
                };
                Some(json!({ "key": key, "range": { op_str: val_json } }))
            }
            FilterOperator::In => {
                if let FilterValue::ListVal(list) = &cond.value {
                    if list.is_empty() { return None; }
                    let values: Vec<Value> = list.iter().map(|v| metadata_value_to_json(v.clone())).collect();
                    Some(json!({ "key": key, "match": { "any": values } }))
                } else {
                    None
                }
            }
            FilterOperator::Nin => Some(json!({
                "key": key,
                "match": { "not_any": metadata_value_to_json(cond.value.clone()) }
            })),
            _ => None,
        }
    }

    fn convert(expr: &FilterExpression) -> Option<Value> {
        match expr {
            FilterExpression::Condition(cond) => cond_to_json(cond),
            FilterExpression::And(list) => {
                if list.is_empty() { return None; }
                let items: Vec<Value> = list.iter().filter_map(convert).collect();
                if items.is_empty() { return None; }
                Some(json!({ "must": items }))
            }
            FilterExpression::Or(list) => {
                if list.is_empty() { return None; }
                let items: Vec<Value> = list.iter().filter_map(convert).collect();
                if items.is_empty() { return None; }
                Some(json!({ "should": items }))
            }
            FilterExpression::Not(inner) => {
                let inner_val = convert(inner)?;
                Some(json!({ "must_not": [inner_val] }))
            }
        }
    }

    let v = convert(&expr?);
    v.map(|val| {
        if let Value::Object(map) = &val {
            let must = map.get("must").and_then(|v| v.as_array().cloned());
            let should = map.get("should").and_then(|v| v.as_array().cloned());
            let must_not = map.get("must_not").and_then(|v| v.as_array().cloned());
            if must.is_some() || should.is_some() || must_not.is_some() {
                QdrantFilter { must, should, must_not }
            } else {
                QdrantFilter { must: Some(vec![val]), should: None, must_not: None }
            }
        } else {
            QdrantFilter { must: Some(vec![val]), should: None, must_not: None }
        }
    })
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

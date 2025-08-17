use crate::client::{CollectionDescription, QdrantFilter, QdrantPoint};
use golem_vector::error::invalid_vector;
use golem_vector::exports::golem::vector::collections::CollectionInfo;
use golem_vector::exports::golem::vector::types::{
    DistanceMetric, FilterExpression, Metadata, MetadataValue, VectorData, VectorError,
    VectorRecord,
};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Converts a WIT `VectorData` into a dense `Vec<f32>` supported by Qdrant.
pub fn vector_data_to_dense(v: VectorData) -> Result<Vec<f32>, VectorError> {
    match v {
        VectorData::Dense(d) => Ok(d),
        _ => Err(invalid_vector(
            "Qdrant currently supports only dense vectors",
        )),
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

/// Maps WIT `DistanceMetric` to Qdrant distance string.
pub fn metric_to_qdrant(metric: DistanceMetric) -> &'static str {
    match metric {
        DistanceMetric::Cosine => "Cosine",
        DistanceMetric::Euclidean => "Euclid",
        DistanceMetric::DotProduct => "Dot",
        _ => "Cosine", // fallback
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

/// Convert a high-level `FilterExpression` into a (very limited) Qdrant filter.
///
/// The mapping supports only a subset of operators that are straightforward to
/// represent with the Qdrant REST API: `eq`, comparison operators (`gt`, `gte`,
/// `lt`, `lte`) and membership operators (`in`, `nin`).
///
/// Complex boolean expressions are flattened into the corresponding `must`,
/// `should` or `must_not` lists at the top level.  Nested combinations work
/// only one level deep – everything deeper is treated as unsupported and
/// silently ignored.  If the expression cannot be translated, `None` is
/// returned so that the caller can fall back to a provider error.
pub fn filter_expression_to_qdrant(expr: Option<FilterExpression>) -> Option<QdrantFilter> {
    use golem_vector::exports::golem::vector::types::{FilterCondition, FilterOperator};

    fn cond_to_json(cond: &FilterCondition) -> Option<Value> {
        let key = &cond.field;
        match cond.operator {
            FilterOperator::Eq => Some(
                json!({ "key": key, "match": { "value": metadata_value_to_json(cond.value.clone()) } }),
            ),
            FilterOperator::Gt | FilterOperator::Gte | FilterOperator::Lt | FilterOperator::Lte => {
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
            FilterOperator::In => Some(
                json!({ "key": key, "match": { "any": metadata_value_to_json(cond.value.clone()) } }),
            ),
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
            FilterExpression::And(list) => {
                for e in list {
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

    let expr = expr?; // early return None if no filter
    let mut must = Vec::new();
    let mut should = Vec::new();
    let mut must_not = Vec::new();
    walk(&expr, &mut must, &mut should, &mut must_not);

    if must.is_empty() && should.is_empty() && must_not.is_empty() {
        None
    } else {
        Some(QdrantFilter {
            must: if must.is_empty() { None } else { Some(must) },
            should: if should.is_empty() {
                None
            } else {
                Some(should)
            },
            must_not: if must_not.is_empty() {
                None
            } else {
                Some(must_not)
            },
        })
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

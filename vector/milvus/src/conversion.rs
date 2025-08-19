//! Conversion helpers for Milvus provider.

use golem_vector::error::{invalid_vector, unsupported_feature, VectorError};
use golem_vector::exports::golem::vector::types::{
<<<<<<< HEAD
    DistanceMetric, FilterExpression, FilterOperator, FilterValue, Metadata, MetadataValue,
    VectorData, VectorError,
};
use golem_vector::conversion_errors::{ConversionError, validate_vector_dimension, validate_filter_depth};
use serde_json::{json, Value};
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
=======
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
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
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

<<<<<<< HEAD
/// Convert distance metric to Milvus metric type with validation
pub fn metric_to_milvus(metric: DistanceMetric) -> Result<&'static str, VectorError> {
    match metric {
        DistanceMetric::Cosine => Ok("COSINE"),
        DistanceMetric::Euclidean => Ok("L2"),
        DistanceMetric::DotProduct => Ok("IP"),
        DistanceMetric::Manhattan => Ok("L1"),
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
pub fn filter_expression_to_milvus(expr: FilterExpression) -> Result<String, VectorError> {
    // Validate filter depth (Milvus has reasonable nesting limits)
    validate_filter_depth(&expr, 0, 8, "Milvus", |e| {
        match e {
            FilterExpression::And(exprs) | FilterExpression::Or(exprs) => exprs.iter().collect(),
            FilterExpression::Not(inner) => vec![inner.as_ref()],
            _ => vec![],
        }
    })?;
    
    convert_filter_expression(&expr)
}

fn convert_filter_expression(expr: &FilterExpression) -> Result<String, VectorError> {
    let s = build_expr(expr)?;
    if s.is_empty() { 
        Err(ConversionError::FilterTranslation("empty filter expression".to_string()).into()) 
    } else { 
        Ok(s) 
    }
}

fn build_expr(expr: &FilterExpression) -> Result<String, VectorError> {
=======
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
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
    use golem_vector::exports::golem::vector::types::{FilterCondition, FilterOperator};
    match expr {
        FilterExpression::Condition(FilterCondition {
            field,
            operator,
            value,
        }) => {
            match operator {
<<<<<<< HEAD
                FilterOperator::Eq => Ok(format!("{} == {}", field, literal(value)?)),
                FilterOperator::Gt => Ok(format!("{} > {}", field, literal(value)?)),
                FilterOperator::Gte => Ok(format!("{} >= {}", field, literal(value)?)),
                FilterOperator::Lt => Ok(format!("{} < {}", field, literal(value)?)),
                FilterOperator::Lte => Ok(format!("{} <= {}", field, literal(value)?)),
                FilterOperator::In => Ok(format!("{} in {}", field, list_literal(value)?)),
                FilterOperator::Nin => Ok(format!("!({} in {})", field, list_literal(value)?)),
                FilterOperator::NotIn => Ok(format!("!({} in {})", field, list_literal(value)?)),
                _ => Err(ConversionError::UnsupportedFilterOperator {
                    operator: format!("{:?}", operator),
                    provider: "Milvus".to_string(),
                }.into()),
            }
        }
        FilterExpression::And(list) => {
            if list.is_empty() {
                return Err(ConversionError::FilterTranslation("AND expression cannot be empty".to_string()).into());
            }
            let mut parts = Vec::new();
            for expr in list {
                let part = build_expr(expr)?;
                if !part.is_empty() {
                    let formatted = if part.contains(" || ") { format!("({})", part) } else { part };
                    parts.push(formatted);
                }
            }
            if parts.is_empty() {
                return Err(ConversionError::FilterTranslation("No valid conditions in AND expression".to_string()).into());
            }
            Ok(parts.join(" && "))
        }
        FilterExpression::Or(list) => {
            if list.is_empty() {
                return Err(ConversionError::FilterTranslation("OR expression cannot be empty".to_string()).into());
            }
            let mut parts = Vec::new();
            for expr in list {
                let part = build_expr(expr)?;
                if !part.is_empty() {
                    let formatted = if part.contains(" && ") { format!("({})", part) } else { part };
                    parts.push(formatted);
                }
            }
            if parts.is_empty() {
                return Err(ConversionError::FilterTranslation("No valid conditions in OR expression".to_string()).into());
            }
            Ok(parts.join(" || "))
        }
        FilterExpression::Not(inner) => {
            let s = build_expr(inner)?;
            if s.is_empty() { 
                Err(ConversionError::FilterTranslation("NOT expression cannot be empty".to_string()).into())
            } else { 
                Ok(format!("!({})", s)) 
            }
        }
    }
}

fn literal(v: &MetadataValue) -> Result<String, VectorError> {
    match v {
        MetadataValue::StringVal(s) => Ok(format!("\"{}\"", s.replace('\"', "\\\""))) ,
        MetadataValue::NumberVal(n) => Ok(n.to_string()),
        MetadataValue::IntegerVal(i) => Ok(i.to_string()),
        MetadataValue::BooleanVal(b) => Ok(b.to_string()),
        _ => Err(ConversionError::UnsupportedMetadata(format!("Unsupported metadata value type: {:?}", v)).into()),
    }
}

fn list_literal(v: &MetadataValue) -> Result<String, VectorError> {
    match v {
        MetadataValue::ArrayVal(arr) => {
            if arr.is_empty() {
                return Err(ConversionError::ValidationFailed("Array cannot be empty for IN operation".to_string()).into());
            }
            let mut items = Vec::new();
            for item in arr {
                items.push(literal(item)?);
            }
            Ok(format!("[{}]", items.join(", ")))
        }
        _ => Err(ConversionError::ValidationFailed("Value must be array or list for IN operation".to_string()).into()),
=======
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
>>>>>>> a6364a7537634b59f83c3bc53e389acf5dd86b49
    }
}

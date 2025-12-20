use crate::client::{
    CountVectorsResponse, CreateTableRequest, SearchRequest as PgSearchRequest,
    SearchResult as PgSearchResult, TableColumn, VectorData as PgVectorData,
    VectorResult as PgVectorResult,
};
use golem_rust::bindings::golem::rdbms::postgres::DbValue;
use golem_vector::golem::vector::search::SearchQuery;
use golem_vector::golem::vector::types::{
    DistanceMetric, FilterCondition, FilterExpression, FilterOperator, MetadataValue, SearchResult,
    VectorData, VectorError, VectorRecord,
};
use golem_vector::golem::vector::{
    analytics::CollectionStats as ExportCollectionStats,
    collections::CollectionInfo as ExportCollectionInfo,
};
use std::collections::HashMap;

pub fn string_value_to_db_value(value: &str, column_type: &str) -> Result<DbValue, VectorError> {
    match column_type.to_uppercase().as_str() {
        "TEXT" | "VARCHAR" | "CHAR" | "BPCHAR" => Ok(DbValue::Text(value.to_string())),
        "INTEGER" | "INT4" => value
            .parse::<i32>()
            .map(DbValue::Int4)
            .map_err(|_| VectorError::InvalidParams(format!("Invalid integer value: {}", value))),
        "BIGINT" | "INT8" => value
            .parse::<i64>()
            .map(DbValue::Int8)
            .map_err(|_| VectorError::InvalidParams(format!("Invalid bigint value: {}", value))),
        "REAL" | "FLOAT4" => value
            .parse::<f32>()
            .map(DbValue::Float4)
            .map_err(|_| VectorError::InvalidParams(format!("Invalid float value: {}", value))),
        "DOUBLE PRECISION" | "FLOAT8" => value
            .parse::<f64>()
            .map(DbValue::Float8)
            .map_err(|_| VectorError::InvalidParams(format!("Invalid double value: {}", value))),
        "BOOLEAN" | "BOOL" => match value.to_lowercase().as_str() {
            "true" | "t" | "yes" | "y" | "1" => Ok(DbValue::Boolean(true)),
            "false" | "f" | "no" | "n" | "0" => Ok(DbValue::Boolean(false)),
            _ => Err(VectorError::InvalidParams(format!(
                "Invalid boolean value: {}",
                value
            ))),
        },
        "TIMESTAMP" | "TIMESTAMPTZ" => Ok(DbValue::Text(value.to_string())),
        "DATE" => Ok(DbValue::Text(value.to_string())),
        "TIME" => Ok(DbValue::Text(value.to_string())),
        _ => Ok(DbValue::Text(value.to_string())),
    }
}

pub fn distance_metric_to_pgvector_operator(metric: &DistanceMetric) -> String {
    match metric {
        DistanceMetric::Cosine => "<=>".to_string(),
        DistanceMetric::Euclidean => "<->".to_string(),
        DistanceMetric::DotProduct => "<#>".to_string(),
        DistanceMetric::Manhattan => "<+>".to_string(),
        DistanceMetric::Hamming => "<->".to_string(),
        DistanceMetric::Jaccard => "<->".to_string(),
    }
}

pub fn string_to_distance_metric(metric: &str) -> DistanceMetric {
    match metric.to_uppercase().as_str() {
        "COSINE" | "<=>" => DistanceMetric::Cosine,
        "L2" | "EUCLIDEAN" | "<->" => DistanceMetric::Euclidean,
        "DOT" | "DOTPRODUCT" | "IP" | "<#>" => DistanceMetric::DotProduct,
        "L1" | "MANHATTAN" | "<+>" => DistanceMetric::Manhattan,
        "HAMMING" => DistanceMetric::Hamming,
        "JACCARD" => DistanceMetric::Jaccard,
        _ => DistanceMetric::Cosine,
    }
}

pub fn table_info_to_export_collection_info(
    table_name: &str,
    columns: &[TableColumn],
    count: u64,
) -> Result<ExportCollectionInfo, VectorError> {
    let vector_column = columns
        .iter()
        .find(|col| {
            col.data_type.starts_with("vector")
                || col.data_type.to_lowercase() == "user-defined"
                || col.name.to_lowercase() == "embedding"
        })
        .ok_or_else(|| VectorError::ProviderError("No vector column found".to_string()))?;

    let dimension = if vector_column.data_type == "vector" {
        0
    } else if vector_column.data_type.starts_with("vector(")
        && vector_column.data_type.ends_with(')')
    {
        let dim_str = &vector_column.data_type[7..vector_column.data_type.len() - 1];
        dim_str.parse::<u32>().unwrap_or(0)
    } else {
        0
    };

    Ok(ExportCollectionInfo {
        name: table_name.to_string(),
        description: None,
        dimension,
        metric: DistanceMetric::Cosine,
        vector_count: count,
        size_bytes: None,
        index_ready: true,
        created_at: None,
        updated_at: None,
        provider_stats: None,
    })
}

pub fn vector_records_to_pgvector_data(
    vectors: &[VectorRecord],
) -> Result<Vec<PgVectorData>, VectorError> {
    let mut pg_vectors = Vec::new();

    for record in vectors {
        let embedding = match &record.vector {
            VectorData::Dense(dense) => dense.clone(),
            VectorData::Sparse(sparse) => {
                let mut dense = vec![0.0; sparse.total_dimensions as usize];
                for (idx, val) in sparse.indices.iter().zip(sparse.values.iter()) {
                    if (*idx as usize) < dense.len() {
                        dense[*idx as usize] = *val;
                    }
                }
                dense
            }
            VectorData::Binary(_) => {
                return Err(VectorError::UnsupportedFeature(
                    "Binary vectors not yet supported for pgvector".to_string(),
                ));
            }
            VectorData::Half(half) => half.data.clone(),
            VectorData::Named(_) => {
                return Err(VectorError::UnsupportedFeature(
                    "Named vectors not supported for pgvector".to_string(),
                ));
            }
            VectorData::Hybrid((dense, _sparse)) => dense.clone(),
        };

        let metadata = if let Some(meta) = &record.metadata {
            metadata_to_string_map(meta)?
        } else {
            HashMap::new()
        };

        pg_vectors.push(PgVectorData {
            id: record.id.clone(),
            embedding,
            metadata,
        });
    }

    Ok(pg_vectors)
}

pub fn create_table_request_from_collection_info(
    name: String,
    dimension: u32,
    metadata: Option<&golem_vector::golem::vector::types::Metadata>,
) -> CreateTableRequest {
    let mut metadata_columns = HashMap::new();

    if let Some(meta) = metadata {
        for (key, value) in meta {
            let column_type = match value {
                MetadataValue::StringVal(_) => "TEXT",
                MetadataValue::NumberVal(_) => "DOUBLE PRECISION",
                MetadataValue::IntegerVal(_) => "BIGINT",
                MetadataValue::BooleanVal(_) => "BOOLEAN",
                MetadataValue::DatetimeVal(_) => "TIMESTAMP",
                _ => "TEXT",
            };
            metadata_columns.insert(key.clone(), column_type.to_string());
        }
    }

    CreateTableRequest {
        table_name: name,
        dimension: if dimension > 0 { Some(dimension) } else { None },
        metadata_columns,
    }
}

pub fn create_search_request(
    table_name: &str,
    query: &SearchQuery,
    limit: u32,
    filter: Option<&FilterExpression>,
    output_fields: Option<&[String]>,
    distance_metric: &str,
) -> Result<PgSearchRequest, VectorError> {
    let query_vector = match query {
        SearchQuery::Vector(vector_data) => match vector_data {
            VectorData::Dense(dense) => dense.clone(),
            VectorData::Sparse(sparse) => {
                let mut dense = vec![0.0; sparse.total_dimensions as usize];
                for (idx, val) in sparse.indices.iter().zip(sparse.values.iter()) {
                    if (*idx as usize) < dense.len() {
                        dense[*idx as usize] = *val;
                    }
                }
                dense
            }
            VectorData::Half(half) => half.data.clone(),
            VectorData::Hybrid((dense, _sparse)) => dense.clone(),
            _ => {
                return Err(VectorError::UnsupportedFeature(
                    "Unsupported vector type for search".to_string(),
                ))
            }
        },
        SearchQuery::ById(_) => {
            return Err(VectorError::UnsupportedFeature(
                "Search by ID not supported, use get_vectors instead".to_string(),
            ));
        }
        SearchQuery::MultiVector(_) => {
            return Err(VectorError::UnsupportedFeature(
                "Multi-vector search not supported".to_string(),
            ));
        }
    };

    let filters = if let Some(filter) = filter {
        filter_expression_to_pg_filters(filter)?
    } else {
        HashMap::new()
    };

    let select_columns = if let Some(fields) = output_fields {
        fields.to_vec()
    } else {
        vec!["id".to_string(), "embedding".to_string()]
    };

    Ok(PgSearchRequest {
        table_name: table_name.to_string(),
        query_vector,
        distance_metric: distance_metric.to_string(),
        limit: limit as i32,
        filters,
        select_columns,
    })
}

pub fn pg_search_results_to_search_results(results: &[PgSearchResult]) -> Vec<SearchResult> {
    results
        .iter()
        .map(|result| {
            let vector_data = Some(VectorData::Dense(result.embedding.clone()));

            let metadata = if result.metadata.is_empty() {
                None
            } else {
                Some(string_map_to_metadata(&result.metadata))
            };

            SearchResult {
                id: result.id.clone(),
                score: 1.0 - result.distance,
                distance: result.distance,
                vector: vector_data,
                metadata,
            }
        })
        .collect()
}

pub fn pg_vector_results_to_vector_records(results: &[PgVectorResult]) -> Vec<VectorRecord> {
    results
        .iter()
        .map(|result| {
            let vector_data = VectorData::Dense(result.embedding.clone());

            let metadata = if result.metadata.is_empty() {
                None
            } else {
                Some(string_map_to_metadata(&result.metadata))
            };

            VectorRecord {
                id: result.id.clone(),
                vector: vector_data,
                metadata,
            }
        })
        .collect()
}

pub fn count_response_to_export_stats(
    count_response: &CountVectorsResponse,
    dimension: u32,
) -> ExportCollectionStats {
    ExportCollectionStats {
        vector_count: count_response.count,
        dimension,
        size_bytes: 0,
        index_size_bytes: None,
        namespace_stats: Vec::new(),
        distance_distribution: None,
    }
}

// helper functions

fn metadata_to_string_map(
    metadata: &golem_vector::golem::vector::types::Metadata,
) -> Result<HashMap<String, String>, VectorError> {
    let mut map = HashMap::new();

    for (key, value) in metadata {
        let string_value = match value {
            MetadataValue::StringVal(s) => s.clone(),
            MetadataValue::NumberVal(n) => n.to_string(),
            MetadataValue::IntegerVal(i) => i.to_string(),
            MetadataValue::BooleanVal(b) => b.to_string(),
            MetadataValue::DatetimeVal(dt) => dt.clone(),
            MetadataValue::NullVal => "null".to_string(),
            _ => {
                return Err(VectorError::UnsupportedFeature(format!(
                    "Unsupported metadata type for key: {}",
                    key
                )));
            }
        };
        map.insert(key.clone(), string_value);
    }

    Ok(map)
}

fn string_map_to_metadata(
    map: &HashMap<String, String>,
) -> golem_vector::golem::vector::types::Metadata {
    let mut metadata = Vec::new();

    for (key, value) in map {
        let metadata_value = if value == "null" {
            MetadataValue::NullVal
        } else if let Ok(b) = value.parse::<bool>() {
            MetadataValue::BooleanVal(b)
        } else if let Ok(i) = value.parse::<i64>() {
            MetadataValue::IntegerVal(i)
        } else if let Ok(f) = value.parse::<f64>() {
            MetadataValue::NumberVal(f)
        } else {
            MetadataValue::StringVal(value.clone())
        };

        metadata.push((key.clone(), metadata_value));
    }

    metadata
}

pub fn filter_expression_to_pg_filters(
    filter: &FilterExpression,
) -> Result<HashMap<String, String>, VectorError> {
    let sql_where = build_sql_where_clause(filter)?;

    let mut result = HashMap::new();
    result.insert("where_clause".to_string(), sql_where);
    Ok(result)
}

fn build_sql_where_clause(filter: &FilterExpression) -> Result<String, VectorError> {
    match filter {
        FilterExpression::Condition(condition) => build_condition_sql(condition),
        FilterExpression::And(filters) => {
            let clauses: Result<Vec<String>, VectorError> = filters
                .iter()
                .map(|f| build_sql_where_clause(f.get()))
                .collect();
            let clauses = clauses?;
            Ok(format!("({})", clauses.join(" AND ")))
        }
        FilterExpression::Or(filters) => {
            let clauses: Result<Vec<String>, VectorError> = filters
                .iter()
                .map(|f| build_sql_where_clause(f.get()))
                .collect();
            let clauses = clauses?;
            Ok(format!("({})", clauses.join(" OR ")))
        }
        FilterExpression::Not(filter_func) => {
            let inner_clause = build_sql_where_clause(filter_func.get())?;
            Ok(format!("NOT ({})", inner_clause))
        }
    }
}

fn build_condition_sql(condition: &FilterCondition) -> Result<String, VectorError> {
    let field = sanitize_field_name(&condition.field)?;
    let (operator_sql, value_sql) = match condition.operator {
        FilterOperator::Eq => ("=", format_sql_value(&condition.value)?),
        FilterOperator::Ne => ("!=", format_sql_value(&condition.value)?),
        FilterOperator::Gt => (">", format_sql_value(&condition.value)?),
        FilterOperator::Gte => (">=", format_sql_value(&condition.value)?),
        FilterOperator::Lt => ("<", format_sql_value(&condition.value)?),
        FilterOperator::Lte => ("<=", format_sql_value(&condition.value)?),
        FilterOperator::In => {
            return Err(VectorError::UnsupportedFeature(
                "IN operator requires array values, not yet implemented".to_string(),
            ));
        }
        FilterOperator::Nin => {
            return Err(VectorError::UnsupportedFeature(
                "NOT IN operator requires array values, not yet implemented".to_string(),
            ));
        }
        FilterOperator::Contains => {
            let value = format_sql_value(&condition.value)?;
            return Ok(format!("{} ILIKE '%' || {} || '%'", field, value));
        }
        FilterOperator::NotContains => {
            let value = format_sql_value(&condition.value)?;
            return Ok(format!("{} NOT ILIKE '%' || {} || '%'", field, value));
        }
        FilterOperator::Regex => {
            let value = format_sql_value(&condition.value)?;
            return Ok(format!("{} ~ {}", field, value));
        }
        FilterOperator::GeoWithin | FilterOperator::GeoBbox => {
            return Err(VectorError::UnsupportedFeature(
                "Geo operators not yet implemented for PostgreSQL".to_string(),
            ));
        }
    };

    Ok(format!("{} {} {}", field, operator_sql, value_sql))
}

fn sanitize_field_name(field: &str) -> Result<String, VectorError> {
    if field.chars().all(|c| c.is_alphanumeric() || c == '_') {
        Ok(format!("\"{}\"", field))
    } else {
        Err(VectorError::InvalidParams(format!(
            "Invalid field name: {}",
            field
        )))
    }
}

fn format_sql_value(value: &MetadataValue) -> Result<String, VectorError> {
    match value {
        MetadataValue::StringVal(s) => {
            let escaped = s.replace("'", "''");
            Ok(format!("'{}'", escaped))
        }
        MetadataValue::NumberVal(n) => Ok(n.to_string()),
        MetadataValue::IntegerVal(i) => Ok(i.to_string()),
        MetadataValue::BooleanVal(b) => Ok(b.to_string()),
        MetadataValue::NullVal => Ok("NULL".to_string()),
        MetadataValue::DatetimeVal(dt) => Ok(format!("'{}'::timestamp", dt)),
        _ => Err(VectorError::UnsupportedFeature(
            "Unsupported metadata type in filter condition".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_conversions() {
        let metadata = vec![
            (
                "string_field".to_string(),
                MetadataValue::StringVal("test".to_string()),
            ),
            ("number_field".to_string(), MetadataValue::NumberVal(42.5)),
            ("int_field".to_string(), MetadataValue::IntegerVal(100)),
            ("bool_field".to_string(), MetadataValue::BooleanVal(true)),
        ];

        let string_map = metadata_to_string_map(&metadata).unwrap();
        assert_eq!(string_map.get("string_field").unwrap(), "test");
        assert_eq!(string_map.get("number_field").unwrap(), "42.5");
        assert_eq!(string_map.get("int_field").unwrap(), "100");
        assert_eq!(string_map.get("bool_field").unwrap(), "true");

        let converted_back = string_map_to_metadata(&string_map);
        assert_eq!(converted_back.len(), 4);
    }

    #[test]
    fn test_vector_data_conversion() {
        let record = VectorRecord {
            id: "test-id".to_string(),
            vector: VectorData::Dense(vec![1.0, 2.0, 3.0]),
            metadata: Some(vec![(
                "field1".to_string(),
                MetadataValue::StringVal("value1".to_string()),
            )]),
        };

        let pg_vectors = vector_records_to_pgvector_data(&[record]).unwrap();
        assert_eq!(pg_vectors.len(), 1);
        assert_eq!(pg_vectors[0].id, "test-id");
        assert_eq!(pg_vectors[0].embedding, vec![1.0, 2.0, 3.0]);
        assert_eq!(pg_vectors[0].metadata.get("field1").unwrap(), "value1");
    }

    #[test]
    fn test_sql_filter_generation() {
        let condition = FilterCondition {
            field: "status".to_string(),
            operator: FilterOperator::Eq,
            value: MetadataValue::StringVal("active".to_string()),
        };

        let sql = build_condition_sql(&condition).unwrap();
        assert_eq!(sql, "\"status\" = 'active'");

        let condition = FilterCondition {
            field: "priority".to_string(),
            operator: FilterOperator::Gt,
            value: MetadataValue::IntegerVal(5),
        };

        let sql = build_condition_sql(&condition).unwrap();
        assert_eq!(sql, "\"priority\" > 5");

        let condition = FilterCondition {
            field: "description".to_string(),
            operator: FilterOperator::Contains,
            value: MetadataValue::StringVal("urgent".to_string()),
        };

        let sql = build_condition_sql(&condition).unwrap();
        assert_eq!(sql, "\"description\" ILIKE '%' || 'urgent' || '%'");
    }

    #[test]
    fn test_sql_injection_protection() {
        let condition = FilterCondition {
            field: "field'; DROP TABLE users; --".to_string(),
            operator: FilterOperator::Eq,
            value: MetadataValue::StringVal("value".to_string()),
        };

        let result = build_condition_sql(&condition);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid field name"));
    }

    #[test]
    fn test_sql_string_escaping() {
        let condition = FilterCondition {
            field: "name".to_string(),
            operator: FilterOperator::Eq,
            value: MetadataValue::StringVal("O'Reilly".to_string()),
        };

        let sql = build_condition_sql(&condition).unwrap();
        assert_eq!(sql, "\"name\" = 'O''Reilly'");
    }
}

use crate::client::{
    CollectionInfo, CollectionStats, DeleteRequest, GetRequest, InsertRequest, QueryRequest,
    SearchRequest, SearchResult as MilvusSearchResult, SparseFloatVector, UpsertRequest,
};
use golem_vector::golem::vector::search::SearchQuery;
use golem_vector::golem::vector::types::{
    BinaryVector, DistanceMetric, FilterCondition, FilterExpression, FilterOperator,
    GeoCoordinates, MetadataFunc, MetadataValue, SearchResult, SparseVector, VectorData,
    VectorError, VectorRecord,
};
use golem_vector::golem::vector::{
    analytics::CollectionStats as ExportCollectionStats,
    collections::CollectionInfo as ExportCollectionInfo,
};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub struct SearchRequestParams<'a> {
    pub collection_name: &'a str,
    pub db_name: &'a str,
    pub query: &'a SearchQuery,
    pub limit: u32,
    pub filter: Option<&'a FilterExpression>,
    pub output_fields: Option<&'a [String]>,
    pub anns_field: &'a str,
    pub metric_type: &'a str,
    pub partition_names: Option<Vec<String>>,
}

pub struct QueryRequestParams<'a> {
    pub collection_name: &'a str,
    pub db_name: &'a str,
    pub ids: Option<&'a [String]>,
    pub filter: Option<&'a FilterExpression>,
    pub output_fields: Option<&'a [String]>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub partition_names: Option<Vec<String>>,
}

pub fn distance_metric_to_string(metric: &DistanceMetric) -> String {
    match metric {
        DistanceMetric::Cosine => "COSINE".to_string(),
        DistanceMetric::Euclidean => "L2".to_string(),
        DistanceMetric::DotProduct => "IP".to_string(),
        DistanceMetric::Manhattan => "L1".to_string(),
        DistanceMetric::Hamming => "HAMMING".to_string(),
        DistanceMetric::Jaccard => "JACCARD".to_string(),
    }
}

pub fn string_to_distance_metric(metric: &str) -> DistanceMetric {
    match metric.to_uppercase().as_str() {
        "COSINE" => DistanceMetric::Cosine,
        "L2" => DistanceMetric::Euclidean,
        "IP" => DistanceMetric::DotProduct,
        "L1" => DistanceMetric::Manhattan,
        "HAMMING" => DistanceMetric::Hamming,
        "JACCARD" => DistanceMetric::Jaccard,
        _ => DistanceMetric::Cosine,
    }
}

pub fn collection_info_to_export_collection_info(
    info: &CollectionInfo,
) -> Result<ExportCollectionInfo, VectorError> {
    let vector_field = info
        .fields
        .iter()
        .find(|f| {
            f.data_type == "FloatVector"
                || f.data_type == "BinaryVector"
                || f.data_type == "SparseFloatVector"
        })
        .ok_or_else(|| VectorError::ProviderError("No vector field found".to_string()))?;

    let dimension = vector_field
        .element_type_params
        .as_ref()
        .and_then(|params_array| {
            params_array.iter().find_map(|params| {
                params
                    .get("key")
                    .and_then(|k| k.as_str())
                    .filter(|&k| k == "dim")
                    .and_then(|_| params.get("value"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<u32>().ok())
            })
        })
        .unwrap_or(0);

    let metric = info
        .indexes
        .first()
        .map(|idx| string_to_distance_metric(&idx.metric_type))
        .unwrap_or(DistanceMetric::Cosine);

    Ok(ExportCollectionInfo {
        name: info.collection_name.clone(),
        description: info.description.clone(),
        dimension,
        metric,
        vector_count: 0,
        size_bytes: None,
        index_ready: info.load == "Loaded",
        created_at: None,
        updated_at: None,
        provider_stats: None,
    })
}

pub fn vector_records_to_insert_request(
    collection_name: &str,
    db_name: &str,
    vectors: &[VectorRecord],
) -> Result<InsertRequest, VectorError> {
    let mut data = Vec::new();

    for record in vectors {
        let mut entity = Map::new();

        entity.insert("id".to_string(), json_value_from_id(&record.id));

        match &record.vector {
            VectorData::Dense(values) => {
                entity.insert(
                    "vector".to_string(),
                    Value::Array(
                        values
                            .iter()
                            .map(|&v| {
                                Value::Number(serde_json::Number::from_f64(v as f64).unwrap())
                            })
                            .collect(),
                    ),
                );
            }
            VectorData::Sparse(sparse) => {
                let sparse_obj = serde_json::json!({
                    "indices": sparse.indices,
                    "values": sparse.values
                });
                entity.insert("sparse_vector".to_string(), sparse_obj);
            }
            VectorData::Binary(binary) => {
                use base64::Engine;
                let encoded = base64::engine::general_purpose::STANDARD.encode(&binary.data);
                entity.insert("binary_vector".to_string(), Value::String(encoded));
            }
            VectorData::Half(half) => {
                entity.insert(
                    "vector".to_string(),
                    Value::Array(
                        half.data
                            .iter()
                            .map(|&v| {
                                Value::Number(serde_json::Number::from_f64(v as f64).unwrap())
                            })
                            .collect(),
                    ),
                );
            }
            VectorData::Named(_) => {
                return Err(VectorError::UnsupportedFeature(
                    "Named vectors not supported".to_string(),
                ));
            }
            VectorData::Hybrid((dense, sparse)) => {
                entity.insert(
                    "vector".to_string(),
                    Value::Array(
                        dense
                            .iter()
                            .map(|&v| {
                                Value::Number(serde_json::Number::from_f64(v as f64).unwrap())
                            })
                            .collect(),
                    ),
                );
                let sparse_obj = serde_json::json!({
                    "indices": sparse.indices,
                    "values": sparse.values
                });
                entity.insert("sparse_vector".to_string(), sparse_obj);
            }
        }

        if let Some(metadata) = &record.metadata {
            for (key, value) in metadata {
                entity.insert(key.clone(), metadata_value_to_json(value)?);
            }
        }

        data.push(Value::Object(entity));
    }

    Ok(InsertRequest {
        db_name: db_name.to_string(),
        collection_name: collection_name.to_string(),
        data,
    })
}

pub fn vector_records_to_upsert_request(
    collection_name: &str,
    db_name: &str,
    vectors: &[VectorRecord],
    partition_name: Option<&str>,
) -> Result<UpsertRequest, VectorError> {
    let insert_req = vector_records_to_insert_request(collection_name, db_name, vectors)?;

    Ok(UpsertRequest {
        db_name: insert_req.db_name,
        collection_name: insert_req.collection_name,
        data: insert_req.data,
        partition_name: partition_name.map(|s| s.to_string()),
    })
}

pub fn create_search_request(params: SearchRequestParams) -> Result<SearchRequest, VectorError> {
    let (dense_data, sparse_data, binary_data) = match params.query {
        SearchQuery::Vector(vector_data) => match vector_data {
            VectorData::Dense(values) => (Some(vec![values.clone()]), None, None),
            VectorData::Sparse(sparse) => {
                let sparse_vec = SparseFloatVector {
                    indices: sparse.indices.clone(),
                    values: sparse.values.clone(),
                };
                (None, Some(vec![sparse_vec]), None)
            }
            VectorData::Binary(binary) => (None, None, Some(vec![binary.data.clone()])),
            VectorData::Half(half) => (Some(vec![half.data.clone()]), None, None),
            VectorData::Hybrid((dense, sparse)) => {
                let sparse_vec = SparseFloatVector {
                    indices: sparse.indices.clone(),
                    values: sparse.values.clone(),
                };
                (Some(vec![dense.clone()]), Some(vec![sparse_vec]), None)
            }
            _ => {
                return Err(VectorError::UnsupportedFeature(
                    "Named vectors not supported for search".to_string(),
                ))
            }
        },
        SearchQuery::ById(_) => {
            return Err(VectorError::UnsupportedFeature(
                "Search by ID not directly supported".to_string(),
            ));
        }
        SearchQuery::MultiVector(_) => {
            return Err(VectorError::UnsupportedFeature(
                "Multi-vector search not supported".to_string(),
            ));
        }
    };

    let filter_expr = if let Some(filter) = params.filter {
        Some(filter_expression_to_milvus_expr(filter)?)
    } else {
        None
    };

    Ok(SearchRequest {
        db_name: params.db_name.to_string(),
        collection_name: params.collection_name.to_string(),
        data: dense_data,
        sparse_float_vectors: sparse_data,
        binary_vectors: binary_data,
        anns_field: params.anns_field.to_string(),
        metric_type: params.metric_type.to_string(),
        limit: params.limit,
        filter: filter_expr,
        output_fields: params.output_fields.map(|f| f.to_vec()),
        search_params: None,
        partition_names: params.partition_names,
    })
}

pub fn create_query_request(params: QueryRequestParams) -> Result<QueryRequest, VectorError> {
    let filter_expr = if let Some(filter) = params.filter {
        Some(filter_expression_to_milvus_expr(filter)?)
    } else {
        None
    };

    let ids_json = params
        .ids
        .map(|ids| ids.iter().map(|id| json_value_from_id(id)).collect());

    Ok(QueryRequest {
        db_name: params.db_name.to_string(),
        collection_name: params.collection_name.to_string(),
        filter: filter_expr,
        ids: ids_json,
        output_fields: params.output_fields.map(|f| f.to_vec()),
        limit: params.limit,
        offset: params.offset,
        partition_names: params.partition_names,
    })
}

pub fn create_get_request(
    collection_name: &str,
    _db_name: &str,
    ids: &[String],
    output_fields: Option<&[String]>,
) -> GetRequest {
    GetRequest {
        collection_name: collection_name.to_string(),
        id: ids.iter().map(|id| json_value_from_id(id)).collect(),
        output_fields: output_fields.map(|f| f.to_vec()),
    }
}

pub fn create_delete_request(
    collection_name: &str,
    _db_name: &str,
    ids: Option<&[String]>,
    filter: Option<&FilterExpression>,
    partition_name: Option<&str>,
) -> Result<DeleteRequest, VectorError> {
    let filter_expr = if let Some(filter) = filter {
        Some(filter_expression_to_milvus_expr(filter)?)
    } else {
        None
    };

    let ids_json = ids.map(|ids| ids.iter().map(|id| json_value_from_id(id)).collect());

    Ok(DeleteRequest {
        collection_name: collection_name.to_string(),
        id: ids_json,
        filter: filter_expr,
        partition_names: partition_name.map(|s| vec![s.to_string()]),
    })
}

pub fn milvus_search_results_to_search_results(
    results_value: &serde_json::Value,
) -> Result<Vec<SearchResult>, VectorError> {
    if let Ok(nested_results) =
        serde_json::from_value::<Vec<Vec<MilvusSearchResult>>>(results_value.clone())
    {
        if nested_results.is_empty() {
            return Ok(Vec::new());
        }
        return convert_results_array(&nested_results[0]);
    }

    if let Ok(flat_results) =
        serde_json::from_value::<Vec<MilvusSearchResult>>(results_value.clone())
    {
        return convert_results_array(&flat_results);
    }

    Ok(Vec::new())
}

fn convert_results_array(results: &[MilvusSearchResult]) -> Result<Vec<SearchResult>, VectorError> {
    let mut search_results = Vec::new();

    for result in results {
        let vector_data = if let Some(entity) = &result.entity {
            Some(entity_to_vector_data(entity)?)
        } else {
            None
        };

        let metadata = if let Some(entity) = &result.entity {
            let mut meta = Vec::new();
            for (key, value) in entity {
                if key != "vector"
                    && key != "id"
                    && key != "sparse_vector"
                    && key != "binary_vector"
                {
                    meta.push((key.clone(), json_to_metadata_value(value)?));
                }
            }
            if meta.is_empty() {
                None
            } else {
                Some(meta)
            }
        } else {
            None
        };

        search_results.push(SearchResult {
            id: json_to_id(&result.id)?,
            score: 1.0 - result.distance,
            distance: result.distance,
            vector: vector_data,
            metadata,
        });
    }

    Ok(search_results)
}

pub fn milvus_entities_to_vector_records(
    entities: &[HashMap<String, Value>],
) -> Result<Vec<VectorRecord>, VectorError> {
    let mut records = Vec::new();

    for entity in entities {
        let id = entity
            .get("id")
            .ok_or_else(|| VectorError::ProviderError("Missing id field".to_string()))?;

        let vector_data = entity_to_vector_data(entity)?;

        let mut metadata = Vec::new();
        for (key, value) in entity {
            if key != "vector" && key != "id" && key != "sparse_vector" && key != "binary_vector" {
                metadata.push((key.clone(), json_to_metadata_value(value)?));
            }
        }

        records.push(VectorRecord {
            id: json_to_id(id)?,
            vector: vector_data,
            metadata: if metadata.is_empty() {
                None
            } else {
                Some(metadata)
            },
        });
    }

    Ok(records)
}

pub fn collection_stats_to_export_stats(stats: &CollectionStats) -> ExportCollectionStats {
    ExportCollectionStats {
        vector_count: stats.row_count,
        dimension: 0,
        size_bytes: 0,
        index_size_bytes: None,
        namespace_stats: Vec::new(),
        distance_distribution: None,
    }
}

// helper functions for conversions

fn json_value_from_id(id: &str) -> Value {
    if let Ok(num) = id.parse::<i64>() {
        Value::Number(serde_json::Number::from(num))
    } else {
        Value::String(id.to_string())
    }
}

fn json_to_id(value: &Value) -> Result<String, VectorError> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        _ => Err(VectorError::ProviderError(format!(
            "Invalid ID type: {:?}",
            value
        ))),
    }
}

fn metadata_value_to_json(value: &MetadataValue) -> Result<Value, VectorError> {
    match value {
        MetadataValue::StringVal(s) => Ok(Value::String(s.clone())),
        MetadataValue::NumberVal(n) => Ok(Value::Number(
            serde_json::Number::from_f64(*n)
                .ok_or_else(|| VectorError::InvalidParams("Invalid number".to_string()))?,
        )),
        MetadataValue::IntegerVal(i) => Ok(Value::Number(serde_json::Number::from(*i))),
        MetadataValue::BooleanVal(b) => Ok(Value::Bool(*b)),
        MetadataValue::ArrayVal(arr) => {
            let mut json_arr = Vec::new();
            for item in arr {
                json_arr.push(metadata_value_to_json(item.get())?);
            }
            Ok(Value::Array(json_arr))
        }
        MetadataValue::ObjectVal(obj) => {
            let mut json_obj = Map::new();
            for (key, value) in obj {
                json_obj.insert(key.clone(), metadata_value_to_json(value.get())?);
            }
            Ok(Value::Object(json_obj))
        }
        MetadataValue::NullVal => Ok(Value::Null),
        MetadataValue::GeoVal(geo) => {
            let mut geo_obj = Map::new();
            geo_obj.insert(
                "latitude".to_string(),
                Value::Number(
                    serde_json::Number::from_f64(geo.latitude).ok_or_else(|| {
                        VectorError::InvalidParams("Invalid latitude".to_string())
                    })?,
                ),
            );
            geo_obj.insert(
                "longitude".to_string(),
                Value::Number(
                    serde_json::Number::from_f64(geo.longitude).ok_or_else(|| {
                        VectorError::InvalidParams("Invalid longitude".to_string())
                    })?,
                ),
            );
            Ok(Value::Object(geo_obj))
        }
        MetadataValue::DatetimeVal(dt) => Ok(Value::String(dt.clone())),
        MetadataValue::BlobVal(blob) => {
            use base64::Engine;
            Ok(Value::String(
                base64::engine::general_purpose::STANDARD.encode(blob),
            ))
        }
    }
}

fn json_to_metadata_value(value: &Value) -> Result<MetadataValue, VectorError> {
    match value {
        Value::String(s) => Ok(MetadataValue::StringVal(s.clone())),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(MetadataValue::IntegerVal(i))
            } else if let Some(f) = n.as_f64() {
                Ok(MetadataValue::NumberVal(f))
            } else {
                Err(VectorError::ProviderError(
                    "Invalid number format".to_string(),
                ))
            }
        }
        Value::Bool(b) => Ok(MetadataValue::BooleanVal(*b)),
        Value::Array(arr) => {
            let mut metadata_arr = Vec::new();
            for item in arr {
                let metadata_val = json_to_metadata_value(item)?;
                metadata_arr.push(MetadataFunc::new(metadata_val));
            }
            Ok(MetadataValue::ArrayVal(metadata_arr))
        }
        Value::Object(obj) => {
            if obj.contains_key("latitude") && obj.contains_key("longitude") {
                let lat = obj
                    .get("latitude")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| {
                        VectorError::ProviderError("Invalid latitude in geo object".to_string())
                    })?;
                let lon = obj
                    .get("longitude")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| {
                        VectorError::ProviderError("Invalid longitude in geo object".to_string())
                    })?;

                return Ok(MetadataValue::GeoVal(GeoCoordinates {
                    latitude: lat,
                    longitude: lon,
                }));
            }

            let mut metadata_obj = Vec::new();
            for (key, value) in obj {
                let metadata_val = json_to_metadata_value(value)?;
                metadata_obj.push((key.clone(), MetadataFunc::new(metadata_val)));
            }
            Ok(MetadataValue::ObjectVal(metadata_obj))
        }
        Value::Null => Ok(MetadataValue::NullVal),
    }
}

fn json_to_vector_data(value: &Value) -> Result<VectorData, VectorError> {
    match value {
        Value::Array(arr) => {
            let mut vector = Vec::new();
            for item in arr {
                if let Some(f) = item.as_f64() {
                    vector.push(f as f32);
                } else {
                    return Err(VectorError::ProviderError(
                        "Invalid vector value".to_string(),
                    ));
                }
            }
            Ok(VectorData::Dense(vector))
        }
        _ => Err(VectorError::ProviderError(
            "Invalid vector format".to_string(),
        )),
    }
}

fn entity_to_vector_data(entity: &HashMap<String, Value>) -> Result<VectorData, VectorError> {
    if let Some(sparse_val) = entity.get("sparse_vector") {
        return json_to_sparse_vector_data(sparse_val);
    }

    if let Some(binary_val) = entity.get("binary_vector") {
        return json_to_binary_vector_data(binary_val);
    }

    if let Some(vector_val) = entity.get("vector") {
        return json_to_vector_data(vector_val);
    }

    Ok(VectorData::Dense(Vec::new()))
}

fn json_to_sparse_vector_data(value: &Value) -> Result<VectorData, VectorError> {
    match value {
        Value::Object(obj) => {
            let indices = obj
                .get("indices")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    VectorError::ProviderError(
                        "Missing or invalid indices in sparse vector".to_string(),
                    )
                })?
                .iter()
                .map(|v| v.as_u64().map(|n| n as u32))
                .collect::<Option<Vec<u32>>>()
                .ok_or_else(|| {
                    VectorError::ProviderError(
                        "Invalid indices format in sparse vector".to_string(),
                    )
                })?;

            let values = obj
                .get("values")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    VectorError::ProviderError(
                        "Missing or invalid values in sparse vector".to_string(),
                    )
                })?
                .iter()
                .map(|v| v.as_f64().map(|f| f as f32))
                .collect::<Option<Vec<f32>>>()
                .ok_or_else(|| {
                    VectorError::ProviderError("Invalid values format in sparse vector".to_string())
                })?;

            let max_dim = indices.iter().max().copied().unwrap_or(0) + 1;
            Ok(VectorData::Sparse(SparseVector {
                indices,
                values,
                total_dimensions: max_dim,
            }))
        }
        _ => Err(VectorError::ProviderError(
            "Invalid sparse vector format".to_string(),
        )),
    }
}

fn json_to_binary_vector_data(value: &Value) -> Result<VectorData, VectorError> {
    match value {
        Value::String(encoded) => {
            use base64::Engine;
            let data = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|e| {
                    VectorError::ProviderError(format!("Failed to decode binary vector: {}", e))
                })?;

            let dimensions = (data.len() * 8) as u32;
            Ok(VectorData::Binary(BinaryVector { data, dimensions }))
        }
        _ => Err(VectorError::ProviderError(
            "Invalid binary vector format".to_string(),
        )),
    }
}

fn filter_expression_to_milvus_expr(filter: &FilterExpression) -> Result<String, VectorError> {
    match filter {
        FilterExpression::Condition(condition) => filter_condition_to_milvus_expr(condition),
        FilterExpression::And(expressions) => {
            let mut expr_parts = Vec::new();
            for expr in expressions {
                expr_parts.push(format!(
                    "({})",
                    filter_expression_to_milvus_expr(expr.get())?
                ));
            }
            Ok(format!("({})", expr_parts.join(" && ")))
        }
        FilterExpression::Or(expressions) => {
            let mut expr_parts = Vec::new();
            for expr in expressions {
                expr_parts.push(format!(
                    "({})",
                    filter_expression_to_milvus_expr(expr.get())?
                ));
            }
            Ok(format!("({})", expr_parts.join(" || ")))
        }
        FilterExpression::Not(expression) => Ok(format!(
            "!({})",
            filter_expression_to_milvus_expr(expression.get())?
        )),
    }
}

fn filter_condition_to_milvus_expr(condition: &FilterCondition) -> Result<String, VectorError> {
    let field = &condition.field;
    let value_str = metadata_value_to_filter_value(&condition.value)?;

    let expr = match condition.operator {
        FilterOperator::Eq => format!("{} == {}", field, value_str),
        FilterOperator::Ne => format!("{} != {}", field, value_str),
        FilterOperator::Gt => format!("{} > {}", field, value_str),
        FilterOperator::Gte => format!("{} >= {}", field, value_str),
        FilterOperator::Lt => format!("{} < {}", field, value_str),
        FilterOperator::Lte => format!("{} <= {}", field, value_str),
        FilterOperator::In => {
            if let MetadataValue::ArrayVal(arr) = &condition.value {
                let values: Result<Vec<String>, _> = arr
                    .iter()
                    .map(|v| metadata_value_to_filter_value(v.get()))
                    .collect();
                format!("{} in [{}]", field, values?.join(", "))
            } else {
                return Err(VectorError::InvalidParams(
                    "IN operator requires array value".to_string(),
                ));
            }
        }
        FilterOperator::Nin => {
            if let MetadataValue::ArrayVal(arr) = &condition.value {
                let values: Result<Vec<String>, _> = arr
                    .iter()
                    .map(|v| metadata_value_to_filter_value(v.get()))
                    .collect();
                format!("{} not in [{}]", field, values?.join(", "))
            } else {
                return Err(VectorError::InvalidParams(
                    "NIN operator requires array value".to_string(),
                ));
            }
        }
        FilterOperator::Contains => {
            return Err(VectorError::UnsupportedFeature(
                "Contains operator not supported in Milvus".to_string(),
            ));
        }
        FilterOperator::NotContains => {
            return Err(VectorError::UnsupportedFeature(
                "NotContains operator not supported in Milvus".to_string(),
            ));
        }
        FilterOperator::Regex => {
            return Err(VectorError::UnsupportedFeature(
                "Regex operator not supported in Milvus".to_string(),
            ));
        }
        FilterOperator::GeoWithin => {
            return Err(VectorError::UnsupportedFeature(
                "GeoWithin operator not supported in Milvus".to_string(),
            ));
        }
        FilterOperator::GeoBbox => {
            return Err(VectorError::UnsupportedFeature(
                "GeoBbox operator not supported in Milvus".to_string(),
            ));
        }
    };

    Ok(expr)
}

fn metadata_value_to_filter_value(value: &MetadataValue) -> Result<String, VectorError> {
    match value {
        MetadataValue::StringVal(s) => Ok(format!("\"{}\"", s)),
        MetadataValue::NumberVal(n) => Ok(n.to_string()),
        MetadataValue::IntegerVal(i) => Ok(i.to_string()),
        MetadataValue::BooleanVal(b) => Ok(b.to_string()),
        _ => Err(VectorError::InvalidParams(
            "Unsupported metadata value type for filtering".to_string(),
        )),
    }
}

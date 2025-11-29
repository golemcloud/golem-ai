use crate::client::{
    BatchSearchRequest, CollectionConfig, CollectionInfo as QdrantCollectionInfo, Condition,
    ContextPair as QdrantContextPair, CountRequest, DeletePointsRequest, DiscoverRequest, Distance,
    FieldCondition, FieldConditionOneOf, Filter, GetPointsRequest, MatchCondition, MatchValue,
    NamedVectorStruct, NamedVectors, PointId, PointStruct, RangeInterface, RecommendExample,
    RecommendRequest, Record, ScoredPoint, ScrollRequest, SearchRequest, UpsertRequest,
    VectorConfig, VectorParams, WithPayloadSelector, WithVectorSelector,
};
use golem_vector::golem::vector::search::SearchQuery;
use golem_vector::golem::vector::types::{
    DistanceMetric, FilterCondition, FilterExpression, FilterOperator, GeoCoordinates, Id,
    MetadataValue, SearchResult, VectorData, VectorError, VectorRecord,
};
use golem_vector::golem::vector::{
    collections::CollectionInfo as ExportCollectionInfo,
    search_extended::{ContextPair, RecommendationExample},
};
use serde_json::{Map, Value};

pub fn distance_metric_to_qdrant_distance(metric: &DistanceMetric) -> Distance {
    match metric {
        DistanceMetric::Cosine => Distance::Cosine,
        DistanceMetric::Euclidean => Distance::Euclid,
        DistanceMetric::DotProduct => Distance::Dot,
        DistanceMetric::Manhattan => Distance::Manhattan,
        _ => Distance::Cosine,
    }
}

pub fn qdrant_distance_to_distance_metric(distance: &Distance) -> DistanceMetric {
    match distance {
        Distance::Cosine => DistanceMetric::Cosine,
        Distance::Euclid => DistanceMetric::Euclidean,
        Distance::Dot => DistanceMetric::DotProduct,
        Distance::Manhattan => DistanceMetric::Manhattan,
    }
}

pub fn collection_info_to_export_collection_info(
    name: &str,
    info: &QdrantCollectionInfo,
) -> Result<ExportCollectionInfo, VectorError> {
    let (dimension, metric) = if let Some(params) = &info.config.params {
        if let Some(vectors) = &params.vectors {
            match vectors {
                VectorConfig::Single(params) => (
                    params.size,
                    qdrant_distance_to_distance_metric(&params.distance),
                ),
                VectorConfig::Multiple(vectors) => {
                    if let Some((_, params)) = vectors.iter().next() {
                        (
                            params.size,
                            qdrant_distance_to_distance_metric(&params.distance),
                        )
                    } else {
                        return Err(VectorError::ProviderError(
                            "No vector configurations found".to_string(),
                        ));
                    }
                }
            }
        } else {
            return Err(VectorError::ProviderError(
                "No vectors configuration found in params".to_string(),
            ));
        }
    } else if let Some(vectors) = &info.config.vectors {
        match vectors {
            VectorConfig::Single(params) => (
                params.size,
                qdrant_distance_to_distance_metric(&params.distance),
            ),
            VectorConfig::Multiple(vectors) => {
                if let Some((_, params)) = vectors.iter().next() {
                    (
                        params.size,
                        qdrant_distance_to_distance_metric(&params.distance),
                    )
                } else {
                    return Err(VectorError::ProviderError(
                        "No vector configurations found".to_string(),
                    ));
                }
            }
        }
    } else {
        return Err(VectorError::ProviderError(
            "No vector configuration found".to_string(),
        ));
    };

    let vector_count = info.points_count.unwrap_or(0);
    let index_ready = info.status == "green";

    Ok(ExportCollectionInfo {
        name: name.to_string(),
        description: None,
        dimension,
        metric,
        vector_count,
        size_bytes: None,
        index_ready,
        created_at: None,
        updated_at: None,
        provider_stats: None,
    })
}

pub fn create_collection_config(dimension: u32, metric: DistanceMetric) -> CollectionConfig {
    let vector_params = VectorParams {
        size: dimension,
        distance: distance_metric_to_qdrant_distance(&metric),
        hnsw_config: None,
        quantization_config: None,
        on_disk: None,
    };

    CollectionConfig {
        params: None,
        hnsw_config: None,
        wal_config: None,
        optimizer_config: None,
        quantization_config: None,
        vectors: Some(VectorConfig::Single(vector_params)),
        shard_number: None,
        on_disk_payload: None,
    }
}

pub fn vector_records_to_upsert_request(
    vectors: &[VectorRecord],
) -> Result<UpsertRequest, VectorError> {
    let mut points = Vec::new();

    for record in vectors {
        let id = id_to_point_id(&record.id)?;

        let vector = match &record.vector {
            VectorData::Dense(values) => NamedVectors::Single(values.clone()),
            VectorData::Sparse(_) => {
                return Err(VectorError::ProviderError(
                    "Sparse vectors not yet supported for Qdrant".to_string(),
                ));
            }
            VectorData::Binary(_) => {
                return Err(VectorError::ProviderError(
                    "Binary vectors not yet supported for Qdrant".to_string(),
                ));
            }
            VectorData::Half(_) => {
                return Err(VectorError::ProviderError(
                    "Half vectors not yet supported for Qdrant".to_string(),
                ));
            }
            VectorData::Named(_) => {
                return Err(VectorError::ProviderError(
                    "Named vectors not yet supported for Qdrant".to_string(),
                ));
            }
            VectorData::Hybrid(_) => {
                return Err(VectorError::ProviderError(
                    "Hybrid vectors not yet supported for Qdrant".to_string(),
                ));
            }
        };

        let payload = if let Some(metadata) = &record.metadata {
            Some(metadata_to_json_value(metadata)?)
        } else {
            None
        };

        points.push(PointStruct {
            id,
            payload,
            vector,
        });
    }

    Ok(UpsertRequest {
        points,
        ordering: None,
    })
}

pub fn create_search_request(
    query: &SearchQuery,
    limit: u32,
    offset: Option<u32>,
    filter: Option<&FilterExpression>,
    with_payload: bool,
    with_vector: bool,
    score_threshold: Option<f64>,
) -> Result<SearchRequest, VectorError> {
    let vector = match query {
        SearchQuery::Vector(vector_data) => match vector_data {
            VectorData::Dense(values) => NamedVectorStruct::Default(values.clone()),
            VectorData::Sparse(_) => {
                return Err(VectorError::ProviderError(
                    "Sparse vector search not yet supported for Qdrant".to_string(),
                ));
            }
            VectorData::Binary(_) => {
                return Err(VectorError::ProviderError(
                    "Binary vector search not yet supported for Qdrant".to_string(),
                ));
            }
            VectorData::Half(_) => {
                return Err(VectorError::ProviderError(
                    "Half vector search not yet supported for Qdrant".to_string(),
                ));
            }
            VectorData::Named(_) => {
                return Err(VectorError::ProviderError(
                    "Named vector search not yet supported for Qdrant".to_string(),
                ));
            }
            VectorData::Hybrid(_) => {
                return Err(VectorError::ProviderError(
                    "Hybrid vector search not yet supported for Qdrant".to_string(),
                ));
            }
        },
        SearchQuery::ById(_) => {
            return Err(VectorError::ProviderError(
                "Search by ID not supported in this context".to_string(),
            ));
        }
        SearchQuery::MultiVector(_) => {
            return Err(VectorError::ProviderError(
                "Multi-vector search not yet supported for Qdrant".to_string(),
            ));
        }
    };

    let qdrant_filter = if let Some(filter) = filter {
        Some(filter_expression_to_qdrant_filter(filter)?)
    } else {
        None
    };

    let with_payload_selector = if with_payload {
        Some(WithPayloadSelector::Bool(true))
    } else {
        Some(WithPayloadSelector::Bool(false))
    };

    let with_vector_selector = if with_vector {
        Some(WithVectorSelector::Bool(true))
    } else {
        Some(WithVectorSelector::Bool(false))
    };

    Ok(SearchRequest {
        vector,
        filter: qdrant_filter,
        params: None,
        limit,
        offset,
        with_payload: with_payload_selector,
        with_vector: with_vector_selector,
        score_threshold,
    })
}

pub fn create_get_points_request(
    ids: &[Id],
    with_payload: bool,
    with_vector: bool,
) -> Result<GetPointsRequest, VectorError> {
    let point_ids: Result<Vec<PointId>, VectorError> = ids.iter().map(id_to_point_id).collect();

    let with_payload_selector = if with_payload {
        Some(WithPayloadSelector::Bool(true))
    } else {
        Some(WithPayloadSelector::Bool(false))
    };

    let with_vector_selector = if with_vector {
        Some(WithVectorSelector::Bool(true))
    } else {
        Some(WithVectorSelector::Bool(false))
    };

    Ok(GetPointsRequest {
        ids: point_ids?,
        with_payload: with_payload_selector,
        with_vector: with_vector_selector,
    })
}

pub fn create_delete_points_request(
    ids: Option<&[Id]>,
    filter: Option<&FilterExpression>,
) -> Result<DeletePointsRequest, VectorError> {
    let point_ids = if let Some(ids) = ids {
        let converted_ids: Result<Vec<PointId>, VectorError> =
            ids.iter().map(id_to_point_id).collect();
        Some(converted_ids?)
    } else {
        None
    };

    let qdrant_filter = if let Some(filter) = filter {
        Some(filter_expression_to_qdrant_filter(filter)?)
    } else {
        None
    };

    Ok(DeletePointsRequest {
        points: point_ids,
        filter: qdrant_filter,
        ordering: None,
    })
}

pub fn create_scroll_request(
    filter: Option<&FilterExpression>,
    limit: Option<u32>,
    offset: Option<&Id>,
    with_payload: bool,
    with_vector: bool,
) -> Result<ScrollRequest, VectorError> {
    let qdrant_filter = if let Some(filter) = filter {
        Some(filter_expression_to_qdrant_filter(filter)?)
    } else {
        None
    };

    let offset_id = if let Some(offset_id) = offset {
        Some(id_to_point_id(offset_id)?)
    } else {
        None
    };

    let with_payload_selector = if with_payload {
        Some(WithPayloadSelector::Bool(true))
    } else {
        Some(WithPayloadSelector::Bool(false))
    };

    let with_vector_selector = if with_vector {
        Some(WithVectorSelector::Bool(true))
    } else {
        Some(WithVectorSelector::Bool(false))
    };

    Ok(ScrollRequest {
        filter: qdrant_filter,
        limit,
        offset: offset_id,
        with_payload: with_payload_selector,
        with_vector: with_vector_selector,
        order_by: None,
    })
}

pub fn create_count_request(
    filter: Option<&FilterExpression>,
) -> Result<CountRequest, VectorError> {
    let qdrant_filter = if let Some(filter) = filter {
        Some(filter_expression_to_qdrant_filter(filter)?)
    } else {
        None
    };

    Ok(CountRequest {
        filter: qdrant_filter,
        exact: Some(true),
    })
}

pub fn create_batch_search_request(
    queries: &[SearchQuery],
    limit: u32,
    filter: Option<&FilterExpression>,
    with_payload: bool,
    with_vector: bool,
) -> Result<BatchSearchRequest, VectorError> {
    let mut searches = Vec::new();

    for query in queries {
        let search_request =
            create_search_request(query, limit, None, filter, with_payload, with_vector, None)?;
        searches.push(search_request);
    }

    Ok(BatchSearchRequest { searches })
}

pub fn create_recommend_request(
    positive: &[RecommendationExample],
    negative: Option<&[RecommendationExample]>,
    limit: u32,
    filter: Option<&FilterExpression>,
    with_payload: bool,
    with_vector: bool,
) -> Result<RecommendRequest, VectorError> {
    let positive_examples: Result<Vec<RecommendExample>, VectorError> = positive
        .iter()
        .map(recommendation_example_to_qdrant)
        .collect();

    let negative_examples = if let Some(negative) = negative {
        let neg_examples: Result<Vec<RecommendExample>, VectorError> = negative
            .iter()
            .map(recommendation_example_to_qdrant)
            .collect();
        Some(neg_examples?)
    } else {
        None
    };

    let qdrant_filter = if let Some(filter) = filter {
        Some(filter_expression_to_qdrant_filter(filter)?)
    } else {
        None
    };

    let with_payload_selector = if with_payload {
        Some(WithPayloadSelector::Bool(true))
    } else {
        Some(WithPayloadSelector::Bool(false))
    };

    let with_vector_selector = if with_vector {
        Some(WithVectorSelector::Bool(true))
    } else {
        Some(WithVectorSelector::Bool(false))
    };

    Ok(RecommendRequest {
        positive: positive_examples?,
        negative: negative_examples,
        filter: qdrant_filter,
        params: None,
        limit,
        offset: None,
        with_payload: with_payload_selector,
        with_vector: with_vector_selector,
        score_threshold: None,
        using: None,
        lookup_from: None,
    })
}

pub fn create_discover_request(
    target: Option<&RecommendationExample>,
    context: &[ContextPair],
    limit: u32,
    filter: Option<&FilterExpression>,
    with_payload: bool,
    with_vector: bool,
) -> Result<DiscoverRequest, VectorError> {
    let target_id = if let Some(target) = target {
        match target {
            RecommendationExample::VectorId(id) => id_to_point_id(id)?,
            RecommendationExample::VectorData(_) => {
                return Err(VectorError::InvalidParams(
                    "Cannot use vector data as discovery target, only vector IDs are supported"
                        .to_string(),
                ));
            }
        }
    } else if let Some(first_pair) = context.first() {
        match &first_pair.positive {
            RecommendationExample::VectorId(id) => id_to_point_id(id)?,
            RecommendationExample::VectorData(_) => {
                return Err(VectorError::InvalidParams(
                    "Cannot use vector data as discovery target, only vector IDs are supported"
                        .to_string(),
                ));
            }
        }
    } else {
        return Err(VectorError::InvalidParams(
            "Discovery requires either a target ID or at least one context pair".to_string(),
        ));
    };

    let context_pairs: Result<Vec<QdrantContextPair>, VectorError> = context
        .iter()
        .map(|pair| {
            let positive_id = match &pair.positive {
                RecommendationExample::VectorId(id) => id_to_point_id(id)?,
                RecommendationExample::VectorData(_) => {
                    return Err(VectorError::InvalidParams(
                        "Context pairs must use vector IDs, not vector data".to_string(),
                    ));
                }
            };
            let negative_id = match &pair.negative {
                RecommendationExample::VectorId(id) => id_to_point_id(id)?,
                RecommendationExample::VectorData(_) => {
                    return Err(VectorError::InvalidParams(
                        "Context pairs must use vector IDs, not vector data".to_string(),
                    ));
                }
            };
            Ok(QdrantContextPair {
                positive: positive_id,
                negative: negative_id,
            })
        })
        .collect();

    let qdrant_filter = if let Some(filter) = filter {
        Some(filter_expression_to_qdrant_filter(filter)?)
    } else {
        None
    };

    let with_payload_selector = if with_payload {
        Some(WithPayloadSelector::Bool(true))
    } else {
        Some(WithPayloadSelector::Bool(false))
    };

    let with_vector_selector = if with_vector {
        Some(WithVectorSelector::Bool(true))
    } else {
        Some(WithVectorSelector::Bool(false))
    };

    Ok(DiscoverRequest {
        target: target_id,
        context: context_pairs?,
        filter: qdrant_filter,
        params: None,
        limit,
        offset: None,
        with_payload: with_payload_selector,
        with_vector: with_vector_selector,
        using: None,
        lookup_from: None,
    })
}

pub fn scored_points_to_search_results(
    scored_points: &[ScoredPoint],
) -> Result<Vec<SearchResult>, VectorError> {
    let mut results = Vec::new();

    for point in scored_points {
        let id = point_id_to_id(&point.id)?;

        let vector = if let Some(vectors) = &point.vector {
            Some(named_vectors_to_vector_data(vectors)?)
        } else {
            None
        };

        let metadata = if let Some(payload) = &point.payload {
            Some(json_value_to_metadata(payload)?)
        } else {
            None
        };

        results.push(SearchResult {
            id,
            score: point.score as f32,
            distance: 1.0 - point.score as f32,
            vector,
            metadata,
        });
    }

    Ok(results)
}

pub fn records_to_vector_records(records: &[Record]) -> Result<Vec<VectorRecord>, VectorError> {
    let mut vector_records = Vec::new();

    for record in records {
        let id = point_id_to_id(&record.id)?;

        let vector = if let Some(vectors) = &record.vector {
            named_vectors_to_vector_data(vectors)?
        } else {
            VectorData::Dense(Vec::new())
        };

        let metadata = if let Some(payload) = &record.payload {
            Some(json_value_to_metadata(payload)?)
        } else {
            None
        };

        vector_records.push(VectorRecord {
            id,
            vector,
            metadata,
        });
    }

    Ok(vector_records)
}

// Helper functions

fn id_to_point_id(id: &Id) -> Result<PointId, VectorError> {
    if let Ok(num) = id.parse::<u64>() {
        Ok(PointId::Integer(num))
    } else {
        Ok(PointId::Uuid(id.clone()))
    }
}

fn point_id_to_id(point_id: &PointId) -> Result<Id, VectorError> {
    match point_id {
        PointId::Integer(i) => Ok((*i as i64).to_string()),
        PointId::Uuid(s) => Ok(s.clone()),
    }
}

fn named_vectors_to_vector_data(vectors: &NamedVectors) -> Result<VectorData, VectorError> {
    match vectors {
        NamedVectors::Single(values) => Ok(VectorData::Dense(values.clone())),
        NamedVectors::Multiple(map) => {
            if let Some((_, values)) = map.iter().next() {
                Ok(VectorData::Dense(values.clone()))
            } else {
                Ok(VectorData::Dense(Vec::new()))
            }
        }
    }
}

fn metadata_to_json_value(metadata: &[(String, MetadataValue)]) -> Result<Value, VectorError> {
    let mut map = Map::new();

    for (key, value) in metadata {
        let json_value = metadata_value_to_json_value(value)?;
        map.insert(key.clone(), json_value);
    }

    Ok(Value::Object(map))
}

fn json_value_to_metadata(value: &Value) -> Result<Vec<(String, MetadataValue)>, VectorError> {
    let mut metadata = Vec::new();

    if let Value::Object(map) = value {
        for (key, val) in map {
            let metadata_value = json_value_to_metadata_value(val)?;
            metadata.push((key.clone(), metadata_value));
        }
    }

    Ok(metadata)
}

fn metadata_value_to_json_value(value: &MetadataValue) -> Result<Value, VectorError> {
    match value {
        MetadataValue::StringVal(s) => Ok(Value::String(s.clone())),
        MetadataValue::IntegerVal(i) => Ok(Value::Number(serde_json::Number::from(*i))),
        MetadataValue::NumberVal(f) => serde_json::Number::from_f64(*f)
            .map(Value::Number)
            .ok_or_else(|| VectorError::InvalidParams("Invalid float value".to_string())),
        MetadataValue::BooleanVal(b) => Ok(Value::Bool(*b)),
        MetadataValue::ArrayVal(arr) => {
            let json_arr: Result<Vec<Value>, VectorError> = arr
                .iter()
                .map(|func| metadata_value_to_json_value(func.get()))
                .collect();
            Ok(Value::Array(json_arr?))
        }
        MetadataValue::ObjectVal(obj) => {
            let mut map = Map::new();
            for (key, func) in obj {
                map.insert(key.clone(), metadata_value_to_json_value(func.get())?);
            }
            Ok(Value::Object(map))
        }
        MetadataValue::NullVal => Ok(Value::Null),
        MetadataValue::GeoVal(geo) => {
            let mut geo_obj = Map::new();
            geo_obj.insert(
                "lat".to_string(),
                Value::Number(serde_json::Number::from_f64(geo.latitude).unwrap()),
            );
            geo_obj.insert(
                "lon".to_string(),
                Value::Number(serde_json::Number::from_f64(geo.longitude).unwrap()),
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

fn json_value_to_metadata_value(value: &Value) -> Result<MetadataValue, VectorError> {
    match value {
        Value::String(s) => Ok(MetadataValue::StringVal(s.clone())),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(MetadataValue::IntegerVal(i))
            } else if let Some(f) = n.as_f64() {
                Ok(MetadataValue::NumberVal(f))
            } else {
                Err(VectorError::InvalidParams(
                    "Invalid number value".to_string(),
                ))
            }
        }
        Value::Bool(b) => Ok(MetadataValue::BooleanVal(*b)),
        Value::Array(arr) => {
            use golem_vector::golem::vector::types::MetadataFunc;
            let metadata_arr: Result<Vec<MetadataFunc>, VectorError> = arr
                .iter()
                .map(|v| {
                    let metadata_val = json_value_to_metadata_value(v)?;
                    Ok(MetadataFunc::new(metadata_val))
                })
                .collect();
            Ok(MetadataValue::ArrayVal(metadata_arr?))
        }
        Value::Object(obj) => {
            if obj.contains_key("lat") && obj.contains_key("lon") {
                let lat = obj
                    .get("lat")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| VectorError::InvalidParams("Invalid latitude".to_string()))?;
                let lon = obj
                    .get("lon")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| VectorError::InvalidParams("Invalid longitude".to_string()))?;

                return Ok(MetadataValue::GeoVal(GeoCoordinates {
                    latitude: lat,
                    longitude: lon,
                }));
            }

            use golem_vector::golem::vector::types::MetadataFunc;
            let mut metadata_obj = Vec::new();
            for (key, val) in obj {
                let metadata_val = json_value_to_metadata_value(val)?;
                metadata_obj.push((key.clone(), MetadataFunc::new(metadata_val)));
            }
            Ok(MetadataValue::ObjectVal(metadata_obj))
        }
        Value::Null => Ok(MetadataValue::NullVal),
    }
}

fn filter_expression_to_qdrant_filter(filter: &FilterExpression) -> Result<Filter, VectorError> {
    match filter {
        FilterExpression::And(filters) => {
            let conditions: Result<Vec<Condition>, VectorError> = filters
                .iter()
                .map(|f| filter_expression_to_qdrant_condition(f.get()))
                .collect();
            Ok(Filter {
                must: Some(conditions?),
                should: None,
                must_not: None,
            })
        }
        FilterExpression::Or(filters) => {
            let conditions: Result<Vec<Condition>, VectorError> = filters
                .iter()
                .map(|f| filter_expression_to_qdrant_condition(f.get()))
                .collect();
            Ok(Filter {
                should: Some(conditions?),
                must: None,
                must_not: None,
            })
        }
        FilterExpression::Not(filter) => {
            let condition = filter_expression_to_qdrant_condition(filter.get())?;
            Ok(Filter {
                must_not: Some(vec![condition]),
                must: None,
                should: None,
            })
        }
        FilterExpression::Condition(condition) => {
            let qdrant_condition = filter_condition_to_qdrant_condition(condition)?;
            Ok(Filter {
                must: Some(vec![qdrant_condition]),
                should: None,
                must_not: None,
            })
        }
    }
}

fn filter_expression_to_qdrant_condition(
    filter: &FilterExpression,
) -> Result<Condition, VectorError> {
    match filter {
        FilterExpression::Condition(condition) => filter_condition_to_qdrant_condition(condition),
        other => {
            let nested_filter = filter_expression_to_qdrant_filter(other)?;
            Ok(Condition::Nested(nested_filter))
        }
    }
}

fn filter_condition_to_qdrant_condition(
    condition: &FilterCondition,
) -> Result<Condition, VectorError> {
    let field_condition = match condition.operator {
        FilterOperator::Eq => {
            let match_value = metadata_value_to_match_value(&condition.value)?;
            FieldCondition {
                key: condition.field.clone(),
                condition: FieldConditionOneOf::Match {
                    r#match: MatchCondition { value: match_value },
                },
            }
        }
        FilterOperator::Ne => {
            return Err(VectorError::ProviderError(
                "NotEqual operator not directly supported, use Not(Equal) instead".to_string(),
            ));
        }
        FilterOperator::Gt => {
            if let MetadataValue::NumberVal(f) = &condition.value {
                FieldCondition {
                    key: condition.field.clone(),
                    condition: FieldConditionOneOf::Range {
                        range: RangeInterface {
                            gt: Some(*f),
                            gte: None,
                            lt: None,
                            lte: None,
                        },
                    },
                }
            } else if let MetadataValue::IntegerVal(i) = &condition.value {
                FieldCondition {
                    key: condition.field.clone(),
                    condition: FieldConditionOneOf::Range {
                        range: RangeInterface {
                            gt: Some(*i as f64),
                            gte: None,
                            lt: None,
                            lte: None,
                        },
                    },
                }
            } else {
                return Err(VectorError::InvalidParams(
                    "GreaterThan operator requires numeric value".to_string(),
                ));
            }
        }
        FilterOperator::Gte => {
            if let MetadataValue::NumberVal(f) = &condition.value {
                FieldCondition {
                    key: condition.field.clone(),
                    condition: FieldConditionOneOf::Range {
                        range: RangeInterface {
                            gte: Some(*f),
                            gt: None,
                            lt: None,
                            lte: None,
                        },
                    },
                }
            } else if let MetadataValue::IntegerVal(i) = &condition.value {
                FieldCondition {
                    key: condition.field.clone(),
                    condition: FieldConditionOneOf::Range {
                        range: RangeInterface {
                            gte: Some(*i as f64),
                            gt: None,
                            lt: None,
                            lte: None,
                        },
                    },
                }
            } else {
                return Err(VectorError::InvalidParams(
                    "GreaterThanOrEqual operator requires numeric value".to_string(),
                ));
            }
        }
        FilterOperator::Lt => {
            if let MetadataValue::NumberVal(f) = &condition.value {
                FieldCondition {
                    key: condition.field.clone(),
                    condition: FieldConditionOneOf::Range {
                        range: RangeInterface {
                            lt: Some(*f),
                            lte: None,
                            gt: None,
                            gte: None,
                        },
                    },
                }
            } else if let MetadataValue::IntegerVal(i) = &condition.value {
                FieldCondition {
                    key: condition.field.clone(),
                    condition: FieldConditionOneOf::Range {
                        range: RangeInterface {
                            lt: Some(*i as f64),
                            lte: None,
                            gt: None,
                            gte: None,
                        },
                    },
                }
            } else {
                return Err(VectorError::InvalidParams(
                    "LessThan operator requires numeric value".to_string(),
                ));
            }
        }
        FilterOperator::Lte => {
            if let MetadataValue::NumberVal(f) = &condition.value {
                FieldCondition {
                    key: condition.field.clone(),
                    condition: FieldConditionOneOf::Range {
                        range: RangeInterface {
                            lte: Some(*f),
                            lt: None,
                            gt: None,
                            gte: None,
                        },
                    },
                }
            } else if let MetadataValue::IntegerVal(i) = &condition.value {
                FieldCondition {
                    key: condition.field.clone(),
                    condition: FieldConditionOneOf::Range {
                        range: RangeInterface {
                            lte: Some(*i as f64),
                            lt: None,
                            gt: None,
                            gte: None,
                        },
                    },
                }
            } else {
                return Err(VectorError::InvalidParams(
                    "LessThanOrEqual operator requires numeric value".to_string(),
                ));
            }
        }
        FilterOperator::In => {
            if let MetadataValue::ArrayVal(arr) = &condition.value {
                let match_values: Result<Vec<String>, VectorError> = arr
                    .iter()
                    .map(|func| {
                        let val = func.get();
                        match val {
                            MetadataValue::StringVal(s) => Ok(s.clone()),
                            MetadataValue::IntegerVal(i) => Ok(i.to_string()),
                            MetadataValue::NumberVal(f) => Ok(f.to_string()),
                            _ => Err(VectorError::InvalidParams(
                                "In operator array must contain strings or numbers".to_string(),
                            )),
                        }
                    })
                    .collect();

                FieldCondition {
                    key: condition.field.clone(),
                    condition: FieldConditionOneOf::Match {
                        r#match: MatchCondition {
                            value: MatchValue::Strings(match_values?),
                        },
                    },
                }
            } else {
                return Err(VectorError::InvalidParams(
                    "In operator requires array value".to_string(),
                ));
            }
        }
        FilterOperator::Nin => {
            return Err(VectorError::ProviderError(
                "NotIn operator not directly supported, use Not(In) instead".to_string(),
            ));
        }
        FilterOperator::Contains => {
            return Err(VectorError::UnsupportedFeature(
                "Contains operator not supported in Qdrant".to_string(),
            ));
        }
        FilterOperator::NotContains => {
            return Err(VectorError::UnsupportedFeature(
                "NotContains operator not supported in Qdrant".to_string(),
            ));
        }
        FilterOperator::Regex => {
            return Err(VectorError::UnsupportedFeature(
                "Regex operator not supported in Qdrant".to_string(),
            ));
        }
        FilterOperator::GeoWithin => {
            return Err(VectorError::UnsupportedFeature(
                "GeoWithin operator not yet implemented for Qdrant".to_string(),
            ));
        }
        FilterOperator::GeoBbox => {
            return Err(VectorError::UnsupportedFeature(
                "GeoBbox operator not yet implemented for Qdrant".to_string(),
            ));
        }
    };

    Ok(Condition::Field(field_condition))
}

fn metadata_value_to_match_value(value: &MetadataValue) -> Result<MatchValue, VectorError> {
    match value {
        MetadataValue::StringVal(s) => Ok(MatchValue::String(s.clone())),
        MetadataValue::IntegerVal(i) => Ok(MatchValue::Integer(*i)),
        MetadataValue::BooleanVal(b) => Ok(MatchValue::Boolean(*b)),
        _ => Err(VectorError::InvalidParams(
            "Unsupported metadata value type for match".to_string(),
        )),
    }
}

fn recommendation_example_to_qdrant(
    example: &RecommendationExample,
) -> Result<RecommendExample, VectorError> {
    match example {
        RecommendationExample::VectorId(id) => {
            let point_id = id_to_point_id(id)?;
            Ok(RecommendExample::PointId(point_id))
        }
        RecommendationExample::VectorData(vector_data) => match vector_data {
            VectorData::Dense(values) => Ok(RecommendExample::Vector(values.clone())),
            VectorData::Sparse(_) => Err(VectorError::ProviderError(
                "Sparse vectors not supported in recommendations".to_string(),
            )),
            VectorData::Binary(_) => Err(VectorError::ProviderError(
                "Binary vectors not supported in recommendations".to_string(),
            )),
            VectorData::Half(_) => Err(VectorError::ProviderError(
                "Half vectors not supported in recommendations".to_string(),
            )),
            VectorData::Named(_) => Err(VectorError::ProviderError(
                "Named vectors not supported in recommendations".to_string(),
            )),
            VectorData::Hybrid(_) => Err(VectorError::ProviderError(
                "Hybrid vectors not supported in recommendations".to_string(),
            )),
        },
    }
}

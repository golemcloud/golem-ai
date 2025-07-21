use crate::client::{
    ElasticsearchHit, ElasticsearchMappings, ElasticsearchQuery, ElasticsearchSearchResponse,
    ElasticsearchSettings,
};
use golem_search::golem::search::types::{
    Doc, FieldType, Schema, SchemaField, SearchHit, SearchQuery, SearchResults,
};
use serde_json::{json, Map, Value};

pub fn doc_to_elasticsearch_document(doc: Doc) -> Result<Value, String> {
    // Validate document ID length (Elasticsearch limit is 512 bytes)
    if doc.id.len() > 512 {
        return Err(format!(
            "Document ID too long: {} bytes (max 512)",
            doc.id.len()
        ));
    }

    let content: Value = serde_json::from_str(&doc.content)
        .map_err(|e| format!("Invalid JSON in document content: {e}"))?;

    let document = match content {
        Value::Object(mut obj) => {
            obj.insert("id".to_string(), Value::String(doc.id));
            Value::Object(obj)
        }
        _ => {
            let mut obj = Map::new();
            obj.insert("id".to_string(), Value::String(doc.id));
            obj.insert("content".to_string(), content);
            Value::Object(obj)
        }
    };

    Ok(document)
}

pub fn elasticsearch_document_to_doc(id: String, source: Value) -> Doc {
    let content = serde_json::to_string(&source).unwrap_or_else(|_| "{}".to_string());
    Doc { id, content }
}

pub fn search_query_to_elasticsearch_query(query: SearchQuery) -> ElasticsearchQuery {
    let mut es_query = ElasticsearchQuery {
        query: None,
        from: query.offset,
        size: query.per_page,
        sort: None,
        highlight: None,
        aggs: None,
        _source: None,
    };

    let mut bool_query = json!({
        "bool": {
            "must": [],
            "filter": []
        }
    });

    if let Some(q) = query.q {
        if !q.trim().is_empty() {
            bool_query["bool"]["must"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "multi_match": {
                        "query": q,
                        "type": "best_fields",
                        "fields": ["*"]
                    }
                }));
        }
    }

    for filter in query.filters {
        if let Ok(filter_value) = serde_json::from_str::<Value>(&filter) {
            // JSON filter
            bool_query["bool"]["filter"]
                .as_array_mut()
                .unwrap()
                .push(filter_value);
        } else if filter.contains(':') {
            let parts: Vec<&str> = filter.splitn(2, ':').collect();
            if parts.len() == 2 {
                let field = parts[0].trim();
                let value = parts[1].trim();
                bool_query["bool"]["filter"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({
                        "term": {
                            field: value
                        }
                    }));
            }
        } else if filter.contains('=') {
            let parts: Vec<&str> = filter.splitn(2, '=').collect();
            if parts.len() == 2 {
                let field = parts[0].trim();
                let value = parts[1].trim().trim_matches('"').trim_matches('\'');
                bool_query["bool"]["filter"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({
                        "term": {
                            field: value
                        }
                    }));
            }
        } else {
            bool_query["bool"]["filter"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "term": {
                        "status": filter
                    }
                }));
        }
    }

    if !bool_query["bool"]["must"].as_array().unwrap().is_empty()
        || !bool_query["bool"]["filter"].as_array().unwrap().is_empty()
    {
        es_query.query = Some(bool_query);
    } else {
        es_query.query = Some(json!({
            "match_all": {}
        }));
    }

    if !query.sort.is_empty() {
        let mut sort_array = Vec::new();
        for sort_field in query.sort {
            if let Some(colon_pos) = sort_field.find(':') {
                let field = &sort_field[..colon_pos];
                let direction = &sort_field[colon_pos + 1..];
                let order = if direction == "desc" { "desc" } else { "asc" };
                sort_array.push(json!({
                    field: {
                        "order": order
                    }
                }));
            } else if let Some(field) = sort_field.strip_prefix('-') {
                sort_array.push(json!({
                    field: {
                        "order": "desc"
                    }
                }));
            } else {
                sort_array.push(json!({
                    sort_field: {
                        "order": "asc"
                    }
                }));
            }
        }
        es_query.sort = Some(sort_array);
    }

    if let Some(highlight_config) = query.highlight {
        let mut highlight = json!({
            "fields": {}
        });

        for field in highlight_config.fields {
            highlight["fields"][field] = json!({});
        }

        if let Some(pre_tag) = highlight_config.pre_tag {
            highlight["pre_tags"] = json!([pre_tag]);
        }
        if let Some(post_tag) = highlight_config.post_tag {
            highlight["post_tags"] = json!([post_tag]);
        }

        if let Some(max_length) = highlight_config.max_length {
            highlight["fragment_size"] = json!(max_length);
        }

        es_query.highlight = Some(highlight);
    }

    if !query.facets.is_empty() {
        let mut aggs = json!({});
        for facet in query.facets {
            aggs[&facet] = json!({
                "terms": {
                    "field": format!("{}.keyword", facet),
                    "size": 10
                }
            });
        }
        es_query.aggs = Some(aggs);
    }

    if let Some(config) = query.config {
        if !config.attributes_to_retrieve.is_empty() {
            es_query._source = Some(json!(config.attributes_to_retrieve));
        }

        if !config.boost_fields.is_empty() && es_query.query.is_some() {
            if let Some(query_obj) = es_query.query.as_mut() {
                if let Some(multi_match) = query_obj
                    .get_mut("bool")
                    .and_then(|b| b.get_mut("must"))
                    .and_then(|m| m.as_array_mut())
                    .and_then(|arr| arr.first_mut())
                    .and_then(|first| first.get_mut("multi_match"))
                {
                    let mut boosted_fields = Vec::new();
                    for (field, boost) in config.boost_fields {
                        boosted_fields.push(format!("{field}^{boost}"));
                    }
                    multi_match["fields"] = json!(boosted_fields);
                }
            }
        }
    }

    es_query
}

pub fn elasticsearch_response_to_search_results(
    response: ElasticsearchSearchResponse,
) -> SearchResults {
    let hits = response
        .hits
        .hits
        .into_iter()
        .map(elasticsearch_hit_to_search_hit)
        .collect();

    let total = match response.hits.total.relation.as_str() {
        "eq" => Some(response.hits.total.value),
        "gte" => Some(response.hits.total.value),
        _ => None,
    };

    SearchResults {
        total,
        page: None, // Elasticsearch uses from/size, not page-based pagination
        per_page: None,
        hits,
        facets: response
            .aggregations
            .map(|aggs| serde_json::to_string(&aggs).unwrap_or_else(|_| "{}".to_string())),
        took_ms: Some(response.took),
    }
}

fn elasticsearch_hit_to_search_hit(hit: ElasticsearchHit) -> SearchHit {
    let content = hit
        .source
        .map(|source| serde_json::to_string(&source).unwrap_or_else(|_| "{}".to_string()));

    let highlights = hit
        .highlight
        .map(|highlight| serde_json::to_string(&highlight).unwrap_or_else(|_| "{}".to_string()));

    SearchHit {
        id: hit.id,
        score: hit.score,
        content,
        highlights,
    }
}

pub fn schema_to_elasticsearch_settings(schema: Schema) -> ElasticsearchSettings {
    let mut properties = Map::new();

    for field in schema.fields {
        let mut field_mapping = Map::new();

        match field.field_type {
            FieldType::Text => {
                field_mapping.insert("type".to_string(), Value::String("text".to_string()));

                field_mapping.insert(
                    "fields".to_string(),
                    json!({
                        "keyword": {
                            "type": "keyword",
                            "ignore_above": 256
                        }
                    }),
                );
            }
            FieldType::Keyword => {
                field_mapping.insert("type".to_string(), Value::String("keyword".to_string()));
            }
            FieldType::Integer => {
                field_mapping.insert("type".to_string(), Value::String("integer".to_string()));
            }
            FieldType::Float => {
                field_mapping.insert("type".to_string(), Value::String("float".to_string()));
            }
            FieldType::Boolean => {
                field_mapping.insert("type".to_string(), Value::String("boolean".to_string()));
            }
            FieldType::Date => {
                field_mapping.insert("type".to_string(), Value::String("date".to_string()));
            }
            FieldType::GeoPoint => {
                field_mapping.insert("type".to_string(), Value::String("geo_point".to_string()));
            }
        }

        if !field.index {
            field_mapping.insert("index".to_string(), Value::Bool(false));
        }

        properties.insert(field.name, Value::Object(field_mapping));
    }

    properties.insert(
        "year".to_string(),
        json!({
            "type": "integer"
        }),
    );
    properties.insert(
        "id".to_string(),
        json!({
            "type": "keyword"
        }),
    );

    let mappings = ElasticsearchMappings {
        properties: Some(properties),
        dynamic: Some(true),
    };

    ElasticsearchSettings {
        mappings: Some(mappings),
        settings: None,
    }
}

pub fn elasticsearch_mappings_to_schema(mappings: Value, index_name: &str) -> Schema {
    let mut fields = Vec::new();

    if let Some(index_mappings) = mappings.get(index_name) {
        if let Some(properties) = index_mappings
            .get("mappings")
            .and_then(|m| m.get("properties"))
            .and_then(|p| p.as_object())
        {
            for (field_name, field_def) in properties {
                if let Some(field_type_str) = field_def.get("type").and_then(|t| t.as_str()) {
                    let field_type = match field_type_str {
                        "text" => FieldType::Text,
                        "keyword" => FieldType::Keyword,
                        "integer" | "long" | "short" | "byte" => FieldType::Integer,
                        "float" | "double" | "half_float" | "scaled_float" => FieldType::Float,
                        "boolean" => FieldType::Boolean,
                        "date" => FieldType::Date,
                        "geo_point" => FieldType::GeoPoint,
                        _ => FieldType::Text,
                    };

                    let index = field_def
                        .get("index")
                        .and_then(|i| i.as_bool())
                        .unwrap_or(true);

                    fields.push(SchemaField {
                        name: field_name.clone(),
                        field_type,
                        required: false, // Elasticsearch doesn't have required fields in mappings
                        facet: field_type == FieldType::Keyword, // Keywords can be used for faceting
                        sort: field_type != FieldType::Text, // Text fields typically can't be sorted directly
                        index,
                    });
                }
            }
        }
    }

    Schema {
        fields,
        primary_key: Some("id".to_string()),
    }
}

pub fn create_retry_query(original_query: &SearchQuery, partial_hits: &[SearchHit]) -> SearchQuery {
    let mut retry_query = original_query.clone();

    if !partial_hits.is_empty() {
        let current_offset = original_query.offset.unwrap_or(0);
        let received_count = partial_hits.len() as u32;
        retry_query.offset = Some(current_offset + received_count);
    }

    retry_query
}

pub fn build_bulk_operations(
    index_name: &str,
    docs: &[Doc],
    operation: &str,
) -> Result<String, String> {
    let mut bulk_ops = String::new();

    for doc in docs {
        let action = json!({
            operation: {
                "_index": index_name,
                "_id": doc.id
            }
        });
        bulk_ops.push_str(&serde_json::to_string(&action).map_err(|e| e.to_string())?);
        bulk_ops.push('\n');

        if operation != "delete" {
            let document = doc_to_elasticsearch_document(doc.clone())?;
            bulk_ops.push_str(&serde_json::to_string(&document).map_err(|e| e.to_string())?);
            bulk_ops.push('\n');
        }
    }

    Ok(bulk_ops)
}

pub fn build_bulk_delete_operations(index_name: &str, ids: &[String]) -> Result<String, String> {
    let mut bulk_ops = String::new();

    for id in ids {
        let action = json!({
            "delete": {
                "_index": index_name,
                "_id": id
            }
        });
        bulk_ops.push_str(&serde_json::to_string(&action).map_err(|e| e.to_string())?);
        bulk_ops.push('\n');
    }

    Ok(bulk_ops)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{
        ElasticsearchHit, ElasticsearchHits, ElasticsearchSearchResponse, ElasticsearchTotal,
    };
    use golem_search::golem::search::types::{HighlightConfig, SearchConfig};

    #[test]
    fn test_doc_to_elasticsearch_document() {
        let doc = Doc {
            id: "test-id".to_string(),
            content: r#"{"title": "Test Document", "content": "This is a test"}"#.to_string(),
        };

        let es_doc = doc_to_elasticsearch_document(doc).unwrap();
        assert_eq!(es_doc["id"], "test-id");
        assert_eq!(es_doc["title"], "Test Document");
        assert_eq!(es_doc["content"], "This is a test");
    }

    #[test]
    fn test_doc_to_elasticsearch_document_id_too_long() {
        let long_id = "a".repeat(600);
        let doc = Doc {
            id: long_id,
            content: r#"{"title": "Test"}"#.to_string(),
        };

        let result = doc_to_elasticsearch_document(doc);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Document ID too long"));
    }

    #[test]
    fn test_doc_to_elasticsearch_document_invalid_json() {
        let doc = Doc {
            id: "test-id".to_string(),
            content: "invalid json".to_string(),
        };

        let result = doc_to_elasticsearch_document(doc);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Invalid JSON in document content"));
    }

    #[test]
    fn test_elasticsearch_document_to_doc() {
        let source = serde_json::json!({
            "title": "Test Document",
            "content": "This is a test"
        });

        let doc = elasticsearch_document_to_doc("test-id".to_string(), source);
        assert_eq!(doc.id, "test-id");
        assert!(doc.content.contains("Test Document"));
        assert!(doc.content.contains("This is a test"));
    }

    #[test]
    fn test_search_query_to_elasticsearch_query() {
        let search_query = SearchQuery {
            q: Some("test query".to_string()),
            filters: vec!["category:electronics".to_string()],
            sort: vec!["price:desc".to_string()],
            facets: vec!["category".to_string()],
            page: None,
            per_page: Some(20),
            offset: Some(10),
            highlight: Some(HighlightConfig {
                fields: vec!["title".to_string(), "description".to_string()],
                pre_tag: Some("<mark>".to_string()),
                post_tag: Some("</mark>".to_string()),
                max_length: Some(200),
            }),
            config: None,
        };

        let es_query = search_query_to_elasticsearch_query(search_query);
        assert_eq!(es_query.from, Some(10));
        assert_eq!(es_query.size, Some(20));
        assert!(es_query.query.is_some());
        assert!(es_query.sort.is_some());
        assert!(es_query.highlight.is_some());
        assert!(es_query.aggs.is_some());
    }

    #[test]
    fn test_search_query_no_query() {
        let search_query = SearchQuery {
            q: None,
            filters: vec![],
            sort: vec![],
            facets: vec![],
            page: None,
            per_page: None,
            offset: None,
            highlight: None,
            config: None,
        };

        let es_query = search_query_to_elasticsearch_query(search_query);
        assert!(es_query.query.is_some());
        // Should have match_all query
        assert_eq!(es_query.query.unwrap()["match_all"], serde_json::json!({}));
    }

    #[test]
    fn test_search_query_with_config() {
        let search_query = SearchQuery {
            q: Some("test".to_string()),
            filters: vec![],
            sort: vec![],
            facets: vec![],
            page: None,
            per_page: None,
            offset: None,
            highlight: None,
            config: Some(SearchConfig {
                attributes_to_retrieve: vec!["title".to_string(), "price".to_string()],
                typo_tolerance: None,
                timeout_ms: None,
                boost_fields: vec![("title".to_string(), 2.0)],
                exact_match_boost: None,
                language: None,
                provider_params: None,
            }),
        };

        let es_query = search_query_to_elasticsearch_query(search_query);
        assert!(es_query._source.is_some());
        assert_eq!(
            es_query._source.unwrap(),
            serde_json::json!(["title", "price"])
        );
    }

    #[test]
    fn test_elasticsearch_response_to_search_results() {
        let es_response = ElasticsearchSearchResponse {
            took: 5,
            timed_out: false,
            hits: ElasticsearchHits {
                total: ElasticsearchTotal {
                    value: 1,
                    relation: "eq".to_string(),
                },
                max_score: Some(1.0),
                hits: vec![ElasticsearchHit {
                    index: "test-index".to_string(),
                    id: "doc1".to_string(),
                    score: Some(1.0),
                    source: Some(serde_json::json!({"title": "Test Document"})),
                    highlight: Some(serde_json::json!({"title": ["Test <em>Document</em>"]})),
                }],
            },
            aggregations: Some(serde_json::json!({"category": {"buckets": []}})),
        };

        let search_results = elasticsearch_response_to_search_results(es_response);
        assert_eq!(search_results.total, Some(1));
        assert_eq!(search_results.hits.len(), 1);
        assert_eq!(search_results.hits[0].id, "doc1");
        assert_eq!(search_results.hits[0].score, Some(1.0));
        assert!(search_results.facets.is_some());
        assert_eq!(search_results.took_ms, Some(5));
    }

    #[test]
    fn test_schema_to_elasticsearch_settings() {
        let schema = Schema {
            fields: vec![
                SchemaField {
                    name: "title".to_string(),
                    field_type: FieldType::Text,
                    required: false,
                    facet: false,
                    sort: false,
                    index: true,
                },
                SchemaField {
                    name: "category".to_string(),
                    field_type: FieldType::Keyword,
                    required: false,
                    facet: true,
                    sort: true,
                    index: true,
                },
                SchemaField {
                    name: "price".to_string(),
                    field_type: FieldType::Float,
                    required: false,
                    facet: false,
                    sort: true,
                    index: false,
                },
            ],
            primary_key: Some("id".to_string()),
        };

        let settings = schema_to_elasticsearch_settings(schema);
        assert!(settings.mappings.is_some());
        let mappings = settings.mappings.unwrap();
        assert!(mappings.properties.is_some());
        let properties = mappings.properties.unwrap();
        assert!(properties.contains_key("title"));
        assert!(properties.contains_key("category"));
        assert!(properties.contains_key("price"));
    }

    #[test]
    fn test_create_retry_query() {
        let original_query = SearchQuery {
            q: Some("test".to_string()),
            filters: vec![],
            sort: vec![],
            facets: vec![],
            page: None,
            per_page: Some(10),
            offset: Some(0),
            highlight: None,
            config: None,
        };

        let partial_hits = vec![
            SearchHit {
                id: "doc1".to_string(),
                score: Some(1.0),
                content: Some("{}".to_string()),
                highlights: None,
            },
            SearchHit {
                id: "doc2".to_string(),
                score: Some(0.8),
                content: Some("{}".to_string()),
                highlights: None,
            },
        ];

        let retry_query = create_retry_query(&original_query, &partial_hits);
        assert_eq!(retry_query.offset, Some(2));
    }

    #[test]
    fn test_build_bulk_operations() {
        let docs = vec![
            Doc {
                id: "doc1".to_string(),
                content: r#"{"title": "Document 1"}"#.to_string(),
            },
            Doc {
                id: "doc2".to_string(),
                content: r#"{"title": "Document 2"}"#.to_string(),
            },
        ];

        let bulk_ops = build_bulk_operations("test-index", &docs, "index").unwrap();
        assert!(bulk_ops.contains("doc1"));
        assert!(bulk_ops.contains("doc2"));
        assert!(bulk_ops.contains("Document 1"));
        assert!(bulk_ops.contains("Document 2"));
    }

    #[test]
    fn test_build_bulk_delete_operations() {
        let ids = vec!["doc1".to_string(), "doc2".to_string()];

        let bulk_ops = build_bulk_delete_operations("test-index", &ids).unwrap();
        assert!(bulk_ops.contains("doc1"));
        assert!(bulk_ops.contains("doc2"));
        assert!(bulk_ops.contains("delete"));
    }
}

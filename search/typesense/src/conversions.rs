use crate::client::{
    CollectionField, CollectionSchema, SearchHit as TypesenseSearchHit,
    SearchQuery as TypesenseSearchQuery, SearchResponse, TypesenseDocument,
};
use golem_search::golem::search::types::{
    Doc, FieldType, Schema, SchemaField, SearchHit, SearchQuery, SearchResults,
};
use serde_json::{Map, Value};

pub fn doc_to_typesense_document(doc: Doc) -> Result<TypesenseDocument, String> {
    let mut fields: Map<String, Value> = serde_json::from_str(&doc.content)
        .map_err(|e| format!("Failed to parse document content as JSON: {e}"))?;

    fields.insert("id".to_string(), Value::String(doc.id));

    Ok(TypesenseDocument { fields })
}

pub fn _typesense_document_to_doc(doc: TypesenseDocument) -> Doc {
    let mut fields = doc.fields;

    let id = fields
        .remove("id")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let content = serde_json::to_string(&fields).unwrap_or_else(|_| "{}".to_string());

    Doc { id, content }
}

pub fn search_query_to_typesense_query(query: SearchQuery) -> TypesenseSearchQuery {
    let mut typesense_query = TypesenseSearchQuery {
        q: query.q.unwrap_or_else(|| "*".to_string()),
        query_by: Some("title,author,description,genre".to_string()),
        filter_by: None,
        sort_by: None,
        facet_by: None,
        max_facet_values: None,
        page: query.page,
        per_page: query.per_page,
        offset: query.offset,
        include_fields: None,
        exclude_fields: None,
        highlight_full_fields: None,
        highlight_affix_num_tokens: None,
        highlight_start_tag: None,
        highlight_end_tag: None,
        snippet_threshold: None,
        num_typos: None,
        min_len_1typo: None,
        min_len_2typo: None,
        typo_tokens_threshold: None,
        drop_tokens_threshold: None,
        pinned_hits: None,
        hidden_hits: None,
        group_by: None,
        group_limit: None,
        limit_hits: None,
        search_cutoff_ms: None,
        exhaustive_search: None,
        use_cache: None,
        cache_ttl: None,
        pre_segmented_query: None,
        enable_overrides: None,
        prioritize_exact_match: None,
        prioritize_token_position: None,
        max_candidates: None,
    };

    if !query.filters.is_empty() {
        typesense_query.filter_by = Some(query.filters.join(" && "));
    }

    if !query.sort.is_empty() {
        typesense_query.sort_by = Some(query.sort.join(","));
    }

    if !query.facets.is_empty() {
        typesense_query.facet_by = Some(query.facets.join(","));
    }

    if let Some(highlight) = query.highlight {
        if let Some(pre_tag) = highlight.pre_tag {
            typesense_query.highlight_start_tag = Some(pre_tag);
        }
        if let Some(post_tag) = highlight.post_tag {
            typesense_query.highlight_end_tag = Some(post_tag);
        }
        if let Some(max_length) = highlight.max_length {
            typesense_query.snippet_threshold = Some(max_length);
        }

        if !highlight.fields.is_empty() {
            typesense_query.highlight_full_fields = Some(highlight.fields.join(","));
        }
    }

    if let Some(config) = query.config {
        if let Some(timeout_ms) = config.timeout_ms {
            typesense_query.search_cutoff_ms = Some(timeout_ms);
        }

        if !config.attributes_to_retrieve.is_empty() {
            typesense_query.include_fields = Some(config.attributes_to_retrieve.join(","));
        }

        if !config.boost_fields.is_empty() {
            let mut query_by_fields = Vec::new();
            for (field, boost) in config.boost_fields {
                query_by_fields.push(format!("{field}:{boost}"));
            }
            typesense_query.query_by = Some(query_by_fields.join(","));
        }

        if let Some(typo_tolerance) = config.typo_tolerance {
            if typo_tolerance {
                typesense_query.num_typos = Some("2".to_string()); // Allow up to 2 typos
            } else {
                typesense_query.num_typos = Some("0".to_string()); // No typos allowed
            }
        }

        if let Some(exact_match_boost) = config.exact_match_boost {
            typesense_query.prioritize_exact_match = Some(exact_match_boost > 0.0);
        }

        // Parse provider-specific parameters
        if let Some(provider_params) = config.provider_params {
            if let Ok(params_map) = serde_json::from_str::<Map<String, Value>>(&provider_params) {
                if let Some(exhaustive_search) = params_map
                    .get("exhaustive_search")
                    .and_then(|v| v.as_bool())
                {
                    typesense_query.exhaustive_search = Some(exhaustive_search);
                }
                if let Some(use_cache) = params_map.get("use_cache").and_then(|v| v.as_bool()) {
                    typesense_query.use_cache = Some(use_cache);
                }
                if let Some(cache_ttl) = params_map.get("cache_ttl").and_then(|v| v.as_u64()) {
                    typesense_query.cache_ttl = Some(cache_ttl as u32);
                }
                if let Some(group_by) = params_map.get("group_by").and_then(|v| v.as_str()) {
                    typesense_query.group_by = Some(group_by.to_string());
                }
                if let Some(group_limit) = params_map.get("group_limit").and_then(|v| v.as_u64()) {
                    typesense_query.group_limit = Some(group_limit as u32);
                }
                if let Some(max_facet_values) =
                    params_map.get("max_facet_values").and_then(|v| v.as_u64())
                {
                    typesense_query.max_facet_values = Some(max_facet_values as u32);
                }
                if let Some(limit_hits) = params_map.get("limit_hits").and_then(|v| v.as_u64()) {
                    typesense_query.limit_hits = Some(limit_hits as u32);
                }
                if let Some(prioritize_token_position) = params_map
                    .get("prioritize_token_position")
                    .and_then(|v| v.as_bool())
                {
                    typesense_query.prioritize_token_position = Some(prioritize_token_position);
                }
                if let Some(max_candidates) =
                    params_map.get("max_candidates").and_then(|v| v.as_u64())
                {
                    typesense_query.max_candidates = Some(max_candidates as u32);
                }
                if let Some(drop_tokens_threshold) = params_map
                    .get("drop_tokens_threshold")
                    .and_then(|v| v.as_u64())
                {
                    typesense_query.drop_tokens_threshold = Some(drop_tokens_threshold as u32);
                }
                if let Some(typo_tokens_threshold) = params_map
                    .get("typo_tokens_threshold")
                    .and_then(|v| v.as_u64())
                {
                    typesense_query.typo_tokens_threshold = Some(typo_tokens_threshold as u32);
                }
                if let Some(min_len_1typo) =
                    params_map.get("min_len_1typo").and_then(|v| v.as_u64())
                {
                    typesense_query.min_len_1typo = Some(min_len_1typo as u32);
                }
                if let Some(min_len_2typo) =
                    params_map.get("min_len_2typo").and_then(|v| v.as_u64())
                {
                    typesense_query.min_len_2typo = Some(min_len_2typo as u32);
                }
                if let Some(enable_overrides) =
                    params_map.get("enable_overrides").and_then(|v| v.as_bool())
                {
                    typesense_query.enable_overrides = Some(enable_overrides);
                }
                if let Some(pre_segmented_query) = params_map
                    .get("pre_segmented_query")
                    .and_then(|v| v.as_bool())
                {
                    typesense_query.pre_segmented_query = Some(pre_segmented_query);
                }
                if let Some(pinned_hits) = params_map.get("pinned_hits").and_then(|v| v.as_str()) {
                    typesense_query.pinned_hits = Some(pinned_hits.to_string());
                }
                if let Some(hidden_hits) = params_map.get("hidden_hits").and_then(|v| v.as_str()) {
                    typesense_query.hidden_hits = Some(hidden_hits.to_string());
                }
                if let Some(exclude_fields) =
                    params_map.get("exclude_fields").and_then(|v| v.as_str())
                {
                    typesense_query.exclude_fields = Some(exclude_fields.to_string());
                }
                if let Some(highlight_affix_num_tokens) = params_map
                    .get("highlight_affix_num_tokens")
                    .and_then(|v| v.as_u64())
                {
                    typesense_query.highlight_affix_num_tokens =
                        Some(highlight_affix_num_tokens as u32);
                }
            }
        }
    }

    typesense_query
}

pub fn typesense_response_to_search_results(response: SearchResponse) -> SearchResults {
    let hits = response
        .hits
        .into_iter()
        .map(typesense_hit_to_search_hit)
        .collect();

    let facets = response.facet_counts.map(|facet_counts| {
        let facets_map: Map<String, Value> = facet_counts
            .into_iter()
            .map(|facet_count| {
                let values: Map<String, Value> = facet_count
                    .counts
                    .into_iter()
                    .map(|facet_value| {
                        (
                            facet_value.value.as_str().unwrap_or("unknown").to_string(),
                            Value::Number(serde_json::Number::from(facet_value.count)),
                        )
                    })
                    .collect();
                (facet_count.field_name, Value::Object(values))
            })
            .collect();
        serde_json::to_string(&facets_map).unwrap_or_default()
    });

    SearchResults {
        total: Some(response.found),
        page: Some(response.page),
        per_page: Some(response.request_params.per_page),
        hits,
        facets,
        took_ms: Some(response.search_time_ms),
    }
}

pub fn typesense_hit_to_search_hit(hit: TypesenseSearchHit) -> SearchHit {
    let mut document = hit.document;

    let (id, content) =
        if let Some(nested_doc) = document.get("document").and_then(|v| v.as_object()) {
            let id = nested_doc
                .get("id")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown".to_string());

            let mut content_doc = nested_doc.clone();
            content_doc.remove("id");
            let content = serde_json::to_string(&content_doc).unwrap_or_else(|_| "{}".to_string());

            (id, content)
        } else {
            let id = document
                .remove("id")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown".to_string());

            let content = serde_json::to_string(&document).unwrap_or_else(|_| "{}".to_string());

            (id, content)
        };

    let score = hit.text_match.map(|tm| tm as f64 / 1000000.0); // Typesense uses large integers for text_match

    let highlights = hit
        .highlight
        .or_else(|| {
            hit.highlights.map(|h_array| {
                let mut highlight_map = serde_json::Map::new();
                for h in h_array {
                    if let Some(obj) = h.as_object() {
                        if let (Some(field), Some(snippet)) = (obj.get("field"), obj.get("snippet"))
                        {
                            if let (Some(field_str), Some(snippet_str)) =
                                (field.as_str(), snippet.as_str())
                            {
                                highlight_map.insert(
                                    field_str.to_string(),
                                    serde_json::Value::String(snippet_str.to_string()),
                                );
                            }
                        }
                    }
                }
                highlight_map
            })
        })
        .map(|h| serde_json::to_string(&h).unwrap_or_default());

    SearchHit {
        id,
        score,
        content: Some(content),
        highlights,
    }
}

pub fn schema_to_typesense_schema(schema: Schema, collection_name: &str) -> CollectionSchema {
    let fields: Vec<CollectionField> = schema
        .fields
        .iter()
        .map(|f| schema_field_to_collection_field(f.clone()))
        .collect();

    let default_sorting_field = schema
        .fields
        .iter()
        .find(|f| f.sort && f.name != "id" && f.required)
        .map(|f| f.name.clone());

    CollectionSchema {
        name: collection_name.to_string(),
        fields,
        default_sorting_field,
        enable_nested_fields: None,
        token_separators: None,
        symbols_to_index: None,
    }
}

pub fn schema_field_to_collection_field(field: SchemaField) -> CollectionField {
    let field_type = match field.field_type {
        FieldType::Text => "string",
        FieldType::Keyword => "string",
        FieldType::Integer => "int32",
        FieldType::Float => "float",
        FieldType::Boolean => "bool",
        FieldType::Date => "int64",
        FieldType::GeoPoint => "geopoint",
    }
    .to_string();

    CollectionField {
        name: field.name,
        field_type,
        facet: Some(field.facet),
        index: Some(field.index),
        sort: Some(field.sort),
        optional: Some(!field.required),
    }
}

pub fn _typesense_schema_to_schema(schema: CollectionSchema) -> Schema {
    let fields = schema
        .fields
        .into_iter()
        .map(collection_field_to_schema_field)
        .collect();

    Schema {
        fields,
        primary_key: schema.default_sorting_field,
    }
}

pub fn collection_field_to_schema_field(field: CollectionField) -> SchemaField {
    let field_type = match field.field_type.as_str() {
        "string" => FieldType::Text,
        "int32" | "int64" => FieldType::Integer,
        "float" => FieldType::Float,
        "bool" => FieldType::Boolean,
        "geopoint" => FieldType::GeoPoint,
        _ => FieldType::Text,
    };

    SchemaField {
        name: field.name,
        field_type,
        required: !field.optional.unwrap_or(false),
        facet: field.facet.unwrap_or(false),
        sort: field.sort.unwrap_or(false),
        index: field.index.unwrap_or(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{FacetCount, FacetValue, RequestParams, SearchHit as TypesenseSearchHit};
    use golem_search::golem::search::types::{HighlightConfig, SearchConfig};

    #[test]
    fn test_doc_to_typesense_document() {
        let doc = Doc {
            id: "test-id".to_string(),
            content: r#"{"title": "Test Document", "content": "This is a test"}"#.to_string(),
        };

        let typesense_doc = doc_to_typesense_document(doc).unwrap();
        assert_eq!(typesense_doc.fields.get("id").unwrap(), "test-id");
        assert_eq!(typesense_doc.fields.get("title").unwrap(), "Test Document");
        assert_eq!(
            typesense_doc.fields.get("content").unwrap(),
            "This is a test"
        );
    }

    #[test]
    fn test_doc_to_typesense_document_invalid_json() {
        let doc = Doc {
            id: "test-id".to_string(),
            content: "invalid json".to_string(),
        };

        let result = doc_to_typesense_document(doc);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Failed to parse document content as JSON"));
    }

    #[test]
    fn test_typesense_document_to_doc() {
        let mut fields = Map::new();
        fields.insert("id".to_string(), Value::String("test-id".to_string()));
        fields.insert(
            "title".to_string(),
            Value::String("Test Document".to_string()),
        );
        fields.insert(
            "content".to_string(),
            Value::String("This is a test".to_string()),
        );

        let typesense_doc = TypesenseDocument { fields };
        let doc = _typesense_document_to_doc(typesense_doc);
        assert_eq!(doc.id, "test-id");
        assert!(doc.content.contains("Test Document"));
        assert!(doc.content.contains("This is a test"));
        assert!(!doc.content.contains("\"id\":"));
    }

    #[test]
    fn test_typesense_document_to_doc_no_id() {
        let mut fields = Map::new();
        fields.insert("title".to_string(), Value::String("Test".to_string()));

        let typesense_doc = TypesenseDocument { fields };
        let doc = _typesense_document_to_doc(typesense_doc);
        assert_eq!(doc.id, "unknown");
    }

    #[test]
    fn test_search_query_to_typesense_query() {
        let search_query = SearchQuery {
            q: Some("test query".to_string()),
            filters: vec!["category:electronics".to_string(), "price:>100".to_string()],
            sort: vec!["price:desc".to_string()],
            facets: vec!["category".to_string(), "brand".to_string()],
            page: Some(1),
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

        let typesense_query = search_query_to_typesense_query(search_query);
        assert_eq!(typesense_query.q, "test query");
        assert_eq!(
            typesense_query.filter_by,
            Some("category:electronics && price:>100".to_string())
        );
        assert_eq!(typesense_query.sort_by, Some("price:desc".to_string()));
        assert_eq!(typesense_query.facet_by, Some("category,brand".to_string()));
        assert_eq!(typesense_query.page, Some(1));
        assert_eq!(typesense_query.per_page, Some(20));
        assert_eq!(typesense_query.offset, Some(10));
        assert_eq!(
            typesense_query.highlight_start_tag,
            Some("<mark>".to_string())
        );
        assert_eq!(
            typesense_query.highlight_end_tag,
            Some("</mark>".to_string())
        );
        assert_eq!(typesense_query.snippet_threshold, Some(200));
        assert_eq!(
            typesense_query.highlight_full_fields,
            Some("title,description".to_string())
        );
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
                typo_tolerance: Some(false),
                timeout_ms: Some(5000),
                boost_fields: vec![("title".to_string(), 2.0), ("description".to_string(), 1.5)],
                exact_match_boost: Some(1.5),
                language: None,
                provider_params: Some(
                    r#"{"exhaustive_search": true, "use_cache": false, "max_facet_values": 100}"#
                        .to_string(),
                ),
            }),
        };

        let typesense_query = search_query_to_typesense_query(search_query);
        assert_eq!(
            typesense_query.include_fields,
            Some("title,price".to_string())
        );
        assert_eq!(typesense_query.num_typos, Some("0".to_string()));
        assert_eq!(typesense_query.search_cutoff_ms, Some(5000));
        assert_eq!(
            typesense_query.query_by,
            Some("title:2,description:1.5".to_string())
        );
        assert_eq!(typesense_query.prioritize_exact_match, Some(true));
        assert_eq!(typesense_query.exhaustive_search, Some(true));
        assert_eq!(typesense_query.use_cache, Some(false));
        assert_eq!(typesense_query.max_facet_values, Some(100));
    }

    #[test]
    fn test_schema_to_typesense_schema() {
        let schema = Schema {
            fields: vec![
                SchemaField {
                    name: "id".to_string(),
                    field_type: FieldType::Text,
                    required: true,
                    facet: false,
                    sort: false,
                    index: true,
                },
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
                    sort: false,
                    index: true,
                },
                SchemaField {
                    name: "price".to_string(),
                    field_type: FieldType::Float,
                    required: false,
                    facet: true,
                    sort: true,
                    index: false,
                },
            ],
            primary_key: Some("id".to_string()),
        };

        let typesense_schema = schema_to_typesense_schema(schema, "test_collection");
        assert_eq!(typesense_schema.name, "test_collection");
        assert_eq!(typesense_schema.fields.len(), 4);

        let title_field = typesense_schema
            .fields
            .iter()
            .find(|f| f.name == "title")
            .unwrap();
        assert_eq!(title_field.field_type, "string");
        assert_eq!(title_field.index, Some(true));

        let category_field = typesense_schema
            .fields
            .iter()
            .find(|f| f.name == "category")
            .unwrap();
        assert_eq!(category_field.facet, Some(true));

        let price_field = typesense_schema
            .fields
            .iter()
            .find(|f| f.name == "price")
            .unwrap();
        assert_eq!(price_field.field_type, "float");
        assert_eq!(price_field.sort, Some(true));
        assert_eq!(price_field.index, Some(false));
    }

    #[test]
    fn test_schema_field_to_collection_field() {
        let field = SchemaField {
            name: "test_field".to_string(),
            field_type: FieldType::Integer,
            required: true,
            facet: true,
            sort: true,
            index: false,
        };

        let collection_field = schema_field_to_collection_field(field);
        assert_eq!(collection_field.name, "test_field");
        assert_eq!(collection_field.field_type, "int32");
        assert_eq!(collection_field.optional, Some(false));
        assert_eq!(collection_field.facet, Some(true));
        assert_eq!(collection_field.sort, Some(true));
        assert_eq!(collection_field.index, Some(false));
    }

    #[test]
    fn test_collection_field_to_schema_field() {
        let collection_field = CollectionField {
            name: "test_field".to_string(),
            field_type: "float".to_string(),
            facet: Some(true),
            index: Some(false),
            sort: Some(true),
            optional: Some(false),
        };

        let schema_field = collection_field_to_schema_field(collection_field);
        assert_eq!(schema_field.name, "test_field");
        assert_eq!(schema_field.field_type, FieldType::Float);
        assert!(schema_field.required);
        assert!(schema_field.facet);
        assert!(schema_field.sort);
        assert!(!schema_field.index);
    }

    #[test]
    fn test_typesense_schema_to_schema() {
        let typesense_schema = CollectionSchema {
            name: "test_collection".to_string(),
            fields: vec![
                CollectionField {
                    name: "title".to_string(),
                    field_type: "string".to_string(),
                    facet: Some(false),
                    index: Some(true),
                    sort: Some(false),
                    optional: Some(true),
                },
                CollectionField {
                    name: "price".to_string(),
                    field_type: "float".to_string(),
                    facet: Some(true),
                    index: Some(false),
                    sort: Some(true),
                    optional: Some(false),
                },
            ],
            default_sorting_field: Some("price".to_string()),
            enable_nested_fields: None,
            token_separators: None,
            symbols_to_index: None,
        };

        let schema = _typesense_schema_to_schema(typesense_schema);
        assert_eq!(schema.primary_key, Some("price".to_string()));
        assert_eq!(schema.fields.len(), 2);

        let title_field = schema.fields.iter().find(|f| f.name == "title").unwrap();
        assert_eq!(title_field.field_type, FieldType::Text);
        assert!(!title_field.required);
        assert!(title_field.index);

        let price_field = schema.fields.iter().find(|f| f.name == "price").unwrap();
        assert_eq!(price_field.field_type, FieldType::Float);
        assert!(price_field.required);
        assert!(price_field.facet);
        assert!(price_field.sort);
    }

    #[test]
    fn test_typesense_response_to_search_results() {
        let mut hit1 = Map::new();
        hit1.insert("id".to_string(), Value::String("doc1".to_string()));
        hit1.insert(
            "title".to_string(),
            Value::String("Test Document 1".to_string()),
        );

        let mut hit2 = Map::new();
        hit2.insert("id".to_string(), Value::String("doc2".to_string()));
        hit2.insert(
            "title".to_string(),
            Value::String("Test Document 2".to_string()),
        );

        let typesense_response = SearchResponse {
            hits: vec![
                TypesenseSearchHit {
                    document: hit1,
                    text_match: Some(1000000),
                    text_match_info: None,
                    highlight: None,
                    highlights: None,
                },
                TypesenseSearchHit {
                    document: hit2,
                    text_match: Some(800000),
                    text_match_info: None,
                    highlight: None,
                    highlights: None,
                },
            ],
            found: 2,
            found_docs: Some(2),
            out_of: 2,
            page: 1,
            request_params: RequestParams {
                collection_name: "test".to_string(),
                per_page: 20,
                q: "test".to_string(),
            },
            search_time_ms: 5,
            search_cutoff: Some(false),
            facet_counts: Some(vec![FacetCount {
                field_name: "category".to_string(),
                counts: vec![
                    FacetValue {
                        count: 1,
                        highlighted: Some("electronics".to_string()),
                        value: Value::String("electronics".to_string()),
                    },
                    FacetValue {
                        count: 1,
                        highlighted: Some("books".to_string()),
                        value: Value::String("books".to_string()),
                    },
                ],
                stats: None,
            }]),
        };

        let search_results = typesense_response_to_search_results(typesense_response);
        assert_eq!(search_results.total, Some(2));
        assert_eq!(search_results.page, Some(1));
        assert_eq!(search_results.per_page, Some(20));
        assert_eq!(search_results.hits.len(), 2);
        assert_eq!(search_results.hits[0].id, "doc1");
        assert_eq!(search_results.hits[0].score, Some(1.0));
        assert_eq!(search_results.hits[1].id, "doc2");
        assert_eq!(search_results.hits[1].score, Some(0.8));
        assert!(search_results.facets.is_some());
        assert_eq!(search_results.took_ms, Some(5));
    }

    #[test]
    fn test_typesense_hit_to_search_hit() {
        let mut document = Map::new();
        document.insert("id".to_string(), Value::String("doc1".to_string()));
        document.insert(
            "title".to_string(),
            Value::String("Test Document".to_string()),
        );

        let highlight = {
            let mut h = Map::new();
            h.insert(
                "title".to_string(),
                Value::String("Test <mark>Document</mark>".to_string()),
            );
            h
        };

        let typesense_hit = TypesenseSearchHit {
            document,
            text_match: Some(1500000),
            text_match_info: None,
            highlight: Some(highlight),
            highlights: None,
        };

        let search_hit = typesense_hit_to_search_hit(typesense_hit);
        assert_eq!(search_hit.id, "doc1");
        assert_eq!(search_hit.score, Some(1.5));
        assert!(search_hit.content.is_some());
        assert!(search_hit.highlights.is_some());
        let highlights = search_hit.highlights.unwrap();
        assert!(highlights.contains("Test <mark>Document</mark>"));
    }

    #[test]
    fn test_typesense_hit_to_search_hit_nested_document() {
        let mut nested_doc = Map::new();
        nested_doc.insert("id".to_string(), Value::String("doc1".to_string()));
        nested_doc.insert(
            "title".to_string(),
            Value::String("Test Document".to_string()),
        );

        let mut document = Map::new();
        document.insert("document".to_string(), Value::Object(nested_doc));

        let typesense_hit = TypesenseSearchHit {
            document,
            text_match: Some(1000000),
            text_match_info: None,
            highlight: None,
            highlights: None,
        };

        let search_hit = typesense_hit_to_search_hit(typesense_hit);
        assert_eq!(search_hit.id, "doc1");
        assert_eq!(search_hit.score, Some(1.0));
        assert!(search_hit.content.is_some());
        let content = search_hit.content.unwrap();
        assert!(content.contains("Test Document"));
        assert!(!content.contains("\"id\":"));
    }

    #[test]
    fn test_typesense_hit_with_highlights_array() {
        let mut document = Map::new();
        document.insert("id".to_string(), Value::String("doc1".to_string()));
        document.insert(
            "title".to_string(),
            Value::String("Test Document".to_string()),
        );

        let highlights = vec![Value::Object({
            let mut h = Map::new();
            h.insert("field".to_string(), Value::String("title".to_string()));
            h.insert(
                "snippet".to_string(),
                Value::String("Test <mark>Document</mark>".to_string()),
            );
            h
        })];

        let typesense_hit = TypesenseSearchHit {
            document,
            text_match: Some(1000000),
            text_match_info: None,
            highlight: None,
            highlights: Some(highlights),
        };

        let search_hit = typesense_hit_to_search_hit(typesense_hit);
        assert_eq!(search_hit.id, "doc1");
        assert!(search_hit.highlights.is_some());
        let highlights_str = search_hit.highlights.unwrap();
        assert!(highlights_str.contains("Test <mark>Document</mark>"));
    }
}

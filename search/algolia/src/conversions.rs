use crate::client::{
    AlgoliaObject, IndexSettings, SearchHit as AlgoliaSearchHit, SearchQuery as AlgoliaSearchQuery,
    SearchResponse,
};
use golem_search::golem::search::types::{
    Doc, FieldType, Schema, SchemaField, SearchHit, SearchQuery, SearchResults,
};
use serde_json::{Map, Value};

pub fn doc_to_algolia_object(doc: Doc) -> Result<AlgoliaObject, String> {
    let content: Value = serde_json::from_str(&doc.content)
        .map_err(|e| format!("Failed to parse document content as JSON: {e}"))?;

    Ok(AlgoliaObject {
        object_id: Some(doc.id),
        content,
    })
}

pub fn algolia_object_to_doc(obj: AlgoliaObject) -> Doc {
    let content = serde_json::to_string(&obj.content).unwrap_or_else(|_| "{}".to_string());
    Doc {
        id: obj.object_id.unwrap_or_else(|| "unknown".to_string()),
        content,
    }
}

pub fn search_query_to_algolia_query(query: SearchQuery) -> AlgoliaSearchQuery {
    let mut algolia_query = AlgoliaSearchQuery {
        query: query.q,
        filters: None,
        numeric_filters: None,
        page: query.page,
        hits_per_page: query.per_page,
        offset: query.offset,
        length: None,
        facets: query.facets,
        attributes_to_retrieve: vec![],
        typo_tolerance: None,
        analytics: Some(false),
    };

    // Handle filters - Algolia uses the filters field for general attribute filtering
    if !query.filters.is_empty() {
        // Each filter should be in the format "attribute:value" or "attribute>value", etc.
        algolia_query.filters = Some(query.filters.join(" AND "));
    }

    // Handle sort - convert to Algolia's ranking format
    if !query.sort.is_empty() {
        // Note: Algolia handles sorting differently via index replicas or custom ranking
        // For now, we'll include this in the provider params if available
    }

    // Note: Algolia handles highlighting automatically in the index settings
    // and returns _highlightResult in search responses. Query-level highlight
    // parameters are not supported in the search API.
    if let Some(_highlight) = query.highlight {
        // Highlighting configuration would need to be set at the index level
        // For now, we acknowledge but ignore highlight settings
    }

    if let Some(config) = query.config {
        algolia_query.attributes_to_retrieve = config.attributes_to_retrieve;
        algolia_query.typo_tolerance = config.typo_tolerance;

        if let Some(provider_params) = config.provider_params {
            if let Ok(params_map) = serde_json::from_str::<Map<String, Value>>(&provider_params) {
                if let Some(filters) = params_map.get("filters").and_then(|v| v.as_str()) {
                    algolia_query.filters = Some(filters.to_string());
                }
                if let Some(numeric_filters) = params_map.get("numericFilters") {
                    algolia_query.numeric_filters = Some(numeric_filters.clone());
                }
                if let Some(analytics) = params_map.get("analytics").and_then(|v| v.as_bool()) {
                    algolia_query.analytics = Some(analytics);
                }
            }
        }
    }

    algolia_query
}

pub fn algolia_response_to_search_results(response: SearchResponse) -> SearchResults {
    let hits = response
        .hits
        .into_iter()
        .map(algolia_hit_to_search_hit)
        .collect();

    SearchResults {
        total: Some(response.nb_hits),
        page: Some(response.page),
        per_page: Some(response.hits_per_page),
        hits,
        facets: response
            .facets
            .map(|f| serde_json::to_string(&f).unwrap_or_default()),
        took_ms: Some(response.processing_time_ms),
    }
}

pub fn algolia_hit_to_search_hit(hit: AlgoliaSearchHit) -> SearchHit {
    let highlights = hit
        .highlight_result
        .map(|h| serde_json::to_string(&h).unwrap_or_default());

    let score = hit.ranking_info.as_ref().map(|info| info.user_score as f64);

    SearchHit {
        id: hit.object_id,
        score,
        content: Some(serde_json::to_string(&hit.content).unwrap_or_else(|_| "{}".to_string())),
        highlights,
    }
}

pub fn schema_to_algolia_settings(schema: Schema) -> IndexSettings {
    let mut settings = IndexSettings::default();

    for field in schema.fields {
        match field.field_type {
            FieldType::Text => {
                if field.index {
                    settings.searchable_attributes.push(field.name.clone());
                }
                if field.facet {
                    settings
                        .attributes_for_faceting
                        .push(format!("filterOnly({})", field.name));
                }
            }
            FieldType::Keyword => {
                if field.facet {
                    settings.attributes_for_faceting.push(field.name.clone());
                }
                if field.index {
                    settings.searchable_attributes.push(field.name.clone());
                }
            }
            FieldType::Integer | FieldType::Float => {
                if field.facet {
                    settings.attributes_for_faceting.push(field.name.clone());
                }
            }
            FieldType::Boolean => {
                if field.facet {
                    settings.attributes_for_faceting.push(field.name.clone());
                }
            }
            FieldType::Date => {
                if field.facet {
                    settings.attributes_for_faceting.push(field.name.clone());
                }
            }
            FieldType::GeoPoint => {
                // Algolia has built-in geo support
                if field.facet {
                    settings.attributes_for_faceting.push(field.name.clone());
                }
            }
        }

        // Handle sorting - in Algolia, sorting is done via custom ranking
        if field.sort {
            settings
                .custom_ranking
                .push(format!("desc({})", field.name));
        }
    }

    settings
}

pub fn algolia_settings_to_schema(settings: IndexSettings) -> Schema {
    let mut fields = Vec::new();

    // Convert searchable attributes to text fields
    for attr in settings.searchable_attributes {
        fields.push(SchemaField {
            name: attr,
            field_type: FieldType::Text,
            required: false,
            facet: false,
            sort: false,
            index: true,
        });
    }

    // Convert faceting attributes to faceted fields
    for attr in settings.attributes_for_faceting {
        // Remove Algolia-specific prefixes like "filterOnly(field)"
        let field_name = if attr.starts_with("filterOnly(") && attr.ends_with(')') {
            attr.trim_start_matches("filterOnly(").trim_end_matches(')')
        } else if attr.starts_with("searchable(") && attr.ends_with(')') {
            attr.trim_start_matches("searchable(").trim_end_matches(')')
        } else {
            &attr
        };

        if let Some(existing_field) = fields.iter_mut().find(|f| f.name == field_name) {
            existing_field.facet = true;
        } else {
            fields.push(SchemaField {
                name: field_name.to_string(),
                field_type: FieldType::Keyword, // Default for faceting
                required: false,
                facet: true,
                sort: false,
                index: false,
            });
        }
    }

    // Convert custom ranking to sortable fields
    for ranking_rule in settings.custom_ranking {
        if let Some(field_name) = extract_field_from_ranking(&ranking_rule) {
            if let Some(existing_field) = fields.iter_mut().find(|f| f.name == field_name) {
                existing_field.sort = true;
            } else {
                fields.push(SchemaField {
                    name: field_name,
                    field_type: FieldType::Integer, // Default for sorting
                    required: false,
                    facet: false,
                    sort: true,
                    index: false,
                });
            }
        }
    }

    Schema {
        fields,
        primary_key: None,
    }
}

fn extract_field_from_ranking(ranking_rule: &str) -> Option<String> {
    if let Some(start) = ranking_rule.find('(') {
        if let Some(end) = ranking_rule.rfind(')') {
            if start < end {
                return Some(ranking_rule[start + 1..end].to_string());
            }
        }
    }
    None
}

pub fn create_retry_query(original_query: &SearchQuery, partial_hits: &[SearchHit]) -> SearchQuery {
    let mut retry_query = original_query.clone();

    if !partial_hits.is_empty() {
        if let Some(current_page) = retry_query.page {
            retry_query.page = Some(current_page + 1);
        } else if let Some(current_offset) = retry_query.offset {
            let hits_received = partial_hits.len() as u32;
            retry_query.offset = Some(current_offset + hits_received);
        } else {
            retry_query.offset = Some(partial_hits.len() as u32);
        }
    }

    retry_query
}

#[cfg(test)]
mod tests {
    use super::*;
    use golem_search::golem::search::types::{HighlightConfig, SearchConfig};

    #[test]
    fn test_doc_to_algolia_object() {
        let doc = Doc {
            id: "test-id".to_string(),
            content: r#"{"title": "Test Document", "content": "This is a test"}"#.to_string(),
        };

        let algolia_obj = doc_to_algolia_object(doc).unwrap();
        assert_eq!(algolia_obj.object_id, Some("test-id".to_string()));
        assert_eq!(algolia_obj.content["title"], "Test Document");
        assert_eq!(algolia_obj.content["content"], "This is a test");
    }

    #[test]
    fn test_doc_to_algolia_object_invalid_json() {
        let doc = Doc {
            id: "test-id".to_string(),
            content: "invalid json".to_string(),
        };

        let result = doc_to_algolia_object(doc);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Failed to parse document content as JSON"));
    }

    #[test]
    fn test_algolia_object_to_doc() {
        let algolia_obj = AlgoliaObject {
            object_id: Some("test-id".to_string()),
            content: serde_json::json!({
                "title": "Test Document",
                "content": "This is a test"
            }),
        };

        let doc = algolia_object_to_doc(algolia_obj);
        assert_eq!(doc.id, "test-id");
        assert!(doc.content.contains("Test Document"));
        assert!(doc.content.contains("This is a test"));
    }

    #[test]
    fn test_algolia_object_to_doc_no_id() {
        let algolia_obj = AlgoliaObject {
            object_id: None,
            content: serde_json::json!({"title": "Test"}),
        };

        let doc = algolia_object_to_doc(algolia_obj);
        assert_eq!(doc.id, "unknown");
    }

    #[test]
    fn test_search_query_conversion() {
        let search_query = SearchQuery {
            q: Some("test query".to_string()),
            filters: vec!["category:electronics".to_string(), "price:>100".to_string()],
            sort: vec!["price:desc".to_string()],
            facets: vec!["category".to_string(), "brand".to_string()],
            page: Some(1),
            per_page: Some(20),
            offset: None,
            highlight: Some(HighlightConfig {
                fields: vec!["title".to_string(), "description".to_string()],
                pre_tag: Some("<mark>".to_string()),
                post_tag: Some("</mark>".to_string()),
                max_length: Some(200),
            }),
            config: None,
        };

        let algolia_query = search_query_to_algolia_query(search_query);
        assert_eq!(algolia_query.query, Some("test query".to_string()));
        assert_eq!(
            algolia_query.filters,
            Some("category:electronics AND price:>100".to_string())
        );
        assert_eq!(
            algolia_query.facets,
            vec!["category".to_string(), "brand".to_string()]
        );
        assert_eq!(algolia_query.page, Some(1));
        assert_eq!(algolia_query.hits_per_page, Some(20));
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
                timeout_ms: None,
                boost_fields: vec![],
                exact_match_boost: None,
                language: None,
                provider_params: Some(
                    r#"{"analytics": true, "numericFilters": ["price>100"]}"#.to_string(),
                ),
            }),
        };

        let algolia_query = search_query_to_algolia_query(search_query);
        assert_eq!(
            algolia_query.attributes_to_retrieve,
            vec!["title".to_string(), "price".to_string()]
        );
        assert_eq!(algolia_query.typo_tolerance, Some(false));
        assert_eq!(algolia_query.analytics, Some(true));
    }

    #[test]
    fn test_schema_conversion() {
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

        let settings = schema_to_algolia_settings(schema);
        assert!(settings
            .searchable_attributes
            .contains(&"title".to_string()));
        assert!(settings
            .searchable_attributes
            .contains(&"category".to_string()));
        assert!(settings
            .attributes_for_faceting
            .contains(&"category".to_string()));
        assert!(settings
            .attributes_for_faceting
            .contains(&"price".to_string()));
        assert!(settings.custom_ranking.contains(&"desc(price)".to_string()));
    }

    #[test]
    fn test_algolia_response_conversion() {
        let algolia_response = SearchResponse {
            hits: vec![AlgoliaSearchHit {
                object_id: "doc1".to_string(),
                content: serde_json::json!({"title": "Test Document 1"}),
                highlight_result: Some(
                    serde_json::json!({"title": {"value": "Test <em>Document</em> 1"}}),
                ),
                snippet_result: None,
                ranking_info: Some(crate::client::RankingInfo {
                    nb_typos: 0,
                    first_matched_word: 0,
                    proximity_distance: 0,
                    user_score: 100,
                    geo_distance: 0,
                    geo_precision: 0,
                    nb_exact_words: 1,
                    words: 1,
                    filters: 0,
                }),
            }],
            page: 0,
            nb_hits: 1,
            nb_pages: 1,
            hits_per_page: 20,
            processing_time_ms: 5,
            facets: Some(serde_json::json!({"category": {"electronics": 1}})),
            facets_stats: None,
            exhaustive_nb_hits: true,
            exhaustive_facets_count: true,
            query: "test".to_string(),
            params: "q=test".to_string(),
        };

        let search_results = algolia_response_to_search_results(algolia_response);
        assert_eq!(search_results.total, Some(1));
        assert_eq!(search_results.page, Some(0));
        assert_eq!(search_results.per_page, Some(20));
        assert_eq!(search_results.hits.len(), 1);
        assert_eq!(search_results.hits[0].id, "doc1");
        assert_eq!(search_results.hits[0].score, Some(100.0));
        assert!(search_results.facets.is_some());
        assert_eq!(search_results.took_ms, Some(5));
    }

    #[test]
    fn test_create_retry_query() {
        let original_query = SearchQuery {
            q: Some("test".to_string()),
            filters: vec![],
            sort: vec![],
            facets: vec![],
            page: Some(1),
            per_page: Some(10),
            offset: None,
            highlight: None,
            config: None,
        };

        let partial_hits = vec![SearchHit {
            id: "doc1".to_string(),
            score: Some(1.0),
            content: Some("{}".to_string()),
            highlights: None,
        }];

        let retry_query = create_retry_query(&original_query, &partial_hits);
        assert_eq!(retry_query.page, Some(2));
    }

    #[test]
    fn test_create_retry_query_with_offset() {
        let original_query = SearchQuery {
            q: Some("test".to_string()),
            filters: vec![],
            sort: vec![],
            facets: vec![],
            page: None,
            per_page: Some(10),
            offset: Some(20),
            highlight: None,
            config: None,
        };

        let partial_hits = vec![SearchHit {
            id: "doc1".to_string(),
            score: Some(1.0),
            content: Some("{}".to_string()),
            highlights: None,
        }];

        let retry_query = create_retry_query(&original_query, &partial_hits);
        assert_eq!(retry_query.offset, Some(21));
    }

    #[test]
    fn test_extract_field_from_ranking() {
        assert_eq!(
            extract_field_from_ranking("desc(price)"),
            Some("price".to_string())
        );
        assert_eq!(
            extract_field_from_ranking("asc(created_at)"),
            Some("created_at".to_string())
        );
        assert_eq!(extract_field_from_ranking("invalid"), None);
        assert_eq!(extract_field_from_ranking("desc()"), Some("".to_string()));
    }
}

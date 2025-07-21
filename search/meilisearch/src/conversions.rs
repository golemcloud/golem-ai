use crate::client::{
    MeilisearchDocument, MeilisearchSearchRequest, MeilisearchSearchResponse, MeilisearchSettings,
};
use golem_search::golem::search::types::{
    Doc, FieldType, Schema, SchemaField, SearchHit, SearchQuery, SearchResults,
};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::HashMap;

pub fn doc_to_meilisearch_document(doc: Doc) -> Result<MeilisearchDocument, String> {
    let mut meilisearch_doc = JsonMap::new();

    meilisearch_doc.insert("id".to_string(), JsonValue::String(doc.id.clone()));

    if let Ok(JsonValue::Object(content_map)) = serde_json::from_str::<JsonValue>(&doc.content) {
        for (key, value) in content_map {
            meilisearch_doc.insert(key, value);
        }
    }

    Ok(meilisearch_doc)
}

pub fn meilisearch_document_to_doc(mut doc: MeilisearchDocument) -> Doc {
    let id = doc
        .remove("id")
        .and_then(|v| match v {
            JsonValue::String(s) => Some(s),
            JsonValue::Number(n) => Some(n.to_string()),
            _ => None,
        })
        .unwrap_or_else(|| "unknown".to_string());

    let content =
        serde_json::to_string(&JsonValue::Object(doc)).unwrap_or_else(|_| "{}".to_string());

    Doc { id, content }
}

pub fn search_query_to_meilisearch_request(query: SearchQuery) -> MeilisearchSearchRequest {
    let mut request = MeilisearchSearchRequest {
        q: query.q,
        offset: query.offset,
        limit: query.per_page,
        filter: None,
        facets: if query.facets.is_empty() {
            None
        } else {
            Some(query.facets)
        },
        sort: if query.sort.is_empty() {
            None
        } else {
            Some(query.sort)
        },
        attributes_to_retrieve: query.config.as_ref().and_then(|c| {
            serde_json::from_str::<JsonValue>(c.provider_params.as_ref()?)
                .ok()
                .and_then(|v| {
                    v.get("attributes_to_retrieve")
                        .and_then(|a| a.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                })
        }),
        attributes_to_highlight: None,
        attributes_to_crop: None,
        crop_length: None,
        show_matches_position: None,
        matching_strategy: None,
        show_ranking_score: None,
    };

    if !query.filters.is_empty() {
        request.filter = Some(convert_filters_to_meilisearch(query.filters));
    }

    request
}

pub fn meilisearch_response_to_search_results(
    response: MeilisearchSearchResponse,
) -> SearchResults {
    let hits: Vec<SearchHit> = response
        .hits
        .into_iter()
        .map(|doc| {
            let converted_doc = meilisearch_document_to_doc(doc.clone());
            SearchHit {
                id: converted_doc.id,
                score: None,
                content: Some(converted_doc.content),
                highlights: None,
            }
        })
        .collect();

    SearchResults {
        total: Some(response.estimated_total_hits),
        page: None, // We'd need to calculate this from offset and limit
        per_page: Some(response.limit),
        hits,
        facets: response
            .facet_distribution
            .map(|facets| serde_json::to_string(&facets).unwrap_or_default()),
        took_ms: Some(response.processing_time_ms),
    }
}

pub fn schema_to_meilisearch_settings(schema: Schema) -> MeilisearchSettings {
    let mut settings = MeilisearchSettings::default();

    let mut searchable_attributes = Vec::new();
    let mut filterable_attributes = Vec::new();
    let mut sortable_attributes = Vec::new();

    for field in schema.fields {
        if field.index {
            searchable_attributes.push(field.name.clone());
        }

        if field.facet {
            filterable_attributes.push(field.name.clone());
        }

        if field.sort {
            sortable_attributes.push(field.name.clone());
        }
    }

    if !searchable_attributes.is_empty() {
        settings.searchable_attributes = Some(searchable_attributes);
    }

    if !filterable_attributes.is_empty() {
        settings.filterable_attributes = Some(filterable_attributes);
    }

    if !sortable_attributes.is_empty() {
        settings.sortable_attributes = Some(sortable_attributes);
    }

    settings
}

pub fn meilisearch_settings_to_schema(settings: MeilisearchSettings) -> Schema {
    let mut fields = Vec::new();

    let mut field_names = std::collections::BTreeSet::new();

    if let Some(searchable) = &settings.searchable_attributes {
        field_names.extend(searchable.iter().cloned());
    }
    if let Some(filterable) = &settings.filterable_attributes {
        field_names.extend(filterable.iter().cloned());
    }
    if let Some(sortable) = &settings.sortable_attributes {
        field_names.extend(sortable.iter().cloned());
    }
    if let Some(displayed) = &settings.displayed_attributes {
        field_names.extend(displayed.iter().cloned());
    }

    if field_names.is_empty() {
        fields.push(SchemaField {
            name: "*".to_string(),
            field_type: FieldType::Text,
            required: false,
            facet: false,
            sort: false,
            index: true,
        });
    } else {
        for field_name in field_names {
            let index = settings
                .searchable_attributes
                .as_ref()
                .map(|attrs| attrs.contains(&field_name))
                .unwrap_or(true);

            let facet = settings
                .filterable_attributes
                .as_ref()
                .map(|attrs| attrs.contains(&field_name))
                .unwrap_or(false);

            let sort = settings
                .sortable_attributes
                .as_ref()
                .map(|attrs| attrs.contains(&field_name))
                .unwrap_or(false);

            fields.push(SchemaField {
                name: field_name,
                field_type: FieldType::Text,
                required: false,
                facet,
                sort,
                index,
            });
        }
    }

    Schema {
        fields,
        primary_key: None,
    }
}

pub fn create_retry_query(original_query: &SearchQuery, partial_hits: &[SearchHit]) -> SearchQuery {
    let mut retry_query = original_query.clone();

    let current_offset = original_query.offset.unwrap_or(0);
    let per_page = original_query.per_page.unwrap_or(20);
    let hits_received = partial_hits.len() as u32;

    if hits_received < per_page {
        retry_query.offset = Some(current_offset + hits_received);
    } else {
        retry_query.offset = Some(current_offset + per_page);
    }

    retry_query
}

fn convert_filters_to_meilisearch(filters: Vec<String>) -> String {
    // Join multiple filters with AND
    // In Meilisearch, filter syntax supports expressions like:
    // "genre = horror AND year > 2000"
    // "color = red OR color = blue"
    filters.join(" AND ")
}

// for later development :-
fn _convert_meilisearch_facets_to_golem(
    facets: JsonMap<String, JsonValue>,
) -> HashMap<String, HashMap<String, u64>> {
    let mut result = HashMap::new();

    for (facet_name, facet_value) in facets {
        if let JsonValue::Object(facet_map) = facet_value {
            let mut facet_counts = HashMap::new();
            for (value, count) in facet_map {
                if let JsonValue::Number(n) = count {
                    if let Some(count_u64) = n.as_u64() {
                        facet_counts.insert(value, count_u64);
                    }
                }
            }
            result.insert(facet_name, facet_counts);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use golem_search::golem::search::types::{HighlightConfig, SearchConfig};
    use serde_json::json;

    #[test]
    fn test_doc_to_meilisearch_document() {
        let doc = Doc {
            id: "test-id".to_string(),
            content: r#"{"title": "Test Document", "content": "This is a test"}"#.to_string(),
        };

        let meilisearch_doc = doc_to_meilisearch_document(doc).unwrap();
        assert_eq!(meilisearch_doc.get("id").unwrap(), "test-id");
        assert_eq!(meilisearch_doc.get("title").unwrap(), "Test Document");
        assert_eq!(meilisearch_doc.get("content").unwrap(), "This is a test");
    }

    #[test]
    fn test_doc_to_meilisearch_document_invalid_json() {
        let doc = Doc {
            id: "test-id".to_string(),
            content: "invalid json".to_string(),
        };

        let result = doc_to_meilisearch_document(doc);
        assert!(result.is_ok()); // Should still work, just with id field
        let meilisearch_doc = result.unwrap();
        assert_eq!(meilisearch_doc.get("id").unwrap(), "test-id");
    }

    #[test]
    fn test_meilisearch_document_to_doc() {
        let meilisearch_doc = json!({
            "id": "test-id",
            "title": "Test Document",
            "content": "This is a test"
        })
        .as_object()
        .unwrap()
        .clone();

        let doc = meilisearch_document_to_doc(meilisearch_doc);
        assert_eq!(doc.id, "test-id");
        assert!(doc.content.contains("Test Document"));
        assert!(doc.content.contains("This is a test"));
    }

    #[test]
    fn test_meilisearch_document_to_doc_no_id() {
        let meilisearch_doc = json!({
            "title": "Test"
        })
        .as_object()
        .unwrap()
        .clone();

        let doc = meilisearch_document_to_doc(meilisearch_doc);
        assert_eq!(doc.id, "unknown");
    }

    #[test]
    fn test_meilisearch_document_to_doc_numeric_id() {
        let meilisearch_doc = json!({
            "id": 123,
            "title": "Test"
        })
        .as_object()
        .unwrap()
        .clone();

        let doc = meilisearch_document_to_doc(meilisearch_doc);
        assert_eq!(doc.id, "123");
    }

    #[test]
    fn test_search_query_to_meilisearch_request() {
        let search_query = SearchQuery {
            q: Some("test query".to_string()),
            filters: vec![
                "category = electronics".to_string(),
                "price > 100".to_string(),
            ],
            sort: vec!["price:desc".to_string()],
            facets: vec!["category".to_string(), "brand".to_string()],
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

        let meilisearch_request = search_query_to_meilisearch_request(search_query);
        assert_eq!(meilisearch_request.q, Some("test query".to_string()));
        assert_eq!(
            meilisearch_request.filter,
            Some("category = electronics AND price > 100".to_string())
        );
        assert_eq!(
            meilisearch_request.sort,
            Some(vec!["price:desc".to_string()])
        );
        assert_eq!(
            meilisearch_request.facets,
            Some(vec!["category".to_string(), "brand".to_string()])
        );
        assert_eq!(meilisearch_request.limit, Some(20));
        assert_eq!(meilisearch_request.offset, Some(10));
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
                    r#"{"attributes_to_retrieve": ["title", "price"]}"#.to_string(),
                ),
            }),
        };

        let meilisearch_request = search_query_to_meilisearch_request(search_query);
        assert_eq!(
            meilisearch_request.attributes_to_retrieve,
            Some(vec!["title".to_string(), "price".to_string()])
        );
    }

    #[test]
    fn test_schema_to_meilisearch_settings() {
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

        let settings = schema_to_meilisearch_settings(schema);
        assert_eq!(
            settings.searchable_attributes,
            Some(vec!["title".to_string(), "category".to_string()])
        );
        assert_eq!(
            settings.filterable_attributes,
            Some(vec!["category".to_string(), "price".to_string()])
        );
        assert_eq!(
            settings.sortable_attributes,
            Some(vec!["price".to_string()])
        );
    }

    #[test]
    fn test_meilisearch_settings_to_schema() {
        let settings = MeilisearchSettings {
            searchable_attributes: Some(vec!["title".to_string(), "content".to_string()]),
            filterable_attributes: Some(vec!["category".to_string(), "price".to_string()]),
            sortable_attributes: Some(vec!["price".to_string(), "created_at".to_string()]),
            displayed_attributes: Some(vec!["title".to_string(), "description".to_string()]),
            ..Default::default()
        };

        let schema = meilisearch_settings_to_schema(settings);
        assert!(!schema.fields.is_empty());

        let title_field = schema.fields.iter().find(|f| f.name == "title").unwrap();
        assert!(title_field.index);

        let category_field = schema.fields.iter().find(|f| f.name == "category").unwrap();
        assert!(category_field.facet);

        let price_field = schema.fields.iter().find(|f| f.name == "price").unwrap();
        assert!(price_field.facet);
        assert!(price_field.sort);
    }

    #[test]
    fn test_meilisearch_response_to_search_results() {
        let hit1 = json!({
            "id": "doc1",
            "title": "Test Document 1"
        })
        .as_object()
        .unwrap()
        .clone();

        let hit2 = json!({
            "id": "doc2",
            "title": "Test Document 2"
        })
        .as_object()
        .unwrap()
        .clone();

        let facet_distribution = json!({
            "category": {
                "electronics": 1,
                "books": 1
            }
        })
        .as_object()
        .unwrap()
        .clone();

        let meilisearch_response = MeilisearchSearchResponse {
            hits: vec![hit1, hit2],
            estimated_total_hits: 2,
            limit: 20,
            offset: 0,
            processing_time_ms: 5,
            facet_distribution: Some(facet_distribution),
            query: "test".to_string(),
        };

        let search_results = meilisearch_response_to_search_results(meilisearch_response);
        assert_eq!(search_results.total, Some(2));
        assert_eq!(search_results.per_page, Some(20));
        assert_eq!(search_results.hits.len(), 2);
        assert_eq!(search_results.hits[0].id, "doc1");
        assert_eq!(search_results.hits[1].id, "doc2");
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
        assert_eq!(retry_query.offset, Some(21)); // 20 + 1 hit received
    }

    #[test]
    fn test_create_retry_query_full_page() {
        let original_query = SearchQuery {
            q: Some("test".to_string()),
            filters: vec![],
            sort: vec![],
            facets: vec![],
            page: None,
            per_page: Some(2),
            offset: Some(10),
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
                score: Some(1.0),
                content: Some("{}".to_string()),
                highlights: None,
            },
        ];

        let retry_query = create_retry_query(&original_query, &partial_hits);
        assert_eq!(retry_query.offset, Some(12)); // 10 + 2 (per_page)
    }

    #[test]
    fn test_convert_filters_to_meilisearch() {
        let filters = vec![
            "category = electronics".to_string(),
            "price > 100".to_string(),
        ];
        let meilisearch_filter = convert_filters_to_meilisearch(filters);
        assert_eq!(meilisearch_filter, "category = electronics AND price > 100");
    }

    #[test]
    fn test_convert_meilisearch_facets_to_golem() {
        let facets = json!({
            "category": {
                "electronics": 5,
                "books": 3
            }
        })
        .as_object()
        .unwrap()
        .clone();

        let golem_facets = _convert_meilisearch_facets_to_golem(facets);
        assert_eq!(golem_facets.len(), 1);
        assert_eq!(
            golem_facets.get("category").unwrap().get("electronics"),
            Some(&5)
        );
        assert_eq!(golem_facets.get("category").unwrap().get("books"), Some(&3));
    }
}

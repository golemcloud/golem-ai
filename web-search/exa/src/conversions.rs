use crate::client::{ExaSearchRequest, ExaSearchResponse, ExaSearchContents, ExaHighlightsOptions};
use golem_web_search::golem::web_search::types::{ImageResult, SearchMetadata, SearchParams, SearchResult};

pub fn convert_params_to_request(
    params: &SearchParams,
    page_offset: Option<u32>,
) -> ExaSearchRequest {
    let mut include_domains = None;
    let mut exclude_domains = None;

    if let Some(ref domains) = params.include_domains {
        if !domains.is_empty() {
            include_domains = Some(domains.clone());
        }
    }

    if let Some(ref domains) = params.exclude_domains {
        if !domains.is_empty() {
            exclude_domains = Some(domains.clone());
        }
    }

    // Implement pagination simulation for Exa
    let (num_results, search_type) = if let Some(offset) = page_offset {
        // For different "pages", we'll vary the approach to get different results
        let base_results = params.max_results.unwrap_or(10).min(20); // Smaller batches for pagination
        let adjusted_results = (base_results + offset * 2).min(100); // Gradually increase results
        
        // Alternate between search types for different pages to get variety
        let search_type = match offset % 3 {
            0 => "auto",
            1 => "neural", 
            _ => "keyword",
        };
        
        (adjusted_results, search_type)
    } else {
        (params.max_results.unwrap_or(10).min(100), "auto")
    };

    // Configure contents to include text content
    let contents = Some(ExaSearchContents {
        text: Some(true), // Always include text content
        highlights: Some(ExaHighlightsOptions {
            num_sentences: Some(2),
            highlights_per_url: Some(3),
            query: None, // Let Exa determine automatically
        }),
        summary: None, // Don't include summary by default
    });

    ExaSearchRequest {
        query: params.query.clone(),
        r#type: Some(search_type.to_string()),
        num_results: Some(num_results),
        category: None, // Could be mapped from future SearchParams extension
        include_domains,
        exclude_domains,
        start_crawl_date: None, // Could be mapped from time_range in future
        end_crawl_date: None,
        start_published_date: None,
        end_published_date: None,
        include_text: None, // Could be used for advanced filtering
        exclude_text: None,
        context: Some(false), // Don't format for LLM context by default
        contents,
    }
}

pub fn convert_response_to_results(
    response: ExaSearchResponse,
    _params: &SearchParams,
) -> (Vec<SearchResult>, Option<SearchMetadata>) {
    let results: Vec<SearchResult> = response
        .results
        .into_iter()
        .map(|exa_result| SearchResult {
            title: exa_result.title,
            url: exa_result.url.clone(),
            snippet: exa_result.text
                .or_else(|| exa_result.summary.clone())
                .unwrap_or_else(|| {
                    // If no text content, try to use highlights or create a snippet from title
                    if let Some(highlights) = &exa_result.highlights {
                        highlights.join(" ... ")
                    } else {
                        match exa_result.score {
                            Some(score) => format!("Score: {:.2}", score),
                            None => "Relevant result".to_string(),
                        }
                    }
                }),
            display_url: Some(exa_result.url.clone()),
            source: extract_domain(&exa_result.url),
            score: exa_result.score,
            html_snippet: None,
            date_published: exa_result.published_date,
            images: exa_result.image.map(|image_url| vec![ImageResult {
                url: image_url,
                description: None,
            }]),
            content_chunks: exa_result.highlights.map(|highlights| highlights),
        })
        .collect();

    let metadata = if results.is_empty() {
        None
    } else {
        Some(SearchMetadata {
            query: "".to_string(), // We don't have the original query in response
            total_results: Some(results.len() as u64),
            search_time_ms: None,
            safe_search: None,
            language: None,
            region: None,
            next_page_token: response.autoprompt_string, // Use autoprompt as page token
            rate_limits: None,
        })
    };

    (results, metadata)
}

fn extract_domain(url: &str) -> Option<String> {
    if let Ok(parsed_url) = url::Url::parse(url) {
        parsed_url.host_str().map(|host| host.to_string())
    } else {
        None
    }
}

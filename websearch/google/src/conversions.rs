use crate::client::{SearchRequest, SearchResponse, SearchResult as ClientSearchResult};
use golem_web_search::golem::web_search::types::SafeSearchLevel;
use golem_web_search::golem::web_search::web_search::{
    SearchError, SearchMetadata, SearchParams, SearchResult,
};

pub fn params_to_request(params: &SearchParams, start: u32) -> Result<SearchRequest, SearchError> {
    // Validate query
    if params.query.trim().is_empty() {
        return Err(SearchError::InvalidQuery);
    }

    // Handle domain filtering in query
    let mut query = params.query.clone();

    // Add included domains
    if let Some(include_domains) = &params.include_domains {
        if !include_domains.is_empty() {
            let site_filter = include_domains
                .iter()
                .map(|domain| format!("site:{domain}"))
                .collect::<Vec<_>>()
                .join(" OR ");
            query = format!("({site_filter}) {query}");
        }
    }

    // Add excluded domains
    if let Some(exclude_domains) = &params.exclude_domains {
        for domain in exclude_domains {
            query.push_str(&format!(" -site:{domain}"));
        }
    }

    Ok(SearchRequest {
        query,
        max_results: params.max_results,
        start: Some(start),
        safe: params.safe_search.map(|safe| match safe {
            SafeSearchLevel::Off => "off".to_string(),
            SafeSearchLevel::Medium => "medium".to_string(),
            SafeSearchLevel::High => "active".to_string(),
        }),
        lr: params.language.clone(),
        gl: params.region.clone(),
        date_restrict: None,
        site_search: None,
        site_search_filter: None,
        img_type: None,
        img_size: None,
    })
}

pub fn response_to_results(
    response: &SearchResponse,
    original_params: &SearchParams,
    current_page: u32,
) -> (Vec<SearchResult>, SearchMetadata) {
    let mut results = Vec::new();

    // Process web results - note: SearchResponse.results, not SearchResponse.web
    for (index, item) in response.results.iter().enumerate() {
        results.push(web_result_to_search_result(item, index));
    }

    let metadata = create_search_metadata(response, original_params, current_page);
    (results, metadata)
}

fn web_result_to_search_result(item: &ClientSearchResult, index: usize) -> SearchResult {
    let mut content_chunks = None;

    // Create content chunks from content
    let mut chunks = Vec::new();
    if !item.content.is_empty() {
        chunks.push(item.content.clone());
    }

    if !chunks.is_empty() {
        content_chunks = Some(chunks);
    }

    // Simple position-based scoring
    let score = 1.0 - (index as f32) * 0.05;

    SearchResult {
        title: item.title.clone(),
        url: item.url.clone(),
        snippet: item.content.clone(),
        display_url: extract_domain(&item.url),
        source: extract_domain(&item.url),
        score: Some(score.clamp(0.0, 1.0) as f64),
        html_snippet: None,
        date_published: item.published_date.clone(),
        images: None,
        content_chunks,
    }
}

fn extract_domain(url: &str) -> Option<String> {
    if let Ok(parsed_url) = url::Url::parse(url) {
        parsed_url.host_str().map(|host| {
            // Remove www. prefix if present
            if let Some(stripped) = host.strip_prefix("www.") {
                stripped.to_string()
            } else {
                host.to_string()
            }
        })
    } else {
        None
    }
}

fn create_search_metadata(
    response: &SearchResponse,
    params: &SearchParams,
    current_page: u32,
) -> SearchMetadata {
    // Create next page token if more results are available
    let next_page_token = response
        .next_page
        .as_ref()
        .map(|p| p.start_index.to_string());

    // Use the actual total_results from the response
    let total_results = response.total_results.or_else(|| {
        if response.results.len() >= (params.max_results.unwrap_or(10) as usize) {
            Some(100000u64) // Conservative estimate
        } else {
            Some(response.results.len() as u64)
        }
    });

    SearchMetadata {
        query: params.query.clone(),
        total_results,
        search_time_ms: Some(response.response_time as f64),
        safe_search: params.safe_search,
        language: params.language.clone(),
        region: params.region.clone(),
        next_page_token,
        rate_limits: None,
        current_page,
    }
}

pub fn validate_search_params(params: &SearchParams) -> Result<(), SearchError> {
    if params.query.trim().is_empty() {
        return Err(SearchError::InvalidQuery);
    }
    if let Some(max_results) = params.max_results {
        if max_results > 100 {
            return Err(SearchError::UnsupportedFeature(
                "max_results cannot exceed 100 for Google Custom Search".to_string(),
            ));
        }
    }
    if params.time_range.is_some() {
        return Err(SearchError::UnsupportedFeature(
            "time-range not supported".to_string(),
        ));
    }
    if params.include_html == Some(true) {
        return Err(SearchError::UnsupportedFeature(
            "include-html not supported".to_string(),
        ));
    }
    if params.advanced_answer == Some(true) {
        return Err(SearchError::UnsupportedFeature(
            "advanced-answer not supported".to_string(),
        ));
    }
    Ok(())
}

use crate::client::{SearchRequest, SearchResponse, WebResult};
use golem_web_search::golem::web_search::web_search::{
    SearchError, SearchMetadata, SearchParams, SearchResult,
};

pub fn params_to_request(
    params: SearchParams,
    api_key: String,
    offset: u32,
) -> Result<SearchRequest, SearchError> {
    // Validate query
    if params.query.trim().is_empty() {
        return Err(SearchError::InvalidQuery);
    }

    // Handle domain filtering in query
    let mut query = params.query.clone();
    if let Some(exclude_domains) = &params.exclude_domains {
        for domain in exclude_domains {
            query.push_str(&format!(" -site:{domain}"));
        }
    }

    Ok(SearchRequest {
        api_key,
        query,
        count: Some(params.max_results.unwrap_or(10)),
        offset: Some(offset),
    })
}

pub fn response_to_results(
    response: SearchResponse,
    original_params: &SearchParams,
    current_offset: u32,
) -> (Vec<SearchResult>, Option<SearchMetadata>) {
    let mut results = Vec::new();

    // Process web results
    if let Some(ref web_results) = response.web {
        for (index, item) in web_results.results.iter().enumerate() {
            results.push(web_result_to_search_result(item, index));
        }
    }

    let metadata = create_search_metadata(&response, original_params, current_offset);
    (results, Some(metadata))
}

fn web_result_to_search_result(item: &WebResult, index: usize) -> SearchResult {
    let mut content_chunks = None;

    // Create content chunks from description
    let mut chunks = Vec::new();
    if !item.description.is_empty() {
        chunks.push(item.description.clone());
    }

    if !chunks.is_empty() {
        content_chunks = Some(chunks);
    }

    // Simple position-based scoring
    let score = 1.0 - (index as f32) * 0.05;

    SearchResult {
        title: item.title.clone(),
        url: item.url.clone(),
        snippet: item.description.clone(),
        display_url: extract_domain(&item.url),
        source: extract_domain(&item.url),
        score: Some(score.clamp(0.0, 1.0) as f64),
        html_snippet: None,
        date_published: item.date.clone(),
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
    current_offset: u32,
) -> SearchMetadata {
    // Check if we got the full count requested
    let has_more_results = if let Some(web_results) = &response.web {
        let requested_count = params.max_results.unwrap_or(10);
        web_results.results.len() == (requested_count as usize)
    } else {
        false
    };

    // Create next page token if more results are available
    let next_page_token = if has_more_results {
        let next_offset = current_offset + params.max_results.unwrap_or(10);
        Some(next_offset.to_string())
    } else {
        None
    };

    // Simple total results estimation
    let total_results = if let Some(web_results) = &response.web {
        if web_results.results.len() >= (params.max_results.unwrap_or(10) as usize) {
            Some(100000u64) // Conservative estimate
        } else {
            Some(web_results.results.len() as u64)
        }
    } else {
        Some(0u64)
    };

    SearchMetadata {
        query: params.query.clone(),
        total_results,
        search_time_ms: None,
        safe_search: params.safe_search,
        language: params.language.clone(),
        region: params.region.clone(),
        next_page_token,
        rate_limits: None,
        current_page: current_offset,
    }
}

pub fn validate_search_params(params: &SearchParams) -> Result<(), SearchError> {
    if params.query.trim().is_empty() {
        return Err(SearchError::InvalidQuery);
    }

    if let Some(max_results) = params.max_results {
        if max_results > 20 {
            return Err(SearchError::UnsupportedFeature(
                "max_results cannot exceed 20 for Brave Search".to_string(),
            ));
        }
    }

    Ok(())
}

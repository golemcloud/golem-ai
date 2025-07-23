use crate::client::{SearchRequest, SearchResponse, SearchResult as SerperSearchResult};
use golem_web_search::golem::web_search::web_search::{
    SearchError, SearchMetadata, SearchParams, SearchResult,
};

pub fn params_to_request(params: SearchParams, page: u32) -> Result<SearchRequest, SearchError> {
    // Validate query
    if params.query.trim().is_empty() {
        return Err(SearchError::InvalidQuery);
    }

    // Convert region to Google country code
    let gl = params
        .region
        .map(|region| match region.to_lowercase().as_str() {
            "us" | "usa" | "united states" => "us".to_string(),
            "uk" | "gb" | "united kingdom" => "uk".to_string(),
            "in" | "india" => "in".to_string(),
            _ => region,
        });

    // Convert language to Google language code
    let hl = params
        .language
        .map(|lang| match lang.to_lowercase().as_str() {
            "english" | "en" => "en".to_string(),
            "spanish" | "es" => "es".to_string(),
            "french" | "fr" => "fr".to_string(),
            _ => lang,
        });

    Ok(SearchRequest {
        q: params.query.clone(),
        gl,
        hl,
        num: params.max_results,
        page: Some(page), // 1-based
    })
}

pub fn response_to_results(
    response: SearchResponse,
    original_params: &SearchParams,
    current_page: u32,
) -> (Vec<SearchResult>, SearchMetadata) {
    let mut results = Vec::new();

    // Process organic search results
    for (index, item) in response.organic.iter().enumerate() {
        results.push(serper_result_to_search_result(item, index));
    }

    let metadata = create_search_metadata(&response, original_params, current_page);
    (results, metadata)
}

fn serper_result_to_search_result(item: &SerperSearchResult, index: usize) -> SearchResult {
    // Calculate score based on position
    let score = 1.0 - (index as f32) * 0.01;

    SearchResult {
        title: item.title.clone(),
        url: item.link.clone(),
        snippet: item.snippet.clone(),
        display_url: extract_domain(&item.link),
        source: extract_domain(&item.link),
        score: Some(score as f64),
        html_snippet: None,
        date_published: None,
        images: None,
        content_chunks: Some(vec![item.snippet.clone()]),
    }
}

fn extract_domain(url: &str) -> Option<String> {
    if let Ok(parsed_url) = url::Url::parse(url) {
        parsed_url.host_str().map(|host| host.to_string())
    } else {
        None
    }
}

fn create_search_metadata(
    response: &SearchResponse,
    params: &SearchParams,
    current_page: u32,
) -> SearchMetadata {
    // Check if we got the full count requested
    let has_more_results = {
        let requested_count = params.max_results.unwrap_or(10);
        response.organic.len() == (requested_count as usize)
    };

    // Create next page token if more results are available
    let next_page_token = if has_more_results {
        let next_page = current_page + 1;
        Some(next_page.to_string())
    } else {
        None
    };

    // Estimate total results
    let total_results = if (response.organic.len() as u32) >= params.max_results.unwrap_or(10) {
        Some(100000u64) // Conservative estimate
    } else {
        Some(response.organic.len() as u64)
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
        current_page: current_page - 1, // 1-based
    }
}

pub fn validate_search_params(params: &SearchParams) -> Result<(), SearchError> {
    if params.query.trim().is_empty() {
        return Err(SearchError::InvalidQuery);
    }

    if let Some(max_results) = params.max_results {
        if max_results > 100 {
            return Err(SearchError::UnsupportedFeature(
                "max_results cannot exceed 100 for Serper Search".to_string(),
            ));
        }
    }
    if params.safe_search.is_some() {
        return Err(SearchError::UnsupportedFeature(
            "safe_search not supported".to_string(),
        ));
    }
    if params.include_html == Some(true) {
        return Err(SearchError::UnsupportedFeature(
            "include-html not supported".to_string(),
        ));
    }
    if params.time_range.is_some() {
        return Err(SearchError::UnsupportedFeature(
            "time-range not supported".to_string(),
        ));
    }
    if params.include_images == Some(true) {
        return Err(SearchError::UnsupportedFeature(
            "include-images not supported".to_string(),
        ));
    }
    if params.advanced_answer == Some(true) {
        return Err(SearchError::UnsupportedFeature(
            "advanced-answer not supported".to_string(),
        ));
    }
    Ok(())
}

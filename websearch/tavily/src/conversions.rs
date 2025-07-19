use crate::client::{SearchRequest, SearchResponse, SearchResult as TavilySearchResult};
use golem_web_search::golem::web_search::types::{ImageResult, TimeRange};
use golem_web_search::golem::web_search::web_search::{
    SearchError, SearchMetadata, SearchParams, SearchResult,
};

pub fn params_to_request(
    params: SearchParams,
    api_key: String,
    _page: u32,
) -> Result<SearchRequest, SearchError> {
    // Validate query
    if params.query.trim().is_empty() {
        return Err(SearchError::InvalidQuery);
    }

    // Determine search depth based on parameters
    let search_depth = determine_search_depth(&params);

    // Convert time range to days
    let days = params.time_range.map(|range| match range {
        TimeRange::Day => 1,
        TimeRange::Week => 7,
        TimeRange::Month => 30,
        TimeRange::Year => 365,
    });

    // Handle domain filtering
    let query = params.query.clone();

    // For exclude_domains, we'll add them to the exclude_domains parameter
    // rather than modifying the query directly
    let exclude_domains = params.exclude_domains.clone();
    let include_domains = params.include_domains.clone();

    // Note: Tavily's SearchRequest doesn't have pagination fields (page/start/offset)
    // This is a limitation of the current API structure
    Ok(SearchRequest {
        api_key,
        query,
        search_depth: Some(search_depth),
        include_images: params.include_images,
        include_answer: Some(true), // Always include answer for better results
        include_raw_content: Some(true), // Include raw content for better content chunks
        max_results: params.max_results,
        include_domains,
        exclude_domains,
        format: Some("json".to_string()),
        days,
    })
}

fn determine_search_depth(params: &SearchParams) -> String {
    // Use "advanced" search depth if we need comprehensive results
    // Use "basic" for faster, simpler searches
    if params.max_results.unwrap_or(10) > 10 || params.include_images == Some(true) {
        "advanced".to_string()
    } else {
        "basic".to_string()
    }
}

pub fn response_to_results(
    response: SearchResponse,
    original_params: &SearchParams,
    current_page: u32,
) -> (Vec<SearchResult>, Option<SearchMetadata>) {
    let mut results = Vec::new();

    // Process main search results
    for (index, item) in response.results.iter().enumerate() {
        results.push(tavily_result_to_search_result(
            item,
            index,
            original_params.include_images.unwrap_or(false),
            &response.images,
        ));
    }

    // If we have an answer, create a special result for it
    if let Some(answer) = &response.answer {
        let answer_result = SearchResult {
            title: "AI-Generated Answer".to_string(),
            url: "https://tavily.com".to_string(), // Placeholder URL
            snippet: answer.clone(),
            display_url: Some("tavily.com".to_string()),
            source: Some("Tavily AI".to_string()),
            score: Some(1.0), // Highest score for AI answer
            html_snippet: None,
            date_published: None,
            images: None,
            content_chunks: Some(vec![answer.clone()]),
        };

        // Insert at the beginning
        results.insert(0, answer_result);
    }

    let metadata = create_search_metadata(&response, original_params, current_page);
    (results, Some(metadata))
}

fn tavily_result_to_search_result(
    item: &TavilySearchResult,
    index: usize,
    include_images: bool,
    response_images: &Option<Vec<String>>,
) -> SearchResult {
    let mut images = None;
    let mut content_chunks = None;

    // Extract images if requested and available
    if include_images {
        if let Some(img_urls) = response_images {
            if !img_urls.is_empty() {
                images = Some(
                    img_urls
                        .iter()
                        .map(|url| ImageResult {
                            url: url.clone(),
                            description: Some(format!("Image related to: {}", item.title)),
                        })
                        .collect(),
                );
            }
        }
    }

    // Create content chunks from both content and raw_content
    let mut chunks = Vec::new();

    // Add main content
    if !item.content.is_empty() {
        chunks.push(item.content.clone());
    }

    // Add raw content if available and different from main content
    if let Some(raw_content) = &item.raw_content {
        if !raw_content.is_empty() && raw_content != &item.content {
            chunks.push(raw_content.clone());
        }
    }

    if !chunks.is_empty() {
        content_chunks = Some(chunks);
    }

    // Use Tavily's score directly, but adjust for position bias
    let adjusted_score = item.score * (1.0 - (index as f32) * 0.01);

    SearchResult {
        title: item.title.clone(),
        url: item.url.clone(),
        snippet: item.content.clone(),
        display_url: extract_domain(&item.url),
        source: extract_domain(&item.url),
        score: Some(adjusted_score as f64),
        html_snippet: None,
        date_published: item.published_date.clone(),
        images,
        content_chunks,
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
    let has_more_results = {
        let requested_count = params.max_results.unwrap_or(10);
        response.results.len() == (requested_count as usize)
    };

    // Create next page token if more results are available
    let next_page_token = if has_more_results {
        let next_page = current_page + 1;
        Some(next_page.to_string())
    } else {
        None
    };

    // Tavily doesn't provide total results count, so we estimate based on results returned
    let total_results = if (response.results.len() as u32) >= params.max_results.unwrap_or(10) {
        Some(100000u64) // Conservative estimate
    } else {
        Some(response.results.len() as u64)
    };

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
    // Only validate essential parameters - be more permissive
    if params.query.trim().is_empty() {
        return Err(SearchError::InvalidQuery);
    }

    // Allow higher max_results but cap at reasonable limit
    if let Some(max_results) = params.max_results {
        if max_results > 500 {
            return Err(SearchError::UnsupportedFeature(
                "max_results cannot exceed 500 for Tavily Search".to_string(),
            ));
        }
    }
    Ok(())
}

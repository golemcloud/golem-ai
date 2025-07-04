use crate::client::{TavilyImage, TavilySearchRequest, TavilySearchResponse};
use golem_web_search::golem::web_search::types::{
    ImageResult, SearchMetadata, SearchParams, SearchResult, TimeRange,
};

pub fn convert_params_to_request(
    params: &SearchParams,
    _offset: Option<u32>,
) -> TavilySearchRequest {
    let max_results = params.max_results.unwrap_or(10).min(20);

    TavilySearchRequest {
        query: params.query.clone(),
        search_depth: Some(
            if params.advanced_answer.unwrap_or(false) {
                "advanced"
            } else {
                "basic"
            }
            .to_string(),
        ),
        topic: Some("news".to_string()),
        max_results: Some(max_results),
        include_answer: params.advanced_answer,
        include_raw_content: Some(false),
        include_images: Some(true),
        include_domains: params.include_domains.clone(),
        exclude_domains: params.exclude_domains.clone(),
        time_range: params.time_range.as_ref().map(|tr| match tr {
            TimeRange::Day => "day".to_string(),
            TimeRange::Week => "week".to_string(),
            TimeRange::Month => "month".to_string(),
            TimeRange::Year => "year".to_string(),
        }),
        country: country_code_to_tavily(params.region.as_ref().unwrap_or(&"us".to_string())),
        days: params.time_range.as_ref().map(|tr| match tr {
            TimeRange::Day => 1,
            TimeRange::Week => 7,
            TimeRange::Month => 30,
            TimeRange::Year => 365,
        }),
    }
}

pub fn country_code_to_tavily(country_code: &str) -> Option<String> {
    match country_code.to_lowercase().as_str() {
        "us" | "usa" | "united states" => Some("us".to_string()),
        "uk" | "gb" | "united kingdom" => Some("gb".to_string()),
        "ca" | "canada" => Some("ca".to_string()),
        "au" | "australia" => Some("au".to_string()),
        "de" | "germany" => Some("de".to_string()),
        "fr" | "france" => Some("fr".to_string()),
        "es" | "spain" => Some("es".to_string()),
        "it" | "italy" => Some("it".to_string()),
        "jp" | "japan" => Some("jp".to_string()),
        "br" | "brazil" => Some("br".to_string()),
        "in" | "india" => Some("in".to_string()),
        "cn" | "china" => Some("cn".to_string()),
        "ru" | "russia" => Some("ru".to_string()),
        "mx" | "mexico" => Some("mx".to_string()),
        "ar" | "argentina" => Some("ar".to_string()),
        "cl" | "chile" => Some("cl".to_string()),
        "co" | "colombia" => Some("co".to_string()),
        "pe" | "peru" => Some("pe".to_string()),
        "za" | "south africa" => Some("za".to_string()),
        "ng" | "nigeria" => Some("ng".to_string()),
        "eg" | "egypt" => Some("eg".to_string()),
        "kr" | "south korea" => Some("kr".to_string()),
        "th" | "thailand" => Some("th".to_string()),
        "sg" | "singapore" => Some("sg".to_string()),
        "my" | "malaysia" => Some("my".to_string()),
        "id" | "indonesia" => Some("id".to_string()),
        "ph" | "philippines" => Some("ph".to_string()),
        "vn" | "vietnam" => Some("vn".to_string()),
        "tw" | "taiwan" => Some("tw".to_string()),
        "hk" | "hong kong" => Some("hk".to_string()),
        "nl" | "netherlands" => Some("nl".to_string()),
        "be" | "belgium" => Some("be".to_string()),
        "ch" | "switzerland" => Some("ch".to_string()),
        "at" | "austria" => Some("at".to_string()),
        "se" | "sweden" => Some("se".to_string()),
        "no" | "norway" => Some("no".to_string()),
        "dk" | "denmark" => Some("dk".to_string()),
        "fi" | "finland" => Some("fi".to_string()),
        "pl" | "poland" => Some("pl".to_string()),
        "cz" | "czech republic" => Some("cz".to_string()),
        "hu" | "hungary" => Some("hu".to_string()),
        "gr" | "greece" => Some("gr".to_string()),
        "pt" | "portugal" => Some("pt".to_string()),
        "tr" | "turkey" => Some("tr".to_string()),
        "il" | "israel" => Some("il".to_string()),
        "ae" | "uae" | "united arab emirates" => Some("ae".to_string()),
        "sa" | "saudi arabia" => Some("sa".to_string()),
        "nz" | "new zealand" => Some("nz".to_string()),
        _ => Some(country_code.to_lowercase()),
    }
}

pub fn convert_response_to_results(
    response: TavilySearchResponse,
    params: &SearchParams,
) -> (Vec<SearchResult>, Option<SearchMetadata>) {
    let search_results: Vec<SearchResult> = response
        .results
        .into_iter()
        .map(|result| {
            let images = response.images.as_ref().and_then(|imgs| {
                if !imgs.is_empty() {
                    Some(
                        imgs.iter()
                            .map(|img| match img {
                                TavilyImage::Url(url) => ImageResult {
                                    url: url.clone(),
                                    description: None,
                                },
                                TavilyImage::Object { url, description } => ImageResult {
                                    url: url.clone(),
                                    description: description.clone(),
                                },
                            })
                            .collect(),
                    )
                } else {
                    None
                }
            });

            SearchResult {
                title: result.title,
                url: result.url,
                snippet: result.content,
                display_url: None,
                source: Some("Tavily".to_string()),
                score: result.score,
                html_snippet: None,
                date_published: result.published_date,
                images,
                content_chunks: None,
            }
        })
        .collect();

    let metadata = Some(SearchMetadata {
        query: response.query,
        total_results: None,
        search_time_ms: response.response_time,
        safe_search: None,
        language: params.language.clone(),
        region: params.region.clone(),
        next_page_token: None,
        rate_limits: None,
    });

    (search_results, metadata)
}

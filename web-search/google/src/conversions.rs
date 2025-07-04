#[cfg(test)]
use crate::client::{GoogleQueries, GoogleQueryInfo, GoogleSearchInformation, GoogleSearchItem};
use crate::client::{GoogleSearchRequest, GoogleSearchResponse};
use golem_web_search::golem::web_search::types::{
    SafeSearchLevel, SearchMetadata, SearchParams, SearchResult, TimeRange,
};

pub fn convert_params_to_request(
    params: &SearchParams,
    start_index: Option<u32>,
) -> GoogleSearchRequest {
    let mut request = GoogleSearchRequest {
        q: params.query.clone(),
        cx: String::new(),
        key: String::new(),
        num: params.max_results,
        start: start_index,
        safe: params.safe_search.as_ref().map(|s| match s {
            SafeSearchLevel::Off => "off".to_string(),
            SafeSearchLevel::Medium => "medium".to_string(),
            SafeSearchLevel::High => "high".to_string(),
        }),
        lr: language_code_to_google(params.language.as_ref().unwrap_or(&"en".to_string())),
        gl: country_code_to_google(params.region.as_ref().unwrap_or(&"us".to_string())),
        date_restrict: params.time_range.as_ref().map(|tr| match tr {
            TimeRange::Day => "d1".to_string(),
            TimeRange::Week => "w1".to_string(),
            TimeRange::Month => "m1".to_string(),
            TimeRange::Year => "y1".to_string(),
        }),
        site_search: None,
        site_search_filter: None,
    };

    if let Some(include_domains) = &params.include_domains {
        if !include_domains.is_empty() {
            request.site_search = Some(
                include_domains
                    .iter()
                    .map(|domain| format!("site:{}", domain))
                    .collect::<Vec<_>>()
                    .join(" OR "),
            );
            request.site_search_filter = Some("i".to_string());
        }
    } else if let Some(exclude_domains) = &params.exclude_domains {
        if !exclude_domains.is_empty() {
            request.site_search = Some(
                exclude_domains
                    .iter()
                    .map(|domain| format!("site:{}", domain))
                    .collect::<Vec<_>>()
                    .join(" OR "),
            );
            request.site_search_filter = Some("e".to_string());
        }
    }

    request
}

pub fn country_code_to_google(country_code: &str) -> Option<String> {
    match country_code.to_lowercase().as_str() {
        "us" | "usa" | "united states" => Some("us".to_string()),
        "uk" | "gb" | "united kingdom" => Some("uk".to_string()),
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

pub fn language_code_to_google(language_code: &str) -> Option<String> {
    let input = language_code.to_lowercase();
    
    if input.starts_with("lang_") {
        return Some(language_code.to_string());
    }
    
    let lang_code = match input.as_str() {
        "en" | "english" => "en",
        "es" | "spanish" => "es", 
        "fr" | "french" => "fr",
        "de" | "german" => "de",
        "it" | "italian" => "it",
        "pt" | "portuguese" => "pt",
        "ru" | "russian" => "ru",
        "zh" | "chinese" => "zh",
        "ja" | "japanese" => "ja",
        "ko" | "korean" => "ko",
        "ar" | "arabic" => "ar",
        "hi" | "hindi" => "hi",
        "th" | "thai" => "th",
        "vi" | "vietnamese" => "vi",
        "id" | "indonesian" => "id",
        "ms" | "malay" => "ms",
        "tl" | "tagalog" => "tl",
        "nl" | "dutch" => "nl",
        "sv" | "swedish" => "sv",
        "no" | "norwegian" => "no",
        "da" | "danish" => "da",
        "fi" | "finnish" => "fi",
        "pl" | "polish" => "pl",
        "cs" | "czech" => "cs",
        "hu" | "hungarian" => "hu",
        "el" | "greek" => "el",
        "tr" | "turkish" => "tr",
        "he" | "hebrew" => "he",
        "fa" | "persian" => "fa",
        "ur" | "urdu" => "ur",
        "bn" | "bengali" => "bn",
        "ta" | "tamil" => "ta",
        "te" | "telugu" => "te",
        "ml" | "malayalam" => "ml",
        "kn" | "kannada" => "kn",
        "gu" | "gujarati" => "gu",
        "pa" | "punjabi" => "pa",
        "mr" | "marathi" => "mr",
        "ne" | "nepali" => "ne",
        "si" | "sinhala" => "si",
        "my" | "myanmar" => "my",
        "km" | "khmer" => "km",
        "lo" | "lao" => "lo",
        "ka" | "georgian" => "ka",
        "hy" | "armenian" => "hy",
        "az" | "azerbaijani" => "az",
        "kk" | "kazakh" => "kk",
        "ky" | "kyrgyz" => "ky",
        "mn" | "mongolian" => "mn",
        "uz" | "uzbek" => "uz",
        "uk" | "ukrainian" => "uk",
        "bg" | "bulgarian" => "bg",
        "hr" | "croatian" => "hr",
        "sr" | "serbian" => "sr",
        "bs" | "bosnian" => "bs",
        "mk" | "macedonian" => "mk",
        "sl" | "slovenian" => "sl",
        "sk" | "slovak" => "sk",
        "ro" | "romanian" => "ro",
        "lv" | "latvian" => "lv",
        "lt" | "lithuanian" => "lt",
        "et" | "estonian" => "et",
        "mt" | "maltese" => "mt",
        "is" | "icelandic" => "is",
        "ga" | "irish" => "ga",
        "cy" | "welsh" => "cy",
        "eu" | "basque" => "eu",
        "ca" | "catalan" => "ca",
        "gl" | "galician" => "gl",
        "af" | "afrikaans" => "af",
        "sw" | "swahili" => "sw",
        "am" | "amharic" => "am",
        "or" | "oriya" => "or",
        "as" | "assamese" => "as",
        "sd" | "sindhi" => "sd",
        "ps" | "pashto" => "ps",
        "tg" | "tajik" => "tg",
        "tk" | "turkmen" => "tk",
        _ => &input,
    };
    Some(format!("lang_{}", lang_code))
}

pub fn convert_response_to_results(
    response: GoogleSearchResponse,
    params: &SearchParams,
) -> (Vec<SearchResult>, Option<SearchMetadata>) {
    let results = if let Some(items) = response.items {
        items
            .into_iter()
            .map(|item| SearchResult {
                title: item.title,
                url: item.link,
                snippet: item.snippet,
                display_url: item.display_link,
                source: Some("Google".to_string()),
                score: None,
                html_snippet: if params.include_html.unwrap_or(false) {
                    item.html_snippet
                } else {
                    None
                },
                date_published: None,
                images: None,
                content_chunks: None,
            })
            .collect()
    } else {
        Vec::new()
    };

    let metadata = SearchMetadata {
        query: params.query.clone(),
        total_results: response
            .search_information
            .as_ref()
            .and_then(|info| info.total_results.as_ref())
            .and_then(|s| s.parse::<u64>().ok()),
        search_time_ms: response
            .search_information
            .as_ref()
            .and_then(|info| info.search_time)
            .map(|t| t * 1000.0),
        safe_search: params.safe_search,
        language: params.language.clone(),
        region: params.region.clone(),
        next_page_token: response
            .queries
            .as_ref()
            .and_then(|q| q.next_page.as_ref())
            .and_then(|np| np.first())
            .and_then(|np| np.start_index)
            .map(|idx| idx.to_string()),
        rate_limits: None,
    };

    (results, Some(metadata))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_params() -> SearchParams {
        SearchParams {
            query: "test query".to_string(),
            safe_search: Some(SafeSearchLevel::Medium),
            language: Some("en".to_string()),
            region: Some("us".to_string()),
            max_results: Some(10),
            time_range: Some(TimeRange::Week),
            include_domains: Some(vec!["example.com".to_string(), "test.org".to_string()]),
            exclude_domains: None,
            include_images: Some(true),
            include_html: Some(true),
            advanced_answer: Some(false),
        }
    }

    fn create_test_response() -> GoogleSearchResponse {
        GoogleSearchResponse {
            items: Some(vec![
                GoogleSearchItem {
                    title: "Test Result 1".to_string(),
                    link: "https://example.com/1".to_string(),
                    snippet: "This is a test snippet 1".to_string(),
                    display_link: Some("example.com".to_string()),
                    html_snippet: Some("<b>Test</b> snippet 1".to_string()),
                    pagemap: None,
                },
                GoogleSearchItem {
                    title: "Test Result 2".to_string(),
                    link: "https://test.org/2".to_string(),
                    snippet: "This is a test snippet 2".to_string(),
                    display_link: Some("test.org".to_string()),
                    html_snippet: Some("<b>Test</b> snippet 2".to_string()),
                    pagemap: None,
                },
            ]),
            search_information: Some(GoogleSearchInformation {
                total_results: Some("1000".to_string()),
                search_time: Some(0.15),
            }),
            queries: Some(GoogleQueries {
                request: None,
                next_page: Some(vec![GoogleQueryInfo {
                    title: "Next Page".to_string(),
                    total_results: Some("1000".to_string()),
                    search_terms: "test query".to_string(),
                    count: Some(10),
                    start_index: Some(11),
                }]),
            }),
            error: None,
        }
    }

    #[test]
    fn test_convert_params_to_request_basic() {
        let params = SearchParams {
            query: "basic test".to_string(),
            safe_search: None,
            language: None,
            region: None,
            max_results: None,
            time_range: None,
            include_domains: None,
            exclude_domains: None,
            include_images: None,
            include_html: None,
            advanced_answer: None,
        };

        let request = convert_params_to_request(&params, None);

        assert_eq!(request.q, "basic test");
        assert_eq!(request.cx, "");
        assert_eq!(request.key, "");
        assert_eq!(request.num, None);
        assert_eq!(request.start, None);
        assert_eq!(request.safe, None);
        assert_eq!(request.lr, None);
        assert_eq!(request.gl, None);
        assert_eq!(request.date_restrict, None);
        assert_eq!(request.site_search, None);
        assert_eq!(request.site_search_filter, None);
    }

    #[test]
    fn test_convert_params_to_request_full() {
        let params = create_test_params();
        let request = convert_params_to_request(&params, Some(21));

        assert_eq!(request.q, "test query");
        assert_eq!(request.num, Some(10));
        assert_eq!(request.start, Some(21));
        assert_eq!(request.safe, Some("medium".to_string()));
        assert_eq!(request.lr, Some("lang_en".to_string()));
        assert_eq!(request.gl, Some("us".to_string()));
        assert_eq!(request.date_restrict, Some("w1".to_string()));
        assert_eq!(
            request.site_search,
            Some("site:example.com OR site:test.org".to_string())
        );
        assert_eq!(request.site_search_filter, Some("i".to_string()));
    }

    #[test]
    fn test_convert_params_safe_search_levels() {
        let test_cases = vec![
            (SafeSearchLevel::Off, "off"),
            (SafeSearchLevel::Medium, "medium"),
            (SafeSearchLevel::High, "high"),
        ];

        for (level, expected) in test_cases {
            let params = SearchParams {
                query: "test".to_string(),
                safe_search: Some(level),
                language: None,
                region: None,
                max_results: None,
                time_range: None,
                include_domains: None,
                exclude_domains: None,
                include_images: None,
                include_html: None,
                advanced_answer: None,
            };

            let request = convert_params_to_request(&params, None);
            assert_eq!(request.safe, Some(expected.to_string()));
        }
    }

    #[test]
    fn test_convert_params_time_ranges() {
        let test_cases = vec![
            (TimeRange::Day, "d1"),
            (TimeRange::Week, "w1"),
            (TimeRange::Month, "m1"),
            (TimeRange::Year, "y1"),
        ];

        for (range, expected) in test_cases {
            let params = SearchParams {
                query: "test".to_string(),
                safe_search: None,
                language: None,
                region: None,
                max_results: None,
                time_range: Some(range),
                include_domains: None,
                exclude_domains: None,
                include_images: None,
                include_html: None,
                advanced_answer: None,
            };

            let request = convert_params_to_request(&params, None);
            assert_eq!(request.date_restrict, Some(expected.to_string()));
        }
    }

    #[test]
    fn test_convert_params_exclude_domains() {
        let params = SearchParams {
            query: "test".to_string(),
            safe_search: None,
            language: None,
            region: None,
            max_results: None,
            time_range: None,
            include_domains: None,
            exclude_domains: Some(vec!["spam.com".to_string(), "bad.org".to_string()]),
            include_images: None,
            include_html: None,
            advanced_answer: None,
        };

        let request = convert_params_to_request(&params, None);
        assert_eq!(
            request.site_search,
            Some("site:spam.com OR site:bad.org".to_string())
        );
        assert_eq!(request.site_search_filter, Some("e".to_string()));
    }

    #[test]
    fn test_convert_response_to_results_basic() {
        let params = create_test_params();
        let response = create_test_response();

        let (results, metadata) = convert_response_to_results(response, &params);

        assert_eq!(results.len(), 2);

        assert_eq!(results[0].title, "Test Result 1");
        assert_eq!(results[0].url, "https://example.com/1");
        assert_eq!(results[0].snippet, "This is a test snippet 1");
        assert_eq!(results[0].display_url, Some("example.com".to_string()));
        assert_eq!(results[0].source, Some("Google".to_string()));
        assert_eq!(results[0].score, None);
        assert_eq!(
            results[0].html_snippet,
            Some("<b>Test</b> snippet 1".to_string())
        );
        assert_eq!(results[0].date_published, None);
        assert_eq!(results[0].images, None);
        assert_eq!(results[0].content_chunks, None);

        assert_eq!(results[1].title, "Test Result 2");
        assert_eq!(results[1].url, "https://test.org/2");
        assert_eq!(results[1].snippet, "This is a test snippet 2");
        assert_eq!(results[1].display_url, Some("test.org".to_string()));

        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert_eq!(meta.query, "test query");
        assert_eq!(meta.total_results, Some(1000));
        assert_eq!(meta.search_time_ms, Some(150.0)); // 0.15 * 1000
        assert_eq!(meta.safe_search, Some(SafeSearchLevel::Medium));
        assert_eq!(meta.language, Some("en".to_string()));
        assert_eq!(meta.region, Some("us".to_string()));
        assert_eq!(meta.next_page_token, Some("11".to_string()));
        assert_eq!(meta.rate_limits, None);
    }

    #[test]
    fn test_convert_response_to_results_no_html() {
        let mut params = create_test_params();
        params.include_html = Some(false);
        let response = create_test_response();

        let (results, _) = convert_response_to_results(response, &params);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].html_snippet, None);
        assert_eq!(results[1].html_snippet, None);
    }

    #[test]
    fn test_convert_response_to_results_empty() {
        let params = create_test_params();
        let response = GoogleSearchResponse {
            items: None,
            search_information: None,
            queries: None,
            error: None,
        };

        let (results, metadata) = convert_response_to_results(response, &params);

        assert_eq!(results.len(), 0);
        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert_eq!(meta.query, "test query");
        assert_eq!(meta.total_results, None);
        assert_eq!(meta.search_time_ms, None);
        assert_eq!(meta.next_page_token, None);
    }

    #[test]
    fn test_convert_response_malformed_total_results() {
        let params = create_test_params();
        let mut response = create_test_response();
        response.search_information = Some(GoogleSearchInformation {
            total_results: Some("not_a_number".to_string()),
            search_time: Some(0.25),
        });

        let (_, metadata) = convert_response_to_results(response, &params);

        let meta = metadata.unwrap();
        assert_eq!(meta.total_results, None);
        assert_eq!(meta.search_time_ms, Some(250.0));
    }
}

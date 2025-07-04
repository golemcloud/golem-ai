use crate::client::{SerperSearchRequest, SerperSearchResponse};
use golem_web_search::golem::web_search::types::{
    ImageResult, SearchMetadata, SearchParams, SearchResult, TimeRange,
};

pub fn convert_params_to_request(params: &SearchParams, page: Option<u32>) -> SerperSearchRequest {
    let num = params.max_results.unwrap_or(10).min(100);

    SerperSearchRequest {
        q: params.query.clone(),
        location: params.region.clone(),
        gl: country_code_to_serper(params.region.as_ref().unwrap_or(&"us".to_string())),     
        hl: language_code_to_serper(params.language.as_ref().unwrap_or(&"en".to_string())), 
        num: Some(num),
        autocorrect: Some(true),
        tbs: params.time_range.as_ref().map(|tr| match tr {
            TimeRange::Day => "d".to_string(),
            TimeRange::Week => "w".to_string(),
            TimeRange::Month => "m".to_string(),
            TimeRange::Year => "y".to_string(),
        }),
        page,
    }
}

pub fn country_code_to_serper(country_code: &str) -> Option<String> {
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

pub fn language_code_to_serper(language_code: &str) -> Option<String> {
    let input = language_code.to_lowercase();
    
    let lang_code = if input.starts_with("lang_") {
        input.strip_prefix("lang_").unwrap_or(&input)
    } else {
        &input
    };
    
    match lang_code {
        "en" | "english" => Some("en".to_string()),
        "es" | "spanish" => Some("es".to_string()),
        "fr" | "french" => Some("fr".to_string()),
        "de" | "german" => Some("de".to_string()),
        "it" | "italian" => Some("it".to_string()),
        "pt" | "portuguese" => Some("pt".to_string()),
        "ru" | "russian" => Some("ru".to_string()),
        "zh" | "chinese" => Some("zh".to_string()),
        "ja" | "japanese" => Some("ja".to_string()),
        "ko" | "korean" => Some("ko".to_string()),
        "ar" | "arabic" => Some("ar".to_string()),
        "hi" | "hindi" => Some("hi".to_string()),
        "th" | "thai" => Some("th".to_string()),
        "vi" | "vietnamese" => Some("vi".to_string()),
        "id" | "indonesian" => Some("id".to_string()),
        "ms" | "malay" => Some("ms".to_string()),
        "tl" | "tagalog" => Some("tl".to_string()),
        "nl" | "dutch" => Some("nl".to_string()),
        "sv" | "swedish" => Some("sv".to_string()),
        "no" | "norwegian" => Some("no".to_string()),
        "da" | "danish" => Some("da".to_string()),
        "fi" | "finnish" => Some("fi".to_string()),
        "pl" | "polish" => Some("pl".to_string()),
        "cs" | "czech" => Some("cs".to_string()),
        "hu" | "hungarian" => Some("hu".to_string()),
        "el" | "greek" => Some("el".to_string()),
        "tr" | "turkish" => Some("tr".to_string()),
        "he" | "hebrew" => Some("he".to_string()),
        "fa" | "persian" => Some("fa".to_string()),
        "ur" | "urdu" => Some("ur".to_string()),
        "bn" | "bengali" => Some("bn".to_string()),
        "ta" | "tamil" => Some("ta".to_string()),
        "te" | "telugu" => Some("te".to_string()),
        "ml" | "malayalam" => Some("ml".to_string()),
        "kn" | "kannada" => Some("kn".to_string()),
        "gu" | "gujarati" => Some("gu".to_string()),
        "pa" | "punjabi" => Some("pa".to_string()),
        "mr" | "marathi" => Some("mr".to_string()),
        "ne" | "nepali" => Some("ne".to_string()),
        "si" | "sinhala" => Some("si".to_string()),
        "my" | "myanmar" => Some("my".to_string()),
        "km" | "khmer" => Some("km".to_string()),
        "lo" | "lao" => Some("lo".to_string()),
        "ka" | "georgian" => Some("ka".to_string()),
        "hy" | "armenian" => Some("hy".to_string()),
        "az" | "azerbaijani" => Some("az".to_string()),
        "kk" | "kazakh" => Some("kk".to_string()),
        "ky" | "kyrgyz" => Some("ky".to_string()),
        "mn" | "mongolian" => Some("mn".to_string()),
        "uz" | "uzbek" => Some("uz".to_string()),
        "uk" | "ukrainian" => Some("uk".to_string()),
        "bg" | "bulgarian" => Some("bg".to_string()),
        "hr" | "croatian" => Some("hr".to_string()),
        "sr" | "serbian" => Some("sr".to_string()),
        "bs" | "bosnian" => Some("bs".to_string()),
        "mk" | "macedonian" => Some("mk".to_string()),
        "sl" | "slovenian" => Some("sl".to_string()),
        "sk" | "slovak" => Some("sk".to_string()),
        "ro" | "romanian" => Some("ro".to_string()),
        "lv" | "latvian" => Some("lv".to_string()),
        "lt" | "lithuanian" => Some("lt".to_string()),
        "et" | "estonian" => Some("et".to_string()),
        "mt" | "maltese" => Some("mt".to_string()),
        "is" | "icelandic" => Some("is".to_string()),
        "ga" | "irish" => Some("ga".to_string()),
        "cy" | "welsh" => Some("cy".to_string()),
        "eu" | "basque" => Some("eu".to_string()),
        "ca" | "catalan" => Some("ca".to_string()),
        "gl" | "galician" => Some("gl".to_string()),
        "af" | "afrikaans" => Some("af".to_string()),
        "sw" | "swahili" => Some("sw".to_string()),
        "am" | "amharic" => Some("am".to_string()),
        "or" | "oriya" => Some("or".to_string()),
        "as" | "assamese" => Some("as".to_string()),
        "sd" | "sindhi" => Some("sd".to_string()),
        "ps" | "pashto" => Some("ps".to_string()),
        "tg" | "tajik" => Some("tg".to_string()),
        "tk" | "turkmen" => Some("tk".to_string()),
        _ => Some(lang_code.to_string()),
    }
}

pub fn convert_response_to_results(
    response: SerperSearchResponse,
    params: &SearchParams,
) -> (Vec<SearchResult>, Option<SearchMetadata>) {
    let mut search_results = Vec::new();

    if let Some(organic_results) = response.organic {
        for result in organic_results {
            let images = response.images.as_ref().and_then(|imgs| {
                if !imgs.is_empty() {
                    Some(
                        imgs.iter() 
                            .map(|img| ImageResult {
                                url: img.image_url.clone(),
                                description: Some(img.title.clone()),
                            })
                            .collect(),
                    )
                } else {
                    None
                }
            });

            search_results.push(SearchResult {
                title: result.title,
                url: result.link,
                snippet: result.snippet,
                display_url: None,
                source: Some("Serper".to_string()),
                score: result.position.map(|p| 1.0 / (p as f64 + 1.0)),
                html_snippet: None,
                date_published: result.date,
                images,
                content_chunks: None,
            });
        }
    }

    if let Some(answer_box) = response.answer_box {
        search_results.insert(
            0,
            SearchResult {
                title: answer_box.title,
                url: answer_box.link.unwrap_or_default(),
                snippet: answer_box.answer.or(answer_box.snippet).unwrap_or_default(),
                display_url: None,
                source: Some("Serper Answer Box".to_string()),
                score: Some(1.0),
                html_snippet: None,
                date_published: None,
                images: None,
                content_chunks: None,
            },
        );
    }

    if let Some(kg) = response.knowledge_graph {
        let kg_images = kg.image_url.map(|url| {
            vec![ImageResult {
                url,
                description: Some(kg.title.clone()),
            }]
        });

        search_results.insert(
            0,
            SearchResult {
                title: kg.title,
                url: kg.website.unwrap_or_default(),
                snippet: kg.description.unwrap_or_default(),
                display_url: None,
                source: Some("Serper Knowledge Graph".to_string()),
                score: Some(1.0),
                html_snippet: None,
                date_published: None,
                images: kg_images,
                content_chunks: None,
            },
        );
    }

    let total_results = response
        .search_information
        .as_ref()
        .and_then(|info| info.total_results.as_ref())
        .and_then(|total| total.parse::<u64>().ok());

    let search_time_ms = response
        .search_information
        .as_ref()
        .and_then(|info| info.time_taken.map(|t| t * 1000.0));

    let metadata = Some(SearchMetadata {
        query: response
            .search_parameters
            .as_ref()
            .map(|sp| sp.q.clone())
            .unwrap_or_else(|| params.query.clone()),
        total_results,
        search_time_ms,
        safe_search: None,
        language: params.language.clone(),
        region: params.region.clone(),
        next_page_token: None,
        rate_limits: None,
    });

    (search_results, metadata)
}

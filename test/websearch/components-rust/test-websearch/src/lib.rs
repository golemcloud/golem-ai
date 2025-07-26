#[allow(static_mut_refs)]
mod bindings;

use golem_rust::atomically;
use crate::bindings::test::helper_client::test_helper_client::TestHelperApi;
use crate::bindings::exports::test::websearch_exports::test_websearch_api::*;
use crate::bindings::golem::web_search::web_search;
use crate::bindings::golem::web_search::types::{
    SearchParams,
    SafeSearchLevel,
    TimeRange,
    SearchError,
};

struct Component;

#[cfg(feature = "google")]
const PROVIDER: &'static str = "google";
#[cfg(feature = "brave")]
const PROVIDER: &'static str = "brave";
#[cfg(feature = "tavily")]
const PROVIDER: &'static str = "tavily";
#[cfg(feature = "serper")]
const PROVIDER: &'static str = "serper";

impl Guest for Component {
    /// test1 demonstrates a simple, one-shot web search query
    fn test1() -> String {
        let params = SearchParams {
            query: "weather forecast Slovenia".to_string(),
            safe_search: Some(SafeSearchLevel::Medium),
            language: Some("en".to_string()),
            region: Some("US".to_string()),
            max_results: Some(5),
            time_range: None,
            include_domains: None,
            exclude_domains: None,
            include_images: None,
            include_html: None,
            advanced_answer: None,
        };

        println!("Sending search request using {} provider...", PROVIDER);
        let response = web_search::search_once(&params);
        println!("Response: {:?}", response);

        match response {
            Ok((results, metadata)) => {
                let mut output = String::new();

                output.push_str(&format!("Found {} results:\n", results.len()));

                for (i, result) in results.iter().enumerate() {
                    output.push_str(
                        &format!(
                            "{}. {}\n   URL: {}\n   Snippet: {}\n",
                            i + 1,
                            result.title,
                            result.url,
                            result.snippet
                        )
                    );

                    if let Some(score) = result.score {
                        output.push_str(&format!("   Score: {:.2}\n", score));
                    }

                    if let Some(date) = &result.date_published {
                        output.push_str(&format!("   Published: {}\n", date));
                    }

                    output.push_str("\n");
                }

                if let Some(meta) = metadata {
                    output.push_str("\nDetailed Search Metadata:\n");
                    output.push_str(&format!("  Query: {}\n", meta.query));
                    if let Some(total) = meta.total_results {
                        output.push_str(&format!("  Total Results: {}\n", total));
                    }
                    if let Some(time) = meta.search_time_ms {
                        output.push_str(&format!("  Search Time: {:.2}ms\n", time));
                    }
                    if let Some(lang) = &meta.language {
                        output.push_str(&format!("  Language: {}\n", lang));
                    }
                    if let Some(reg) = &meta.region {
                        output.push_str(&format!("  Region: {}\n", reg));
                    }
                    if let Some(safe) = meta.safe_search {
                        output.push_str(&format!("  Safe Search Level: {:?}\n", safe));
                    }
                    if let Some(rate_limit) = &meta.rate_limits {
                        output.push_str(
                            &format!(
                                "  Rate Limit: {}/{} requests remaining (reset: {})\n",
                                rate_limit.remaining,
                                rate_limit.limit,
                                rate_limit.reset_timestamp
                            )
                        );
                    }
                }

                output
            }
            Err(error) => {
                let error_msg = format_search_error(error);
                println!("{}", error_msg);
                error_msg
            }
        }
    }

    /// test2 simulates a crash during a streaming web search response, but only first time.
    /// It demonstrates paginated search using search sessions.
    /// after the automatic recovery it will continue and finish the request successfully.
    fn test2() -> String {
        let params = SearchParams {
            query: "Rust programming language tutorials".to_string(),
            safe_search: Some(SafeSearchLevel::Off),
            language: Some("en".to_string()),
            region: Some("US".to_string()),
            max_results: Some(3),
            time_range: None,
            include_domains: None,
            exclude_domains: None,
            include_images: None,
            include_html: None,
            advanced_answer: None,
        };

        println!("Starting search session using {} provider...", PROVIDER);

        let session = match web_search::start_search(&params) {
            Ok(session) => session,
            Err(error) => {
                let error_msg = format_search_error(error);
                println!("{}", error_msg);
                return error_msg;
            }
        };

        let mut output = String::new();
        output.push_str("Search session started successfully!\n\n");
        let name = std::env::var("GOLEM_WORKER_NAME").unwrap();
        let mut round = 0;

        // Get first page
        println!("Getting first page...");
        match session.next_page() {
            Ok(results) => {
                output.push_str(&format!("First page - {} results:\n", results.len()));
                for (i, result) in results.iter().enumerate() {
                    output.push_str(&format!("{}. {}\n   {}\n", i + 1, result.title, result.url));
                }
                output.push_str("\n");
            }
            Err(error) => {
                let error_msg = format_search_error(error);
                println!("{}", error_msg);
                output.push_str(&format!("{}\n\n", error_msg));
            }
        }
        round += 1;

        // Add a delay before the next request to avoid rate limiting
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Crash simulation before getting second page
        if round == 1 {
            atomically(|| {
                let client = TestHelperApi::new(&name);
                let answer = client.blocking_inc_and_get();
                if answer == 1 {
                    panic!("Simulating crash")
                }
            });
        }

        // Get second page
        println!("Getting second page...");
        match session.next_page() {
            Ok(results) => {
                if results.is_empty() {
                    output.push_str("No more results available (end of pagination)\n");
                } else {
                    output.push_str(&format!("Second page - {} results:\n", results.len()));
                    for (i, result) in results.iter().enumerate() {
                        output.push_str(
                            &format!("{}. {}\n   {}\n", i + 1, result.title, result.url)
                        );
                    }
                }
            }
            Err(error) => {
                let error_msg = format_search_error(error);
                println!("{}", error_msg);
                output.push_str(&format!("{}\n", error_msg));
            }
        }

        // Get metadata
        if let Some(metadata) = session.get_metadata() {
            output.push_str(&format!("\nDetailed Session Metadata:\n"));
            output.push_str(&format!("  Query: {}\n", metadata.query));
            if let Some(total) = metadata.total_results {
                output.push_str(&format!("  Total Results: {}\n", total));
            }
            if let Some(time) = metadata.search_time_ms {
                output.push_str(&format!("  Search Time: {:.2}ms\n", time));
            }
            if let Some(lang) = &metadata.language {
                output.push_str(&format!("  Language: {}\n", lang));
            }
            if let Some(reg) = &metadata.region {
                output.push_str(&format!("  Region: {}\n", reg));
            }
            if let Some(safe) = metadata.safe_search {
                output.push_str(&format!("  Safe Search Level: {:?}\n", safe));
            }
            if let Some(rate_limits) = &metadata.rate_limits {
                output.push_str(
                    &format!(
                        "  Rate Limits: {}/{} remaining (reset: {})\n",
                        rate_limits.remaining,
                        rate_limits.limit,
                        rate_limits.reset_timestamp
                    )
                );
            }
            // Assert and output current_page
            let expected_page = 1; // After two next_page() calls, should be on page 1 (0-based)
            assert_eq!(
                metadata.current_page,
                expected_page,
                "Expected current_page to be {} after two next_page() calls, got {}",
                expected_page,
                metadata.current_page
            );
            output.push_str(&format!("  Current Page: {}\n", metadata.current_page));
        }

        output
    }

    /// test3 demonstrates time-filtered search for recent news
    fn test3() -> String {
        let params = SearchParams {
            query: "artificial intelligence breakthrough".to_string(),
            safe_search: Some(SafeSearchLevel::Medium),
            language: Some("en".to_string()),
            region: Some("US".to_string()),
            max_results: Some(5),
            time_range: Some(TimeRange::Week),
            include_domains: None,
            exclude_domains: None,
            include_images: None,
            include_html: None,
            advanced_answer: None,
        };

        println!("Searching for recent AI news using {} provider...", PROVIDER);
        let response = web_search::search_once(&params);

        match response {
            Ok((results, metadata)) => {
                let mut output = String::new();
                output.push_str("Recent AI news (past week):\n\n");

                for (i, result) in results.iter().enumerate() {
                    output.push_str(&format!("{}. {}\n", i + 1, result.title));
                    output.push_str(&format!("   URL: {}\n", result.url));
                    output.push_str(&format!("   Snippet: {}\n", result.snippet));

                    if let Some(date) = &result.date_published {
                        output.push_str(&format!("   Published: {}\n", date));
                    }

                    if let Some(source) = &result.source {
                        output.push_str(&format!("   Source: {}\n", source));
                    }

                    output.push_str("\n");
                }

                if let Some(meta) = metadata {
                    output.push_str(
                        &format!("Search parameters: time_range={:?}\n", TimeRange::Week)
                    );
                    if let Some(total) = meta.total_results {
                        output.push_str(&format!("Total results available: {}\n", total));
                    }
                }

                output
            }
            Err(error) => {
                let error_msg = format_search_error(error);
                println!("{}", error_msg);
                error_msg
            }
        }
    }

    /// test4 demonstrates domain filtering (include specific domains)
    fn test4() -> String {
        let domains = vec![
            "nature.com".to_string(),
            "science.org".to_string(),
            "sciencedirect.com".to_string()
        ];

        let params = SearchParams {
            query: "climate change research".to_string(),
            safe_search: Some(SafeSearchLevel::Medium),
            language: Some("en".to_string()),
            region: Some("US".to_string()),
            max_results: Some(6),
            time_range: None,
            include_domains: None,
            exclude_domains: None,
            include_images: None,
            include_html: None,
            advanced_answer: None,
        };

        println!("Searching academic sources for climate research using {} provider...", PROVIDER);
        let response = web_search::search_once(&params);

        match response {
            Ok((results, metadata)) => {
                let mut output = String::new();
                output.push_str("Climate research from academic sources:\n\n");

                if results.is_empty() {
                    output.push_str("No results found from the specified academic domains.\n");
                }

                for (i, result) in results.iter().enumerate() {
                    output.push_str(&format!("{}. {}\n", i + 1, result.title));
                    output.push_str(&format!("   URL: {}\n", result.url));
                    output.push_str(&format!("   Snippet: {}\n", result.snippet));

                    if let Some(display_url) = &result.display_url {
                        output.push_str(&format!("   Display URL: {}\n", display_url));
                    }

                    output.push_str("\n");
                }

                output.push_str(&format!("Target academic domains: {}\n", domains.join(", ")));

                if let Some(meta) = metadata {
                    output.push_str("\nSearch metadata:\n");
                    output.push_str(&format!("  Query: {}\n", meta.query));
                    if let Some(total) = meta.total_results {
                        output.push_str(&format!("  Total results: {}\n", total));
                    }
                    if let Some(time) = meta.search_time_ms {
                        output.push_str(&format!("  Search time: {:.2}ms\n", time));
                    }
                }

                output
            }
            Err(error) => {
                let error_msg = format_search_error(error);
                println!("{}", error_msg);
                error_msg
            }
        }
    }

    /// test5 demonstrates domain exclusion and image inclusion
    fn test5() -> String {
        let excluded_domains = vec![
            "amazon.com".to_string(),
            "ebay.com".to_string(),
            "aliexpress.com".to_string()
        ];

        let params = SearchParams {
            query: "mountain hiking gear reviews".to_string(),
            safe_search: Some(SafeSearchLevel::Off),
            language: Some("en".to_string()),
            region: Some("US".to_string()),
            max_results: Some(4),
            time_range: None,
            include_domains: None,
            exclude_domains: Some(excluded_domains.clone()),
            include_images: None,
            include_html: None,
            advanced_answer: None,
        };

        println!("Searching hiking gear reviews (excluding e-commerce) using {} provider...", PROVIDER);
        let response = web_search::search_once(&params);

        match response {
            Ok((results, metadata)) => {
                let mut output = String::new();
                output.push_str("Hiking gear reviews (non-commercial sources):\n\n");

                for (i, result) in results.iter().enumerate() {
                    output.push_str(&format!("{}. {}\n", i + 1, result.title));
                    output.push_str(&format!("   URL: {}\n", result.url));
                    output.push_str(&format!("   Snippet: {}\n", result.snippet));

                    if let Some(images) = &result.images {
                        if !images.is_empty() {
                            output.push_str(&format!("   Images found: {}\n", images.len()));
                            for (j, image) in images.iter().enumerate().take(2) {
                                output.push_str(&format!("     Image {}: {}\n", j + 1, image.url));
                                if let Some(desc) = &image.description {
                                    output.push_str(&format!("     Description: {}\n", desc));
                                }
                            }
                        }
                    }

                    if let Some(html) = &result.html_snippet {
                        output.push_str(
                            &format!("   HTML content available: {} chars\n", html.len())
                        );
                    }

                    output.push_str("\n");
                }

                output.push_str(&format!("Excluded domains: {}\n", excluded_domains.join(", ")));

                if let Some(meta) = metadata {
                    output.push_str("\nSearch metadata:\n");
                    output.push_str(&format!("  Query: {}\n", meta.query));
                    if let Some(total) = meta.total_results {
                        output.push_str(&format!("  Total results: {}\n", total));
                    }
                    if let Some(time) = meta.search_time_ms {
                        output.push_str(&format!("  Search time: {:.2}ms\n", time));
                    }
                }

                output
            }
            Err(error) => {
                let error_msg = format_search_error(error);
                println!("{}", error_msg);
                error_msg
            }
        }
    }

    /// test6 demonstrates multilingual search with specific region
    fn test6() -> String {
        let params = SearchParams {
            query: "slovenian recipes".to_string(),
            safe_search: Some(SafeSearchLevel::Medium),
            language: Some("en".to_string()),
            region: Some("US".to_string()),
            max_results: Some(5),
            time_range: None,
            include_domains: None,
            exclude_domains: None,
            include_images: None,
            include_html: None,
            advanced_answer: None,
        };

        println!("Searching Slovenian recipes in Slovenian language using {} provider...", PROVIDER);
        let response = web_search::search_once(&params);

        match response {
            Ok((results, metadata)) => {
                let mut output = String::new();
                output.push_str("Slovenian traditional recipes (in Slovenian):\n\n");

                if results.is_empty() {
                    output.push_str("No results found. This might be because:\n");
                    output.push_str("- The provider doesn't support Slovenian language searches\n");
                    output.push_str("- Limited content available in Slovenian\n");
                    output.push_str("- Regional restrictions\n\n");
                }

                for (i, result) in results.iter().enumerate() {
                    output.push_str(&format!("{}. {}\n", i + 1, result.title));
                    output.push_str(&format!("   URL: {}\n", result.url));
                    output.push_str(&format!("   Snippet: {}\n", result.snippet));

                    if let Some(images) = &result.images {
                        if !images.is_empty() {
                            output.push_str(&format!("   Recipe images: {}\n", images.len()));
                        }
                    }

                    output.push_str("\n");
                }

                if let Some(meta) = metadata {
                    output.push_str(
                        &format!(
                            "Search performed in: language={}, region={}\n",
                            meta.language.as_deref().unwrap_or("unknown"),
                            meta.region.as_deref().unwrap_or("unknown")
                        )
                    );
                }

                output
            }
            Err(error) => {
                let error_msg = format_search_error(error);
                println!("{}", error_msg);
                error_msg
            }
        }
    }

    /// test7 demonstrates advanced search with high safe search and content chunks
    fn test7() -> String {
        let trusted_domains = vec![
            "commonsensemedia.org".to_string(),
            "safekids.org".to_string(),
            "connectsafely.org".to_string()
        ];

        let params = SearchParams {
            query: "child safety internet guidelines parents".to_string(),
            safe_search: Some(SafeSearchLevel::High),
            language: Some("en".to_string()),
            region: Some("US".to_string()),
            max_results: Some(4),
            time_range: None,
            include_domains: None,
            exclude_domains: None,
            include_images: None,
            include_html: None,
            advanced_answer: None,
        };

        println!("Searching child safety resources with high safe search using {} provider...", PROVIDER);
        let response = web_search::search_once(&params);

        match response {
            Ok((results, metadata)) => {
                let mut output = String::new();
                output.push_str("Child Internet Safety Resources (High Safe Search):\n\n");

                for (i, result) in results.iter().enumerate() {
                    output.push_str(&format!("{}. {}\n", i + 1, result.title));
                    output.push_str(&format!("   URL: {}\n", result.url));
                    output.push_str(&format!("   Snippet: {}\n", result.snippet));

                    if let Some(chunks) = &result.content_chunks {
                        output.push_str(&format!("   Content chunks: {}\n", chunks.len()));
                        for (j, chunk) in chunks.iter().enumerate().take(2) {
                            let preview = if chunk.len() > 100 {
                                format!("{}...", &chunk[..100])
                            } else {
                                chunk.clone()
                            };
                            output.push_str(&format!("     Chunk {}: {}\n", j + 1, preview));
                        }
                    }

                    if let Some(score) = result.score {
                        output.push_str(&format!("   Relevance score: {:.2}\n", score));
                    }

                    output.push_str("\n");
                }

                if let Some(meta) = metadata {
                    output.push_str(&format!("Safe search level: {:?}\n", meta.safe_search));
                    output.push_str(&format!("Time range: past year\n"));
                    output.push_str(
                        &format!("Target trusted domains: {}\n", trusted_domains.join(", "))
                    );

                    if let Some(rate_limit) = &meta.rate_limits {
                        output.push_str(
                            &format!(
                                "Rate limit: {}/{} requests remaining\n",
                                rate_limit.remaining,
                                rate_limit.limit
                            )
                        );
                    }
                }

                output
            }
            Err(error) => {
                let error_msg = format_search_error(error);
                println!("{}", error_msg);
                error_msg
            }
        }
    }
}

fn format_search_error(error: SearchError) -> String {
    match error {
        SearchError::InvalidQuery => "ERROR: Invalid query provided".to_string(),
        SearchError::RateLimited(retry_after) => {
            format!("ERROR: Rate limited. Retry after {} seconds", retry_after)
        }
        SearchError::UnsupportedFeature(feature) => {
            format!("ERROR: Unsupported feature: {}", feature)
        }
        SearchError::BackendError(message) => { format!("ERROR: Backend error: {}", message) }
    }
}

bindings::export!(Component with_types_in bindings);

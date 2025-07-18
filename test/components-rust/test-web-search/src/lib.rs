#[allow(static_mut_refs)]
mod bindings;

use crate::bindings::exports::test::web_search_exports::test_web_search_api::*;
use crate::bindings::golem::web_search::web_search;
use crate::bindings::test::helper_client::test_helper_client::TestHelperApi;
use golem_rust::atomically;

struct Component;

impl Guest for Component {
    /// test1 demonstrates a simple web search query
    fn test1() -> String {
        let params = web_search::SearchParams {
            query: "Rust programming language".to_string(),
            safe_search: None,
            language: None,
            region: None,
            max_results: Some(10),
            time_range: None,
            include_domains: None,
            exclude_domains: None,
            include_images: None,
            include_html: None,
            advanced_answer: None,
        };

        println!("Sending web search request...");
        let response = web_search::search_once(&params);
        println!("Response: {:?}", response);

        match response {
            Ok((results, metadata)) => {
                format!(
                    "Found {} results.\nResults: {:?}\nMetadata: {:?}",
                    results.len(),
                    results,
                    metadata
                )
            }
            Err(error) => {
                format!("ERROR: {:?}", error)
            }
        }
    }

    /// test2 demonstrates a more complex web search query with multiple terms
    fn test2() -> String {
        let params = web_search::SearchParams {
            query: "WebAssembly WASI components tutorial".to_string(),
            safe_search: None,
            language: None,
            region: None,
            max_results: Some(5),
            time_range: None,
            include_domains: None,
            exclude_domains: None,
            include_images: None,
            include_html: None,
            advanced_answer: None,
        };

        println!("Sending complex web search request...");
        let response = web_search::search_once(&params);
        println!("Response: {:?}", response);

        match response {
            Ok((results, metadata)) => {
                format!(
                    "Found {} results.\nResults: {:?}\nMetadata: {:?}",
                    results.len(),
                    results,
                    metadata
                )
            }
            Err(error) => {
                format!("ERROR: {:?}", error)
            }
        }
    }

    /// test3 demonstrates error handling with an invalid search query
    fn test3() -> String {
        let params = web_search::SearchParams {
            query: "".to_string(),
            safe_search: None,
            language: None,
            region: None,
            max_results: Some(10),
            time_range: None,
            include_domains: None,
            exclude_domains: None,
            include_images: None,
            include_html: None,
            advanced_answer: None,
        };

        println!("Sending empty web search request...");
        let response = web_search::search_once(&params);
        println!("Response: {:?}", response);

        match response {
            Ok((results, _metadata)) => {
                format!("Unexpected success with {} results", results.len())
            }
            Err(error) => {
                format!("Expected error: {:?}", error)
            }
        }
    }

    /// test4 simulates a crash during a web search session, but only first time.
    /// after the automatic recovery it will continue and finish the request successfully.
    fn test4() -> String {
        let params = web_search::SearchParams {
            query: "Golem cloud WebAssembly components".to_string(),
            safe_search: None,
            language: None,
            region: None,
            max_results: Some(10),
            time_range: None,
            include_domains: None,
            exclude_domains: None,
            include_images: None,
            include_html: None,
            advanced_answer: None,
        };

        println!("Starting web search session for durability test...");
        let session = match web_search::start_search(&params) {
            Ok(session) => {
                println!("Created session successfully");
                session
            }
            Err(error) => {
                return format!("Failed to create session: {:?}", error);
            }
        };

        let mut result = String::new();
        let name = std::env::var("GOLEM_WORKER_NAME").unwrap();
        let mut round = 0;

        loop {
            match session.next_page() {
                Ok(search_result) => {
                    println!("Result: {}\n",search_result.title );
                    result.push_str(&format!(
                        "Result: {} ({})\n",
                        search_result.title,
                        search_result.url
                    ));
                }
                Err(error) => {
                    result.push_str(&format!("\n ERROR: {:?}\n", error));
                    break;
                }
            }

            if round == 1 {
                atomically(|| {
                    let client = TestHelperApi::new(&name);
                    let answer = client.blocking_inc_and_get();
                    if answer == 1 {
                        panic!("Simulating crash...")
                    }
                });
            }

            round += 1;
            
            if round >= 2 {
                break;
            }
        }
        result
    }
}

bindings::export!(Component with_types_in bindings);

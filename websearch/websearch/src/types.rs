use serde::{Deserialize, Serialize};
/// A single search result entry returned in the NDJSON stream.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResult {
    /// Kind of message (should be `"result"`)
    pub kind: String,
    /// Title of the search result
    pub title: String,
    /// URL of the result
    pub url: String,
    /// Text snippet summarizing the result
    pub snippet: String,
    /// Display URL (if different from `url`)
    #[serde(rename = "display-url")]
    pub display_url: Option<String>,
    /// Source or provider of the result
    pub source: Option<String>,
    /// Relevance score (if provided)
    pub score: Option<f32>,
    /// HTML-formatted snippet (if available)
    #[serde(rename = "html-snippet")]
    pub html_snippet: Option<String>,
    /// Publication date (if known)
    #[serde(rename = "date-published")]
    pub date_published: Option<String>,
    /// Associated images (if any)
    pub images: Option<Vec<ImageResult>>,
    /// Optional semantic content chunks
    #[serde(rename = "content-chunks")]
    pub content_chunks: Option<Vec<String>>,
}

/// An image associated with a search result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageResult {
    /// Direct image URL
    pub url: String,
    /// Optional description of the image
    pub description: Option<String>,
}

/// Search metadata, typically emitted at the end of a stream.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchMetadata {
    /// Kind of message (should be `"meta"`)
    pub kind: String,
    /// Original query string
    pub query: String,
    /// Total number of results found
    #[serde(rename = "total-results")]
    pub total_results: Option<u64>,
    /// Time taken to perform the search (in milliseconds)
    #[serde(rename = "search-time-ms")]
    pub search_time_ms: Option<f32>,
    /// Safe search level applied
    #[serde(rename = "safe-search")]
    pub safe_search: Option<SafeSearchLevel>,
    /// Language used for the search
    pub language: Option<String>,
    /// Region or locale of the search
    pub region: Option<String>,
    /// Token for fetching the next page
    #[serde(rename = "next-page-token")]
    pub next_page_token: Option<String>,
    /// Rate limit information
    #[serde(rename = "rate-limits")]
    pub rate_limits: Option<RateLimitInfo>,
}

/// Level of safe search filtering.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SafeSearchLevel {
    Off,
    Medium,
    High,
}

/// Metadata about the API's rate limits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RateLimitInfo {
    /// Maximum allowed requests
    pub limit: u32,
    /// Remaining requests before throttling
    pub remaining: u32,
    /// Reset time (epoch milliseconds)
    #[serde(rename = "reset-timestamp")]
    pub reset_timestamp: u64,
}

/// Marker indicating the end of a stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEnd {
    /// Kind of message (should be `"done"`)
    pub kind: String,
}

/// A parsed item from the NDJSON search stream.
#[derive(Debug, Clone, PartialEq)]
pub enum WebsearchStreamEntry {
    /// A search result
    Result(SearchResult),
    /// Summary metadata
    Metadata(SearchMetadata),
    /// Stream termination signal
    Done,
    /// An unrecognized or malformed line
    Unknown(String),
}

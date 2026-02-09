pub mod types {
    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct ImageResult {
        pub url: String,
        pub description: Option<String>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct SearchResult {
        pub title: String,
        pub url: String,
        pub snippet: String,
        pub display_url: Option<String>,
        pub source: Option<String>,
        pub score: Option<f64>,
        pub html_snippet: Option<String>,
        pub date_published: Option<String>,
        pub images: Option<Vec<ImageResult>>,
        pub content_chunks: Option<Vec<String>>,
    }

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum SafeSearchLevel {
        Off,
        Medium,
        High,
    }

    #[derive(
        Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue,
    )]
    pub struct RateLimitInfo {
        pub limit: u32,
        pub remaining: u32,
        pub reset_timestamp: u64,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct SearchMetadata {
        pub query: String,
        pub total_results: Option<u64>,
        pub search_time_ms: Option<f64>,
        pub safe_search: Option<SafeSearchLevel>,
        pub language: Option<String>,
        pub region: Option<String>,
        pub next_page_token: Option<String>,
        pub rate_limits: Option<RateLimitInfo>,
        pub current_page: u32,
    }

    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        golem_rust::FromValueAndType,
        golem_rust::IntoValue,
    )]
    pub enum TimeRange {
        Day,
        Week,
        Month,
        Year,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub struct SearchParams {
        pub query: String,
        pub safe_search: Option<SafeSearchLevel>,
        pub language: Option<String>,
        pub region: Option<String>,
        pub max_results: Option<u32>,
        pub time_range: Option<TimeRange>,
        pub include_domains: Option<Vec<String>>,
        pub exclude_domains: Option<Vec<String>>,
        pub include_images: Option<bool>,
        pub include_html: Option<bool>,
        pub advanced_answer: Option<bool>,
    }

    #[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    pub enum SearchError {
        InvalidQuery,
        RateLimited(u32),
        UnsupportedFeature(String),
        BackendError(String),
    }

    impl core::fmt::Display for SearchError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl std::error::Error for SearchError {}
}

pub mod web_search {
    use crate::SearchSessionInterface;

    pub type SearchParams = super::types::SearchParams;
    pub type SearchResult = super::types::SearchResult;
    pub type SearchMetadata = super::types::SearchMetadata;
    pub type SearchError = super::types::SearchError;

    pub struct SearchSession {
        inner: Box<dyn SearchSessionInterface>,
    }

    impl SearchSession {
        pub fn new<T: SearchSessionInterface>(val: T) -> Self {
            Self {
                inner: Box::new(val),
            }
        }

        pub fn get<T: SearchSessionInterface>(&self) -> &T {
            self.inner
                .as_any()
                .downcast_ref::<T>()
                .expect("SearchSession type mismatch")
        }

        pub fn get_mut<T: SearchSessionInterface>(&mut self) -> &mut T {
            self.inner
                .as_any_mut()
                .downcast_mut::<T>()
                .expect("SearchSession type mismatch")
        }
    }

    impl std::ops::Deref for SearchSession {
        type Target = dyn SearchSessionInterface;

        fn deref(&self) -> &Self::Target {
            &*self.inner
        }
    }

    impl std::ops::DerefMut for SearchSession {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut *self.inner
        }
    }

    impl std::fmt::Debug for SearchSession {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("SearchSession").finish()
        }
    }
}

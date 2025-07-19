use crate::exports::golem::web_search::web_search::Guest;
use crate::exports::golem::web_search::web_search::{SearchError, SearchParams, SearchResult};
use golem_rust::wasm_rpc::Pollable;
use std::marker::PhantomData;

/// Wraps a websearch implementation with custom durability
pub struct Durablewebsearch<Impl> {
    phantom: PhantomData<Impl>,
}

/// Trait to be implemented in addition to the websearch `Guest` trait when wrapping it with `Durablewebsearch`.
pub trait ExtendedwebsearchGuest: Guest + 'static {
    /// Creates an instance of the websearch specific `SearchSession` without wrapping it in a `Resource`
    fn unwrapped_search_session(params: SearchParams) -> Result<Self::SearchSession, SearchError>;

    /// Creates the retry prompt with a combination of the original search params, and the partially received
    /// search results. There is a default implementation here, but it can be overridden with provider-specific
    /// parameters if needed.
    fn retry_params(
        original_params: &SearchParams,
        partial_results: &[SearchResult],
    ) -> SearchParams {
        // For search, we typically want to continue from where we left off
        // This could involve adjusting max_results or using pagination tokens
        let mut retry_params = original_params.clone();

        if let Some(max_results) = retry_params.max_results {
            // Reduce max_results by the number of results we already have
            let remaining = max_results.saturating_sub(partial_results.len() as u32);
            retry_params.max_results = Some(remaining.max(1));
        }

        retry_params
    }

    #[allow(dead_code)]
    fn subscribe(session: &Self::SearchSession) -> Pollable;
}

/// When the durability feature flag is off, wrapping with `Durablewebsearch` is just a passthrough
#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use crate::durability::{Durablewebsearch, ExtendedwebsearchGuest};
    use crate::golem::web_search::web_search::{Guest, SearchSession};
    use crate::golem::web_search::web_search::{
        SearchError, SearchMetadata, SearchParams, SearchResult,
    };

    impl<Impl: ExtendedwebsearchGuest> Guest for Durablewebsearch<Impl> {
        type SearchSession = Impl::SearchSession;

        fn start_search(params: SearchParams) -> Result<SearchSession, SearchError> {
            Impl::start_search(params)
        }

        fn search_once(
            params: SearchParams,
        ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
            Impl::search_once(params)
        }
    }
}

/// When the durability feature flag is on, wrapping with `Durablewebsearch` adds custom durability
/// on top of the provider-specific websearch implementation using Golem's special host functions and
/// the `golem-rust` helper library.
///
/// There will be custom durability entries saved in the oplog, with the full websearch request and configuration
/// stored as input, and the full response stored as output. To serialize these in a way it is
/// observable by oplog consumers, each relevant data type has to be converted to/from `ValueAndType`
/// which is implemented using the type classes and builder in the `golem-rust` library.
#[cfg(feature = "durability")]
mod durable_impl {
    use crate::durability::{Durablewebsearch, ExtendedwebsearchGuest};
    use crate::exports::golem::web_search::web_search::{Guest, GuestSearchSession, SearchSession};
    use crate::exports::golem::web_search::web_search::{
        SearchError, SearchMetadata, SearchParams, SearchResult,
    };
    use golem_rust::bindings::golem::durability::durability::{
        DurableFunctionType, LazyInitializedPollable,
    };
    use golem_rust::durability::Durability;
    use golem_rust::wasm_rpc::Pollable;
    use golem_rust::{with_persistence_level, FromValueAndType, IntoValue, PersistenceLevel};
    use std::cell::RefCell;
    use std::fmt::{Display, Formatter};

    impl<Impl: ExtendedwebsearchGuest> Guest for Durablewebsearch<Impl> {
        type SearchSession = DurableSearchSession<Impl>;

        fn start_search(params: SearchParams) -> Result<SearchSession, SearchError> {
            let durability = Durability::<SearchParams, UnusedError>::new(
                "golem_websearch",
                "start_search",
                DurableFunctionType::WriteRemote,
            );

            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    match Impl::start_search(params.clone()) {
                        Ok(_session) => Ok(params.clone()),
                        Err(e) => Err(e),
                    }
                });

                let persisted_params = result?;
                durability.persist_infallible(
                    StartSearchInput {
                        params: persisted_params.clone(),
                    },
                    persisted_params.clone(),
                );
                Ok(SearchSession::new(DurableSearchSession::<Impl>::live(
                    Impl::unwrapped_search_session(persisted_params).unwrap(),
                )))
            } else {
                let result = durability.replay_infallible();
                let session = SearchSession::new(DurableSearchSession::<Impl>::replay(result));
                Ok(session)
            }
        }
        fn search_once(
            params: SearchParams,
        ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
            let durability =
                Durability::<(Vec<SearchResult>, Option<SearchMetadata>), UnusedError>::new(
                    "golem_websearch",
                    "search_once",
                    DurableFunctionType::WriteRemote,
                );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::search_once(params.clone())
                });
                let (results, metadata) = result?;
                durability.persist_infallible(
                    SearchOnceInput { params },
                    (results.clone(), metadata.clone()),
                );
                Ok((results, metadata))
            } else {
                let result: (Vec<SearchResult>, Option<SearchMetadata>) =
                    durability.replay_infallible();
                let (results, metadata) = result;
                Ok((results, metadata))
            }
        }
    }

    /// Represents the durable search session's state
    ///
    /// In live mode it directly calls the underlying websearch session which is implemented on
    /// top of HTTP requests to search providers.
    ///
    /// In replay mode it buffers the replayed search results, and also tracks the created pollables
    /// to be able to reattach them to the new live session when the switch to live mode
    /// happens.
    ///
    /// When reaching the end of the replay mode, if the replayed session was not finished yet,
    /// the retry parameters implemented in `ExtendedwebsearchGuest` is used to create a new websearch session
    /// and continue the search seamlessly.
    enum DurableSearchSessionState<Impl: ExtendedwebsearchGuest> {
        Live {
            session: Impl::SearchSession,
            pollables: Vec<LazyInitializedPollable>,
        },
        Replay {
            original_params: SearchParams,
            pollables: Vec<LazyInitializedPollable>,
            partial_results: Vec<SearchResult>,
            metadata: Box<Option<SearchMetadata>>,
            finished: bool,
        },
    }

    pub struct DurableSearchSession<Impl: ExtendedwebsearchGuest> {
        state: RefCell<Option<DurableSearchSessionState<Impl>>>,
        subscription: RefCell<Option<Pollable>>,
    }

    impl<Impl: ExtendedwebsearchGuest> DurableSearchSession<Impl> {
        fn live(session: Impl::SearchSession) -> Self {
            Self {
                state: RefCell::new(Some(DurableSearchSessionState::Live {
                    session,
                    pollables: Vec::new(),
                })),
                subscription: RefCell::new(None),
            }
        }

        fn replay(original_params: SearchParams) -> Self {
            Self {
                state: RefCell::new(Some(DurableSearchSessionState::Replay {
                    original_params,
                    pollables: Vec::new(),
                    partial_results: Vec::new(),
                    metadata: Box::new(None),
                    finished: false,
                })),
                subscription: RefCell::new(None),
            }
        }

        #[allow(dead_code)]
        fn subscribe(&self) -> Pollable {
            let mut state = self.state.borrow_mut();
            match &mut *state {
                Some(DurableSearchSessionState::Live { session, .. }) => Impl::subscribe(session),
                Some(DurableSearchSessionState::Replay { pollables, .. }) => {
                    let lazy_pollable = LazyInitializedPollable::new();
                    let pollable = lazy_pollable.subscribe();
                    pollables.push(lazy_pollable);
                    pollable
                }
                None => {
                    unreachable!()
                }
            }
        }
    }

    impl<Impl: ExtendedwebsearchGuest> Drop for DurableSearchSession<Impl> {
        fn drop(&mut self) {
            let _ = self.subscription.take();
            match self.state.take() {
                Some(DurableSearchSessionState::Live {
                    mut pollables,
                    session,
                }) => {
                    with_persistence_level(PersistenceLevel::PersistNothing, move || {
                        pollables.clear();
                        drop(session);
                    });
                }
                Some(DurableSearchSessionState::Replay { mut pollables, .. }) => {
                    pollables.clear();
                }
                None => {}
            }
        }
    }

    impl<Impl: ExtendedwebsearchGuest> GuestSearchSession for DurableSearchSession<Impl> {
        fn next_page(&self) -> Result<Vec<SearchResult>, SearchError> {
            let durability = Durability::<Vec<SearchResult>, UnusedError>::new(
                "golem_websearch",
                "next_page",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let mut state = self.state.borrow_mut();
                let (result, new_live_session) = match &mut *state {
                    Some(DurableSearchSessionState::Live { session, .. }) => {
                        let result =
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                session.next_page()
                            });
                        let value = result?;
                        (Ok(durability.persist_infallible(NoInput, value)), None)
                    }
                    Some(DurableSearchSessionState::Replay {
                        original_params,
                        pollables,
                        partial_results,
                        finished,
                        ..
                    }) => {
                        if *finished {
                            (Ok(Vec::new()), None)
                        } else {
                            let retry_params = Impl::retry_params(original_params, partial_results);
                            let (session, first_live_result) =
                                with_persistence_level(PersistenceLevel::PersistNothing, || {
                                    let session =
                                        <Impl as ExtendedwebsearchGuest>::unwrapped_search_session(
                                            retry_params,
                                        )
                                        .unwrap();
                                    for lazy_initialized_pollable in pollables {
                                        lazy_initialized_pollable.set(Impl::subscribe(&session));
                                    }
                                    let next = session.next_page();
                                    (session, next)
                                });
                            let value = first_live_result.clone()?;
                            // Append new results to partial_results
                            partial_results.extend(value.clone());
                            let _ = durability.persist_infallible(NoInput, value.clone());
                            (Ok(value), Some(session))
                        }
                    }
                    None => {
                        unreachable!()
                    }
                };

                if let Some(session) = new_live_session {
                    let pollables = match state.take() {
                        Some(DurableSearchSessionState::Live { pollables, .. }) => pollables,
                        Some(DurableSearchSessionState::Replay { pollables, .. }) => pollables,
                        None => {
                            unreachable!()
                        }
                    };
                    *state = Some(DurableSearchSessionState::Live { session, pollables });
                }

                result
            } else {
                let result: Vec<SearchResult> = durability.replay_infallible();
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurableSearchSessionState::Live { .. }) => {
                        unreachable!("Durable search session cannot be in live mode during replay");
                    }
                    Some(DurableSearchSessionState::Replay {
                        partial_results: _,
                        finished: _,
                        ..
                    }) => Ok(result),
                    None => {
                        unreachable!();
                    }
                }
            }
        }

        fn get_metadata(&self) -> Option<SearchMetadata> {
            let durability = Durability::<Option<SearchMetadata>, UnusedError>::new(
                "golem_websearch",
                "get_metadata",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableSearchSessionState::Live { session, .. }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            session.get_metadata()
                        })
                    }
                    Some(DurableSearchSessionState::Replay {
                        original_params,
                        partial_results,
                        ..
                    }) => Some(SearchMetadata {
                        query: original_params.query.clone(),
                        total_results: Some(partial_results.len() as u64),
                        search_time_ms: None,
                        safe_search: None,
                        language: None,
                        region: None,
                        next_page_token: None,
                        rate_limits: None,
                        current_page: partial_results.len() as u32,
                    }),
                    None => {
                        unreachable!()
                    }
                };
                let _ = durability.persist_infallible(NoInput, result.clone());
                result
            } else {
                let result: Option<SearchMetadata> = durability.replay_infallible();
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurableSearchSessionState::Live { .. }) => {
                        unreachable!("Durable search session cannot be in live mode during replay");
                    }
                    Some(DurableSearchSessionState::Replay { metadata, .. }) => {
                        *metadata = Box::new(result.clone());
                    }
                    None => {
                        unreachable!();
                    }
                }
                result
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, IntoValue)]
    struct StartSearchInput {
        params: SearchParams,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue)]
    struct SearchOnceInput {
        params: SearchParams,
    }

    #[derive(Debug, IntoValue)]
    struct NoInput;

    #[derive(Debug, FromValueAndType, IntoValue)]
    struct UnusedError;

    impl Display for UnusedError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "UnusedError")
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::durability::durable_impl::{SearchOnceInput, StartSearchInput};
        use crate::golem::web_search::types::{
            ImageResult, RateLimitInfo, SafeSearchLevel, TimeRange,
        };
        use crate::golem::web_search::web_search::{
            SearchError, SearchMetadata, SearchParams, SearchResult,
        };
        use golem_rust::value_and_type::{FromValueAndType, IntoValueAndType};
        use golem_rust::wasm_rpc::WitTypeNode;
        use std::fmt::Debug;

        fn roundtrip_test<T: Debug + Clone + PartialEq + IntoValueAndType + FromValueAndType>(
            value: T,
        ) {
            let vnt = value.clone().into_value_and_type();
            let extracted = T::from_value_and_type(vnt).unwrap();
            assert_eq!(value, extracted);
        }

        #[test]
        fn safe_search_level_roundtrip() {
            roundtrip_test(SafeSearchLevel::Off);
            roundtrip_test(SafeSearchLevel::Medium);
            roundtrip_test(SafeSearchLevel::High);
        }

        #[test]
        fn time_range_roundtrip() {
            roundtrip_test(TimeRange::Day);
            roundtrip_test(TimeRange::Week);
            roundtrip_test(TimeRange::Month);
            roundtrip_test(TimeRange::Year);
        }

        #[test]
        fn search_error_roundtrip() {
            roundtrip_test(SearchError::InvalidQuery);
            roundtrip_test(SearchError::RateLimited(3600));
            roundtrip_test(SearchError::UnsupportedFeature(
                "advanced search".to_string(),
            ));
            roundtrip_test(SearchError::BackendError("Service unavailable".to_string()));
        }

        #[test]
        fn image_result_roundtrip() {
            roundtrip_test(ImageResult {
                url: "https://example.com/image.png".to_string(),
                description: Some("A sample image".to_string()),
            });
            roundtrip_test(ImageResult {
                url: "https://example.com/image2.jpg".to_string(),
                description: None,
            });
        }

        #[test]
        fn rate_limit_info_roundtrip() {
            roundtrip_test(RateLimitInfo {
                limit: 1000,
                remaining: 500,
                reset_timestamp: 1698761200,
            });
        }

        #[test]
        fn search_result_roundtrip() {
            roundtrip_test(SearchResult {
                title: "Sample Search Result".to_string(),
                url: "https://example.com/page".to_string(),
                snippet: "This is a sample search result snippet".to_string(),
                display_url: Some("example.com/page".to_string()),
                source: Some("Example Website".to_string()),
                score: Some(0.95),
                html_snippet: Some("<p>This is a sample search result snippet</p>".to_string()),
                date_published: Some("2023-10-01".to_string()),
                images: Some(vec![ImageResult {
                    url: "https://example.com/thumb.jpg".to_string(),
                    description: Some("Thumbnail".to_string()),
                }]),
                content_chunks: Some(vec![
                    "First chunk of content".to_string(),
                    "Second chunk of content".to_string(),
                ]),
            });
        }

        #[test]
        fn search_metadata_roundtrip() {
            roundtrip_test(SearchMetadata {
                query: "sample search query".to_string(),
                total_results: Some(1500),
                search_time_ms: Some(125.5),
                safe_search: Some(SafeSearchLevel::Medium),
                language: Some("en".to_string()),
                region: Some("US".to_string()),
                next_page_token: Some("next_page_123".to_string()),
                rate_limits: Some(RateLimitInfo {
                    limit: 1000,
                    remaining: 999,
                    reset_timestamp: 1698761200,
                }),
                current_page: 0,
            });
        }

        #[test]
        fn search_params_roundtrip() {
            roundtrip_test(SearchParams {
                query: "rust programming language".to_string(),
                safe_search: Some(SafeSearchLevel::High),
                language: Some("en".to_string()),
                region: Some("US".to_string()),
                max_results: Some(50),
                time_range: Some(TimeRange::Month),
                include_domains: Some(vec![
                    "rust-lang.org".to_string(),
                    "doc.rust-lang.org".to_string(),
                ]),
                exclude_domains: Some(vec!["spam.com".to_string()]),
                include_images: Some(true),
                include_html: Some(false),
                advanced_answer: Some(true),
            });
        }

        #[test]
        fn start_search_input_encoding() {
            let input = StartSearchInput {
                params: SearchParams {
                    query: "machine learning tutorials".to_string(),
                    safe_search: Some(SafeSearchLevel::Medium),
                    language: Some("en".to_string()),
                    region: Some("US".to_string()),
                    max_results: Some(25),
                    time_range: Some(TimeRange::Week),
                    include_domains: Some(vec![
                        "github.com".to_string(),
                        "stackoverflow.com".to_string(),
                    ]),
                    exclude_domains: Some(vec!["ads.com".to_string()]),
                    include_images: Some(true),
                    include_html: Some(true),
                    advanced_answer: Some(false),
                },
            };

            let encoded = input.into_value_and_type();
            println!("{encoded:#?}");

            for wit_type in encoded.typ.nodes {
                if let WitTypeNode::ListType(idx) = wit_type {
                    assert!(idx >= 0);
                }
            }
        }

        #[test]
        fn search_once_input_encoding() {
            let input = SearchOnceInput {
                params: SearchParams {
                    query: "web development best practices".to_string(),
                    safe_search: Some(SafeSearchLevel::Off),
                    language: Some("en".to_string()),
                    region: Some("GB".to_string()),
                    max_results: Some(10),
                    time_range: None,
                    include_domains: None,
                    exclude_domains: None,
                    include_images: Some(false),
                    include_html: Some(true),
                    advanced_answer: Some(true),
                },
            };

            let encoded = input.into_value_and_type();
            println!("{encoded:#?}");

            for wit_type in encoded.typ.nodes {
                if let WitTypeNode::ListType(idx) = wit_type {
                    assert!(idx >= 0);
                }
            }
        }
    }
}

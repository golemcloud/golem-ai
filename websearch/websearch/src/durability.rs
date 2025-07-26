use crate::exports::golem::web_search::web_search::Guest;
use crate::exports::golem::web_search::web_search::{SearchError, SearchParams};
use golem_rust::value_and_type::{FromValueAndType, IntoValue as IntoValueTrait};
use std::marker::PhantomData;

/// Wraps a websearch implementation with custom durability
pub struct Durablewebsearch<Impl> {
    phantom: PhantomData<Impl>,
}

/// Trait to be implemented in addition to the websearch `Guest` trait when wrapping it with `Durablewebsearch`.
pub trait ExtendedwebsearchGuest: Guest + 'static {
    type ReplayState: std::fmt::Debug + Clone + IntoValueTrait + FromValueAndType;

    /// Creates an instance of the websearch specific `SearchSession` without wrapping it in a `Resource`
    fn unwrapped_search_session(params: SearchParams) -> Result<Self::SearchSession, SearchError>;

    /// Used at the end of replay to go from replay to live mode
    fn session_to_state(session: &Self::SearchSession) -> Self::ReplayState;
    fn session_from_state(
        state: &Self::ReplayState,
        params: SearchParams,
    ) -> Result<Self::SearchSession, SearchError>;
}

/// When the durability feature flag is off, wrapping with `Durablewebsearch` is just a passthrough
#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use crate::durability::{Durablewebsearch, ExtendedwebsearchGuest};
    use crate::golem::web_search::web_search::{Guest, SearchSession};
    use crate::golem::web_search::web_search::{
        SearchError, SearchMetadata, SearchParams, SearchResult,
    };
    use crate::init_logging;

    impl<Impl: ExtendedwebsearchGuest> Guest for Durablewebsearch<Impl> {
        type SearchSession = Impl::SearchSession;

        fn start_search(params: SearchParams) -> Result<SearchSession, SearchError> {
            init_logging();
            Impl::start_search(params)
        }

        fn search_once(
            params: SearchParams,
        ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
            init_logging();
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
    use crate::init_logging;
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    use golem_rust::durability::Durability;
    use golem_rust::{with_persistence_level, PersistenceLevel};
    use std::cell::RefCell;

    #[derive(Debug, golem_rust::IntoValue)]
    struct NoInput;

    // Add the From implementation for SearchError to satisfy the Durability trait bounds
    impl From<&SearchError> for SearchError {
        fn from(error: &SearchError) -> Self {
            error.clone()
        }
    }

    impl<Impl: ExtendedwebsearchGuest> Guest for Durablewebsearch<Impl> {
        type SearchSession = DurableSearchSession<Impl>;

        fn start_search(params: SearchParams) -> Result<SearchSession, SearchError> {
            init_logging();

            let durability = Durability::<Impl::ReplayState, SearchError>::new(
                "golem_websearch",
                "start_search",
                DurableFunctionType::WriteRemote,
            );

            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::unwrapped_search_session(params.clone())
                });

                match result {
                    Ok(session) => {
                        let replay_state = Impl::session_to_state(&session);
                        let _ = durability.persist(params.clone(), Ok(replay_state));
                        Ok(SearchSession::new(DurableSearchSession::<Impl>::live(
                            session, params,
                        )))
                    }
                    Err(error) => {
                        let _ = durability.persist(params.clone(), Err(error.clone()));
                        Err(error)
                    }
                }
            } else {
                let replay_state = durability.replay::<Impl::ReplayState, SearchError>()?;
                let session = DurableSearchSession::<Impl>::replay(replay_state, params)?;
                Ok(SearchSession::new(session))
            }
        }

        fn search_once(
            params: SearchParams,
        ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
            init_logging();

            let durability =
                Durability::<(Vec<SearchResult>, Option<SearchMetadata>), SearchError>::new(
                    "golem_websearch",
                    "search_once",
                    DurableFunctionType::WriteRemote,
                );

            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::search_once(params.clone())
                });
                durability.persist(params, result)
            } else {
                durability.replay()
            }
        }
    }

    /// Represents the durable search session's state
    ///
    /// In live mode it directly calls the underlying websearch session which is implemented on
    /// top of HTTP requests to search providers.
    ///
    /// In replay mode it uses the replay state to reconstruct the session state accurately,
    /// tracking accumulated results and metadata.
    ///
    /// When reaching the end of the replay mode, if the replayed session was not finished yet,
    /// the retry parameters implemented in `ExtendedwebsearchGuest` is used to create a new websearch session
    /// and continue the search seamlessly.
    enum DurableSearchSessionState<Impl: ExtendedwebsearchGuest> {
        Live { session: Impl::SearchSession },
        Replay { replay_state: Impl::ReplayState },
    }

    pub struct DurableSearchSession<Impl: ExtendedwebsearchGuest> {
        state: RefCell<Option<DurableSearchSessionState<Impl>>>,
        params: SearchParams,
    }

    impl<Impl: ExtendedwebsearchGuest> DurableSearchSession<Impl> {
        fn live(session: Impl::SearchSession, params: SearchParams) -> Self {
            Self {
                state: RefCell::new(Some(DurableSearchSessionState::Live { session })),
                params,
            }
        }

        fn replay(
            replay_state: Impl::ReplayState,
            params: SearchParams,
        ) -> Result<Self, SearchError> {
            Ok(Self {
                state: RefCell::new(Some(DurableSearchSessionState::Replay { replay_state })),
                params,
            })
        }
    }

    impl<Impl: ExtendedwebsearchGuest> Drop for DurableSearchSession<Impl> {
        fn drop(&mut self) {
            match self.state.take() {
                Some(DurableSearchSessionState::Live { session }) => {
                    with_persistence_level(PersistenceLevel::PersistNothing, move || {
                        drop(session);
                    });
                }
                Some(DurableSearchSessionState::Replay { .. }) => {
                    // Nothing special to clean up for replay state
                }
                None => {}
            }
        }
    }

    impl<Impl: ExtendedwebsearchGuest> GuestSearchSession for DurableSearchSession<Impl> {
        fn next_page(&self) -> Result<Vec<SearchResult>, SearchError> {
            let durability = Durability::<(Vec<SearchResult>, Impl::ReplayState), SearchError>::new(
                "golem_websearch",
                "next_page",
                DurableFunctionType::ReadRemote,
            );

            if durability.is_live() {
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurableSearchSessionState::Live { session }) => {
                        let result =
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                session.next_page()
                            });

                        match result {
                            Ok(value) => {
                                let replay_state = Impl::session_to_state(session);
                                let persisted_result = durability
                                    .persist(NoInput, Ok((value.clone(), replay_state)))?;
                                Ok(persisted_result.0)
                            }
                            Err(error) => {
                                let _ = durability.persist::<
                                    _,
                                    (Vec<SearchResult>, Impl::ReplayState),
                                    SearchError
                                >(NoInput, Err(error.clone()));
                                Err(error)
                            }
                        }
                    }
                    Some(DurableSearchSessionState::Replay { replay_state }) => {
                        let session = Impl::session_from_state(replay_state, self.params.clone())?;
                        let result =
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                session.next_page()
                            });

                        match result {
                            Ok(value) => {
                                let new_replay_state = Impl::session_to_state(&session);
                                let persisted_result = durability
                                    .persist(NoInput, Ok((value.clone(), new_replay_state)))?;
                                *state = Some(DurableSearchSessionState::Live { session });
                                Ok(persisted_result.0)
                            }
                            Err(error) => {
                                let _ = durability.persist::<
                                    _,
                                    (Vec<SearchResult>, Impl::ReplayState),
                                    SearchError
                                >(NoInput, Err(error.clone()));
                                Err(error)
                            }
                        }
                    }
                    None => unreachable!(),
                }
            } else {
                let (result, next_replay_state) =
                    durability.replay::<(Vec<SearchResult>, Impl::ReplayState), SearchError>()?;
                let mut state = self.state.borrow_mut();

                match &mut *state {
                    Some(DurableSearchSessionState::Live { .. }) => {
                        unreachable!("Durable search session cannot be in live mode during replay");
                    }
                    Some(DurableSearchSessionState::Replay { replay_state: _ }) => {
                        *state = Some(DurableSearchSessionState::Replay {
                            replay_state: next_replay_state.clone(),
                        });
                        Ok(result)
                    }
                    None => {
                        unreachable!();
                    }
                }
            }
        }

        fn get_metadata(&self) -> Option<SearchMetadata> {
            let state = self.state.borrow();
            match &*state {
                Some(DurableSearchSessionState::Live { session }) => {
                    with_persistence_level(PersistenceLevel::PersistNothing, || {
                        session.get_metadata()
                    })
                }
                Some(DurableSearchSessionState::Replay { replay_state }) => {
                    let session =
                        Impl::session_from_state(replay_state, self.params.clone()).ok()?;
                    session.get_metadata()
                }
                None => {
                    unreachable!()
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
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
            let input = SearchParams {
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
            let input = SearchParams {
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

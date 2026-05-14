use crate::WebSearchProvider;
use std::marker::PhantomData;

#[cfg(feature = "golem")]
use crate::model::web_search::{SearchError, SearchParams};
#[cfg(feature = "golem")]
use golem_rust::value_and_type::{FromValueAndType, IntoValue as IntoValueTrait};

/// Wraps a websearch implementation with custom durability
pub struct DurableWebSearch<Impl> {
    phantom: PhantomData<Impl>,
}

/// Trait to be implemented in addition to the websearch `WebSearchProvider` trait when wrapping
/// it with `DurableWebSearch`.
#[cfg(feature = "golem")]
pub trait ExtendedWebSearchProvider: WebSearchProvider + 'static {
    type ReplayState: std::fmt::Debug + Clone + IntoValueTrait + FromValueAndType;

    /// Creates an instance of the websearch specific `SearchSession` without wrapping it in a `Resource`
    fn unwrapped_search_session(
        provider_config: Self::ProviderConfig,
        params: SearchParams,
    ) -> Result<Self::SearchSession, SearchError>;

    /// Used at the end of replay to go from replay to live mode
    fn session_to_state(session: &Self::SearchSession) -> Self::ReplayState;
    fn session_from_state(
        provider_config: Self::ProviderConfig,
        state: &Self::ReplayState,
        params: SearchParams,
    ) -> Result<Self::SearchSession, SearchError>;
}

/// Trait to be implemented in addition to the websearch `WebSearchProvider` trait when wrapping
/// it with `DurableWebSearch`. When the `golem` feature is off, no replay support is needed.
#[cfg(not(feature = "golem"))]
pub trait ExtendedWebSearchProvider: WebSearchProvider + 'static {}

/// When the durability feature flag is off, wrapping with `DurableWebSearch` is just a passthrough
#[cfg(not(feature = "golem"))]
mod passthrough_impl {
    use crate::durability::{DurableWebSearch, ExtendedWebSearchProvider};
    use crate::init_logging;
    use crate::model::web_search::SearchSession;
    use crate::model::web_search::{SearchError, SearchMetadata, SearchParams, SearchResult};
    use crate::WebSearchProvider;

    impl<Impl: ExtendedWebSearchProvider> WebSearchProvider for DurableWebSearch<Impl> {
        type SearchSession = Impl::SearchSession;
        type ProviderConfig = Impl::ProviderConfig;

        fn start_search(
            provider_config: Self::ProviderConfig,
            params: SearchParams,
        ) -> Result<SearchSession, SearchError> {
            init_logging();
            Impl::start_search(provider_config, params)
        }

        fn search_once(
            provider_config: Self::ProviderConfig,
            params: SearchParams,
        ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
            init_logging();
            Impl::search_once(provider_config, params)
        }
    }
}

/// When the durability feature flag is on, wrapping with `DurableWebSearch` adds custom durability
/// on top of the provider-specific websearch implementation using Golem's special host functions and
/// the `golem-rust` helper library.
///
/// There will be custom durability entries saved in the oplog, with the full websearch request
/// stored as input, and the full response stored as output. To serialize these in a way it is
/// observable by oplog consumers, each relevant data type has to be converted to/from `ValueAndType`
/// which is implemented using the type classes and builder in the `golem-rust` library.
///
/// The `provider_config` is intentionally **not** persisted in the input payloads because it
/// can carry secrets (API keys etc.). Instead, every replay path expects the caller to supply
/// a fresh `provider_config`, which is captured inside the durable session so that subsequent
/// `next_page` requests can re-resolve any contained secrets right before issuing each HTTP call.
#[cfg(feature = "golem")]
mod durable_impl {
    use crate::durability::{DurableWebSearch, ExtendedWebSearchProvider};
    use crate::model::web_search::SearchSession;
    use crate::model::web_search::{SearchError, SearchMetadata, SearchParams, SearchResult};
    use crate::{init_logging, SearchSessionInterface, WebSearchProvider};
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    use golem_rust::durability::Durability;
    use golem_rust::{with_persistence_level, FromValueAndType, IntoValue, PersistenceLevel};
    use std::cell::RefCell;

    #[derive(Debug, golem_rust::IntoValue)]
    struct NoInput;

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct StartSearchInput {
        params: SearchParams,
    }

    #[derive(Debug, Clone, PartialEq, IntoValue, FromValueAndType)]
    struct SearchOnceInput {
        params: SearchParams,
    }

    // Add the From implementation for SearchError to satisfy the Durability trait bounds
    impl From<&SearchError> for SearchError {
        fn from(error: &SearchError) -> Self {
            error.clone()
        }
    }

    impl<Impl: ExtendedWebSearchProvider> WebSearchProvider for DurableWebSearch<Impl> {
        type SearchSession = DurableSearchSession<Impl>;
        type ProviderConfig = Impl::ProviderConfig;

        fn start_search(
            provider_config: Self::ProviderConfig,
            params: SearchParams,
        ) -> Result<SearchSession, SearchError> {
            init_logging();

            let durability = Durability::<Impl::ReplayState, SearchError>::new(
                "golem_websearch",
                "start_search",
                DurableFunctionType::WriteRemote,
            );

            if durability.is_live() {
                let provider_config_for_call = provider_config.clone();
                let params_for_call = params.clone();
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::unwrapped_search_session(provider_config_for_call, params_for_call)
                });

                match result {
                    Ok(session) => {
                        let replay_state = Impl::session_to_state(&session);
                        // NOTE: `provider_config` deliberately not included in the persisted
                        // input, because it can carry secrets (API keys etc.).
                        let _ = durability.persist(
                            StartSearchInput {
                                params: params.clone(),
                            },
                            Ok(replay_state),
                        );
                        Ok(SearchSession::new(DurableSearchSession::<Impl>::live(
                            provider_config,
                            session,
                            params,
                        )))
                    }
                    Err(error) => {
                        let _ = durability.persist(
                            StartSearchInput {
                                params: params.clone(),
                            },
                            Err(error.clone()),
                        );
                        Err(error)
                    }
                }
            } else {
                let replay_state = durability.replay::<Impl::ReplayState, SearchError>()?;
                let session =
                    DurableSearchSession::<Impl>::replay(provider_config, replay_state, params)?;
                Ok(SearchSession::new(session))
            }
        }

        fn search_once(
            provider_config: Self::ProviderConfig,
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
                let params_for_call = params.clone();
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::search_once(provider_config, params_for_call)
                });
                // NOTE: `provider_config` deliberately not included in the persisted input,
                // because it can carry secrets (API keys etc.).
                durability.persist(SearchOnceInput { params }, result)
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
    /// the retry parameters implemented in `ExtendedWebSearchProvider` is used to create a new
    /// websearch session and continue the search seamlessly. The `provider_config` (which carries
    /// any secrets) is kept inside this struct so that subsequent live `next_page` requests can
    /// re-resolve those secrets immediately before each HTTP call.
    enum DurableSearchSessionState<Impl: ExtendedWebSearchProvider> {
        Live { session: Impl::SearchSession },
        Replay { replay_state: Impl::ReplayState },
    }

    pub struct DurableSearchSession<Impl: ExtendedWebSearchProvider> {
        provider_config: Impl::ProviderConfig,
        state: RefCell<Option<DurableSearchSessionState<Impl>>>,
        params: SearchParams,
    }

    impl<Impl: ExtendedWebSearchProvider> DurableSearchSession<Impl> {
        fn live(
            provider_config: Impl::ProviderConfig,
            session: Impl::SearchSession,
            params: SearchParams,
        ) -> Self {
            Self {
                provider_config,
                state: RefCell::new(Some(DurableSearchSessionState::Live { session })),
                params,
            }
        }

        fn replay(
            provider_config: Impl::ProviderConfig,
            replay_state: Impl::ReplayState,
            params: SearchParams,
        ) -> Result<Self, SearchError> {
            Ok(Self {
                provider_config,
                state: RefCell::new(Some(DurableSearchSessionState::Replay { replay_state })),
                params,
            })
        }
    }

    impl<Impl: ExtendedWebSearchProvider> Drop for DurableSearchSession<Impl> {
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

    impl<Impl: ExtendedWebSearchProvider> SearchSessionInterface for DurableSearchSession<Impl> {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }

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
                        let session = Impl::session_from_state(
                            self.provider_config.clone(),
                            replay_state,
                            self.params.clone(),
                        )?;
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
                    let session = Impl::session_from_state(
                        self.provider_config.clone(),
                        replay_state,
                        self.params.clone(),
                    )
                    .ok()?;
                    session.get_metadata()
                }
                None => {
                    unreachable!()
                }
            }
        }
    }
}

use crate::WebSearchProvider;
use std::marker::PhantomData;

#[cfg(feature = "golem")]
use crate::model::web_search::{SearchError, SearchParams};
#[cfg(feature = "golem")]
use golem_rust::{FromSchema, IntoSchema};

/// Wraps a websearch implementation with custom durability
pub struct DurableWebSearch<Impl> {
    phantom: PhantomData<Impl>,
}

/// Trait to be implemented in addition to the websearch `WebSearchProvider` trait when wrapping
/// it with `DurableWebSearch`.
#[cfg(feature = "golem")]
pub trait ExtendedWebSearchProvider: WebSearchProvider + 'static {
    type ReplayState: std::fmt::Debug + Clone + IntoSchema + FromSchema;

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

        async fn search_once(
            provider_config: Self::ProviderConfig,
            params: SearchParams,
        ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
            init_logging();
            Impl::search_once(provider_config, params).await
        }
    }
}

/// When the durability feature flag is on, wrapping with `DurableWebSearch` adds custom durability
/// on top of the provider-specific websearch implementation using Golem's special host functions and
/// the `golem-rust` helper library.
///
/// There will be custom durability entries saved in the oplog, with the full websearch request
/// stored as input, and the full response stored as output. To serialize these in a way it is
/// observable by oplog consumers, each relevant data type has to implement the schema conversion
/// traits provided by the `golem-rust` library.
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
    use crate::{init_logging, SearchPageFuture, SearchSessionInterface, WebSearchProvider};
    use golem_rust::durability::{Durability, DurableFunctionType};
    use golem_rust::{FromSchema, IntoSchema};
    use std::cell::RefCell;

    #[derive(Debug, Clone, IntoSchema)]
    struct NoInput;

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
    struct StartSearchInput {
        params: SearchParams,
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema, FromSchema)]
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
                &StartSearchInput {
                    params: params.clone(),
                },
            );

            let mut session = None;
            let replay_state = durability.run(|| {
                let created =
                    Impl::unwrapped_search_session(provider_config.clone(), params.clone())?;
                let replay_state = Impl::session_to_state(&created);
                session = Some(created);
                Ok(replay_state)
            })?;
            // NOTE: `provider_config` deliberately not included in the persisted input,
            // because it can carry secrets (API keys etc.).
            if let Some(session) = session {
                Ok(SearchSession::new(DurableSearchSession::<Impl>::live(
                    provider_config,
                    session,
                    params,
                )))
            } else {
                let session =
                    DurableSearchSession::<Impl>::replay(provider_config, replay_state, params)?;
                Ok(SearchSession::new(session))
            }
        }

        async fn search_once(
            provider_config: Self::ProviderConfig,
            params: SearchParams,
        ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
            init_logging();

            let durability =
                Durability::<(Vec<SearchResult>, Option<SearchMetadata>), SearchError>::new(
                    "golem_websearch",
                    "search_once",
                    DurableFunctionType::WriteRemote,
                    &SearchOnceInput {
                        params: params.clone(),
                    },
                );

            // NOTE: `provider_config` deliberately not included in the persisted input,
            // because it can carry secrets (API keys etc.).
            durability
                .run_async(|| Impl::search_once(provider_config, params))
                .await
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

    struct TakenSessionState<'a, Impl: ExtendedWebSearchProvider> {
        slot: &'a RefCell<Option<DurableSearchSessionState<Impl>>>,
        state: Option<DurableSearchSessionState<Impl>>,
    }

    impl<'a, Impl: ExtendedWebSearchProvider> TakenSessionState<'a, Impl> {
        fn take(slot: &'a RefCell<Option<DurableSearchSessionState<Impl>>>) -> Self {
            Self {
                slot,
                state: slot.borrow_mut().take(),
            }
        }

        fn get_mut(&mut self) -> &mut DurableSearchSessionState<Impl> {
            self.state.as_mut().expect("missing session state")
        }
    }

    impl<Impl: ExtendedWebSearchProvider> Drop for TakenSessionState<'_, Impl> {
        fn drop(&mut self) {
            let mut slot = self.slot.borrow_mut();
            if slot.is_none() {
                *slot = self.state.take();
            }
        }
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
            if let Some(DurableSearchSessionState::Live { session }) = self.state.take() {
                drop(session);
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

        fn next_page(&self) -> SearchPageFuture<'_> {
            Box::pin(async move {
                let durability =
                    Durability::<(Vec<SearchResult>, Impl::ReplayState), SearchError>::new(
                        "golem_websearch",
                        "next_page",
                        DurableFunctionType::ReadRemote,
                        &NoInput,
                    );

                let persisted = durability
                    .run_async(|| async {
                        let mut state = TakenSessionState::take(&self.state);
                        let current_state = state.get_mut();
                        match current_state {
                            DurableSearchSessionState::Live { session } => {
                                let value = session.next_page().await?;
                                let replay_state = Impl::session_to_state(session);
                                Ok((value, replay_state))
                            }
                            DurableSearchSessionState::Replay { replay_state } => {
                                let session = Impl::session_from_state(
                                    self.provider_config.clone(),
                                    replay_state,
                                    self.params.clone(),
                                )?;
                                let value = session.next_page().await?;
                                let new_replay_state = Impl::session_to_state(&session);
                                *current_state = DurableSearchSessionState::Live { session };
                                Ok((value, new_replay_state))
                            }
                        }
                    })
                    .await?;
                if let Some(DurableSearchSessionState::Replay { replay_state }) =
                    self.state.borrow_mut().as_mut()
                {
                    *replay_state = persisted.1.clone();
                }
                Ok(persisted.0)
            })
        }

        fn get_metadata(&self) -> Option<SearchMetadata> {
            let state = self.state.borrow();
            match &*state {
                Some(DurableSearchSessionState::Live { session }) => session.get_metadata(),
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

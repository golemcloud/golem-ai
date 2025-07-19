use std::marker::PhantomData;
use std::cell::RefCell;
use crate::exports::golem::web_search::web_search::{
    Guest, SearchError, SearchParams
};

/// Wraps a web search implementation with custom durability
pub struct DurableWebSearch<Impl> {
    phantom: PhantomData<Impl>,
}

/// Trait to be implemented in addition to the web search `Guest` trait when wrapping it with `DurableWebSearch`.
pub trait ExtendedGuest: Guest + 'static {
    /// Creates an instance of the web search specific `SearchSession` without wrapping it in a `Resource`
    fn unwrapped_search_session(params: SearchParams) -> Result<Self::SearchSession, SearchError>;

    /// Creates a search session from stored state (for recovery)
    fn session_from_state(params: SearchParams, page_count: u32) -> Result<Self::SearchSession, SearchError>;

    /// Creates retry parameters when resuming a search session after interruption
    fn retry_search_params(original_params: &SearchParams, _page_count: u32) -> SearchParams {
        // Default implementation just returns original params
        // Providers can override this to adjust parameters for retry
        original_params.clone()
    }
}

/// When the durability feature flag is off, wrapping with `DurableWebSearch` is just a passthrough
#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use crate::durability::{DurableWebSearch, ExtendedGuest};
    use crate::exports::golem::web_search::web_search::{
        Guest, SearchError, SearchMetadata, SearchParams, SearchResult, SearchSession,
    };

    impl<Impl: ExtendedGuest> Guest for DurableWebSearch<Impl> {
        type SearchSession = Impl::SearchSession;

        fn start_search(params: SearchParams) -> Result<SearchSession, SearchError> {
            println!("[DURABILITY] start_search: Passthrough mode - no durability");
            Impl::start_search(params)
        }

        fn search_once(
            params: SearchParams,
        ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
            println!("[DURABILITY] search_once: Passthrough mode - no durability");
            Impl::search_once(params)
        }
    }
}

/// When the durability feature flag is on, wrapping with `DurableWebSearch` adds custom durability
/// on top of the provider-specific web search implementation using Golem's special host functions and
/// the `golem-rust` helper library.
///
/// There will be custom durability entries saved in the oplog, with the full web search request and configuration
/// stored as input, and the full response stored as output. To serialize these in a way it is
/// observable by oplog consumers, each relevant data type has to be converted to/from `ValueAndType`
/// which is implemented using the type classes and builder in the `golem-rust` library.
#[cfg(feature = "durability")]
mod durable_impl {
    use crate::durability::{DurableWebSearch, ExtendedGuest};
    use crate::exports::golem::web_search::web_search::{
        Guest, SearchError, SearchMetadata, SearchParams, SearchResult, SearchSession,
    };
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    use golem_rust::durability::Durability;
    use golem_rust::{with_persistence_level, FromValueAndType, IntoValue, PersistenceLevel};
    use std::fmt::{Display, Formatter};
    use std::marker::PhantomData;
    use std::cell::RefCell;

    /// Durable search session state for replay and recovery
    #[derive(Debug)]
    enum DurableSearchSessionState<Impl: ExtendedGuest> {
        Live {
            session: Impl::SearchSession,
        },
        Replay {
            original_params: SearchParams,
            page_count: u32,
            finished: bool,
        },
    }

    /// Durable wrapper for search sessions that handles recovery and replay
    pub struct DurableSearchSession<Impl: ExtendedGuest> {
        state: RefCell<Option<DurableSearchSessionState<Impl>>>,
        phantom: PhantomData<Impl>,
    }

    impl<Impl: ExtendedGuest> DurableSearchSession<Impl> {
        fn new(session: Impl::SearchSession, _params: SearchParams) -> Self {
            Self {
                state: RefCell::new(Some(DurableSearchSessionState::Live { session })),
                phantom: PhantomData,
            }
        }
    }

    impl<Impl: ExtendedGuest> crate::exports::golem::web_search::web_search::GuestSearchSession for DurableSearchSession<Impl> {
        fn next_page(&self) -> Result<SearchResult, SearchError> {
            let durability = Durability::<Option<SearchResult>, UnusedError>::new(
                "golem_web_search",
                "next_page",
                DurableFunctionType::ReadRemote,
            );
            
            if durability.is_live() {
                let mut state = self.state.borrow_mut();
                let result = match &*state {
                    Some(DurableSearchSessionState::Live { session }) => {
                        println!("[DURABILITY] next_page: LIVE mode - executing search");
                        let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                            session.next_page()
                        });
                        durability.persist_infallible(NoInput, result.clone().ok())
                    }
                    Some(DurableSearchSessionState::Replay { original_params, page_count, finished: _ }) => {
                        println!("[DURABILITY] next_page: LIVE mode - creating new session for replay");
                        let retry_params = Impl::retry_search_params(original_params, *page_count);
                        let new_session = with_persistence_level(PersistenceLevel::PersistNothing, || {
                            Impl::unwrapped_search_session(retry_params)
                        })?;
                        let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                            new_session.next_page()
                        });
                        *state = Some(DurableSearchSessionState::Live { session: new_session });
                        durability.persist_infallible(NoInput, result.clone().ok())
                    }
                    None => {
                        return Err(SearchError::BackendError("Invalid session state".to_string()));
                    }
                };
                result.ok_or_else(|| SearchError::BackendError("No result from live session".to_string()))
            } else {
                println!("[DURABILITY] next_page: REPLAY mode - retrieving persisted result");
                let result: Option<SearchResult> = durability.replay_infallible();
                match result {
                    Some(search_result) => {
                        println!("[DURABILITY] next_page: REPLAY mode - replayed result: {}", search_result.title);
                        Ok(search_result)
                    }
                    None => {
                        Err(SearchError::BackendError("Session finished during replay".to_string()))
                    }
                }
            }
        }

        fn get_metadata(&self) -> Option<SearchMetadata> {
            let state = self.state.borrow();
            match &*state {
                Some(DurableSearchSessionState::Live { session }) => session.get_metadata(),
                _ => None,
            }
        }
    }

    impl<Impl: ExtendedGuest> Guest for DurableWebSearch<Impl> {
        type SearchSession = DurableSearchSession<Impl>;

        fn start_search(params: SearchParams) -> Result<SearchSession, SearchError> {
            println!("[DURABILITY] start_search: Creating durable search session with state management");
            let inner_session = Impl::unwrapped_search_session(params.clone())?;
            Ok(SearchSession::new(DurableSearchSession::<Impl>::new(inner_session, params)))
        }

        fn search_once(
            params: SearchParams,
        ) -> Result<(Vec<SearchResult>, Option<SearchMetadata>), SearchError> {
            let durability =
                Durability::<(Vec<SearchResult>, Option<SearchMetadata>), UnusedError>::new(
                    "golem_web_search",
                    "search_once",
                    DurableFunctionType::WriteRemote,
                );
            if durability.is_live() {
                println!("[DURABILITY] search_once: LIVE mode - executing search");
                let result = Impl::search_once(params.clone());
                match result {
                    Ok(success) => {
                        println!("[DURABILITY] search_once: LIVE mode - persisting {} results", success.0.len());
                        Ok(durability.persist_infallible(params, success))
                    }
                    Err(err) => {
                        println!("[DURABILITY] search_once: LIVE mode - error occurred");
                        Err(err)
                    }
                }
            } else {
                println!("[DURABILITY] search_once: REPLAY mode - retrieving persisted results");
                let result: (Vec<SearchResult>, Option<SearchMetadata>) =
                    durability.replay_infallible();
                println!("[DURABILITY] search_once: REPLAY mode - replayed {} results", result.0.len());
                Ok(result)
            }
        }
    }

    #[derive(Debug, Clone, IntoValue, FromValueAndType)]
    struct NoInput;

    #[derive(Debug, IntoValue, FromValueAndType)]
    struct UnusedError;

    impl Display for UnusedError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "UnusedError")
        }
    }
}
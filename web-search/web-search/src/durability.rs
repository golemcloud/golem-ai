use crate::exports::golem::web_search::web_search::{Guest, SearchError, SearchParams};
use std::marker::PhantomData;

/// Wraps a web search implementation with custom durability
pub struct DurableWebSearch<Impl> {
    phantom: PhantomData<Impl>,
}

/// Trait to be implemented in addition to the web search `Guest` trait when wrapping it with `DurableWebSearch`.
pub trait ExtendedGuest: Guest + 'static {
    /// Creates an instance of the web search specific `SearchSession` without wrapping it in a `Resource`
    fn unwrapped_search_session(params: SearchParams) -> Result<Self::SearchSession, SearchError>;

    /// Creates a search session from stored state (for recovery)
    fn session_for_page(
        params: SearchParams,
        page_count: u32,
    ) -> Result<Self::SearchSession, SearchError>;

    /// Check if the session has finished (no more results)
    fn is_session_finished(session: &Self::SearchSession) -> bool;

    /// Set the session as finished (used during replay state reconstruction)
    fn mark_session_finished(session: &Self::SearchSession);
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
    use std::cell::RefCell;
    use std::fmt::{Display, Formatter};
    use std::marker::PhantomData;

    /// Durable search session state for replay and recovery
    #[derive(Debug)]
    #[allow(dead_code)]
    enum DurableSearchSessionState<Impl: ExtendedGuest> {
        Live {
            session: Impl::SearchSession,
            page_count: u32, // Track current page in live mode too
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
                state: RefCell::new(Some(DurableSearchSessionState::Live { 
                    session, 
                    page_count: 0 
                })),
                phantom: PhantomData,
            }
        }

        fn replay(original_params: SearchParams, page_count: u32) -> Self {
            Self {
                state: RefCell::new(Some(DurableSearchSessionState::Replay {
                    original_params,
                    page_count,
                    finished: false,
                })),
                phantom: PhantomData,
            }
        }
    }

    impl<Impl: ExtendedGuest> crate::exports::golem::web_search::web_search::GuestSearchSession
        for DurableSearchSession<Impl>
    {
        fn next_page(&self) -> Result<SearchResult, SearchError> {
            let durability = Durability::<Result<SearchResult, SearchError>, UnusedError>::new(
                "golem_web_search",
                "next_page",
                DurableFunctionType::ReadRemote,
            );

            if durability.is_live() {
                let mut state = self.state.borrow_mut();
                let result = match &mut *state {
                    Some(DurableSearchSessionState::Live { session, page_count }) => {
                        println!("[DURABILITY] next_page: LIVE mode - executing search, current_page: {}", *page_count);
                        let result =
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                session.next_page()
                            });
                        let persisted_result = durability.persist_infallible(NoInput, result.clone());
                        
                        // Increment page count if successful, mark as finished if error
                        match &persisted_result {
                            Ok(_) => {
                                *page_count += 1;
                                println!("[DURABILITY] next_page: LIVE mode - success, incremented to page: {}", *page_count);
                            }
                            Err(_) => {
                                // Mark session as finished when we get an error (no more results)
                                Impl::mark_session_finished(session);
                                println!("[DURABILITY] next_page: LIVE mode - marked session as finished due to error");
                            }
                        }
                        
                        persisted_result
                    }
                    Some(DurableSearchSessionState::Replay {
                        original_params,
                        page_count,
                        finished,
                    }) => {
                        if *finished {
                            Err(SearchError::BackendError("Session finished".to_string()))
                        } else {
                            println!("[DURABILITY] next_page: REPLAY→LIVE transition - creating live session from state, page_count: {page_count}");

                            let (session, first_live_result) =
                                with_persistence_level(PersistenceLevel::PersistNothing, || {
                                    let session =
                                        Impl::session_for_page(original_params.clone(), *page_count)?;
                                    let result = session.next_page();
                                    Ok((session, result))
                                })?;

                            let persisted_result = durability
                                .persist_infallible(NoInput, first_live_result.clone());
                            
                            // Switch to live mode with current page count + 1 if successful
                            let next_page_count = if persisted_result.is_ok() { 
                                *page_count + 1 
                            } else { 
                                *page_count 
                            };
                            
                            // Check if session is finished and update finished state if needed
                            if persisted_result.is_err() {
                                // If the live request failed, mark the session as finished
                                Impl::mark_session_finished(&session);
                            }
                            
                            *state = Some(DurableSearchSessionState::Live { 
                                session, 
                                page_count: next_page_count 
                            });
                            
                            println!("[DURABILITY] next_page: REPLAY→LIVE transition complete, now on page: {}", next_page_count);
                            persisted_result
                        }
                    }
                    None => {
                        Err(SearchError::BackendError(
                            "Invalid session state".to_string(),
                        ))
                    }
                };

                result
            } else {
                println!("[DURABILITY] next_page: REPLAY mode - retrieving persisted result");
                let result: Result<SearchResult, SearchError> = durability.replay_infallible();
                
                // Update state during replay to maintain consistency
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurableSearchSessionState::Replay { 
                        page_count, 
                        finished, 
                        .. 
                    }) => {
                        match &result {
                            Ok(_) => {
                                // Increment page count on successful replay
                                *page_count += 1;
                                println!("[DURABILITY] next_page: REPLAY mode - incremented page_count to: {}", *page_count);
                            }
                            Err(_) => {
                                // Mark as finished on error
                                *finished = true;
                                println!("[DURABILITY] next_page: REPLAY mode - marked as finished due to error");
                            }
                        }
                    }
                    _ => {
                        // This shouldn't happen during replay mode
                        println!("[DURABILITY] next_page: REPLAY mode - unexpected state");
                    }
                }
                
                match result {
                    Ok(search_result) => {
                        println!(
                            "[DURABILITY] next_page: REPLAY mode - replayed result: {}",
                            search_result.title
                        );
                        Ok(search_result)
                    }
                    Err(error) => {
                        println!(
                            "[DURABILITY] next_page: REPLAY mode - replayed error: {:?}",
                            error
                        );
                        Err(error)
                    }
                }
            }
        }

        fn get_metadata(&self) -> Option<SearchMetadata> {
            let durability = Durability::<Option<SearchMetadata>, UnusedError>::new(
                "golem_web_search",
                "get_metadata", 
                DurableFunctionType::ReadRemote,
            );

            if durability.is_live() {
                println!("[DURABILITY] get_metadata: LIVE mode - retrieving metadata");
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableSearchSessionState::Live { session, .. }) => session.get_metadata(),
                    _ => None,
                };
                println!("[DURABILITY] get_metadata: LIVE mode - persisting metadata result");
                durability.persist_infallible(NoInput, result)
            } else {
                println!("[DURABILITY] get_metadata: REPLAY mode - retrieving persisted metadata");
                let result: Option<SearchMetadata> = durability.replay_infallible();
                println!("[DURABILITY] get_metadata: REPLAY mode - replayed metadata");
                result
            }
        }
    }

    impl<Impl: ExtendedGuest> Guest for DurableWebSearch<Impl> {
        type SearchSession = DurableSearchSession<Impl>;

        fn start_search(params: SearchParams) -> Result<SearchSession, SearchError> {
            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_web_search",
                "start_search",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                println!("[DURABILITY] start_search: LIVE mode - creating new search session");
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    let inner_session = Impl::unwrapped_search_session(params.clone())?;
                    Ok(SearchSession::new(DurableSearchSession::<Impl>::new(
                        inner_session,
                        params,
                    )))
                });
                let _ = durability.persist_infallible(NoInput, NoOutput);
                result
            } else {
                println!("[DURABILITY] start_search: REPLAY mode - creating replay session");
                let _: NoOutput = durability.replay_infallible();
                // In replay mode, start with page count 0 - individual next_page calls will be replayed
                Ok(SearchSession::new(DurableSearchSession::<Impl>::replay(
                    params, 0,
                )))
            }
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
                        println!(
                            "[DURABILITY] search_once: LIVE mode - persisting {} results",
                            success.0.len()
                        );
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
                println!(
                    "[DURABILITY] search_once: REPLAY mode - replayed {} results",
                    result.0.len()
                );
                Ok(result)
            }
        }
    }

    #[derive(Debug, Clone, IntoValue, FromValueAndType)]
    struct NoInput;

    #[derive(Debug, Clone, IntoValue, FromValueAndType)]
    struct NoOutput;

    #[derive(Debug, IntoValue, FromValueAndType)]
    struct UnusedError;

    impl Display for UnusedError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "UnusedError")
        }
    }
}

use crate::model::{Config, ContentPart, Error, Event, Message, Role, StreamDelta};
use crate::wasi_compat::Pollable;
use crate::LlmProvider;
use indoc::indoc;
use std::marker::PhantomData;

/// Wraps an LLM implementation with custom durability
pub struct DurableLLM<Impl> {
    phantom: PhantomData<Impl>,
}

/// Trait implemented by provider crates in addition to `LlmProvider`, providing the hooks that
/// `DurableLLM` needs for durable replay (constructing a raw `ChatStream`, subscribing a
/// pollable, and producing a retry prompt from the partial streamed response).
#[allow(async_fn_in_trait)]
pub trait ExtendedLlmProvider: LlmProvider + 'static {
    /// Creates an instance of the LLM specific `ChatStream` without wrapping it in a `Resource`
    async fn unwrapped_stream(
        provider_config: Self::ProviderConfig,
        events: Vec<Event>,
        config: Config,
    ) -> Self::ChatStream;

    /// Creates the retry prompt with a combination of the original events, and the partially received
    /// streaming responses. There is a default implementation here, but it can be overridden with provider-specific
    /// prompts if needed.
    fn retry_prompt(
        original_events: &[Result<Event, Error>],
        partial_result: &[StreamDelta],
    ) -> Vec<Event> {
        let mut extended_events = Vec::new();
        extended_events.push(
            Event::Message(Message {
            role: Role::System,
            name: None,
            content: vec![
                ContentPart::Text(indoc!{"
                  You were asked the same question previously, but the response was interrupted before completion.
                  Please continue your response from where you left off.
                  Do not include the part of the response that was already seen."
                }.to_string()),
                ContentPart::Text("Here is the original question:".to_string()),
            ],
        }));
        extended_events.extend(
            original_events
                .iter()
                .filter_map(|event| event.as_ref().ok().cloned()),
        );

        let mut partial_result_as_content = Vec::new();
        for delta in partial_result {
            if let Some(contents) = &delta.content {
                partial_result_as_content.extend_from_slice(contents);
            }
            if let Some(tool_calls) = &delta.tool_calls {
                for tool_call in tool_calls {
                    partial_result_as_content.push(ContentPart::Text(format!(
                        "<tool-call id=\"{}\" name=\"{}\" arguments=\"{}\"/>",
                        tool_call.id, tool_call.name, tool_call.arguments_json,
                    )));
                }
            }
        }

        extended_events.push(Event::Message(Message {
            role: Role::System,
            name: None,
            content: vec![ContentPart::Text(
                "Here is the partial response that was successfully received:".to_string(),
            )]
            .into_iter()
            .chain(partial_result_as_content)
            .collect(),
        }));
        extended_events
    }

    fn subscribe(stream: &Self::ChatStream) -> Pollable;
}

/// When the durability feature flag is off, `DurableLLM<Impl>` is a transparent wrapper that
/// forwards every call to the inner provider without any oplog persistence.
#[cfg(not(feature = "golem"))]
mod passthrough_impl {
    use crate::durability::{DurableLLM, ExtendedLlmProvider};
    use crate::init_logging;
    use crate::model::{ChatStream, Config, Error, Event, Response};
    use crate::LlmProvider;

    impl<Impl: ExtendedLlmProvider> LlmProvider for DurableLLM<Impl> {
        type ChatStream = Impl::ChatStream;
        type ProviderConfig = Impl::ProviderConfig;

        async fn send(
            provider_config: Self::ProviderConfig,
            events: Vec<Event>,
            config: Config,
        ) -> Result<Response, Error> {
            init_logging();
            Impl::send(provider_config, events, config).await
        }

        async fn stream(
            provider_config: Self::ProviderConfig,
            events: Vec<Event>,
            config: Config,
        ) -> ChatStream {
            init_logging();
            Impl::stream(provider_config, events, config).await
        }
    }
}

/// When the durability feature flag is on, wrapping with `DurableLLM` adds custom durability
/// on top of the provider-specific LLM implementation using Golem's special host functions and
/// the `golem-rust` helper library.
///
/// There will be custom durability entries saved in the oplog, with the full LLM request and configuration
/// stored as input, and the full response stored as output. To serialize these in a way it is
/// observable by oplog consumers, each relevant data type has to be converted to/from `ValueAndType`
/// which is implemented using the type classes and builder in the `golem-rust` library.
#[cfg(feature = "golem")]
mod durable_impl {
    use crate::durability::{DurableLLM, ExtendedLlmProvider};
    use crate::model::{ChatStream, Config, Error, Event, Response, StreamDelta, StreamEvent};
    use crate::wasi_compat::Pollable;
    use crate::{init_logging, ChatStreamInterface, LlmProvider};
    use async_trait::async_trait;
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    #[cfg(not(feature = "nopoll"))]
    use golem_rust::bindings::golem::durability::durability::LazyInitializedPollable;
    use golem_rust::durability::Durability;
    use golem_rust::{
        with_persistence_level, with_persistence_level_async, FromValueAndType, IntoValue,
        PersistenceLevel,
    };
    use std::cell::RefCell;
    use std::fmt::{Display, Formatter};
    use std::rc::Rc;

    impl<Impl: ExtendedLlmProvider> LlmProvider for DurableLLM<Impl> {
        type ChatStream = DurableChatStream<Impl>;
        type ProviderConfig = Impl::ProviderConfig;

        async fn send(
            provider_config: Self::ProviderConfig,
            events: Vec<Event>,
            config: Config,
        ) -> Result<Response, Error> {
            init_logging();

            let durability = Durability::<Response, Error>::new(
                "golem_ai_llm",
                "send",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let events_clone = events.clone();
                let config_clone = config.clone();
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async move {
                        Impl::send(provider_config, events_clone, config_clone).await
                    })
                    .await;
                // NOTE: `provider_config` deliberately not included in the persisted input,
                // because it can carry secrets (API keys etc.).
                durability.persist_serializable(SendInput { events, config }, result.clone());
                result
            } else {
                durability.replay_serializable()
            }
        }

        async fn stream(
            provider_config: Self::ProviderConfig,
            events: Vec<Event>,
            config: Config,
        ) -> ChatStream {
            init_logging();

            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_ai_llm",
                "stream",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let events_clone = events.clone();
                let config_clone = config.clone();
                let provider_config_clone = provider_config.clone();
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async move {
                        ChatStream::new(DurableChatStream::<Impl>::live(
                            provider_config_clone.clone(),
                            <Impl as ExtendedLlmProvider>::unwrapped_stream(
                                provider_config_clone,
                                events_clone,
                                config_clone,
                            )
                            .await,
                        ))
                    })
                    .await;
                // NOTE: `provider_config` deliberately not included in the persisted input.
                let _ = durability.persist_infallible(SendInput { events, config }, NoOutput);
                result
            } else {
                let _: NoOutput = durability.replay_infallible();
                ChatStream::new(DurableChatStream::<Impl>::replay(
                    provider_config,
                    events.into_iter().map(Ok).collect(),
                    config,
                ))
            }
        }
    }

    /// Represents the durable chat stream's state
    ///
    /// In live mode it directly calls the underlying LLM stream which is implemented on
    /// top of an SSE parser using the wasi-http response body stream.
    /// When the `nopoll` feature flag is enabled, all polling related features are disabled
    /// and events rely solely on the mechanism defined in the Implementation. Useful for implementations
    /// that do not expose a wasi-http response body stream e.g AWS Bedrock.
    ///
    /// In replay mode it buffers the replayed messages, and also tracks the created pollables
    /// to be able to reattach them to the new live stream when the switch to live mode
    /// happens.
    ///
    /// When reaching the end of the replay mode, if the replayed stream was not finished yet,
    /// the replay prompt implemented in `ExtendedGuest` is used to create a new LLM response
    /// stream and continue the response seamlessly.
    enum DurableChatStreamState<Impl: ExtendedLlmProvider> {
        Live {
            stream: Rc<Impl::ChatStream>,
            #[cfg(not(feature = "nopoll"))]
            pollables: Vec<LazyInitializedPollable>,
        },
        Replay {
            original_events: Vec<Result<Event, Error>>,
            config: Config,
            #[cfg(not(feature = "nopoll"))]
            pollables: Vec<LazyInitializedPollable>,
            partial_result: Vec<StreamDelta>,
            finished: bool,
            continuation_started: bool,
        },
    }

    pub struct DurableChatStream<Impl: ExtendedLlmProvider> {
        provider_config: Impl::ProviderConfig,
        state: RefCell<Option<DurableChatStreamState<Impl>>>,
        subscription: RefCell<Option<Pollable>>,
        starting_replay_continuation: RefCell<bool>,
    }

    struct ReplayContinuationGuard<'a> {
        in_progress: &'a RefCell<bool>,
    }

    impl Drop for ReplayContinuationGuard<'_> {
        fn drop(&mut self) {
            *self.in_progress.borrow_mut() = false;
        }
    }

    impl<Impl: ExtendedLlmProvider> DurableChatStream<Impl> {
        fn live(provider_config: Impl::ProviderConfig, stream: Impl::ChatStream) -> Self {
            Self {
                provider_config,
                state: RefCell::new(Some(DurableChatStreamState::Live {
                    stream: Rc::new(stream),
                    #[cfg(not(feature = "nopoll"))]
                    pollables: Vec::new(),
                })),
                subscription: RefCell::new(None),
                starting_replay_continuation: RefCell::new(false),
            }
        }

        fn replay(
            provider_config: Impl::ProviderConfig,
            original_events: Vec<Result<Event, Error>>,
            config: Config,
        ) -> Self {
            Self {
                provider_config,
                state: RefCell::new(Some(DurableChatStreamState::Replay {
                    original_events,
                    config,
                    #[cfg(not(feature = "nopoll"))]
                    pollables: Vec::new(),
                    partial_result: Vec::new(),
                    finished: false,
                    continuation_started: false,
                })),
                subscription: RefCell::new(None),
                starting_replay_continuation: RefCell::new(false),
            }
        }

        fn begin_replay_continuation(&self) -> Option<ReplayContinuationGuard<'_>> {
            let mut in_progress = self.starting_replay_continuation.borrow_mut();
            if *in_progress {
                None
            } else {
                *in_progress = true;
                Some(ReplayContinuationGuard {
                    in_progress: &self.starting_replay_continuation,
                })
            }
        }

        #[cfg(not(feature = "nopoll"))]
        fn subscribe(&self) -> Pollable {
            let mut state = self.state.borrow_mut();
            match &mut *state {
                Some(DurableChatStreamState::Live { stream, .. }) => {
                    Impl::subscribe(stream.as_ref())
                }
                Some(DurableChatStreamState::Replay { pollables, .. }) => {
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

    impl<Impl: ExtendedLlmProvider> Drop for DurableChatStream<Impl> {
        fn drop(&mut self) {
            let _ = self.subscription.take();

            match self.state.take() {
                Some(DurableChatStreamState::Live {
                    #[cfg(not(feature = "nopoll"))]
                    mut pollables,
                    stream,
                }) => {
                    with_persistence_level(PersistenceLevel::PersistNothing, move || {
                        #[cfg(not(feature = "nopoll"))]
                        pollables.clear();
                        drop(stream);
                    });
                }
                Some(DurableChatStreamState::Replay {
                    #[cfg(not(feature = "nopoll"))]
                    mut pollables,
                    ..
                }) => {
                    #[cfg(not(feature = "nopoll"))]
                    pollables.clear();
                }
                None => {}
            }
        }
    }

    async fn poll_live_stream<Impl: ExtendedLlmProvider>(
        stream: Rc<Impl::ChatStream>,
    ) -> Option<Vec<Result<StreamEvent, Error>>> {
        with_persistence_level_async(PersistenceLevel::PersistNothing, || async move {
            stream.poll_next().await
        })
        .await
    }

    fn public_poll_result_from_persisted(
        result: PersistedPollResult,
    ) -> Option<Vec<Result<StreamEvent, Error>>> {
        match result {
            PersistedPollResult::Pending => None,
            PersistedPollResult::Events(events) => Some(events),
            PersistedPollResult::Terminal => Some(vec![]),
            PersistedPollResult::StartedReplayContinuation(result) => result,
        }
    }

    fn persisted_poll_result_from_public(
        result: Option<Vec<Result<StreamEvent, Error>>>,
    ) -> PersistedPollResult {
        match result {
            None => PersistedPollResult::Pending,
            Some(events) if events.is_empty() => PersistedPollResult::Terminal,
            Some(events) => PersistedPollResult::Events(events),
        }
    }

    fn update_replay_progress(
        result: Option<&[Result<StreamEvent, Error>]>,
        partial_result: &mut Vec<StreamDelta>,
        finished: &mut bool,
    ) {
        if let Some(result) = result {
            if result.is_empty() {
                *finished = true;
                return;
            }

            for event in result {
                match event {
                    Ok(StreamEvent::Delta(delta)) => {
                        partial_result.push(delta.clone());
                    }
                    Ok(StreamEvent::Finish(_)) | Err(_) => {
                        *finished = true;
                    }
                }
            }
        }
    }

    #[async_trait(?Send)]
    impl<Impl: ExtendedLlmProvider> ChatStreamInterface for DurableChatStream<Impl> {
        async fn poll_next(&self) -> Option<Vec<Result<StreamEvent, Error>>> {
            let durability = Durability::<PersistedPollResult, UnusedError>::new(
                "golem_ai_llm",
                "poll_next",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                enum PollAction<Impl: ExtendedLlmProvider> {
                    PollLive(Rc<Impl::ChatStream>),
                    StartReplayContinuation {
                        config: Config,
                        extended_events: Vec<Event>,
                        continuation_already_started: bool,
                    },
                    FinishedReplay,
                }

                let action = {
                    let state = self.state.borrow();
                    match &*state {
                        Some(DurableChatStreamState::Live { stream, .. }) => {
                            PollAction::<Impl>::PollLive(Rc::clone(stream))
                        }
                        Some(DurableChatStreamState::Replay {
                            config,
                            original_events,
                            partial_result,
                            finished,
                            continuation_started,
                            ..
                        }) => {
                            if *finished {
                                PollAction::<Impl>::FinishedReplay
                            } else {
                                PollAction::<Impl>::StartReplayContinuation {
                                    config: config.clone(),
                                    extended_events: Impl::retry_prompt(
                                        original_events,
                                        partial_result,
                                    ),
                                    continuation_already_started: *continuation_started,
                                }
                            }
                        }
                        None => unreachable!(),
                    }
                };

                match action {
                    PollAction::PollLive(stream) => {
                        let result = poll_live_stream::<Impl>(stream).await;
                        let persisted_result = persisted_poll_result_from_public(result);
                        durability.persist_infallible(NoInput, persisted_result.clone());
                        public_poll_result_from_persisted(persisted_result)
                    }
                    PollAction::FinishedReplay => {
                        let persisted_result = PersistedPollResult::Terminal;
                        durability.persist_infallible(NoInput, persisted_result.clone());
                        public_poll_result_from_persisted(persisted_result)
                    }
                    PollAction::StartReplayContinuation {
                        config,
                        extended_events,
                        continuation_already_started,
                    } => {
                        let _guard = self.begin_replay_continuation()?;

                        let provider_config = self.provider_config.clone();
                        let stream = with_persistence_level_async(
                            PersistenceLevel::PersistNothing,
                            || async move {
                                <Impl as ExtendedLlmProvider>::unwrapped_stream(
                                    provider_config,
                                    extended_events,
                                    config,
                                )
                                .await
                            },
                        )
                        .await;
                        let stream = Rc::new(stream);

                        let stream_to_poll = {
                            let mut state = self.state.borrow_mut();
                            match &mut *state {
                                Some(DurableChatStreamState::Replay {
                                    #[cfg(not(feature = "nopoll"))]
                                    pollables,
                                    ..
                                }) => {
                                    #[cfg(not(feature = "nopoll"))]
                                    for lazy_initialized_pollable in pollables.iter() {
                                        lazy_initialized_pollable
                                            .set(Impl::subscribe(stream.as_ref()));
                                    }

                                    #[cfg(not(feature = "nopoll"))]
                                    let pollables = std::mem::take(pollables);

                                    *state = Some(DurableChatStreamState::Live {
                                        stream: Rc::clone(&stream),
                                        #[cfg(not(feature = "nopoll"))]
                                        pollables,
                                    });
                                    Rc::clone(&stream)
                                }
                                Some(DurableChatStreamState::Live { stream, .. }) => {
                                    // Another caller completed the transition while this
                                    // async call was suspended. Keep that state intact and
                                    // poll the stream already stored there.
                                    Rc::clone(stream)
                                }
                                None => unreachable!(),
                            }
                        };

                        let result = poll_live_stream::<Impl>(stream_to_poll).await;
                        let persisted_result = if continuation_already_started {
                            persisted_poll_result_from_public(result)
                        } else {
                            PersistedPollResult::StartedReplayContinuation(result)
                        };
                        durability.persist_infallible(NoInput, persisted_result.clone());
                        public_poll_result_from_persisted(persisted_result)
                    }
                }
            } else {
                let persisted_result: PersistedPollResult = durability.replay_infallible();
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurableChatStreamState::Live { .. }) => {
                        unreachable!("Durable chat stream cannot be in live mode during replay")
                    }
                    Some(DurableChatStreamState::Replay {
                        partial_result,
                        finished,
                        continuation_started,
                        ..
                    }) => match &persisted_result {
                        PersistedPollResult::Pending => {}
                        PersistedPollResult::Terminal => {
                            *finished = true;
                        }
                        PersistedPollResult::Events(result) => {
                            update_replay_progress(Some(result), partial_result, finished);
                        }
                        PersistedPollResult::StartedReplayContinuation(result) => {
                            *continuation_started = true;
                            update_replay_progress(result.as_deref(), partial_result, finished);
                        }
                    },
                    None => {
                        unreachable!()
                    }
                }
                public_poll_result_from_persisted(persisted_result)
            }
        }

        async fn get_next(&self) -> Vec<Result<StreamEvent, Error>> {
            loop {
                // Acquire and release the subscription borrow within the loop body so we
                // never hold a RefCell borrow across the .await on `poll_next`.
                #[cfg(not(feature = "nopoll"))]
                {
                    let mut subscription = self.subscription.borrow_mut();
                    if subscription.is_none() {
                        *subscription = Some(self.subscribe());
                    }
                    subscription
                        .as_ref()
                        .expect("subscription just initialized")
                        .block();
                }
                if let Some(events) = self.poll_next().await {
                    return events;
                }
            }
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    #[derive(Debug, Clone, PartialEq, IntoValue)]
    struct SendInput {
        events: Vec<Event>,
        config: Config,
    }

    #[derive(Debug, IntoValue)]
    struct NoInput;

    #[derive(Debug, Clone, FromValueAndType, IntoValue)]
    struct NoOutput;

    #[derive(Debug, Clone, FromValueAndType, IntoValue)]
    enum PersistedPollResult {
        Pending,
        Events(Vec<Result<StreamEvent, Error>>),
        Terminal,
        StartedReplayContinuation(Option<Vec<Result<StreamEvent, Error>>>),
    }

    #[derive(Debug, FromValueAndType, IntoValue)]
    struct UnusedError;

    impl Display for UnusedError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "UnusedError")
        }
    }
}

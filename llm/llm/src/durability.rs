use crate::model::{Config, ContentPart, Error, Event, Message, Role, StreamDelta};
use crate::LlmProvider;
use indoc::indoc;
use std::marker::PhantomData;

/// Wraps an LLM implementation with custom durability
pub struct DurableLLM<Impl> {
    phantom: PhantomData<Impl>,
}

/// Trait implemented by provider crates in addition to `LlmProvider`, providing the hooks that
/// `DurableLLM` needs for durable replay (constructing a raw `ChatStream` and producing a retry
/// prompt from the partial streamed response).
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
    use crate::{init_logging, ChatStreamInterface, LlmFuture, LlmProvider};
    use futures::lock::Mutex;
    use golem_rust::durability::{Durability, DurableFunctionType};
    use golem_rust::{
        with_persistence_level, with_persistence_level_async, FromSchema, IntoSchema,
        PersistenceLevel,
    };
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
    /// In live mode it directly awaits the underlying provider stream. Replay results resolve
    /// immediately; one continuation stream is created when replay reaches live mode.
    ///
    /// When reaching the end of the replay mode, if the replayed stream was not finished yet,
    /// the replay prompt implemented in `ExtendedGuest` is used to create a new LLM response
    /// stream and continue the response seamlessly.
    enum DurableChatStreamState<Impl: ExtendedLlmProvider> {
        Live {
            stream: Rc<Impl::ChatStream>,
        },
        Replay {
            original_events: Vec<Result<Event, Error>>,
            config: Config,
            partial_result: Vec<StreamDelta>,
            finished: bool,
            continuation_started: bool,
        },
    }

    pub struct DurableChatStream<Impl: ExtendedLlmProvider> {
        provider_config: Impl::ProviderConfig,
        state: Mutex<Option<DurableChatStreamState<Impl>>>,
    }

    impl<Impl: ExtendedLlmProvider> DurableChatStream<Impl> {
        fn live(provider_config: Impl::ProviderConfig, stream: Impl::ChatStream) -> Self {
            Self {
                provider_config,
                state: Mutex::new(Some(DurableChatStreamState::Live {
                    stream: Rc::new(stream),
                })),
            }
        }

        fn replay(
            provider_config: Impl::ProviderConfig,
            original_events: Vec<Result<Event, Error>>,
            config: Config,
        ) -> Self {
            Self {
                provider_config,
                state: Mutex::new(Some(DurableChatStreamState::Replay {
                    original_events,
                    config,
                    partial_result: Vec::new(),
                    finished: false,
                    continuation_started: false,
                })),
            }
        }
    }

    impl<Impl: ExtendedLlmProvider> Drop for DurableChatStream<Impl> {
        fn drop(&mut self) {
            match self.state.get_mut().take() {
                Some(DurableChatStreamState::Live { stream }) => {
                    with_persistence_level(PersistenceLevel::PersistNothing, move || {
                        drop(stream);
                    });
                }
                Some(DurableChatStreamState::Replay { .. }) => {}
                None => {}
            }
        }
    }

    async fn read_live_stream<Impl: ExtendedLlmProvider>(
        stream: Rc<Impl::ChatStream>,
    ) -> Vec<Result<StreamEvent, Error>> {
        with_persistence_level_async(PersistenceLevel::PersistNothing, || async move {
            stream.get_next().await
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
        result: Vec<Result<StreamEvent, Error>>,
    ) -> PersistedPollResult {
        match result {
            events if events.is_empty() => PersistedPollResult::Terminal,
            events => PersistedPollResult::Events(events),
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

    impl<Impl: ExtendedLlmProvider> ChatStreamInterface for DurableChatStream<Impl> {
        fn get_next(&self) -> LlmFuture<'_, Vec<Result<StreamEvent, Error>>> {
            Box::pin(async move {
                let mut state = self.state.lock().await;
                loop {
                    let durability = Durability::<PersistedPollResult, UnusedError>::new(
                        "golem_ai_llm",
                        "poll_next",
                        DurableFunctionType::ReadRemote,
                    );
                    if !durability.is_live() {
                        let persisted: PersistedPollResult = durability.replay_infallible();
                        let visible = public_poll_result_from_persisted(persisted.clone());
                        match state.as_mut().expect("stream state") {
                            DurableChatStreamState::Live { .. } => {
                                unreachable!("live during replay")
                            }
                            DurableChatStreamState::Replay {
                                partial_result,
                                finished,
                                continuation_started,
                                ..
                            } => {
                                if matches!(
                                    persisted,
                                    PersistedPollResult::StartedReplayContinuation(_)
                                ) {
                                    *continuation_started = true;
                                }
                                update_replay_progress(
                                    visible.as_deref(),
                                    partial_result,
                                    finished,
                                );
                            }
                        }
                        if let Some(events) = visible {
                            return events;
                        }
                        continue;
                    }

                    let (result, marker) = match state.as_ref().expect("stream state") {
                        DurableChatStreamState::Live { stream } => {
                            (read_live_stream::<Impl>(Rc::clone(stream)).await, false)
                        }
                        DurableChatStreamState::Replay { finished: true, .. } => (vec![], false),
                        DurableChatStreamState::Replay {
                            original_events,
                            config,
                            partial_result,
                            continuation_started,
                            ..
                        } => {
                            let marker = !*continuation_started;
                            let events = Impl::retry_prompt(original_events, partial_result);
                            let config = config.clone();
                            let provider_config = self.provider_config.clone();
                            let stream = with_persistence_level_async(
                                PersistenceLevel::PersistNothing,
                                || async move {
                                    Rc::new(
                                        Impl::unwrapped_stream(provider_config, events, config)
                                            .await,
                                    )
                                },
                            )
                            .await;
                            let result = read_live_stream::<Impl>(Rc::clone(&stream)).await;
                            *state = Some(DurableChatStreamState::Live { stream });
                            (result, marker)
                        }
                    };
                    let persisted = if marker {
                        PersistedPollResult::StartedReplayContinuation(Some(result.clone()))
                    } else {
                        persisted_poll_result_from_public(result.clone())
                    };
                    durability.persist_infallible(NoInput, persisted);
                    return result;
                }
            })
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    #[derive(Debug, Clone, PartialEq, IntoSchema)]
    struct SendInput {
        events: Vec<Event>,
        config: Config,
    }

    #[derive(Debug, IntoSchema)]
    struct NoInput;

    #[derive(Debug, Clone, FromSchema, IntoSchema)]
    struct NoOutput;

    #[derive(Debug, Clone, FromSchema, IntoSchema)]
    enum PersistedPollResult {
        Pending,
        Events(Vec<Result<StreamEvent, Error>>),
        Terminal,
        StartedReplayContinuation(Option<Vec<Result<StreamEvent, Error>>>),
    }

    #[derive(Debug, FromSchema, IntoSchema)]
    struct UnusedError;

    impl Display for UnusedError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "UnusedError")
        }
    }
}

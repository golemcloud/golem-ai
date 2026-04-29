use crate::model::{Config, ContentPart, Error, Event, Message, Role, StreamDelta};
use crate::LlmProvider;
use golem_rust::golem_wasm::Pollable;
use indoc::indoc;
use std::marker::PhantomData;

/// Wraps an LLM implementation with custom durability
pub struct DurableLLM<Impl> {
    phantom: PhantomData<Impl>,
}

/// Trait to be implemented in addition to the LLM `Guest` trait when wrapping it with `DurableLLM`.
#[allow(async_fn_in_trait)]
pub trait ExtendedLlmProvider: LlmProvider + 'static {
    /// Creates an instance of the LLM specific `ChatStream` without wrapping it in a `Resource`
    async fn unwrapped_stream(events: Vec<Event>, config: Config) -> Self::ChatStream;

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

/// When the durability feature flag is off, wrapping with `DurableLLM` is just a passthrough.
/// The WIT `Guest` exports are synchronous, so we wrap the async provider calls in `block_on` at
/// the WIT boundary. This path is for standalone WASM components that are not running inside an
/// agent's outer `block_on`, so a single `block_on` here is safe.
#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use crate::durability::{DurableLLM, ExtendedLlmProvider};
    use crate::init_logging;
    use crate::model::{
        ChatStream, Config, Error, Event, Guest, Message, Response, ToolCall, ToolResult,
    };
    use wstd::runtime::block_on;

    impl<Impl: ExtendedLlmProvider> Guest for DurableLLM<Impl> {
        type ChatStream = Impl::ChatStream;

        fn send(events: Vec<Event>, config: Config) -> Result<Response, Error> {
            init_logging();
            block_on(Impl::send(events, config))
        }

        fn stream(events: Vec<Event>, config: Config) -> ChatStream {
            init_logging();
            block_on(Impl::stream(events, config))
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
#[cfg(feature = "durability")]
mod durable_impl {
    use crate::durability::{DurableLLM, ExtendedLlmProvider};
    use crate::model::{ChatStream, Config, Error, Event, Response, StreamDelta, StreamEvent};
    use crate::{init_logging, ChatStreamInterface, LlmProvider};
    use async_trait::async_trait;
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    #[cfg(not(feature = "nopoll"))]
    use golem_rust::bindings::golem::durability::durability::LazyInitializedPollable;
    use golem_rust::durability::Durability;
    use golem_rust::golem_wasm::Pollable;
    use golem_rust::{
        with_persistence_level, with_persistence_level_async, FromValueAndType, IntoValue,
        PersistenceLevel,
    };
    use std::cell::RefCell;
    use std::fmt::{Display, Formatter};

    impl<Impl: ExtendedLlmProvider> LlmProvider for DurableLLM<Impl> {
        type ChatStream = DurableChatStream<Impl>;

        async fn send(events: Vec<Event>, config: Config) -> Result<Response, Error> {
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
                        Impl::send(events_clone, config_clone).await
                    })
                    .await;
                durability.persist_serializable(SendInput { events, config }, result.clone());
                result
            } else {
                durability.replay_serializable()
            }
        }

        async fn stream(events: Vec<Event>, config: Config) -> ChatStream {
            init_logging();

            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_ai_llm",
                "stream",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let events_clone = events.clone();
                let config_clone = config.clone();
                let result =
                    with_persistence_level_async(PersistenceLevel::PersistNothing, || async move {
                        ChatStream::new(DurableChatStream::<Impl>::live(
                            <Impl as ExtendedLlmProvider>::unwrapped_stream(
                                events_clone,
                                config_clone,
                            )
                            .await,
                        ))
                    })
                    .await;
                let _ = durability.persist_infallible(SendInput { events, config }, NoOutput);
                result
            } else {
                let _: NoOutput = durability.replay_infallible();
                ChatStream::new(DurableChatStream::<Impl>::replay(
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
            stream: Impl::ChatStream,
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
        },
    }

    pub struct DurableChatStream<Impl: ExtendedLlmProvider> {
        state: RefCell<Option<DurableChatStreamState<Impl>>>,
        subscription: RefCell<Option<Pollable>>,
    }

    impl<Impl: ExtendedLlmProvider> DurableChatStream<Impl> {
        fn live(stream: Impl::ChatStream) -> Self {
            Self {
                state: RefCell::new(Some(DurableChatStreamState::Live {
                    stream,
                    #[cfg(not(feature = "nopoll"))]
                    pollables: Vec::new(),
                })),
                subscription: RefCell::new(None),
            }
        }

        fn replay(original_events: Vec<Result<Event, Error>>, config: Config) -> Self {
            Self {
                state: RefCell::new(Some(DurableChatStreamState::Replay {
                    original_events,
                    config,
                    #[cfg(not(feature = "nopoll"))]
                    pollables: Vec::new(),
                    partial_result: Vec::new(),
                    finished: false,
                })),
                subscription: RefCell::new(None),
            }
        }
        #[cfg(not(feature = "nopoll"))]
        fn subscribe(&self) -> Pollable {
            let mut state = self.state.borrow_mut();
            match &mut *state {
                Some(DurableChatStreamState::Live { stream, .. }) => Impl::subscribe(stream),
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

    #[async_trait(?Send)]
    impl<Impl: ExtendedLlmProvider> ChatStreamInterface for DurableChatStream<Impl> {
        async fn poll_next(&self) -> Option<Vec<Result<StreamEvent, Error>>> {
            let durability =
                Durability::<Option<Vec<Result<StreamEvent, Error>>>, UnusedError>::new(
                    "golem_ai_llm",
                    "poll_next",
                    DurableFunctionType::ReadRemote,
                );
            if durability.is_live() {
                // Take the state out of the RefCell so we don't hold a borrow across .await.
                // We always put a state back before returning.
                let taken_state = self.state.borrow_mut().take();
                let (result, next_state) = match taken_state {
                    Some(DurableChatStreamState::Live {
                        stream,
                        #[cfg(not(feature = "nopoll"))]
                        pollables,
                    }) => {
                        let (stream, result) = with_persistence_level_async(
                            PersistenceLevel::PersistNothing,
                            || async move {
                                let result = stream.poll_next().await;
                                (stream, result)
                            },
                        )
                        .await;
                        durability.persist_infallible(NoInput, result.clone());
                        (
                            result,
                            DurableChatStreamState::Live {
                                stream,
                                #[cfg(not(feature = "nopoll"))]
                                pollables,
                            },
                        )
                    }
                    Some(DurableChatStreamState::Replay {
                        config,
                        original_events,
                        #[cfg(not(feature = "nopoll"))]
                        pollables,
                        partial_result,
                        finished,
                    }) => {
                        if finished {
                            (
                                None,
                                DurableChatStreamState::Replay {
                                    config,
                                    original_events,
                                    #[cfg(not(feature = "nopoll"))]
                                    pollables,
                                    partial_result,
                                    finished,
                                },
                            )
                        } else {
                            let extended_events =
                                Impl::retry_prompt(&original_events, &partial_result);

                            #[cfg(not(feature = "nopoll"))]
                            let (stream, first_live_result, pollables) =
                                with_persistence_level_async(
                                    PersistenceLevel::PersistNothing,
                                    || async move {
                                        let stream =
                                            <Impl as ExtendedLlmProvider>::unwrapped_stream(
                                                extended_events,
                                                config,
                                            )
                                            .await;
                                        for lazy_initialized_pollable in &pollables {
                                            lazy_initialized_pollable.set(Impl::subscribe(&stream));
                                        }
                                        let next = stream.poll_next().await;
                                        (stream, next, pollables)
                                    },
                                )
                                .await;
                            #[cfg(feature = "nopoll")]
                            let (stream, first_live_result) = with_persistence_level_async(
                                PersistenceLevel::PersistNothing,
                                || async move {
                                    let stream = <Impl as ExtendedLlmProvider>::unwrapped_stream(
                                        extended_events,
                                        config,
                                    )
                                    .await;
                                    let next = stream.poll_next().await;
                                    (stream, next)
                                },
                            )
                            .await;

                            durability.persist_infallible(NoInput, first_live_result.clone());
                            (
                                first_live_result,
                                DurableChatStreamState::Live {
                                    stream,
                                    #[cfg(not(feature = "nopoll"))]
                                    pollables,
                                },
                            )
                        }
                    }
                    None => unreachable!(),
                };

                *self.state.borrow_mut() = Some(next_state);

                result
            } else {
                let result: Option<Vec<Result<StreamEvent, Error>>> =
                    durability.replay_infallible();
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurableChatStreamState::Live { .. }) => {
                        unreachable!("Durable chat stream cannot be in live mode during replay")
                    }
                    Some(DurableChatStreamState::Replay {
                        partial_result,
                        finished,
                        ..
                    }) => match &result {
                        Some(result) => {
                            for event in result {
                                match event {
                                    Ok(StreamEvent::Delta(delta)) => {
                                        partial_result.push(delta.clone());
                                    }
                                    Ok(StreamEvent::Finish(_)) => {
                                        *finished = true;
                                    }
                                    Err(_) => {
                                        *finished = true;
                                    }
                                }
                            }
                        }
                        None => {
                            // NOP
                        }
                    },
                    None => {
                        unreachable!()
                    }
                }
                result
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

    #[derive(Debug, FromValueAndType, IntoValue)]
    struct UnusedError;

    impl Display for UnusedError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "UnusedError")
        }
    }
}

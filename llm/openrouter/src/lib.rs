mod client;
mod conversions;

use crate::client::{ChatCompletionChunk, CompletionsApi, CompletionsRequest, FunctionCall};
use crate::conversions::{
    convert_finish_reason, convert_usage, events_to_request, process_response,
};
use golem_llm::chat_stream::{LlmChatStream, LlmChatStreamState};
use golem_llm::config::{get_config_key, with_config_key};
use golem_llm::durability::{DurableLLM, ExtendedGuest};
use golem_llm::error::error_code_from_status;
use golem_llm::event_source::EventSource;
use golem_llm::golem::llm::llm::{
    ChatStream, Config, ContentPart, Error, ErrorCode, Event, FinishReason, Guest, Message,
    Response, ResponseMetadata, Role, StreamDelta, StreamEvent, ToolCall,
};
use golem_rust::golem_wasm::Pollable;
use golem_wasi_http::StatusCode;
use log::trace;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
struct JsonFragment {
    id: String,
    name: String,
    json: String,
}

struct OpenRouterChatStream {
    stream: RefCell<Option<EventSource>>,
    failure: Option<Error>,
    finished: RefCell<bool>,
    finish_reason: RefCell<Option<FinishReason>>,
    json_fragments: RefCell<HashMap<u32, JsonFragment>>,
}

impl OpenRouterChatStream {
    pub fn new(stream: EventSource) -> LlmChatStream<Self> {
        LlmChatStream::new(OpenRouterChatStream {
            stream: RefCell::new(Some(stream)),
            failure: None,
            finished: RefCell::new(false),
            finish_reason: RefCell::new(None),
            json_fragments: RefCell::new(HashMap::new()),
        })
    }

    pub fn failed(error: Error) -> LlmChatStream<Self> {
        LlmChatStream::new(OpenRouterChatStream {
            stream: RefCell::new(None),
            failure: Some(error),
            finished: RefCell::new(false),
            finish_reason: RefCell::new(None),
            json_fragments: RefCell::new(HashMap::new()),
        })
    }
}

impl LlmChatStreamState for OpenRouterChatStream {
    fn failure(&self) -> &Option<Error> {
        &self.failure
    }

    fn is_finished(&self) -> bool {
        *self.finished.borrow()
    }

    fn set_finished(&self) {
        *self.finished.borrow_mut() = true;
    }

    fn stream(&self) -> Ref<'_, Option<EventSource>> {
        self.stream.borrow()
    }

    fn stream_mut(&self) -> RefMut<'_, Option<EventSource>> {
        self.stream.borrow_mut()
    }

    fn decode_message(&self, raw: &str) -> Result<Option<StreamEvent>, Error> {
        fn decode_internal_error<S: Into<String>>(message: S) -> Error {
            Error {
                code: ErrorCode::InternalError,
                message: message.into(),
                provider_error_json: None,
            }
        }

        trace!("Received raw stream event: {raw}");
        if raw.starts_with(": ") {
            Ok(None) // comment
        } else {
            let json: serde_json::Value = serde_json::from_str(raw).map_err(|err| {
                decode_internal_error(format!("Failed to deserialize stream event: {err}"))
            })?;

            let typ = json
                .as_object()
                .and_then(|obj| obj.get("object"))
                .and_then(|v| v.as_str());
            match typ {
                Some("chat.completion.chunk") => {
                    let message: ChatCompletionChunk =
                        serde_json::from_value(json).map_err(|err| {
                            decode_internal_error(format!("Failed to parse stream event: {err}"))
                        })?;
                    if let Some(usage) = message.usage {
                        let finish_reason = self.finish_reason.borrow();
                        Ok(Some(StreamEvent::Finish(ResponseMetadata {
                            finish_reason: *finish_reason,
                            usage: Some(convert_usage(&usage)),
                            provider_id: None,
                            timestamp: Some(message.created.to_string()),
                            provider_metadata_json: None,
                        })))
                    } else if let Some(choice) = message.choices.into_iter().next() {
                        if let Some(finish_reason) = choice.finish_reason {
                            *self.finish_reason.borrow_mut() =
                                Some(convert_finish_reason(&finish_reason));
                        }
                        if let Some(error) = choice.error {
                            Err(Error {
                                code: error_code_from_status(
                                    TryInto::<u16>::try_into(error.code)
                                        .ok()
                                        .and_then(|code| StatusCode::from_u16(code).ok())
                                        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                                ),
                                message: error.message,
                                provider_error_json: error
                                    .metadata
                                    .map(|value| serde_json::to_string(&value).unwrap()),
                            })
                        } else {
                            let content = choice
                                .delta
                                .content
                                .map(|text| vec![ContentPart::Text(text)]);

                            let mut seen_indices = HashSet::new();
                            let mut tool_calls = Vec::new();
                            let mut json_fragments = self.json_fragments.borrow_mut();

                            for tool_call in choice.delta.tool_calls.unwrap_or_default() {
                                match tool_call {
                                    client::ToolCall::Function {
                                        id: Some(id),
                                        function:
                                            FunctionCall {
                                                name: Some(name),
                                                arguments,
                                            },
                                        index: None,
                                    } => {
                                        // Full tool call
                                        tool_calls.push(ToolCall {
                                            id,
                                            name,
                                            arguments_json: arguments,
                                        });
                                    }
                                    client::ToolCall::Function {
                                        id: Some(id),
                                        function:
                                            FunctionCall {
                                                name: Some(name),
                                                arguments,
                                            },
                                        index: Some(index),
                                    } => {
                                        // Beginning of a streamed tool call
                                        json_fragments.insert(
                                            index,
                                            JsonFragment {
                                                id,
                                                name,
                                                json: arguments,
                                            },
                                        );
                                        seen_indices.insert(index);
                                    }
                                    client::ToolCall::Function {
                                        id: _,
                                        function: FunctionCall { name: _, arguments },
                                        index: Some(index),
                                    } => {
                                        // Fragment
                                        let fragment = json_fragments.entry(index).or_default();
                                        fragment.json.push_str(&arguments);
                                        seen_indices.insert(index);
                                    }
                                    _ => {
                                        return Err(decode_internal_error(format!(
                                            "Unexpected tool call format: {tool_call:?}"
                                        )));
                                    }
                                }
                            }

                            let indices =
                                json_fragments.keys().copied().collect::<Vec<_>>().clone();
                            for index in indices {
                                if !seen_indices.contains(&index) {
                                    // Emitting finished tool call
                                    let fragment = json_fragments.remove(&index).unwrap();
                                    tool_calls.push(ToolCall {
                                        id: fragment.id,
                                        name: fragment.name,
                                        arguments_json: fragment.json,
                                    });
                                }
                            }

                            Ok(Some(StreamEvent::Delta(StreamDelta {
                                content,
                                tool_calls: if tool_calls.is_empty() {
                                    None
                                } else {
                                    Some(tool_calls)
                                },
                            })))
                        }
                    } else {
                        Ok(None)
                    }
                }
                Some(_) => Ok(None),
                None => Err(decode_internal_error(
                    "Unexpected stream event format, does not have 'object' field".to_string(),
                )),
            }
        }
    }
}

struct OpenRouterComponent;

impl OpenRouterComponent {
    const ENV_VAR_NAME: &'static str = "OPENROUTER_API_KEY";

    fn request(client: CompletionsApi, request: CompletionsRequest) -> Result<Response, Error> {
        let response = client.send_messages(request)?;
        process_response(response)
    }

    fn streaming_request(
        client: CompletionsApi,
        mut request: CompletionsRequest,
    ) -> LlmChatStream<OpenRouterChatStream> {
        request.stream = Some(true);
        match client.stream_send_messages(request) {
            Ok(stream) => OpenRouterChatStream::new(stream),
            Err(err) => OpenRouterChatStream::failed(err),
        }
    }
}

impl Guest for OpenRouterComponent {
    type ChatStream = LlmChatStream<OpenRouterChatStream>;

    fn send(events: Vec<Event>, config: Config) -> Result<Response, Error> {
        let openrouter_api_key = get_config_key(Self::ENV_VAR_NAME)?;
        let client = CompletionsApi::new(openrouter_api_key);
        let request = events_to_request(events, config)?;
        Self::request(client, request)
    }

    fn stream(events: Vec<Event>, config: Config) -> ChatStream {
        ChatStream::new(Self::unwrapped_stream(events, config))
    }
}

impl ExtendedGuest for OpenRouterComponent {
    fn unwrapped_stream(events: Vec<Event>, config: Config) -> LlmChatStream<OpenRouterChatStream> {
        with_config_key(
            Self::ENV_VAR_NAME,
            OpenRouterChatStream::failed,
            |openrouter_api_key| {
                let client = CompletionsApi::new(openrouter_api_key);

                match events_to_request(events, config) {
                    Ok(request) => Self::streaming_request(client, request),
                    Err(err) => OpenRouterChatStream::failed(err),
                }
            },
        )
    }

    fn retry_prompt(
        original_events: &[Result<Event, Error>],
        partial_result: &[StreamDelta],
    ) -> Vec<Event> {
        let mut extended_events = Vec::new();
        extended_events.push(Event::Message(Message {
            role: Role::System,
            name: None,
            content: vec![
                ContentPart::Text(
                    "You were asked the same question previously, but the response was interrupted before completion. \
                     Please continue your response from where you left off. \
                     Do not include the part of the response that was already seen.".to_string()),
            ],
        }));
        extended_events.push(Event::Message(Message {
            role: Role::User,
            name: None,
            content: vec![ContentPart::Text(
                "Here is the original question:".to_string(),
            )],
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
            role: Role::User,
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

    fn subscribe(stream: &Self::ChatStream) -> Pollable {
        stream.subscribe()
    }
}

type DurableOpenRouterComponent = DurableLLM<OpenRouterComponent>;

golem_llm::export_llm!(DurableOpenRouterComponent with_types_in golem_llm);

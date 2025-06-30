mod client;
mod conversions;

use crate::client::{ChatCompletionChunk, CompletionsApi, CompletionsRequest};
use crate::conversions::{
    convert_finish_reason, convert_usage, create_request, process_response,
    tool_results_to_messages,
};
use golem_llm::chat_stream::{LlmChatStream, LlmChatStreamState};
use golem_llm::config::with_config_key;
use golem_llm::durability::{DurableLLM, ExtendedGuest};
use golem_llm::event_source::EventSource;
use golem_llm::golem::llm::llm::{
    ChatEvent, ChatStream, Config, ContentPart, Error, FinishReason, Guest, Message,
    ResponseMetadata, Role, StreamDelta, StreamEvent, ToolCall, ToolResult,
};
use golem_llm::LOGGING_STATE;
use golem_rust::wasm_rpc::Pollable;
use log::trace;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;

#[derive(Default)]
struct JsonFragment {
    id: String,
    name: String,
    json: String,
}

struct OpenAIChatStream {
    stream: RefCell<Option<EventSource>>,
    failure: Option<Error>,
    finished: RefCell<bool>,
    finish_reason: RefCell<Option<FinishReason>>,
    json_fragments: RefCell<HashMap<u32, JsonFragment>>,
}

impl OpenAIChatStream {
    pub fn new(stream: EventSource) -> LlmChatStream<Self> {
        LlmChatStream::new(OpenAIChatStream {
            stream: RefCell::new(Some(stream)),
            failure: None,
            finished: RefCell::new(false),
            finish_reason: RefCell::new(None),
            json_fragments: RefCell::new(HashMap::new()),
        })
    }

    pub fn failed(error: Error) -> LlmChatStream<Self> {
        LlmChatStream::new(OpenAIChatStream {
            stream: RefCell::new(None),
            failure: Some(error),
            finished: RefCell::new(false),
            finish_reason: RefCell::new(None),
            json_fragments: RefCell::new(HashMap::new()),
        })
    }

    fn set_finished(&self) {
        *self.finished.borrow_mut() = true;
    }

    fn set_finish_reason(&self, finish_reason: FinishReason) {
        *self.finish_reason.borrow_mut() = Some(finish_reason);
    }

    fn get_finish_reason(&self) -> Option<FinishReason> {
        *self.finish_reason.borrow()
    }
}

impl LlmChatStreamState for OpenAIChatStream {
    fn failure(&self) -> &Option<Error> {
        &self.failure
    }

    fn is_finished(&self) -> bool {
        *self.finished.borrow()
    }

    fn set_finished(&self) {
        *self.finished.borrow_mut() = true;
    }

    fn stream(&self) -> Ref<Option<EventSource>> {
        self.stream.borrow()
    }

    fn stream_mut(&self) -> RefMut<Option<EventSource>> {
        self.stream.borrow_mut()
    }

    fn decode_message(&self, raw: &str) -> Result<Option<StreamEvent>, String> {
        trace!("Received raw stream event: {raw}");

        if raw.starts_with("data: [DONE]") {
            self.set_finished();
            return Ok(None);
        }

        if !raw.starts_with("data: ") {
            return Ok(None);
        }

        let json_str = &raw[6..];
        let json: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|err| format!("Failed to parse stream event JSON: {err}"))?;

        let chunk: ChatCompletionChunk = serde_json::from_value(json)
            .map_err(|err| format!("Failed to deserialize stream event: {err}"))?;

        if let Some(choice) = chunk.choices.into_iter().next() {
            if let Some(finish_reason) = choice.finish_reason {
                self.set_finish_reason(convert_finish_reason(&finish_reason));
            }

            let delta = &choice.delta;

            if let Some(content) = &delta.content {
                return Ok(Some(StreamEvent::Delta(StreamDelta {
                    content: Some(vec![ContentPart::Text(content.clone())]),
                    tool_calls: None,
                })));
            }

            if let Some(tool_calls) = &delta.tool_calls {
                let mut fragments = self.json_fragments.borrow_mut();
                let mut result_tool_calls = Vec::new();

                for tool_call in tool_calls {
                    match tool_call {
                        crate::client::ToolCall::Function {
                            function,
                            id,
                            index,
                        } => {
                            let idx = index.unwrap_or(0);

                            let fragment = fragments.entry(idx).or_insert_with(|| JsonFragment {
                                id: id.clone(),
                                name: function.name.clone(),
                                json: String::new(),
                            });

                            if !function.arguments.is_empty() {
                                fragment.json.push_str(&function.arguments);
                            }

                            // Only emit when we have content to add
                            if !fragment.id.is_empty() && !fragment.name.is_empty() {
                                result_tool_calls.push(ToolCall {
                                    id: fragment.id.clone(),
                                    name: fragment.name.clone(),
                                    arguments_json: fragment.json.clone(),
                                });
                            }
                        }
                    }
                }

                if !result_tool_calls.is_empty() {
                    return Ok(Some(StreamEvent::Delta(StreamDelta {
                        content: None,
                        tool_calls: Some(result_tool_calls),
                    })));
                }
            }
        }

        if let Some(usage) = chunk.usage {
            let finish_reason = self.get_finish_reason();
            return Ok(Some(StreamEvent::Finish(ResponseMetadata {
                finish_reason,
                usage: Some(convert_usage(&usage)),
                provider_id: Some(chunk.id),
                timestamp: Some(chunk.created.to_string()),
                provider_metadata_json: None,
            })));
        }

        Ok(None)
    }
}

struct OpenAIComponent;

impl OpenAIComponent {
    const ENV_VAR_NAME: &'static str = "OPENAI_API_KEY";

    fn request(client: CompletionsApi, request: CompletionsRequest) -> ChatEvent {
        match client.send_messages(request) {
            Ok(response) => process_response(response),
            Err(error) => ChatEvent::Error(error),
        }
    }

    fn streaming_request(
        client: CompletionsApi,
        mut request: CompletionsRequest,
    ) -> LlmChatStream<OpenAIChatStream> {
        request.stream = Some(true);
        match client.stream_send_messages(request) {
            Ok(stream) => OpenAIChatStream::new(stream),
            Err(error) => OpenAIChatStream::failed(error),
        }
    }
}

impl Guest for OpenAIComponent {
    type ChatStream = LlmChatStream<OpenAIChatStream>;

    fn send(messages: Vec<Message>, config: Config) -> ChatEvent {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        with_config_key(Self::ENV_VAR_NAME, ChatEvent::Error, |openai_api_key| {
            let client = CompletionsApi::new(openai_api_key);

            match create_request(messages, config) {
                Ok(request) => Self::request(client, request),
                Err(err) => ChatEvent::Error(err),
            }
        })
    }

    fn continue_(
        messages: Vec<Message>,
        tool_results: Vec<(ToolCall, ToolResult)>,
        config: Config,
    ) -> ChatEvent {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        with_config_key(Self::ENV_VAR_NAME, ChatEvent::Error, |openai_api_key| {
            let client = CompletionsApi::new(openai_api_key);

            match create_request(messages, config) {
                Ok(mut request) => {
                    request
                        .messages
                        .extend(tool_results_to_messages(tool_results));
                    Self::request(client, request)
                }
                Err(err) => ChatEvent::Error(err),
            }
        })
    }

    fn stream(messages: Vec<Message>, config: Config) -> ChatStream {
        ChatStream::new(Self::unwrapped_stream(messages, config))
    }
}

impl ExtendedGuest for OpenAIComponent {
    fn unwrapped_stream(messages: Vec<Message>, config: Config) -> LlmChatStream<OpenAIChatStream> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        with_config_key(
            Self::ENV_VAR_NAME,
            OpenAIChatStream::failed,
            |openai_api_key| {
                let client = CompletionsApi::new(openai_api_key);

                match create_request(messages, config) {
                    Ok(request) => Self::streaming_request(client, request),
                    Err(err) => OpenAIChatStream::failed(err),
                }
            },
        )
    }

    fn retry_prompt(original_messages: &[Message], partial_result: &[StreamDelta]) -> Vec<Message> {
        let mut extended_messages = Vec::new();
        extended_messages.push(Message {
            role: Role::System,
            name: None,
            content: vec![
                ContentPart::Text(
                    "You were asked the same question previously, but the response was interrupted before completion. \
                     Please continue your response from where you left off. \
                     Do not include the part of the response that was already seen.".to_string()),
            ],
        });
        extended_messages.push(Message {
            role: Role::User,
            name: None,
            content: vec![ContentPart::Text(
                "Here is the original question:".to_string(),
            )],
        });
        extended_messages.extend_from_slice(original_messages);

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

        extended_messages.push(Message {
            role: Role::User,
            name: None,
            content: vec![ContentPart::Text(
                "Here is the partial response that was successfully received:".to_string(),
            )]
            .into_iter()
            .chain(partial_result_as_content)
            .collect(),
        });
        extended_messages
    }

    fn subscribe(stream: &Self::ChatStream) -> Pollable {
        stream.subscribe()
    }
}

type DurableOpenAIComponent = DurableLLM<OpenAIComponent>;

golem_llm::export_llm!(DurableOpenAIComponent with_types_in golem_llm);

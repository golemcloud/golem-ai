mod client;
mod conversions;

use crate::client::{BedrockClient, ConverseRequest};
use crate::conversions::{
    convert_usage, messages_to_request, process_response, stop_reason_to_finish_reason,
    tool_results_to_messages,
};
use golem_llm::chat_stream::{LlmChatStream, LlmChatStreamState};
use golem_llm::config::with_config_keys;
use golem_llm::durability::{DurableLLM, ExtendedGuest};
use golem_llm::event_source::EventSource;
use golem_llm::golem::llm::llm::{
    ChatEvent, ChatStream, Config, ContentPart, Error, Guest, Message, ResponseMetadata, Role,
    StreamDelta, StreamEvent, ToolCall, ToolResult,
};
use golem_llm::LOGGING_STATE;
use golem_rust::wasm_rpc::Pollable;
use log::trace;
use serde::Deserialize;
use serde_json::Value;
use std::cell::{Ref, RefCell, RefMut};

struct BedrockChatStream {
    stream: RefCell<Option<EventSource>>,
    failure: Option<Error>,
    finished: RefCell<bool>,
}

#[derive(Debug, Deserialize)]
pub struct EventContentBlock {
    #[serde(rename = "contentBlockIndex")]
    pub content_block_index: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<Delta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<ToolUseStart>,
    pub p: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Delta {
    ToolUse {
        #[serde(rename = "toolUse")]
        tool_use: ToolUse,
    },
    Text {
        text: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct ToolUse {
    pub input: String,
}

#[derive(Debug, Deserialize)]
pub struct ToolUseStart {
    #[serde(rename = "toolUse")]
    pub tool_use: ToolUseInfo,
}

#[derive(Debug, Deserialize)]
pub struct ToolUseInfo {
    pub name: String,
    #[serde(rename = "toolUseId")]
    pub tool_use_id: String,
}

#[derive(Debug, Deserialize)]
pub struct MetadataMessage {
    pub p: String,
    pub usage: Option<Usage>,
    pub metrics: Option<Metrics>,
}

#[derive(Debug, Deserialize)]
pub struct Metrics {
    #[serde(rename = "latencyMs")]
    pub latency_ms: u32,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    #[serde(rename = "outputTokens")]
    pub output_tokens: u32,
    #[serde(rename = "totalTokens")]
    pub total_tokens: u32,
}

impl BedrockChatStream {
    pub fn new(stream: EventSource) -> LlmChatStream<Self> {
        LlmChatStream::new(BedrockChatStream {
            stream: RefCell::new(Some(stream)),
            failure: None,
            finished: RefCell::new(false),
        })
    }

    pub fn failed(error: Error) -> LlmChatStream<Self> {
        LlmChatStream::new(BedrockChatStream {
            stream: RefCell::new(None),
            failure: Some(error),
            finished: RefCell::new(false),
        })
    }
}

impl LlmChatStreamState for BedrockChatStream {
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

        let json: Value = serde_json::from_str(raw)
            .map_err(|err| format!("Failed to deserialize stream event: {err}"))?;

        if json.get("role").is_some() {
            return Ok(Some(StreamEvent::Delta(StreamDelta {
                content: Some(vec![ContentPart::Text(json.to_string())]),
                tool_calls: None,
            })));
        }

        if json.get("usage").is_some() || json.get("metrics").is_some() {
            if let Ok(metadata) = serde_json::from_value::<MetadataMessage>(json.clone()) {
                let usage = metadata.usage.unwrap();
                return Ok(Some(StreamEvent::Finish(ResponseMetadata {
                    finish_reason: None,
                    usage: Some(golem_llm::golem::llm::llm::Usage {
                        input_tokens: Some(usage.total_tokens - usage.output_tokens),
                        output_tokens: Some(usage.output_tokens),
                        total_tokens: Some(usage.total_tokens),
                    }),
                    provider_id: None,
                    timestamp: None,
                    provider_metadata_json: if metadata.metrics.is_some() {
                        Some(format!("{:?}", metadata.metrics.unwrap()))
                    }else {
                        None
                    },
                })));
            }
        }

        match serde_json::from_value::<EventContentBlock>(json.clone()) {
            Ok(event_content_block) => {
                if let Some(delta) = event_content_block.delta {
                    match delta {
                        Delta::Text { text } => {
                            return Ok(Some(StreamEvent::Delta(StreamDelta {
                                content: Some(vec![ContentPart::Text(text)]),
                                tool_calls: None,
                            })));
                        }
                        Delta::ToolUse { tool_use } => {
                            return Ok(Some(StreamEvent::Delta(StreamDelta {
                                content: Some(vec![ContentPart::Text(tool_use.input)]),
                                tool_calls: None,
                            })));
                        }
                    }
                }
                if let Some(tool_use_start) = event_content_block.start {
                    return Ok(Some(StreamEvent::Delta(StreamDelta {
                        content: Some(vec![]),
                        tool_calls: Some(vec![ToolCall {
                            id: tool_use_start.tool_use.tool_use_id,
                            name: tool_use_start.tool_use.name,
                            arguments_json: "".to_string(),
                        }]),
                    })));
                }
            }
            Err(_) => {}
        }

        Ok(None)
    }
}

struct BedrockComponent;

impl BedrockComponent {
    const ACCESS_KEY_ID_ENV_VAR: &'static str = "AWS_ACCESS_KEY_ID";
    const SECRET_ACCESS_KEY_ENV_VAR: &'static str = "AWS_SECRET_ACCESS_KEY";
    const REGION_ENV_VAR: &'static str = "AWS_REGION";

    fn request(client: BedrockClient, model_id: &str, request: ConverseRequest) -> ChatEvent {
        match client.converse(model_id, request) {
            Ok(response) => process_response(response),
            Err(err) => ChatEvent::Error(err),
        }
    }

    fn streaming_request(
        client: BedrockClient,
        model_id: &str,
        request: ConverseRequest,
    ) -> LlmChatStream<BedrockChatStream> {
        match client.converse_stream(model_id, request) {
            Ok(stream) => BedrockChatStream::new(stream),
            Err(err) => BedrockChatStream::failed(err),
        }
    }
}

impl Guest for BedrockComponent {
    type ChatStream = LlmChatStream<BedrockChatStream>;

    fn send(messages: Vec<Message>, config: Config) -> ChatEvent {
        LOGGING_STATE.with_borrow_mut(|state| state.init());
        with_config_keys(
            &[
                Self::ACCESS_KEY_ID_ENV_VAR,
                Self::SECRET_ACCESS_KEY_ENV_VAR,
                Self::REGION_ENV_VAR,
            ],
            ChatEvent::Error,
            |bedrock_api_keys| {
                let client = BedrockClient::new(
                    bedrock_api_keys[Self::ACCESS_KEY_ID_ENV_VAR].clone(),
                    bedrock_api_keys[Self::SECRET_ACCESS_KEY_ENV_VAR].clone(),
                    bedrock_api_keys[Self::REGION_ENV_VAR].clone(),
                );

                match messages_to_request(messages, config.clone()) {
                    Ok(request) => Self::request(client, &config.model, request),
                    Err(err) => ChatEvent::Error(err),
                }
            },
        )
    }

    fn continue_(
        messages: Vec<Message>,
        tool_results: Vec<(ToolCall, ToolResult)>,
        config: Config,
    ) -> ChatEvent {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        with_config_keys(
            &[
                Self::ACCESS_KEY_ID_ENV_VAR,
                Self::SECRET_ACCESS_KEY_ENV_VAR,
                Self::REGION_ENV_VAR,
            ],
            ChatEvent::Error,
            |bedrock_api_keys| {
                let client = BedrockClient::new(
                    bedrock_api_keys[Self::ACCESS_KEY_ID_ENV_VAR].clone(),
                    bedrock_api_keys[Self::SECRET_ACCESS_KEY_ENV_VAR].clone(),
                    bedrock_api_keys[Self::REGION_ENV_VAR].clone(),
                );

                match messages_to_request(messages, config.clone()) {
                    Ok(mut request) => {
                        request
                            .messages
                            .extend(tool_results_to_messages(tool_results));
                        Self::request(client, &config.model, request)
                    }
                    Err(err) => ChatEvent::Error(err),
                }
            },
        )
    }

    fn stream(messages: Vec<Message>, config: Config) -> ChatStream {
        ChatStream::new(Self::unwrapped_stream(messages, config))
    }
}

impl ExtendedGuest for BedrockComponent {
    fn unwrapped_stream(
        messages: Vec<Message>,
        config: Config,
    ) -> LlmChatStream<BedrockChatStream> {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        with_config_keys(
            &[
                Self::ACCESS_KEY_ID_ENV_VAR,
                Self::SECRET_ACCESS_KEY_ENV_VAR,
                Self::REGION_ENV_VAR,
            ],
            BedrockChatStream::failed,
            |bedrock_api_keys| {
                let client = BedrockClient::new(
                    bedrock_api_keys[Self::ACCESS_KEY_ID_ENV_VAR].clone(),
                    bedrock_api_keys[Self::SECRET_ACCESS_KEY_ENV_VAR].clone(),
                    bedrock_api_keys[Self::REGION_ENV_VAR].clone(),
                );

                match messages_to_request(messages, config.clone()) {
                    Ok(request) => Self::streaming_request(client, &config.model, request),
                    Err(err) => BedrockChatStream::failed(err),
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

type DurableBedrockComponent = DurableLLM<BedrockComponent>;

golem_llm::export_llm!(DurableBedrockComponent with_types_in golem_llm);

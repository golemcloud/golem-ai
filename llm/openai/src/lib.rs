use crate::client::{
    ChatCompletionChunk, ChatMessage, ChatApi,
};
use crate::conversions::{
    create_request, create_chunk_response_metadata, messages_to_chat_messages, parse_error_code,
    process_chat_completion, tool_defs_to_tools, tool_results_to_chat_messages,
};
use golem_llm::chat_stream::{LlmChatStream, LlmChatStreamState};
use golem_llm::config::with_config_key;
use golem_llm::durability::{DurableLLM, ExtendedGuest};
use golem_llm::event_source::EventSource;
use golem_llm::golem::llm::llm::{
    ChatEvent, ChatStream, Config, ContentPart, Error, Guest, Message, StreamDelta,
    StreamEvent, ToolCall, ToolResult,
};
use golem_llm::LOGGING_STATE;
use golem_rust::wasm_rpc::Pollable;
use log::trace;
use std::cell::{Ref, RefCell, RefMut};

mod client;
mod conversions;

struct OpenAIChatStream {
    stream: RefCell<Option<EventSource>>,
    failure: Option<Error>,
    finished: RefCell<bool>,
}

impl OpenAIChatStream {
    pub fn new(stream: EventSource) -> LlmChatStream<Self> {
        LlmChatStream::new(OpenAIChatStream {
            stream: RefCell::new(Some(stream)),
            failure: None,
            finished: RefCell::new(false),
        })
    }

    pub fn failed(error: Error) -> LlmChatStream<Self> {
        LlmChatStream::new(OpenAIChatStream {
            stream: RefCell::new(None),
            failure: Some(error),
            finished: RefCell::new(false),
        })
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
        
        // Handle the special [DONE] message
        if raw.trim() == "[DONE]" {
            return Ok(None);
        }
        
        let json: serde_json::Value = serde_json::from_str(raw)
            .map_err(|err| format!("Failed to deserialize stream event: {err}"))?;

        // Check if this is an error response
        if let Some(error) = json.get("error") {
            let error_message = error.get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            let error_code = error.get("code")
                .and_then(|c| c.as_str())
                .unwrap_or("unknown");
            
            return Ok(Some(StreamEvent::Error(Error {
                code: parse_error_code(error_code.to_string()),
                message: error_message.to_string(),
                provider_error_json: Some(raw.to_string()),
            })));
        }

        let decoded = serde_json::from_value::<ChatCompletionChunk>(json)
            .map_err(|err| format!("Failed to deserialize stream event: {err}"))?;

        if decoded.choices.is_empty() {
            return Ok(None);
        }

        let choice = &decoded.choices[0];
        
        // Handle finish_reason for completion
        if let Some(finish_reason) = &choice.finish_reason {
            if finish_reason == "stop" || finish_reason == "length" || finish_reason == "tool_calls" {
                let metadata = create_chunk_response_metadata(&decoded);
                return Ok(Some(StreamEvent::Finish(metadata)));
            }
        }

        // Handle content delta
        if let Some(content) = &choice.delta.content {
            return Ok(Some(StreamEvent::Delta(StreamDelta {
                content: Some(vec![ContentPart::Text(content.clone())]),
                tool_calls: None,
            })));
        }

        // Handle tool calls delta
        if !choice.delta.tool_calls.is_empty() {
            let mut tool_calls = Vec::new();
            for tool_call_delta in &choice.delta.tool_calls {
                if let (Some(id), Some(function)) = (&tool_call_delta.id, &tool_call_delta.function) {
                    if let (Some(name), Some(arguments)) = (&function.name, &function.arguments) {
                        tool_calls.push(ToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            arguments_json: arguments.clone(),
                        });
                    }
                }
            }
            if !tool_calls.is_empty() {
                return Ok(Some(StreamEvent::Delta(StreamDelta {
                    content: None,
                    tool_calls: Some(tool_calls),
                })));
            }
        }

        Ok(None)
    }
}

struct OpenAIComponent;

impl OpenAIComponent {
    const ENV_VAR_NAME: &'static str = "OPENAI_API_KEY";

    fn request(client: ChatApi, messages: Vec<ChatMessage>, config: Config) -> ChatEvent {
        match tool_defs_to_tools(&config.tools) {
            Ok(tools) => {
                let request = create_request(messages, config, tools);
                match client.create_chat_completion(request) {
                    Ok(response) => process_chat_completion(response),
                    Err(error) => ChatEvent::Error(error),
                }
            }
            Err(error) => ChatEvent::Error(error),
        }
    }

    fn streaming_request(
        client: ChatApi,
        messages: Vec<ChatMessage>,
        config: Config,
    ) -> LlmChatStream<OpenAIChatStream> {
        match tool_defs_to_tools(&config.tools) {
            Ok(tools) => {
                let mut request = create_request(messages, config, tools);
                request.stream = true;
                match client.stream_chat_completion(request) {
                    Ok(stream) => OpenAIChatStream::new(stream),
                    Err(error) => OpenAIChatStream::failed(error),
                }
            }
            Err(error) => OpenAIChatStream::failed(error),
        }
    }
}

impl Guest for OpenAIComponent {
    type ChatStream = LlmChatStream<OpenAIChatStream>;

    fn send(messages: Vec<Message>, config: Config) -> ChatEvent {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        with_config_key(Self::ENV_VAR_NAME, ChatEvent::Error, |openai_api_key| {
            let client = ChatApi::new(openai_api_key);

            let chat_messages = messages_to_chat_messages(messages);
            Self::request(client, chat_messages, config)
        })
    }

    fn continue_(
        messages: Vec<Message>,
        tool_results: Vec<(ToolCall, ToolResult)>,
        config: Config,
    ) -> ChatEvent {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        with_config_key(Self::ENV_VAR_NAME, ChatEvent::Error, |openai_api_key| {
            let client = ChatApi::new(openai_api_key);

            let mut chat_messages = messages_to_chat_messages(messages);
            chat_messages.extend(tool_results_to_chat_messages(tool_results));
            Self::request(client, chat_messages, config)
        })
    }

    fn stream(messages: Vec<Message>, config: Config) -> ChatStream {
        ChatStream::new(Self::unwrapped_stream(messages, config))
    }
}

impl ExtendedGuest for OpenAIComponent {
    fn unwrapped_stream(messages: Vec<Message>, config: Config) -> Self::ChatStream {
        LOGGING_STATE.with_borrow_mut(|state| state.init());

        with_config_key(
            Self::ENV_VAR_NAME,
            OpenAIChatStream::failed,
            |openai_api_key| {
                let client = ChatApi::new(openai_api_key);

                let chat_messages = messages_to_chat_messages(messages);
                Self::streaming_request(client, chat_messages, config)
            },
        )
    }

    fn subscribe(stream: &Self::ChatStream) -> Pollable {
        stream.subscribe()
    }
}

type DurableOpenAIComponent = DurableLLM<OpenAIComponent>;

golem_llm::export_llm!(DurableOpenAIComponent with_types_in golem_llm);

#[cfg(test)]
mod tests {
    use super::*;
    use golem_llm::golem::llm::llm::{Config, ContentPart, Message, Role, ToolCall, ToolResult, ToolSuccess};

    #[test]
    fn test_multi_turn_conversation_message_structure() {
        // Test that messages are correctly converted for multi-turn conversations
        let messages = vec![
            Message {
                role: Role::User,
                name: Some("vigoo".to_string()),
                content: vec![ContentPart::Text("Do you know what a haiku is?".to_string())],
            },
            Message {
                role: Role::Assistant,
                name: Some("assistant".to_string()),
                content: vec![ContentPart::Text("Yes, a haiku is a traditional Japanese poem with three lines following a 5-7-5 syllable pattern.".to_string())],
            },
            Message {
                role: Role::User,
                name: Some("vigoo".to_string()),
                content: vec![ContentPart::Text("Can you write one for me?".to_string())],
            },
        ];

        let chat_messages = conversions::messages_to_chat_messages(messages);
        
        // Verify the structure is correct for OpenAI Chat Completions API
        assert_eq!(chat_messages.len(), 3);
        
        // Check roles
        assert_eq!(chat_messages[0].role, "user");
        assert_eq!(chat_messages[1].role, "assistant");
        assert_eq!(chat_messages[2].role, "user");
        
        // Check names are preserved
        assert_eq!(chat_messages[0].name, Some("vigoo".to_string()));
        assert_eq!(chat_messages[1].name, Some("assistant".to_string()));
        assert_eq!(chat_messages[2].name, Some("vigoo".to_string()));
        
        // Check content
        if let Some(crate::client::MessageContent::Text(text)) = &chat_messages[0].content {
            assert_eq!(text, "Do you know what a haiku is?");
        } else {
            panic!("Expected text content");
        }
        
        if let Some(crate::client::MessageContent::Text(text)) = &chat_messages[1].content {
            assert!(text.contains("haiku"));
        } else {
            panic!("Expected text content");
        }
        
        if let Some(crate::client::MessageContent::Text(text)) = &chat_messages[2].content {
            assert_eq!(text, "Can you write one for me?");
        } else {
            panic!("Expected text content");
        }
        
        println!("✅ Multi-turn conversation message structure test passed!");
    }

    #[test]
    fn test_tool_results_conversion() {
        // Test that tool results are correctly converted to chat messages
        let tool_call = ToolCall {
            id: "call_123".to_string(),
            name: "test-tool".to_string(),
            arguments_json: r#"{"maximum": 10}"#.to_string(),
        };
        
        let tool_result = ToolResult::Success(ToolSuccess {
            id: "call_123".to_string(),
            name: "test-tool".to_string(),
            result_json: r#"{"value": 6}"#.to_string(),
            execution_time_ms: None,
        });
        
        let chat_messages = conversions::tool_results_to_chat_messages(vec![(tool_call, tool_result)]);
        
        // Should create assistant message with tool calls + tool result message
        assert_eq!(chat_messages.len(), 2);
        
        // First message should be assistant with tool calls
        assert_eq!(chat_messages[0].role, "assistant");
        assert_eq!(chat_messages[0].tool_calls.len(), 1);
        assert_eq!(chat_messages[0].tool_calls[0].id, "call_123");
        assert_eq!(chat_messages[0].tool_calls[0].function.name, "test-tool");
        
        // Second message should be tool result
        assert_eq!(chat_messages[1].role, "tool");
        assert_eq!(chat_messages[1].name, Some("call_123".to_string()));
        if let Some(crate::client::MessageContent::Text(content)) = &chat_messages[1].content {
            assert_eq!(content, r#"{"value": 6}"#);
        } else {
            panic!("Expected text content");
        }
        
        println!("✅ Tool results conversion test passed!");
    }

    #[test]
    fn test_request_creation() {
        // Test that requests are properly created for the Chat Completions API
        let chat_messages = vec![
            crate::client::ChatMessage {
                role: "user".to_string(),
                content: Some(crate::client::MessageContent::Text("Hello".to_string())),
                name: None,
                tool_calls: vec![],
            }
        ];
        
        let config = Config {
            model: "gpt-3.5-turbo".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(100),
            stop_sequences: Some(vec!["END".to_string()]),
            tools: vec![],
            tool_choice: None,
            provider_options: vec![],
        };
        
        let request = conversions::create_request(chat_messages, config, vec![]);
        
        // Verify request structure
        assert_eq!(request.model, "gpt-3.5-turbo");
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(100));
        assert_eq!(request.stop, vec!["END".to_string()]);
        assert_eq!(request.stream, false);
        assert_eq!(request.messages.len(), 1);
        
        println!("✅ Request creation test passed!");
    }

    #[test]
    fn test_finish_reason_conversion() {
        // Test that finish reasons are correctly converted
        assert_eq!(conversions::string_to_finish_reason("stop"), Some(golem_llm::golem::llm::llm::FinishReason::Stop));
        assert_eq!(conversions::string_to_finish_reason("length"), Some(golem_llm::golem::llm::llm::FinishReason::Length));
        assert_eq!(conversions::string_to_finish_reason("tool_calls"), Some(golem_llm::golem::llm::llm::FinishReason::ToolCalls));
        assert_eq!(conversions::string_to_finish_reason("content_filter"), Some(golem_llm::golem::llm::llm::FinishReason::ContentFilter));
        assert_eq!(conversions::string_to_finish_reason("error"), Some(golem_llm::golem::llm::llm::FinishReason::Error));
        assert_eq!(conversions::string_to_finish_reason("unknown"), Some(golem_llm::golem::llm::llm::FinishReason::Other));
        
        println!("✅ Finish reason conversion test passed!");
    }
}

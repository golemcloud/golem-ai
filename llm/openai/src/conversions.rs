use crate::client::{
    ChatMessage, ContentPart as ClientContentPart, CreateChatCompletionRequest,
    CreateChatCompletionResponse, ImageUrl, MessageContent, Tool, ToolFunction,
};
use base64::{engine::general_purpose, Engine as _};
use golem_llm::error::error_code_from_status;
use golem_llm::golem::llm::llm::{
    ChatEvent, CompleteResponse, Config, ContentPart, Error, ErrorCode, FinishReason, ImageDetail,
    ImageReference, Message, ResponseMetadata, Role, ToolCall, ToolDefinition, ToolResult, Usage,
};
use reqwest::StatusCode;
use std::collections::HashMap;
use std::str::FromStr;

pub fn create_request(
    messages: Vec<ChatMessage>,
    config: Config,
    tools: Vec<Tool>,
) -> CreateChatCompletionRequest {
    let options = config
        .provider_options
        .into_iter()
        .map(|kv| (kv.key, kv.value))
        .collect::<HashMap<_, _>>();

    CreateChatCompletionRequest {
        messages,
        model: config.model,
        temperature: config.temperature,
        max_tokens: config.max_tokens,
        tools,
        tool_choice: config.tool_choice,
        stream: false,
        top_p: options
            .get("top_p")
            .and_then(|top_p_s| top_p_s.parse::<f32>().ok()),
        user: options
            .get("user")
            .and_then(|user_s| user_s.parse::<String>().ok()),
        stop: config.stop_sequences.unwrap_or_default(),
    }
}

pub fn messages_to_chat_messages(messages: Vec<Message>) -> Vec<ChatMessage> {
    let mut chat_messages = Vec::new();
    for message in messages {
        let role = to_openai_role_name(message.role).to_string();
        let content = if message.content.len() == 1 && matches!(message.content[0], ContentPart::Text(_)) {
            // Single text content
            match &message.content[0] {
                ContentPart::Text(text) => Some(MessageContent::Text(text.clone())),
                _ => unreachable!(),
            }
        } else {
            // Multiple content parts or images
            let parts: Vec<ClientContentPart> = message.content
                .into_iter()
                .map(content_part_to_client_content_part)
                .collect();
            Some(MessageContent::Array(parts))
        };

        chat_messages.push(ChatMessage {
            role,
            content,
            name: message.name,
            tool_calls: vec![], // Tool calls will be handled separately
        });
    }
    chat_messages
}

pub fn tool_results_to_chat_messages(tool_results: Vec<(ToolCall, ToolResult)>) -> Vec<ChatMessage> {
    let mut messages = Vec::new();
    
    // Group tool calls by their original assistant message
    let mut tool_calls_map = std::collections::HashMap::new();
    let mut tool_results_map = std::collections::HashMap::new();
    
    for (tool_call, tool_result) in tool_results {
        let call_id = tool_call.id.clone();
        tool_calls_map.insert(call_id.clone(), tool_call);
        tool_results_map.insert(call_id, tool_result);
    }
    
    // Create assistant message with tool calls
    if !tool_calls_map.is_empty() {
        let tool_calls: Vec<crate::client::ToolCall> = tool_calls_map
            .values()
            .map(|tc| crate::client::ToolCall {
                id: tc.id.clone(),
                tool_type: "function".to_string(),
                function: crate::client::FunctionCall {
                    name: tc.name.clone(),
                    arguments: tc.arguments_json.clone(),
                },
            })
            .collect();

        messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: None,
            name: None,
            tool_calls,
        });

        // Create tool result messages
        for (call_id, tool_result) in tool_results_map {
            let content = match tool_result {
                ToolResult::Success(success) => success.result_json,
                ToolResult::Error(error) => format!(
                    r#"{{"error": {{"code": {}, "message": "{}"}} }}"#,
                    error.error_code.unwrap_or_default(),
                    error.error_message
                ),
            };
            
            messages.push(ChatMessage {
                role: "tool".to_string(),
                content: Some(MessageContent::Text(content)),
                name: Some(call_id),
                tool_calls: vec![],
            });
        }
    }
    
    messages
}

pub fn tool_defs_to_tools(tool_definitions: &[ToolDefinition]) -> Result<Vec<Tool>, Error> {
    let mut tools = Vec::new();
    for tool_def in tool_definitions {
        match serde_json::from_str(&tool_def.parameters_schema) {
            Ok(value) => {
                let tool = Tool::Function {
                    function: ToolFunction {
                        name: tool_def.name.clone(),
                        description: tool_def.description.clone(),
                        parameters: Some(value),
                        strict: true,
                    },
                };
                tools.push(tool);
            }
            Err(error) => {
                Err(Error {
                    code: ErrorCode::InternalError,
                    message: format!(
                        "Failed to parse tool parameters for {}: {error}",
                        tool_def.name
                    ),
                    provider_error_json: None,
                })?;
            }
        }
    }
    Ok(tools)
}

pub fn to_openai_role_name(role: Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::System => "system",
        Role::Tool => "tool",
    }
}

pub fn content_part_to_client_content_part(content_part: ContentPart) -> ClientContentPart {
    match content_part {
        ContentPart::Text(msg) => ClientContentPart::Text { text: msg },
        ContentPart::Image(image_reference) => match image_reference {
            ImageReference::Url(image_url) => ClientContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: image_url.url,
                    detail: image_url.detail.map(|d| match d {
                        ImageDetail::Auto => "auto".to_string(),
                        ImageDetail::Low => "low".to_string(),
                        ImageDetail::High => "high".to_string(),
                    }),
                },
            },
            ImageReference::Inline(image_source) => {
                let base64_data = general_purpose::STANDARD.encode(&image_source.data);
                let mime_type = &image_source.mime_type;
                let data_url = format!("data:{};base64,{}", mime_type, base64_data);

                ClientContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: data_url,
                        detail: image_source.detail.map(|d| match d {
                            ImageDetail::Auto => "auto".to_string(),
                            ImageDetail::Low => "low".to_string(),
                            ImageDetail::High => "high".to_string(),
                        }),
                    },
                }
            }
        },
    }
}

pub fn parse_error_code(code: String) -> ErrorCode {
    if let Some(code) = <u16 as FromStr>::from_str(&code)
        .ok()
        .and_then(|code| StatusCode::from_u16(code).ok())
    {
        error_code_from_status(code)
    } else {
        ErrorCode::InternalError
    }
}

pub fn process_chat_completion(response: CreateChatCompletionResponse) -> ChatEvent {
    if response.choices.is_empty() {
        return ChatEvent::Error(Error {
            code: ErrorCode::InternalError,
            message: "No choices in response".to_string(),
            provider_error_json: None,
        });
    }

    let choice = &response.choices[0];
    let message = &choice.message;
    
    let mut contents = Vec::new();
    let mut tool_calls = Vec::new();

    // Extract content
    if let Some(content) = &message.content {
        match content {
            MessageContent::Text(text) => {
                contents.push(ContentPart::Text(text.clone()));
            }
            MessageContent::Array(parts) => {
                for part in parts {
                    match part {
                        ClientContentPart::Text { text } => {
                            contents.push(ContentPart::Text(text.clone()));
                        }
                        ClientContentPart::ImageUrl { image_url } => {
                            // Convert back to image reference if needed
                            // For now, just add as text description
                            contents.push(ContentPart::Text(format!("Image: {}", image_url.url)));
                        }
                    }
                }
            }
        }
    }

    // Extract tool calls
    for tool_call in &message.tool_calls {
        tool_calls.push(ToolCall {
            id: tool_call.id.clone(),
            name: tool_call.function.name.clone(),
            arguments_json: tool_call.function.arguments.clone(),
        });
    }

    let metadata = create_response_metadata(&response);

    if !tool_calls.is_empty() {
        ChatEvent::ToolRequest(tool_calls)
    } else {
        ChatEvent::Message(CompleteResponse {
            id: response.id,
            content: contents,
            tool_calls: vec![],
            metadata,
        })
    }
}

pub fn create_response_metadata(response: &CreateChatCompletionResponse) -> ResponseMetadata {
    ResponseMetadata {
        finish_reason: response.choices.first().and_then(|c| c.finish_reason.as_ref().and_then(|fr| string_to_finish_reason(fr))),
        usage: response.usage.as_ref().map(|usage| Usage {
            input_tokens: Some(usage.prompt_tokens),
            output_tokens: Some(usage.completion_tokens),
            total_tokens: Some(usage.total_tokens),
        }),
        provider_id: Some(response.id.clone()),
        timestamp: Some(response.created.to_string()),
        provider_metadata_json: response.system_fingerprint.as_ref().map(|sf| format!(r#"{{"system_fingerprint": "{}"}}"#, sf)),
    }
}

pub fn create_chunk_response_metadata(chunk: &crate::client::ChatCompletionChunk) -> ResponseMetadata {
    ResponseMetadata {
        finish_reason: chunk.choices.first().and_then(|c| c.finish_reason.as_ref().and_then(|fr| string_to_finish_reason(fr))),
        usage: None, // Usage is typically not available in streaming chunks
        provider_id: Some(chunk.id.clone()),
        timestamp: Some(chunk.created.to_string()),
        provider_metadata_json: chunk.system_fingerprint.as_ref().map(|sf| format!(r#"{{"system_fingerprint": "{}"}}"#, sf)),
    }
}

pub fn string_to_finish_reason(reason: &str) -> Option<FinishReason> {
    match reason {
        "stop" => Some(FinishReason::Stop),
        "length" => Some(FinishReason::Length),
        "tool_calls" => Some(FinishReason::ToolCalls),
        "content_filter" => Some(FinishReason::ContentFilter),
        "error" => Some(FinishReason::Error),
        _ => Some(FinishReason::Other),
    }
}

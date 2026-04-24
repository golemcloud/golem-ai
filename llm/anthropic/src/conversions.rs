use crate::client::{
    Content, ImageSource as ClientImageSource, MediaType, MessagesRequest, MessagesRequestMetadata,
    MessagesResponse, StopReason, Tool, ToolChoice,
};
use base64::{engine::general_purpose, Engine as _};
use golem_ai_llm::model::{
    Config, ContentPart, Error, ErrorCode, Event, FinishReason, ImageReference, ImageSource,
    ImageUrl, Kv, Response, ResponseMetadata, Role, ToolCall, ToolDefinition, ToolResult, Usage,
};
use std::collections::HashMap;

pub fn events_to_request(events: Vec<Event>, config: Config) -> Result<MessagesRequest, Error> {
    let provider_options = config.provider_options.clone();
    let options = provider_options_to_string_map(provider_options.clone());
    let mut additional_params = provider_options_to_json_map(provider_options);

    let (user_messages, system_messages) = events_to_messages_and_system_messages(events);

    let tool_choice = config.tool_choice.map(convert_tool_choice);
    let tools = config
        .tools
        .and_then(|tools| {
            (!tools.is_empty()).then(|| {
                tools
                    .into_iter()
                    .map(tool_definition_to_tool)
                    .collect::<Result<Vec<_>, _>>()
            })
        })
        .transpose()?;

    let metadata = options
        .get("user_id")
        .map(|user_id| MessagesRequestMetadata {
            user_id: Some(user_id.to_string()),
        });
    let stop_sequences = config.stop_sequences;
    let temperature = config.temperature;
    let top_k = options
        .get("top_k")
        .and_then(|top_k_s| top_k_s.parse::<u32>().ok());
    let top_p = options
        .get("top_p")
        .and_then(|top_p_s| top_p_s.parse::<f32>().ok());

    if metadata.is_some() {
        additional_params.remove("user_id");
    }
    if stop_sequences.is_some() {
        additional_params.remove("stop_sequences");
    }
    if temperature.is_some() {
        additional_params.remove("temperature");
    }
    if tool_choice.is_some() {
        additional_params.remove("tool_choice");
    }
    if tools.is_some() {
        additional_params.remove("tools");
    }
    if top_k.is_some() {
        additional_params.remove("top_k");
    }
    if top_p.is_some() {
        additional_params.remove("top_p");
    }

    additional_params.remove("max_tokens");
    additional_params.remove("messages");
    additional_params.remove("model");
    additional_params.remove("stream");
    additional_params.remove("system");

    Ok(MessagesRequest {
        max_tokens: config.max_tokens.unwrap_or(4096),
        messages: user_messages,
        model: config.model,
        metadata,
        stop_sequences,
        stream: false,
        system: system_messages,
        temperature,
        tool_choice,
        tools,
        top_k,
        top_p,
        additional_params,
    })
}

fn events_to_messages_and_system_messages(
    events: Vec<Event>,
) -> (Vec<crate::client::Message>, Vec<Content>) {
    let mut messages: Vec<crate::client::Message> = vec![];
    let mut system_messages: Vec<Content> = vec![];

    for event in events {
        match event {
            Event::Message(message) if message.role == Role::System => {
                system_messages.extend(content_parts_to_content(message.content));
            }
            Event::Message(message) => messages.push(crate::client::Message {
                role: match &message.role {
                    Role::User => crate::client::Role::User,
                    Role::Assistant => crate::client::Role::Assistant,
                    Role::Tool => crate::client::Role::User,
                    Role::System => unreachable!(),
                },
                content: content_parts_to_content(message.content),
            }),
            Event::Response(response) => {
                if !response.content.is_empty() {
                    messages.push(crate::client::Message {
                        role: crate::client::Role::Assistant,
                        content: content_parts_to_content(response.content),
                    })
                }
                if !response.tool_calls.is_empty() {
                    messages.push(crate::client::Message {
                        role: crate::client::Role::Assistant,
                        content: response
                            .tool_calls
                            .into_iter()
                            .map(tool_call_to_conent)
                            .collect(),
                    })
                }
            }
            Event::ToolResults(tool_results) => {
                messages.extend(tool_results.into_iter().map(tool_result_to_message))
            }
        }
    }

    (messages, system_messages)
}

fn convert_tool_choice(tool_name: String) -> ToolChoice {
    if &tool_name == "auto" {
        ToolChoice::Auto {
            disable_parallel_tool_use: None,
        }
    } else if &tool_name == "none" {
        ToolChoice::None {}
    } else if &tool_name == "any" {
        ToolChoice::Any {
            disable_parallel_tool_use: None,
        }
    } else {
        ToolChoice::Tool {
            name: tool_name,
            disable_parallel_tool_use: None,
        }
    }
}

pub fn process_response(response: MessagesResponse) -> Result<Response, Error> {
    let mut contents = Vec::new();
    let mut tool_calls = Vec::new();

    for content in response.content {
        match content {
            Content::Text { text, .. } => contents.push(ContentPart::Text(text)),
            Content::Image { source, .. } => match source {
                ClientImageSource::Url { url } => {
                    contents.push(ContentPart::Image(ImageReference::Url(ImageUrl {
                        url,
                        detail: None,
                    })))
                }
                ClientImageSource::Base64 { data, media_type } => {
                    match general_purpose::STANDARD.decode(data) {
                        Ok(decoded_data) => {
                            let mime_type_str = match media_type {
                                MediaType::Jpeg => "image/jpeg".to_string(),
                                MediaType::Png => "image/png".to_string(),
                                MediaType::Gif => "image/gif".to_string(),
                                MediaType::Webp => "image/webp".to_string(),
                            };
                            contents.push(ContentPart::Image(ImageReference::Inline(
                                ImageSource {
                                    data: decoded_data,
                                    mime_type: mime_type_str,
                                    detail: None,
                                },
                            )));
                        }
                        Err(e) => {
                            return Err(Error {
                                code: ErrorCode::InvalidRequest,
                                message: format!("Failed to decode base64 image data: {e}"),
                                provider_error_json: None,
                            });
                        }
                    }
                }
            },
            Content::ToolUse {
                id, input, name, ..
            } => tool_calls.push(ToolCall {
                id,
                name,
                arguments_json: serde_json::to_string(&input).unwrap(),
            }),
            Content::ToolResult { .. } => {}
        }
    }

    let metadata = ResponseMetadata {
        finish_reason: response.stop_reason.map(stop_reason_to_finish_reason),
        usage: Some(convert_usage(response.usage)),
        provider_id: None,
        timestamp: None,
        provider_metadata_json: None,
    };

    Ok(Response {
        id: response.id,
        content: contents,
        tool_calls,
        metadata,
    })
}

pub fn tool_call_to_conent(tool_call: ToolCall) -> Content {
    Content::ToolUse {
        id: tool_call.id.clone(),
        input: serde_json::from_str(&tool_call.arguments_json).unwrap(),
        name: tool_call.name,
        cache_control: None,
    }
}

pub fn tool_result_to_message(tool_result: ToolResult) -> crate::client::Message {
    crate::client::Message {
        content: vec![match tool_result {
            ToolResult::Success(success) => Content::ToolResult {
                tool_use_id: success.id,
                cache_control: None,
                content: vec![Content::Text {
                    text: success.result_json,
                    cache_control: None,
                }],
                is_error: false,
            },
            ToolResult::Error(error) => Content::ToolResult {
                tool_use_id: error.id,
                cache_control: None,
                content: vec![Content::Text {
                    text: error.error_message,
                    cache_control: None,
                }],
                is_error: true,
            },
        }],
        role: crate::client::Role::User,
    }
}

pub fn stop_reason_to_finish_reason(stop_reason: StopReason) -> FinishReason {
    match stop_reason {
        StopReason::EndTurn => FinishReason::Other,
        StopReason::MaxTokens => FinishReason::Length,
        StopReason::StopSequence => FinishReason::Stop,
        StopReason::ToolUse => FinishReason::ToolCalls,
    }
}

pub fn convert_usage(usage: crate::client::Usage) -> Usage {
    Usage {
        input_tokens: Some(usage.input_tokens),
        output_tokens: Some(usage.output_tokens),
        total_tokens: None,
    }
}

fn content_parts_to_content(content_parts: Vec<ContentPart>) -> Vec<Content> {
    let mut result = Vec::new();

    for content_part in content_parts {
        match content_part {
            ContentPart::Text(text) => result.push(Content::Text {
                text: text.clone(),
                cache_control: None,
            }),
            ContentPart::Image(image_reference) => match image_reference {
                ImageReference::Url(image_url) => result.push(Content::Image {
                    source: ClientImageSource::Url {
                        url: image_url.url.clone(),
                    },
                    cache_control: None,
                }),
                ImageReference::Inline(image_source) => {
                    let base64_data = general_purpose::STANDARD.encode(&image_source.data);
                    let media_type = match image_source.mime_type.as_str() {
                        "image/jpeg" => MediaType::Jpeg,
                        "image/png" => MediaType::Png,
                        "image/gif" => MediaType::Gif,
                        "image/webp" => MediaType::Webp,
                        _ => MediaType::Jpeg,
                    };

                    result.push(Content::Image {
                        source: ClientImageSource::Base64 {
                            data: base64_data,
                            media_type,
                        },
                        cache_control: None,
                    });
                }
            },
        }
    }

    result
}

fn tool_definition_to_tool(tool: ToolDefinition) -> Result<Tool, Error> {
    match serde_json::from_str(&tool.parameters_schema) {
        Ok(value) => Ok(Tool::CustomTool {
            input_schema: value,
            name: tool.name,
            cache_control: None,
            description: tool.description,
        }),
        Err(error) => Err(Error {
            code: ErrorCode::InternalError,
            message: format!("Failed to parse tool parameters for {}: {error}", tool.name),
            provider_error_json: None,
        }),
    }
}

fn provider_options_to_string_map(provider_options: Option<Vec<Kv>>) -> HashMap<String, String> {
    provider_options
        .unwrap_or_default()
        .into_iter()
        .map(|kv| (kv.key, kv.value))
        .collect::<HashMap<_, _>>()
}

fn provider_options_to_json_map(
    provider_options: Option<Vec<Kv>>,
) -> HashMap<String, serde_json::Value> {
    provider_options
        .unwrap_or_default()
        .into_iter()
        .map(|kv| (kv.key, parse_provider_option_value(&kv.value)))
        .collect::<HashMap<_, _>>()
}

fn parse_provider_option_value(value: &str) -> serde_json::Value {
    serde_json::from_str(value).unwrap_or_else(|_| serde_json::Value::String(value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use golem_ai_llm::model::{Config, Kv};

    fn kv(key: &str, value: &str) -> Kv {
        Kv {
            key: key.to_string(),
            value: value.to_string(),
        }
    }

    fn base_config(provider_options: Option<Vec<Kv>>) -> Config {
        Config {
            model: "claude-test".to_string(),
            temperature: Some(0.5),
            max_tokens: Some(128),
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            provider_options,
        }
    }

    #[test]
    fn forwards_unmapped_provider_options() {
        let request = events_to_request(
            Vec::new(),
            base_config(Some(vec![
                kv("user_id", "u1"),
                kv("top_k", "50"),
                kv("custom_mode", "\"json\""),
            ])),
        )
        .unwrap();

        assert_eq!(
            request.metadata.and_then(|metadata| metadata.user_id),
            Some("u1".to_string())
        );
        assert_eq!(request.top_k, Some(50));
        assert_eq!(
            request.additional_params.get("custom_mode"),
            Some(&serde_json::json!("json"))
        );
        assert!(!request.additional_params.contains_key("user_id"));
        assert!(!request.additional_params.contains_key("top_k"));
    }

    #[test]
    fn keeps_known_option_when_typed_parse_fails() {
        let request =
            events_to_request(Vec::new(), base_config(Some(vec![kv("top_p", "adaptive")])))
                .unwrap();

        assert_eq!(request.top_p, None);
        assert_eq!(
            request.additional_params.get("top_p"),
            Some(&serde_json::json!("adaptive"))
        );
    }
}

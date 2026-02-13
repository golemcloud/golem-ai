use crate::client::{CompletionsRequest, CompletionsResponse, Detail, Effort};
use base64::{engine::general_purpose, Engine as _};
use golem_ai_llm::model::{
    Config, ContentPart, Error, ErrorCode, Event, FinishReason, ImageDetail, ImageReference, Kv,
    Response, ResponseMetadata, Role, ToolCall, ToolDefinition, ToolResult, Usage,
};
use std::collections::HashMap;

pub fn events_to_request(events: Vec<Event>, config: Config) -> Result<CompletionsRequest, Error> {
    let provider_options = config.provider_options.clone();
    let options = provider_options_to_string_map(provider_options.clone());
    let mut additional_params = provider_options_to_json_map(provider_options);

    let mut completion_messages = Vec::new();
    for event in events {
        match event {
            Event::Message(message) => match message.role {
                Role::User => completion_messages.push(crate::client::Message::User {
                    name: message.name,
                    content: convert_content_parts_to_client_content(message.content),
                }),
                Role::Assistant => completion_messages.push(crate::client::Message::Assistant {
                    name: message.name,
                    content: Some(convert_content_parts_to_client_content(message.content)),
                    tool_calls: None,
                }),
                Role::System => completion_messages.push(crate::client::Message::System {
                    name: message.name,
                    content: convert_content_parts_to_client_content(message.content),
                }),
                Role::Tool => completion_messages.push(crate::client::Message::Tool {
                    name: message.name,
                    content: convert_content_parts_to_client_content(message.content),
                    tool_call_id: None,
                }),
            },
            Event::ToolResults(tool_results) => {
                if !tool_results.is_empty() {
                    completion_messages.extend(tool_results.into_iter().map(tool_result_to_message))
                }
            }
            Event::Response(response) => {
                if !response.content.is_empty() || !response.tool_calls.is_empty() {
                    completion_messages.push(crate::client::Message::Assistant {
                        content: (!response.content.is_empty())
                            .then(|| convert_content_parts_to_client_content(response.content)),
                        name: None,
                        tool_calls: (!response.tool_calls.is_empty()).then(|| {
                            response
                                .tool_calls
                                .into_iter()
                                .map(convert_tool_call_to_client_tool_call)
                                .collect::<Vec<_>>()
                        }),
                    })
                }
            }
        }
    }

    let mut tools = Vec::new();
    for tool in config.tools.unwrap_or_default() {
        tools.push(tool_definition_to_tool(tool)?)
    }

    let frequency_penalty = options
        .get("frequency_penalty")
        .and_then(|fp_s| fp_s.parse::<f32>().ok());
    let n = options.get("n").and_then(|n_s| n_s.parse::<u32>().ok());
    let presence_penalty = options
        .get("presence_penalty")
        .and_then(|pp_s| pp_s.parse::<f32>().ok());
    let reasoning_effort = options
        .get("reasoning_effort")
        .and_then(|effort_s| effort_s.parse::<Effort>().ok());
    let seed = options
        .get("seed")
        .and_then(|seed_s| seed_s.parse::<u32>().ok());
    let stop = config.stop_sequences;
    let temperature = config.temperature;
    let tool_choice = config.tool_choice;
    let top_logprobs = options
        .get("top_logprobs")
        .and_then(|top_logprobs_s| top_logprobs_s.parse::<u8>().ok());
    let top_p = options
        .get("top_p")
        .and_then(|top_p_s| top_p_s.parse::<f32>().ok());
    let user = options.get("user_id").cloned();

    if frequency_penalty.is_some() {
        additional_params.remove("frequency_penalty");
    }
    if n.is_some() {
        additional_params.remove("n");
    }
    if presence_penalty.is_some() {
        additional_params.remove("presence_penalty");
    }
    if reasoning_effort.is_some() {
        additional_params.remove("reasoning_effort");
    }
    if seed.is_some() {
        additional_params.remove("seed");
    }
    if stop.is_some() {
        additional_params.remove("stop");
    }
    if temperature.is_some() {
        additional_params.remove("temperature");
    }
    if tool_choice.is_some() {
        additional_params.remove("tool_choice");
    }
    if top_logprobs.is_some() {
        additional_params.remove("top_logprobs");
    }
    if top_p.is_some() {
        additional_params.remove("top_p");
    }
    if user.is_some() {
        additional_params.remove("user_id");
    }
    if !tools.is_empty() {
        additional_params.remove("tools");
    }
    if config.max_tokens.is_some() {
        additional_params.remove("max_completion_tokens");
    }

    additional_params.remove("messages");
    additional_params.remove("model");
    additional_params.remove("stream");
    additional_params.remove("stream_options");

    Ok(CompletionsRequest {
        messages: completion_messages,
        model: config.model,
        frequency_penalty,
        max_completion_tokens: config.max_tokens,
        n,
        presence_penalty,
        reasoning_effort,
        seed,
        stop,
        stream: Some(false),
        stream_options: None,
        temperature,
        tool_choice,
        tools,
        top_logprobs,
        top_p,
        user,
        additional_params,
    })
}

pub fn process_response(mut response: CompletionsResponse) -> Result<Response, Error> {
    let choice = response.choices.pop();
    match choice {
        Some(choice) => {
            let content = choice
                .message
                .content
                .into_iter()
                .map(ContentPart::Text)
                .collect();

            let tool_calls = choice
                .message
                .tool_calls
                .map(|tool_calls| {
                    tool_calls
                        .into_iter()
                        .map(convert_client_tool_call_to_tool_call)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let metadata = ResponseMetadata {
                finish_reason: choice.finish_reason.as_ref().map(convert_finish_reason),
                usage: response.usage.as_ref().map(convert_usage),
                provider_id: None,
                timestamp: Some(response.created.to_string()),
                provider_metadata_json: None,
            };

            Ok(Response {
                id: response.id,
                content,
                tool_calls,
                metadata,
            })
        }
        None => Err(Error {
            code: ErrorCode::InternalError,
            message: "No choices in response".to_string(),
            provider_error_json: None,
        }),
    }
}

pub fn tool_result_to_message(tool_result: ToolResult) -> crate::client::Message {
    match tool_result {
        ToolResult::Success(success) => crate::client::Message::Tool {
            name: None,
            content: crate::client::Content::List(vec![crate::client::ContentPart::TextInput {
                text: success.result_json,
            }]),
            tool_call_id: Some(success.id),
        },
        ToolResult::Error(failure) => crate::client::Message::Tool {
            name: None,
            content: crate::client::Content::List(vec![crate::client::ContentPart::TextInput {
                text: failure.error_message,
            }]),
            tool_call_id: Some(failure.id),
        },
    }
}

pub fn convert_client_tool_call_to_tool_call(tool_call: crate::client::ToolCall) -> ToolCall {
    match tool_call {
        crate::client::ToolCall::Function { function, id, .. } => ToolCall {
            id,
            name: function.name,
            arguments_json: function.arguments,
        },
    }
}

pub fn convert_tool_call_to_client_tool_call(tool_call: ToolCall) -> crate::client::ToolCall {
    crate::client::ToolCall::Function {
        id: tool_call.id,
        function: crate::client::FunctionCall {
            name: tool_call.name,
            arguments: tool_call.arguments_json,
        },
        index: None,
    }
}

fn convert_content_parts_to_client_content(contents: Vec<ContentPart>) -> crate::client::Content {
    let mut result = Vec::new();
    for content in contents {
        match content {
            ContentPart::Text(text) => result.push(crate::client::ContentPart::TextInput { text }),
            ContentPart::Image(image_reference) => match image_reference {
                ImageReference::Url(image_url) => {
                    result.push(crate::client::ContentPart::ImageInput {
                        image_url: crate::client::ImageUrl {
                            url: image_url.url,
                            detail: image_url.detail.map(|d| d.into()),
                        },
                    })
                }
                ImageReference::Inline(image_source) => {
                    let base64_data = general_purpose::STANDARD.encode(&image_source.data);
                    let media_type = &image_source.mime_type; // This is already a string
                    result.push(crate::client::ContentPart::ImageInput {
                        image_url: crate::client::ImageUrl {
                            url: format!("data:{media_type};base64,{base64_data}"),
                            detail: image_source.detail.map(|d| d.into()),
                        },
                    });
                }
            },
        }
    }
    crate::client::Content::List(result)
}

impl From<ImageDetail> for Detail {
    fn from(value: ImageDetail) -> Self {
        match value {
            ImageDetail::Auto => Self::Auto,
            ImageDetail::Low => Self::Low,
            ImageDetail::High => Self::High,
        }
    }
}

pub fn convert_finish_reason(value: &crate::client::FinishReason) -> FinishReason {
    match value {
        crate::client::FinishReason::Stop => FinishReason::Stop,
        crate::client::FinishReason::Length => FinishReason::Length,
        crate::client::FinishReason::EndTurn => FinishReason::Other,
        crate::client::FinishReason::ToolCalls => FinishReason::ToolCalls,
    }
}

pub fn convert_usage(value: &crate::client::Usage) -> Usage {
    Usage {
        input_tokens: Some(value.prompt_tokens),
        output_tokens: Some(value.completion_tokens),
        total_tokens: Some(value.total_tokens),
    }
}

fn tool_definition_to_tool(tool: ToolDefinition) -> Result<crate::client::Tool, Error> {
    match serde_json::from_str(&tool.parameters_schema) {
        Ok(value) => Ok(crate::client::Tool::Function {
            function: crate::client::Function {
                name: tool.name,
                description: tool.description,
                parameters: Some(value),
            },
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
            model: "grok-test".to_string(),
            temperature: Some(0.1),
            max_tokens: Some(48),
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
            base_config(Some(vec![kv("user_id", "alice"), kv("custom_depth", "3")])),
        )
        .unwrap();

        assert_eq!(request.user, Some("alice".to_string()));
        assert!(!request.additional_params.contains_key("user_id"));
        assert_eq!(
            request.additional_params.get("custom_depth"),
            Some(&serde_json::json!(3))
        );
    }

    #[test]
    fn keeps_option_when_typed_parse_fails() {
        let request = events_to_request(
            Vec::new(),
            base_config(Some(vec![kv("top_logprobs", "many")])),
        )
        .unwrap();

        assert_eq!(request.top_logprobs, None);
        assert_eq!(
            request.additional_params.get("top_logprobs"),
            Some(&serde_json::json!("many"))
        );
    }
}

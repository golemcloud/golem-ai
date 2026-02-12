use crate::client::{
    CompletionsRequest, CompletionsResponse, Detail, FunctionName, ToolChoiceFunction,
};
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
                    content: convert_content_parts(message.content),
                }),
                Role::Assistant => completion_messages.push(crate::client::Message::Assistant {
                    name: message.name,
                    content: Some(convert_content_parts(message.content)),
                    tool_calls: None,
                }),
                Role::System => completion_messages.push(crate::client::Message::System {
                    name: message.name,
                    content: convert_content_parts(message.content),
                }),
                Role::Tool => completion_messages.push(crate::client::Message::Tool {
                    name: message.name,
                    content: convert_content_parts_to_string(message.content),
                    tool_call_id: "unknown".to_string(),
                }),
            },
            Event::Response(response) => {
                completion_messages.push(crate::client::Message::Assistant {
                    name: None,
                    content: {
                        (!response.content.is_empty())
                            .then(|| convert_content_parts(response.content))
                    },
                    tool_calls: (!response.tool_calls.is_empty()).then(|| {
                        response
                            .tool_calls
                            .into_iter()
                            .map(tool_call_to_client_tool_call)
                            .collect()
                    }),
                });
            }
            Event::ToolResults(tool_results) => {
                completion_messages.extend(tool_results.into_iter().map(tool_result_to_message))
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
    let presence_penalty = options
        .get("presence_penalty")
        .and_then(|pp_s| pp_s.parse::<f32>().ok());
    let repetition_penalty = options
        .get("repetition_penalty")
        .and_then(|rp_s| rp_s.parse::<f32>().ok());
    let seed = options
        .get("seed")
        .and_then(|seed_s| seed_s.parse::<u32>().ok());
    let stop = config.stop_sequences;
    let temperature = config.temperature;
    let tool_choice = config.tool_choice.map(convert_tool_choice);
    let top_p = options
        .get("top_p")
        .and_then(|top_p_s| top_p_s.parse::<f32>().ok());
    let top_k = options
        .get("top_k")
        .and_then(|top_k_s| top_k_s.parse::<f32>().ok());
    let min_p = options
        .get("min_p")
        .and_then(|min_p_s| min_p_s.parse::<f32>().ok());
    let top_a = options
        .get("top_a")
        .and_then(|top_a_s| top_a_s.parse::<f32>().ok());

    if frequency_penalty.is_some() {
        additional_params.remove("frequency_penalty");
    }
    if presence_penalty.is_some() {
        additional_params.remove("presence_penalty");
    }
    if repetition_penalty.is_some() {
        additional_params.remove("repetition_penalty");
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
    if top_p.is_some() {
        additional_params.remove("top_p");
    }
    if top_k.is_some() {
        additional_params.remove("top_k");
    }
    if min_p.is_some() {
        additional_params.remove("min_p");
    }
    if top_a.is_some() {
        additional_params.remove("top_a");
    }
    if !tools.is_empty() {
        additional_params.remove("tools");
    }
    if config.max_tokens.is_some() {
        additional_params.remove("max_tokens");
    }

    additional_params.remove("messages");
    additional_params.remove("model");
    additional_params.remove("stream");

    Ok(CompletionsRequest {
        messages: completion_messages,
        model: config.model,
        frequency_penalty,
        max_tokens: config.max_tokens,
        presence_penalty,
        repetition_penalty,
        seed,
        stop,
        stream: Some(false),
        temperature,
        tool_choice,
        tools,
        top_p,
        top_k,
        min_p,
        top_a,
        additional_params,
    })
}

pub fn process_response(response: CompletionsResponse) -> Result<Response, Error> {
    let choice = response.choices.first();
    if let Some(choice) = choice {
        let mut contents = Vec::new();
        let mut tool_calls = Vec::new();

        if let Some(content) = &choice.message.content {
            contents.push(ContentPart::Text(content.clone()));
        }

        let empty = Vec::new();
        for tool_call in choice.message.tool_calls.as_ref().unwrap_or(&empty) {
            tool_calls.push(convert_tool_call(tool_call));
        }

        let metadata = ResponseMetadata {
            finish_reason: choice.finish_reason.as_ref().map(convert_finish_reason),
            usage: response.usage.as_ref().map(convert_usage),
            provider_id: None,
            timestamp: Some(response.created.to_string()),
            provider_metadata_json: None,
        };

        Ok(Response {
            id: response.id,
            content: contents,
            tool_calls,
            metadata,
        })
    } else {
        Err(Error {
            code: ErrorCode::InternalError,
            message: "No choices in response".to_string(),
            provider_error_json: None,
        })
    }
}

pub fn tool_call_to_client_tool_call(tool_call: ToolCall) -> crate::client::ToolCall {
    crate::client::ToolCall::Function {
        function: crate::client::FunctionCall {
            arguments: tool_call.arguments_json,
            name: Some(tool_call.name),
        },
        id: Some(tool_call.id.clone()),
        index: None,
    }
}

pub fn tool_result_to_message(tool_result: ToolResult) -> crate::client::Message {
    match tool_result {
        ToolResult::Success(success) => crate::client::Message::Tool {
            name: None,
            content: success.result_json,
            tool_call_id: success.id,
        },
        ToolResult::Error(error) => crate::client::Message::Tool {
            name: None,
            content: error.error_message,
            tool_call_id: error.id,
        },
    }
}

pub fn convert_tool_call(tool_call: &crate::client::ToolCall) -> ToolCall {
    match tool_call {
        crate::client::ToolCall::Function { function, id, .. } => ToolCall {
            id: id.clone().unwrap_or_default(),
            name: function.name.clone().unwrap_or_default(),
            arguments_json: function.arguments.clone(),
        },
    }
}

fn convert_content_parts(contents: Vec<ContentPart>) -> crate::client::Content {
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

fn convert_content_parts_to_string(contents: Vec<ContentPart>) -> String {
    let mut result = String::new();
    for content in contents {
        match content {
            ContentPart::Text(text) => result.push_str(&text),
            ContentPart::Image(_) => {} // Correctly ignores any image content
        }
    }
    result
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
        crate::client::FinishReason::ContentFilter => FinishReason::ContentFilter,
        crate::client::FinishReason::ToolCalls => FinishReason::ToolCalls,
        crate::client::FinishReason::Error => FinishReason::Error,
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
                parameters: value,
            },
        }),
        Err(error) => Err(Error {
            code: ErrorCode::InternalError,
            message: format!("Failed to parse tool parameters for {}: {error}", tool.name),
            provider_error_json: None,
        }),
    }
}

fn convert_tool_choice(tool_choice: String) -> crate::client::ToolChoice {
    match tool_choice.as_str() {
        "auto" | "none" => crate::client::ToolChoice::String(tool_choice),
        _ => crate::client::ToolChoice::Function(ToolChoiceFunction::Function {
            function: FunctionName { name: tool_choice },
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
            model: "openrouter-test".to_string(),
            temperature: Some(0.2),
            max_tokens: Some(64),
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            provider_options,
        }
    }

    #[test]
    fn includes_unmapped_options_in_passthrough_map() {
        let request = events_to_request(
            Vec::new(),
            base_config(Some(vec![
                kv("frequency_penalty", "0.1"),
                kv("experimental", "{\"reasoning\":true}"),
            ])),
        )
        .unwrap();

        assert_eq!(request.frequency_penalty, Some(0.1));
        assert!(!request.additional_params.contains_key("frequency_penalty"));
        assert_eq!(
            request.additional_params.get("experimental"),
            Some(&serde_json::json!({"reasoning": true}))
        );
    }

    #[test]
    fn keeps_failed_typed_parsing_as_passthrough() {
        let request =
            events_to_request(Vec::new(), base_config(Some(vec![kv("top_p", "rapid")]))).unwrap();

        assert_eq!(request.top_p, None);
        assert_eq!(
            request.additional_params.get("top_p"),
            Some(&serde_json::json!("rapid"))
        );
    }
}

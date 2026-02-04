use crate::client::{
    CreateModelResponseRequest, CreateModelResponseResponse, Detail, InnerInput, InnerInputItem,
    Input, InputItem, OpenOutputItem, OutputItem, OutputMessageContent, Tool,
};
use base64::{engine::general_purpose, Engine as _};
use golem_llm::error::error_code_from_status;
use golem_llm::golem::llm::llm::{
    Config, ContentPart, Error, ErrorCode, Event, ImageDetail, ImageReference, Message, Response,
    ResponseMetadata, Role, ToolCall, ToolDefinition, ToolResult, Usage,
};
use golem_wasi_http::StatusCode;
use log::trace;
use std::collections::HashMap;
use std::str::FromStr;

pub fn create_request(
    items: Vec<InputItem>,
    config: Config,
    tools: Vec<Tool>,
) -> CreateModelResponseRequest {
    let options = config
        .provider_options
        .map(|options| {
            options
                .into_iter()
                .map(|kv| (kv.key, kv.value))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();

    CreateModelResponseRequest {
        input: Input::List(items),
        model: config.model,
        temperature: config.temperature,
        max_output_tokens: config.max_tokens,
        tools,
        tool_choice: config.tool_choice,
        stream: false,
        top_p: options
            .get("top_p")
            .and_then(|top_p_s| top_p_s.parse::<f32>().ok()),
        user: options
            .get("user")
            .and_then(|user_s| user_s.parse::<String>().ok()),
    }
}

pub fn events_to_input_items(events: Vec<Event>) -> Vec<InputItem> {
    let mut items = Vec::new();
    for event in events {
        match event {
            Event::Message(message) => items.push(llm_message_to_openai_input_item(message)),
            Event::Response(response) => items.extend(response_to_openai_input_items(response)),
            Event::ToolResults(tool_results) => {
                items.extend(tool_results.into_iter().map(tool_result_to_input_item))
            }
        }
    }
    items
}

pub fn tool_call_to_input_item(tool_call: ToolCall) -> InputItem {
    InputItem::ToolCall {
        arguments: tool_call.arguments_json,
        call_id: tool_call.id,
        name: tool_call.name,
    }
}

pub fn tool_result_to_input_item(tool_result: ToolResult) -> InputItem {
    match tool_result {
        ToolResult::Success(success) => InputItem::ToolResult {
            call_id: success.id,
            output: format!(r#"{{ "success": {} }}"#, success.result_json),
        },
        ToolResult::Error(error) => InputItem::ToolResult {
            call_id: error.id,
            output: format!(
                r#"{{ "error": {{ "code": {}, "message": {} }} }}"#,
                error.error_code.unwrap_or_default(),
                error.error_message
            ),
        },
    }
}

pub fn tool_defs_to_tools(
    tool_definitions: Option<Vec<ToolDefinition>>,
) -> Result<Vec<Tool>, Error> {
    let mut tools = Vec::new();
    for tool_def in tool_definitions.unwrap_or_default() {
        match serde_json::from_str(&tool_def.parameters_schema) {
            Ok(value) => {
                let tool = Tool::Function {
                    name: tool_def.name.clone(),
                    description: tool_def.description.clone(),
                    parameters: Some(value),
                    strict: true,
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

pub fn to_openai_role_name(role: &Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::System => "system",
        Role::Tool => "tool",
    }
}

pub fn content_part_to_inner_input_item(role: &Role, content_part: ContentPart) -> InnerInputItem {
    fn convert_image_detail(detail: Option<ImageDetail>) -> Detail {
        match detail {
            Some(ImageDetail::Auto) => Detail::Auto,
            Some(ImageDetail::Low) => Detail::Low,
            Some(ImageDetail::High) => Detail::High,
            None => Detail::default(),
        }
    }

    match content_part {
        ContentPart::Text(msg) => match role {
            Role::Assistant => InnerInputItem::TextOutput { text: msg },
            _ => InnerInputItem::TextInput { text: msg },
        },
        ContentPart::Image(image_reference) => match image_reference {
            ImageReference::Url(image_url) => InnerInputItem::ImageInput {
                image_url: image_url.url,
                detail: convert_image_detail(image_url.detail),
            },
            ImageReference::Inline(image_source) => {
                let base64_data = general_purpose::STANDARD.encode(&image_source.data);
                let mime_type = &image_source.mime_type; // This is already a string
                let data_url = format!("data:{mime_type};base64,{base64_data}");

                InnerInputItem::ImageInput {
                    image_url: data_url,
                    detail: convert_image_detail(image_source.detail),
                }
            }
        },
    }
}

pub fn llm_message_to_openai_input_item(message: Message) -> InputItem {
    let role = message.role;
    InputItem::InputMessage {
        role: to_openai_role_name(&role).to_string(),
        content: InnerInput::List(
            message
                .content
                .into_iter()
                .map(|part| content_part_to_inner_input_item(&role, part))
                .collect(),
        ),
    }
}

pub fn response_to_openai_input_items(message: Response) -> Vec<InputItem> {
    let mut items = Vec::new();

    let role = Role::Assistant;

    if !message.content.is_empty() {
        items.push(InputItem::InputMessage {
            role: to_openai_role_name(&role).to_string(),
            content: InnerInput::List(
                message
                    .content
                    .into_iter()
                    .map(|part| content_part_to_inner_input_item(&role, part))
                    .collect(),
            ),
        })
    }

    if !message.tool_calls.is_empty() {
        items.extend(message.tool_calls.into_iter().map(tool_call_to_input_item))
    }

    items
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

pub fn process_model_response(response: CreateModelResponseResponse) -> Result<Response, Error> {
    if let Some(error) = response.error {
        Err(Error {
            code: parse_error_code(error.code),
            message: error.message,
            provider_error_json: None,
        })
    } else {
        let mut contents = Vec::new();
        let mut tool_calls = Vec::new();

        let metadata = create_response_metadata(&response);

        for output_item in response.output {
            match output_item {
                OpenOutputItem::Known(output_item) => match output_item {
                    OutputItem::Message { content, .. } => {
                        for content in content {
                            match content {
                                OutputMessageContent::Text { text, .. } => {
                                    contents.push(ContentPart::Text(text));
                                }
                                OutputMessageContent::Refusal { refusal, .. } => {
                                    contents.push(ContentPart::Text(format!("Refusal: {refusal}")));
                                }
                            }
                        }
                    }
                    OutputItem::ToolCall {
                        arguments,
                        call_id,
                        name,
                        ..
                    } => {
                        let tool_call = ToolCall {
                            id: call_id,
                            name,
                            arguments_json: arguments,
                        };
                        tool_calls.push(tool_call);
                    }
                },
                OpenOutputItem::Other(value) => {
                    trace!("Ignoring unknown output item: {value:?}");
                }
            }
        }

        Ok(Response {
            id: response.id,
            content: contents,
            tool_calls,
            metadata,
        })
    }
}

pub fn create_response_metadata(response: &CreateModelResponseResponse) -> ResponseMetadata {
    ResponseMetadata {
        finish_reason: None,
        usage: response.usage.as_ref().map(|usage| Usage {
            input_tokens: Some(usage.input_tokens),
            output_tokens: Some(usage.output_tokens),
            total_tokens: Some(usage.total_tokens),
        }),
        provider_id: Some(response.id.clone()),
        timestamp: Some(response.created_at.to_string()),
        provider_metadata_json: response.metadata.as_ref().map(|m| m.to_string()),
    }
}

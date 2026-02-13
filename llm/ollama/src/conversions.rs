use crate::client::MessageRole::Assistant;
use crate::client::{
    image_to_base64, CompletionsRequest, CompletionsResponse, FunctionTool, MessageRequest,
    MessageRole, OllamaModelOptions, Tool,
};
use base64::{engine::general_purpose, Engine};
use golem_ai_llm::model::{
    Config, ContentPart, Error, ErrorCode, Event, FinishReason, ImageReference, Kv, Message,
    Response, ResponseMetadata, Role, ToolCall as GolemToolCall, ToolResult, Usage,
};
use log::trace;
use std::collections::HashMap;

pub fn events_to_request(events: Vec<Event>, config: Config) -> Result<CompletionsRequest, Error> {
    let provider_options = config.provider_options.clone();
    let options = provider_options_to_string_map(provider_options.clone());
    let mut option_additional_params = provider_options_to_json_map(provider_options);
    let mut request_additional_params = HashMap::new();

    if let Some(serde_json::Value::Object(object)) = option_additional_params.remove("options") {
        for (key, value) in object {
            option_additional_params.insert(key, value);
        }
    }

    let option_prefixed_keys = option_additional_params
        .keys()
        .filter(|key| key.starts_with("options."))
        .cloned()
        .collect::<Vec<_>>();
    for key in option_prefixed_keys {
        if let Some(value) = option_additional_params.remove(&key) {
            option_additional_params.insert(key.trim_start_matches("options.").to_string(), value);
        }
    }

    let request_prefixed_keys = option_additional_params
        .keys()
        .filter(|key| key.starts_with("request."))
        .cloned()
        .collect::<Vec<_>>();
    for key in request_prefixed_keys {
        if let Some(value) = option_additional_params.remove(&key) {
            request_additional_params.insert(key.trim_start_matches("request.").to_string(), value);
        }
    }

    let mut request_messages = Vec::new();

    for event in events {
        match event {
            Event::Message(message) => request_messages.push(message_to_request(message)),
            Event::Response(response) => request_messages.push(response_to_request(response)),
            Event::ToolResults(tool_results) => {
                request_messages.extend(tool_results.into_iter().map(tool_result_to_request))
            }
        }
    }

    let mut tools = Vec::new();
    for tool in config.tools.unwrap_or_default() {
        let param = serde_json::from_str(&tool.parameters_schema).map_err(|err| Error {
            code: ErrorCode::InternalError,
            message: format!("Failed to parse tool parameters for {}: {err}", tool.name),
            provider_error_json: None,
        })?;
        tools.push(Tool {
            tool_type: String::from("function"),
            function: FunctionTool {
                description: tool.description.unwrap_or_default(),
                name: tool.name,
                parameters: param,
            },
        });
    }

    let min_p = parse_option(&options, "min_p");
    let top_p = parse_option(&options, "top_p");
    let top_k = parse_option(&options, "top_k");
    let num_predict = parse_option(&options, "num_predict");
    let stop = config.stop_sequences.clone();
    let repeat_penalty = parse_option(&options, "repeat_penalty");
    let num_ctx = parse_option(&options, "num_ctx");
    let seed = parse_option(&options, "seed");
    let mirostat = parse_option(&options, "mirostat");
    let mirostat_eta = parse_option(&options, "mirostat_eta");
    let mirostat_tau = parse_option(&options, "mirostat_tau");
    let num_gpu = parse_option(&options, "num_gpu");
    let num_thread = parse_option(&options, "num_thread");
    let penalize_newline = parse_option(&options, "penalize_newline");
    let num_keep = parse_option(&options, "num_keep");
    let typical_p = parse_option(&options, "typical_p");
    let repeat_last_n = parse_option(&options, "repeat_last_n");
    let presence_penalty = parse_option(&options, "presence_penalty");
    let frequency_penalty = parse_option(&options, "frequency_penalty");
    let numa = parse_option(&options, "numa");
    let num_batch = parse_option(&options, "num_batch");
    let main_gpu = parse_option(&options, "main_gpu");
    let use_mmap = parse_option(&options, "use_mmap");
    let format = options.get("format").cloned();
    let keep_alive = options.get("keep_alive").cloned();

    if min_p.is_some() {
        option_additional_params.remove("min_p");
    }
    if top_p.is_some() {
        option_additional_params.remove("top_p");
    }
    if top_k.is_some() {
        option_additional_params.remove("top_k");
    }
    if num_predict.is_some() {
        option_additional_params.remove("num_predict");
    }
    if stop.is_some() {
        option_additional_params.remove("stop");
    }
    if repeat_penalty.is_some() {
        option_additional_params.remove("repeat_penalty");
    }
    if num_ctx.is_some() {
        option_additional_params.remove("num_ctx");
    }
    if seed.is_some() {
        option_additional_params.remove("seed");
    }
    if mirostat.is_some() {
        option_additional_params.remove("mirostat");
    }
    if mirostat_eta.is_some() {
        option_additional_params.remove("mirostat_eta");
    }
    if mirostat_tau.is_some() {
        option_additional_params.remove("mirostat_tau");
    }
    if num_gpu.is_some() {
        option_additional_params.remove("num_gpu");
    }
    if num_thread.is_some() {
        option_additional_params.remove("num_thread");
    }
    if penalize_newline.is_some() {
        option_additional_params.remove("penalize_newline");
    }
    if num_keep.is_some() {
        option_additional_params.remove("num_keep");
    }
    if typical_p.is_some() {
        option_additional_params.remove("typical_p");
    }
    if repeat_last_n.is_some() {
        option_additional_params.remove("repeat_last_n");
    }
    if presence_penalty.is_some() {
        option_additional_params.remove("presence_penalty");
    }
    if frequency_penalty.is_some() {
        option_additional_params.remove("frequency_penalty");
    }
    if numa.is_some() {
        option_additional_params.remove("numa");
    }
    if num_batch.is_some() {
        option_additional_params.remove("num_batch");
    }
    if main_gpu.is_some() {
        option_additional_params.remove("main_gpu");
    }
    if use_mmap.is_some() {
        option_additional_params.remove("use_mmap");
    }
    if config.temperature.is_some() {
        option_additional_params.remove("temperature");
    }

    if format.is_some() {
        option_additional_params.remove("format");
        request_additional_params.remove("format");
    }
    if keep_alive.is_some() {
        option_additional_params.remove("keep_alive");
        request_additional_params.remove("keep_alive");
    }

    option_additional_params.remove("model");
    option_additional_params.remove("messages");
    option_additional_params.remove("tools");
    option_additional_params.remove("stream");
    option_additional_params.remove("keep_alive");
    option_additional_params.remove("format");
    option_additional_params.remove("options");

    request_additional_params.remove("model");
    request_additional_params.remove("messages");
    request_additional_params.remove("tools");
    request_additional_params.remove("stream");
    request_additional_params.remove("options");

    let ollama_options = OllamaModelOptions {
        min_p,
        temperature: config.temperature,
        top_p,
        top_k,
        num_predict,
        stop,
        repeat_penalty,
        num_ctx,
        seed,
        mirostat,
        mirostat_eta,
        mirostat_tau,
        num_gpu,
        num_thread,
        penalize_newline,
        num_keep,
        typical_p,
        repeat_last_n,
        presence_penalty,
        frequency_penalty,
        numa,
        num_batch,
        main_gpu,
        use_mmap,
        additional_params: option_additional_params,
    };

    Ok(CompletionsRequest {
        model: Some(config.model.clone()),
        messages: Some(request_messages),
        tools: Some(tools),
        format,
        options: Some(ollama_options),
        keep_alive,
        stream: Some(false),
        additional_params: request_additional_params,
    })
}

fn message_to_request(message: Message) -> MessageRequest {
    let message_role = match message.role {
        Role::Assistant => MessageRole::Assistant,
        Role::System => MessageRole::System,
        Role::User => MessageRole::User,
        Role::Tool => MessageRole::User, // Ollama treats tool results as user input
    };

    let mut message_content = String::new();
    let mut attached_image = Vec::new();

    for content_part in message.content {
        match content_part {
            ContentPart::Text(text) => {
                if !message_content.is_empty() {
                    message_content.push('\n');
                }
                message_content.push_str(&text);
            }
            ContentPart::Image(reference) => match reference {
                ImageReference::Url(image_url) => {
                    let url = &image_url.url;
                    match image_to_base64(url) {
                        Ok(image) => attached_image.push(image),
                        Err(err) => {
                            trace!("Failed to encode image: {url}\nError: {err}\n");
                        }
                    }
                }
                ImageReference::Inline(image_source) => {
                    let base64_data = general_purpose::STANDARD.encode(&image_source.data);
                    attached_image.push(base64_data);
                }
            },
        }
    }

    MessageRequest {
        content: message_content,
        role: message_role,
        images: if attached_image.is_empty() {
            None
        } else {
            Some(attached_image)
        },
        tools_calls: None,
    }
}

fn response_to_request(response: Response) -> MessageRequest {
    let mut message_content = String::new();
    let mut attached_image = Vec::new();

    for content_part in response.content {
        match content_part {
            ContentPart::Text(text) => {
                if !message_content.is_empty() {
                    message_content.push('\n');
                }
                message_content.push_str(&text);
            }
            ContentPart::Image(reference) => match reference {
                ImageReference::Url(image_url) => {
                    let url = &image_url.url;
                    match image_to_base64(url) {
                        Ok(image) => attached_image.push(image),
                        Err(err) => {
                            trace!("Failed to encode image: {url}\nError: {err}\n");
                        }
                    }
                }
                ImageReference::Inline(image_source) => {
                    let base64_data = general_purpose::STANDARD.encode(&image_source.data);
                    attached_image.push(base64_data);
                }
            },
        }
    }

    let tool_calls = response
        .tool_calls
        .into_iter()
        .map(|tool_call| Tool {
            tool_type: String::from("function"),
            function: FunctionTool {
                name: tool_call.name.clone(),
                description: "".to_string(),
                parameters: tool_call.arguments_json.clone().parse().unwrap_or_default(),
            },
        })
        .collect::<Vec<_>>();

    MessageRequest {
        content: message_content,
        role: Assistant,
        images: if attached_image.is_empty() {
            None
        } else {
            Some(attached_image)
        },
        tools_calls: (!tool_calls.is_empty()).then_some(tool_calls),
    }
}

fn tool_result_to_request(tool_result: ToolResult) -> MessageRequest {
    MessageRequest {
        role: MessageRole::User,
        // For better durability, we will add the tool call result in a structured format.
        // This will help in retying and continuing the interrupted conversation.
        // This will help prevent branching conversations and repeating the tool call.
        content: match tool_result {
            ToolResult::Success(success) => {
                format!(
                    "[ToolCall Result]: Successed , [ToolCall ID]: {}, [ToolCall Name]: {}, [Result]: {}] ",
                    success.id,
                    success.name,
                    success.result_json,
                )
            }
            ToolResult::Error(error) => format!(
                "[ToolCall Result]: Failed, [ToolCall ID]: {}, [ErrorName]: {}, [ErrorCode]: {}, [Error]: {}",
                error.id,
                error.name,
                error.error_code.clone().unwrap_or_default(),
                error.error_message,
            ),
        },
        images: None,
        tools_calls: None,
    }
}

fn parse_option<T: std::str::FromStr>(options: &HashMap<String, String>, key: &str) -> Option<T> {
    options.get(key).and_then(|v| v.parse::<T>().ok())
}

pub fn process_response(response: CompletionsResponse) -> Result<Response, Error> {
    if let Some(ref message) = response.message {
        let mut content = Vec::<ContentPart>::new();
        let mut tool_calls = Vec::<GolemToolCall>::new();

        if let Some(ref message_content) = message.content {
            content.push(ContentPart::Text(message_content.clone()));
        }

        if let Some(ref message_tool_calls) = message.tool_calls {
            for tool_call in message_tool_calls {
                tool_calls.push(GolemToolCall {
                    id: format!("ollama-{}", response.created_at.clone()),
                    name: tool_call.name.clone().unwrap_or_default(),
                    arguments_json: tool_call.function.as_ref().unwrap().arguments.to_string(),
                });
            }
        }

        let finish_reason = if response.done.unwrap_or(false) {
            Some(FinishReason::Stop)
        } else {
            None
        };
        let input_tokens = response.prompt_eval_count.map(|c| c as u32);
        let output_tokens = response.eval_count.map(|c| c as u32);

        let usage = Usage {
            input_tokens,
            output_tokens,
            total_tokens: Some(input_tokens.unwrap_or(0) + output_tokens.unwrap_or(0)),
        };

        let timestamp = response.created_at.clone();

        let metadata = ResponseMetadata {
            finish_reason,
            usage: Some(usage),
            provider_id: Some("ollama".to_string()),
            timestamp: Some(timestamp.clone()),
            provider_metadata_json: Some(get_provider_metadata(&response)),
        };

        Ok(Response {
            id: format!("ollama-{timestamp}"),
            content,
            tool_calls,
            metadata,
        })
    } else {
        Err(Error {
            code: ErrorCode::InternalError,
            message: String::from("No messages in response"),
            provider_error_json: None,
        })
    }
}

pub fn get_provider_metadata(response: &CompletionsResponse) -> String {
    format!(
        r#"{{
            "total_duration":"{}",
            "load_duration":"{}",
            "prompt_eval_duration":{},
            "eval_duration":{},
            "context":{},
        }}"#,
        response.total_duration.unwrap_or(0),
        response.load_duration.unwrap_or(0),
        response.prompt_eval_duration.unwrap_or(0),
        response.eval_duration.unwrap_or(0),
        response.eval_count.unwrap_or(0)
    )
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
            model: "ollama-test".to_string(),
            temperature: Some(0.4),
            max_tokens: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            provider_options,
        }
    }

    #[test]
    fn forwards_unmapped_options_inside_ollama_options() {
        let request = events_to_request(
            Vec::new(),
            base_config(Some(vec![
                kv("top_p", "0.8"),
                kv("new_sampling_mode", "\"aggressive\""),
                kv("format", "json"),
                kv("request.raw", "true"),
            ])),
        )
        .unwrap();

        assert_eq!(request.format, Some("json".to_string()));
        assert_eq!(
            request.additional_params.get("raw"),
            Some(&serde_json::json!(true))
        );

        let options = request.options.expect("options should be present");
        assert_eq!(options.top_p, Some(0.8));
        assert_eq!(
            options.additional_params.get("new_sampling_mode"),
            Some(&serde_json::json!("aggressive"))
        );
        assert!(!options.additional_params.contains_key("top_p"));
    }

    #[test]
    fn keeps_known_option_when_typed_parse_fails() {
        let request =
            events_to_request(Vec::new(), base_config(Some(vec![kv("num_ctx", "large")]))).unwrap();

        let options = request.options.expect("options should be present");
        assert_eq!(options.num_ctx, None);
        assert_eq!(
            options.additional_params.get("num_ctx"),
            Some(&serde_json::json!("large"))
        );
    }
}

use crate::client::{
    CreateModelResponseResponse, InputItem, OutputItem, ResponseOutputItemDone,
    ResponseOutputTextDelta, ResponsesApi,
};
use crate::conversions::{
    create_request, create_response_metadata, events_to_input_items, parse_error_code,
    process_model_response, tool_defs_to_tools,
};
use golem_llm::chat_stream::{LlmChatStream, LlmChatStreamState};
use golem_llm::config::{get_config_key, with_config_key};
use golem_llm::durability::{DurableLLM, ExtendedGuest};
use golem_llm::event_source::EventSource;
use golem_llm::golem::llm::llm::{
    ChatStream, Config, ContentPart, Error, ErrorCode, Event, Guest, Response, StreamDelta,
    StreamEvent, ToolCall,
};
use golem_rust::golem_wasm::Pollable;
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
        let json: serde_json::Value = serde_json::from_str(raw).map_err(|err| {
            decode_internal_error(format!("Failed to deserialize stream event: {err}"))
        })?;

        let typ = json
            .as_object()
            .and_then(|obj| obj.get("type"))
            .and_then(|v| v.as_str());
        match typ {
            Some("response.failed") => {
                let response = json
                    .as_object()
                    .and_then(|obj| obj.get("response"))
                    .ok_or_else(|| {
                        decode_internal_error(
                            "Unexpected stream event format, does not have 'response' field",
                        )
                    })?;
                let decoded =
                    serde_json::from_value::<CreateModelResponseResponse>(response.clone())
                        .map_err(|err| {
                            decode_internal_error(format!(
                                "Failed to deserialize stream event's response field: {err}"
                            ))
                        })?;

                if let Some(error) = decoded.error {
                    Err(Error {
                        code: parse_error_code(error.code),
                        message: error.message,
                        provider_error_json: None,
                    })
                } else {
                    Err(Error {
                        code: ErrorCode::Unknown,
                        message: "Unknown error".to_string(),
                        provider_error_json: None,
                    })
                }
            }
            Some("response.completed") => {
                let response = json
                    .as_object()
                    .and_then(|obj| obj.get("response"))
                    .ok_or_else(|| {
                        decode_internal_error(
                            "Unexpected stream event format, does not have 'response' field",
                        )
                    })?;
                let decoded =
                    serde_json::from_value::<CreateModelResponseResponse>(response.clone())
                        .map_err(|err| {
                            decode_internal_error(format!(
                                "Failed to deserialize stream event's response field: {err}"
                            ))
                        })?;
                Ok(Some(StreamEvent::Finish(create_response_metadata(
                    &decoded,
                ))))
            }
            Some("response.output_text.delta") => {
                let decoded =
                    serde_json::from_value::<ResponseOutputTextDelta>(json).map_err(|err| {
                        decode_internal_error(format!("Failed to deserialize stream event: {err}"))
                    })?;
                Ok(Some(StreamEvent::Delta(StreamDelta {
                    content: Some(vec![ContentPart::Text(decoded.delta)]),
                    tool_calls: None,
                })))
            }
            Some("response.output_item.done") => {
                let decoded =
                    serde_json::from_value::<ResponseOutputItemDone>(json).map_err(|err| {
                        decode_internal_error(format!("Failed to deserialize stream event: {err}"))
                    })?;
                if let OutputItem::ToolCall {
                    arguments,
                    call_id,
                    name,
                    ..
                } = decoded.item
                {
                    Ok(Some(StreamEvent::Delta(StreamDelta {
                        content: None,
                        tool_calls: Some(vec![ToolCall {
                            id: call_id,
                            name,
                            arguments_json: arguments,
                        }]),
                    })))
                } else {
                    Ok(None)
                }
            }
            Some(_) => Ok(None),
            None => Err(decode_internal_error(
                "Unexpected stream event format, does not have 'type' field",
            )),
        }
    }
}

struct OpenAIComponent;

impl OpenAIComponent {
    const ENV_VAR_NAME: &'static str = "OPENAI_API_KEY";

    fn request(
        client: ResponsesApi,
        items: Vec<InputItem>,
        config: Config,
    ) -> Result<Response, Error> {
        let tools = tool_defs_to_tools(config.tools.clone())?;
        let request = create_request(items, config, tools);
        let response = client.create_model_response(request)?;
        process_model_response(response)
    }

    fn streaming_request(
        client: ResponsesApi,
        items: Vec<InputItem>,
        config: Config,
    ) -> LlmChatStream<OpenAIChatStream> {
        match tool_defs_to_tools(config.tools.clone()) {
            Ok(tools) => {
                let mut request = create_request(items, config, tools);
                request.stream = true;
                match client.stream_model_response(request) {
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

    fn send(events: Vec<Event>, config: Config) -> Result<Response, Error> {
        let openai_api_key = get_config_key(Self::ENV_VAR_NAME)?;
        let client = ResponsesApi::new(openai_api_key);
        let items = events_to_input_items(events);
        Self::request(client, items, config)
    }

    fn stream(events: Vec<Event>, config: Config) -> ChatStream {
        ChatStream::new(Self::unwrapped_stream(events, config))
    }
}

impl ExtendedGuest for OpenAIComponent {
    fn unwrapped_stream(events: Vec<Event>, config: Config) -> Self::ChatStream {
        with_config_key(
            Self::ENV_VAR_NAME,
            OpenAIChatStream::failed,
            |openai_api_key| {
                let client = ResponsesApi::new(openai_api_key);
                let items = events_to_input_items(events);
                Self::streaming_request(client, items, config)
            },
        )
    }

    fn subscribe(stream: &Self::ChatStream) -> Pollable {
        stream.subscribe()
    }
}

type DurableOpenAIComponent = DurableLLM<OpenAIComponent>;

golem_llm::export_llm!(DurableOpenAIComponent with_types_in golem_llm);

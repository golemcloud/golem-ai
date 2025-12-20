mod client;
mod conversions;

use crate::client::{ChatCompletionChunk, CompletionsApi, CompletionsRequest, StreamOptions};
use crate::conversions::{
    convert_client_tool_call_to_tool_call, convert_finish_reason, convert_usage, events_to_request,
    process_response,
};
use golem_llm::chat_stream::{LlmChatStream, LlmChatStreamState};
use golem_llm::config::{get_config_key, with_config_key};
use golem_llm::durability::{DurableLLM, ExtendedGuest};
use golem_llm::event_source::EventSource;
use golem_llm::golem::llm::llm::{
    ChatStream, Config, ContentPart, Error, ErrorCode, Event, FinishReason, Guest, Response,
    ResponseMetadata, StreamDelta, StreamEvent,
};
use golem_rust::golem_wasm::Pollable;
use log::trace;
use std::cell::{Ref, RefCell, RefMut};

struct GrokChatStream {
    stream: RefCell<Option<EventSource>>,
    failure: Option<Error>,
    finished: RefCell<bool>,
    finish_reason: RefCell<Option<FinishReason>>,
}

impl GrokChatStream {
    pub fn new(stream: EventSource) -> LlmChatStream<Self> {
        LlmChatStream::new(GrokChatStream {
            stream: RefCell::new(Some(stream)),
            failure: None,
            finished: RefCell::new(false),
            finish_reason: RefCell::new(None),
        })
    }

    pub fn failed(error: Error) -> LlmChatStream<Self> {
        LlmChatStream::new(GrokChatStream {
            stream: RefCell::new(None),
            failure: Some(error),
            finished: RefCell::new(false),
            finish_reason: RefCell::new(None),
        })
    }
}

impl LlmChatStreamState for GrokChatStream {
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
            .and_then(|obj| obj.get("object"))
            .and_then(|v| v.as_str());
        match typ {
            Some("chat.completion.chunk") => {
                let message: ChatCompletionChunk = serde_json::from_value(json).map_err(|err| {
                    decode_internal_error(format!("Failed to parse stream event: {err}"))
                })?;
                if let Some(choice) = message.choices.into_iter().next() {
                    if let Some(finish_reason) = choice.finish_reason {
                        *self.finish_reason.borrow_mut() =
                            Some(convert_finish_reason(&finish_reason));
                    }
                    Ok(Some(StreamEvent::Delta(StreamDelta {
                        content: choice
                            .delta
                            .content
                            .map(|text| vec![ContentPart::Text(text)]),
                        tool_calls: choice.delta.tool_calls.map(|calls| {
                            calls
                                .into_iter()
                                .map(convert_client_tool_call_to_tool_call)
                                .collect()
                        }),
                    })))
                } else if let Some(usage) = message.usage {
                    let finish_reason = self.finish_reason.borrow();
                    Ok(Some(StreamEvent::Finish(ResponseMetadata {
                        finish_reason: *finish_reason,
                        usage: Some(convert_usage(&usage)),
                        provider_id: None,
                        timestamp: Some(message.created.to_string()),
                        provider_metadata_json: None,
                    })))
                } else {
                    Ok(None)
                }
            }
            Some(_) => Ok(None),
            None => Err(decode_internal_error(
                "Unexpected stream event format, does not have 'object' field",
            )),
        }
    }
}

struct GrokComponent;

impl GrokComponent {
    const ENV_VAR_NAME: &'static str = "XAI_API_KEY";

    fn request(client: CompletionsApi, request: CompletionsRequest) -> Result<Response, Error> {
        let response = client.send_messages(request)?;
        process_response(response)
    }

    fn streaming_request(
        client: CompletionsApi,
        mut request: CompletionsRequest,
    ) -> LlmChatStream<GrokChatStream> {
        request.stream = Some(true);
        request.stream_options = Some(StreamOptions {
            include_usage: true,
        });
        match client.stream_send_messages(request) {
            Ok(stream) => GrokChatStream::new(stream),
            Err(err) => GrokChatStream::failed(err),
        }
    }
}

impl Guest for GrokComponent {
    type ChatStream = LlmChatStream<GrokChatStream>;

    fn send(events: Vec<Event>, config: Config) -> Result<Response, Error> {
        let xai_api_key = get_config_key(Self::ENV_VAR_NAME)?;
        let client = CompletionsApi::new(xai_api_key);
        let request = events_to_request(events, config)?;
        Self::request(client, request)
    }

    fn stream(messages: Vec<Event>, config: Config) -> ChatStream {
        ChatStream::new(Self::unwrapped_stream(messages, config))
    }
}

impl ExtendedGuest for GrokComponent {
    fn unwrapped_stream(messages: Vec<Event>, config: Config) -> LlmChatStream<GrokChatStream> {
        with_config_key(Self::ENV_VAR_NAME, GrokChatStream::failed, |xai_api_key| {
            let client = CompletionsApi::new(xai_api_key);

            match events_to_request(messages, config) {
                Ok(request) => Self::streaming_request(client, request),
                Err(err) => GrokChatStream::failed(err),
            }
        })
    }

    fn subscribe(stream: &Self::ChatStream) -> Pollable {
        stream.subscribe()
    }
}

type DurableGrokComponent = DurableLLM<GrokComponent>;

golem_llm::export_llm!(DurableGrokComponent with_types_in golem_llm);

use crate::conversions::{converse_stream_output_to_stream_event, custom_error, merge_metadata};
use async_trait::async_trait;
use aws_sdk_bedrockruntime::{
    self as bedrock, primitives::event_stream::EventReceiver,
    types::error::ConverseStreamOutputError,
};
use golem_ai_llm::{model as llm, ChatStreamInterface};
use std::cell::RefCell;

type BedrockEventSource =
    EventReceiver<bedrock::types::ConverseStreamOutput, ConverseStreamOutputError>;

pub struct BedrockChatStream {
    stream: RefCell<Option<BedrockEventSource>>,
    failure: Option<llm::Error>,
    finished: RefCell<bool>,
}

impl BedrockChatStream {
    pub fn new(stream: BedrockEventSource) -> BedrockChatStream {
        BedrockChatStream {
            stream: RefCell::new(Some(stream)),
            failure: None,
            finished: RefCell::new(false),
        }
    }

    pub fn failed(error: llm::Error) -> BedrockChatStream {
        BedrockChatStream {
            stream: RefCell::new(None),
            failure: Some(error),
            finished: RefCell::new(true),
        }
    }

    fn failure(&self) -> &Option<llm::Error> {
        &self.failure
    }

    fn is_finished(&self) -> bool {
        *self.finished.borrow()
    }

    fn set_finished(&self) {
        *self.finished.borrow_mut() = true;
    }
    async fn get_single_event(&self) -> Option<Result<llm::StreamEvent, llm::Error>> {
        // Take the stream out of the RefCell so we don't hold a borrow across .await,
        // then put it back. Single-threaded WASM means no one else can observe the
        // momentarily-empty slot.
        let taken_stream = self.stream.borrow_mut().take();
        let (maybe_token, stream_back) = match taken_stream {
            Some(mut stream) => {
                let token = stream.recv().await;
                (Some(token), Some(stream))
            }
            None => (None, None),
        };
        *self.stream.borrow_mut() = stream_back;

        match maybe_token {
            Some(token) => {
                log::trace!("Bedrock stream event: {token:?}");
                match token {
                    Ok(Some(output)) => {
                        log::trace!("Processing bedrock stream event: {output:?}");
                        converse_stream_output_to_stream_event(output).map(Ok)
                    }
                    Ok(None) => {
                        log::trace!("running set_finished on stream due to None event received");
                        self.set_finished();
                        None
                    }
                    Err(error) => {
                        log::trace!("running set_finished on stream due to error: {error:?}");
                        self.set_finished();
                        Some(Err(custom_error(
                            llm::ErrorCode::InternalError,
                            format!("An error occurred while reading event stream: {error}"),
                        )))
                    }
                }
            }
            None => {
                if let Some(error) = self.failure() {
                    self.set_finished();
                    Some(Err(error.clone()))
                } else {
                    None
                }
            }
        }
    }
}

#[async_trait(?Send)]
impl ChatStreamInterface for BedrockChatStream {
    async fn poll_next(&self) -> Option<Vec<Result<llm::StreamEvent, llm::Error>>> {
        if self.is_finished() {
            return Some(vec![]);
        }
        let event = self.get_single_event().await?;
        if let Ok(llm::StreamEvent::Finish(metadata)) = &event {
            if let Some(Ok(llm::StreamEvent::Finish(final_metadata))) =
                self.get_single_event().await
            {
                return Some(vec![Ok(llm::StreamEvent::Finish(merge_metadata(
                    metadata.clone(),
                    final_metadata,
                )))]);
            }
        }
        Some(vec![event])
    }

    async fn get_next(&self) -> Vec<Result<llm::StreamEvent, llm::Error>> {
        loop {
            if let Some(events) = self.poll_next().await {
                return events;
            }
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

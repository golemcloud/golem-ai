use crate::conversions::{converse_stream_output_to_stream_event, custom_error, merge_metadata};
use aws_sdk_bedrockruntime::{
    self as bedrock, primitives::event_stream::EventReceiver,
    types::error::ConverseStreamOutputError,
};
use futures::lock::Mutex;
use golem_ai_llm::{model as llm, ChatStreamInterface, LlmFuture};
use std::cell::RefCell;

type BedrockEventSource =
    EventReceiver<bedrock::types::ConverseStreamOutput, ConverseStreamOutputError>;

pub struct BedrockChatStream {
    stream: Mutex<Option<BedrockEventSource>>,
    failure: Option<llm::Error>,
    finished: RefCell<bool>,
}

impl BedrockChatStream {
    pub fn new(stream: BedrockEventSource) -> BedrockChatStream {
        BedrockChatStream {
            stream: Mutex::new(Some(stream)),
            failure: None,
            finished: RefCell::new(false),
        }
    }

    pub fn failed(error: llm::Error) -> BedrockChatStream {
        BedrockChatStream {
            stream: Mutex::new(None),
            failure: Some(error),
            finished: RefCell::new(false),
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
}

impl ChatStreamInterface for BedrockChatStream {
    fn get_next(&self) -> LlmFuture<'_, Vec<Result<llm::StreamEvent, llm::Error>>> {
        Box::pin(async move {
            if self.is_finished() {
                return vec![];
            }
            let mut stream_guard = self.stream.lock().await;
            if self.is_finished() {
                return vec![];
            }
            let Some(stream) = stream_guard.as_mut() else {
                if let Some(error) = self.failure() {
                    self.set_finished();
                    return vec![Err(error.clone())];
                }
                return vec![];
            };

            loop {
                let token = stream.recv().await;
                log::trace!("Bedrock stream event: {token:?}");
                let event = match token {
                    Ok(Some(output)) => {
                        log::trace!("Processing bedrock stream event: {output:?}");
                        let Some(event) = converse_stream_output_to_stream_event(output) else {
                            continue;
                        };
                        Ok(event)
                    }
                    Ok(None) => {
                        log::trace!("running set_finished on stream due to None event received");
                        self.set_finished();
                        return vec![];
                    }
                    Err(error) => {
                        log::trace!("running set_finished on stream due to error: {error:?}");
                        self.set_finished();
                        return vec![Err(custom_error(
                            llm::ErrorCode::InternalError,
                            format!("An error occurred while reading event stream: {error}"),
                        ))];
                    }
                };

                if let Ok(llm::StreamEvent::Finish(metadata)) = &event {
                    let token = stream.recv().await;
                    log::trace!("Bedrock stream event: {token:?}");
                    match token {
                        Ok(Some(output)) => {
                            log::trace!("Processing bedrock stream event: {output:?}");
                            if let Some(llm::StreamEvent::Finish(final_metadata)) =
                                converse_stream_output_to_stream_event(output)
                            {
                                return vec![Ok(llm::StreamEvent::Finish(merge_metadata(
                                    metadata.clone(),
                                    final_metadata,
                                )))];
                            }
                        }
                        Ok(None) => {
                            log::trace!(
                                "running set_finished on stream due to None event received"
                            );
                            self.set_finished();
                        }
                        Err(error) => {
                            log::trace!("running set_finished on stream due to error: {error:?}");
                            self.set_finished();
                        }
                    }
                }
                return vec![event];
            }
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_stream_emits_error_once() {
        let error = custom_error(llm::ErrorCode::InvalidRequest, "test failure".to_string());
        let stream = BedrockChatStream::failed(error.clone());

        assert_eq!(
            futures::executor::block_on(stream.get_next()),
            vec![Err(error)]
        );
        assert!(futures::executor::block_on(stream.get_next()).is_empty());
    }
}

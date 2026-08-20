use crate::event_source::{Event, EventSource, MessageEvent};
use crate::model::{Error, ErrorCode, StreamEvent};
use crate::{ChatStreamInterface, LlmFuture};
use futures::lock::Mutex;

pub trait LlmChatStreamState: 'static {
    fn failure(&self) -> &Option<Error>;
    fn is_finished(&self) -> bool;
    fn set_finished(&self);
    fn stream(&self) -> &Mutex<Option<EventSource>>;
    fn decode_message(&self, raw: &str) -> Result<Option<StreamEvent>, Error>;
}

pub struct LlmChatStream<T> {
    implementation: T,
}

impl<T: LlmChatStreamState> LlmChatStream<T> {
    pub fn new(implementation: T) -> Self {
        Self { implementation }
    }
}

impl<T: LlmChatStreamState> ChatStreamInterface for LlmChatStream<T> {
    fn get_next(&self) -> LlmFuture<'_, Vec<Result<StreamEvent, Error>>> {
        Box::pin(async move {
            if self.implementation.is_finished() {
                return vec![];
            }
            let mut source = self.implementation.stream().lock().await;
            // A concurrent caller may have observed the stream as unfinished before waiting
            // for this lock, then yielded while the active reader received a finish event.
            if self.implementation.is_finished() {
                return vec![];
            }
            if let Some(source) = source.as_mut() {
                loop {
                    match source.next().await {
                        None | Some(Err(crate::event_source::error::Error::StreamEnded)) => {
                            self.implementation.set_finished();
                            return vec![];
                        }
                        Some(Err(error)) => {
                            self.implementation.set_finished();
                            return vec![Err(Error {
                                code: ErrorCode::InternalError,
                                message: error.to_string(),
                                provider_error_json: None,
                            })];
                        }
                        Some(Ok(Event::Open)) => {}
                        Some(Ok(Event::Message(MessageEvent { data, .. }))) if data == "[DONE]" => {
                        }
                        Some(Ok(Event::Message(MessageEvent { data, .. }))) => {
                            match self.implementation.decode_message(&data) {
                                Ok(Some(event)) => {
                                    if matches!(event, StreamEvent::Finish(_)) {
                                        self.implementation.set_finished();
                                    }
                                    return vec![Ok(event)];
                                }
                                Ok(None) => {}
                                Err(error) => return vec![Err(error)],
                            }
                        }
                    }
                }
            } else if let Some(error) = self.implementation.failure().clone() {
                self.implementation.set_finished();
                vec![Err(error)]
            } else {
                vec![]
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

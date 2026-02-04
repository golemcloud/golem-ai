use crate::event_source::{Event, EventSource, MessageEvent};
use crate::golem::llm::llm::{Error, ErrorCode, GuestChatStream, StreamEvent};
use golem_rust::golem_wasm::Pollable;
use std::cell::{Ref, RefMut};
use std::task::Poll;

pub trait LlmChatStreamState: 'static {
    fn failure(&self) -> &Option<Error>;
    fn is_finished(&self) -> bool;
    fn set_finished(&self);
    fn stream(&self) -> Ref<'_, Option<EventSource>>;
    fn stream_mut(&self) -> RefMut<'_, Option<EventSource>>;
    fn decode_message(&self, raw: &str) -> Result<Option<StreamEvent>, Error>;
}

pub struct LlmChatStream<T> {
    implementation: T,
}

impl<T: LlmChatStreamState> LlmChatStream<T> {
    pub fn new(implementation: T) -> Self {
        Self { implementation }
    }

    pub fn subscribe(&self) -> Pollable {
        if let Some(stream) = self.implementation.stream().as_ref() {
            stream.subscribe()
        } else {
            golem_rust::bindings::wasi::clocks::monotonic_clock::subscribe_duration(0)
        }
    }
}

impl<T: LlmChatStreamState> GuestChatStream for LlmChatStream<T> {
    fn poll_next(&self) -> Option<Vec<Result<StreamEvent, Error>>> {
        if self.implementation.is_finished() {
            return Some(vec![]);
        }

        let mut stream = self.implementation.stream_mut();
        if let Some(stream) = stream.as_mut() {
            match stream.poll_next() {
                Poll::Ready(None) => {
                    self.implementation.set_finished();
                    Some(vec![])
                }
                Poll::Ready(Some(Err(crate::event_source::error::Error::StreamEnded))) => {
                    self.implementation.set_finished();
                    Some(vec![])
                }
                Poll::Ready(Some(Err(error))) => {
                    self.implementation.set_finished();
                    Some(vec![Err(Error {
                        code: ErrorCode::InternalError,
                        message: error.to_string(),
                        provider_error_json: None,
                    })])
                }
                Poll::Ready(Some(Ok(event))) => {
                    let mut events = vec![];

                    match event {
                        Event::Open => {}
                        Event::Message(MessageEvent { data, .. }) => {
                            if data != "[DONE]" {
                                match self.implementation.decode_message(&data) {
                                    Ok(Some(stream_event)) => {
                                        if matches!(stream_event, StreamEvent::Finish(_)) {
                                            self.implementation.set_finished();
                                        }
                                        events.push(Ok(stream_event));
                                    }
                                    Ok(None) => {
                                        // Ignored event
                                    }
                                    Err(err) => events.push(Err(err)),
                                }
                            }
                        }
                    }

                    if events.is_empty() {
                        None
                    } else {
                        Some(events)
                    }
                }
                Poll::Pending => None,
            }
        } else if let Some(error) = self.implementation.failure().clone() {
            self.implementation.set_finished();
            Some(vec![Err(error)])
        } else {
            None
        }
    }

    fn get_next(&self) -> Vec<Result<StreamEvent, Error>> {
        let pollable = self.subscribe();
        loop {
            pollable.block();
            if let Some(events) = self.poll_next() {
                return events;
            }
        }
    }
}

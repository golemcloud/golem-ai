// Based on https://github.com/jpopesculian/eventsource-stream and https://github.com/jpopesculian/reqwest-eventsource
// modified to use the wasi-http based reqwest, and wasi pollables

pub mod error;
mod event_stream;
mod message_event;
mod ndjson_stream;
mod parser;
mod stream;
mod utf8_stream;

use crate::event_source::error::Error;
use crate::event_source::event_stream::EventStream;
use golem_ai_http::{HeaderValue, Response, StatusCode};
pub use message_event::MessageEvent;
use ndjson_stream::NdJsonStream;
use stream::{LlmStream, StreamType};

/// The ready state of an [`EventSource`]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u8)]
pub enum ReadyState {
    /// The EventSource is waiting on a response from the endpoint
    Connecting = 0,
    /// The EventSource is connected
    Open = 1,
    /// The EventSource is closed and no longer emitting Events
    Closed = 2,
}

pub struct EventSource {
    /// stream is the type which implements Stream trait
    stream: StreamType,
    is_closed: bool,
}

impl EventSource {
    #[allow(clippy::result_large_err)]
    pub fn new(response: Response) -> Result<Self, Error> {
        match check_response(response) {
            Ok(response) => {
                let stream = if response
                    .headers()
                    .get(golem_ai_http::header::CONTENT_TYPE)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .contains("ndjson")
                {
                    StreamType::NdJsonStream(NdJsonStream::new(response.into_body()))
                } else {
                    StreamType::EventStream(EventStream::new(response.into_body()))
                };
                Ok(Self {
                    stream,
                    is_closed: false,
                })
            }
            Err(err) => Err(err),
        }
    }

    /// Close the EventSource stream and stop trying to reconnect
    pub fn close(&mut self) {
        self.is_closed = true;
    }

    /// Get the current ready state
    pub fn ready_state(&self) -> ReadyState {
        if self.is_closed {
            ReadyState::Closed
        } else {
            ReadyState::Open
        }
    }

    #[allow(clippy::result_large_err)]
    pub async fn next(&mut self) -> Option<Result<Event, Error>> {
        if self.is_closed {
            return None;
        }

        match &mut self.stream {
            StreamType::EventStream(stream) => stream
                .next()
                .await
                .map(|r| r.map(Event::Message).map_err(Into::into)),
            StreamType::NdJsonStream(stream) => stream
                .next()
                .await
                .map(|r| r.map(Event::Message).map_err(Into::into)),
        }
    }
}

#[allow(clippy::result_large_err)]
fn check_response(response: Response) -> Result<Response, Error> {
    match response.status() {
        StatusCode::OK => {}
        status => {
            return Err(Error::InvalidStatusCode(status, response));
        }
    }
    let content_type =
        if let Some(content_type) = response.headers().get(golem_ai_http::header::CONTENT_TYPE) {
            content_type
        } else {
            return Err(Error::InvalidContentType(
                HeaderValue::from_static(""),
                response,
            ));
        };
    if content_type
        .to_str()
        .map_err(|_| ())
        .and_then(|s| s.parse::<mime::Mime>().map_err(|_| ()))
        .map(|mime_type| {
            matches!(
                (mime_type.type_(), mime_type.subtype()),
                (mime::TEXT, mime::EVENT_STREAM)
            ) || mime_type.subtype().as_str().contains("ndjson")
        })
        .unwrap_or(false)
    {
        Ok(response)
    } else {
        Err(Error::InvalidContentType(content_type.clone(), response))
    }
}

/// Events created by the [`EventSource`]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Event {
    /// The event fired when the connection is opened
    Open,
    /// The event fired when a [`MessageEvent`] is received
    Message(MessageEvent),
}

impl From<MessageEvent> for Event {
    fn from(event: MessageEvent) -> Self {
        Event::Message(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use golem_ai_http::{Bytes, HeaderMap, Url};

    fn response(status: StatusCode, content_type: Option<&str>, body: &[u8]) -> Response {
        let mut headers = HeaderMap::new();
        if let Some(content_type) = content_type {
            headers.insert(
                golem_ai_http::header::CONTENT_TYPE,
                HeaderValue::from_str(content_type).unwrap(),
            );
        }
        Response::from_bytes(
            status,
            headers,
            Bytes::copy_from_slice(body),
            Url::parse("https://example.com/stream").unwrap(),
        )
    }

    #[test]
    fn reads_buffered_sse_and_terminates() {
        block_on(async {
            let mut source = EventSource::new(response(
                StatusCode::OK,
                Some("text/event-stream"),
                b"\xef\xbb\xbf:id ignored\r\nid: 7\rdata: hello\r\ndata: world\r\nretry: 10\r\n\r\n",
            ))
            .unwrap();
            let Event::Message(event) = source.next().await.unwrap().unwrap() else {
                panic!("expected message")
            };
            assert_eq!(event.data, "hello\nworld");
            assert_eq!(event.id, "7");
            assert_eq!(event.retry, Some(core::time::Duration::from_millis(10)));
            assert!(source.next().await.is_none());
        });
    }

    #[test]
    fn reads_ndjson_lines_and_flushes_final_line() {
        block_on(async {
            let mut source = EventSource::new(response(
                StatusCode::OK,
                Some("application/x-ndjson"),
                b"\n {\"one\":1}\r\n{\"two\":2}",
            ))
            .unwrap();
            let Event::Message(first) = source.next().await.unwrap().unwrap() else {
                panic!("expected message")
            };
            let Event::Message(second) = source.next().await.unwrap().unwrap() else {
                panic!("expected message")
            };
            assert_eq!(first.data, "{\"one\":1}");
            assert_eq!(second.data, "{\"two\":2}");
            assert!(source.next().await.is_none());
        });
    }

    #[test]
    fn rejects_invalid_status_and_content_type() {
        assert!(matches!(
            EventSource::new(response(
                StatusCode::BAD_REQUEST,
                Some("text/event-stream"),
                b""
            )),
            Err(Error::InvalidStatusCode(StatusCode::BAD_REQUEST, _))
        ));
        assert!(matches!(
            EventSource::new(response(StatusCode::OK, Some("application/json"), b"")),
            Err(Error::InvalidContentType(_, _))
        ));
        assert!(matches!(
            EventSource::new(response(StatusCode::OK, None, b"")),
            Err(Error::InvalidContentType(_, _))
        ));
    }
}

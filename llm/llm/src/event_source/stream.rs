use super::{
    event_stream::EventStream, ndjson_stream::NdJsonStream, utf8_stream::Utf8StreamError,
    MessageEvent,
};
use core::fmt;
use nom::error::Error as NomError;
use std::string::FromUtf8Error;

pub enum StreamType {
    EventStream(EventStream),
    NdJsonStream(NdJsonStream),
}

pub trait LlmStream {
    fn new(body: golem_ai_http::ResponseBody) -> Self;
    fn set_last_event_id(&mut self, id: impl Into<String>);
    fn last_event_id(&self) -> &str;
    async fn next(&mut self) -> Option<Result<MessageEvent, StreamError<golem_ai_http::Error>>>;
}

#[derive(Debug)]
pub enum StreamError<E> {
    Utf8(FromUtf8Error),
    Parser(NomError<String>),
    Transport(E),
}

impl<E> From<Utf8StreamError<E>> for StreamError<E> {
    fn from(err: Utf8StreamError<E>) -> Self {
        match err {
            Utf8StreamError::Utf8(e) => Self::Utf8(e),
            Utf8StreamError::Transport(e) => Self::Transport(e),
        }
    }
}
impl<E> From<NomError<&str>> for StreamError<E> {
    fn from(err: NomError<&str>) -> Self {
        Self::Parser(NomError::new(err.input.to_string(), err.code))
    }
}
impl<E: fmt::Display> fmt::Display for StreamError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Utf8(e) => write!(f, "UTF8 error: {e}"),
            Self::Parser(e) => write!(f, "Parse error: {e}"),
            Self::Transport(e) => write!(f, "Transport error: {e}"),
        }
    }
}
impl<E: fmt::Display + fmt::Debug> std::error::Error for StreamError<E> {}

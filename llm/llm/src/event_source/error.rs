use core::fmt;
use golem_ai_http::{Error as HttpError, HeaderValue, Response, StatusCode};
use nom::error::Error as NomError;
use std::string::FromUtf8Error;
use thiserror::Error;

use super::stream::StreamError;

/// Error raised when a [`RequestBuilder`] cannot be cloned. See [`RequestBuilder::try_clone`] for
/// more information
#[derive(Debug, Clone, Copy)]
pub struct CannotCloneRequestError;

impl fmt::Display for CannotCloneRequestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("expected a cloneable request")
    }
}

impl std::error::Error for CannotCloneRequestError {}

/// Error raised by the EventSource stream fetching and parsing
#[derive(Debug, Error)]
pub enum Error {
    /// Source stream is not valid UTF8
    #[error(transparent)]
    Utf8(FromUtf8Error),
    /// Source stream is not a valid EventStream
    #[error("Protocol parser error: {0:?}")]
    Parser(NomError<String>),
    /// The HTTP Request could not be completed
    #[error(transparent)]
    Transport(HttpError),
    /// Underlying HTTP response stream error
    #[error("Transport stream error: {0}")]
    TransportStream(String),
    /// The `Content-Type` returned by the server is invalid
    #[error("Invalid header value: {0:?}")]
    InvalidContentType(HeaderValue, Response),
    /// The status code returned by the server is invalid
    #[error("Invalid status code: {0}")]
    InvalidStatusCode(StatusCode, Response),
    /// The `Last-Event-ID` cannot be formed into a Header to be submitted to the server
    #[error("Invalid `Last-Event-ID`: {0}")]
    InvalidLastEventId(String),
    /// The stream ended
    #[error("Stream ended")]
    StreamEnded,
}

impl From<StreamError<HttpError>> for Error {
    fn from(err: StreamError<HttpError>) -> Self {
        match err {
            StreamError::Utf8(err) => Self::Utf8(err),
            StreamError::Parser(err) => Self::Parser(err),
            StreamError::Transport(err) => Self::Transport(err),
        }
    }
}

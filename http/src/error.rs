use http::StatusCode;
use url::Url;
use wasip3::http::types::ErrorCode;

/// An error produced while building, sending, or consuming an HTTP request.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The request could not be constructed.
    #[error("invalid HTTP request: {0}")]
    Builder(String),

    /// The WASI HTTP client rejected or failed the request.
    #[error("WASI HTTP request failed: {0}")]
    Request(#[source] ErrorCode),

    /// The request body could not be transmitted.
    #[error("HTTP request body transmission failed: {0}")]
    RequestBody(#[source] wasip3::http_compat::Error),

    /// The WASI response could not be converted to an HTTP response.
    #[error("invalid WASI HTTP response: {0}")]
    Response(#[source] ErrorCode),

    /// The response body stream failed.
    #[error("HTTP response body failed: {0}")]
    ResponseBody(#[source] ErrorCode),

    /// The response body could not be decoded as the requested representation.
    #[error("failed to decode HTTP response: {0}")]
    Decode(String),

    /// The server returned a client or server error status.
    #[error("HTTP request failed with status {status} for {url}")]
    Status {
        /// The response status.
        status: StatusCode,
        /// The request URL.
        url: Url,
    },

    /// The P3 host transport was invoked outside WebAssembly.
    #[error("WASI P3 HTTP transport is only available on wasm32 targets")]
    UnsupportedPlatform,
}

impl Error {
    pub(crate) fn builder(error: impl std::fmt::Display) -> Self {
        Self::Builder(error.to_string())
    }

    /// Returns the HTTP status associated with an error created by
    /// [`crate::Response::error_for_status`].
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Self::Status { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Returns whether this error is a timeout reported by the WASI HTTP host.
    pub fn is_timeout(&self) -> bool {
        let code = match self {
            Self::Request(code) | Self::Response(code) | Self::ResponseBody(code) => code,
            _ => return false,
        };

        matches!(
            code,
            ErrorCode::DnsTimeout
                | ErrorCode::ConnectionTimeout
                | ErrorCode::ConnectionReadTimeout
                | ErrorCode::ConnectionWriteTimeout
                | ErrorCode::HttpResponseTimeout
        )
    }

    /// Returns whether this error happened while transmitting a request.
    ///
    /// Request-construction failures remain separately identifiable as
    /// [`Error::Builder`].
    pub fn is_request(&self) -> bool {
        matches!(self, Self::Request(_) | Self::RequestBody(_))
    }
}

/// The result type used by this crate.
pub type Result<T> = std::result::Result<T, Error>;

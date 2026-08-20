use bytes::{Bytes, BytesMut};
use http::{HeaderMap, StatusCode};
#[cfg(target_arch = "wasm32")]
use http_body_util::BodyExt as _;
use serde::de::DeserializeOwned;
use std::fmt;
use url::Url;
#[cfg(target_arch = "wasm32")]
use wasip3::http_compat::IncomingResponseBody;

use crate::{Error, Result};

enum BodyInner {
    #[cfg(target_arch = "wasm32")]
    Wasi(IncomingResponseBody),
    Buffered(Option<Bytes>),
}

/// A response body that can be consumed incrementally or buffered.
pub struct ResponseBody {
    inner: BodyInner,
    trailers: Option<HeaderMap>,
}

impl ResponseBody {
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn wasi(body: IncomingResponseBody) -> Self {
        Self {
            inner: BodyInner::Wasi(body),
            trailers: None,
        }
    }

    pub(crate) fn buffered(body: Bytes) -> Self {
        Self {
            inner: BodyInner::Buffered(Some(body)),
            trailers: None,
        }
    }

    /// Reads the next body data chunk.
    ///
    /// HTTP trailers are retained and can be inspected with [`Self::trailers`]
    /// after the stream reaches its end.
    pub async fn chunk(&mut self) -> Result<Option<Bytes>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let BodyInner::Buffered(body) = &mut self.inner;
            Ok(body.take().filter(|body| !body.is_empty()))
        }

        #[cfg(target_arch = "wasm32")]
        {
            loop {
                match &mut self.inner {
                    BodyInner::Buffered(body) => {
                        return Ok(body.take().filter(|body| !body.is_empty()));
                    }
                    BodyInner::Wasi(body) => {
                        let Some(frame) = body.frame().await else {
                            return Ok(None);
                        };
                        let frame = frame.map_err(Error::ResponseBody)?;
                        match frame.into_data() {
                            Ok(data) => return Ok(Some(data)),
                            Err(frame) => {
                                if let Ok(trailers) = frame.into_trailers() {
                                    self.trailers = Some(trailers);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Returns response trailers after the body stream has yielded its final frame.
    pub fn trailers(&self) -> Option<&HeaderMap> {
        self.trailers.as_ref()
    }

    async fn collect(mut self) -> Result<Bytes> {
        let mut result = BytesMut::new();
        while let Some(chunk) = self.chunk().await? {
            result.extend_from_slice(&chunk);
        }
        Ok(result.freeze())
    }
}

/// An HTTP response returned by [`crate::Client`].
pub struct Response {
    inner: http::Response<ResponseBody>,
    url: Url,
}

impl fmt::Debug for Response {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Response")
            .field("status", &self.status())
            .field("headers", self.headers())
            .field("url", self.url())
            .finish_non_exhaustive()
    }
}

impl Response {
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn from_wasi(response: wasip3::http::types::Response, url: Url) -> Result<Self> {
        let response =
            wasip3::http_compat::http_from_wasi_response(response).map_err(Error::Response)?;
        let (parts, body) = response.into_parts();
        Ok(Self {
            inner: http::Response::from_parts(parts, ResponseBody::wasi(body)),
            url,
        })
    }

    /// Constructs an in-memory response. This is primarily useful for transport mocks.
    pub fn from_bytes(status: StatusCode, headers: HeaderMap, body: Bytes, url: Url) -> Self {
        let mut response = http::Response::new(ResponseBody::buffered(body));
        *response.status_mut() = status;
        *response.headers_mut() = headers;
        Self {
            inner: response,
            url,
        }
    }

    /// Returns the response status.
    pub fn status(&self) -> StatusCode {
        self.inner.status()
    }

    /// Returns the response headers.
    pub fn headers(&self) -> &HeaderMap {
        self.inner.headers()
    }

    /// Returns the URL used for this request.
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Returns a mutable streaming response body.
    pub fn body_mut(&mut self) -> &mut ResponseBody {
        self.inner.body_mut()
    }

    /// Splits the response into its metadata and streaming body.
    pub fn into_body(self) -> ResponseBody {
        self.inner.into_body()
    }

    /// Converts 4xx and 5xx statuses into [`Error::Status`].
    pub fn error_for_status(self) -> Result<Self> {
        if self.status().is_client_error() || self.status().is_server_error() {
            Err(Error::Status {
                status: self.status(),
                url: self.url,
            })
        } else {
            Ok(self)
        }
    }

    /// Buffers the full response body.
    pub async fn bytes(self) -> Result<Bytes> {
        self.inner.into_body().collect().await
    }

    /// Buffers and decodes the response body as UTF-8 text.
    pub async fn text(self) -> Result<String> {
        String::from_utf8(self.bytes().await?.to_vec())
            .map_err(|error| Error::Decode(error.to_string()))
    }

    /// Buffers and deserializes the response body as JSON.
    pub async fn json<T: DeserializeOwned>(self) -> Result<T> {
        serde_json::from_slice(&self.bytes().await?)
            .map_err(|error| Error::Decode(error.to_string()))
    }
}

use std::time::Duration;

use bytes::{Bytes, BytesMut};
use derive_more::From;
use http::{Request, Response};
use wstd::{
    http::{
        error::WasiHttpErrorCode::{
            ConnectionLimitReached, ConnectionReadTimeout, ConnectionRefused, ConnectionTerminated,
            ConnectionTimeout, ConnectionWriteTimeout, TlsCertificateError,
        },
        Body, Client,
    },
    io::AsyncRead,
};

use crate::{
    retry::{Retry, RetryConfig},
    runtime::WasiAsyncRuntime,
};

#[allow(unused)]
#[derive(Debug, From)]
pub enum Error {
    #[from]
    HttpError(http::Error),
    #[from]
    WstdHttpError(wstd::http::Error),
    Generic(String),
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}

#[allow(async_fn_in_trait)]
pub trait HttpClient {
    async fn execute(&self, request: Request<Bytes>) -> Result<Response<Vec<u8>>, Error>;
}

pub struct WstdHttpClient {
    client: Client,
    retry: Retry<WasiAsyncRuntime>,
}

impl WstdHttpClient {
    pub fn new() -> Self {
        let max_retries = std::env::var("STT_PROVIDER_MAX_RETRIES")
            .ok()
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or(10);

        let retry_config = RetryConfig::new()
            .with_max_attempts(max_retries)
            .with_min_delay(Duration::from_millis(500))
            .with_max_delay(Duration::from_secs(10)); // until https://github.com/golemcloud/golem/issues/1848 is fixed this should not be configurable

        Self {
            client: Client::new(),
            retry: Retry::new(retry_config, WasiAsyncRuntime::new()),
        }
    }

    pub fn new_with_timeout(connection_timeout: Duration, first_byte_timeout: Duration) -> Self {
        let mut client = Client::new();
        client.set_connect_timeout(connection_timeout);
        client.set_first_byte_timeout(first_byte_timeout);

        let max_retries = std::env::var("STT_PROVIDER_MAX_RETRIES")
            .ok()
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or(10);

        let retry_config = RetryConfig::new()
            .with_max_attempts(max_retries)
            .with_min_delay(Duration::from_millis(500))
            .with_max_delay(Duration::from_secs(10)); // until https://github.com/golemcloud/golem/issues/1848 is fixed this should not be configurable

        Self {
            client,
            retry: Retry::new(retry_config, WasiAsyncRuntime::new()),
        }
    }

    fn should_retry_wstd_result(
        result: &Result<wstd::http::Response<wstd::http::body::IncomingBody>, wstd::http::Error>,
    ) -> bool {
        match result {
            Err(wstd_error) => Self::is_retryable_wstd_error(wstd_error),
            Ok(response) => Self::is_retryable_status_code(response.status()),
        }
    }

    fn is_retryable_wstd_error(error: &wstd::http::Error) -> bool {
        use wstd::http::body::ErrorVariant;

        matches!(
            error.variant(),
            ErrorVariant::WasiHttp(ConnectionLimitReached)
                | ErrorVariant::WasiHttp(ConnectionReadTimeout)
                | ErrorVariant::WasiHttp(ConnectionWriteTimeout)
                | ErrorVariant::WasiHttp(ConnectionTimeout)
                | ErrorVariant::WasiHttp(ConnectionTerminated)
                | ErrorVariant::WasiHttp(ConnectionRefused)
                | ErrorVariant::WasiHttp(TlsCertificateError)
                | ErrorVariant::BodyIo(_)
        )
    }

    fn is_retryable_status_code(status: http::StatusCode) -> bool {
        matches!(status.as_u16(), 429 | 500..=599)
    }
}

impl Default for WstdHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

struct WasiRequest<T: Body>(wstd::http::Request<T>);

#[derive(Debug)]
struct BytesCursor {
    cursor: wstd::io::Cursor<Bytes>,
}

impl BytesCursor {
    fn new(bytes: Bytes) -> Self {
        Self {
            cursor: wstd::io::Cursor::new(bytes),
        }
    }
}

impl AsyncRead for BytesCursor {
    async fn read(&mut self, buf: &mut [u8]) -> wstd::io::Result<usize> {
        self.cursor.read(buf).await
    }

    async fn read_to_end(&mut self, buf: &mut Vec<u8>) -> wstd::io::Result<usize> {
        self.cursor.read_to_end(buf).await
    }
}

impl Body for BytesCursor {
    fn len(&self) -> Option<usize> {
        Some(self.cursor.get_ref().len())
    }
}

impl Clone for BytesCursor {
    fn clone(&self) -> Self {
        Self::new(self.cursor.get_ref().clone())
    }
}

impl From<Request<Bytes>> for WasiRequest<BytesCursor> {
    fn from(request: Request<Bytes>) -> Self {
        let (parts, body) = request.into_parts();

        let cursor_body = BytesCursor::new(body);

        let mut req = wstd::http::Request::builder()
            .uri(parts.uri)
            .method(parts.method)
            .version(parts.version)
            .body(cursor_body)
            .expect("Known valid");

        *req.headers_mut() = parts.headers;

        WasiRequest(req)
    }
}

impl HttpClient for WstdHttpClient {
    async fn execute(&self, request: Request<Bytes>) -> Result<Response<Vec<u8>>, Error> {
        let wasi_request = WasiRequest::from(request).0;

        let wasi_response = self
            .retry
            .retry_when(Self::should_retry_wstd_result, || async {
                self.client.send(wasi_request.clone()).await
            })
            .await?;

        let mut wasi_response = wasi_response;

        let status = wasi_response.status();
        let headers = wasi_response.headers().clone();
        let body = wasi_response.body_mut().bytes().await?;

        let mut response = Response::builder().status(status).body(body)?;
        *response.headers_mut() = headers;

        Ok(response)
    }
}

pub struct MultipartBuilder {
    boundary: String,
    buffer: BytesMut,
}

impl MultipartBuilder {
    pub fn new() -> Self {
        Self {
            boundary: format!("----formdata-{}", uuid::Uuid::new_v4()),
            buffer: BytesMut::new(),
        }
    }

    pub fn new_with_capacity(estimated_size: usize) -> Self {
        Self {
            boundary: format!("----formdata-{}", uuid::Uuid::new_v4()),
            buffer: BytesMut::with_capacity(estimated_size),
        }
    }

    pub fn add_bytes(&mut self, name: &str, filename: &str, content_type: &str, data: &[u8]) {
        let header = format!(
            "--{}\r\nContent-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: {}\r\n\r\n",
            self.boundary, name, filename, content_type
        );
        self.buffer.extend_from_slice(header.as_bytes());
        self.buffer.extend_from_slice(data);
        self.buffer.extend_from_slice(b"\r\n");
    }

    pub fn add_field(&mut self, name: &str, value: &str) {
        let field = format!(
            "--{}\r\nContent-Disposition: form-data; name=\"{}\"\r\n\r\n{}\r\n",
            self.boundary, name, value
        );
        self.buffer.extend_from_slice(field.as_bytes());
    }

    pub fn finish(mut self) -> (String, Bytes) {
        let end_boundary = format!("--{}--\r\n", self.boundary);
        self.buffer.extend_from_slice(end_boundary.as_bytes());

        let content_type = format!("multipart/form-data; boundary={}", self.boundary);
        (content_type, self.buffer.freeze())
    }
}

impl Default for MultipartBuilder {
    fn default() -> Self {
        Self::new()
    }
}

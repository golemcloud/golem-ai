use std::time::Duration;

use bytes::{Bytes, BytesMut};
use http::{Request, Response};
use wstd::http::{
    error::ErrorCode, Client,
};

use crate::{
    retry::{Retry, RetryConfig},
    runtime::WasiAsyncRuntime,
};

#[allow(unused)]
pub enum Error {
    HttpError(http::Error),
    WstdHttpError(wstd::http::error::Error),
    Generic(String),
}

impl core::fmt::Debug for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::HttpError(e) => write!(fmt, "HttpError({e:?})"),
            Error::WstdHttpError(e) => write!(fmt, "WstdHttpError({e:?})"),
            Error::Generic(e) => write!(fmt, "Generic({e:?})"),
        }
    }
}

impl From<http::Error> for Error {
    fn from(err: http::Error) -> Self {
        Error::HttpError(err)
    }
}

impl From<wstd::http::error::Error> for Error {
    fn from(err: wstd::http::error::Error) -> Self {
        Error::WstdHttpError(err)
    }
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
        result: &Result<Response<wstd::http::Body>, wstd::http::error::Error>,
    ) -> bool {
        match result {
            Err(wstd_error) => Self::is_retryable_wstd_error(wstd_error),
            Ok(response) => Self::is_retryable_status_code(response.status()),
        }
    }

    fn is_retryable_wstd_error(error: &wstd::http::error::Error) -> bool {
        if let Some(error_code) = error.downcast_ref::<ErrorCode>() {
            matches!(
                error_code,
                ErrorCode::ConnectionLimitReached
                    | ErrorCode::ConnectionReadTimeout
                    | ErrorCode::ConnectionWriteTimeout
                    | ErrorCode::ConnectionTimeout
                    | ErrorCode::ConnectionTerminated
                    | ErrorCode::ConnectionRefused
                    | ErrorCode::TlsCertificateError
            )
        } else {
            true
        }
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

fn to_wstd_request(request: Request<Bytes>) -> Request<Vec<u8>> {
    let (parts, body) = request.into_parts();
    Request::from_parts(parts, body.to_vec())
}

impl HttpClient for WstdHttpClient {
    async fn execute(&self, request: Request<Bytes>) -> Result<Response<Vec<u8>>, Error> {
        let wstd_request = to_wstd_request(request);

        let mut wasi_response = self
            .retry
            .retry_when(Self::should_retry_wstd_result, || async {
                self.client.send(wstd_request.clone()).await
            })
            .await?;

        let status = wasi_response.status();
        let headers = wasi_response.headers().clone();
        let body_bytes = wasi_response.body_mut().contents().await?;

        let mut response = Response::builder().status(status).body(body_bytes.to_vec())?;
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

use std::sync::Arc;

use aws_sdk_bedrockruntime::{config, error::ConnectorError};
use aws_smithy_runtime_api::{
    client::http::{
        HttpConnector, HttpConnectorFuture, HttpConnectorSettings, SharedHttpConnector,
    },
    http::{Headers, Response, StatusCode},
};
use aws_smithy_types::body::SdkBody;
use reqwest::Method;
use wstd::http;

use crate::async_utils::UnsafeFuture;

#[derive(Debug)]
pub struct WasiClient;

impl WasiClient {
    pub fn new() -> Self {
        Self
    }
}

impl config::HttpClient for WasiClient {
    fn http_connector(
        &self,
        settings: &HttpConnectorSettings,
        _components: &config::RuntimeComponents,
    ) -> SharedHttpConnector {
        let mut client = http::Client::new();

        if let Some(conn_timeout) = settings.connect_timeout() {
            client.set_connect_timeout(conn_timeout);
        }
        if let Some(read_timeout) = settings.read_timeout() {
            client.set_first_byte_timeout(read_timeout);
        }
        let connector = SharedWasiConnector::new(client);
        SharedHttpConnector::new(connector)
    }
}

unsafe impl Send for WasiClient {}
unsafe impl Sync for WasiClient {}

#[derive(Debug)]
struct SharedWasiConnector {
    inner: Arc<WasiConnector>,
}

impl SharedWasiConnector {
    fn new(client: http::Client) -> Self {
        Self {
            inner: Arc::new(WasiConnector(client)),
        }
    }
}

#[derive(Debug)]
struct WasiConnector(http::Client);

unsafe impl Send for WasiConnector {}
unsafe impl Sync for WasiConnector {}

impl WasiConnector {
    async fn handle(
        &self,
        request: config::http::HttpRequest,
    ) -> Result<http::Response<http::body::IncomingBody>, ConnectorError> {
        let method = Method::from_bytes(request.method().as_bytes()).expect("Valid http method");
        let url = request.uri().to_owned();
        let parts = request.into_parts();

        let mut request = http::Request::builder().uri(url).method(method);

        for header in parts.headers.iter() {
            request = request.header(header.0, header.1);
        }

        let request = request
            .body(BodyReader::new(parts.body))
            .expect("Valid request should be formed");

        self.0
            .send(request)
            .await
            .map_err(|e| ConnectorError::other(e.into(), None))
    }
}

impl HttpConnector for SharedWasiConnector {
    fn call(&self, request: config::http::HttpRequest) -> HttpConnectorFuture {
        let inner_clone = Arc::clone(&self.inner);

        let future = async move {
            let response = inner_clone.handle(request).await?;
            log::trace!("WasiConnector: response received {response:?}");

            let status_code: StatusCode = response.status().into();
            let headers_map = response.headers().clone();
            let extensions = response.extensions().clone();

            let body = response
                .into_body()
                .bytes()
                .await
                .map(|body| {
                    if body.is_empty() {
                        SdkBody::empty()
                    } else {
                        SdkBody::from(body)
                    }
                })
                .map_err(|e| ConnectorError::other(e.into(), None))?;

            let mut headers = Headers::new();
            for header in headers_map {
                if let Some(key) = header.0 {
                    if let Ok(value) = header.1.to_str() {
                        headers.insert(key.to_string(), value.to_string());
                    }
                }
            }

            let mut sdk_response = Response::new(status_code, body);
            *sdk_response.headers_mut() = headers;
            sdk_response.add_extension(extensions);

            Ok(sdk_response)
        };

        HttpConnectorFuture::new(UnsafeFuture::new(future))
    }
}

struct BodyReader {
    body: SdkBody,
    position: usize,
}

impl From<SdkBody> for BodyReader {
    fn from(value: SdkBody) -> Self {
        Self::new(value)
    }
}

impl BodyReader {
    fn new(body: SdkBody) -> Self {
        Self { body, position: 0 }
    }
}

impl http::Body for BodyReader {
    fn len(&self) -> Option<usize> {
        let body_bytes = self.body.bytes();

        match body_bytes {
            Some(bytes) => {
                let total_length = bytes.len();

                Some(total_length - self.position)
            }
            None => None,
        }
    }
}

impl wstd::io::AsyncRead for BodyReader {
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let body_bytes = self.body.bytes();

        match body_bytes {
            Some(bytes) => {
                if self.position >= bytes.len() {
                    return Ok(0); // EOF
                }
                let remaining = &bytes[self.position..];
                let amt = std::cmp::min(buf.len(), remaining.len());
                buf[..amt].copy_from_slice(&remaining[..amt]);
                self.position += amt;
                Ok(amt)
            }
            None => Ok(0),
        }
    }
}

use std::sync::Arc;

use aws_sdk_bedrockruntime::{config, error::ConnectorError};
use aws_smithy_runtime_api::{
    client::http::{
        HttpConnector, HttpConnectorFuture, HttpConnectorSettings, SharedHttpConnector,
    },
    http::{Headers, Response, StatusCode},
};
use aws_smithy_types::body::SdkBody;
use wstd::http::{self, Body, Method};

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
    ) -> Result<http::Response<Body>, ConnectorError> {
        let method = Method::from_bytes(request.method().as_bytes()).expect("Valid http method");
        let url = request.uri().to_owned();
        let parts = request.into_parts();

        let body_bytes = sdk_body_to_vec(parts.body);

        let mut request = http::Request::builder().uri(url).method(method);

        for header in parts.headers.iter() {
            request = request.header(header.0, header.1);
        }

        let request = request
            .body(body_bytes)
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

            let body_bytes = response
                .into_body()
                .contents()
                .await
                .map(|body| {
                    if body.is_empty() {
                        SdkBody::empty()
                    } else {
                        SdkBody::from(body.to_vec())
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

            let mut sdk_response = Response::new(status_code, body_bytes);
            *sdk_response.headers_mut() = headers;
            sdk_response.add_extension(extensions);

            Ok(sdk_response)
        };

        HttpConnectorFuture::new(UnsafeFuture::new(future))
    }
}

fn sdk_body_to_vec(body: SdkBody) -> Vec<u8> {
    body.bytes().map(|b| b.to_vec()).unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
use std::future::IntoFuture as _;
use std::{fmt, time::Duration};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use bytes::Bytes;
use http::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT},
    HeaderMap, HeaderName, HeaderValue, Method, Request,
};
#[cfg(target_arch = "wasm32")]
use http_body_util::Full;
use serde::Serialize;
use url::Url;
#[cfg(target_arch = "wasm32")]
use wasip3::http::types;

use crate::{Error, Response, Result};

/// Per-phase timeouts supported by WASI HTTP 0.3.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Timeouts {
    /// Maximum time spent establishing a connection.
    pub connect: Option<Duration>,
    /// Maximum time from finishing the request until receiving response headers.
    pub first_byte: Option<Duration>,
    /// Maximum time allowed between consecutive response-body bytes.
    pub between_bytes: Option<Duration>,
}

impl Timeouts {
    /// Creates timeout options with every phase left to the host default.
    pub const fn new() -> Self {
        Self {
            connect: None,
            first_byte: None,
            between_bytes: None,
        }
    }

    /// Sets the connection timeout.
    pub const fn connect(mut self, timeout: Duration) -> Self {
        self.connect = Some(timeout);
        self
    }

    /// Sets the first-byte timeout.
    pub const fn first_byte(mut self, timeout: Duration) -> Self {
        self.first_byte = Some(timeout);
        self
    }

    /// Sets the between-bytes timeout.
    pub const fn between_bytes(mut self, timeout: Duration) -> Self {
        self.between_bytes = Some(timeout);
        self
    }

    fn validate(self) -> Result<()> {
        for (name, timeout) in [
            ("connect", self.connect),
            ("first-byte", self.first_byte),
            ("between-bytes", self.between_bytes),
        ] {
            if let Some(timeout) = timeout {
                duration_nanos(timeout).map_err(|_| {
                    Error::builder(format!("{name} timeout exceeds u64 nanoseconds"))
                })?;
            }
        }
        Ok(())
    }
}

/// An asynchronous WASI HTTP 0.3 client.
#[derive(Clone, Debug)]
pub struct Client {
    default_headers: HeaderMap,
    timeouts: Timeouts,
}

impl Client {
    /// Creates a client with the default configuration.
    pub fn new() -> Self {
        Self::builder()
            .build()
            .expect("the default HTTP client configuration is valid")
    }

    /// Creates a client builder.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Starts building a request.
    pub fn request<U: AsRef<str>>(&self, method: Method, url: U) -> RequestBuilder {
        RequestBuilder::new(self.clone(), method, url.as_ref())
    }

    /// Starts building a GET request.
    pub fn get<U: AsRef<str>>(&self, url: U) -> RequestBuilder {
        self.request(Method::GET, url)
    }

    /// Starts building a POST request.
    pub fn post<U: AsRef<str>>(&self, url: U) -> RequestBuilder {
        self.request(Method::POST, url)
    }

    /// Starts building a PUT request.
    pub fn put<U: AsRef<str>>(&self, url: U) -> RequestBuilder {
        self.request(Method::PUT, url)
    }

    /// Starts building a PATCH request.
    pub fn patch<U: AsRef<str>>(&self, url: U) -> RequestBuilder {
        self.request(Method::PATCH, url)
    }

    /// Starts building a DELETE request.
    pub fn delete<U: AsRef<str>>(&self, url: U) -> RequestBuilder {
        self.request(Method::DELETE, url)
    }

    /// Sends an already-built request.
    ///
    /// The response body remains streaming; use [`Response::bytes`],
    /// [`Response::text`], or [`Response::json`] to buffer it.
    pub async fn execute(&self, request: Request<Bytes>) -> Result<Response> {
        let request = self.prepare(request);
        let url = Url::parse(&request.uri().to_string()).map_err(Error::builder)?;
        request
            .extensions()
            .get::<Timeouts>()
            .copied()
            .unwrap_or_default()
            .validate()?;

        #[cfg(target_arch = "wasm32")]
        {
            execute_wasi(request, url).await
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = request;
            let _ = url;
            Err(Error::UnsupportedPlatform)
        }
    }

    fn prepare(&self, mut request: Request<Bytes>) -> Request<Bytes> {
        for name in self.default_headers.keys() {
            if !request.headers().contains_key(name) {
                for value in self.default_headers.get_all(name) {
                    request.headers_mut().append(name, value.clone());
                }
            }
        }
        if request.extensions().get::<Timeouts>().is_none() {
            request.extensions_mut().insert(self.timeouts);
        }
        request
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds a [`Client`].
#[derive(Debug)]
pub struct ClientBuilder {
    default_headers: HeaderMap,
    timeouts: Timeouts,
    error: Option<Error>,
}

impl ClientBuilder {
    /// Creates a builder with `Accept: */*` and host-default timeouts.
    pub fn new() -> Self {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        Self {
            default_headers,
            timeouts: Timeouts::new(),
            error: None,
        }
    }

    /// Sets the default `User-Agent` header.
    pub fn user_agent<V>(mut self, value: V) -> Self
    where
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: fmt::Display,
    {
        match HeaderValue::try_from(value) {
            Ok(value) => {
                self.default_headers.insert(USER_AGENT, value);
            }
            Err(error) => self.error = Some(Error::builder(error)),
        }
        self
    }

    /// Adds default request headers. New values replace defaults with the same name.
    pub fn default_headers(mut self, headers: HeaderMap) -> Self {
        replace_headers(&mut self.default_headers, headers);
        self
    }

    /// Sets all per-phase timeout options.
    pub fn timeouts(mut self, timeouts: Timeouts) -> Self {
        self.timeouts = timeouts;
        self
    }

    /// Sets the connection timeout.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.timeouts.connect = Some(timeout);
        self
    }

    /// Sets the first-byte timeout.
    pub fn first_byte_timeout(mut self, timeout: Duration) -> Self {
        self.timeouts.first_byte = Some(timeout);
        self
    }

    /// Sets the between-bytes timeout.
    pub fn between_bytes_timeout(mut self, timeout: Duration) -> Self {
        self.timeouts.between_bytes = Some(timeout);
        self
    }

    /// Builds the client.
    pub fn build(self) -> Result<Client> {
        if let Some(error) = self.error {
            return Err(error);
        }
        self.timeouts.validate()?;
        Ok(Client {
            default_headers: self.default_headers,
            timeouts: self.timeouts,
        })
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds and sends one HTTP request.
pub struct RequestBuilder {
    client: Client,
    method: Method,
    url: Option<Url>,
    headers: HeaderMap,
    body: Bytes,
    timeouts: Option<Timeouts>,
    error: Option<Error>,
}

impl RequestBuilder {
    fn new(client: Client, method: Method, url: &str) -> Self {
        match Url::parse(url) {
            Ok(url) => Self {
                client,
                method,
                url: Some(url),
                headers: HeaderMap::new(),
                body: Bytes::new(),
                timeouts: None,
                error: None,
            },
            Err(error) => Self {
                client,
                method,
                url: None,
                headers: HeaderMap::new(),
                body: Bytes::new(),
                timeouts: None,
                error: Some(Error::builder(error)),
            },
        }
    }

    /// Appends a request header.
    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: fmt::Display,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: fmt::Display,
    {
        if self.error.is_none() {
            match (HeaderName::try_from(key), HeaderValue::try_from(value)) {
                (Ok(key), Ok(value)) => {
                    self.headers.append(key, value);
                }
                (Err(error), _) => self.error = Some(Error::builder(error)),
                (_, Err(error)) => self.error = Some(Error::builder(error)),
            }
        }
        self
    }

    /// Adds request headers. New values replace existing values with the same name.
    pub fn headers(mut self, headers: HeaderMap) -> Self {
        replace_headers(&mut self.headers, headers);
        self
    }

    /// Enables HTTP Basic authentication.
    pub fn basic_auth<U, P>(self, username: U, password: Option<P>) -> Self
    where
        U: fmt::Display,
        P: fmt::Display,
    {
        let credentials = match password {
            Some(password) => format!("{username}:{password}"),
            None => format!("{username}:"),
        };
        let mut value = HeaderValue::from_str(&format!("Basic {}", STANDARD.encode(credentials)))
            .expect("base64 credentials always form a valid header value");
        value.set_sensitive(true);
        self.header(AUTHORIZATION, value)
    }

    /// Enables HTTP Bearer authentication.
    pub fn bearer_auth<T: fmt::Display>(mut self, token: T) -> Self {
        if self.error.is_none() {
            match HeaderValue::from_str(&format!("Bearer {token}")) {
                Ok(mut value) => {
                    value.set_sensitive(true);
                    self.headers.insert(AUTHORIZATION, value);
                }
                Err(error) => self.error = Some(Error::builder(error)),
            }
        }
        self
    }

    /// Appends serialized query parameters to the URL.
    pub fn query<T: Serialize + ?Sized>(mut self, query: &T) -> Self {
        if self.error.is_none() {
            let url = self.url.as_mut().expect("a valid builder has a URL");
            let mut pairs = url.query_pairs_mut();
            let serializer = serde_urlencoded::Serializer::new(&mut pairs);
            if let Err(error) = query.serialize(serializer) {
                self.error = Some(Error::builder(error));
            }
            drop(pairs);
            if url.query() == Some("") {
                url.set_query(None);
            }
        }
        self
    }

    /// Sets a buffered request body.
    pub fn body<T: Into<Bytes>>(mut self, body: T) -> Self {
        self.body = body.into();
        self
    }

    /// Serializes a URL-encoded form request body.
    pub fn form<T: Serialize + ?Sized>(mut self, form: &T) -> Self {
        if self.error.is_none() {
            match serde_urlencoded::to_string(form) {
                Ok(body) => {
                    self.headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_static("application/x-www-form-urlencoded"),
                    );
                    self.body = body.into();
                }
                Err(error) => self.error = Some(Error::builder(error)),
            }
        }
        self
    }

    /// Serializes a JSON request body.
    pub fn json<T: Serialize + ?Sized>(mut self, value: &T) -> Self {
        if self.error.is_none() {
            match serde_json::to_vec(value) {
                Ok(body) => {
                    if !self.headers.contains_key(CONTENT_TYPE) {
                        self.headers
                            .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                    }
                    self.body = body.into();
                }
                Err(error) => self.error = Some(Error::builder(error)),
            }
        }
        self
    }

    /// Overrides the client's timeout options for this request.
    pub fn timeouts(mut self, timeouts: Timeouts) -> Self {
        self.timeouts = Some(timeouts);
        self
    }

    /// Builds an `http` request without invoking the WASI host.
    pub fn build(self) -> Result<Request<Bytes>> {
        let (client, request) = self.build_unprepared()?;
        Ok(client.prepare(request))
    }

    /// Builds and asynchronously sends this request.
    pub async fn send(self) -> Result<Response> {
        let (client, request) = self.build_unprepared()?;
        client.execute(request).await
    }

    fn build_unprepared(self) -> Result<(Client, Request<Bytes>)> {
        if let Some(error) = self.error {
            return Err(error);
        }
        let url = self.url.expect("a valid builder has a URL");
        if !matches!(url.scheme(), "http" | "https") {
            return Err(Error::builder("URL scheme must be http or https"));
        }
        if let Some(timeouts) = self.timeouts {
            timeouts.validate()?;
        }
        let mut request = Request::builder()
            .method(self.method)
            .uri(url.as_str())
            .body(self.body)
            .map_err(Error::builder)?;
        *request.headers_mut() = self.headers;
        if let Some(timeouts) = self.timeouts {
            request.extensions_mut().insert(timeouts);
        }
        Ok((self.client, request))
    }
}

fn replace_headers(destination: &mut HeaderMap, source: HeaderMap) {
    for name in source.keys() {
        destination.remove(name);
    }
    destination.extend(source);
}

#[cfg(target_arch = "wasm32")]
async fn execute_wasi(request: Request<Bytes>, url: Url) -> Result<Response> {
    use wasip3::{http::client, http_compat::BodyWriter, wit_future};

    let (parts, body) = request.into_parts();
    let timeouts = parts
        .extensions
        .get::<Timeouts>()
        .copied()
        .unwrap_or_default();
    let options = request_options(timeouts)?;
    let headers = types::Fields::try_from(parts.headers).map_err(Error::builder)?;

    if body.is_empty() {
        let (trailers_writer, trailers_reader) = wit_future::new::<
            std::result::Result<Option<types::Trailers>, types::ErrorCode>,
        >(|| Ok(None));
        let (request, transmitted) = types::Request::new(headers, None, trailers_reader, options);
        configure_request(&request, &parts.method, &parts.uri)?;

        let finish_body = async move {
            trailers_writer.write(Ok(None)).await.map_err(|_| {
                Error::Request(types::ErrorCode::InternalError(Some(
                    "request trailers receiver closed".to_string(),
                )))
            })
        };
        let (response, transmitted, body_result) = futures::join!(
            client::send(request),
            transmitted.into_future(),
            finish_body
        );
        body_result?;
        transmitted.map_err(Error::Request)?;
        Response::from_wasi(response.map_err(Error::Request)?, url)
    } else {
        let (body_writer, body_reader, trailers_reader) = BodyWriter::new();
        let (request, transmitted) =
            types::Request::new(headers, Some(body_reader), trailers_reader, options);
        configure_request(&request, &parts.method, &parts.uri)?;

        let mut body = Full::new(body);
        let (response, transmitted, body_result) = futures::join!(
            client::send(request),
            transmitted.into_future(),
            body_writer.send_http_body(&mut body)
        );
        body_result.map_err(Error::RequestBody)?;
        transmitted.map_err(Error::Request)?;
        Response::from_wasi(response.map_err(Error::Request)?, url)
    }
}

#[cfg(target_arch = "wasm32")]
fn configure_request(request: &types::Request, method: &Method, uri: &http::Uri) -> Result<()> {
    request
        .set_method(&method.into())
        .map_err(|()| Error::Request(types::ErrorCode::HttpRequestMethodInvalid))?;
    let scheme = uri.scheme().map(Into::into);
    request
        .set_scheme(scheme.as_ref())
        .map_err(|()| Error::Request(types::ErrorCode::HttpProtocolError))?;
    request
        .set_authority(uri.authority().map(|authority| authority.as_str()))
        .map_err(|()| Error::Request(types::ErrorCode::HttpRequestUriInvalid))?;
    request
        .set_path_with_query(uri.path_and_query().map(|path| path.as_str()))
        .map_err(|()| Error::Request(types::ErrorCode::HttpRequestUriInvalid))?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn request_options(timeouts: Timeouts) -> Result<Option<types::RequestOptions>> {
    if timeouts == Timeouts::default() {
        return Ok(None);
    }

    let options = types::RequestOptions::new();
    if let Some(timeout) = timeouts.connect {
        options
            .set_connect_timeout(Some(duration_nanos(timeout)?))
            .map_err(|error| Error::builder(format!("invalid connect timeout: {error}")))?;
    }
    if let Some(timeout) = timeouts.first_byte {
        options
            .set_first_byte_timeout(Some(duration_nanos(timeout)?))
            .map_err(|error| Error::builder(format!("invalid first-byte timeout: {error}")))?;
    }
    if let Some(timeout) = timeouts.between_bytes {
        options
            .set_between_bytes_timeout(Some(duration_nanos(timeout)?))
            .map_err(|error| Error::builder(format!("invalid between-bytes timeout: {error}")))?;
    }
    Ok(Some(options))
}

fn duration_nanos(duration: Duration) -> Result<u64> {
    u64::try_from(duration.as_nanos())
        .map_err(|_| Error::builder("timeout exceeds u64 nanoseconds"))
}

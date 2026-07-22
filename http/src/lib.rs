//! Shared asynchronous HTTP transport for Golem AI crates on WASI 0.3.
//!
//! Request construction is platform-neutral and can be unit-tested natively.
//! Sending uses the `wasi:http@0.3.0` client import on `wasm32-wasip2`, the
//! Rust compilation target used for WASI P3 components.

mod client;
mod error;
mod response;

pub use client::{Client, ClientBuilder, RequestBuilder, Timeouts};
pub use error::{Error, Result};
pub use response::{Response, ResponseBody};

pub use bytes::Bytes;
pub use http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
pub use url::Url;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
    use serde::{Deserialize, Serialize};

    use super::*;

    #[test]
    fn appends_encoded_query_parameters() {
        let request = Client::new()
            .get("https://example.com/models?existing=yes")
            .query(&[("name", "a value"), ("name", "second")])
            .build()
            .unwrap();

        assert_eq!(
            request.uri().to_string(),
            "https://example.com/models?existing=yes&name=a+value&name=second"
        );
    }

    #[test]
    fn request_headers_override_client_defaults() {
        let mut defaults = HeaderMap::new();
        defaults.insert("x-source", HeaderValue::from_static("client"));
        let request = Client::builder()
            .user_agent("golem-ai-test")
            .default_headers(defaults)
            .build()
            .unwrap()
            .get("https://example.com")
            .header("x-source", "request")
            .build()
            .unwrap();

        assert_eq!(request.headers()[ACCEPT], "*/*");
        assert_eq!(request.headers()[USER_AGENT], "golem-ai-test");
        assert_eq!(request.headers()["x-source"], "request");
    }

    #[test]
    fn constructs_basic_and_bearer_auth_headers() {
        let basic = Client::new()
            .get("https://example.com")
            .basic_auth("user", Some("password"))
            .build()
            .unwrap();
        assert_eq!(basic.headers()[AUTHORIZATION], "Basic dXNlcjpwYXNzd29yZA==");
        assert!(basic.headers()[AUTHORIZATION].is_sensitive());

        let bearer = Client::new()
            .get("https://example.com")
            .bearer_auth("secret")
            .build()
            .unwrap();
        assert_eq!(bearer.headers()[AUTHORIZATION], "Bearer secret");
        assert!(bearer.headers()[AUTHORIZATION].is_sensitive());
    }

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Payload {
        prompt: String,
        count: u32,
    }

    #[test]
    fn serializes_json_and_form_bodies() {
        let payload = Payload {
            prompt: "hello world".to_string(),
            count: 2,
        };

        let json = Client::new()
            .post("https://example.com")
            .json(&payload)
            .build()
            .unwrap();
        assert_eq!(json.headers()[CONTENT_TYPE], "application/json");
        assert_eq!(
            serde_json::from_slice::<Payload>(json.body()).unwrap(),
            payload
        );

        let form = Client::new()
            .post("https://example.com")
            .form(&payload)
            .build()
            .unwrap();
        assert_eq!(
            form.headers()[CONTENT_TYPE],
            "application/x-www-form-urlencoded"
        );
        assert_eq!(form.body(), "prompt=hello+world&count=2");
    }

    #[test]
    fn request_timeouts_override_client_timeouts() {
        let client_timeouts = Timeouts::new().connect(Duration::from_secs(1));
        let request_timeouts = Timeouts::new()
            .first_byte(Duration::from_secs(2))
            .between_bytes(Duration::from_secs(3));
        let request = Client::builder()
            .timeouts(client_timeouts)
            .build()
            .unwrap()
            .get("https://example.com")
            .timeouts(request_timeouts)
            .build()
            .unwrap();

        assert_eq!(
            request.extensions().get::<Timeouts>(),
            Some(&request_timeouts)
        );
    }

    #[test]
    fn rejects_timeouts_that_do_not_fit_wasi_duration() {
        let error = Client::builder()
            .connect_timeout(Duration::MAX)
            .build()
            .unwrap_err();
        assert!(matches!(error, Error::Builder(message) if message.contains("connect timeout")));
    }

    #[test]
    fn rejects_non_http_urls() {
        let error = Client::new().get("file:///tmp/model").build().unwrap_err();
        assert!(matches!(error, Error::Builder(_)));
    }

    #[test]
    fn classifies_wasi_timeout_errors() {
        let error = Error::Request(wasip3::http::types::ErrorCode::ConnectionTimeout);
        assert!(error.is_request());
        assert!(error.is_timeout());
    }

    #[test]
    fn classifies_status_errors() {
        let response = Response::from_bytes(
            StatusCode::TOO_MANY_REQUESTS,
            HeaderMap::new(),
            Bytes::new(),
            Url::parse("https://example.com").unwrap(),
        );
        let error = response.error_for_status().unwrap_err();
        assert_eq!(error.status(), Some(StatusCode::TOO_MANY_REQUESTS));
        assert!(!error.is_request());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_execute_does_not_call_wasi_imports() {
        futures::executor::block_on(async {
            let error = Client::new()
                .get("https://example.com")
                .send()
                .await
                .unwrap_err();
            assert!(matches!(error, Error::UnsupportedPlatform));
        });
    }

    #[test]
    fn buffers_and_decodes_mock_responses() {
        futures::executor::block_on(async {
            let payload = Payload {
                prompt: "response".to_string(),
                count: 3,
            };
            let response = Response::from_bytes(
                StatusCode::OK,
                HeaderMap::new(),
                Bytes::from(serde_json::to_vec(&payload).unwrap()),
                Url::parse("https://example.com").unwrap(),
            );

            assert_eq!(response.json::<Payload>().await.unwrap(), payload);
        });
    }
}

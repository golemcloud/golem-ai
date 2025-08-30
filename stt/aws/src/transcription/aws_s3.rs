use bytes::Bytes;
use chrono::Utc;
use golem_stt::{error::Error, http::HttpClient};
use http::{Request, StatusCode};
use log::trace;

use super::aws_signer::AwsSignatureV4;

#[allow(async_fn_in_trait)]
pub trait S3Service {
    async fn put_object(
        &self,
        request_id: &str,
        bucket: &str,
        object_name: &str,
        content: Bytes,
    ) -> Result<(), golem_stt::error::Error>;

    async fn delete_object(
        &self,
        request_id: &str,
        bucket: &str,
        object_name: &str,
    ) -> Result<(), golem_stt::error::Error>;
}

pub struct S3Client<HC: HttpClient> {
    http_client: HC,
    signer: AwsSignatureV4,
}

impl<HC: HttpClient> S3Client<HC> {
    pub fn new(access_key: String, secret_key: String, region: String, http_client: HC) -> Self {
        Self {
            http_client,
            signer: AwsSignatureV4::for_s3(access_key, secret_key, region),
        }
    }
}

impl<HC: HttpClient> S3Service for S3Client<HC> {
    async fn put_object(
        &self,
        request_id: &str,
        bucket: &str,
        object_name: &str,
        content: Bytes,
    ) -> Result<(), golem_stt::error::Error> {
        let timestamp = Utc::now();
        let uri = format!("https://{bucket}.s3.amazonaws.com/{object_name}");

        trace!("uploading file to {uri}");

        let content_length = content.len().to_string();

        let request = Request::builder()
            .method("PUT")
            .uri(&uri)
            .header("Content-Type", "application/octet-stream")
            .header("Content-Length", &content_length)
            .body(content)
            .map_err(|e| {
                Error::Http(request_id.to_string(), golem_stt::http::Error::HttpError(e))
            })?;

        let signed_request = self
            .signer
            .sign_request(request, timestamp)
            .map_err(|err| {
                (
                    request_id.to_string(),
                    golem_stt::http::Error::Generic(format!("Failed to sign request: {err}")),
                )
            })?;

        let response = self
            .http_client
            .execute(signed_request)
            .await
            .map_err(|err| (request_id.to_string(), err))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_body = String::from_utf8(response.body().to_vec())
                .unwrap_or_else(|e| format!("Unknown error, {e}"));

            let status = response.status();
            let request_id = request_id.to_string();

            match status {
                StatusCode::BAD_REQUEST => Err(golem_stt::error::Error::APIBadRequest {
                    request_id,
                    provider_error: format!("S3 PutObject bad request: {error_body}",),
                }),
                s if s.is_server_error() => Err(golem_stt::error::Error::APIInternalServerError {
                    request_id,
                    provider_error: format!("S3 PutObject server error ({status}): {error_body}"),
                }),
                _ => Err(golem_stt::error::Error::APIUnknown {
                    request_id,
                    provider_error: format!(
                        "S3 PutObject unexpected error ({status}): {error_body}",
                    ),
                }),
            }
        }
    }

    async fn delete_object(
        &self,
        request_id: &str,
        bucket: &str,
        object_name: &str,
    ) -> Result<(), golem_stt::error::Error> {
        let timestamp = Utc::now();
        let uri = format!("https://{bucket}.s3.amazonaws.com/{object_name}");

        let request = Request::builder()
            .method("DELETE")
            .uri(&uri)
            .body(Bytes::new())
            .map_err(|e| (request_id.to_string(), golem_stt::http::Error::HttpError(e)))?;

        let signed_request = self
            .signer
            .sign_request(request, timestamp)
            .map_err(|err| {
                (
                    request_id.to_string(),
                    golem_stt::http::Error::Generic(format!("Failed to sign request: {err}")),
                )
            })?;

        let response = self
            .http_client
            .execute(signed_request)
            .await
            .map_err(|err| (request_id.to_string(), err))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_body = String::from_utf8(response.body().to_vec())
                .unwrap_or_else(|e| format!("Unknown error, {e}"));

            let status = response.status();
            let request_id = request_id.to_string();

            match status {
                StatusCode::BAD_REQUEST => Err(golem_stt::error::Error::APIBadRequest {
                    request_id,
                    provider_error: format!("S3 DeleteObject bad request: {error_body}"),
                }),
                s if s.is_server_error() => Err(golem_stt::error::Error::APIInternalServerError {
                    request_id,
                    provider_error: format!(
                        "S3 DeleteObject server error ({status}): {error_body}"
                    ),
                }),
                _ => Err(golem_stt::error::Error::APIUnknown {
                    request_id,
                    provider_error: format!(
                        "S3 DeleteObject unexpected error ({status}): {error_body}"
                    ),
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Ref, RefCell},
        collections::VecDeque,
    };

    use http::{Response, StatusCode};

    use super::*;

    struct MockHttpClient {
        pub responses: RefCell<VecDeque<Result<Response<Vec<u8>>, golem_stt::http::Error>>>,
        pub captured_requests: RefCell<Vec<Request<Bytes>>>,
    }

    #[allow(unused)]
    impl MockHttpClient {
        pub fn new() -> Self {
            Self {
                responses: RefCell::new(VecDeque::new()),
                captured_requests: RefCell::new(Vec::new()),
            }
        }

        pub fn expect_response(&self, response: Response<Vec<u8>>) {
            self.responses.borrow_mut().push_back(Ok(response));
        }

        pub fn get_captured_requests(&self) -> Ref<'_, Vec<Request<Bytes>>> {
            self.captured_requests.borrow()
        }

        pub fn clear_captured_requests(&self) {
            self.captured_requests.borrow_mut().clear();
        }

        pub fn captured_request_count(&self) -> usize {
            self.captured_requests.borrow().len()
        }

        pub fn last_captured_request(&self) -> Option<Ref<'_, Request<Bytes>>> {
            let borrow = self.captured_requests.borrow();
            if borrow.is_empty() {
                None
            } else {
                Some(Ref::map(borrow, |requests| requests.last().unwrap()))
            }
        }
    }

    impl HttpClient for MockHttpClient {
        async fn execute(
            &self,
            request: Request<Bytes>,
        ) -> Result<Response<Vec<u8>>, golem_stt::http::Error> {
            self.captured_requests.borrow_mut().push(request);
            self.responses
                .borrow_mut()
                .pop_front()
                .unwrap_or(Err(golem_stt::http::Error::Generic(
                    "unexpected error".to_string(),
                )))
        }
    }

    #[wstd::test]
    async fn test_s3_put_object_request() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";

        let mock_client = MockHttpClient::new();

        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::OK)
                .body(vec![])
                .unwrap(),
        );

        let s3_client = S3Client::new(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
            mock_client,
        );

        let bucket = "test-bucket";
        let object_name = "test-object.txt";
        let content = Bytes::from("Hello, World!");

        s3_client
            .put_object("some-request-id", bucket, object_name, content.clone())
            .await
            .unwrap();

        let captured_request = s3_client.http_client.last_captured_request();
        let request = captured_request.as_ref().unwrap();

        assert_eq!(request.method(), "PUT");

        let expected_uri = format!("https://{bucket}.s3.amazonaws.com/{object_name}");
        assert_eq!(request.uri().to_string(), expected_uri);

        assert_eq!(request.body(), &content);

        assert!(request.headers().contains_key("x-amz-date"));
        assert!(request.headers().contains_key("x-amz-content-sha256"));
        assert!(request.headers().contains_key("content-length"));
        assert!(request.headers().contains_key("authorization"));

        let auth_header = request
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(auth_header.starts_with("AWS4-HMAC-SHA256"));
        assert!(auth_header.contains("Credential="));
        assert!(auth_header.contains("SignedHeaders="));
        assert!(auth_header.contains("Signature="));
    }

    #[wstd::test]
    async fn test_s3_delete_object_request() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";

        let mock_client = MockHttpClient::new();

        mock_client.expect_response(
            Response::builder()
                .status(StatusCode::NO_CONTENT)
                .body(vec![])
                .unwrap(),
        );

        let s3_client = S3Client::new(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
            mock_client,
        );

        let bucket = "test-bucket";
        let object_name = "test-object.txt";

        s3_client
            .delete_object("some-request-id", bucket, object_name)
            .await
            .unwrap();

        let captured_request = s3_client.http_client.last_captured_request();
        let request = captured_request.as_ref().unwrap();

        assert_eq!(request.method(), "DELETE");

        let expected_uri = format!("https://{bucket}.s3.amazonaws.com/{object_name}");
        assert_eq!(request.uri().to_string(), expected_uri);

        let expected_body: Vec<u8> = vec![];
        assert_eq!(request.body(), &expected_body);

        assert!(request.headers().contains_key("x-amz-date"));
        assert!(request.headers().contains_key("x-amz-content-sha256"));
        assert!(request.headers().contains_key("authorization"));

        let auth_header = request
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(auth_header.starts_with("AWS4-HMAC-SHA256"));
        assert!(auth_header.contains("Credential="));
        assert!(auth_header.contains("SignedHeaders="));
        assert!(auth_header.contains("Signature="));
    }
}

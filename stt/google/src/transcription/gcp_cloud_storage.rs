use std::sync::Arc;

use bytes::Bytes;
use golem_stt::{error::Error as SttError, http::HttpClient};
use http::{Request, StatusCode};

use super::gcp_auth::GcpAuth;

#[allow(async_fn_in_trait)]
pub trait CloudStorageService {
    async fn put_object(
        &self,
        request_id: &str,
        bucket: &str,
        object_name: &str,
        content: Bytes,
    ) -> Result<(), SttError>;

    async fn delete_object(
        &self,
        request_id: &str,
        bucket: &str,
        object_name: &str,
    ) -> Result<(), SttError>;
}

pub struct CloudStorageClient<HC: HttpClient> {
    http_client: HC,
    auth: Arc<GcpAuth<HC>>,
}

impl<HC: HttpClient> CloudStorageClient<HC> {
    pub fn new(auth: Arc<GcpAuth<HC>>, http_client: HC) -> Self {
        Self { http_client, auth }
    }
}

impl<HC: HttpClient> CloudStorageService for CloudStorageClient<HC> {
    async fn put_object(
        &self,
        request_id: &str,
        bucket: &str,
        object_name: &str,
        content: Bytes,
    ) -> Result<(), golem_stt::error::Error> {
        let access_token = self.auth.get_access_token().await.map_err(|e| {
            SttError::Http(
                request_id.to_string(),
                golem_stt::http::Error::Generic(format!("Failed to get access token: {e}")),
            )
        })?;

        let uri = format!(
            "https://storage.googleapis.com/upload/storage/v1/b/{}/o?uploadType=media&name={}",
            bucket,
            urlencoding::encode(object_name)
        );

        let content_length = content.len().to_string();

        let request = Request::builder()
            .method("POST")
            .uri(&uri)
            .header("Content-Type", "application/octet-stream")
            .header("Content-Length", &content_length)
            .header("Authorization", format!("Bearer {access_token}"))
            .body(content)
            .map_err(|e| {
                SttError::Http(request_id.to_string(), golem_stt::http::Error::HttpError(e))
            })?;

        let response = self
            .http_client
            .execute(request)
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
                StatusCode::BAD_REQUEST => Err(SttError::APIBadRequest {
                    request_id,
                    provider_error: format!("Cloud Storage upload bad request: {error_body}"),
                }),
                StatusCode::UNAUTHORIZED => Err(SttError::APIForbidden {
                    request_id,
                    provider_error: format!("Cloud Storage upload forbidden error: {error_body}"),
                }),
                StatusCode::FORBIDDEN => Err(SttError::APIUnauthorized {
                    request_id,
                    provider_error: format!(
                        "Cloud Storage upload unauthorized error: {error_body}",
                    ),
                }),
                StatusCode::NOT_FOUND => Err(SttError::APIConflict {
                    request_id,
                    provider_error: format!("Cloud Storage upload conflict error: {error_body}"),
                }),
                StatusCode::TOO_MANY_REQUESTS => Err(SttError::APIRateLimit {
                    request_id,
                    provider_error: format!("Cloud Storage upload rate limit error: {error_body}",),
                }),
                s if s.is_server_error() => Err(SttError::APIInternalServerError {
                    request_id,
                    provider_error: format!(
                        "Cloud Storage upload server error ({status}): {error_body}",
                    ),
                }),
                _ => Err(SttError::APIUnknown {
                    request_id,
                    provider_error: format!(
                        "Cloud Storage upload unexpected error ({status}): {error_body}",
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
        let access_token = self.auth.get_access_token().await.map_err(|e| {
            SttError::Http(
                request_id.to_string(),
                golem_stt::http::Error::Generic(format!("Failed to get access token: {e}")),
            )
        })?;

        let uri = format!(
            "https://storage.googleapis.com/storage/v1/b/{}/o/{}",
            bucket,
            urlencoding::encode(object_name)
        );

        let request = Request::builder()
            .method("DELETE")
            .header("Authorization", format!("Bearer {access_token}"))
            .uri(&uri)
            .body(Bytes::new())
            .map_err(|e| {
                SttError::Http(request_id.to_string(), golem_stt::http::Error::HttpError(e))
            })?;

        let response = self
            .http_client
            .execute(request)
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
                StatusCode::BAD_REQUEST => Err(SttError::APIBadRequest {
                    request_id,
                    provider_error: format!("Cloud Storage delete bad request: {error_body}"),
                }),
                StatusCode::FORBIDDEN => Err(SttError::APIForbidden {
                    request_id,
                    provider_error: format!("Cloud Storage delete forbidden error: {error_body}"),
                }),
                StatusCode::UNAUTHORIZED => Err(SttError::APIUnauthorized {
                    request_id,
                    provider_error: format!(
                        "Cloud Storage delete unauthorized error: {error_body}"
                    ),
                }),
                StatusCode::CONFLICT => Err(SttError::APIConflict {
                    request_id,
                    provider_error: format!("Cloud Storage delete conflict error: {error_body}"),
                }),
                StatusCode::TOO_MANY_REQUESTS => Err(SttError::APIRateLimit {
                    request_id,
                    provider_error: format!("Cloud Storage delete rate limit error: {error_body}",),
                }),
                s if s.is_server_error() => Err(SttError::APIInternalServerError {
                    request_id,
                    provider_error: format!(
                        "Cloud Storage delete server error ({status}): {error_body}",
                    ),
                }),
                _ => Err(SttError::APIUnknown {
                    request_id,
                    provider_error: format!(
                        "Cloud Storage delete unexpected error ({status}): {error_body}",
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

    use crate::transcription::gcp_auth::ServiceAccountKey;

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

    fn create_test_service_account_key() -> ServiceAccountKey {
        ServiceAccountKey {
            key_type: "service_account".to_string(),
            project_id: "test-project-id".to_string(),
            private_key_id: "test-key-id".to_string(),
            private_key: "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC3nmCgsAlob5Fb\n8J81FCw+80nAilI2soaayyr7nYUPQJORtu4mNEOSdnLBTk4RFvaH8UAJ7h21fcF2\nUEn3YOB0yUYIKBDS3uB60oplwJOnbis3lAlsT0VZ/UtngF6zNhJBVpz/RrwSJ1Po\nTnOrlkrrRXgPK6t5AxuR0n+h4P3YMU7hLZ46A5m/7YLJdWkVE1p3GYcrlltm2sos\nWWUpiNGIDflG42tlJVwG+QXL7J9D4ua/jbkFOvKI0Dl893ka0gkUCR0T0Cm1TRwo\nbBTBV/b/YXVCSJug0KsIIxYG0izSzlETH0Ql9tl6G+q0C4H0HUkN/UZ3QFYPmZUs\nX3Wu8DmvAgMBAAECggEBAKIU4YK2IXfYk90uZ7q41d2zb7TP5IZ3zC2zjXuRrjSq\nchi7+zgqBkOw3tcXwf1/4ZpaMIcTc5ITMcS4VrJRB5DPYkws4bziFBEW7CepeCzh\nKLDksfSzfKpU1kzEmdNjtXWLeQY1cCouIPj810ntXrCTH8l0aOZnAd0UjKleK3S7\ngva0IYHvCtoYFdvvwCOfxRQKAufcwotkgJPs6m95QJYwwfN3EaZi7duuNu0fKRkH\nu2sfRqDcJR3Yo4Nt9LhqB/OfkfL0TuzkNbXi0ZsUTJ5pFRx1m+Gtbb3qC95MBeey\ng/F9slQwRpDyJdxIrNVn7tv5tsd8v+4USwAC+cklQnECgYEA2wFvJ4KykuKG4RXO\nbWG0pavchTIixcC86y1ht/OxZFx13KmVzyE0PiOGTozAJCAHu1JK5gLxgGzXgLLr\nnT55kBvTzQ7+HQh+jhjrIIruicfiugzEQ6MivSw0pnk2Lkta25AeHuW1bKao1dOr\nnBDrtAZ1oKybBcna8SkYHprXh/0CgYEA1qKwRoZjfokzwmLwCyXDQyDKgUM0OOLq\nMXsCVv8BXltoSH5/vlDKSePs+4Er3o596QJRUosuwLgfIHsqFSFpUDk3lIctkqOt\nT1P1tjBZg8qMCSFzIwqsyj0lXN5IK6Zqvi7WikVVQ7gN3Stu4H0C9OgyV+kzHlNW\niV8cfvMJChsCgYAWnQRMMRudPRSuQyEofDE59g/0FOQwRSF8qxfu9ZO4iC+HVF9q\nnsQVMnfYvoHMeR4zQmEHdQBYwWRTHqZjeyL0NVteThEBEHJ426vTlWTiByirC0xs\nq3iXzeu10Mg+aXt9NllV2WQtTtwaEBwlJj4gPZaBu7DaHSilRBgAeP6ORQKBgGsV\nZe75s3/5AdrUs8BMCdxe6smM9uv+wisHnQY8Wblyz1eDzUXtVs+AqMZeDr4Nx2HO\nJzaQfDXoZpc0+6zpK3q74S/4NVN418nBMNDB1Jc9IZqYlrH/7G9GDHMF72nfsFfM\nVHtN1hlgJYKX3cygci4v/pX/oeJaX81Pp47qwDLLAoGAJadd2du9Nrd5WNohsPBH\nNGtq6QMJsjAABKkFXlqFM4Jsc/zaEOa/fsLCp6lbrVEqvHZGFc+OoukDlhY+c3QU\nSFVTtnsNi4YIbd8xNUpRNw7neShlG64wG0tLTI+y7a7Xh7GWkfYdfA950O8QEh46\nrecURYwOhS+7tjhb0xXs4kU=\n-----END PRIVATE KEY-----".to_string(),
            client_email: "test@test-project-id.iam.gserviceaccount.com".to_string(),
            client_id: "test-client-id".to_string(),
            auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            auth_provider_x509_cert_url: "https://www.googleapis.com/oauth2/v1/certs".to_string(),
            client_x509_cert_url: "https://www.googleapis.com/robot/v1/metadata/x509/test%40test-project-id.iam.gserviceaccount.com".to_string(),
        }
    }

    #[wstd::test]
    async fn test_cloud_storage_put_object_request() {
        let auth_mock_client = MockHttpClient::new();

        auth_mock_client.expect_response(
                Response::builder()
                    .status(StatusCode::OK)
                    .body(br#"{"access_token":"test-access-token","token_type":"Bearer","expires_in":3600}"#.to_vec())
                    .unwrap(),
            );

        let storage_mock_client = MockHttpClient::new();
        storage_mock_client.expect_response(
            Response::builder()
                .status(StatusCode::OK)
                .body(vec![])
                .unwrap(),
        );

        let service_account_key = create_test_service_account_key();

        let auth = GcpAuth::new(service_account_key, auth_mock_client).unwrap();

        let cloud_storage_client = CloudStorageClient::new(auth.into(), storage_mock_client);

        let bucket = "test-bucket";
        let object_name = "test-object.txt";
        let content = Bytes::from("Hello, World!");

        cloud_storage_client
            .put_object("some-request-id", bucket, object_name, content.clone())
            .await
            .unwrap();

        let captured_request = cloud_storage_client.http_client.last_captured_request();
        let request = captured_request.as_ref().unwrap();

        assert_eq!(request.method(), "POST");

        let expected_uri = format!(
            "https://storage.googleapis.com/upload/storage/v1/b/{}/o?uploadType=media&name={}",
            bucket,
            urlencoding::encode(object_name)
        );
        assert_eq!(request.uri().to_string(), expected_uri);

        assert_eq!(request.body(), &content);

        // Check headers
        assert_eq!(
            request.headers().get("content-type").unwrap(),
            "application/octet-stream"
        );
        assert_eq!(
            request.headers().get("content-length").unwrap(),
            &content.len().to_string()
        );

        let auth_header = request
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(auth_header, "Bearer test-access-token");
    }

    #[wstd::test]
    async fn test_cloud_storage_delete_object_request() {
        let auth_mock_client = MockHttpClient::new();

        auth_mock_client.expect_response(
                Response::builder()
                    .status(StatusCode::OK)
                    .body(br#"{"access_token":"test-access-token","token_type":"Bearer","expires_in":3600}"#.to_vec())
                    .unwrap(),
            );

        let storage_mock_client = MockHttpClient::new();

        storage_mock_client.expect_response(
            Response::builder()
                .status(StatusCode::NO_CONTENT)
                .body(vec![])
                .unwrap(),
        );

        let service_account_key = create_test_service_account_key();

        let auth = GcpAuth::new(service_account_key, auth_mock_client).unwrap();

        let cloud_storage_client = CloudStorageClient::new(auth.into(), storage_mock_client);

        let bucket = "test-bucket";
        let object_name = "test-object.txt";

        cloud_storage_client
            .delete_object("some-request-id", bucket, object_name)
            .await
            .unwrap();

        let captured_request = cloud_storage_client.http_client.last_captured_request();
        let request = captured_request.as_ref().unwrap();

        assert_eq!(request.method(), "DELETE");

        let expected_uri = format!(
            "https://storage.googleapis.com/storage/v1/b/{}/o/{}",
            bucket,
            urlencoding::encode(object_name)
        );
        assert_eq!(request.uri().to_string(), expected_uri);

        let expected_body: Vec<u8> = vec![];
        assert_eq!(request.body(), &expected_body);

        let auth_header = request
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(auth_header, "Bearer test-access-token");
    }
}

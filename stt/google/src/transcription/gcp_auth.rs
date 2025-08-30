use async_lock::Mutex;
use bytes::Bytes;
use std::sync::Arc;

use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Duration, Utc};
use golem_stt::http::HttpClient;
use http::Request;
use rsa::Pkcs1v15Sign;
use rsa::{pkcs8::DecodePrivateKey, RsaPrivateKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[allow(unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct ServiceAccountKey {
    #[serde(rename = "type")]
    pub key_type: String,
    pub project_id: String,
    pub private_key_id: String,
    pub private_key: String,
    pub client_email: String,
    pub client_id: String,
    pub auth_uri: String,
    pub token_uri: String,
    pub auth_provider_x509_cert_url: String,
    pub client_x509_cert_url: String,
}

impl ServiceAccountKey {
    pub fn new(project_id: String, client_email: String, private_key: String) -> Self {
        Self {
            key_type: "".to_string(),
            project_id,
            private_key_id: "".to_string(),
            private_key,
            client_email,
            client_id: "".to_string(),
            auth_uri: "".to_string(),
            token_uri: "".to_string(),
            auth_provider_x509_cert_url: "".to_string(),
            client_x509_cert_url: "".to_string(),
        }
    }
}

#[allow(unused)]
#[derive(Debug)]
pub enum Error {
    #[allow(clippy::enum_variant_names)]
    JsonError(serde_json::Error),
    #[allow(clippy::enum_variant_names)]
    CryptoError(String),
    #[allow(clippy::enum_variant_names)]
    HttpError(String),
    TokenExchange(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Serialize, Deserialize)]
struct JwtHeader {
    alg: String,
    typ: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct JwtClaim {
    iss: String,
    scope: String,
    aud: String,
    exp: i64,
    iat: i64,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: Option<i64>,
    token_type: String,
}

#[derive(Clone)]
pub struct GcpAuth<HC: HttpClient> {
    http_client: HC,
    project_id: String,
    client_email: String,
    private_key: RsaPrivateKey,
    token_data: Arc<Mutex<TokenData>>,
}

#[derive(Debug)]
struct TokenData {
    access_token: Option<String>,
    token_expires_at: Option<DateTime<Utc>>,
}

// based on https://developers.google.com/identity/protocols/oauth2/service-account#httprest
impl<HC: HttpClient> GcpAuth<HC> {
    pub fn new(service_account_key: ServiceAccountKey, http_client: HC) -> Result<Self, Error> {
        let private_key = Self::parse_private_key(&service_account_key.private_key)?;

        Ok(Self {
            http_client,
            project_id: service_account_key.project_id,
            client_email: service_account_key.client_email,
            private_key,
            token_data: Arc::new(Mutex::new(TokenData {
                access_token: None,
                token_expires_at: None,
            })),
        })
    }

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    fn parse_private_key(pem_key: &str) -> Result<RsaPrivateKey, Error> {
        let cleaned = pem_key.replace("\\n", "\n");

        RsaPrivateKey::from_pkcs8_pem(&cleaned)
            .map_err(|e| Error::CryptoError(format!("Failed to parse private key: {e}")))
    }

    pub async fn get_access_token(&self) -> Result<String, Error> {
        // First, check if we have a valid token (quick read-only check)
        {
            let token_data = self.token_data.lock().await;

            if let (Some(token), Some(expires_at)) =
                (&token_data.access_token, &token_data.token_expires_at)
            {
                if Utc::now() < *expires_at - Duration::minutes(5) {
                    return Ok(token.clone());
                }
            }
        }

        let mut token_data = self.token_data.lock().await;

        // Double-check if another concurrent access refreshed token while we were waiting
        if let (Some(token), Some(expires_at)) =
            (&token_data.access_token, &token_data.token_expires_at)
        {
            if Utc::now() < *expires_at - Duration::minutes(5) {
                return Ok(token.clone());
            }
        }

        // Refresh token
        let jwt = self.create_signed_jwt()?;
        let access_token = self.exchange_jwt_for_oauth_token(jwt).await?;

        // Update token
        token_data.access_token = Some(access_token.clone());
        token_data.token_expires_at = Some(Utc::now() + Duration::minutes(55));

        Ok(access_token)
    }

    fn create_signed_jwt(&self) -> Result<String, Error> {
        let now = Utc::now().timestamp();
        let exp = now + 3600;

        let header = JwtHeader {
            alg: "RS256".to_string(),
            typ: "JWT".to_string(),
        };

        let claim = JwtClaim {
            iss: self.client_email.clone(),
            scope: "https://www.googleapis.com/auth/cloud-platform".to_string(),
            aud: "https://oauth2.googleapis.com/token".to_string(),
            exp,
            iat: now,
        };

        let header_json = serde_json::to_string(&header).map_err(Error::JsonError)?;
        let claim_json = serde_json::to_string(&claim).map_err(Error::JsonError)?;

        let header_b64 = general_purpose::URL_SAFE_NO_PAD.encode(header_json.as_bytes());
        let claim_b64 = general_purpose::URL_SAFE_NO_PAD.encode(claim_json.as_bytes());

        let to_be_signed = format!("{header_b64}.{claim_b64}");

        // Sign with RSASSA-PKCS1-v1_5 (JWT RS256 standard)
        let signature = self.calculate_signature(to_be_signed.as_bytes())?;
        let signature_b64 = general_purpose::URL_SAFE_NO_PAD.encode(&signature);

        Ok(format!("{to_be_signed}.{signature_b64}"))
    }

    fn calculate_signature(&self, data: &[u8]) -> Result<Vec<u8>, Error> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();

        let padding = Pkcs1v15Sign::new::<Sha256>();

        let mut rng = rand::thread_rng();
        let signature = self
            .private_key
            .sign_with_rng(&mut rng, padding, &hash)
            .map_err(|e| Error::CryptoError(format!("Failed to sign data: {e}")))?;

        Ok(signature)
    }

    async fn exchange_jwt_for_oauth_token(&self, jwt: String) -> Result<String, Error> {
        let form_data = format!(
            "grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer&assertion={}",
            urlencoding::encode(&jwt)
        );

        let content_length = form_data.len();
        let body_bytes = Bytes::from(form_data);

        let request = Request::builder()
            .method("POST")
            .uri("https://oauth2.googleapis.com/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Content-Length", content_length)
            .body(body_bytes)
            .map_err(|e| Error::HttpError(format!("Failed to build token request: {e}")))?;

        let response = self
            .http_client
            .execute(request)
            .await
            .map_err(|e| Error::HttpError(format!("Token request failed: {e:?}")))?;

        if !response.status().is_success() {
            let error_body = String::from_utf8_lossy(response.body());
            return Err(Error::TokenExchange(format!(
                "Token exchange failed with status {}: {}",
                response.status(),
                error_body
            )));
        }

        let token_response: TokenResponse =
            serde_json::from_slice(response.body()).map_err(Error::JsonError)?;

        Ok(token_response.access_token)
    }
}

#[cfg(test)]
mod tests {

    use http::Response;
    use rsa::RsaPublicKey;

    use super::*;
    use std::{
        cell::{Ref, RefCell},
        collections::VecDeque,
    };

    struct MockHttpClient {
        pub responses: RefCell<VecDeque<Result<Response<Vec<u8>>, golem_stt::http::Error>>>,
        pub captured_requests: RefCell<Vec<Request<Bytes>>>,
    }

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
    async fn test_token_exchange_flow() {
        let mock_client = MockHttpClient::new();

        let token_response =
            r#"{"access_token": "ya29.test_token", "expires_in": 3600, "token_type": "Bearer"}"#;
        mock_client.expect_response(
            Response::builder()
                .status(200)
                .body(token_response.as_bytes().to_vec())
                .unwrap(),
        );

        let project_id = "test-project-123";
        let client_email = "test-service-account@test-project-123.iam.gserviceaccount.com";
        let private_key = "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC3nmCgsAlob5Fb\n8J81FCw+80nAilI2soaayyr7nYUPQJORtu4mNEOSdnLBTk4RFvaH8UAJ7h21fcF2\nUEn3YOB0yUYIKBDS3uB60oplwJOnbis3lAlsT0VZ/UtngF6zNhJBVpz/RrwSJ1Po\nTnOrlkrrRXgPK6t5AxuR0n+h4P3YMU7hLZ46A5m/7YLJdWkVE1p3GYcrlltm2sos\nWWUpiNGIDflG42tlJVwG+QXL7J9D4ua/jbkFOvKI0Dl893ka0gkUCR0T0Cm1TRwo\nbBTBV/b/YXVCSJug0KsIIxYG0izSzlETH0Ql9tl6G+q0C4H0HUkN/UZ3QFYPmZUs\nX3Wu8DmvAgMBAAECggEBAKIU4YK2IXfYk90uZ7q41d2zb7TP5IZ3zC2zjXuRrjSq\nchi7+zgqBkOw3tcXwf1/4ZpaMIcTc5ITMcS4VrJRB5DPYkws4bziFBEW7CepeCzh\nKLDksfSzfKpU1kzEmdNjtXWLeQY1cCouIPj810ntXrCTH8l0aOZnAd0UjKleK3S7\ngva0IYHvCtoYFdvvwCOfxRQKAufcwotkgJPs6m95QJYwwfN3EaZi7duuNu0fKRkH\nu2sfRqDcJR3Yo4Nt9LhqB/OfkfL0TuzkNbXi0ZsUTJ5pFRx1m+Gtbb3qC95MBeey\ng/F9slQwRpDyJdxIrNVn7tv5tsd8v+4USwAC+cklQnECgYEA2wFvJ4KykuKG4RXO\nbWG0pavchTIixcC86y1ht/OxZFx13KmVzyE0PiOGTozAJCAHu1JK5gLxgGzXgLLr\nnT55kBvTzQ7+HQh+jhjrIIruicfiugzEQ6MivSw0pnk2Lkta25AeHuW1bKao1dOr\nnBDrtAZ1oKybBcna8SkYHprXh/0CgYEA1qKwRoZjfokzwmLwCyXDQyDKgUM0OOLq\nMXsCVv8BXltoSH5/vlDKSePs+4Er3o596QJRUosuwLgfIHsqFSFpUDk3lIctkqOt\nT1P1tjBZg8qMCSFzIwqsyj0lXN5IK6Zqvi7WikVVQ7gN3Stu4H0C9OgyV+kzHlNW\niV8cfvMJChsCgYAWnQRMMRudPRSuQyEofDE59g/0FOQwRSF8qxfu9ZO4iC+HVF9q\nnsQVMnfYvoHMeR4zQmEHdQBYwWRTHqZjeyL0NVteThEBEHJ426vTlWTiByirC0xs\nq3iXzeu10Mg+aXt9NllV2WQtTtwaEBwlJj4gPZaBu7DaHSilRBgAeP6ORQKBgGsV\nZe75s3/5AdrUs8BMCdxe6smM9uv+wisHnQY8Wblyz1eDzUXtVs+AqMZeDr4Nx2HO\nJzaQfDXoZpc0+6zpK3q74S/4NVN418nBMNDB1Jc9IZqYlrH/7G9GDHMF72nfsFfM\nVHtN1hlgJYKX3cygci4v/pX/oeJaX81Pp47qwDLLAoGAJadd2du9Nrd5WNohsPBH\nNGtq6QMJsjAABKkFXlqFM4Jsc/zaEOa/fsLCp6lbrVEqvHZGFc+OoukDlhY+c3QU\nSFVTtnsNi4YIbd8xNUpRNw7neShlG64wG0tLTI+y7a7Xh7GWkfYdfA950O8QEh46\nrecURYwOhS+7tjhb0xXs4kU=\n-----END PRIVATE KEY-----";

        let service_account_key = ServiceAccountKey::new(
            project_id.to_string(),
            client_email.to_string(),
            private_key.to_string(),
        );

        let auth = GcpAuth::new(service_account_key, mock_client).unwrap();

        let _result = auth.get_access_token().await;

        let request = auth.http_client.last_captured_request().unwrap();
        assert_eq!(request.method(), "POST");
        assert_eq!(request.uri(), "https://oauth2.googleapis.com/token");

        let content_type = request.headers().get("content-type").unwrap();
        assert_eq!(content_type, "application/x-www-form-urlencoded");

        let body = String::from_utf8_lossy(request.body());
        assert!(body.contains("grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer"));
        assert!(body.contains("assertion="));

        // Extract and verify the JWT assertion structure
        let assertion_start = body.find("assertion=").unwrap() + "assertion=".len();
        let assertion_end = body[assertion_start..]
            .find('&')
            .unwrap_or(body[assertion_start..].len());
        let assertion_encoded = &body[assertion_start..assertion_start + assertion_end];
        let assertion = urlencoding::decode(assertion_encoded).unwrap();

        let jwt_parts: Vec<&str> = assertion.split('.').collect();
        assert_eq!(
            jwt_parts.len(),
            3,
            "JWT should have 3 parts: header.claim.signature"
        );

        let header_b64 = jwt_parts[0];
        let claim_b64 = jwt_parts[1];
        let signature_b64 = jwt_parts[2];

        let public_key = RsaPublicKey::from(&auth.private_key);
        let padding = Pkcs1v15Sign::new::<Sha256>();

        let to_be_signed = format!("{header_b64}.{claim_b64}");

        let signature_bytes = general_purpose::URL_SAFE_NO_PAD
            .decode(signature_b64)
            .unwrap();

        let mut hasher = Sha256::new();
        hasher.update(to_be_signed.as_bytes());
        let hash = hasher.finalize();

        public_key
            .verify(padding, &hash, &signature_bytes)
            .expect("JWT signature should be cryptographically valid");

        // Decode and verify the JSON content structure
        let header_json = general_purpose::URL_SAFE_NO_PAD.decode(header_b64).unwrap();
        let claim_json = general_purpose::URL_SAFE_NO_PAD.decode(claim_b64).unwrap();

        let parsed_header: JwtHeader = serde_json::from_slice(&header_json).unwrap();
        let parsed_claim: JwtClaim = serde_json::from_slice(&claim_json).unwrap();

        assert_eq!(parsed_header.alg, "RS256");
        assert_eq!(parsed_header.typ, "JWT");

        // Verify payload structure and content
        assert_eq!(
            parsed_claim.iss,
            "test-service-account@test-project-123.iam.gserviceaccount.com"
        );
        assert_eq!(
            parsed_claim.scope,
            "https://www.googleapis.com/auth/cloud-platform"
        );
        assert_eq!(parsed_claim.aud, "https://oauth2.googleapis.com/token");

        // Verify timestamps are reasonable (within last minute and next hour)
        let now = Utc::now().timestamp();
        assert!(parsed_claim.iat >= now - 60, "iat should be recent");
        assert!(parsed_claim.iat <= now + 60, "iat should not be in future");
        assert_eq!(
            parsed_claim.exp,
            parsed_claim.iat + 3600,
            "exp should be 1 hour after iat"
        );
    }
}

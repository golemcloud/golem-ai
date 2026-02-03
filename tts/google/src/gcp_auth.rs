use async_lock::Mutex;
use bytes::Bytes;
use std::sync::Arc;

use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Duration, Utc};
use golem_tts::http::HttpClient;
use http::Request;
use rsa::Pkcs1v15Sign;
use rsa::{pkcs8::DecodePrivateKey, RsaPrivateKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Debug)]
pub enum Error {
    JsonError(serde_json::Error),
    CryptoError(String),
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

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: Option<i64>,
    token_type: String,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct GcpAuth<HC: HttpClient> {
    pub(crate) http_client: HC,
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

    #[allow(dead_code)]
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    fn parse_private_key(pem_key: &str) -> Result<RsaPrivateKey, Error> {
        let cleaned = pem_key.replace("\\n", "\n");

        RsaPrivateKey::from_pkcs8_pem(&cleaned)
            .map_err(|e| Error::CryptoError(format!("Failed to parse private key: {e}")))
    }

    pub async fn get_access_token(&self) -> Result<String, Error> {
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

        if let (Some(token), Some(expires_at)) =
            (&token_data.access_token, &token_data.token_expires_at)
        {
            if Utc::now() < *expires_at - Duration::minutes(5) {
                return Ok(token.clone());
            }
        }

        let jwt = self.create_signed_jwt()?;
        let access_token = self.exchange_jwt_for_oauth_token(jwt).await?;

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
        let request = Request::builder()
            .method(http::Method::POST)
            .uri("https://oauth2.googleapis.com/token")
            .header(http::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Bytes::from(form_data))
            .map_err(|e| Error::HttpError(format!("Failed to create request: {e}")))?;
        let response = self
            .http_client
            .execute(request)
            .await
            .map_err(|e| Error::HttpError(format!("HTTP error: {e}")))?;
        if !response.status().is_success() {
            return Err(Error::TokenExchange(
                String::from_utf8_lossy(response.body()).to_string(),
            ));
        }
        let token: TokenResponse = serde_json::from_slice(response.body())
            .map_err(Error::JsonError)?;
        Ok(token.access_token)
    }
}

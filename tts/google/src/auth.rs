use base64::{engine::general_purpose, Engine};
use golem_tts::{config::get_env, golem::tts::types::TtsError};
use reqwest::Client;
use rsa::Pkcs1v15Sign;
use rsa::{pkcs8::DecodePrivateKey, RsaPrivateKey};
use rsa::sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::UNIX_EPOCH;

use crate::google::Google;

/// Google Cloud service account credentials JSON structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAccountCredentials {
    #[serde(rename = "type")]
    pub credential_type: String,
    pub project_id: Option<String>,
    pub private_key_id: Option<String>,
    pub private_key: String,
    pub client_email: String,
    pub client_id: Option<String>,
    pub auth_uri: Option<String>,
    pub token_uri: Option<String>,
    pub auth_provider_x509_cert_url: Option<String>,
    pub client_x509_cert_url: Option<String>,
}

impl Google {
    /// Get an access token for Google Cloud API authentication.
    ///
    /// Requires `GOOGLE_APPLICATION_CREDENTIALS` environment variable set to the path of a service account JSON file.
    /// The method caches tokens and automatically refreshes them when they expire.
    pub fn get_access_token(&self) -> Result<String, TtsError> {
        // Check if we have a valid cached token
        {
            let token_data = self.token_data.lock().unwrap();
            if let (Some(token), Some(expires_at)) =
                (&token_data.access_token, &token_data.expires_at)
            {
                let now = std::time::SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;

                if now < expires_at - 300 {
                    // 5 minutes buffer
                    return Ok(token.clone());
                }
            }
        }

        // Load credentials from GOOGLE_APPLICATION_CREDENTIALS or GOOGLE_SERVICE_ACCOUNT_JSON
        let credentials = if let Ok(json_content) = get_env("GOOGLE_SERVICE_ACCOUNT_JSON") {
            // Direct JSON content provided
            Self::parse_service_account_credentials(&json_content)?
        } else if let Ok(creds_path) = get_env("GOOGLE_APPLICATION_CREDENTIALS") {
            // File path provided
            Self::load_service_account_credentials(&creds_path)?
        } else {
            return Err(TtsError::InvalidConfiguration(
                "Missing GOOGLE_SERVICE_ACCOUNT_JSON or GOOGLE_APPLICATION_CREDENTIALS environment variable. \
                 Set GOOGLE_SERVICE_ACCOUNT_JSON to the JSON content or GOOGLE_APPLICATION_CREDENTIALS to the file path.".to_string(),
            ));
        };
        let client_email = credentials.client_email;
        let private_key_pem = credentials.private_key;

        let private_key = RsaPrivateKey::from_pkcs8_pem(&private_key_pem).map_err(|e| {
            TtsError::InvalidConfiguration(format!("Failed to parse private key: {e}"))
        })?;

        // Create JWT and exchange for access token
        let jwt = self.create_jwt(private_key, client_email)?;
        let access_token = self.exchange_jwt_for_token(jwt)?;

        // Update token cache
        {
            let mut token_data = self.token_data.lock().unwrap();
            let now = std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            token_data.access_token = Some(access_token.clone());
            token_data.expires_at = Some(now + 3300); // 55 minutes
        }

        Ok(access_token)
    }

    /// Parse service account credentials from JSON string
    fn parse_service_account_credentials(json_content: &str) -> Result<ServiceAccountCredentials, TtsError> {
        let credentials: ServiceAccountCredentials = serde_json::from_str(json_content).map_err(|e| {
            TtsError::InvalidConfiguration(format!(
                "Failed to parse service account credentials JSON: {}",
                e
            ))
        })?;

        // Validate that it's a service account
        if credentials.credential_type != "service_account" {
            return Err(TtsError::InvalidConfiguration(format!(
                "Invalid credential type '{}'. Expected 'service_account'",
                credentials.credential_type
            )));
        }

        Ok(credentials)
    }

    /// Load service account credentials from a JSON file
    fn load_service_account_credentials(path: &str) -> Result<ServiceAccountCredentials, TtsError> {
        let json_content = std::fs::read_to_string(path).map_err(|e| {
            TtsError::InvalidConfiguration(format!(
                "Failed to read service account credentials file at '{}': {}",
                path, e
            ))
        })?;

        Self::parse_service_account_credentials(&json_content)
    }

    fn create_jwt(
        &self,
        private_key: RsaPrivateKey,
        client_email: String,
    ) -> Result<String, TtsError> {
        let now = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let exp = now + 3600; // 1 hour

        let header = serde_json::json!({
            "alg": "RS256",
            "typ": "JWT"
        });

        let claims = serde_json::json!({
            "iss": client_email,
            "scope": "https://www.googleapis.com/auth/cloud-platform",
            "aud": "https://oauth2.googleapis.com/token",
            "exp": exp,
            "iat": now
        });

        let header_b64 =
            general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).unwrap());
        let claims_b64 =
            general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap());
        let to_be_signed = format!("{}.{}", header_b64, claims_b64);

        // Sign with RSA using PKCS#1 v1.5 padding with SHA256
        let mut hasher = Sha256::new();
        hasher.update(to_be_signed.as_bytes());
        let hash = hasher.finalize();

        let padding = Pkcs1v15Sign::new::<Sha256>();
        let mut rng = rand::thread_rng();
        let signature = private_key
            .sign_with_rng(&mut rng, padding, &hash)
            .map_err(|e| TtsError::InternalError(format!("Failed to sign JWT: {e}")))?;

        let signature_b64 = general_purpose::URL_SAFE_NO_PAD.encode(&signature);

        Ok(format!("{}.{}", to_be_signed, signature_b64))
    }

    fn exchange_jwt_for_token(&self, jwt: String) -> Result<String, TtsError> {
        let form_data = format!(
            "grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer&assertion={}",
            urlencoding::encode(&jwt)
        );

        let response = Client::new()
            .post("https://oauth2.googleapis.com/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form_data)
            .send()
            .map_err(|e| TtsError::NetworkError(format!("Token request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().unwrap_or_else(|_| "Unable to read error body".to_string());
            return Err(TtsError::Unauthorized(format!(
                "Token exchange failed: {} - {}",
                status,
                error_body
            )));
        }

        let token_response: Value = response
            .json()
            .map_err(|e| TtsError::InternalError(format!("Failed to parse token response: {e}")))?;

        token_response["access_token"]
            .as_str()
            .ok_or_else(|| TtsError::InternalError("Missing access_token in response".to_string()))
            .map(|s| s.to_string())
    }
}

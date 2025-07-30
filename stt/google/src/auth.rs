use serde::{Deserialize, Serialize};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use golem_stt::golem::stt::types::SttError;
use crate::config::GoogleConfig;
use reqwest::Client;

#[derive(Serialize, Deserialize)]
struct Claims<'a> {
    iss: &'a str,
    scope: &'a str,
    aud: &'a str,
    exp: usize,
    iat: usize,
}

#[allow(dead_code)]
pub struct TokenInfo {
    pub access_token: String,
    pub _expires_at: usize,
}

pub fn fetch_token(cfg: &GoogleConfig) -> Result<TokenInfo, SttError> {
    #[derive(Deserialize)]
    struct Creds {
        client_email: String,
        private_key: String,
        token_uri: String,
    }
    let creds: Creds = serde_json::from_str(&cfg.credentials_json)
        .map_err(|e| SttError::Unauthorized(format!("invalid creds json: {e}")))?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    let claims = Claims {
        iss: &creds.client_email,
        scope: "https://www.googleapis.com/auth/cloud-platform",
        aud: &creds.token_uri,
        exp: now + 3600,
        iat: now,
    };
    let header = Header::new(Algorithm::RS256);
    let jwt = encode(&header, &claims, &EncodingKey::from_rsa_pem(creds.private_key.as_bytes()).map_err(|e| SttError::Unauthorized(format!("key error {e}")))?)
        .map_err(|e| SttError::Unauthorized(format!("jwt error {e}")))?;

    let client = Client::new();
    let resp = client
        .post(&creds.token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &jwt),
        ])
        .send()
        .map_err(|e| SttError::NetworkError(format!("{e}")))?;
    #[derive(Deserialize)]
    struct Resp { access_token: String, expires_in: usize }
    let token: Resp = resp.json().map_err(|e| SttError::InternalError(format!("{e}")))?;
    Ok(TokenInfo { access_token: token.access_token, _expires_at: now + token.expires_in })
} 
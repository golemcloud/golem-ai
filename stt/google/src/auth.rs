use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use golem_stt::golem::stt::types::SttError;
use crate::config::GoogleConfig;
#[allow(unused_imports)]
use reqwest::Client;
use std::cell::RefCell;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize)]
struct Claims<'a> {
    iss: &'a str,
    scope: &'a str,
    aud: &'a str,
    exp: usize,
    iat: usize,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct TokenInfo {
    pub access_token: String,
    pub expires_at: usize,
}

thread_local! {
    static TOKEN_CACHE: RefCell<Option<TokenInfo>> = const { RefCell::new(None) };
}

#[allow(dead_code)]
fn now_secs() -> usize {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
}

#[allow(dead_code)]
fn should_retry_status(status: u16) -> bool { status == 429 || status == 500 || status == 502 || status == 503 }

#[cfg(target_arch = "wasm32")]
pub fn fetch_token(_cfg: &GoogleConfig) -> Result<TokenInfo, SttError> {
    if let Ok(tok) = std::env::var("GOOGLE_ACCESS_TOKEN") {
        return Ok(TokenInfo { access_token: tok, expires_at: now_secs() + 3000 });
    }
    Err(SttError::Unauthorized("missing GOOGLE_ACCESS_TOKEN for wasm32 build".into()))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn fetch_token(cfg: &GoogleConfig) -> Result<TokenInfo, SttError> {
    let skew_secs: usize = 60;

    if let Some(cached) = TOKEN_CACHE.with(|c| c.borrow().clone()) {
        if cached.expires_at.saturating_sub(now_secs()) > skew_secs {
            return Ok(cached);
        }
    }

    #[derive(Deserialize)]
    struct Creds {
        client_email: String,
        private_key: String,
        token_uri: String,
    }
    let creds: Creds = serde_json::from_str(&cfg.credentials_json)
        .map_err(|e| SttError::Unauthorized(format!("invalid creds json: {e}")))?;
    let mut attempt: u32 = 0;
    let max_attempts = cfg.max_retries.max(1);
    loop {
        let now = now_secs();
        let claims = Claims {
            iss: &creds.client_email,
            scope: "https://www.googleapis.com/auth/cloud-platform",
            aud: &creds.token_uri,
            exp: now + 3600,
            iat: now,
        };
        let header = Header::new(Algorithm::RS256);
        let jwt = match EncodingKey::from_rsa_pem(creds.private_key.as_bytes())
            .map_err(|e| SttError::Unauthorized(format!("key error {e}")))
            .and_then(|k| encode(&header, &claims, &k).map_err(|e| SttError::Unauthorized(format!("jwt error {e}"))))
        {
            Ok(j) => j,
            Err(e) => return Err(e),
        };

        let client = Client::new();
        let resp = match client
            .post(&creds.token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &jwt),
            ])
            .send()
        {
            Ok(r) => r,
            Err(e) => {
                attempt += 1;
                if attempt >= max_attempts {
                    return Err(SttError::NetworkError(format!("{e}")));
                }
                let delay_ms = 100u64.saturating_mul(1u64 << (attempt - 1));
                let jitter_ms = (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos() as u64) % 100;
                std::thread::sleep(Duration::from_millis(delay_ms + jitter_ms));
                continue;
            }
        };

        let status = resp.status().as_u16();
        if status == 200 {
            #[derive(Deserialize)]
            struct Resp { access_token: String, expires_in: usize }
            let token: Resp = resp
                .json()
                .map_err(|e| SttError::InternalError(format!("{e}")))?;
            let info = TokenInfo { access_token: token.access_token, expires_at: now + token.expires_in };
            TOKEN_CACHE.with(|c| c.replace(Some(info.clone())));
            return Ok(info);
        }

        attempt += 1;
        if attempt >= max_attempts || !should_retry_status(status) {
            return match status {
                400 | 401 => Err(SttError::Unauthorized("unauthorized".into())),
                403 => Err(SttError::AccessDenied("access denied".into())),
                429 => Err(SttError::RateLimited(0)),
                500 | 502 | 503 => Err(SttError::ServiceUnavailable("service unavailable".into())),
                _ => Err(SttError::InternalError(format!("http {status}"))),
            };
        }
        let delay_ms = 100u64.saturating_mul(1u64 << (attempt - 1));
        let jitter_ms = (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos() as u64) % 100;
        std::thread::sleep(Duration::from_millis(delay_ms + jitter_ms));
    }
} 
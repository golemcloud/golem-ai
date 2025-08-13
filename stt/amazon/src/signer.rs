use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use time::{OffsetDateTime, macros::format_description};

type HmacSha256 = Hmac<Sha256>;

pub struct SigV4Params {
    pub access_key: String,
    pub secret_key: String,
    pub session_token: Option<String>,
    pub region: String,
    pub service: String,
}

pub fn hash_sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub fn hmac_sha256(key: &[u8], data: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).unwrap();
    mac.update(data.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

pub fn derive_signing_key(secret: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
    let k_secret = format!("AWS4{}", secret);
    let k_date = hmac_sha256(k_secret.as_bytes(), date);
    let k_region = hmac_sha256(&k_date, region);
    let k_service = hmac_sha256(&k_region, service);
    hmac_sha256(&k_service, "aws4_request")
}

pub fn sigv4_headers(params: &SigV4Params, method: &str, host: &str, uri: &str, amz_target: &str, payload: &[u8]) -> (String, String, Option<(String, String)>, String) {
    let content_type = "application/x-amz-json-1.1";
    let now = OffsetDateTime::now_utc();
    let date_fmt = format_description!("%Y%m%d");
    let datetime_fmt = format_description!("%Y%m%dT%H%M%SZ");
    let date_stamp = now.format(&date_fmt).unwrap();
    let amz_date = now.format(&datetime_fmt).unwrap();

    let signed_headers_base = if params.session_token.is_some() {
        "content-type;host;x-amz-date;x-amz-security-token;x-amz-target"
    } else {
        "content-type;host;x-amz-date;x-amz-target"
    };

    let payload_hash = hash_sha256_hex(payload);
    let mut canonical_headers = String::new();
    canonical_headers.push_str(&format!("content-type:{}\n", content_type));
    canonical_headers.push_str(&format!("host:{}\n", host));
    canonical_headers.push_str(&format!("x-amz-date:{}\n", amz_date));
    if let Some(token) = &params.session_token {
        canonical_headers.push_str(&format!("x-amz-security-token:{}\n", token));
    }
    canonical_headers.push_str(&format!("x-amz-target:{}\n", amz_target));

    let canonical_request = format!(
        "{}\n{}\n\n{}\n{}\n{}",
        method,
        uri,
        canonical_headers,
        signed_headers_base,
        payload_hash,
    );
    let canonical_request_hash = hash_sha256_hex(canonical_request.as_bytes());
    let credential_scope = format!("{}/{}/{}/aws4_request", date_stamp, params.region, params.service);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        amz_date,
        credential_scope,
        canonical_request_hash,
    );
    let signing_key = derive_signing_key(&params.secret_key, &date_stamp, &params.region, &params.service);
    let signature = hex::encode(hmac_sha256(&signing_key, &string_to_sign));
    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        params.access_key,
        credential_scope,
        signed_headers_base,
        signature,
    );
    let security_header = params.session_token.as_ref().map(|t| ("x-amz-security-token".to_string(), t.clone()));
    (amz_date, authorization, security_header, content_type.to_string())
}


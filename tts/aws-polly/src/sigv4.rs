//! AWS Signature Version 4 (SigV4) Implementation
//!
//! Provides signing logic for AWS Polly API requests in a WASM environment.
//! Adheres to professional standards (no unwrap, explicit error mapping).

// use chrono::Utc; (removed - not needed with DateTime::from_timestamp)
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Errors that can occur during SigV4 signing
#[derive(Debug)]
pub enum SigV4Error {
    InternalError(String),
}

type HmacSha256 = Hmac<Sha256>;

/// AWS Credentials
pub struct Credentials {
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
    pub service: String,
}

/// Signed Request Result
pub struct SignedHeaders {
    pub authorization: String,
    pub x_amz_date: String,
    pub x_amz_content_sha256: String,
}

/// Main signing function
pub fn sign_request(
    credentials: &Credentials,
    timestamp_secs: i64,
    method: &str,
    path: &str,
    query: &BTreeMap<String, String>,
    headers: &BTreeMap<String, String>,
    payload: &[u8],
) -> Result<SignedHeaders, SigV4Error> {
    // 1. Format dates
    let dt = chrono::DateTime::from_timestamp(timestamp_secs, 0)
        .ok_or_else(|| SigV4Error::InternalError("Invalid timestamp".to_string()))?;

    let date_time = dt.format("%Y%m%dT%H%M%SZ").to_string();
    let date_only = dt.format("%Y%m%d").to_string();

    // 2. Calculate payload hash
    let payload_hash = hex::encode(Sha256::digest(payload));

    // 3. Create canonical request
    let mut canonical_headers = String::new();
    let mut signed_headers_list = Vec::new();

    // Copy and filter headers (lowercase keys)
    let mut normalized_headers = BTreeMap::new();
    for (k, v) in headers {
        normalized_headers.insert(k.to_lowercase(), v.trim().to_string());
    }

    // Add required AWS headers if not present
    normalized_headers.insert(
        "host".to_string(),
        format!(
            "{}.{}.amazonaws.com",
            credentials.service, credentials.region
        ),
    );
    normalized_headers.insert("x-amz-date".to_string(), date_time.clone());
    normalized_headers.insert("x-amz-content-sha256".to_string(), payload_hash.clone());

    for (k, v) in &normalized_headers {
        canonical_headers.push_str(&format!("{}:{}\n", k, v));
        signed_headers_list.push(k.clone());
    }

    let signed_headers = signed_headers_list.join(";");

    let canonical_query = format_query(query);

    let canonical_request = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        method, path, canonical_query, canonical_headers, signed_headers, payload_hash
    );

    let canonical_request_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));

    // 4. Create string to sign
    let credential_scope = format!(
        "{}/{}/{}/aws4_request",
        date_only, credentials.region, credentials.service
    );
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        date_time, credential_scope, canonical_request_hash
    );

    // 5. Calculate signing key
    let signing_key = calculate_signing_key(
        &credentials.secret_key,
        &date_only,
        &credentials.region,
        &credentials.service,
    )?;

    // 6. Calculate signature
    let signature = hmac_sha256(&signing_key, string_to_sign.as_bytes())?;
    let signature_hex = hex::encode(signature);

    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        credentials.access_key, credential_scope, signed_headers, signature_hex
    );

    return Ok(SignedHeaders {
        authorization,
        x_amz_date: date_time,
        x_amz_content_sha256: payload_hash,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aws_sigv4_signing_determinism() {
        let creds = Credentials {
            access_key: "AKIA_TEST".to_string(),
            secret_key: "SECRET_TEST".to_string(),
            region: "us-east-1".to_string(),
            service: "polly".to_string(),
        };

        let timestamp = 1705430000; // Fixed timestamp
        let method = "POST";
        let path = "/v1/speech";
        let query = BTreeMap::new();
        let mut headers = BTreeMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("X-Golem-Oplog-Index".to_string(), "1".to_string());
        let payload = b"{\"text\":\"hello\"}";

        let signed1 =
            sign_request(&creds, timestamp, method, path, &query, &headers, payload).unwrap();
        let signed2 =
            sign_request(&creds, timestamp, method, path, &query, &headers, payload).unwrap();

        // 1. Verify determinism
        assert_eq!(signed1.authorization, signed2.authorization);
        assert_eq!(signed1.x_amz_date, signed2.x_amz_date);

        // 2. Verify signature format
        assert!(signed1.authorization.contains("AWS4-HMAC-SHA256"));
        assert!(signed1.authorization.contains("Credential=AKIA_TEST"));

        // 3. Verify different oplog index produces different signature (Entropy)
        headers.insert("X-Golem-Oplog-Index".to_string(), "2".to_string());
        let signed3 =
            sign_request(&creds, timestamp, method, path, &query, &headers, payload).unwrap();
        assert_ne!(
            signed1.authorization, signed3.authorization,
            "Signature must differ with oplog index"
        );
    }
}

// ============================================================
// HELPERS
// ============================================================

fn format_query(query: &BTreeMap<String, String>) -> String {
    let mut parts = Vec::new();
    for (k, v) in query {
        parts.push(format!("{}={}", url_encode(k), url_encode(v)));
    }
    return parts.join("&");
}

fn url_encode(s: &str) -> String {
    let mut encoded = String::new();
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*b as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    return encoded;
}

fn calculate_signing_key(
    secret_key: &str,
    date: &str,
    region: &str,
    service: &str,
) -> Result<Vec<u8>, SigV4Error> {
    let k_secret = format!("AWS4{}", secret_key);
    let k_date = hmac_sha256(k_secret.as_bytes(), date.as_bytes())?;
    let k_region = hmac_sha256(&k_date, region.as_bytes())?;
    let k_service = hmac_sha256(&k_region, service.as_bytes())?;
    let k_signing = hmac_sha256(&k_service, b"aws4_request")?;
    return Ok(k_signing);
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Result<Vec<u8>, SigV4Error> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|e| SigV4Error::InternalError(format!("HMAC key error: {}", e)))?;
    mac.update(data);
    return Ok(mac.finalize().into_bytes().to_vec());
}

mod hex {
    pub fn encode(data: impl AsRef<[u8]>) -> String {
        let mut s = String::with_capacity(data.as_ref().len() * 2);
        for &b in data.as_ref() {
            s.push_str(&format!("{:02x}", b));
        }
        return s;
    }
}

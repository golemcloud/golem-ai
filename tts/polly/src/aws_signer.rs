use std::fmt;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use derive_more::From;
use hmac::digest::InvalidLength;
use hmac::{Hmac, Mac};
use http::header::InvalidHeaderValue;
use http::{HeaderMap, HeaderValue, Request};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use sha2::{Digest, Sha256};

#[allow(unused)]
#[derive(Debug, From)]
pub enum Error {
    #[from]
    InvalidHeader(InvalidHeaderValue),
    #[from]
    HmacSha256ErrorInvalidLength(InvalidLength),
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}

type HmacSha256 = Hmac<Sha256>;

pub enum AwsService {
    Polly,
}

impl fmt::Display for AwsService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AwsService::Polly => write!(f, "polly"),
        }
    }
}

const URI_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'!')
    .add(b'"')
    .add(b'#')
    .add(b'$')
    .add(b'%')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b',')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'?')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

const QUERY_ENCODE_SET: &AsciiSet = &URI_ENCODE_SET.add(b'=').add(b'&').add(b'+');

#[allow(dead_code)]
#[derive(Clone)]
pub struct AwsSignatureV4 {
    access_key: String,
    secret_key: String,
    region: String,
    service: String,
}

impl AwsSignatureV4 {
    pub fn new(access_key: String, secret_key: String, region: String, service: AwsService) -> Self {
        Self {
            access_key,
            secret_key,
            region,
            service: service.to_string(),
        }
    }

    pub fn for_polly(access_key: String, secret_key: String, region: String) -> Self {
        Self::new(access_key, secret_key, region, AwsService::Polly)
    }

    pub fn get_region(&self) -> &str {
        &self.region
    }

    pub fn sign_request(
        &self,
        request: Request<Bytes>,
        timestamp: DateTime<Utc>,
    ) -> Result<Request<Bytes>, Error> {
        let (mut parts, body) = request.into_parts();

        let date_stamp = timestamp.format("%Y%m%d").to_string();
        let amz_date = timestamp.format("%Y%m%dT%H%M%SZ").to_string();

        parts
            .headers
            .insert("x-amz-date", HeaderValue::from_str(&amz_date)?);

        let content_sha256 = self.hash_payload(body.as_ref());
        parts.headers.insert(
            "x-amz-content-sha256",
            HeaderValue::from_str(&content_sha256)?,
        );

        let mut headers_for_signing = parts.headers.clone();

        if !headers_for_signing.contains_key("host") {
            if let Some(host) = parts.uri.host() {
                let host_header = if let Some(port) = parts.uri.port_u16() {
                    if (parts.uri.scheme_str() == Some("https") && port == 443)
                        || (parts.uri.scheme_str() == Some("http") && port == 80)
                    {
                        host.to_string()
                    } else {
                        format!("{host}:{port}")
                    }
                } else {
                    host.to_string()
                };

                headers_for_signing.insert("host", HeaderValue::from_str(&host_header)?);
            }
        }

        let canonical_request = self.create_canonical_request(
            &parts.method,
            &parts.uri,
            &headers_for_signing,
            &content_sha256,
        );

        let string_to_sign = self.create_string_to_sign(&canonical_request, &amz_date, &date_stamp);

        let signature = self.calculate_signature(&string_to_sign, &date_stamp)?;

        let signed_headers = self.get_signed_headers(&headers_for_signing);

        let credential = format!(
            "{}/{}/{}/aws4_request",
            date_stamp, self.region, self.service
        );

        let auth_header = format!(
            "AWS4-HMAC-SHA256 Credential={credential}, SignedHeaders={signed_headers}, Signature={signature}"
        );

        parts.headers.insert(
            "Authorization",
            HeaderValue::from_str(&auth_header)?,
        );

        Ok(Request::from_parts(parts, body))
    }

    fn create_canonical_request(
        &self,
        method: &http::Method,
        uri: &http::Uri,
        headers: &HeaderMap,
        payload_hash: &str,
    ) -> String {
        let canonical_uri = uri
            .path()
            .split('/')
            .map(|segment| utf8_percent_encode(segment, URI_ENCODE_SET).to_string())
            .collect::<Vec<_>>()
            .join("/");

        let canonical_query_string = uri
            .query()
            .unwrap_or("")
            .split('&')
            .map(|pair| {
                let mut parts = pair.splitn(2, '=');
                let key = utf8_percent_encode(parts.next().unwrap_or(""), QUERY_ENCODE_SET);
                let value = utf8_percent_encode(parts.next().unwrap_or(""), QUERY_ENCODE_SET);
                format!("{key}={value}")
            })
            .collect::<Vec<_>>()
            .join("&");

        let canonical_headers = self.get_canonical_headers(headers);
        let signed_headers = self.get_signed_headers(headers);

        format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method.as_str(),
            canonical_uri,
            canonical_query_string,
            canonical_headers,
            signed_headers,
            payload_hash
        )
    }

    fn get_canonical_headers(&self, headers: &HeaderMap) -> String {
        let mut header_pairs: Vec<(String, String)> = headers
            .iter()
            .filter_map(|(key, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|value_str| (key.to_string().to_lowercase(), value_str.trim().to_string()))
            })
            .collect();

        header_pairs.sort_by(|a, b| a.0.cmp(&b.0));

        header_pairs
            .iter()
            .map(|(key, value)| format!("{key}:{value}\n"))
            .collect()
    }

    fn get_signed_headers(&self, headers: &HeaderMap) -> String {
        let mut signed_headers: Vec<String> = headers
            .iter()
            .filter_map(|(key, _)| Some(key.to_string().to_lowercase()))
            .collect();

        signed_headers.sort();
        signed_headers.join(";")
    }

    fn create_string_to_sign(
        &self,
        canonical_request: &str,
        amz_date: &str,
        date_stamp: &str,
    ) -> String {
        let credential_scope = format!("{}/{}/{}/aws4_request", date_stamp, self.region, self.service);
        let hash = self.hash_payload(canonical_request.as_bytes());
        format!("AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{hash}")
    }

    fn calculate_signature(&self, string_to_sign: &str, date_stamp: &str) -> Result<String, Error> {
        let k_secret = format!("AWS4{}", self.secret_key);
        let k_date = self.hmac_sha256(k_secret.as_bytes(), date_stamp)?;
        let k_region = self.hmac_sha256(&k_date, &self.region)?;
        let k_service = self.hmac_sha256(&k_region, &self.service)?;
        let k_signing = self.hmac_sha256(&k_service, "aws4_request")?;
        let signature = self.hmac_sha256(&k_signing, string_to_sign)?;
        Ok(hex::encode(signature))
    }

    fn hmac_sha256(&self, key: &[u8], data: impl AsRef<[u8]>) -> Result<Vec<u8>, Error> {
        let mut mac = HmacSha256::new_from_slice(key)?;
        mac.update(data.as_ref());
        Ok(mac.finalize().into_bytes().to_vec())
    }

    fn hash_payload(&self, payload: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(payload);
        hex::encode(hasher.finalize())
    }
}

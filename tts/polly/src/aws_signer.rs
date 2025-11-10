use bytes::Bytes;
use chrono::Utc;
use golem_tts::golem::tts::types::TtsError;
use hmac::{Hmac, Mac};
use http::{HeaderMap, HeaderValue, Request};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

// Percent-encoding set for URI paths
// Why this is needed see here https://docs.aws.amazon.com/IAM/latest/UserGuide/reference_sigv-create-signed-request.html
// AWS uri encoding has special characters that need to be percent-encoded
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

// AWS-specific percent-encoding set for query strings
const QUERY_ENCODE_SET: &AsciiSet = &URI_ENCODE_SET.add(b'=').add(b'&').add(b'+');

#[derive(Clone)]
pub struct PollySigner {
    access_key: String,
    secret_key: String,
    region: String,
}

impl PollySigner {
    pub fn new(access_key: String, secret_key: String, region: String) -> Self {
        Self {
            access_key,
            secret_key,
            region,
        }
    }

    pub fn sign_request(&self, request: Request<Bytes>) -> Result<Request<Bytes>, TtsError> {
        let timestamp = Utc::now();
        let (mut parts, body) = request.into_parts();

        let date_stamp = timestamp.format("%Y%m%d").to_string();
        let amz_date = timestamp.format("%Y%m%dT%H%M%SZ").to_string();

        parts.headers.insert(
            "x-amz-date",
            HeaderValue::from_str(&amz_date)
                .map_err(|e| TtsError::InternalError(format!("Invalid header value: {}", e)))?,
        );

        let content_sha256 = self.hash_payload(body.as_ref());
        parts.headers.insert(
            "x-amz-content-sha256",
            HeaderValue::from_str(&content_sha256)
                .map_err(|e| TtsError::InternalError(format!("Invalid header value: {}", e)))?,
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

                headers_for_signing.insert(
                    "host",
                    HeaderValue::from_str(&host_header).map_err(|e| {
                        TtsError::InternalError(format!("Invalid header value: {}", e))
                    })?,
                );
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
            "{}/{}/{}/polly/aws4_request",
            self.access_key, date_stamp, self.region
        );
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={credential}, SignedHeaders={signed_headers}, Signature={signature}"
        );

        parts.headers.insert(
            "authorization",
            HeaderValue::from_str(&authorization)
                .map_err(|e| TtsError::InternalError(format!("Invalid header value: {}", e)))?,
        );

        Ok(Request::from_parts(parts, body))
    }

    fn create_canonical_request(
        &self,
        method: &http::Method,
        uri: &http::Uri,
        headers: &HeaderMap,
        content_sha256: &str,
    ) -> String {
        let canonical_uri = self.canonical_uri(uri.path());

        let canonical_query_string = self.canonical_query_string(uri.query().unwrap_or(""));

        let canonical_headers = self.canonical_headers(headers);

        let signed_headers = self.get_signed_headers(headers);

        let hashed_payload = content_sha256;

        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method.as_str().to_uppercase(),
            canonical_uri,
            canonical_query_string,
            canonical_headers,
            signed_headers,
            hashed_payload
        );

        canonical_request
    }

    fn canonical_uri(&self, path: &str) -> String {
        if path.is_empty() {
            "/".to_string()
        } else {
            // URI encode each segment
            let segments: Vec<String> = path
                .split('/')
                .map(|segment| utf8_percent_encode(segment, URI_ENCODE_SET).to_string())
                .collect();
            segments.join("/")
        }
    }

    fn canonical_query_string(&self, query: &str) -> String {
        if query.is_empty() {
            return String::new();
        }

        let mut params: Vec<(String, String)> = query
            .split('&')
            .map(|param| {
                if let Some(eq_pos) = param.find('=') {
                    let key = &param[..eq_pos];
                    let value = &param[eq_pos + 1..];
                    (
                        utf8_percent_encode(key, QUERY_ENCODE_SET).to_string(),
                        utf8_percent_encode(value, QUERY_ENCODE_SET).to_string(),
                    )
                } else {
                    (
                        utf8_percent_encode(param, QUERY_ENCODE_SET).to_string(),
                        String::new(),
                    )
                }
            })
            .collect();

        params.sort_by(|a, b| a.0.cmp(&b.0));

        params
            .into_iter()
            .map(|(key, value)| {
                if value.is_empty() {
                    key
                } else {
                    format!("{key}={value}")
                }
            })
            .collect::<Vec<_>>()
            .join("&")
    }

    fn canonical_headers(&self, headers: &HeaderMap) -> String {
        let mut canonical_headers = String::new();

        let mut sorted_headers: Vec<_> = headers
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_lowercase(),
                    value.to_str().unwrap_or("").trim(),
                )
            })
            .collect();
        sorted_headers.sort_by(|a, b| a.0.cmp(&b.0));

        for (name, value) in sorted_headers {
            canonical_headers.push_str(&format!("{name}:{value}\n"));
        }

        canonical_headers
    }

    fn get_signed_headers(&self, headers: &HeaderMap) -> String {
        let mut signed_headers: Vec<String> = headers
            .keys()
            .map(|key| key.as_str().to_lowercase())
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
        let algorithm = "AWS4-HMAC-SHA256";
        let credential_scope = format!("{}/{}/polly/aws4_request", date_stamp, self.region);

        let hashed_canonical_request = self.hash_payload(canonical_request.as_bytes());

        let string_to_sign =
            format!("{algorithm}\n{amz_date}\n{credential_scope}\n{hashed_canonical_request}");

        string_to_sign
    }

    fn calculate_signature(
        &self,
        string_to_sign: &str,
        date_stamp: &str,
    ) -> Result<String, TtsError> {
        let secret = format!("AWS4{}", self.secret_key);

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| TtsError::InternalError(format!("HMAC error: {}", e)))?;
        mac.update(date_stamp.as_bytes());
        let date_key = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&date_key)
            .map_err(|e| TtsError::InternalError(format!("HMAC error: {}", e)))?;
        mac.update(self.region.as_bytes());
        let date_region_key = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&date_region_key)
            .map_err(|e| TtsError::InternalError(format!("HMAC error: {}", e)))?;
        mac.update(b"polly");
        let date_region_service_key = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&date_region_service_key)
            .map_err(|e| TtsError::InternalError(format!("HMAC error: {}", e)))?;
        mac.update(b"aws4_request");
        let signing_key = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&signing_key)
            .map_err(|e| TtsError::InternalError(format!("HMAC error: {}", e)))?;
        mac.update(string_to_sign.as_bytes());
        let signature = mac.finalize().into_bytes();

        Ok(hex::encode(signature))
    }

    fn hash_payload(&self, payload: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(payload);
        hex::encode(hasher.finalize())
    }
}

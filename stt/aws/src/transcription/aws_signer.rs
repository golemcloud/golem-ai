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
    S3,
    Transcribe,
}

impl fmt::Display for AwsService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AwsService::S3 => write!(f, "s3"),
            AwsService::Transcribe => write!(f, "transcribe"),
        }
    }
}

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

/// AWS-specific percent-encoding set for query strings
const QUERY_ENCODE_SET: &AsciiSet = &URI_ENCODE_SET.add(b'=').add(b'&').add(b'+');

pub struct AwsSignatureV4 {
    access_key: String,
    secret_key: String,
    region: String,
    service: String,
}

impl AwsSignatureV4 {
    pub fn new(
        access_key: String,
        secret_key: String,
        region: String,
        service: AwsService,
    ) -> Self {
        Self {
            access_key,
            secret_key,
            region,
            service: service.to_string(),
        }
    }

    pub fn for_s3(access_key: String, secret_key: String, region: String) -> Self {
        Self::new(access_key, secret_key, region, AwsService::S3)
    }

    pub fn for_transcribe(access_key: String, secret_key: String, region: String) -> Self {
        Self::new(access_key, secret_key, region, AwsService::Transcribe)
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
            "{}/{}/{}/{}/aws4_request",
            self.access_key, date_stamp, self.region, self.service
        );
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={credential}, SignedHeaders={signed_headers}, Signature={signature}"
        );

        parts
            .headers
            .insert("authorization", HeaderValue::from_str(&authorization)?);

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
        let credential_scope = format!(
            "{}/{}/{}/aws4_request",
            date_stamp, self.region, self.service
        );

        let hashed_canonical_request = self.hash_payload(canonical_request.as_bytes());

        let string_to_sign =
            format!("{algorithm}\n{amz_date}\n{credential_scope}\n{hashed_canonical_request}");

        string_to_sign
    }

    fn calculate_signature(&self, string_to_sign: &str, date_stamp: &str) -> Result<String, Error> {
        let secret = format!("AWS4{}", self.secret_key);

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())?;
        mac.update(date_stamp.as_bytes());
        let date_key = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&date_key)?;
        mac.update(self.region.as_bytes());
        let date_region_key = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&date_region_key)?;
        mac.update(self.service.as_bytes());
        let date_region_service_key = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&date_region_service_key)?;
        mac.update(b"aws4_request");
        let signing_key = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&signing_key)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    use aws_credential_types::Credentials;
    use aws_sigv4::{
        http_request::{sign, SignableBody, SignableRequest, SigningSettings},
        sign::v4,
    };
    use http::Method;

    fn sign_with_aws_sdk(
        mut request: Request<Bytes>,
        access_key: &str,
        secret_key: &str,
        region: &str,
        service: &str,
        timestamp: DateTime<Utc>,
    ) -> Request<Bytes> {
        let creds = Credentials::new(access_key, secret_key, None, None, "iam");
        let identity = creds.into();

        let signing_settings = SigningSettings::default();
        let signing_params = v4::SigningParams::builder()
            .identity(&identity)
            .region(region)
            .name(service)
            .time(timestamp.into())
            .settings(signing_settings)
            .build()
            .unwrap()
            .into();

        let mut hasher = Sha256::new();
        hasher.update(request.body());
        let hashed_content = hex::encode(hasher.finalize());

        request.headers_mut().append(
            "x-amz-content-sha256",
            HeaderValue::from_str(&hashed_content).unwrap(),
        );

        let signable_request = SignableRequest::new(
            request.method().as_str(),
            request.uri().to_string(),
            request
                .headers()
                .iter()
                .map(|(k, v)| (k.as_str(), std::str::from_utf8(v.as_bytes()).unwrap())),
            SignableBody::Bytes(request.body().as_ref()),
        )
        .unwrap();

        let (signing_instructions, _signature) = sign(signable_request, &signing_params)
            .unwrap()
            .into_parts();
        signing_instructions.apply_to_request_http1x(&mut request);

        request
    }

    #[test]
    fn test_uri_encoding_all_characters() {
        let signer = AwsSignatureV4::for_s3(
            "test".to_string(),
            "test".to_string(),
            "us-east-1".to_string(),
        );

        assert_eq!(signer.canonical_uri("test file.txt"), "test%20file.txt");

        assert_eq!(signer.canonical_uri("test!file.txt"), "test%21file.txt");

        assert_eq!(signer.canonical_uri("test\"file.txt"), "test%22file.txt");

        assert_eq!(signer.canonical_uri("test#file.txt"), "test%23file.txt");

        assert_eq!(signer.canonical_uri("test$file.txt"), "test%24file.txt");

        assert_eq!(signer.canonical_uri("test%file.txt"), "test%25file.txt");

        assert_eq!(signer.canonical_uri("test'file.txt"), "test%27file.txt");

        assert_eq!(signer.canonical_uri("test(file.txt"), "test%28file.txt");

        assert_eq!(signer.canonical_uri("test)file.txt"), "test%29file.txt");

        assert_eq!(signer.canonical_uri("test*file.txt"), "test%2Afile.txt");

        assert_eq!(signer.canonical_uri("test,file.txt"), "test%2Cfile.txt");

        assert_eq!(signer.canonical_uri("folder/file.txt"), "folder/file.txt");

        assert_eq!(signer.canonical_uri("test:file.txt"), "test%3Afile.txt");

        assert_eq!(signer.canonical_uri("test;file.txt"), "test%3Bfile.txt");

        assert_eq!(signer.canonical_uri("test?file.txt"), "test%3Ffile.txt");

        assert_eq!(signer.canonical_uri("test@file.txt"), "test%40file.txt");

        assert_eq!(signer.canonical_uri("test[file.txt"), "test%5Bfile.txt");

        assert_eq!(signer.canonical_uri("test\\file.txt"), "test%5Cfile.txt");

        assert_eq!(signer.canonical_uri("test]file.txt"), "test%5Dfile.txt");

        assert_eq!(signer.canonical_uri("test^file.txt"), "test%5Efile.txt");

        assert_eq!(signer.canonical_uri("test`file.txt"), "test%60file.txt");

        assert_eq!(signer.canonical_uri("test{file.txt"), "test%7Bfile.txt");

        assert_eq!(signer.canonical_uri("test|file.txt"), "test%7Cfile.txt");

        assert_eq!(signer.canonical_uri("test}file.txt"), "test%7Dfile.txt");

        assert_eq!(signer.canonical_uri("test~file.txt"), "test~file.txt");

        assert_eq!(
            signer.canonical_uri("test-file_123.txt"),
            "test-file_123.txt"
        );
    }

    #[test]
    fn test_query_encoding_all_characters() {
        let signer = AwsSignatureV4::for_s3(
            "test".to_string(),
            "test".to_string(),
            "us-east-1".to_string(),
        );

        assert_eq!(
            signer.canonical_query_string("key=value=with=equals"),
            "key=value%3Dwith%3Dequals",
        );

        assert_eq!(
            signer.canonical_query_string("key=value+with+plus"),
            "key=value%2Bwith%2Bplus",
        );

        assert_eq!(
            signer.canonical_query_string("key=value with spaces"),
            "key=value%20with%20spaces",
        );

        assert_eq!(
            signer.canonical_query_string("filter=name=\"John Doe\"&sort=date:desc"),
            "filter=name%3D%22John%20Doe%22&sort=date%3Adesc",
        );

        assert_eq!(
            signer.canonical_query_string("z-param=last&a-param=first&m-param=middle"),
            "a-param=first&m-param=middle&z-param=last",
        );
    }

    #[test]
    fn test_s3_get_object_authorization_header() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";

        let signer = AwsSignatureV4::for_s3(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
        );

        let request = Request::builder()
            .method(Method::GET)
            .uri("s3://examplebucket.s3.amazonaws.com/foo/bar/test@file.txt")
            .header("Range", "bytes=0-9")
            .body(Bytes::new())
            .unwrap();

        let request_for_aws_sdk = request.clone();

        let timestamp = DateTime::parse_from_rfc2822("Fri, 24 May 2013 00:00:00 GMT")
            .unwrap()
            .with_timezone(&Utc);

        let result = signer.sign_request(request, timestamp);
        assert!(result.is_ok(), "Failed to sign request: {:?}", result.err());

        let signed_request = result.unwrap();

        let auth_header = signed_request
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();

        let aws_signed_request = sign_with_aws_sdk(
            request_for_aws_sdk,
            access_key,
            secret_key,
            region,
            "s3",
            timestamp,
        );

        let expected_auth = aws_signed_request
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();

        assert_eq!(auth_header, expected_auth, "Authorization header mismatch");
    }

    #[test]
    fn test_s3_put_object_authorization_header() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";

        let signer = AwsSignatureV4::for_s3(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
        );

        let request = Request::builder()
            .method(Method::PUT)
            .uri("s3://examplebucket.s3.amazonaws.com/test$file.text")
            .header("Date", "Fri, 24 May 2013 00:00:00 GMT")
            .header("x-amz-storage-class", "REDUCED_REDUNDANCY")
            .body(b"Welcome to Amazon S3.".to_vec().into())
            .unwrap();

        let request_for_aws_sdk = request.clone();

        let timestamp = DateTime::parse_from_rfc2822("Fri, 24 May 2013 00:00:00 GMT")
            .unwrap()
            .with_timezone(&Utc);

        let result = signer.sign_request(request, timestamp);
        assert!(result.is_ok(), "Failed to sign request: {:?}", result.err());

        let signed_request = result.unwrap();

        let auth_header = signed_request
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();

        let aws_signed_request = sign_with_aws_sdk(
            request_for_aws_sdk,
            access_key,
            secret_key,
            region,
            "s3",
            timestamp,
        );

        let expected_auth = aws_signed_request
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();

        assert_eq!(auth_header, expected_auth, "Authorization header mismatch");
    }

    #[test]
    fn test_s3_list_objects_authorization_header() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";

        let signer = AwsSignatureV4::for_s3(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
        );

        let request = Request::builder()
            .method(Method::GET)
            .uri("s3://examplebucket.s3.amazonaws.com/?max-keys=2&prefix=J")
            .body(Bytes::new())
            .unwrap();

        let request_for_aws_sdk = request.clone();

        let timestamp = DateTime::parse_from_rfc2822("Fri, 24 May 2013 00:00:00 GMT")
            .unwrap()
            .with_timezone(&Utc);

        let result = signer.sign_request(request, timestamp);
        assert!(result.is_ok(), "Failed to sign request: {:?}", result.err());

        let signed_request = result.unwrap();

        let auth_header = signed_request
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();

        let aws_signed_request = sign_with_aws_sdk(
            request_for_aws_sdk,
            access_key,
            secret_key,
            region,
            "s3",
            timestamp,
        );

        let expected_auth = aws_signed_request
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();

        assert_eq!(auth_header, expected_auth, "Authorization header mismatch");
    }

    #[test]
    fn test_batch_transcription_authorization_header() {
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";

        let signer = AwsSignatureV4::for_transcribe(
            access_key.to_string(),
            secret_key.to_string(),
            region.to_string(),
        );

        let body = r#"
            {
                "TranscriptionJobName": "my-first-transcription-job",
                "LanguageCode": "en-US",
                "Media": {
                    "MediaFileUri": "s3://amzn-s3-demo-bucket/my-input-files/my-media-file.flac"
                },
                "OutputBucketName": "amzn-s3-demo-bucket",
                "OutputKey": "my-output-files/"
            }
            "#;

        let request = Request::builder()
            .method(Method::GET)
            .uri("https://transcribe.us-east-1.amazonaws.com:443/") // test also that in case of https port is stripped
            .body(body.as_bytes().into())
            .unwrap();

        let request_for_aws_sdk = request.clone();

        // Parse the Date header directly: "Fri, 24 May 2013 00:00:00 GMT"
        let timestamp = DateTime::parse_from_rfc2822("Fri, 24 May 2013 00:00:00 GMT")
            .unwrap()
            .with_timezone(&Utc);

        let result = signer.sign_request(request, timestamp);
        assert!(result.is_ok(), "Failed to sign request: {:?}", result.err());

        let signed_request = result.unwrap();

        let auth_header = signed_request
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();

        let aws_signed_request = sign_with_aws_sdk(
            request_for_aws_sdk,
            access_key,
            secret_key,
            region,
            "transcribe",
            timestamp,
        );

        let expected_auth = aws_signed_request
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();

        assert_eq!(auth_header, expected_auth, "Authorization header mismatch");
    }
}

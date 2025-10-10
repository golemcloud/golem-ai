use hmac::{Hmac, Mac};
use reqwest::header::HeaderMap;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;

use crate::polly::Polly;

impl Polly {
    pub fn generate_sigv4_headers(
        &self,
        method: &str,
        uri: &str,
        body: &str,
    ) -> Result<HeaderMap, Box<dyn std::error::Error>> {
        let host = &format!("polly.{}.amazonaws.com", self.region);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let timestamp = OffsetDateTime::from_unix_timestamp(now.as_secs() as i64).unwrap();

        let date_str = format!(
            "{:04}{:02}{:02}",
            timestamp.year(),
            timestamp.month() as u8,
            timestamp.day()
        );
        let datetime_str = format!(
            "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
            timestamp.year(),
            timestamp.month() as u8,
            timestamp.day(),
            timestamp.hour(),
            timestamp.minute(),
            timestamp.second()
        );

        let (canonical_uri, canonical_query_string) = if let Some(query_pos) = uri.find('?') {
            let path = &uri[..query_pos];
            let query = &uri[query_pos + 1..];

            let encoded_path = if path.contains(':') {
                path.replace(':', "%3A")
            } else {
                path.to_string()
            };

            let mut query_params: Vec<&str> = query.split('&').collect();
            query_params.sort();
            (encoded_path, query_params.join("&"))
        } else {
            let encoded_path = if uri.contains(':') {
                uri.replace(':', "%3A")
            } else {
                uri.to_string()
            };
            (encoded_path, String::new())
        };

        let mut headers = BTreeMap::new();
        headers.insert("content-type", "application/x-amz-json-1.0");
        headers.insert("x-amz-target", "Polly_2016-06-10.SynthesizeSpeech");
        headers.insert("host", host);
        headers.insert("x-amz-date", &datetime_str);

        let canonical_headers = headers
            .iter()
            .map(|(k, v)| format!("{}:{}", k.to_lowercase().trim(), v.trim()))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";

        let signed_headers = headers
            .keys()
            .map(|k| k.to_lowercase())
            .collect::<Vec<_>>()
            .join(";");

        let payload_hash = format!("{:x}", Sha256::digest(body.as_bytes()));

        let canonical_request = format!(
    "{method}\n{canonical_uri}\n{canonical_query_string}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
);

        let credential_scope = format!("{date_str}/{}/polly/aws4_request", self.region.clone());
        let canonical_request_hash = format!("{:x}", Sha256::digest(canonical_request.as_bytes()));
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{datetime_str}\n{credential_scope}\n{canonical_request_hash}"
        );

        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(
            format!("AWS4{}", self.secret_access_key.clone()).as_bytes(),
        )?;
        mac.update(date_str.as_bytes());
        let date_key = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&date_key)?;
        mac.update(self.region.clone().as_bytes());
        let region_key = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&region_key)?;
        mac.update("polly".as_bytes());
        let service_key = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&service_key)?;
        mac.update(b"aws4_request");
        let signing_key = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&signing_key)?;
        mac.update(string_to_sign.as_bytes());
        let signature = format!("{:x}", mac.finalize().into_bytes());

        let auth_header = format!(
    "AWS4-HMAC-SHA256 Credential={}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}",self.access_key_id.clone()
);

        let mut headers = HeaderMap::new();
        headers.insert("authorization", auth_header.parse().unwrap());
        headers.insert("x-amz-date", datetime_str.parse().unwrap());
        headers.insert(
            "content-type",
            "application/x-amz-json-1.0".parse().unwrap(),
        );

        Ok(headers)
    }
}

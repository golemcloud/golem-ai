use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use chrono::Utc;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC};

const RPC_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

pub enum AwsService {
    Polly,
}

impl AwsService {
    pub fn as_str(&self) -> &str {
        match self {
            AwsService::Polly => "polly",
        }
    }
}

pub struct AwsSignatureV4 {
    access_key: String,
    secret_key: String,
    region: String,
    service: AwsService,
}

impl AwsSignatureV4 {
    pub fn new(access_key: String, secret_key: String, region: String, service: AwsService) -> Self {
        Self { access_key, secret_key, region, service }
    }

    pub fn sign_request(
        &self,
        method: &str,
        url: &str,
        headers: &mut http::HeaderMap,
        body: &[u8],
    ) -> Result<(), String> {
        let now = Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let datestamp = now.format("%Y%m%d").to_string();

        headers.insert("x-amz-date", amz_date.parse().unwrap());
        
        let parsed_url = url::Url::parse(url).map_err(|e: url::ParseError| e.to_string())?;
        let host = parsed_url.host_str().ok_or("No host")?.to_string();
        headers.insert("host", host.parse().unwrap());

        let canonical_uri = "/";
        let canonical_querystring = ""; 

        let mut signed_headers_list: Vec<String> = headers.keys()
            .map(|k| k.as_str().to_lowercase())
            .collect();
        signed_headers_list.sort();
        let signed_headers = signed_headers_list.join(";");

        let mut canonical_headers = String::new();
        for header in &signed_headers_list {
            let val = headers.get(header).unwrap().to_str().unwrap().trim();
            canonical_headers.push_str(&format!("{}:{}\n", header, val));
        }

        let payload_hash = hex::encode(Sha256::digest(body));
        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method, canonical_uri, canonical_querystring, canonical_headers, signed_headers, payload_hash
        );

        let credential_scope = format!("{}/{}/{}/aws4_request", datestamp, self.region, self.service.as_str());
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            amz_date,
            credential_scope,
            hex::encode(Sha256::digest(canonical_request.as_bytes()))
        );

        let signing_key = self.get_signature_key(&datestamp);
        let signature = hex::encode(self.hmac_sha256(&signing_key, string_to_sign.as_bytes()));

        let authorization_header = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.access_key, credential_scope, signed_headers, signature
        );

        headers.insert("Authorization", authorization_header.parse().unwrap());

        Ok(())
    }

    fn hmac_sha256(&self, key: &[u8], data: &[u8]) -> Vec<u8> {
        let mut mac = Hmac::<Sha256>::new_from_slice(key).unwrap();
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }

    fn get_signature_key(&self, datestamp: &str) -> Vec<u8> {
        let k_secret = format!("AWS4{}", self.secret_key);
        let k_date = self.hmac_sha256(k_secret.as_bytes(), datestamp.as_bytes());
        let k_region = self.hmac_sha256(&k_date, self.region.as_bytes());
        let k_service = self.hmac_sha256(&k_region, self.service.as_str().as_bytes());
        self.hmac_sha256(&k_service, b"aws4_request")
    }
}

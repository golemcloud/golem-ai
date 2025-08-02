use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

fn hex_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn hmac_sha256(key: &[u8], data: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(data.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

fn derive_signing_key(secret: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
    let k_date = hmac_sha256(format!("AWS4{secret}").as_bytes(), date);
    let k_region = hmac_sha256(&k_date, region);
    let k_service = hmac_sha256(&k_region, service);
    hmac_sha256(&k_service, "aws4_request")
}

pub struct SigV4Params {
    pub method: String,
    pub service: String,
    pub region: String,
    pub host: String,
    pub canonical_uri: String,
    pub canonical_querystring: String,
    pub payload_sha256: String,
    pub access_key: String,
    pub secret_key: String,
    pub session_token: Option<String>,
    pub amz_date: String,   // yyyymmddThhmmssZ
    pub date_stamp: String, // yyyymmdd
    pub content_type: Option<String>,
}

pub fn sign(params: SigV4Params) -> (String, Vec<(String, String)>) {
    // Canonical headers
    let mut headers: Vec<(String, String)> = vec![
        ("host".into(), params.host.clone()),
        ("x-amz-date".into(), params.amz_date.clone()),
    ];
    if let Some(ct) = &params.content_type {
        headers.push(("content-type".into(), ct.clone()));
    }
    if let Some(t) = &params.session_token {
        headers.push(("x-amz-security-token".into(), t.clone()));
    }
    headers.sort_by(|a, b| a.0.cmp(&b.0));
    let signed_headers = headers
        .iter()
        .map(|(k, _)| k.clone())
        .collect::<Vec<_>>()
        .join(";");

    let canonical_headers = headers
        .iter()
        .map(|(k, v)| format!("{}:{}\n", k.to_ascii_lowercase(), v.trim()))
        .collect::<String>();

    let canonical_request = format!(
        "{method}\n{uri}\n{query}\n{headers}\n{signed}\n{payload}",
        method = params.method,
        uri = params.canonical_uri,
        query = params.canonical_querystring,
        headers = canonical_headers,
        signed = signed_headers,
        payload = params.payload_sha256
    );

    let algorithm = "AWS4-HMAC-SHA256";
    let credential_scope = format!(
        "{}/{}/{}/aws4_request",
        params.date_stamp, params.region, params.service
    );
    let string_to_sign = format!(
        "{algorithm}\n{amz_date}\n{scope}\n{hash}",
        algorithm = algorithm,
        amz_date = params.amz_date,
        scope = credential_scope,
        hash = hex_sha256(canonical_request.as_bytes())
    );

    let signing_key = derive_signing_key(
        &params.secret_key,
        &params.date_stamp,
        &params.region,
        &params.service,
    );
    let signature = hex::encode(hmac_sha256(&signing_key, &string_to_sign));

    let authorization_header = format!(
        "{algorithm} Credential={access_key}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
        algorithm = algorithm,
        access_key = params.access_key,
        scope = credential_scope,
        signed_headers = signed_headers,
        signature = signature
    );

    (authorization_header, headers)
}

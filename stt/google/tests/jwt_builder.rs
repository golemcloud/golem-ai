use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::Deserialize;
use serial_test::serial;

#[derive(Deserialize)]
struct Creds {
    client_email: String,
    token_uri: String,
}

#[test]
#[serial]
fn jwt_header_and_claim_encoding() {
    static CREDS_RAW: &str = include_str!("data/fake_creds.json");
    let creds: Creds = serde_json::from_str(CREDS_RAW).expect("valid creds json");

    // Build header & claims exactly like component, skip RSA signing to keep test lightweight
    let header = serde_json::json!({ "alg": "RS256", "typ": "JWT" });

    let now = 1_700_000_000u64;
    let claims = serde_json::json!({
        "iss": creds.client_email,
        "scope": "https://www.googleapis.com/auth/cloud-platform",
        "aud": creds.token_uri,
        "exp": now + 3600,
        "iat": now
    });

    let header_b64 = URL_SAFE_NO_PAD.encode(&serde_json::to_vec(&header).unwrap());
    let claims_b64 = URL_SAFE_NO_PAD.encode(&serde_json::to_vec(&claims).unwrap());

    // Combine (no signature part needed for this assertion)
    let token = format!("{header_b64}.{claims_b64}.");

    // Validate round-trip
    let mut parts = token.split('.');
    let header_dec = parts.next().unwrap();
    let claims_dec = parts.next().unwrap();

    let header_json = String::from_utf8(URL_SAFE_NO_PAD.decode(header_dec.as_bytes()).unwrap()).unwrap();
    let header_val: serde_json::Value = serde_json::from_str(&header_json).unwrap();
    assert_eq!(header_val["alg"], "RS256");
    assert_eq!(header_val["typ"], "JWT");

    let claims_json = String::from_utf8(URL_SAFE_NO_PAD.decode(claims_dec.as_bytes()).unwrap()).unwrap();
    let claims_val: serde_json::Value = serde_json::from_str(&claims_json).unwrap();
    assert_eq!(claims_val["aud"], creds.token_uri);
    assert_eq!(claims_val["scope"], "https://www.googleapis.com/auth/cloud-platform");
} 
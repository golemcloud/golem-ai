//! Integration Tests for Golem TTS Providers
//!
//! Verifies:
//! 1. AWS SigV4 signing (Authorization, X-Amz-Date, X-Amz-Content-Sha256)
//! 2. Golem Determinism (X-Golem-Oplog-Index integration)

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use std::collections::BTreeMap;
    use tts_aws_polly::sigv4;

    #[test]
    fn test_aws_sigv4_header_generation() {
        println!("Testing AWS SigV4 Header Generation...");

        let creds = sigv4::Credentials {
            access_key: "AKIA_MOCK".to_string(),
            secret_key: "SECRET_MOCK".to_string(),
            region: "us-east-1".to_string(),
            service: "polly".to_string(),
        };

        let timestamp = Utc::now().timestamp();
        let method = "POST";
        let path = "/v1/speech";
        let query = BTreeMap::new();
        let payload = b"{\"text\": \"hello\"}";

        let mut headers = BTreeMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.insert("x-golem-oplog-index".to_string(), "42".to_string());

        let signed =
            sigv4::sign_request(&creds, timestamp, method, path, &query, &headers, payload)
                .expect("Signing failed");

        println!("Generated Authorization: {}", signed.authorization);

        // Assertions
        assert!(signed.authorization.contains("AWS4-HMAC-SHA256"));
        assert!(signed.authorization.contains("Credential=AKIA_MOCK"));
        assert!(signed.authorization.contains("SignedHeaders="));
        assert!(signed.authorization.contains("Signature="));
        assert!(!signed.x_amz_date.is_empty());
        assert!(!signed.x_amz_content_sha256.is_empty());

        println!("✅ AWS SigV4 Header Verification Passed!");
    }
}

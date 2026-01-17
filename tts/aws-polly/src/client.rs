//! Durable HTTP client for AWS Polly
//!
//! Handles external communication with AWS using SigV4 signing.

use crate::sigv4;
use crate::types;
use crate::wit_types;
use golem_rust::mark_atomic_operation;
use std::collections::BTreeMap;
use wasi::http::outgoing_handler;
use wasi::http::types as http_types;

/// Get AWS credentials from environment variables
fn get_credentials() -> Result<sigv4::Credentials, wit_types::TtsError> {
    let access_key = std::env::var("AWS_ACCESS_KEY_ID")
        .map_err(|_| types::auth_error("AWS_ACCESS_KEY_ID not set"))?;
    let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY")
        .map_err(|_| types::auth_error("AWS_SECRET_ACCESS_KEY not set"))?;
    let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());

    return Ok(sigv4::Credentials {
        access_key,
        secret_key,
        region,
        service: "polly".to_string(),
    });
}

/// Get current Unix timestamp using WASI clocks
fn get_timestamp() -> i64 {
    let now = wasi::clocks::wall_clock::now();
    return now.seconds as i64;
}

/// Send a signed HTTP request to AWS Polly
fn send_signed_request(
    method: &str,
    path: &str,
    query: BTreeMap<String, String>,
    payload: Vec<u8>,
) -> Result<Vec<u8>, wit_types::TtsError> {
    let creds = get_credentials()?;
    let timestamp = get_timestamp();

    // 1. Prepare base headers for signing
    let mut headers_to_sign = BTreeMap::new();
    if !payload.is_empty() {
        headers_to_sign.insert("content-type".to_string(), "application/json".to_string());
    }

    // Add Golem Oplog Index to make signature unique per call (avoids replay issues)
    #[cfg(target_arch = "wasm32")]
    let oplog_index = golem_rust::bindings::golem::api::host::get_oplog_index().to_string();
    #[cfg(not(target_arch = "wasm32"))]
    let oplog_index = "0".to_string();

    headers_to_sign.insert("x-golem-oplog-index".to_string(), oplog_index.clone());

    // 2. Sign request
    let signed = sigv4::sign_request(
        &creds,
        timestamp,
        method,
        path,
        &query,
        &headers_to_sign,
        &payload,
    )
    .map_err(|e| types::internal_error(&format!("Signing failed: {:?}", e)))?;

    let path_with_query = if query.is_empty() {
        path.to_string()
    } else {
        let qs: Vec<String> = query.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
        format!("{}?{}", path, qs.join("&"))
    };

    let (scheme, authority) = match std::env::var("POLLY_ENDPOINT") {
        Ok(url) => {
            if url.starts_with("https://") {
                (http_types::Scheme::Https, url.replace("https://", ""))
            } else {
                (http_types::Scheme::Http, url.replace("http://", ""))
            }
        }
        Err(_) => (
            http_types::Scheme::Https,
            format!("polly.{}.amazonaws.com", creds.region),
        ),
    };

    let request = http_types::OutgoingRequest::new(http_types::Fields::new());
    request
        .set_method(&match method {
            "GET" => http_types::Method::Get,
            "POST" => http_types::Method::Post,
            _ => http_types::Method::Get,
        })
        .map_err(|_| types::internal_error("Failed to set method"))?;

    request
        .set_path_with_query(Some(&path_with_query))
        .map_err(|_| types::internal_error("Failed to set path"))?;

    request
        .set_scheme(Some(&scheme))
        .map_err(|_| types::internal_error("Failed to set scheme"))?;

    request
        .set_authority(Some(&authority))
        .map_err(|_| types::internal_error("Failed to set authority"))?;

    let headers = request.headers();
    headers
        .set(
            &"Authorization".to_string(),
            &[signed.authorization.as_bytes().to_vec()],
        )
        .map_err(|_| types::internal_error("Failed to set auth header"))?;
    headers
        .set(
            &"X-Amz-Date".to_string(),
            &[signed.x_amz_date.as_bytes().to_vec()],
        )
        .map_err(|_| types::internal_error("Failed to set date header"))?;
    headers
        .set(
            &"X-Amz-Content-Sha256".to_string(),
            &[signed.x_amz_content_sha256.as_bytes().to_vec()],
        )
        .map_err(|_| types::internal_error("Failed to set sha header"))?;
    headers
        .set(
            &"X-Golem-Oplog-Index".to_string(),
            &[oplog_index.as_bytes().to_vec()],
        )
        .map_err(|_| types::internal_error("Failed to set oplog header"))?;

    if !payload.is_empty() {
        headers
            .set(&"Content-Type".to_string(), &[b"application/json".to_vec()])
            .map_err(|_| types::internal_error("Failed to set content-type"))?;

        let body = request
            .body()
            .map_err(|_| types::internal_error("Failed to get body"))?;
        let stream = body
            .write()
            .map_err(|_| types::internal_error("Failed to get write stream"))?;
        stream
            .blocking_write_and_flush(&payload)
            .map_err(|_| types::internal_error("Failed to write payload"))?;
        http_types::OutgoingBody::finish(body, None)
            .map_err(|_| types::internal_error("Failed to finish body"))?;
    }

    // 4. Send request (Wrapped in atomic operation for durability)
    let _guard = mark_atomic_operation();
    let future = outgoing_handler::handle(request, None)
        .map_err(|_| types::internal_error("HTTP request failed"))?;

    // Wait for response
    let incoming_response = loop {
        match future.get() {
            Some(result) => match result {
                Ok(Ok(resp)) => break resp,
                Ok(Err(_)) => {
                    return Err(types::internal_error("HTTP request failed"));
                }
                Err(_) => {
                    return Err(types::internal_error("Failed to get response"));
                }
            },
            None => {
                let pollable = future.subscribe();
                wasi::io::poll::poll(&[&pollable]);
            }
        }
    };

    let status = incoming_response.status();

    let resp_body = incoming_response
        .consume()
        .map_err(|_| types::internal_error("Failed to consume response body"))?;
    let stream = resp_body
        .stream()
        .map_err(|_| types::internal_error("Failed to get response stream"))?;

    let mut data = Vec::new();
    while let Ok(chunk) = stream.blocking_read(16384) {
        if chunk.is_empty() {
            break;
        }
        data.extend(chunk);
    }

    if status < 200 || status >= 300 {
        let error_msg = String::from_utf8_lossy(&data);
        return Err(types::internal_error(&format!(
            "AWS Error ({}): {}",
            status, error_msg
        )));
    }

    let response = data;

    return Ok(response);
}

// ============================================================
// PUBLIC API FUNCTIONS
// ============================================================

/// Synthesize speech using AWS Polly
pub fn synthesize_api(
    voice_id: &str,
    text: &str,
    engine: &str,
    format: &str,
) -> Result<Vec<u8>, wit_types::TtsError> {
    let request = types::PollySynthesizeRequest {
        output_format: format.to_string(),
        text: text.to_string(),
        voice_id: voice_id.to_string(),
        engine: Some(engine.to_string()),
        language_code: None,
        sample_rate: None,
        text_type: None,
    };

    let payload = serde_json::to_vec(&request)
        .map_err(|_| types::internal_error("Failed to serialize request"))?;

    return send_signed_request("POST", "/v1/speech", BTreeMap::new(), payload);
}

/// List available voices from AWS Polly
pub fn list_voices_api() -> Result<types::PollyVoiceList, wit_types::TtsError> {
    let data = send_signed_request("GET", "/v1/voices", BTreeMap::new(), Vec::new())?;
    let voices: types::PollyVoiceList = serde_json::from_slice(&data)
        .map_err(|_| types::internal_error("Failed to deserialize voice list"))?;
    return Ok(voices);
}

//! HTTP Client for Deepgram Aura TTS API
//!
//! Adheres to best practices (no unwrap, explicit error mapping, durable).

use crate::types;
use golem_rust::mark_atomic_operation;
use wasi::http::outgoing_handler;
use wasi::http::types as http_types;

fn get_endpoint() -> String {
    std::env::var("DEEPGRAM_ENDPOINT").unwrap_or_else(|_| "api.deepgram.com".to_string())
}

/// Get API key from environment
fn get_api_key() -> Result<String, crate::bindings::exports::golem::tts::types::TtsError> {
    match std::env::var("DEEPGRAM_API_KEY") {
        Ok(key) => {
            if key.is_empty() {
                return Err(types::unauthorized_error("DEEPGRAM_API_KEY is empty"));
            }
            Ok(key)
        }
        Err(_) => Err(types::unauthorized_error(
            "DEEPGRAM_API_KEY environment variable not set",
        )),
    }
}

/// Perform POST request to Deepgram speak endpoint
pub fn post_speak(
    query_params: &str,
    body_bytes: &[u8],
) -> Result<Vec<u8>, crate::bindings::exports::golem::tts::types::TtsError> {
    let _guard = mark_atomic_operation();

    let api_key = get_api_key()?;
    let path = format!("/v1/speak?{}", query_params);

    let headers = match http_types::Fields::new() {
        h => h,
    };
    let _ = headers.append(
        &http_types::FieldKey::from("Authorization"),
        &format!("Token {}", api_key).as_bytes().to_vec(),
    );
    let _ = headers.append(
        &http_types::FieldKey::from("Content-Type"),
        &b"application/json".to_vec(),
    );

    let request = match http_types::OutgoingRequest::new(headers) {
        r => r,
    };

    let endpoint = get_endpoint();
    let scheme = if endpoint.contains("localhost") || endpoint.contains("127.0.0.1") {
        http_types::Scheme::Http
    } else {
        http_types::Scheme::Https
    };

    let _ = request.set_method(&http_types::Method::Post);
    let _ = request.set_path_with_query(Some(&path));
    let _ = request.set_scheme(Some(&scheme));
    let _ = request.set_authority(Some(&endpoint));

    let outgoing_body = request
        .body()
        .map_err(|_| types::internal_error("Failed to get request body"))?;
    let body_stream = outgoing_body
        .write()
        .map_err(|_| types::internal_error("Failed to get body stream"))?;

    body_stream
        .write(body_bytes)
        .map_err(|_| types::internal_error("Failed to write body chunk"))?;
    drop(body_stream);

    http_types::OutgoingBody::finish(outgoing_body, None)
        .map_err(|_| types::internal_error("Failed to finish body"))?;

    let future = outgoing_handler::handle(request, None)
        .map_err(|_| types::internal_error("HTTP handle failed"))?;

    let response = loop {
        match future.get() {
            Some(result) => match result {
                Ok(Ok(resp)) => break resp,
                Ok(Err(_)) => return Err(types::internal_error("HTTP request failed")),
                Err(_) => return Err(types::internal_error("Failed to get response")),
            },
            None => {
                let pollable = future.subscribe();
                wasi::io::poll::poll(&[&pollable]);
            }
        }
    };

    let status = response.status();
    let body = response
        .consume()
        .map_err(|_| types::internal_error("Failed to consume body"))?;
    let stream = body
        .stream()
        .map_err(|_| types::internal_error("Failed to get body stream"))?;

    let mut data = Vec::new();
    while let Ok(chunk) = stream.blocking_read(16384) {
        if chunk.is_empty() {
            break;
        }
        data.extend(chunk);
    }

    if status < 200 || status >= 300 {
        let err_text = String::from_utf8_lossy(&data);
        return Err(types::internal_error(&format!(
            "Deepgram API Error ({}): {}",
            status, err_text
        )));
    }

    Ok(data)
}

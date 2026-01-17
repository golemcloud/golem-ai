//! HTTP Client for Google Cloud TTS API
//!
//! All external API calls are wrapped in `mark_atomic_operation` for durability.
//! Adheres to best practices (no unwrap, explicit error mapping).

use crate::auth;
use crate::types;
use golem_rust::mark_atomic_operation;
use wasi::http::outgoing_handler;
use wasi::http::types as http_types;

fn get_endpoint() -> String {
    std::env::var("GOOGLE_TTS_ENDPOINT")
        .unwrap_or_else(|_| "https://texttospeech.googleapis.com/v1".to_string())
}

/// Perform a POST request to Google Cloud TTS
pub fn post_request(
    path: &str,
    body_bytes: &[u8],
) -> Result<Vec<u8>, crate::bindings::exports::golem::tts::types::TtsError> {
    let _guard = mark_atomic_operation();

    let api_key = auth::get_auth_key()?;
    let path_with_key = if path.contains('?') {
        format!("{}&key={}", path, api_key)
    } else {
        format!("{}?key={}", path, api_key)
    };

    let headers = match http_types::Fields::new() {
        h => h,
    };
    let _ = headers.append(
        &http_types::FieldKey::from("Content-Type"),
        &b"application/json".to_vec(),
    );

    let request = match http_types::OutgoingRequest::new(headers) {
        r => r,
    };

    let endpoint = get_endpoint();
    let (scheme, authority) = if endpoint.starts_with("https://") {
        (http_types::Scheme::Https, endpoint.replace("https://", ""))
    } else {
        (http_types::Scheme::Http, endpoint.replace("http://", ""))
    };

    let _ = request.set_method(&http_types::Method::Post);
    let _ = request.set_path_with_query(Some(&path_with_key));
    let _ = request.set_scheme(Some(&scheme));
    let _ = request.set_authority(Some(&authority));

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
            "Google API Error ({}): {}",
            status, err_text
        )));
    }

    Ok(data)
}

/// Perform a GET request (simulated via POST if path supports it, or standard GET)
pub fn get_request(
    path: &str,
) -> Result<Vec<u8>, crate::bindings::exports::golem::tts::types::TtsError> {
    let _guard = mark_atomic_operation();

    let api_key = auth::get_auth_key()?;
    let path_with_key = if path.contains('?') {
        format!("{}&key={}", path, api_key)
    } else {
        format!("{}?key={}", path, api_key)
    };

    let headers = match http_types::Fields::new() {
        h => h,
    };

    let request = match http_types::OutgoingRequest::new(headers) {
        r => r,
    };

    let _ = request.set_method(&http_types::Method::Get);
    let _ = request.set_path_with_query(Some(&path_with_key));
    let _ = request.set_scheme(Some(&http_types::Scheme::Https));
    let _ = request.set_authority(Some("texttospeech.googleapis.com"));

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
            "Google API Error ({}): {}",
            status, err_text
        )));
    }

    Ok(data)
}

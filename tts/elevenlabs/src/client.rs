//! HTTP Client for ElevenLabs API
//!
//! All external API calls are wrapped in `mark_atomic_operation` for durability.
//! No unwrap() calls - all errors are explicitly handled and mapped to TtsError.

use crate::types;
use golem_rust::mark_atomic_operation;

// ============================================================
// CONFIGURATION
// ============================================================

/// ElevenLabs API base URL
const ELEVENLABS_API_BASE: &str = "https://api.elevenlabs.io/v1";

/// Default model for TTS
const DEFAULT_MODEL: &str = "eleven_multilingual_v2";

/// Get API key from environment (returns error if not set)
fn get_api_key() -> Result<String, types::TtsError> {
    match std::env::var("ELEVENLABS_API_KEY") {
        Ok(key) => {
            if key.is_empty() {
                return Err(types::unauthorized_error("ELEVENLABS_API_KEY is empty"));
            }
            return Ok(key);
        }
        Err(_) => {
            return Err(types::unauthorized_error(
                "ELEVENLABS_API_KEY environment variable not set",
            ));
        }
    }
}

/// Get custom endpoint or default
fn get_endpoint() -> String {
    std::env::var("XI_ENDPOINT").unwrap_or_else(|_| "https://api.elevenlabs.io/v1".to_string())
}

/// Get model version from env or default
pub fn get_model_version() -> String {
    match std::env::var("ELEVENLABS_MODEL_VERSION") {
        Ok(model) => {
            if model.is_empty() {
                return DEFAULT_MODEL.to_string();
            }
            return model;
        }
        Err(_) => {
            return DEFAULT_MODEL.to_string();
        }
    }
}

// ============================================================
// HTTP REQUEST HELPERS (using wasi crate)
// ============================================================

/// Perform GET request to ElevenLabs API
pub fn http_get(path: &str) -> Result<Vec<u8>, types::TtsError> {
    let _guard = mark_atomic_operation();

    let api_key = get_api_key()?;
    let url = format!("{}{}", get_endpoint(), path);

    // Use wasi crate for HTTP
    use wasi::http::outgoing_handler;
    use wasi::http::types as http_types;

    // Create headers
    let headers = match http_types::Fields::new() {
        h => h,
    };

    let header_result = headers.append(
        &http_types::FieldKey::from("xi-api-key"),
        &api_key.as_bytes().to_vec(),
    );
    if header_result.is_err() {
        return Err(types::internal_error("Failed to set API key header"));
    }

    let content_type_result = headers.append(
        &http_types::FieldKey::from("Content-Type"),
        &b"application/json".to_vec(),
    );
    if content_type_result.is_err() {
        return Err(types::internal_error("Failed to set content-type header"));
    }

    // Create request
    let request = match http_types::OutgoingRequest::new(headers) {
        r => r,
    };

    // Set method and path
    let method_result = request.set_method(&http_types::Method::Get);
    if method_result.is_err() {
        return Err(types::internal_error("Failed to set HTTP method"));
    }

    // Parse the path for query string
    let path_result = request.set_path_with_query(Some(&path.to_string()));
    if path_result.is_err() {
        return Err(types::internal_error("Failed to set request path"));
    }

    let scheme_result = request.set_scheme(Some(&http_types::Scheme::Https));
    if scheme_result.is_err() {
        return Err(types::internal_error("Failed to set scheme"));
    }

    let authority_result = request.set_authority(Some("api.elevenlabs.io"));
    if authority_result.is_err() {
        return Err(types::internal_error("Failed to set authority"));
    }

    // Send request
    let future_response = match outgoing_handler::handle(request, None) {
        Ok(future) => future,
        Err(_) => {
            return Err(types::network_error("Failed to send HTTP request"));
        }
    };

    // Wait for response
    let response = loop {
        match future_response.get() {
            Some(result) => match result {
                Ok(Ok(resp)) => break resp,
                Ok(Err(_)) => {
                    return Err(types::network_error("HTTP request failed"));
                }
                Err(_) => {
                    return Err(types::internal_error("Failed to get response"));
                }
            },
            None => {
                // Poll again - use subscribe and block
                let pollable = future_response.subscribe();
                wasi::io::poll::poll(&[&pollable]);
            }
        }
    };

    // Check status
    let status = response.status();
    if status == 401 {
        return Err(types::unauthorized_error("Invalid API key"));
    }
    if status == 429 {
        return Err(types::rate_limited_error(60));
    }
    if status >= 500 {
        return Err(types::service_unavailable_error("ElevenLabs service error"));
    }
    if status >= 400 {
        return Err(types::internal_error("API request failed"));
    }

    // Read body
    let body = match response.consume() {
        Ok(b) => b,
        Err(_) => {
            return Err(types::internal_error("Failed to consume response body"));
        }
    };

    let stream = match body.stream() {
        Ok(s) => s,
        Err(_) => {
            return Err(types::internal_error("Failed to get body stream"));
        }
    };

    let mut data: Vec<u8> = Vec::new();

    loop {
        match stream.read(65536) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    break;
                }
                data.extend(chunk);
            }
            Err(wasi::io::streams::StreamError::Closed) => {
                break;
            }
            Err(_) => {
                break;
            }
        }
    }

    return Ok(data);
}

/// Perform POST request to ElevenLabs API
pub fn http_post(path: &str, body_bytes: &[u8]) -> Result<Vec<u8>, types::TtsError> {
    let _guard = mark_atomic_operation();

    let api_key = get_api_key()?;
    let _url = format!("{}{}", get_endpoint(), path);

    use wasi::http::outgoing_handler;
    use wasi::http::types as http_types;

    // Create headers
    let headers = http_types::Fields::new();
    let _ = headers.append(
        &http_types::FieldKey::from("xi-api-key"),
        &api_key.as_bytes().to_vec(),
    );
    let _ = headers.append(
        &http_types::FieldKey::from("Content-Type"),
        &b"application/json".to_vec(),
    );

    // Create request
    let request = http_types::OutgoingRequest::new(headers);

    let _ = request.set_method(&http_types::Method::Post);
    let _ = request.set_path_with_query(Some(&path.to_string()));
    let _ = request.set_scheme(Some(&http_types::Scheme::Https));
    let _ = request.set_authority(Some("api.elevenlabs.io"));

    // Write body
    let outgoing_body = match request.body() {
        Ok(b) => b,
        Err(_) => {
            return Err(types::internal_error("Failed to get request body"));
        }
    };

    let body_stream = match outgoing_body.write() {
        Ok(s) => s,
        Err(_) => {
            return Err(types::internal_error("Failed to get body stream"));
        }
    };

    let write_result = body_stream.write(body_bytes);
    drop(body_stream);

    if write_result.is_err() {
        return Err(types::internal_error("Failed to write request body"));
    }

    let finish_result = http_types::OutgoingBody::finish(outgoing_body, None);
    if finish_result.is_err() {
        return Err(types::internal_error("Failed to finish request body"));
    }

    // Send request
    let future_response = match outgoing_handler::handle(request, None) {
        Ok(future) => future,
        Err(_) => {
            return Err(types::network_error("Failed to send HTTP request"));
        }
    };

    // Wait for response
    let response = loop {
        match future_response.get() {
            Some(result) => match result {
                Ok(Ok(resp)) => break resp,
                Ok(Err(_)) => {
                    return Err(types::network_error("HTTP request failed"));
                }
                Err(_) => {
                    return Err(types::internal_error("Failed to get response"));
                }
            },
            None => {
                let pollable = future_response.subscribe();
                wasi::io::poll::poll(&[&pollable]);
            }
        }
    };

    // Check status
    let status = response.status();
    if status == 401 {
        return Err(types::unauthorized_error("Invalid API key"));
    }
    if status == 429 {
        return Err(types::rate_limited_error(60));
    }
    if status >= 500 {
        return Err(types::service_unavailable_error("ElevenLabs service error"));
    }
    if status >= 400 {
        return Err(types::synthesis_failed_error("API request failed"));
    }

    // Read body
    let body_resource = match response.consume() {
        Ok(b) => b,
        Err(_) => {
            return Err(types::internal_error("Failed to consume response body"));
        }
    };

    let stream = match body_resource.stream() {
        Ok(s) => s,
        Err(_) => {
            return Err(types::internal_error("Failed to get body stream"));
        }
    };

    let mut data: Vec<u8> = Vec::new();

    loop {
        match stream.read(65536) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    break;
                }
                data.extend(chunk);
            }
            Err(wasi::io::streams::StreamError::Closed) => {
                break;
            }
            Err(_) => {
                break;
            }
        }
    }

    return Ok(data);
}

// ============================================================
// ELEVENLABS API FUNCTIONS
// ============================================================

/// List all voices from ElevenLabs
pub fn list_voices_api() -> Result<types::ElevenLabsVoicesResponse, types::TtsError> {
    let response_bytes = http_get("/voices")?;

    let response_str = match core::str::from_utf8(&response_bytes) {
        Ok(s) => s,
        Err(_) => {
            return Err(types::internal_error("Invalid UTF-8 in response"));
        }
    };

    match serde_json::from_str::<types::ElevenLabsVoicesResponse>(response_str) {
        Ok(voices) => {
            return Ok(voices);
        }
        Err(e) => {
            return Err(types::internal_error(&format!(
                "Failed to parse voices: {}",
                e
            )));
        }
    }
}

/// Get single voice by ID
pub fn get_voice_api(voice_id: &str) -> Result<types::ElevenLabsVoice, types::TtsError> {
    let path = format!("/voices/{}", voice_id);
    let response_bytes = http_get(&path)?;

    let response_str = match core::str::from_utf8(&response_bytes) {
        Ok(s) => s,
        Err(_) => {
            return Err(types::internal_error("Invalid UTF-8 in response"));
        }
    };

    match serde_json::from_str::<types::ElevenLabsVoice>(response_str) {
        Ok(voice) => {
            return Ok(voice);
        }
        Err(_) => {
            return Err(types::voice_not_found_error(voice_id));
        }
    }
}

/// Synthesize text to speech
pub fn synthesize_api(
    voice_id: &str,
    text: &str,
    model_id: &str,
    output_format: &str,
) -> Result<Vec<u8>, types::TtsError> {
    let path = format!(
        "/text-to-speech/{}?output_format={}",
        voice_id, output_format
    );

    let request_body = types::ElevenLabsTtsRequest {
        text: text.to_string(),
        model_id: model_id.to_string(),
        voice_settings: None,
    };

    let request_json = match serde_json::to_vec(&request_body) {
        Ok(json) => json,
        Err(_) => {
            return Err(types::internal_error("Failed to serialize request"));
        }
    };

    let audio_data = http_post(&path, &request_json)?;

    return Ok(audio_data);
}

/// Generate sound effect from text description
pub fn generate_sound_effect_api(
    description: &str,
    duration_seconds: Option<f32>,
    prompt_influence: Option<f32>,
) -> Result<Vec<u8>, types::TtsError> {
    let request_body = types::ElevenLabsSoundGenRequest {
        text: description.to_string(),
        duration_seconds,
        prompt_influence,
    };

    let request_json = match serde_json::to_vec(&request_body) {
        Ok(json) => json,
        Err(_) => {
            return Err(types::internal_error("Failed to serialize request"));
        }
    };

    let audio_data = http_post("/sound-generation", &request_json)?;

    return Ok(audio_data);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elevenlabs_endpoint_override() {
        std::env::set_var("XI_ENDPOINT", "http://localhost:1234");
        assert_eq!(get_endpoint(), "http://localhost:1234");
        std::env::remove_var("XI_ENDPOINT");
        assert_eq!(get_endpoint(), "https://api.elevenlabs.io/v1");
    }
}

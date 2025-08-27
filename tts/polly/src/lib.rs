mod bindings;

use bindings::exports::golem::tts::api::Guest;
use bindings::wasi::http::types as http;
use bindings::wasi::http::outgoing_handler;
use bindings::wasi::io::streams;

use serde_json::json;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

mod sigv4;

struct Component;

impl Guest for Component {
    fn health() -> String { "ok".to_string() }

    fn synth_b64(voice: String, text: String) -> Result<String, String> {
        // 1) AWS env
        let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());
        let akid   = std::env::var("AWS_ACCESS_KEY_ID").map_err(|_| "AWS_ACCESS_KEY_ID not set".to_string())?;
        let secret = std::env::var("AWS_SECRET_ACCESS_KEY").map_err(|_| "AWS_SECRET_ACCESS_KEY not set".to_string())?;
        let session = std::env::var("AWS_SESSION_TOKEN").ok();
        let amz_date = std::env::var("AWS_AMZ_DATE").ok(); // optional host override

        // 2) Endpoint + body
        let host = format!("polly.{region}.amazonaws.com");
        let uri  = "/v1/speech";
        let body = json!({
            "Text": text,
            "VoiceId": voice,
            "OutputFormat": "mp3"
        }).to_string();

        // 3) SigV4 headers
        let sig = sigv4::sign_post_json(&host, uri, &region, "polly", &akid, &secret, session.as_deref(), amz_date.as_deref(), &body);

        let mut headers = http::Fields::new();
        headers.append(&http::HeaderName::from_bytes(b"content-type").unwrap(),
                       &http::HeaderValue::from_bytes(b"application/json").unwrap()).unwrap();
        headers.append(&http::HeaderName::from_bytes(b"host").unwrap(),
                       &http::HeaderValue::from_bytes(host.as_bytes()).unwrap()).unwrap();
        headers.append(&http::HeaderName::from_bytes(b"x-amz-content-sha256").unwrap(),
                       &http::HeaderValue::from_bytes(sig.content_sha256.as_bytes()).unwrap()).unwrap();
        headers.append(&http::HeaderName::from_bytes(b"x-amz-date").unwrap(),
                       &http::HeaderValue::from_bytes(sig.amz_date.as_bytes()).unwrap()).unwrap();
        headers.append(&http::HeaderName::from_bytes(b"authorization").unwrap(),
                       &http::HeaderValue::from_bytes(sig.authorization.as_bytes()).unwrap()).unwrap();
        if let Some(tok) = session.as_deref() {
            headers.append(&http::HeaderName::from_bytes(b"x-amz-security-token").unwrap(),
                           &http::HeaderValue::from_bytes(tok.as_bytes()).unwrap()).unwrap();
        }

        // 4) Request
        let req = http::OutgoingRequest::new(
            &http::Scheme::Https,
            Some(&host),
            &uri.to_string(),
            &http::Method::Post,
            Some(&headers)
        );

        let out = req.body().expect("body");
        out.write().expect("stream")
            .blocking_write_and_flush(body.as_bytes()).expect("write");
        out.finish().expect("finish");

        let resp = match outgoing_handler::handle(req, None) {
            Ok(r) => r,
            Err(e) => return Err(format!("dispatch: {:?}", e)),
        };

        // 5) Read body (Polly returns raw MP3 bytes)
        let status = resp.status();
        let in_body = resp.consume().expect("consume");
        let stream = in_body.stream().expect("stream");
        let mut mp3 = Vec::new();
        loop {
            match stream.read(32 * 1024) {
                Ok(Some(chunk)) => mp3.extend_from_slice(&chunk),
                Ok(None) => break,
                Err(e) => return Err(format!("read: {:?}", e)),
            }
        }

        if u16::from(status) / 100 != 2 {
            return Err(format!("polly http {}: {}", u16::from(status), String::from_utf8_lossy(&mp3)));
        }

        Ok(B64.encode(mp3))
    }
}

bindings::export!(Component);

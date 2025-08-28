mod bindings;

use bindings::exports::golem::tts::api::Guest;
// WASI HTTP (Preview 2)
use bindings::wasi::http::types as http;
use bindings::wasi::http::outgoing_handler;
use bindings::wasi::io::streams;

use serde::Deserialize;
use serde_json::json;

struct Component;

#[derive(Deserialize)]
struct GResp {
    #[serde(rename="audioContent")]
    audio_content: String,
}

impl Guest for Component {
    fn health() -> String { "ok".to_string() }

    fn synth_b64(voice: String, text: String) -> Result<String, String> {
        // 1) Bearer token (ADC)
        let token = std::env::var("GOOGLE_OAUTH_ACCESS_TOKEN")
            .map_err(|_| "GOOGLE_OAUTH_ACCESS_TOKEN not set".to_string())?;

        // 2) Body per REST v1 text:synthesize
        // voice like "en-US-Neural2-C" => languageCode "en-US"
        let lang = voice.splitn(3, '-').take(2).collect::<Vec<_>>().join("-");
        let body = json!({
            "input":       { "text": text },
            "voice":       { "languageCode": lang, "name": voice },
            "audioConfig": { "audioEncoding": "MP3" }
        }).to_string();

        // 3) Build request
        let mut headers = http::Fields::new();
        headers.append(&http::HeaderName::from_bytes(b"content-type").unwrap(),
                       &http::HeaderValue::from_bytes(b"application/json").unwrap()).unwrap();
        let auth = format!("Bearer {}", token);
        headers.append(&http::HeaderName::from_bytes(b"authorization").unwrap(),
                       &http::HeaderValue::from_bytes(auth.as_bytes()).unwrap()).unwrap();

        let req = http::OutgoingRequest::new(
            &http::Scheme::Https,
            Some(&"texttospeech.googleapis.com".to_string()),
            &"/v1/text:synthesize".to_string(),
            &http::Method::Post,
            Some(&headers)
        );

        // 4) Body
        let out = req.body().expect("body");
        out.write().expect("stream")
            .blocking_write_and_flush(body.as_bytes()).expect("write");
        out.finish().expect("finish");

        // 5) Send and read response
        let resp = match outgoing_handler::handle(req, None) {
            Ok(r) => r,
            Err(e) => return Err(format!("dispatch: {:?}", e)),
        };
        let status = resp.status();
        let in_body = resp.consume().expect("consume");
        let stream = in_body.stream().expect("stream");
        let mut buf = Vec::new();
        loop {
            match stream.read(32 * 1024) {
                Ok(Some(chunk)) => buf.extend_from_slice(&chunk),
                Ok(None) => break,
                Err(e) => return Err(format!("read: {:?}", e)),
            }
        }

        if u16::from(status) / 100 != 2 {
            return Err(format!("google http {}: {}", u16::from(status), String::from_utf8_lossy(&buf)));
        }

        // 6) Parse JSON and return audioContent (base64)
        let parsed: GResp = serde_json::from_slice(&buf)
            .map_err(|e| format!("json: {e}; body: {}", String::from_utf8_lossy(&buf)))?;
        Ok(parsed.audio_content)
    }
}

bindings::export!(Component);

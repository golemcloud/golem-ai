#![allow(static_mut_refs)]
mod bindings;

struct Component;
bindings::export!(Component with_types_in bindings);

/// ---------- tiny helpers ----------
mod http {
    use reqwest::Client;

    pub fn api_key() -> Result<String, String> {
        std::env::var("ELEVENLABS_API_KEY").map_err(|_| "ELEVENLABS_API_KEY not set".to_string())
    }

    pub fn client() -> Result<Client, String> {
        Client::builder()
            .build()
            .map_err(|e| format!("reqwest client build: {e}"))
    }
}

/// ---------- bootstrap interface ----------
mod bootstrap_impl {
    use super::bindings::exports::golem::tts::bootstrap;
    impl bootstrap::Guest for super::Component {
        fn ping() -> String {
            "ok".to_string()
        }
    }
}

/// ---------- voices interface ----------
mod voices_impl {
    use super::bindings::exports::golem::tts::voices;
    use super::http;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct VoiceItem {
        voice_id: String,
        name: String,
    }
    #[derive(Deserialize)]
    struct VoicesResponse {
        voices: Vec<VoiceItem>,
    }

    impl voices::Guest for super::Component {
        fn list_voices() -> Result<Vec<voices::Voice>, String> {
            let key = http::api_key()?;
            let client = http::client()?;

            let resp = client
                .get("https://api.elevenlabs.io/v1/voices")
                .header("xi-api-key", &key)
                .header("accept", "application/json")
                .send()
                .map_err(|e| format!("GET /v1/voices: {e}"))?;

            let status = resp.status();
            let body = resp.bytes().map_err(|e| format!("read body: {e}"))?;
            if !status.is_success() {
                return Err(format!(
                    "ElevenLabs voices failed: HTTP {status} - {}",
                    String::from_utf8_lossy(&body)
                ));
            }

            let parsed: VoicesResponse = serde_json::from_slice(&body).map_err(|e| {
                format!(
                    "parse voices JSON: {e}; body: {}",
                    String::from_utf8_lossy(&body)
                )
            })?;

            Ok(parsed
                .voices
                .into_iter()
                .map(|v| voices::Voice {
                    id: v.voice_id,
                    name: v.name,
                })
                .collect())
        }
    }
}

/// ---------- synthesis interface ----------
mod synth_impl {
    use super::bindings::exports::golem::tts::synthesis;
    use super::http;
    use serde::Serialize;

    #[derive(Serialize)]
    struct SynthReq<'a> {
        text: &'a str,
        // you can add model_id / voice_settings here if you want
    }

    /// Internal helper both the interface and world-level export will call.
    pub fn synthesize_mp3(voice_id: &str, text: &str) -> Result<Vec<u8>, String> {
        let key = http::api_key()?;
        let client = http::client()?;
        let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{voice_id}");
        let body = SynthReq { text };

        let resp = client
            .post(url)
            .header("xi-api-key", &key)
            .header("accept", "audio/mpeg")
            .header("content-type", "application/json")
            .body(serde_json::to_vec(&body).map_err(|e| format!("encode JSON: {e}"))?)
            .send()
            .map_err(|e| format!("POST text-to-speech: {e}"))?;

        let status = resp.status();
        let bytes = resp.bytes().map_err(|e| format!("read audio bytes: {e}"))?;
        if !status.is_success() {
            return Err(format!(
                "synthesize failed: HTTP {status} - {}",
                String::from_utf8_lossy(&bytes)
            ));
        }
        Ok(bytes.to_vec())
    }

    impl synthesis::Guest for super::Component {
        fn synthesize(voice_id: String, text: String) -> Result<Vec<u8>, String> {
            synthesize_mp3(&voice_id, &text)
        }
    }
}

/// ---------- world-level exports (for easy CLI testing) ----------
mod world_impl {
    use super::bindings::Guest as WorldGuest;
    use super::synth_impl;
    use base64::{engine::general_purpose, Engine as _};

    impl WorldGuest for super::Component {
        fn health() -> String {
            "ok".to_string()
        }

        fn synth_b64(voice_id: String, text: String) -> Result<String, String> {
            let bytes = synth_impl::synthesize_mp3(&voice_id, &text)?;
            Ok(general_purpose::STANDARD.encode(bytes))
        }
    }
}

use std::process::Command;
use base64::{engine::general_purpose, Engine as _};

fn wasm_path() -> String {
    // Use workspace target if CARGO_TARGET_DIR is not set
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .unwrap_or_else(|_| format!("{}/../../target", env!("CARGO_MANIFEST_DIR")));
    format!("{}/wasm32-wasip1/release/tts_elevenlabs.wasm", target_dir)
}

// Parse Wasmtime's printed WIT values: ok("…") / err("…") / plain "…"
fn strip_wave(s: &str) -> Result<String, String> {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("ok(\"").and_then(|x| x.strip_suffix("\")")) {
        Ok(rest.to_string())
    } else if let Some(rest) = s.strip_prefix('"').and_then(|x| x.strip_suffix('"')) {
        Ok(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("err(\"").and_then(|x| x.strip_suffix("\")")) {
        Err(rest.to_string())
    } else {
        Ok(s.to_string())
    }
}

#[test]
fn health_ok() {
    // No key needed for health()
    let out = Command::new("wasmtime")
        .args([
            "run", "-S", "http",
            "--invoke", "health()",
            &wasm_path(),
        ])
        .output()
        .expect("run wasmtime");
    assert!(out.status.success(), "wasmtime exit {:?}", out.status);
    let s = String::from_utf8_lossy(&out.stdout);
    assert_eq!(s.trim(), "\"ok\"");
}

#[test]
fn synth_nonempty_mp3() {
    let key = match std::env::var("ELEVENLABS_API_KEY") {
        Ok(k) => k,
        Err(_) => {
            eprintln!("ELEVENLABS_API_KEY not set -> skipping synth test");
            return;
        }
    };

    let voice = std::env::var("VOICE_ID").unwrap_or_else(|_| "21m00Tcm4TlvDq8ikWAM".to_string());
    let text  = "Hello from ElevenLabs on Golem!";

    let out = Command::new("wasmtime")
        .args([
            "run", "-S", "http",
            "--env", &format!("ELEVENLABS_API_KEY={key}"),
            "--invoke", &format!("synth-b64(\"{voice}\",\"{text}\")"),
            &wasm_path(),
        ])
        .output()
        .expect("run wasmtime");

    assert!(out.status.success(), "wasmtime exit {:?}", out.status);

    let printed = String::from_utf8_lossy(&out.stdout);
    let b64 = strip_wave(&printed).expect("ok(...) result");

    // Unescape the simple backslash escapes Wasmtime prints
    let cleaned = b64.replace("\\n", "")
                     .replace("\\/", "/")
                     .replace("\\\"", "\"");

    let audio = general_purpose::STANDARD.decode(cleaned)
        .expect("valid base64");

    // Sanity: produced MP3 should be at least ~10KB
    assert!(audio.len() > 10_000, "MP3 too small ({} bytes)", audio.len());
}

use std::process::Command;
#[test]
fn synth_ok() {
    // Expect a bearer token in GOOGLE_OAUTH_ACCESS_TOKEN (mint via: gcloud auth application-default print-access-token)
    let token = match std::env::var("GOOGLE_OAUTH_ACCESS_TOKEN") {
        Ok(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => { eprintln!("skip: GOOGLE_OAUTH_ACCESS_TOKEN not set"); return; }
    };
    let voice = std::env::var("VOICE_GCP").unwrap_or_else(|_| "en-US-Neural2-C".into());
    let text  = "Hello from Google TTS via Wasm test.";

    assert!(Command::new("cargo").args(["component","build","--release","-p","tts-google"]).status().unwrap().success());

    let invoke = format!("synth-b64(\"{}\",\"{}\")", voice, text);
    let out = Command::new("wasmtime")
        .env("GOOGLE_OAUTH_ACCESS_TOKEN", &token)
        .args(["run","-S","http","--invoke",&invoke,"target/wasm32-wasip1/release/tts_google.wasm"])
        .output().expect("spawn wasmtime");
    assert!(out.status.success(), "wasmtime exit {:?}", out.status);
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("ok(\"") && s.contains("\")"), "unexpected stdout: {}", s);
}

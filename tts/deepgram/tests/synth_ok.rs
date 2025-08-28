use std::process::Command;
#[test]
fn synth_ok() {
    let key = match std::env::var("DEEPGRAM_API_KEY") {
        Ok(k) if !k.trim().is_empty() => k.trim().to_string(),
        _ => { eprintln!("skip: DEEPGRAM_API_KEY not set"); return; }
    };
    let model = std::env::var("VOICE_DG").unwrap_or_else(|_| "aura-2-thalia-en".into());
    let text  = "Hello from Deepgram via Wasm test.";

    assert!(Command::new("cargo").args(["component","build","--release","-p","tts-deepgram"]).status().unwrap().success());

    let invoke = format!("synth-b64(\"{}\",\"{}\")", model, text);
    let out = Command::new("wasmtime")
        .env("DEEPGRAM_API_KEY", &key)
        .args(["run","-S","http","--invoke",&invoke,"target/wasm32-wasip1/release/tts_deepgram.wasm"])
        .output().expect("spawn wasmtime");
    assert!(out.status.success(), "wasmtime exit {:?}", out.status);
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("ok(\"") && s.contains("\")"), "unexpected stdout: {}", s);
}

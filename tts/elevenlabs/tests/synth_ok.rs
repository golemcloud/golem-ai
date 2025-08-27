use std::process::Command;
#[test]
fn synth_ok() {
    let key = match std::env::var("ELEVENLABS_API_KEY") {
        Ok(k) if !k.trim().is_empty() => k.trim().to_string(),
        _ => { eprintln!("skip: ELEVENLABS_API_KEY not set"); return; }
    };
    let voice = std::env::var("VOICE_EL").unwrap_or_else(|_| "21m00Tcm4TlvDq8ikWAM".into());
    let text  = "Hello from ElevenLabs via Wasm test.";

    // Ensure wasm exists
    assert!(Command::new("cargo").args(["component","build","--release","-p","tts-elevenlabs"]).status().unwrap().success());

    // WAVE invoke: --invoke 'synth-b64("voice","text")'
    let invoke = format!("synth-b64(\"{}\",\"{}\")", voice, text);
    let out = Command::new("wasmtime")
        .env("ELEVENLABS_API_KEY", &key)
        .args(["run","-S","http","--invoke",&invoke,"target/wasm32-wasip1/release/tts_elevenlabs.wasm"])
        .output().expect("spawn wasmtime");
    assert!(out.status.success(), "wasmtime exit {:?}", out.status);
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("ok(\"") && s.contains("\")"), "unexpected stdout: {}", s);
}

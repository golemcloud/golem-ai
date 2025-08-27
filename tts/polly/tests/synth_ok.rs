use std::process::Command;
#[test]
fn synth_ok() {
    let akid = match std::env::var("AWS_ACCESS_KEY_ID") { Ok(v) if !v.trim().is_empty() => v, _ => { eprintln!("skip: AWS_ACCESS_KEY_ID not set"); return; } };
    let asec = match std::env::var("AWS_SECRET_ACCESS_KEY") { Ok(v) if !v.trim().is_empty() => v, _ => { eprintln!("skip: AWS_SECRET_ACCESS_KEY not set"); return; } };
    let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".into());
    let voice  = std::env::var("VOICE_AWS").unwrap_or_else(|_| "Joanna".into());
    let text   = "Hello from Polly via Wasm test.";

    assert!(Command::new("cargo").args(["component","build","--release","-p","tts-polly"]).status().unwrap().success());

    let invoke = format!("synth-b64(\"{}\",\"{}\")", voice, text);
    let mut cmd = Command::new("wasmtime");
    cmd.env("AWS_REGION", &region)
       .env("AWS_ACCESS_KEY_ID", &akid)
       .env("AWS_SECRET_ACCESS_KEY", &asec);
    if let Ok(tok) = std::env::var("AWS_SESSION_TOKEN") { if !tok.is_empty() { cmd.env("AWS_SESSION_TOKEN", tok); } }
    let out = cmd
        .args(["run","-S","http","--invoke",&invoke,"target/wasm32-wasip1/release/tts_polly.wasm"])
        .output().expect("spawn wasmtime");
    assert!(out.status.success(), "wasmtime exit {:?}", out.status);
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("ok(\"") && s.contains("\")"), "unexpected stdout: {}", s);
}

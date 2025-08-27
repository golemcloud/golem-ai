use std::process::Command;
#[test]
fn health_ok() {
    let out = Command::new("wasmtime")
        .args(["run","-S","http","--invoke","health()","target/wasm32-wasip1/release/tts_deepgram.wasm"])
        .output().expect("spawn wasmtime");
    assert!(out.status.success(), "wasmtime exit {:?}", out.status);
    let txt = String::from_utf8_lossy(&out.stdout);
    assert_eq!(txt.trim().trim_matches('"'), "ok", "unexpected stdout: {:?}", txt);
}

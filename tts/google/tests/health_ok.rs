use std::process::Command;
#[test] fn health_ok() {
    let out = Command::new("wasmtime").args([
        "run","-S","http","--invoke","health()",
        "target/wasm32-wasip1/release/tts_google.wasm",
    ]).output().expect("spawn wasmtime");
    assert!(out.status.success(), "wasmtime exit: {:?}", out.status);
    let norm = String::from_utf8_lossy(&out.stdout).trim().trim_matches('"').to_string();
    assert_eq!(norm, "ok", "unexpected stdout: {:?}", String::from_utf8_lossy(&out.stdout));
}

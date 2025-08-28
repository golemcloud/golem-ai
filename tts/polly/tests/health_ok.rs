use std::process::Command;
use std::path::PathBuf;

fn ws_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2).unwrap().to_path_buf()
}
fn wasm() -> String {
    ws_root().join("target/wasm32-wasip1/release/tts_polly.wasm").to_string_lossy().to_string()
}

#[test]
fn health_ok() {
    let ok = Command::new("cargo")
        .args(["component","build","--release","-p","tts-polly"])
        .status().expect("spawn cargo component build (polly)");
    assert!(ok.success(), "cargo component build failed (polly)");

    let out = Command::new("wasmtime")
        .args(["run","-S","http","--invoke","health()", &wasm()])
        .output().expect("spawn wasmtime");

    assert!(out.status.success(), "wasmtime exit: {:?}", out.status);
    let txt = String::from_utf8_lossy(&out.stdout);
    let norm = txt.trim().trim_matches('"');
    assert_eq!(norm, "ok", "unexpected stdout: {:?}", txt);
}

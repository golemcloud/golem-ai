use std::process::Command;
use std::path::PathBuf;

fn run(prog: &str, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(prog).args(args).output().expect("spawn");
    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    (code, stdout, stderr)
}

// Build the component so the .wasm exists for wasmtime.
fn ensure_component_built() {
    let status = Command::new("cargo")
        .args(["component", "build", "--release", "-p", "tts-deepgram"])
        .status()
        .expect("spawn cargo component build");
    assert!(status.success(), "cargo component build failed");
}

// Workspace target: <repo>/target/wasm32-wasip1/release/tts_deepgram.wasm
fn wasm_path() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let mut p = PathBuf::from(manifest_dir);
    p.pop(); // .../tts/deepgram -> .../tts
    p.pop(); // .../tts -> repo root
    p.push("target/wasm32-wasip1/release/tts_deepgram.wasm");
    p.to_string_lossy().into_owned()
}

#[test]
fn health_ok() {
    ensure_component_built();
    let wasm = wasm_path();
    let (code, out, err) =
        run("wasmtime", &["run", "-S", "http", "--invoke", "health()", &wasm]);
    assert_eq!(code, 0, "wasmtime exit {}.\nstderr:\n{}", code, err);
    assert!(out.trim()=="ok" || out.trim().starts_with("ok("), "unexpected stdout:\n{}", out);
}

#[test]
fn synth_nonempty_mp3() {
    ensure_component_built();

    let key = match std::env::var("DEEPGRAM_API_KEY") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            eprintln!("DEEPGRAM_API_KEY not set -> skipping");
            return;
        }
    };

    let wasm = wasm_path();
    let voice = "aura-2-thalia-en";
    let text  = "Hello from Deepgram on Golem!";
    let env_arg = format!("DEEPGRAM_API_KEY={}", key);
    let invoke  = format!("synth-b64(\"{}\",\"{}\")", voice, text);

    let (code, out, err) = run(
        "wasmtime",
        &["run", "-S", "http", "--env", &env_arg, "--invoke", &invoke, &wasm],
    );
    assert_eq!(code, 0, "wasmtime exit {}.\nstderr:\n{}", code, err);

    let s = out.trim();
    assert!(s=="ok" || out.trim().starts_with("ok(") && s.ends_with(')'), "unexpected stdout:\n{}\n\nstderr:\n{}", out, err);
    let inner = &s[3..s.len() - 1]; // strip ok(…)
    let b64: String = serde_json::from_str(inner).expect("json string inner result");

    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    let bytes = STANDARD.decode(b64.as_bytes()).expect("base64 decode");
    assert!(bytes.len() > 100, "synthesized payload too small ({} bytes)", bytes.len());
}

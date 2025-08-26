use std::path::PathBuf;
use std::process::Command;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

fn run(prog: &str, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(prog).args(args).output().expect("spawn");
    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    (code, stdout, stderr)
}

// Build the component so the .wasm exists before invoking it with wasmtime.
fn ensure_component_built() {
    let status = Command::new("cargo")
        .args(["component", "build", "--release", "-p", "tts-deepgram"])
        .status()
        .expect("spawn cargo component build");
    assert!(status.success(), "cargo component build failed");
}

// Workspace target path: <repo>/target/wasm32-wasip1/release/tts_deepgram.wasm
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
    let (code, out, err) = run(
        "wasmtime",
        &["run", "-S", "http", "--invoke", "health()", &wasm]
    );
    assert_eq!(code, 0, "wasmtime exit code {}, stderr:\n{}", code, err);
    let t = out.trim();
    // Accept ok, "ok", or ok(...)
    assert!(t == "ok" || t == "\"ok\"" || t.starts_with("ok("),
            "unexpected stdout:\n{}", out);
}

#[test]
fn synth_nonempty_mp3() {
    ensure_component_built();

    // Key-gated: skip if no key
    let key = match std::env::var("DEEPGRAM_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            eprintln!("DEEPGRAM_API_KEY not set -> skipping synth test");
            return;
        }
    };

    let wasm = wasm_path();
    let voice = "aura-2-thalia-en";
    let text = "Hello from Deepgram on Golem!";
    let fcall = format!("synth-b64(\"{}\",\"{}\")", voice, text);

    let (code, out, err) = run(
        "wasmtime",
        &[
            "run",
            "-S","http",
            "--env",&format!("DEEPGRAM_API_KEY={}", key),
            "--invoke",&fcall,
            &wasm
        ]
    );
    assert_eq!(code, 0, "wasmtime exit code {}, stderr:\n{}", code, err);

    let t = out.trim();
    // Handle ok("..."), ok(...), or just a quoted base64
    let b64 = if t.starts_with("ok(") {
        let inner = t.trim_start_matches("ok(").trim_end_matches(')');
        inner.trim_matches('"').to_string()
    } else {
        t.trim_matches('"').to_string()
    };

    let bytes = B64.decode(b64.as_bytes()).expect("base64 decode");
    assert!(bytes.len() > 256, "synth should return a non-trivial MP3 payload");
}

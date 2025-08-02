pub mod component;
pub mod config;
pub mod durability;
pub mod errors;
pub mod http;
pub mod mapping;
pub mod stream;

wit_bindgen::generate!({
    path: "../wit",
    world: "stt-library",
    generate_all,
    generate_unused_types: true,
    pub_export_macro: true,
});

pub use __export_stt_library_impl as export_stt;
pub use component::Component;

use base64::Engine;
use log::{LevelFilter, Metadata, Record};

/// Simple logger suitable for WASM components that honors STT_PROVIDER_LOG_LEVEL.
struct SimpleLogger;

static LOGGER: SimpleLogger = SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        log::max_level() >= metadata.level().to_level_filter()
    }
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // Emit to the configured logger backend; avoid direct println in WASI.
            log::logger().log(
                &Record::builder()
                    .args(*record.args())
                    .level(record.level())
                    .target(record.target())
                    .module_path(record.module_path_static())
                    .file(record.file_static())
                    .line(record.line())
                    .build(),
            );
        }
    }
    fn flush(&self) {}
}

/// Initialize logging once based on provided level string.
pub fn init_logging_from_env(level: Option<String>) {
    let level = level.as_deref().unwrap_or("info").to_ascii_lowercase();

    let filter = match level.as_str() {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        "warn" | "warning" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        "off" => LevelFilter::Off,
        _ => LevelFilter::Info,
    };

    // It's okay if set_logger/set_max_level are called multiple times in practice,
    // but log only allows setting logger once per process. We swallow the error.
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(filter);
}

/// Utility to parse a level string to LevelFilter.
pub fn parse_level_filter(s: &str) -> LevelFilter {
    match s.to_ascii_lowercase().as_str() {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        "warn" | "warning" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        "off" => LevelFilter::Off,
        _ => LevelFilter::Info,
    }
}

/// Produce a stable request checksum (base64 of sha256) for idempotency.
/// Avoid pulling in heavy deps; use a lightweight wasm-safe implementation if available.
/// Here we implement a tiny wrapper over the wstd sha256 if exposed; otherwise fallback to a simple XOR
/// (good enough only to detect accidental duplicates in durability; not for security).
pub fn request_checksum(data: &[u8]) -> String {
    // Prefer a real hash when available. wstd does not expose sha256 today,
    // keep a simple rolling hash for idempotency.
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    base64::engine::general_purpose::STANDARD.encode(h.to_le_bytes())
}

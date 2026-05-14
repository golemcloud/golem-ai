//! Compatibility shim that re-exports the WASI types used by the LLM
//! infrastructure.
//!
//! All in-tree code MUST go through this module instead of importing
//! `golem_rust::bindings::wasi::*` or `golem_rust::golem_wasm::*` directly,
//! so that the umbrella crate can be compiled without any `golem:*` WIT
//! imports leaking into the produced WASM component when the `golem`
//! feature is disabled.
//!
//! We intentionally use the `wasip2` crate (which is also what
//! `golem-wasi-http` and `golem-rust` use internally) so all WASI handles
//! are bit-compatible across crate boundaries.

pub use wasip2::io::poll::Pollable;
pub use wasip2::io::streams::{InputStream, StreamError};

pub fn subscribe_zero() -> Pollable {
    wasip2::clocks::monotonic_clock::subscribe_duration(0)
}

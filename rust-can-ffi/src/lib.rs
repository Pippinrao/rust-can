//! Minimal C FFI version surface for rust-can.
//!
//! The full ABI is defined after the Rust-side IO and adapter APIs stabilize.

/// Returns the crate version string.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

//! Backend implementations for various CAN hardware interfaces.
//!
//! Each backend is behind a feature flag. The `virtual` backend is
//! always available as it requires no hardware.

#[cfg(feature = "virtual")]
/// Virtual adapter implementation.
pub mod r#virtual;

//! # rust-can-adapters
//!
//! CAN bus adapter implementations with an open, trait-based interface.
//!
//! ## Architecture
//!
//! The crate defines the [`CanAdapter`] trait — a minimal interface that
//! any third-party CAN hardware backend must implement to integrate with
//! the rust-can ecosystem. This design ensures:
//!
//! - **No vendor lock-in**: Any hardware can be supported
//! - **Minimum API surface**: Only 7 required methods
//! - **Plugin architecture**: Compile-time (feature flags) or runtime (dlopen)
//! - **Safety by default**: Send + Sync guarantees for multi-threaded use
//!
//! ## Implementing a Custom Adapter
//!
//! ```rust,no_run
//! use rust_can_adapters::{CanAdapter, AdapterConfig, AdapterInfo};
//! use rust_can_core::error::CanError;
//! use rust_can_core::frame::CanFrame;
//! use rust_can_core::error::Result;
//!
//! struct MyAdapter;
//!
//! impl CanAdapter for MyAdapter {
//!     fn open(config: &AdapterConfig) -> Result<Self> { Ok(Self) }
//!     fn read_frame(&self, timeout: Option<std::time::Duration>) -> Result<CanFrame> {
//!         Err(CanError::not_supported("read_frame", "example adapter has no device"))
//!     }
//!     fn write_frame(&self, frame: &CanFrame, timeout: Option<std::time::Duration>) -> Result<()> {
//!         Err(CanError::not_supported("write_frame", "example adapter has no device"))
//!     }
//!     fn info(&self) -> AdapterInfo { AdapterInfo::new("example", "example adapter") }
//!     fn close(&self) -> Result<()> { Ok(()) }
//! }
//! ```

/// Adapter trait definitions.
pub mod adapter;
/// Built-in adapter implementations.
pub mod backends;
/// Adapter configuration values.
pub mod config;
/// Adapter discovery registry.
pub mod registry;

pub use adapter::CanAdapter;
pub use config::AdapterConfig;
pub use registry::{AdapterInfo, ADAPTER_REGISTRY};

//! # rust-can-core
//!
//! Core abstractions for CAN bus communication in Rust.
//!
//! This crate provides the foundational types, traits, and utilities
//! for working with CAN (Controller Area Network) buses, supporting
//! CAN 2.0, CAN FD, and CAN XL protocols.
//!
//! ## Architecture
//!
//! The crate is organized into several modules:
//!
//! - [`message`] - CAN message data structures (CanMessage, CanMessageXL)
//! - [`frame`] - Low-level CAN frame representation
//! - [`bus`] - CAN bus abstraction traits (CanBus, PeriodicSendBus)
//! - [`protocol`] - Protocol and bus state enums
//! - [`error`] - Error types and Result alias
//! - [`filter`] - Message filter definitions
//! - [`listener`] - Listener trait and built-in listeners
//! - [`bit_timing`] - Bit timing calculations for CAN 2.0 and FD
//! - [`cyclic`] - Cyclic message sending tasks
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rust_can_core::message::CanMessage;
//! use rust_can_core::bus::CanBus;
//! use std::time::Duration;
//!
//! // Create a message
//! let msg = CanMessage::new(0x123, &[0x01, 0x02, 0x03], false).unwrap();
//!
//! // Send and receive (using any bus implementation)
//! // bus.send(&msg, Some(Duration::from_secs(1))).await?;
//! // let received = bus.recv(Some(Duration::from_secs(1))).await?;
//! ```

/// CAN bit timing calculation and validation.
pub mod bit_timing;
/// CAN bus traits and periodic task interfaces.
pub mod bus;
/// Periodic transmit task implementation.
pub mod cyclic;
/// Error types used across rust-can crates.
pub mod error;
/// Acceptance filter definitions.
pub mod filter;
/// Low-level frame representation.
pub mod frame;
/// Listener traits and buffered/printing listeners.
pub mod listener;
/// High-level CAN message types.
pub mod message;
/// Protocol and bus state enums.
pub mod protocol;

// Re-exports for convenient access
pub use bus::CanBus;
pub use error::{CanError, Result};
pub use filter::{CanFilter, CanFilters};
pub use frame::CanFrame;
pub use listener::Listener;
pub use message::CanMessage;
pub use protocol::{BusState, CanProtocol};

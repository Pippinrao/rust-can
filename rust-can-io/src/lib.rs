//! # rust-can-io
//!
//! CAN bus log format readers and writers.
//!
//! Current target: ASC and BLF streaming readers/writers.
//!
//! ASC supports CAN, CAN FD, and LIN records from the real corpus under
//! `data/`. Additional formats can be added through the log event model.

pub mod event;
pub mod formats;
/// Replay timing helpers.
pub mod player;
/// Reader-side format detection and dispatch helpers.
pub mod reader;
/// Writer-side traits and dispatch helpers.
pub mod writer;

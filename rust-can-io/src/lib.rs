//! # rust-can-io
//!
//! CAN bus log format readers and writers.
//!
//! Current target: ASC and BLF streaming readers/writers.
//!
//! ASC supports CAN, CAN FD, and LIN records from the real corpus under
//! `data/`. Additional formats can be added through the log event model.

#[cfg(feature = "profile")]
pub mod prof;

// Always compiled (but its body is feature-gated) so the `prof_scope!`
// macro is at the crate root whether the `profile` feature is on or
// off. The body becomes a no-op when profiling is disabled.
#[macro_use]
mod prof_macro;

pub mod event;
pub mod formats;
/// Replay timing helpers.
pub mod player;
/// Reader-side format detection and dispatch helpers.
pub mod reader;
/// Writer-side traits and dispatch helpers.
pub mod writer;

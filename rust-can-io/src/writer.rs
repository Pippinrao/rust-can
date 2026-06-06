//! Writer-side traits for log sinks.

use std::io;

use crate::event::LogEvent;

/// Common interface for streaming log writers.
pub trait EventWriter {
    /// Writes one log event to the underlying sink.
    fn write_event(&mut self, event: &LogEvent) -> io::Result<()>;

    /// Flushes pending data.
    fn flush(&mut self) -> io::Result<()>;
}

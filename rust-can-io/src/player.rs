//! Replay timing helpers for log playback.

use std::time::Duration;

use crate::event::LogEvent;

/// One event paired with the delay before it should be replayed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayItem {
    /// Delay before sending or emitting the event.
    pub delay: Duration,
    /// Event to replay.
    pub event: LogEvent,
}

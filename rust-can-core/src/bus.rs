/// CAN bus abstraction trait.
use std::time::Duration;

#[cfg(unix)]
use std::os::fd::RawFd;

use async_trait::async_trait;

use crate::error::Result;
use crate::filter::CanFilters;
use crate::message::CanMessage;
use crate::protocol::{BusState, CanProtocol};

/// A handle to a periodic send task.
pub trait CyclicTask: Send + Sync {
    /// Stops the periodic task.
    fn stop(&self) -> Result<()>;
    /// Modifies each message owned by the task.
    fn modify(&self, modifier: &dyn Fn(&mut CanMessage)) -> Result<()>;
    /// Returns true while the periodic task is active.
    fn is_running(&self) -> bool;
}

/// The CAN Bus trait — core abstraction for CAN communication.
///
/// Uses `#[async_trait]` for dyn compatibility.
#[async_trait]
pub trait CanBus: Send + Sync {
    /// Receive a message from the bus.
    async fn recv(&self, timeout: Option<Duration>) -> Result<Option<CanMessage>>;

    /// Send a message on the bus.
    async fn send(&self, msg: &CanMessage, timeout: Option<Duration>) -> Result<()>;

    /// Set message filters.
    fn set_filters(&self, _filters: &CanFilters) -> Result<()> {
        Ok(())
    }

    /// Get the CAN protocol supported by this bus.
    fn protocol(&self) -> CanProtocol;

    /// Get the current bus state.
    fn state(&self) -> BusState {
        BusState::Active
    }

    /// Flush the transmit buffer.
    fn flush_tx_buffer(&self) -> Result<()> {
        Err(crate::error::CanError::not_supported(
            "flush_tx_buffer",
            "not implemented by this bus",
        ))
    }

    /// Shut down the bus.
    fn shutdown(&self) -> Result<()>;

    /// Get a human-readable description.
    fn channel_info(&self) -> &str {
        "unknown"
    }

    /// Get the file descriptor for event-loop integration.
    #[cfg(unix)]
    fn fileno(&self) -> Option<RawFd> {
        None
    }

    /// Get the raw socket/handle for Windows event-loop integration.
    #[cfg(windows)]
    fn fileno(&self) -> Option<std::os::windows::io::RawSocket> {
        None
    }

    /// Start sending messages at a fixed period.
    async fn send_periodic(
        &self,
        msgs: &[CanMessage],
        period: Duration,
        duration: Option<Duration>,
    ) -> Result<Box<dyn CyclicTask>>;
}

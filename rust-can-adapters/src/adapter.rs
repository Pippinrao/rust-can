/// The CAN adapter trait — the universal interface for CAN hardware backends.
///
/// Any third-party CAN hardware can integrate with rust-can by implementing
/// this trait. The design follows these principles:
///
/// - **Minimum required methods**: Only 7 methods, 5 of which have defaults
/// - **Send + Sync**: All adapters must be thread-safe
/// - **Composable**: Adapters can be composed with filters, loggers, etc.
use std::time::Duration;

#[cfg(unix)]
use std::os::fd::RawFd;

use rust_can_core::error::Result;
use rust_can_core::filter::CanFilters;
use rust_can_core::frame::CanFrame;

use crate::config::AdapterConfig;
use crate::registry::AdapterInfo;

/// The universal CAN adapter trait.
///
/// This is the minimum interface that any CAN hardware backend
/// must implement. The trait is designed to be:
///
/// - **Simple**: Only 2 truly required methods (`read_frame`, `write_frame`)
/// - **Optional features**: Hardware filtering, fd polling, etc. are optional
/// - **Safe**: Send + Sync guarantees for use in multi-threaded contexts
pub trait CanAdapter: Send + Sync {
    /// Open and initialize the adapter from configuration.
    ///
    /// # Arguments
    /// * `config` - Adapter-specific configuration (serialized to key-value pairs)
    ///
    /// # Returns
    /// * The initialized adapter on success
    /// * `CanError::InitializationError` on failure
    fn open(config: &AdapterConfig) -> Result<Self>
    where
        Self: Sized;

    /// Read a single CAN frame from the hardware.
    ///
    /// # Arguments
    /// * `timeout` - Maximum time to wait. `None` means wait indefinitely.
    ///
    /// # Returns
    /// * The received frame on success
    /// * `CanError::TimeoutError` if no frame received within timeout
    /// * `CanError::OperationError` on hardware failure
    fn read_frame(&self, timeout: Option<Duration>) -> Result<CanFrame>;

    /// Write a single CAN frame to the hardware.
    ///
    /// # Arguments
    /// * `frame` - The frame to transmit
    /// * `timeout` - Maximum time to wait for TX completion. `None` means wait indefinitely.
    fn write_frame(&self, frame: &CanFrame, timeout: Option<Duration>) -> Result<()>;

    /// Apply hardware-level message filters (optional).
    ///
    /// If supported, this programs the hardware acceptance filters
    /// to reduce CPU load by only receiving matching messages.
    /// The default implementation returns `NotSupported`.
    fn apply_hardware_filters(&self, _filters: &CanFilters) -> Result<()> {
        Err(rust_can_core::error::CanError::not_supported(
            "hardware_filters",
            "this adapter uses software filtering",
        ))
    }

    /// Get a file descriptor suitable for epoll/kqueue event loops.
    ///
    /// Returns `None` if the adapter does not support fd-based polling
    /// (in which case polling with `read_frame` is used instead).
    #[cfg(unix)]
    fn fileno(&self) -> Option<RawFd> {
        None
    }

    /// Get a raw socket/handle for Windows IOCP event loops.
    #[cfg(windows)]
    fn fileno(&self) -> Option<std::os::windows::io::RawSocket> {
        None
    }

    /// Get information about this adapter.
    fn info(&self) -> AdapterInfo;

    /// Close the adapter and release all resources.
    ///
    /// Must be safe to call multiple times (idempotent).
    fn close(&self) -> Result<()>;

    /// Flush any pending transmit buffers.
    fn flush_tx(&self) -> Result<()> {
        // Default: no-op
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use rust_can_core::error::CanError;

    struct MockAdapter;

    impl CanAdapter for MockAdapter {
        fn open(_config: &AdapterConfig) -> Result<Self> {
            Ok(Self)
        }

        fn read_frame(&self, _timeout: Option<Duration>) -> Result<CanFrame> {
            Ok(CanFrame::new_data(0x123, Bytes::from_static(&[1, 2, 3]), false))
        }

        fn write_frame(&self, _frame: &CanFrame, _timeout: Option<Duration>) -> Result<()> {
            Ok(())
        }

        fn info(&self) -> AdapterInfo {
            AdapterInfo::new("mock", "Mock adapter")
        }

        fn close(&self) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn adapter_defaults_are_safe_noop_or_not_supported() {
        let adapter = MockAdapter::open(&AdapterConfig::new()).unwrap();
        assert!(matches!(
            adapter.apply_hardware_filters(&CanFilters::new()),
            Err(CanError::NotSupported { .. })
        ));
        assert!(adapter.flush_tx().is_ok());
        assert_eq!(adapter.info().name, "mock");
        assert_eq!(adapter.read_frame(None).unwrap().can_id, 0x123);
        adapter
            .write_frame(&CanFrame::new_data(0x123, Bytes::new(), false), None)
            .unwrap();
        adapter.close().unwrap();
    }
}

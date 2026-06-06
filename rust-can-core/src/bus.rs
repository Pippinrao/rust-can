/// CAN bus abstraction trait.
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::fd::RawFd;

use async_trait::async_trait;
use parking_lot::Mutex;

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

/// Software filter wrapper around any [`CanBus`] implementation.
///
/// Matches python-can `BusABC.recv` behaviour: loops on the inner bus until a
/// frame passes the active [`CanFilters`] or the timeout expires.
pub struct FilteredBus<B: CanBus + ?Sized> {
    inner: Arc<B>,
    filters: Mutex<CanFilters>,
}

impl<B: CanBus + ?Sized> FilteredBus<B> {
    /// Wraps a bus with an empty (match-all) filter set.
    pub fn new(inner: Arc<B>) -> Self {
        Self {
            inner,
            filters: Mutex::new(CanFilters::new()),
        }
    }

    /// Wraps a bus with an initial filter set.
    pub fn with_filters(inner: Arc<B>, filters: CanFilters) -> Self {
        Self {
            inner,
            filters: Mutex::new(filters),
        }
    }

    /// Returns the active filter set.
    pub fn filters(&self) -> CanFilters {
        self.filters.lock().clone()
    }
}

#[async_trait]
impl<B: CanBus + ?Sized> CanBus for FilteredBus<B> {
    async fn recv(&self, timeout: Option<Duration>) -> Result<Option<CanMessage>> {
        let deadline = timeout.map(|duration| Instant::now() + duration);
        loop {
            let remaining = deadline.map(|deadline| {
                deadline
                    .checked_duration_since(Instant::now())
                    .unwrap_or(Duration::ZERO)
            });
            if matches!(remaining, Some(duration) if duration.is_zero()) {
                return Ok(None);
            }

            match self.inner.recv(remaining).await? {
                None => return Ok(None),
                Some(message) if self.filters.lock().matches(&message) => return Ok(Some(message)),
                Some(_) => continue,
            }
        }
    }

    async fn send(&self, msg: &CanMessage, timeout: Option<Duration>) -> Result<()> {
        self.inner.send(msg, timeout).await
    }

    fn set_filters(&self, filters: &CanFilters) -> Result<()> {
        *self.filters.lock() = filters.clone();
        self.inner.set_filters(filters)
    }

    fn protocol(&self) -> CanProtocol {
        self.inner.protocol()
    }

    fn state(&self) -> BusState {
        self.inner.state()
    }

    fn flush_tx_buffer(&self) -> Result<()> {
        self.inner.flush_tx_buffer()
    }

    fn shutdown(&self) -> Result<()> {
        self.inner.shutdown()
    }

    fn channel_info(&self) -> &str {
        self.inner.channel_info()
    }

    #[cfg(unix)]
    fn fileno(&self) -> Option<RawFd> {
        self.inner.fileno()
    }

    #[cfg(windows)]
    fn fileno(&self) -> Option<std::os::windows::io::RawSocket> {
        self.inner.fileno()
    }

    async fn send_periodic(
        &self,
        msgs: &[CanMessage],
        period: Duration,
        duration: Option<Duration>,
    ) -> Result<Box<dyn CyclicTask>> {
        self.inner.send_periodic(msgs, period, duration).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::error::CanError;
    use crate::filter::{CanFilter, CanFilters};

    struct MockBus {
        queue: Mutex<VecDeque<CanMessage>>,
        recv_calls: AtomicUsize,
    }

    impl MockBus {
        fn with_messages(messages: Vec<CanMessage>) -> Self {
            Self {
                queue: Mutex::new(messages.into()),
                recv_calls: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl CanBus for MockBus {
        async fn recv(&self, timeout: Option<Duration>) -> Result<Option<CanMessage>> {
            self.recv_calls.fetch_add(1, Ordering::Relaxed);
            if timeout == Some(Duration::ZERO) {
                return Ok(None);
            }
            Ok(self.queue.lock().pop_front())
        }

        async fn send(&self, _msg: &CanMessage, _timeout: Option<Duration>) -> Result<()> {
            Ok(())
        }

        fn set_filters(&self, _filters: &CanFilters) -> Result<()> {
            Ok(())
        }

        fn protocol(&self) -> CanProtocol {
            CanProtocol::Can20
        }

        fn shutdown(&self) -> Result<()> {
            Ok(())
        }

        async fn send_periodic(
            &self,
            _msgs: &[CanMessage],
            _period: Duration,
            _duration: Option<Duration>,
        ) -> Result<Box<dyn CyclicTask>> {
            Err(CanError::not_supported("send_periodic", "mock bus"))
        }
    }

    #[tokio::test]
    async fn filtered_bus_skips_non_matching_frames() {
        let bus = Arc::new(MockBus::with_messages(vec![
            CanMessage::new(0x100, &[0x01], false).unwrap(),
            CanMessage::new(0x200, &[0x02], false).unwrap(),
            CanMessage::new(0x300, &[0x03], false).unwrap(),
        ]));
        let filters = CanFilters::from(CanFilter::new(0x200, 0x7FF, Some(false)));
        let filtered = FilteredBus::with_filters(bus.clone(), filters);

        let received = filtered
            .recv(Some(Duration::from_millis(100)))
            .await
            .unwrap()
            .expect("matching frame should be returned");
        assert_eq!(received.arbitration_id, 0x200);
        assert_eq!(bus.recv_calls.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn filtered_bus_returns_none_on_timeout() {
        let bus = Arc::new(MockBus::with_messages(vec![
            CanMessage::new(0x100, &[0x01], false).unwrap(),
        ]));
        let filters = CanFilters::from(CanFilter::new(0x200, 0x7FF, Some(false)));
        let filtered = FilteredBus::with_filters(bus, filters);

        let received = filtered
            .recv(Some(Duration::from_millis(10)))
            .await
            .unwrap();
        assert!(received.is_none());
    }

    #[tokio::test]
    async fn empty_filters_match_all_frames() {
        let bus = Arc::new(MockBus::with_messages(vec![
            CanMessage::new(0x999, &[0x01], false).unwrap(),
        ]));
        let filtered = FilteredBus::new(bus);

        let received = filtered.recv(None).await.unwrap().unwrap();
        assert_eq!(received.arbitration_id, 0x999);
    }

    #[tokio::test]
    async fn set_filters_updates_local_state() {
        let bus = Arc::new(MockBus::with_messages(vec![
            CanMessage::new(0x100, &[0x01], false).unwrap(),
        ]));
        let filtered = FilteredBus::new(bus);
        filtered
            .set_filters(&CanFilters::from(CanFilter::new(0x100, 0x7FF, Some(false))))
            .unwrap();

        let received = filtered.recv(None).await.unwrap().unwrap();
        assert_eq!(received.arbitration_id, 0x100);
        assert_eq!(filtered.filters().len(), 1);
    }

    #[tokio::test]
    async fn filtered_bus_delegates_send_and_shutdown() {
        let bus = Arc::new(MockBus::with_messages(vec![]));
        let filtered = FilteredBus::new(bus);
        let msg = CanMessage::new(0x123, &[0x01], false).unwrap();
        filtered.send(&msg, None).await.unwrap();
        assert_eq!(filtered.protocol(), CanProtocol::Can20);
        filtered.shutdown().unwrap();
    }

    #[tokio::test]
    async fn filtered_bus_delegates_send_periodic() {
        let bus = Arc::new(MockBus::with_messages(vec![]));
        let filtered = FilteredBus::new(bus);
        let result = filtered
            .send_periodic(&[], Duration::from_millis(10), None)
            .await;
        assert!(result.is_err());
    }
}

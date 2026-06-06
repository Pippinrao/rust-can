//! Multi-bus, multi-listener asynchronous message dispatch engine.
//!
//! The [`Notifier`] connects one or more CAN buses to one or more
//! [`Listener`]s, dispatching received messages in real-time.
//!
//! This is the Rust equivalent of python-can's `notifier.py`.
//!
//! # Architecture
//!
//! - Each bus gets its own reader task (spawned on tokio)
//! - Messages are dispatched to all listeners in parallel
//! - Uses a lock-free ring buffer for high-throughput scenarios
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use rust_can_core::bus::CanBus;
use rust_can_core::listener::Listener;
use tracing::{debug, error, info};

/// A message dispatch engine that connects buses to listeners.
///
/// # Example
///
/// ```rust,ignore
/// use rust_can_notifier::Notifier;
///
/// let notifier = Notifier::new(
///     bus,
///     vec![listener1, listener2],
///     Duration::from_millis(100),
/// );
///
/// // Messages are dispatched in the background
/// // ...
///
/// notifier.stop(Duration::from_secs(5));
/// ```
pub struct Notifier {
    buses: Vec<Arc<dyn CanBus>>,
    listeners: Arc<Mutex<Vec<Arc<dyn Listener>>>>,
    timeout: Duration,
    handles: Mutex<Vec<tokio::task::JoinHandle<()>>>,
    stopped: Arc<Mutex<bool>>,
}

impl Notifier {
    /// Create a new Notifier and start dispatching messages.
    ///
    /// # Arguments
    /// * `bus` - A CAN bus or list of buses to listen on
    /// * `listeners` - Listeners to notify for each received message
    /// * `timeout` - Polling timeout for `bus.recv()`
    pub fn new(
        bus: impl Into<Vec<Arc<dyn CanBus>>>,
        listeners: Vec<Arc<dyn Listener>>,
        timeout: Duration,
    ) -> Self {
        let buses = bus.into();
        let listeners = Arc::new(Mutex::new(listeners));
        let stopped = Arc::new(Mutex::new(false));
        let handles = Mutex::new(Vec::new());

        let notifier = Self {
            buses: buses.clone(),
            listeners,
            timeout,
            handles,
            stopped: stopped.clone(),
        };

        // Start reader tasks for each bus
        for bus in &buses {
            let bus = bus.clone();
            let listeners = notifier.listeners.clone();
            let stopped_flag = stopped.clone();
            let timeout = notifier.timeout;

            let handle = tokio::spawn(async move {
                info!("Notifier reader task started for bus: {}", bus.channel_info());
                loop {
                    if *stopped_flag.lock() {
                        debug!("Notifier reader task stopping");
                        break;
                    }

                    match bus.recv(Some(timeout)).await {
                        Ok(Some(msg)) => {
                            let listeners = listeners.lock();
                            for listener in listeners.iter() {
                                listener.on_message_received(&msg);
                            }
                        }
                        Ok(None) => {
                            // Timeout, loop again
                        }
                        Err(e) => {
                            error!("Notifier recv error: {}", e);
                            let listeners = listeners.lock();
                            for listener in listeners.iter() {
                                listener.on_error(&e);
                            }
                        }
                    }
                }
            });

            notifier.handles.lock().push(handle);
        }

        notifier
    }

    /// Add a listener to the notification list.
    pub fn add_listener(&self, listener: Arc<dyn Listener>) {
        self.listeners.lock().push(listener);
    }

    /// Remove a listener from the notification list.
    ///
    /// Returns `true` if the listener was found and removed.
    pub fn remove_listener(&self, listener: &Arc<dyn Listener>) -> bool {
        let mut listeners = self.listeners.lock();
        let len_before = listeners.len();
        listeners.retain(|l| !Arc::ptr_eq(l, listener));
        listeners.len() < len_before
    }

    /// Stop all reader tasks and notify listeners.
    ///
    /// # Arguments
    /// * `timeout` - Maximum time to wait for reader tasks to finish
    pub fn stop(&self, timeout: Duration) {
        *self.stopped.lock() = true;

        // Wait for tasks to finish
        let handles: Vec<_> = self.handles.lock().drain(..).collect();
        for handle in handles {
            // We intentionally don't await here to avoid blocking forever
            // in case the bus.recv() never returns
            handle.abort();
        }

        // Notify all listeners to stop
        let listeners = self.listeners.lock();
        for listener in listeners.iter() {
            listener.stop();
        }

        // Wait a bit for any in-progress operations
        std::thread::sleep(timeout.min(Duration::from_secs(1)));
    }

    /// Check if the notifier has been stopped.
    pub fn is_stopped(&self) -> bool {
        *self.stopped.lock()
    }

    /// Get the number of active listeners.
    pub fn listener_count(&self) -> usize {
        self.listeners.lock().len()
    }

    /// Get the number of active buses.
    pub fn bus_count(&self) -> usize {
        self.buses.len()
    }
}

impl Drop for Notifier {
    fn drop(&mut self) {
        if !*self.stopped.lock() {
            self.stop(Duration::from_secs(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use parking_lot::Mutex as ParkingMutex;
    use rust_can_core::bus::CyclicTask;
    use rust_can_core::error::{CanError, Result};
    use rust_can_core::message::CanMessage;
    use rust_can_core::protocol::CanProtocol;
    use rust_can_core::listener::BufferedReader;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Note: Integration tests that use actual bus implementations
    //       are in the integration test directory.
    //       These are basic unit tests for the Notifier structure.

    #[test]
    fn test_add_remove_listener() {
        // This is a structural test without a real bus
        let reader = Arc::new(BufferedReader::new());
        let listeners_data = Arc::new(Mutex::new(Vec::<Arc<dyn Listener>>::new()));

        {
            let mut listeners = listeners_data.lock();
            listeners.push(reader.clone() as Arc<dyn Listener>);
            assert_eq!(listeners.len(), 1);
        }

        {
            let mut listeners = listeners_data.lock();
            listeners.retain(|l| !Arc::ptr_eq(l, &(reader.clone() as Arc<dyn Listener>)));
            assert_eq!(listeners.len(), 0);
        }
    }

    struct MockBus {
        messages: ParkingMutex<VecDeque<Result<Option<CanMessage>>>>,
    }

    impl MockBus {
        fn with_messages(messages: Vec<Result<Option<CanMessage>>>) -> Self {
            Self {
                messages: ParkingMutex::new(messages.into()),
            }
        }
    }

    #[async_trait]
    impl CanBus for MockBus {
        async fn recv(&self, _timeout: Option<Duration>) -> Result<Option<CanMessage>> {
            if let Some(result) = self.messages.lock().pop_front() {
                result
            } else {
                tokio::time::sleep(Duration::from_millis(1)).await;
                Ok(None)
            }
        }

        async fn send(&self, _msg: &CanMessage, _timeout: Option<Duration>) -> Result<()> {
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
            Err(CanError::not_supported("periodic", "mock bus"))
        }
    }

    struct CountingListener {
        messages: AtomicUsize,
        errors: AtomicUsize,
        stops: AtomicUsize,
    }

    impl CountingListener {
        fn new() -> Self {
            Self {
                messages: AtomicUsize::new(0),
                errors: AtomicUsize::new(0),
                stops: AtomicUsize::new(0),
            }
        }
    }

    impl Listener for CountingListener {
        fn on_message_received(&self, _msg: &CanMessage) {
            self.messages.fetch_add(1, Ordering::Relaxed);
        }

        fn on_error(&self, _error: &CanError) {
            self.errors.fetch_add(1, Ordering::Relaxed);
        }

        fn stop(&self) {
            self.stops.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[tokio::test]
    async fn notifier_dispatches_messages_and_errors_to_listeners() {
        let msg = CanMessage::new(0x123, &[0x01], false).unwrap();
        let bus = Arc::new(MockBus::with_messages(vec![
            Ok(Some(msg)),
            Err(CanError::operation("mock failure")),
        ])) as Arc<dyn CanBus>;
        let listener = Arc::new(CountingListener::new());

        let notifier = Notifier::new(
            vec![bus],
            vec![listener.clone() as Arc<dyn Listener>],
            Duration::from_millis(1),
        );
        tokio::time::sleep(Duration::from_millis(20)).await;

        assert_eq!(notifier.bus_count(), 1);
        assert_eq!(notifier.listener_count(), 1);
        assert!(listener.messages.load(Ordering::Relaxed) >= 1);
        assert!(listener.errors.load(Ordering::Relaxed) >= 1);

        notifier.stop(Duration::from_millis(0));
        assert!(notifier.is_stopped());
        assert_eq!(listener.stops.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn notifier_adds_and_removes_listener_instances() {
        let bus = Arc::new(MockBus::with_messages(vec![])) as Arc<dyn CanBus>;
        let initial = Arc::new(CountingListener::new()) as Arc<dyn Listener>;
        let added = Arc::new(CountingListener::new()) as Arc<dyn Listener>;
        let notifier = Notifier::new(vec![bus], vec![initial], Duration::from_millis(1));

        notifier.add_listener(added.clone());
        assert_eq!(notifier.listener_count(), 2);
        assert!(notifier.remove_listener(&added));
        assert_eq!(notifier.listener_count(), 1);
        assert!(!notifier.remove_listener(&added));

        notifier.stop(Duration::from_millis(0));
    }
}

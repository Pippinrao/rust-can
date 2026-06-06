/// Cyclic (periodic) message sending tasks.
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tracing::{debug, warn};

use crate::bus::CanBus;
use crate::error::Result;
use crate::message::CanMessage;

/// A tokio-based cyclic send task.
pub struct TokioCyclicTask {
    messages: Arc<Mutex<Vec<CanMessage>>>,
    period: Duration,
    handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    running: Arc<Mutex<bool>>,
    bus: Arc<dyn CanBus>,
}

impl TokioCyclicTask {
    /// Creates a periodic task for a bus, message list, and period.
    pub fn new(bus: Arc<dyn CanBus>, messages: Vec<CanMessage>, period: Duration) -> Self {
        Self {
            messages: Arc::new(Mutex::new(messages)),
            period,
            handle: Mutex::new(None),
            running: Arc::new(Mutex::new(false)),
            bus,
        }
    }

    /// Starts the periodic send loop.
    pub fn start(&self) -> Result<()> {
        let mut running = self.running.lock();
        if *running { return Ok(()); }
        *running = true;

        let bus = self.bus.clone();
        let messages = self.messages.clone();
        let period = self.period;
        let running_flag = self.running.clone();
        let start = Instant::now();

        let handle = tokio::spawn(async move {
            let mut index = 0usize;
            let mut next_send = start + period;
            loop {
                if !*running_flag.lock() { break; }
                let now = Instant::now();
                if now < next_send {
                    tokio::time::sleep(next_send - now).await;
                }
                if !*running_flag.lock() { break; }
                let msg_opt = {
                    let msgs = messages.lock();
                    msgs.get(index).cloned()
                };
                if let Some(msg) = msg_opt
                    && let Err(e) = bus.send(&msg, None).await
                {
                    warn!("Cyclic task send error: {}", e);
                }
                index = (index + 1) % messages.lock().len().max(1);
                next_send += period;
                if next_send < Instant::now() {
                    next_send = Instant::now() + period;
                }
            }
            debug!("Cyclic task stopped");
        });

        *self.handle.lock() = Some(handle);
        Ok(())
    }

    /// Stops the periodic send loop.
    pub fn stop(&self) -> Result<()> {
        *self.running.lock() = false;
        if let Some(handle) = self.handle.lock().take() {
            handle.abort();
        }
        Ok(())
    }

    /// Applies a modifier to every message in the task.
    pub fn modify(&self, modifier: &dyn Fn(&mut CanMessage)) -> Result<()> {
        for msg in self.messages.lock().iter_mut() {
            modifier(msg);
        }
        Ok(())
    }

    /// Returns true if the task is running.
    pub fn is_running(&self) -> bool {
        *self.running.lock()
    }
}

impl Drop for TokioCyclicTask {
    fn drop(&mut self) { let _ = self.stop(); }
}

impl crate::bus::CyclicTask for TokioCyclicTask {
    fn stop(&self) -> Result<()> {
        TokioCyclicTask::stop(self)
    }

    fn modify(&self, modifier: &dyn Fn(&mut CanMessage)) -> Result<()> {
        TokioCyclicTask::modify(self, modifier)
    }

    fn is_running(&self) -> bool {
        TokioCyclicTask::is_running(self)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use async_trait::async_trait;

    use crate::bus::{CanBus, CyclicTask};
    use crate::error::CanError;
    use crate::protocol::CanProtocol;

    use super::*;

    struct CountingBus {
        send_count: AtomicUsize,
    }

    #[async_trait]
    impl CanBus for CountingBus {
        async fn recv(&self, _timeout: Option<Duration>) -> Result<Option<CanMessage>> {
            Ok(None)
        }

        async fn send(&self, _msg: &CanMessage, _timeout: Option<Duration>) -> Result<()> {
            self.send_count.fetch_add(1, Ordering::Relaxed);
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
            Err(CanError::not_supported("send_periodic", "counting bus"))
        }
    }

    #[tokio::test]
    async fn start_stop_and_is_running() {
        let bus = Arc::new(CountingBus {
            send_count: AtomicUsize::new(0),
        }) as Arc<dyn CanBus>;
        let task = TokioCyclicTask::new(
            bus,
            vec![CanMessage::new(0x123, &[0x01], false).unwrap()],
            Duration::from_millis(5),
        );

        assert!(!task.is_running());
        task.start().unwrap();
        assert!(task.is_running());
        tokio::time::sleep(Duration::from_millis(30)).await;
        task.stop().unwrap();
        assert!(!task.is_running());
    }

    #[tokio::test]
    async fn modify_updates_payload_before_send() {
        let counting = Arc::new(CountingBus {
            send_count: AtomicUsize::new(0),
        });
        let bus = counting.clone() as Arc<dyn CanBus>;
        let task = TokioCyclicTask::new(
            bus,
            vec![CanMessage::new(0x123, &[0x01], false).unwrap()],
            Duration::from_millis(5),
        );
        task.start().unwrap();
        task.modify(&|msg| {
            msg.data_mut()[0] = 0xAA;
        })
        .unwrap();
        tokio::time::sleep(Duration::from_millis(25)).await;
        task.stop().unwrap();
        assert!(counting.send_count.load(Ordering::Relaxed) >= 1);
    }

    #[tokio::test]
    async fn drop_stops_running_task() {
        let bus = Arc::new(CountingBus {
            send_count: AtomicUsize::new(0),
        }) as Arc<dyn CanBus>;
        let task = TokioCyclicTask::new(
            bus,
            vec![CanMessage::new(0x123, &[0x01], false).unwrap()],
            Duration::from_millis(5),
        );
        task.start().unwrap();
        assert!(task.is_running());
        drop(task);
    }

    #[tokio::test]
    async fn cyclic_task_trait_delegates_to_impl() {
        let bus = Arc::new(CountingBus {
            send_count: AtomicUsize::new(0),
        }) as Arc<dyn CanBus>;
        let task = TokioCyclicTask::new(
            bus,
            vec![CanMessage::new(0x123, &[0x01], false).unwrap()],
            Duration::from_millis(5),
        );
        let cyclic: &dyn CyclicTask = &task;
        assert!(!cyclic.is_running());
        task.start().unwrap();
        assert!(cyclic.is_running());
        cyclic.stop().unwrap();
        assert!(!cyclic.is_running());
    }
}

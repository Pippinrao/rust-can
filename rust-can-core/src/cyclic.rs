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

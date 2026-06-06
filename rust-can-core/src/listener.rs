/// Listener and message buffering traits.
use std::sync::Arc;

use crossbeam::channel::{self, Receiver, Sender};
use parking_lot::Mutex;

use crate::message::CanMessage;

/// The basic listener trait.
pub trait Listener: Send + Sync {
    /// Called when a new message is received.
    fn on_message_received(&self, msg: &CanMessage);

    /// Called when an error occurs in the notification thread.
    fn on_error(&self, _error: &crate::error::CanError) {}

    /// Called when the listener should stop and flush any pending data.
    fn stop(&self) {}
}

/// A listener that buffers messages in a FIFO queue.
#[derive(Clone)]
pub struct BufferedReader {
    tx: Sender<CanMessage>,
    rx: Receiver<CanMessage>,
    stopped: Arc<Mutex<bool>>,
}

impl BufferedReader {
    /// Creates an empty buffered reader.
    pub fn new() -> Self {
        let (tx, rx) = channel::unbounded();
        Self { tx, rx, stopped: Arc::new(Mutex::new(false)) }
    }

    /// Receives the next buffered message, waiting up to `timeout`.
    pub fn get_message(&self, timeout: Option<std::time::Duration>) -> Option<CanMessage> {
        let stopped = *self.stopped.lock();
        match timeout {
            None => {
                if stopped { self.rx.try_recv().ok() } else { None }
            }
            Some(dur) => {
                if stopped { self.rx.try_recv().ok() }
                else { self.rx.recv_timeout(dur).ok() }
            }
        }
    }

    /// Attempts to receive a buffered message without blocking.
    pub fn try_get_message(&self) -> Option<CanMessage> {
        self.rx.try_recv().ok()
    }
}

impl Listener for BufferedReader {
    fn on_message_received(&self, msg: &CanMessage) {
        let stopped = *self.stopped.lock();
        if !stopped {
            let _ = self.tx.send(msg.clone());
        }
    }

    fn stop(&self) {
        *self.stopped.lock() = true;
    }
}

impl Default for BufferedReader {
    fn default() -> Self { Self::new() }
}

/// A listener that prints each message to stdout.
pub struct PrinterListener {
    prefix: String,
}

impl PrinterListener {
    /// Creates a printing listener with an optional prefix.
    pub fn new(prefix: impl Into<String>) -> Self {
        Self { prefix: prefix.into() }
    }
}

impl Listener for PrinterListener {
    fn on_message_received(&self, msg: &CanMessage) {
        if self.prefix.is_empty() {
            println!("{}", msg);
        } else {
            println!("{} {}", self.prefix, msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffered_reader_basic() {
        let reader = BufferedReader::new();
        let msg = CanMessage::new(0x100, &[0x01], false).unwrap();
        reader.on_message_received(&msg);
        assert!(reader.try_get_message().is_some());
    }

    #[test]
    fn test_buffered_reader_stop() {
        let reader = BufferedReader::new();
        reader.stop();
        let msg = CanMessage::new(0x100, &[0x01], false).unwrap();
        reader.on_message_received(&msg);
        assert!(reader.try_get_message().is_none());
    }

    #[test]
    fn buffered_reader_waits_with_timeout_until_message_arrives() {
        let reader = BufferedReader::new();
        let cloned = reader.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(5));
            let msg = CanMessage::new(0x200, &[0x02], false).unwrap();
            cloned.on_message_received(&msg);
        });

        let received = reader
            .get_message(Some(std::time::Duration::from_millis(100)))
            .unwrap();
        assert_eq!(received.arbitration_id, 0x200);
    }

    #[test]
    fn stopped_reader_drains_already_buffered_messages_without_blocking() {
        let reader = BufferedReader::new();
        let msg = CanMessage::new(0x300, &[0x03], false).unwrap();
        reader.on_message_received(&msg);
        reader.stop();
        assert_eq!(reader.get_message(None).unwrap().arbitration_id, 0x300);
        assert!(reader.get_message(None).is_none());
    }

    #[test]
    fn printer_listener_accepts_empty_and_prefixed_output_paths() {
        let msg = CanMessage::new(0x100, &[0x01], false).unwrap();
        PrinterListener::new("").on_message_received(&msg);
        PrinterListener::new("can0").on_message_received(&msg);
    }
}

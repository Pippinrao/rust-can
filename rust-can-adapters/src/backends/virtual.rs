/// Virtual CAN bus adapter — no hardware required.
use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use rust_can_core::error::{CanError, Result};
use rust_can_core::filter::CanFilters;
use rust_can_core::frame::CanFrame;

use crate::adapter::CanAdapter;
use crate::config::AdapterConfig;
use crate::registry::{AdapterInfo, ADAPTER_REGISTRY};

static CHANNELS: LazyLock<Mutex<HashMap<String, Vec<crossbeam::channel::Sender<CanFrame>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// Register virtual adapter on module load
static _REGISTER: LazyLock<()> = LazyLock::new(|| {
    ADAPTER_REGISTRY.write().push(
        AdapterInfo::new("virtual", "Virtual CAN bus for testing")
            .with_fd_support(true)
            .with_xl_support(true),
    );
});

/// In-memory adapter used for tests and benchmarks.
pub struct VirtualAdapter {
    channel_name: String,
    receive_own_messages: bool,
    preserve_timestamps: bool,
    tx: crossbeam::channel::Sender<CanFrame>,
    rx: crossbeam::channel::Receiver<CanFrame>,
    _filters: Mutex<CanFilters>,
    is_open: Mutex<bool>,
}

impl VirtualAdapter {
    fn now_nanos() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }
}

impl CanAdapter for VirtualAdapter {
    fn open(config: &AdapterConfig) -> Result<Self> {
        let channel = config.channel.clone().unwrap_or_else(|| "vcan0".to_string());
        let receive_own = config.get_bool("receive_own_messages").unwrap_or(false);
        let preserve_ts = config.get_bool("preserve_timestamps").unwrap_or(false);

        let rx_queue_size = config.get_int("rx_queue_size").unwrap_or(0) as usize;
        let (tx, rx) = if rx_queue_size > 0 {
            crossbeam::channel::bounded(rx_queue_size)
        } else {
            crossbeam::channel::unbounded()
        };

        let mut channels = CHANNELS.lock();
        let entry = channels.entry(channel.clone()).or_default();
        entry.push(tx.clone());

        Ok(Self {
            channel_name: channel,
            receive_own_messages: receive_own,
            preserve_timestamps: preserve_ts,
            tx,
            rx,
            _filters: Mutex::new(CanFilters::new()),
            is_open: Mutex::new(true),
        })
    }

    fn read_frame(&self, timeout: Option<Duration>) -> Result<CanFrame> {
        if !*self.is_open.lock() {
            return Err(CanError::operation("Cannot read from a closed bus"));
        }
        match timeout {
            None => self.rx.recv().map_err(|e| CanError::operation(format!("recv error: {}", e))),
            Some(dur) => self.rx.recv_timeout(dur).map_err(|e| match e {
                crossbeam::channel::RecvTimeoutError::Timeout => CanError::timeout("recv timeout"),
                crossbeam::channel::RecvTimeoutError::Disconnected => CanError::operation("channel disconnected"),
            }),
        }
    }

    fn write_frame(&self, frame: &CanFrame, timeout: Option<Duration>) -> Result<()> {
        if !*self.is_open.lock() {
            return Err(CanError::operation("Cannot write to a closed bus"));
        }
        let timestamp = if self.preserve_timestamps && frame.timestamp > 0 {
            frame.timestamp
        } else {
            Self::now_nanos()
        };

        let channels = CHANNELS.lock();
        if let Some(senders) = channels.get(&self.channel_name) {
            for sender in senders {
                if sender.same_channel(&self.tx) && !self.receive_own_messages {
                    continue;
                }
                let mut frame_copy = frame.clone();
                frame_copy.timestamp = timestamp;
                let result: std::result::Result<(), String> = match timeout {
                    None => sender.send(frame_copy).map_err(|e| e.to_string()),
                    Some(dur) => sender
                        .send_timeout(frame_copy, dur)
                        .map_err(|e| e.to_string()),
                };
                if let Err(e) = result {
                    return Err(CanError::operation(format!("Failed to deliver message: {}", e)));
                }
            }
        }
        Ok(())
    }

    fn info(&self) -> AdapterInfo {
        AdapterInfo::new("virtual", "Virtual CAN bus for testing")
            .with_fd_support(true)
            .with_xl_support(true)
    }

    fn close(&self) -> Result<()> {
        let mut is_open = self.is_open.lock();
        if !*is_open { return Ok(()); }
        *is_open = false;
        let mut channels = CHANNELS.lock();
        if let Some(senders) = channels.get_mut(&self.channel_name) {
            senders.retain(|s| !s.same_channel(&self.tx));
            if senders.is_empty() {
                channels.remove(&self.channel_name);
            }
        }
        Ok(())
    }
}

impl Drop for VirtualAdapter {
    fn drop(&mut self) { let _ = self.close(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_virtual_send_recv() {
        let config1 = AdapterConfig::with_interface_and_channel("virtual", "test-chan-1");
        let config2 = AdapterConfig::with_interface_and_channel("virtual", "test-chan-1");
        let adapter1 = VirtualAdapter::open(&config1).unwrap();
        let adapter2 = VirtualAdapter::open(&config2).unwrap();
        let frame = CanFrame::new_data(0x123, Bytes::from(vec![1u8, 2, 3]), false);
        adapter1.write_frame(&frame, None).unwrap();
        let received = adapter2.read_frame(Some(Duration::from_millis(100))).unwrap();
        assert_eq!(received.can_id, 0x123);
        assert_eq!(&received.data[..], &[1, 2, 3]);
    }

    #[test]
    fn test_receive_own_messages() {
        let mut config = AdapterConfig::with_interface_and_channel("virtual", "test-chan-2");
        config.set_bool("receive_own_messages", true);
        let adapter = VirtualAdapter::open(&config).unwrap();
        let frame = CanFrame::new_data(0x200, Bytes::from(vec![4u8, 5]), false);
        adapter.write_frame(&frame, None).unwrap();
        let received = adapter.read_frame(Some(Duration::from_millis(100))).unwrap();
        assert_eq!(received.can_id, 0x200);
    }

    #[test]
    fn test_multiple_receivers() {
        let mut config = AdapterConfig::with_interface_and_channel("virtual", "test-chan-3");
        config.set_bool("receive_own_messages", true);
        let adapter1 = VirtualAdapter::open(&config).unwrap();
        let adapter2 = VirtualAdapter::open(&config).unwrap();
        let adapter3 = VirtualAdapter::open(&config).unwrap();
        let frame = CanFrame::new_data(0x300, Bytes::from(vec![7u8]), false);
        adapter1.write_frame(&frame, None).unwrap();
        assert_eq!(adapter2.read_frame(Some(Duration::from_millis(100))).unwrap().can_id, 0x300);
        assert_eq!(adapter3.read_frame(Some(Duration::from_millis(100))).unwrap().can_id, 0x300);
        assert_eq!(adapter1.read_frame(Some(Duration::from_millis(100))).unwrap().can_id, 0x300);
    }

    #[test]
    fn read_timeout_and_close_errors_are_reported() {
        let adapter = VirtualAdapter::open(&AdapterConfig::with_interface_and_channel(
            "virtual",
            "test-timeout-close",
        ))
        .unwrap();
        assert!(adapter.read_frame(Some(Duration::from_millis(1))).is_err());
        adapter.close().unwrap();
        assert!(adapter.read_frame(Some(Duration::from_millis(1))).is_err());
        assert!(adapter.write_frame(&CanFrame::new_data(0x123, Bytes::new(), false), None).is_err());
        adapter.close().unwrap();
    }

    #[test]
    fn preserve_timestamps_keeps_nonzero_source_timestamp() {
        let mut config1 =
            AdapterConfig::with_interface_and_channel("virtual", "test-preserve-timestamp");
        config1.set_bool("preserve_timestamps", true);
        config1.set_bool("receive_own_messages", true);
        let adapter = VirtualAdapter::open(&config1).unwrap();

        let frame = CanFrame::new_data(0x456, Bytes::from_static(&[0x44]), false)
            .with_timestamp(123_456);
        adapter.write_frame(&frame, None).unwrap();
        let received = adapter.read_frame(Some(Duration::from_millis(100))).unwrap();
        assert_eq!(received.timestamp, 123_456);
    }

    #[test]
    fn bounded_queue_reports_delivery_failure_when_receiver_is_full() {
        let mut sender_config =
            AdapterConfig::with_interface_and_channel("virtual", "test-bounded-queue");
        sender_config.set_bool("receive_own_messages", false);
        let sender = VirtualAdapter::open(&sender_config).unwrap();

        let mut receiver_config =
            AdapterConfig::with_interface_and_channel("virtual", "test-bounded-queue");
        receiver_config.set_int("rx_queue_size", 1);
        let _receiver = VirtualAdapter::open(&receiver_config).unwrap();

        let frame = CanFrame::new_data(0x777, Bytes::from_static(&[0x77]), false);
        sender.write_frame(&frame, Some(Duration::from_millis(1))).unwrap();
        assert!(sender.write_frame(&frame, Some(Duration::from_millis(1))).is_err());
    }

    #[test]
    fn info_reports_virtual_capabilities() {
        let adapter = VirtualAdapter::open(&AdapterConfig::with_interface_and_channel(
            "virtual",
            "test-info",
        ))
        .unwrap();
        let info = adapter.info();
        assert_eq!(info.name, "virtual");
        assert!(info.supports_fd);
        assert!(info.supports_xl);
    }
}

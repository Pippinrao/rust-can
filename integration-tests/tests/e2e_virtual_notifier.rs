//! E2E-ADP-001 / E2E-NTF-001: virtual adapter and notifier → BufferedReader chain.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use rust_can_adapters::CanAdapter;
use rust_can_adapters::backends::r#virtual::VirtualAdapter;
use rust_can_adapters::config::AdapterConfig;
use rust_can_core::bus::{CanBus, CyclicTask};
use rust_can_core::error::{CanError, Result};
use rust_can_core::frame::CanFrame;
use rust_can_core::listener::{BufferedReader, Listener};
use rust_can_core::message::CanMessage;
use rust_can_core::protocol::CanProtocol;
use rust_can_notifier::Notifier;

/// Wraps a virtual adapter as an async [`CanBus`] for notifier integration.
struct VirtualBus {
    adapter: Arc<VirtualAdapter>,
    channel: String,
}

impl VirtualBus {
    fn open_receiver(channel: &str) -> Result<Arc<Self>> {
        let mut config = AdapterConfig::with_interface_and_channel("virtual", channel);
        config.set_bool("receive_own_messages", false);
        let adapter = Arc::new(VirtualAdapter::open(&config)?);
        Ok(Arc::new(Self {
            adapter,
            channel: channel.to_string(),
        }))
    }
}

fn open_sender(channel: &str) -> Result<VirtualAdapter> {
    let mut config = AdapterConfig::with_interface_and_channel("virtual", channel);
    config.set_bool("receive_own_messages", false);
    VirtualAdapter::open(&config)
}

#[async_trait]
impl CanBus for VirtualBus {
    async fn recv(&self, timeout: Option<Duration>) -> Result<Option<CanMessage>> {
        let adapter = self.adapter.clone();
        tokio::task::spawn_blocking(move || match adapter.read_frame(timeout) {
            Ok(frame) => Ok(Some(frame.into())),
            Err(CanError::TimeoutError { .. }) => Ok(None),
            Err(error) => Err(error),
        })
        .await
        .expect("virtual recv task should complete")
    }

    async fn send(&self, msg: &CanMessage, timeout: Option<Duration>) -> Result<()> {
        let frame: CanFrame = msg.clone().into();
        let adapter = self.adapter.clone();
        tokio::task::spawn_blocking(move || adapter.write_frame(&frame, timeout))
            .await
            .expect("virtual send task should complete")
    }

    fn protocol(&self) -> CanProtocol {
        CanProtocol::Can20
    }

    fn shutdown(&self) -> Result<()> {
        self.adapter.close()
    }

    fn channel_info(&self) -> &str {
        &self.channel
    }

    async fn send_periodic(
        &self,
        _msgs: &[CanMessage],
        _period: Duration,
        _duration: Option<Duration>,
    ) -> Result<Box<dyn CyclicTask>> {
        Err(CanError::not_supported("send_periodic", "virtual bus e2e"))
    }
}

#[test]
fn e2e_adp_001_virtual_send_recv_multiple_receivers() {
    let channel = format!("e2e-adp-{}", std::process::id());
    let mut config = AdapterConfig::with_interface_and_channel("virtual", &channel);
    config.set_bool("receive_own_messages", true);

    let sender = VirtualAdapter::open(&config).unwrap();
    let receiver_a = VirtualAdapter::open(&config).unwrap();
    let receiver_b = VirtualAdapter::open(&config).unwrap();

    let frame = CanFrame::new_data(0x300, Bytes::from_static(&[0xAA, 0xBB]), false);
    sender.write_frame(&frame, None).unwrap();

    let received_a = receiver_a
        .read_frame(Some(Duration::from_millis(200)))
        .unwrap();
    let received_b = receiver_b
        .read_frame(Some(Duration::from_millis(200)))
        .unwrap();

    assert_eq!(received_a.can_id, 0x300);
    assert_eq!(received_b.can_id, 0x300);
    assert_eq!(&received_a.data[..], &[0xAA, 0xBB]);
    assert_eq!(&received_b.data[..], &[0xAA, 0xBB]);
}

#[tokio::test]
async fn e2e_ntf_001_virtual_notifier_buffered_reader_chain() {
    let channel = format!("e2e-ntf-{}", std::process::id());
    let bus = VirtualBus::open_receiver(&channel).unwrap();
    let sender = open_sender(&channel).unwrap();
    let reader = Arc::new(BufferedReader::new());

    let notifier = Notifier::new(
        vec![bus.clone() as Arc<dyn CanBus>],
        vec![reader.clone() as Arc<dyn Listener>],
        Duration::from_millis(5),
    );

    let frame = CanFrame::new_data(0x456, Bytes::from_static(&[0x10, 0x20, 0x30]), false);
    sender.write_frame(&frame, None).unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    let received = reader
        .get_message(Some(Duration::from_millis(500)))
        .expect("BufferedReader should receive dispatched message");
    assert_eq!(received.arbitration_id, 0x456);
    assert_eq!(received.data_slice(), &[0x10, 0x20, 0x30]);

    notifier.stop(Duration::from_millis(100));
    assert!(notifier.is_stopped());
}

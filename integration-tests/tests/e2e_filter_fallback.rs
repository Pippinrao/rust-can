//! E2E-COR-001: software filter fallback on bus recv path.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use rust_can_adapters::CanAdapter;
use rust_can_adapters::backends::r#virtual::VirtualAdapter;
use rust_can_adapters::config::AdapterConfig;
use rust_can_core::bus::{CanBus, CyclicTask, FilteredBus};
use rust_can_core::error::{CanError, Result};
use rust_can_core::filter::{CanFilter, CanFilters};
use rust_can_core::frame::CanFrame;
use rust_can_core::message::CanMessage;
use rust_can_core::protocol::CanProtocol;

struct VirtualBus {
    adapter: Arc<VirtualAdapter>,
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

    fn set_filters(&self, filters: &CanFilters) -> Result<()> {
        self.adapter.apply_hardware_filters(filters)
    }

    fn protocol(&self) -> CanProtocol {
        CanProtocol::Can20
    }

    fn shutdown(&self) -> Result<()> {
        self.adapter.close()
    }

    async fn send_periodic(
        &self,
        _msgs: &[CanMessage],
        _period: Duration,
        _duration: Option<Duration>,
    ) -> Result<Box<dyn CyclicTask>> {
        Err(CanError::not_supported("send_periodic", "e2e virtual bus"))
    }
}

fn open_virtual(channel: &str, receive_own: bool) -> Result<VirtualAdapter> {
    let mut config = AdapterConfig::with_interface_and_channel("virtual", channel);
    config.set_bool("receive_own_messages", receive_own);
    VirtualAdapter::open(&config)
}

#[tokio::test]
async fn e2e_cor_001_filtered_bus_recv_skips_unmatched_ids() {
    let channel = format!("e2e-cor-001-{}", std::process::id());
    let sender = open_virtual(&channel, false).unwrap();
    let receiver = open_virtual(&channel, false).unwrap();
    let inner = Arc::new(VirtualBus {
        adapter: Arc::new(receiver),
    });
    let filters = CanFilters::from(CanFilter::new(0x200, 0x7FF, Some(false)));
    let bus = FilteredBus::with_filters(inner, filters);

    sender
        .write_frame(&CanFrame::new_data(0x100, Bytes::from_static(&[0x01]), false), None)
        .unwrap();
    sender
        .write_frame(&CanFrame::new_data(0x200, Bytes::from_static(&[0x02]), false), None)
        .unwrap();

    let received = bus
        .recv(Some(Duration::from_millis(500)))
        .await
        .unwrap()
        .expect("filtered recv should return matching ID");
    assert_eq!(received.arbitration_id, 0x200);
    assert_eq!(received.data_slice(), &[0x02]);
}

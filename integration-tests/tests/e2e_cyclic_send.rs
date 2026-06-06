//! E2E-CYC-001: TokioCyclicTask periodic send on virtual bus.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use rust_can_adapters::CanAdapter;
use rust_can_adapters::backends::r#virtual::VirtualAdapter;
use rust_can_adapters::config::AdapterConfig;
use rust_can_core::bus::{CanBus, CyclicTask};
use rust_can_core::cyclic::TokioCyclicTask;
use rust_can_core::error::{CanError, Result};
use rust_can_core::frame::CanFrame;
use rust_can_core::message::CanMessage;
use rust_can_core::protocol::CanProtocol;

struct CyclicVirtualBus {
    adapter: VirtualAdapter,
    send_count: Arc<AtomicUsize>,
}

#[async_trait]
impl CanBus for CyclicVirtualBus {
    async fn recv(&self, timeout: Option<Duration>) -> Result<Option<CanMessage>> {
        match self.adapter.read_frame(timeout) {
            Ok(frame) => Ok(Some(frame.into())),
            Err(CanError::TimeoutError { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }

    async fn send(&self, msg: &CanMessage, timeout: Option<Duration>) -> Result<()> {
        self.send_count.fetch_add(1, Ordering::Relaxed);
        let frame: CanFrame = msg.clone().into();
        self.adapter.write_frame(&frame, timeout)
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
        Err(CanError::not_supported("send_periodic", "e2e cyclic bus"))
    }
}

#[tokio::test]
async fn e2e_cyc_001_tokio_cyclic_task_sends_periodically() {
    let channel = format!("e2e-cyc-001-{}", std::process::id());
    let mut config = AdapterConfig::with_interface_and_channel("virtual", &channel);
    config.set_bool("receive_own_messages", true);
    let send_count = Arc::new(AtomicUsize::new(0));
    let bus = Arc::new(CyclicVirtualBus {
        adapter: VirtualAdapter::open(&config).unwrap(),
        send_count: send_count.clone(),
    }) as Arc<dyn CanBus>;

    let task = TokioCyclicTask::new(
        bus,
        vec![CanMessage::new(0x123, &[0x01, 0x02], false).unwrap()],
        Duration::from_millis(10),
    );
    task.start().unwrap();
    tokio::time::sleep(Duration::from_millis(120)).await;
    task.stop().unwrap();

    assert!(!task.is_running());
    assert!(
        send_count.load(Ordering::Relaxed) >= 3,
        "expected multiple periodic sends, got {}",
        send_count.load(Ordering::Relaxed)
    );
}

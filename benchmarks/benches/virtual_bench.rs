#![allow(missing_docs)]
//! Criterion benchmarks for virtual adapter throughput and notifier dispatch.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use parking_lot::Mutex;
use rust_can_adapters::CanAdapter;
use rust_can_adapters::backends::r#virtual::VirtualAdapter;
use rust_can_adapters::config::AdapterConfig;
use rust_can_core::bus::{CanBus, CyclicTask};
use rust_can_core::error::{CanError, Result};
use rust_can_core::frame::CanFrame;
use rust_can_core::listener::Listener;
use rust_can_core::message::CanMessage;
use rust_can_core::protocol::CanProtocol;
use rust_can_notifier::Notifier;

struct BenchVirtualBus {
    adapter: Mutex<VirtualAdapter>,
}

impl BenchVirtualBus {
    fn pair(channel: &str) -> (Self, VirtualAdapter) {
        let mut tx_config = AdapterConfig::with_interface_and_channel("virtual", channel);
        tx_config.set_bool("receive_own_messages", false);
        let mut rx_config = AdapterConfig::with_interface_and_channel("virtual", channel);
        rx_config.set_bool("receive_own_messages", false);
        let tx_adapter = VirtualAdapter::open(&tx_config).unwrap();
        let rx_adapter = VirtualAdapter::open(&rx_config).unwrap();
        (
            Self {
                adapter: Mutex::new(tx_adapter),
            },
            rx_adapter,
        )
    }
}

struct CountingListener {
    count: std::sync::atomic::AtomicUsize,
}

impl Listener for CountingListener {
    fn on_message_received(&self, _msg: &CanMessage) {
        self.count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

#[async_trait]
impl CanBus for BenchVirtualBus {
    async fn recv(&self, timeout: Option<Duration>) -> Result<Option<CanMessage>> {
        match self.adapter.lock().read_frame(timeout) {
            Ok(frame) => Ok(Some(frame.into())),
            Err(CanError::TimeoutError { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }

    async fn send(&self, msg: &CanMessage, timeout: Option<Duration>) -> Result<()> {
        let frame: CanFrame = msg.clone().into();
        self.adapter.lock().write_frame(&frame, timeout)
    }

    fn protocol(&self) -> CanProtocol {
        CanProtocol::Can20
    }

    fn shutdown(&self) -> Result<()> {
        self.adapter.lock().close()
    }

    async fn send_periodic(
        &self,
        _msgs: &[CanMessage],
        _period: Duration,
        _duration: Option<Duration>,
    ) -> Result<Box<dyn CyclicTask>> {
        Err(CanError::not_supported("send_periodic", "bench virtual bus"))
    }
}

fn bench_virtual_send_recv(c: &mut Criterion) {
    let channel = format!("virtual-bench-{}", std::process::id());
    let (tx_bus, rx_adapter) = BenchVirtualBus::pair(&channel);
    let frame = CanFrame::new_data(0x123, Bytes::from_static(&[1, 2, 3, 4]), false);

    c.bench_function("virtual_send_recv_roundtrip", |b| {
        b.iter(|| {
            tx_bus
                .adapter
                .lock()
                .write_frame(black_box(&frame), None)
                .unwrap();
            let received = rx_adapter
                .read_frame(Some(Duration::from_millis(10)))
                .unwrap();
            black_box(received);
        });
    });
}

fn bench_notifier_dispatch(c: &mut Criterion) {
    c.bench_function("notifier_listener_dispatch", |b| {
        b.iter_batched(
            || {
                let channel = format!("notifier-bench-{}", std::process::id());
                let bus = Arc::new(BenchVirtualBus::pair(&channel).0) as Arc<dyn CanBus>;
                let listener = Arc::new(CountingListener {
                    count: std::sync::atomic::AtomicUsize::new(0),
                });
                let notifier = Notifier::new(
                    vec![bus.clone()],
                    vec![listener.clone() as Arc<dyn Listener>],
                    Duration::from_millis(1),
                );
                (bus, listener, notifier)
            },
            |(bus, listener, notifier)| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    for index in 0..32 {
                        let msg =
                            CanMessage::new(0x100 + index, &[index as u8], false).unwrap();
                        bus.send(&msg, None).await.unwrap();
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    black_box(listener.count.load(std::sync::atomic::Ordering::Relaxed));
                });
                notifier.stop(Duration::from_millis(0));
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, bench_virtual_send_recv, bench_notifier_dispatch);
criterion_main!(benches);

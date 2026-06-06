//! Emit machine-readable virtual bus throughput results for python-can comparison.

use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use bytes::Bytes;
use rust_can_adapters::CanAdapter;
use rust_can_adapters::backends::r#virtual::VirtualAdapter;
use rust_can_adapters::config::AdapterConfig;
use rust_can_core::bus::{CanBus, CyclicTask};
use rust_can_core::error::{CanError, Result};
use rust_can_core::frame::CanFrame;
use rust_can_core::message::CanMessage;
use rust_can_core::protocol::CanProtocol;
use tokio::runtime::Runtime;

struct BenchVirtualBus {
    tx: VirtualAdapter,
    rx: VirtualAdapter,
}

impl BenchVirtualBus {
    fn pair(channel: &str) -> Self {
        let mut tx_config = AdapterConfig::with_interface_and_channel("virtual", channel);
        tx_config.set_bool("receive_own_messages", false);
        let mut rx_config = AdapterConfig::with_interface_and_channel("virtual", channel);
        rx_config.set_bool("receive_own_messages", false);
        Self {
            tx: VirtualAdapter::open(&tx_config).unwrap(),
            rx: VirtualAdapter::open(&rx_config).unwrap(),
        }
    }
}

#[async_trait]
impl CanBus for BenchVirtualBus {
    async fn recv(&self, timeout: Option<Duration>) -> Result<Option<CanMessage>> {
        match self.rx.read_frame(timeout) {
            Ok(frame) => Ok(Some(frame.into())),
            Err(CanError::TimeoutError { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }

    async fn send(&self, msg: &CanMessage, timeout: Option<Duration>) -> Result<()> {
        let frame: CanFrame = msg.clone().into();
        self.tx.write_frame(&frame, timeout)
    }

    fn protocol(&self) -> CanProtocol {
        CanProtocol::Can20
    }

    fn shutdown(&self) -> Result<()> {
        self.tx.close()?;
        self.rx.close()
    }

    async fn send_periodic(
        &self,
        _msgs: &[CanMessage],
        _period: Duration,
        _duration: Option<Duration>,
    ) -> Result<Box<dyn CyclicTask>> {
        Err(CanError::not_supported("send_periodic", "virtual perf compare"))
    }
}

struct BenchResult {
    name: &'static str,
    iterations: u64,
    total_ns: u128,
}

fn bench_sync(name: &'static str, iterations: u64, mut f: impl FnMut()) -> BenchResult {
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    BenchResult {
        name,
        iterations,
        total_ns: start.elapsed().as_nanos(),
    }
}

fn main() {
    let iterations = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse::<u64>().ok())
        .unwrap_or(100_000);

    let channel = format!("virtual-perf-{}", std::process::id());
    let bus = BenchVirtualBus::pair(&channel);
    let frame = CanFrame::new_data(0x123, Bytes::from_static(&[1, 2, 3, 4]), false);

    let sync_result = bench_sync("virtual_send_recv_roundtrip", iterations, || {
        bus.tx.write_frame(black_box(&frame), None).unwrap();
        let received = bus
            .rx
            .read_frame(Some(Duration::from_millis(10)))
            .unwrap();
        black_box(received);
    });

    let rt = Runtime::new().expect("tokio runtime should start");
    let async_channel = format!("virtual-async-perf-{}", std::process::id());
    let async_bus = Arc::new(BenchVirtualBus::pair(&async_channel));
    let async_msg = CanMessage::new(0x456, &[0x10, 0x20], false).unwrap();
    let async_start = Instant::now();
    rt.block_on(async {
        for _ in 0..iterations {
            async_bus.send(black_box(&async_msg), None).await.unwrap();
            let received = async_bus.recv(Some(Duration::from_millis(10))).await.unwrap();
            black_box(received);
        }
    });
    let async_result = BenchResult {
        name: "virtual_async_send_recv_roundtrip",
        iterations,
        total_ns: async_start.elapsed().as_nanos(),
    };

    let results = [sync_result, async_result];
    println!("{{");
    println!("  \"language\": \"rust\",");
    println!("  \"iterations\": {},", iterations);
    println!("  \"results\": [");
    for (idx, result) in results.iter().enumerate() {
        let comma = if idx + 1 == results.len() { "" } else { "," };
        let ns_per_iter = result.total_ns as f64 / result.iterations as f64;
        println!(
            "    {{\"name\":\"{}\",\"iterations\":{},\"total_ns\":{},\"ns_per_iter\":{:.3}}}{}",
            result.name, result.iterations, result.total_ns, ns_per_iter, comma
        );
    }
    println!("  ]");
    println!("}}");
}

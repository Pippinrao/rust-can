//! Allocation counting benchmark for CAN message creation (PERF-ALLOC-001).
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use rust_can_core::message::CanMessage;

struct CountingAlloc;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATED: AtomicUsize = AtomicUsize::new(0);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

struct BenchResult {
    name: &'static str,
    iterations: u64,
    bytes_allocated: usize,
    alloc_count: usize,
}

fn bench(name: &'static str, iterations: u64, mut f: impl FnMut()) -> BenchResult {
    ALLOCATED.store(0, Ordering::Relaxed);
    DEALLOCATED.store(0, Ordering::Relaxed);
    ALLOCS.store(0, Ordering::Relaxed);
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let _elapsed = start.elapsed();
    BenchResult {
        name,
        iterations,
        bytes_allocated: ALLOCATED.load(Ordering::Relaxed),
        alloc_count: ALLOCS.load(Ordering::Relaxed),
    }
}

fn main() {
    let iterations = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse::<u64>().ok())
        .unwrap_or(100_000);

    let classic_payload = [1_u8, 2, 3, 4, 5, 6, 7, 8];
    let fd_payload = [0xAA_u8; 64];

    let results = [
        bench("classic_message_create_8b_alloc", iterations, || {
            black_box(CanMessage::new(0x123, black_box(&classic_payload), false).unwrap());
        }),
        bench("fd_message_create_64b_alloc", iterations, || {
            black_box(
                CanMessage::new_fd(0x18FF_50E5, black_box(&fd_payload), true, true).unwrap(),
            );
        }),
    ];

    println!("{{");
    println!("  \"language\": \"rust\",");
    println!("  \"scenario\": \"PERF-ALLOC-001\",");
    println!("  \"iterations\": {},", iterations);
    println!("  \"results\": [");
    for (idx, result) in results.iter().enumerate() {
        let comma = if idx + 1 == results.len() { "" } else { "," };
        let bytes_per_iter = result.bytes_allocated as f64 / result.iterations as f64;
        let allocs_per_iter = result.alloc_count as f64 / result.iterations as f64;
        println!(
            "    {{\"name\":\"{}\",\"iterations\":{},\"bytes_allocated\":{},\"bytes_per_iter\":{:.3},\"alloc_count\":{},\"allocs_per_iter\":{:.3}}}{}",
            result.name,
            result.iterations,
            result.bytes_allocated,
            bytes_per_iter,
            result.alloc_count,
            allocs_per_iter,
            comma
        );
    }
    println!("  ]");
    println!("}}");
}

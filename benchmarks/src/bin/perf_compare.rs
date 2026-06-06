//! Emit machine-readable microbenchmark results for core message operations.

use std::hint::black_box;
use std::time::Instant;

use rust_can_core::filter::{CanFilter, CanFilters};
use rust_can_core::message::CanMessage;

struct BenchResult {
    name: &'static str,
    iterations: u64,
    total_ns: u128,
}

impl BenchResult {
    fn ns_per_iter(&self) -> f64 {
        self.total_ns as f64 / self.iterations as f64
    }
}

fn bench<F>(name: &'static str, iterations: u64, mut f: F) -> BenchResult
where
    F: FnMut(),
{
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
        .unwrap_or(1_000_000);

    let classic_payload = [1_u8, 2, 3, 4, 5, 6, 7, 8];
    let fd_payload = [0xAA_u8; 64];
    let clone_msg = CanMessage::new(0x123, &classic_payload, false).unwrap();
    let validate_msg = CanMessage::new(0x123, &classic_payload, false).unwrap();
    let filter_msg = CanMessage::new(0x18FF_50E5, &classic_payload, true).unwrap();
    let filters = CanFilters::from_filters(vec![
        CanFilter::new(0x100, 0x700, Some(false)),
        CanFilter::new(0x200, 0x700, Some(false)),
        CanFilter::new(0x18FF_0000, 0x1FFF_0000, Some(true)),
        CanFilter::new(0x7E8, 0x7FF, Some(false)),
    ]);

    let results = [
        bench("classic_message_create_8b", iterations, || {
            black_box(CanMessage::new(0x123, black_box(&classic_payload), false).unwrap());
        }),
        bench("fd_message_create_64b", iterations, || {
            black_box(CanMessage::new_fd(
                0x18FF_50E5,
                black_box(&fd_payload),
                true,
                true,
            )
            .unwrap());
        }),
        bench("message_clone_8b", iterations, || {
            black_box(black_box(&clone_msg).clone());
        }),
        bench("message_validate_8b", iterations, || {
            black_box(&validate_msg).validate().unwrap();
            black_box(());
        }),
        bench("filter_match_4_filters", iterations, || {
            black_box(black_box(&filters).matches(black_box(&filter_msg)));
        }),
    ];

    println!("{{");
    println!("  \"language\": \"rust\",");
    println!("  \"iterations\": {},", iterations);
    println!("  \"results\": [");
    for (idx, result) in results.iter().enumerate() {
        let comma = if idx + 1 == results.len() { "" } else { "," };
        println!(
            "    {{\"name\":\"{}\",\"iterations\":{},\"total_ns\":{},\"ns_per_iter\":{:.3}}}{}",
            result.name,
            result.iterations,
            result.total_ns,
            result.ns_per_iter(),
            comma
        );
    }
    println!("  ]");
    println!("}}");
}

#[cfg(test)]
mod tests {
    use super::{bench, BenchResult};

    #[test]
    fn bench_result_reports_nanoseconds_per_iteration() {
        let result = BenchResult {
            name: "unit",
            iterations: 4,
            total_ns: 20,
        };

        assert_eq!(result.ns_per_iter(), 5.0);
        assert_eq!(result.name, "unit");
    }

    #[test]
    fn bench_invokes_closure_for_each_iteration() {
        let mut calls = 0_u64;
        let result = bench("count", 3, || {
            calls += 1;
        });

        assert_eq!(calls, 3);
        assert_eq!(result.name, "count");
        assert_eq!(result.iterations, 3);
    }
}

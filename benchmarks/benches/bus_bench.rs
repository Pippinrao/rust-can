#![allow(missing_docs)]
//! Criterion benchmarks for bus-adjacent filter matching paths.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rust_can_core::filter::{CanFilter, CanFilters};
use rust_can_core::message::CanMessage;

fn bench_filter_match(c: &mut Criterion) {
    let filters = CanFilters::from_filters(vec![
        CanFilter::new(0x100, 0x700, Some(false)),
        CanFilter::new(0x200, 0x700, Some(false)),
        CanFilter::new(0x18FF_0000, 0x1FFF_0000, Some(true)),
        CanFilter::new(0x7E8, 0x7FF, Some(false)),
    ]);
    let msg = CanMessage::new(0x18FF_50E5, &[0x01, 0x02, 0x03, 0x04], true).unwrap();

    c.bench_function("filter_match_four_rules", |b| {
        b.iter(|| {
            black_box(black_box(&filters).matches(black_box(&msg)));
        });
    });
}

criterion_group!(benches, bench_filter_match);
criterion_main!(benches);

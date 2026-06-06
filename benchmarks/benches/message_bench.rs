#![allow(missing_docs)]
//! Criterion benchmarks for CAN message creation, cloning, and validation.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rust_can_core::message::CanMessage;

fn bench_message_create(c: &mut Criterion) {
    c.bench_function("message_create_can20", |b| {
        b.iter(|| {
            CanMessage::new(0x123, &[0x01, 0x02, 0x03], false).unwrap()
        });
    });

    c.bench_function("message_create_canfd", |b| {
        let data = vec![0xAAu8; 64];
        b.iter(|| {
            CanMessage::new_fd(0x12345678, &data, true, true).unwrap()
        });
    });
}

fn bench_message_clone(c: &mut Criterion) {
    let msg = CanMessage::new(0x123, &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08], false).unwrap();

    c.bench_function("message_clone", |b| {
        b.iter(|| {
            black_box(msg.clone())
        });
    });
}

fn bench_message_validate(c: &mut Criterion) {
    let msg = CanMessage::new(0x123, &[0x01, 0x02, 0x03], false).unwrap();

    c.bench_function("message_validate", |b| {
        b.iter(|| {
            msg.validate().unwrap();
            black_box(());
        });
    });
}

criterion_group!(benches, bench_message_create, bench_message_clone, bench_message_validate);
criterion_main!(benches);

# rust-can Real Log IO Benchmark Report

Date: 2026-06-05

## Scope

This report compares rust-can and patched python-can on the same real-log data:

- ASC: first 100,000 CAN/CANFD messages from `data\extracted\CDC_VHR_LZ4CCAN_PRTO-M183 5.0.0.60M(C00E11R1P4log)_20260529091109(+0800)_20260529091332(+0800)_58143(parsed)\CDC_VHR_LZ4CCAN_1_20260529091108_20260529091332(UTC+8)_58143.asc`
- BLF: `data\generated\real_can_canfd_10000.blf`

LIN parsing is implemented in rust-can ASC IO, but python-can's comparable `Message` stream only covers CAN/CANFD. The ASC throughput comparison therefore counts the first 100,000 CAN/CANFD messages and parses LIN without counting it in the comparable sample.

## Commands

```powershell
cargo run --release -p rust-can-benchmarks --bin real_log_io -- "<ASC path>" "data\generated\real_can_canfd_10000.blf" 100000 5
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo llvm-cov --workspace --all-features --summary-only --fail-under-lines 80
```

Python baseline:

```powershell
python benchmarks\python\real_log_io.py --asc "<ASC path>" --blf "data\generated\real_can_canfd_10000.blf" --asc-limit 100000 --runs 5
```

Baseline JSON: `benchmarks/results/2026-06-05/python-can-real-log-io.json`
Rust JSON: `benchmarks/results/2026-06-05/rust-can-real-log-io.json`

## Results

| Scenario | python-can mean | rust-can mean | Speedup | 20x Status |
| --- | ---: | ---: | ---: | --- |
| ASC real-log CAN/CANFD read | 257,678 msg/s | 2,757,467 msg/s | 10.70x | Exception: below target |
| BLF CAN/CANFD read | 533,956 msg/s | 3,709,850 msg/s | 6.95x | Exception: below target |

The 20x real-log IO target is not met as of this report. The correct status is an explicit performance exception, not a pass.

## Correctness Checks

- ASC comparable sample matched python-can baseline: 14,755 classic CAN and 85,245 CANFD messages.
- BLF fixture matched python-can baseline: 1,486 classic CAN and 8,514 CANFD messages.
- rust-can ASC also parses LIN records as `LogEvent::Lin`; LIN is excluded from this python-can comparable throughput ratio.

## Verification

- `cargo clippy --workspace --all-features --all-targets -- -D warnings`: passed.
- `cargo llvm-cov --workspace --all-features --summary-only --fail-under-lines 80`: passed with 80.59% line coverage.
- `cargo test --workspace --all-features`: passed.

Archived command outputs:

- `benchmarks/results/2026-06-05/cargo-test-workspace.txt`
- `benchmarks/results/2026-06-05/cargo-clippy.txt`
- `benchmarks/results/2026-06-05/cargo-llvm-cov-summary.txt`

## Next Optimization Targets

- Replace `String::lines()` ASC parsing with byte-slice tokenization to reduce UTF-8/string overhead.
- Add borrowed payload event variants or a visitor API that exposes payload slices without `Vec` allocation.
- Reuse scratch buffers across ASC parser calls.
- Stream BLF container objects without building intermediate event vectors.
- Add allocation profiling for ASC/BLF hot paths and fail reports when allocation count regresses.

# Real Log IO Performance Report

Date: 2026-06-06

## Scope

This report compares rust-can and the local patched python-can clone on the same real-log datasets.

- ASC source: `data\extracted\CDC_VHR_LZ4CCAN_PRTO-M183 5.0.0.60M(C00E11R1P4log)_20260529091109(+0800)_20260529091332(+0800)_58143(parsed)\CDC_VHR_LZ4CCAN_1_20260529091108_20260529091332(UTC+8)_58143.asc`
- ASC limit: 100,000 CAN/CANFD messages
- python-can zlib BLF source: `data\generated\real_can_canfd_100000.blf`
- rust-can no-compression BLF source: `data\generated\rust_can_canfd_100000.blf`
- Message mix for the 100k datasets: 85,245 CANFD + 14,755 classic CAN
- Runs per tool: 5

## Commands

```powershell
python benchmarks\python\prepare_real_log_fixtures.py --limit 100000 --lin-limit 1000
cargo run --release -p rust-can-benchmarks --bin prepare_rust_blf -- "<ASC path>" "data\generated\rust_can_canfd_100000.blf" 100000

python benchmarks\python\real_log_io.py --blf data\generated\real_can_canfd_100000.blf --asc-limit 100000 --runs 5
target\release\real_log_io.exe "<ASC path>" data\generated\real_can_canfd_100000.blf 100000 5

python benchmarks\python\real_log_io.py --blf data\generated\rust_can_canfd_100000.blf --asc-limit 100000 --runs 5
target\release\real_log_io.exe "<ASC path>" data\generated\rust_can_canfd_100000.blf 100000 5
```

## Result Files

Final 100k comparison files:

- `python-can-real-log-io-100k-blf.json`
- `rust-can-real-log-io-100k-blf.json`
- `python-can-real-log-io-rust-blf-100k.json`
- `rust-can-real-log-io-rust-blf-100k.json`

Auxiliary 10k comparison files retained for traceability, not used for the final 100k conclusions:

- `python-can-real-log-io.json`
- `rust-can-real-log-io.json`

## Results

| Scenario | python-can mean | rust-can mean | Mean speedup | Status |
| --- | ---: | ---: | ---: | --- |
| ASC read, first 100,000 CAN/CANFD messages | 275,862 msg/s | 6,746,798 msg/s | 24.46x | Meets 20x |
| BLF read, python-can zlib `real_can_canfd_100000.blf` | 580,077 msg/s | 8,616,803 msg/s | 14.85x | Exception: below 20x |
| BLF read, rust-can no-compression `rust_can_canfd_100000.blf` | 599,179 msg/s | 53,886,448 msg/s | 89.93x | Meets 20x |

| Scenario | python-can min | rust-can min | Min speedup | Status |
| --- | ---: | ---: | ---: | --- |
| ASC read, first 100,000 CAN/CANFD messages | 271,771 msg/s | 6,636,008 msg/s | 24.42x | Meets 20x |
| BLF read, python-can zlib `real_can_canfd_100000.blf` | 568,419 msg/s | 5,929,088 msg/s | 10.43x | Exception: below 20x |
| BLF read, rust-can no-compression `rust_can_canfd_100000.blf` | 596,486 msg/s | 46,234,223 msg/s | 77.51x | Meets 20x |

## Interpretation

The ASC fast scan path meets the 20x target on the same real ASC data.

The rust-can no-compression BLF object scan path meets the 20x target and is compatible with python-can after fixing the CAN FD object layout to the python-can BLF structure.

The python-can zlib BLF fixture remains below the 20x target. This is a documented exception for compressed BLF containers and should remain visible until a decompression-path optimization is measured above 20x on the same fixture.

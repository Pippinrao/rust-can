# rust-can

rust-can is an early Rust CAN toolkit workspace. The current focus is ASC/BLF log IO, real-log test corpora, same-data performance comparison, and an extensible hardware adapter injection boundary.

Current status: architecture and early implementation stage. It is not yet a drop-in replacement for python-can; core messages/filters, the virtual adapter, a Notifier prototype, ASC CAN/CANFD/LIN readers/writers, BLF CAN/CANFD readers/writers, and real-log benchmarks are implemented. CLI command families, C FFI, Python bindings, and real hardware backends are still incomplete. Real hardware backend implementation is not a current-stage goal; only the adapter injection API is being designed.

## Project Goals

- Cover python-can core capabilities: message, bus, filters, notifier, listeners, cyclic send tasks, logging, replay, bridge, viewer, and backend discovery.
- Support CAN 2.0, CAN FD, CAN FD non-ISO, and CAN XL.
- Prioritize ASC records for CAN, CANFD, and LIN, with a future-proof `LogEvent` model for additional formats.
- Provide an injectable adapter SPI for virtual/mock and future hardware implementations. SocketCAN, serial/slcan, UDP multicast, gs_usb, socketcand, and vendor SDK backends are later candidates, not current implementation targets.
- Claim 20x+ over python-can only for critical paths proven by same-data, same-machine, same-scenario benchmarks.
- Avoid heap allocation on classic CAN and CAN FD message creation hot paths, and prefer zero-copy or single-copy designs.
- Maintain 80%+ line coverage when the coverage harness is available.

## Current Status

Implemented or partially implemented:

- `CanMessage`: owned message type for classic CAN and CAN FD; CAN XL preserves the full payload and the long-payload truncation bug has been fixed.
- `CanFrame` raw frame type.
- `CanFilter` and `CanFilters`.
- `CanProtocol`, `BusState`, and basic `CanError`.
- Basic `BitTiming` and `BitTimingFd`.
- Initial `CanBus`, `CyclicTask`, and `CanAdapter` traits.
- `AdapterConfig`, `AdapterInfo`, adapter registry, and virtual backend prototype.
- `Listener`, `BufferedReader`, and `Notifier` prototypes.
- `rust-can-io::event` log event model for CAN, CANFD, LIN, metadata/raw, and unknown future records.
- ASC reader/writer for current real-log CAN, CANFD, and LIN records, including streaming limit/visitor APIs and roundtrip tests.
- BLF reader/writer for CAN/CANFD fixture reads and CAN/CANFD roundtrip writes.
- Message/filter microbenchmarks, real ASC/BLF IO benchmarks, and same-data python-can comparison results.

Not implemented or not verified:

- `Bus()` factory, configuration file/environment loading, and backend autodetect.
- Non-`async_trait` bus fast path; bus send/recv has not been proven 20x with same-data benchmarks.
- Full software filter fallback, iteration, context management, and complete cyclic send semantics.
- `ThreadSafeBus`, `RedirectReader`, and `AsyncBufferedReader`.
- Notifier registry, fd/handle reactor, and async callback management; Notifier performance is not measured.
- TRC, CSV, canutils, MF4, and SQLite are deferred compatibility targets.
- CLI commands such as `dump`, `logger`, `player`, `bridge`, `logconvert`, `viewer`, and `detect`.
- C FFI and Python bindings.
- Real hardware backend implementations; the current scope is adapter injection, capability metadata, and mock/virtual conformance tests.
- The 80%+ coverage gate is established. On 2026-06-06, `cargo llvm-cov --workspace --all-features --summary-only --fail-under-lines 80` measured 81.35% line coverage, so the gate is met.
- Complete allocation statistics.

## Workspace Crates

- `rust-can-core`: core types, errors, filters, bus traits, bit timing, listener, and cyclic foundations.
- `rust-can-adapters`: backend adapter interfaces, configuration, registry, and virtual backend.
- `rust-can-io`: log event model, ASC CAN/CANFD/LIN reader/writer, BLF CAN/CANFD reader/writer, and format detection.
- `rust-can-notifier`: multi-bus listener dispatch; currently a prototype without registry, fd/handle reactor, or async callback management.
- `rust-can-cli`: user-facing command-line entry point; command families still need implementation.
- `rust-can-ffi`: target crate for C ABI exports; currently exports version information.
- `benchmarks`: Criterion benchmarks, message/filter comparison, and real ASC/BLF IO comparison harness; bus comparison is incomplete.

See [docs/design/](docs/design/) for design docs and [docs/test/](docs/test/) for test reports. Full architecture: [docs/design/en/overview.md](docs/design/en/overview.md). Real-log IO plan: [docs/design/real-log-io.md](docs/design/real-log-io.md). python-can compatibility matrix: [docs/design/python-can-compatibility.md](docs/design/python-can-compatibility.md). Per-module design: [docs/design/details/](docs/design/details/). Per-module test reports: [docs/test/details/](docs/test/details/).

Git branch workflow is documented in [AGENTS.md](AGENTS.md#git-workflow).

## Real Log Data Status

- The five ZIP files under `data/` have been extracted into `data/extracted/`, producing 10 ASC files and roughly 900 MB of text.
- The real corpus contains 2,384,077 classic CAN records, 11,303,173 CANFD records, and 705,176 LIN-like records.
- The local `.external/python-can` `ASCReader` has been adjusted so it can read the current ASC CANFD dialect.
- `data/generated/real_can_canfd_10000.blf` and `data/generated/real_can_canfd_100000.blf` have been generated from real ASC data, and python-can `BLFReader` reads back the corresponding CAN/CANFD messages.
- `data/generated/rust_can_canfd_100000.blf` has been written by the rust-can BLF writer; python-can verifies 100,000 messages with 85,245 CANFD and 14,755 classic CAN.
- `data/generated/real_lin_1000.jsonl` contains 1,000 parsed LIN event samples.

## Measured Performance Summary

Only the scenarios in this table may be described as proven 20x+ wins. bus, Notifier, CLI, FFI, and hardware adapters do not yet have same-data comparisons, so they must be treated as unverified. For real-log IO, ASC and rust-can no-compression BLF are now proven 20x+; python-can zlib-compressed BLF is still below 20x and remains a documented exception.

Test date: 2026-06-03. Raw results are stored in `benchmarks/results/2026-06-03/`.

Environment summary:

- OS: Microsoft Windows 10.0.26200, X64
- Rust: `rustc 1.96.0 (ac68faa20 2026-05-25)`, `cargo 1.96.0 (30a34c682 2026-05-25)`
- Python: `Python 3.14.3`
- python-can: `491a691fd1faffab1c48956bafd711e7c653db54`
- Iterations: `1000000`

Commands:

```powershell
cargo run -p rust-can-benchmarks --release --bin perf_compare -- 1000000
python benchmarks\python\perf_compare.py 1000000
cargo bench -p rust-can-benchmarks --bench message_bench
python -m pytest benchmarks\python\test_python_can_benchmark.py --benchmark-only --benchmark-json=benchmarks\results\2026-06-03\pytest-benchmark-python.json
cargo llvm-cov --workspace --all-features --summary-only
```

| Scenario | Rust ns/iter | python-can ns/iter | Speedup | Status |
| --- | ---: | ---: | ---: | --- |
| classic 8B message create | 9.876 | 362.038 | 36.7x | Meets 20x |
| CAN FD 64B message create | 10.713 | 400.237 | 37.4x | Meets 20x |
| 8B message clone/copy | 1.115 | 594.139 | 532.9x | Meets 20x |
| 8B message validate | 0.848 | 186.935 | 220.4x | Meets 20x |
| 4-filter match | 1.792 | 212.592 | 118.6x | Meets 20x |

Limits:

- These are message and filter hot-path microbenchmarks, not proof that the whole project is 20x faster.
- Rust Criterion and Python pytest-benchmark were also run; the speedup table comes from the same-data `perf_compare` harness.
- Allocation statistics still need to be added.
- Unmeasured scenarios must be documented as “unverified”, and unimplemented functionality must remain “not implemented”.

Real-log IO comparison:

Test date: 2026-06-06. Raw results are stored in `benchmarks/results/2026-06-06/`.

| Scenario | python-can | rust-can | Speedup | Status |
| --- | ---: | ---: | ---: | --- |
| ASC read, first 100,000 CAN/CANFD messages from a real large ASC | 275,862 msg/s | 6,746,798 msg/s | 24.46x | Meets 20x |
| BLF read, python-can zlib `real_can_canfd_100000.blf` | 580,077 msg/s | 8,616,803 msg/s | 14.85x | Exception: below 20x |
| BLF read, rust-can no-compression `rust_can_canfd_100000.blf` | 599,179 msg/s | 53,886,448 msg/s | 89.93x | Meets 20x |

Measured coverage:

| Metric | Current | Target | Status |
| --- | ---: | ---: | --- |
| Region coverage | 80.83% | 80%+ | Met |
| Function coverage | 80.99% | 80%+ | Met |
| Line coverage | 81.35% | 80%+ | Met |

## Development, Testing, and Benchmarking

Run workspace tests:

```powershell
cargo test --workspace --all-features
```

Run Rust benchmarks:

```powershell
cargo bench --workspace
```

Run the current same-data performance comparison:

```powershell
cargo run -p rust-can-benchmarks --release --bin perf_compare -- 1000000
python benchmarks\python\perf_compare.py 1000000
cargo run --release -p rust-can-benchmarks --bin real_log_io -- "<ASC path>" "data\generated\real_can_canfd_100000.blf" 100000 5
cargo run --release -p rust-can-benchmarks --bin prepare_rust_blf -- "<ASC path>" "data\generated\rust_can_canfd_100000.blf" 100000
cargo run --release -p rust-can-benchmarks --bin real_log_io -- "<ASC path>" "data\generated\rust_can_canfd_100000.blf" 100000 5
```

Run the Python benchmark-tool suite:

```powershell
python -m pytest benchmarks\python\test_python_can_benchmark.py --benchmark-only --benchmark-json=benchmarks\results\2026-06-03\pytest-benchmark-python.json
```

Coverage target:

```powershell
cargo llvm-cov --workspace --all-features --fail-under-lines 80
```

If `cargo llvm-cov` is not installed:

```powershell
cargo install cargo-llvm-cov
```

Performance-related changes must:

- Feed one generated dataset to both Rust and python-can.
- Record Rust and Python output, machine information, and tool versions.
- Save results under `benchmarks/results/YYYY-MM-DD/`.
- Track allocation counts or memory traffic for allocation-sensitive scenarios.
- Mark any target scenario below 20x as an exception instead of changing the claim.

## Upstream python-can Reference

The upstream source used for the architecture analysis and compatibility matrix is [hardbyte/python-can](https://github.com/hardbyte/python-can) at commit `491a691fd1faffab1c48956bafd711e7c653db54`.

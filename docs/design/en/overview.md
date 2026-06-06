# rust-can Architecture Overview (English)

> Chinese version: [../overview.md](../overview.md)

rust-can aims to be a faster, more extensible CAN toolkit than python-can. Current focus: ASC/BLF log IO with real corpora under `data/`, same-data benchmarks vs python-can, and pluggable adapter SPI (no real hardware backends yet).

## Workspace Crates

| Crate | Role |
| --- | --- |
| `rust-can-core` | Messages, frames, filters, bus traits, listeners, bit timing |
| `rust-can-adapters` | `CanAdapter` trait, registry, virtual backend |
| `rust-can-io` | `LogEvent`, ASC/BLF readers/writers |
| `rust-can-notifier` | Multi-bus listener dispatch |
| `rust-can-cli` | CLI entry (commands TBD) |
| `rust-can-ffi` | C ABI (minimal) |
| `benchmarks` | Criterion + python-can comparison harness |

## Performance Targets

- Minimum **20x** vs python-can on proven same-data scenarios; goal **100x**.
- Verified: message/filter microbenchmarks (36–533x), ASC read (24.46x), uncompressed BLF (89.93x).
- Exception: python-can zlib BLF (14.85x). Unverified: bus, notifier, CLI, FFI.

## References

- Real-log IO: [../real-log-io.md](../real-log-io.md)
- Compatibility matrix: [../python-can-compatibility.md](../python-can-compatibility.md)
- Module design: [../details/](../details/)
- Test reports: [../../test/details/](../../test/details/)

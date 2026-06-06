# AGENTS.md

This repository is an early rust-can workspace. Treat it as a performance-sensitive systems library, not as a loose prototype.

## Primary Mission

Build a Rust CAN toolkit that is more complete than python-can and measurably faster on critical paths. The current focus is real-log ASC/BLF IO, especially ASC records for CAN, CANFD, and LIN. The headline performance target is 20x+ over python-can, but only for scenarios proven by same-data benchmarks.

## Ground Rules

- Never claim a performance target is met without fresh benchmark evidence.
- Keep unimplemented functionality explicitly marked as not implemented.
- Prefer zero-copy or single-copy designs on hot paths.
- Avoid heap allocation in classic CAN and CAN FD message creation.
- Keep public APIs easy to use, but provide fast-path APIs for static dispatch and low allocation.
- Do not implement real hardware backends in the current scope unless explicitly requested. Provide adapter injection, capability metadata, and mock/virtual conformance tests.
- Do not hide hardware-specific behavior. Future backend capabilities must be explicit.
- Maintain 80%+ line coverage when the coverage harness is available.
- Use the `feature/bugfix -> dev -> master` Git workflow documented in [docs/GIT_WORKFLOW.md](docs/GIT_WORKFLOW.md).
- Do not put implementation commits directly on `master`; `master` receives reviewed release-ready changes from `dev`.

## Expected Verification

Before calling work complete, run the narrowest command that proves the change:

```powershell
cargo test --workspace --all-features
cargo bench --workspace
cargo llvm-cov --workspace --all-features --fail-under-lines 80
```

Use only the commands relevant to the files changed. If a command cannot be run, state why.

Current measured coverage on 2026-06-06 meets the line target: `cargo llvm-cov --workspace --all-features --summary-only --fail-under-lines 80` passed with 81.35% line coverage.

## Performance Benchmark Requirements

For any performance-related change:

- Compare Rust and python-can with the same generated dataset.
- Record Rust Criterion output and Python benchmark output.
- Include machine information and tool versions.
- Save results under `benchmarks/results/YYYY-MM-DD/`.
- If rust-can is below 20x for a target scenario, mark it as an exception instead of adjusting the claim.
- Track allocation count or memory traffic when the scenario is allocation-sensitive.

Current measured 20x+ claims are limited to message and filter microbenchmarks saved under `benchmarks/results/2026-06-03/`. bus, notifier, CLI, FFI, and hardware adapter paths are unverified.

Real-log IO comparison from 2026-06-06 is saved under `benchmarks/results/2026-06-06/`. rust-can reads the real ASC sample at about 6,746,798 msg/s for the first 100,000 CAN/CANFD messages, which is 24.46x over python-can. rust-can reads the python-can zlib BLF fixture at about 8,616,803 msg/s, which is 14.85x and remains a documented performance exception. rust-can reads its no-compression BLF fixture at about 53,886,448 msg/s, which is 89.93x over python-can on the same messages.

## Architecture References

Read [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) before making architectural changes.
Read [docs/REAL_LOG_IO_ARCHITECTURE.md](docs/REAL_LOG_IO_ARCHITECTURE.md) before changing IO, ASC, BLF, fixture, benchmark, or adapter-scope decisions.

Important current gaps:

- The bus API needs a non-`async_trait` fast path.
- CLI command families, C FFI, and Python bindings need implementation.
- Real hardware backend implementation is out of current scope; only adapter injection and capabilities are in scope.
- Virtual adapter fan-out needs lock and clone reduction.
- ASC/BLF IO is implemented for current CAN/CANFD/LIN ASC needs and CAN/CANFD BLF fixtures. ASC and no-compression BLF meet the 20x real-log IO target; python-can zlib BLF remains below 20x and must be marked as an exception.

## Editing Guidance

- Keep crate boundaries focused:
  - `rust-can-core` for protocol-independent core types and traits.
  - `rust-can-adapters` for adapter injection, capability metadata, registry, and mock/virtual integration.
  - `rust-can-io` for log events, ASC/BLF formats, and streaming codecs.
  - `rust-can-notifier` for dispatch.
  - `rust-can-cli` for user-facing tools.
  - `rust-can-ffi` for C ABI.
- Do not add a new crate unless it removes real coupling or repeated complexity.
- Keep unsafe code isolated and documented at the boundary.
- Prefer small focused tests over broad smoke tests.
- For hardware backends, provide mock or virtual tests and gate real hardware tests behind features or environment variables.

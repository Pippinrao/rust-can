# Python-can Parity With Real IO Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement rust-can non-hardware python-can parity with real ASC/BLF IO, warning-free builds, 80%+ line coverage, and same-data performance reports.

**Architecture:** Keep hardware as an injected adapter SPI. Put log protocol support in `rust-can-io` using a non-exhaustive `LogEvent` model so CAN, CAN FD, LIN, metadata, and future records share one streaming interface without forcing non-CAN data into `CanMessage`.

**Tech Stack:** Rust 2024, cargo test/clippy/llvm-cov, Criterion/manual throughput harness, patched local python-can for real ASC dialect, real data under `data/extracted` and generated fixtures under `data/generated`.

---

## File Map

- `rust-can-io/src/event.rs`: log event types for CAN, CAN FD, LIN, metadata, raw, and unknown records.
- `rust-can-io/src/formats/asc.rs`: ASC streaming parser and writer for CAN/CANFD/LIN.
- `rust-can-io/src/formats/blf.rs`: BLF CAN/CANFD reader and writer for generated real fixtures.
- `rust-can-io/src/reader.rs`: extension-based `LogReader` and `MessageReader` adapters.
- `rust-can-io/src/writer.rs`: extension-based `LogWriter` and `MessageWriter` adapters.
- `rust-can-io/src/player.rs`: replay timing iterator for parsed log events.
- `rust-can-cli/src/main.rs`: real CLI commands for `logconvert`, `inspect`, and benchmark helpers.
- `benchmarks/src/bin/real_log_io.rs`: Rust real-log IO throughput harness.
- `benchmarks/python/real_log_io_benchmark.py`: python-can real-log IO throughput harness.
- Existing core/adapters/notifier files: warning cleanup and focused tests to lift coverage.

## Task 1: ASC Log Event Model

**Files:**
- Create: `rust-can-io/src/event.rs`
- Modify: `rust-can-io/src/lib.rs`
- Test: `rust-can-io/src/event.rs`

- [ ] Write failing unit tests for CAN, CANFD, and LIN event construction, including future-safe `Unknown` records.
- [ ] Run `cargo test -p rust-can-io event::tests` and verify the tests fail because `event` is missing.
- [ ] Implement `Direction`, `Timestamp`, `Payload`, `CanLogEvent`, `CanFdLogEvent`, `LinLogEvent`, `MetadataEvent`, `RawEvent`, `UnknownEvent`, and `LogEvent`.
- [ ] Run `cargo test -p rust-can-io event::tests` and verify the tests pass with no warnings.

## Task 2: ASC Streaming Reader

**Files:**
- Create: `rust-can-io/src/formats/asc.rs`
- Modify: `rust-can-io/src/formats/mod.rs`
- Test: `rust-can-io/src/formats/asc.rs`

- [ ] Write failing parser tests for current real lines:
  - `0.000080 2 1D1 Rx d 8 00 00 00 00 F8 00 82 D9`
  - `0.000000 CANFD 6 637 Rx 0 0 d 10 16 ...`
  - `0.000030 L11 1 Rx 8 ... checksum = 00`
- [ ] Write failing tests for header/comment/trigger handling and malformed-line recovery.
- [ ] Run `cargo test -p rust-can-io asc::tests` and verify failures are caused by missing ASC parser.
- [ ] Implement line-level parser and streaming `AscReader<R: BufRead>`.
- [ ] Run parser tests and real fixture smoke tests against `data/generated/real_lin_1000.jsonl`.

## Task 3: ASC Writer and Roundtrip

**Files:**
- Modify: `rust-can-io/src/formats/asc.rs`
- Test: `rust-can-io/src/formats/asc.rs`

- [ ] Write failing roundtrip tests for CAN, CANFD, LIN, metadata, and unknown raw preservation.
- [ ] Run tests and verify writer is missing.
- [ ] Implement `AscWriter<W: Write>` with stable canonical output for CAN/CANFD/LIN.
- [ ] Run roundtrip tests.

## Task 4: BLF CAN/CANFD Reader and Writer

**Files:**
- Create: `rust-can-io/src/formats/blf.rs`
- Modify: `rust-can-io/src/formats/mod.rs`
- Test: `rust-can-io/src/formats/blf.rs`

- [ ] Write failing tests that read `data/generated/real_can_canfd_10000.blf` and assert 10,000 messages, 8,514 CANFD, 1,486 classic CAN.
- [ ] Write failing tests for a small BLF writer roundtrip with classic CAN and CANFD.
- [ ] Implement BLF container/object parsing for python-can-compatible CAN message, CAN message2, CAN FD message, and CAN FD message 64.
- [ ] Implement BLF writer for CAN/CANFD fixtures.
- [ ] Run BLF tests and smoke-compare with python-can.

## Task 5: Reader/Writer Registry and Player

**Files:**
- Modify: `rust-can-io/src/reader.rs`
- Modify: `rust-can-io/src/writer.rs`
- Modify: `rust-can-io/src/player.rs`
- Test: same files

- [ ] Write failing tests for `.asc` and `.blf` extension routing.
- [ ] Write failing `MessageSync` timing tests with deterministic timestamps.
- [ ] Implement reader/writer routing and replay timing iterator.
- [ ] Run `cargo test -p rust-can-io`.

## Task 6: CLI Non-hardware Commands

**Files:**
- Modify: `rust-can-cli/src/main.rs`
- Test: `rust-can-cli/src/main.rs`

- [ ] Write failing tests for `inspect`, `logconvert`, and `player --dry-run` command parsing and outputs.
- [ ] Implement CLI commands around file IO only.
- [ ] Run `cargo test -p rust-can-cli`.

## Task 7: Warning Cleanup

**Files:**
- Modify: all crates as needed

- [ ] Run `cargo test --workspace --all-features` and capture warnings.
- [x] Remove unused imports and replace unfinished-marker text in code.
- [ ] Add focused public docs where valuable; adjust lint policy only for generated or binary crates when documentation adds noise rather than value.
- [ ] Run `cargo clippy --workspace --all-features -- -D warnings`.

## Task 8: Coverage Lift

**Files:**
- Modify: core/adapters/notifier/ffi/cli/io tests

- [ ] Run `cargo llvm-cov --workspace --all-features --summary-only`.
- [ ] Add failing tests for uncovered protocol, errors, config, registry, bus default behavior, cyclic state, notifier dispatch, FFI version, benchmark helpers, and CLI.
- [ ] Implement minimal fixes only when tests expose behavior gaps.
- [ ] Run `cargo llvm-cov --workspace --all-features --fail-under-lines 80`.

## Task 9: Performance Tooling and Report

**Files:**
- Create: `benchmarks/src/bin/real_log_io.rs`
- Create: `benchmarks/python/real_log_io_benchmark.py`
- Create: `benchmarks/results/2026-06-04/REAL_LOG_IO_REPORT.md`

- [ ] Write failing test or smoke assertion that benchmark output JSON has Rust/Python comparable fields.
- [ ] Implement Rust ASC/BLF throughput harness on real fixtures.
- [ ] Implement Python benchmark harness using patched python-can.
- [ ] Run both tools on the same real data and save JSON.
- [ ] Generate report with throughput, speedup, coverage, warnings, and exceptions. Any below-20x result is marked as an exception.

## Task 10: Independent Reviews

**Files:**
- Review all changed files and generated reports.

- [ ] Dispatch independent spec reviewer agent for python-can parity minus real hardware.
- [ ] Dispatch independent code quality reviewer agent.
- [ ] Dispatch independent test report reviewer agent.
- [ ] Fix every critical or important finding.
- [x] Re-run full verification: tests, clippy, coverage, performance tools, unfinished-marker scan.

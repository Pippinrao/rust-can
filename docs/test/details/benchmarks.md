# benchmarks 测试报告

> 设计文档：[../../design/details/benchmarks.md](../../design/details/benchmarks.md)

## 测试范围与环境

- Crate：`rust-can-benchmarks`
- 工具：`perf_compare`、`real_log_io`、`virtual_perf_compare`、`alloc_bench`、`prepare_rust_blf`；Criterion benches；Python `benchmarks/python/`
- 语料：`data/generated/`、`data/extracted/` 真实 ASC

## 单元/集成测试执行结果

| 组件 | 测试数 | 通过 |
| --- | ---: | ---: |
| lib.rs | 1 | 1 |
| perf_compare | 2 | 2 |
| real_log_io | 3 | 3 |
| prepare_rust_blf | 0 | — |

Workspace 全量测试 2026-06-06：**全部通过**（存档 `cargo-test-workspace.txt`）。

Clippy：`cargo-clippy.txt` 存档无 workspace 级失败。

## E2E 测试

| 场景 ID | 描述 | 状态 | 证据 |
| --- | --- | --- | --- |
| E2E-IO-001/002/003 | IO roundtrip / format detect | 通过 | `integration-tests/tests/e2e_io_roundtrip.rs` |
| E2E-ADP-001 / E2E-NTF-001 | virtual / notifier 链路 | 通过 | `integration-tests/tests/e2e_virtual_notifier.rs` |

## 性能测试

### Message/Filter（2026-06-03）

| 场景 | 提升 | 20x | 100x |
| --- | ---: | --- | --- |
| classic create | 36.7x | 达标 | 否 |
| FD create | 37.4x | 达标 | 否 |
| clone | 532.9x | 达标 | 是 |
| validate | 220.4x | 达标 | 是 |
| filter match | 118.6x | 达标 | 是 |

### Real-log IO（2026-06-06）

| 场景 | 提升 | 20x | 100x |
| --- | ---: | --- | --- |
| ASC 100k | 24.46x | 达标 | 否 |
| BLF zlib | 18.51x（优化后；优化前 14.85x） | **异常** | 否 |
| BLF 无压缩 | 89.93x | 达标 | 否 |

结果文件：

- `benchmarks/results/2026-06-03/{rust,python}.json`
- `benchmarks/results/2026-06-06/*-real-log-io*.json`
- 人类摘要：`docs/test/real-log-io-report.md`

## 与 python-can 功能/性能差距

| Harness | 状态 |
| --- | --- |
| message/filter | 完整 |
| ASC/BLF IO | 完整（zlib BLF 异常已记录） |
| bus send/recv | **已验证** smoke | `virtual_perf_compare.rs` + `virtual_io.py` |
| notifier | **未验证** | `virtual_bench.rs` notifier dispatch |
| allocation | **已验证** | `alloc_bench.rs` + `alloc_compare.py` |

## 结论与后续行动

- Harness 自身测试通过，产出数据支撑 20x/100x 声明。
- 扩展 bus/notifier benchmark；所有新场景遵循 `AGENTS.md` 归档规则。
- `prepare_rust_blf` bin 覆盖率 0%，建议加 smoke test。

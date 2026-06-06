# benchmarks 模块设计

> Test report: [../../test/details/benchmarks.md](../../test/details/benchmarks.md)

## 架构设计

`benchmarks` crate 提供 Criterion bench 与同数据 Rust/python-can 对比 harness。

```
┌─────────────────┐     ┌──────────────────┐
│ perf_compare    │────▶│ rust.json /      │
│ (message/filter)│     │ python.json      │
├─────────────────┤     ├──────────────────┤
│ real_log_io     │────▶│ *-real-log-io*   │
│ (ASC/BLF)       │     │ .json            │
├─────────────────┤     ├──────────────────┤
│ prepare_rust_blf│────▶│ data/generated/  │
└─────────────────┘     └──────────────────┘
         ▲
         │ fixtures
┌────────┴────────┐
│ benchmarks/     │
│ python/*.py     │
└─────────────────┘
```

结果归档至 `benchmarks/results/YYYY-MM-DD/`；人类可读摘要同步至 `docs/test/`。

## 接口设计

| Binary / Bench | 用途 |
| --- | --- |
| `perf_compare` | 1M iter message/filter 对比 |
| `real_log_io` | ASC/BLF 吞吐，参数：asc path, blf path, limit, runs |
| `prepare_rust_blf` | 从 ASC 生成 rust-can 无压缩 BLF |
| `message_bench` (Criterion) | Rust-only microbench |
| `benchmarks/python/perf_compare.py` | python-can 对照 |
| `benchmarks/python/real_log_io.py` | python-can IO 对照 |

## DFX 设计

| 维度 | 设计 |
| --- | --- |
| 可靠性 | 固定 seed/语料；JSON 输出可机器解析 |
| 可维护性 | Rust/Python harness 场景名对齐 |
| 可测试性 | bin 内单元测试验证统计逻辑 |
| 可观测性 | 输出 msg/s、ns/iter、runs 汇总 |
| 可扩展性 | 新场景加 bin + python 脚本 + results 目录 |

## 性能指标

本 crate 是性能证据来源，不自身宣称倍率，而是产出对比数据：

| Harness | 最高实测提升 | 最低实测提升 |
| --- | ---: | ---: |
| perf_compare | 532.9x (clone) | 36.7x (create) |
| real_log_io | 89.93x (BLF 无压缩) | 14.85x (BLF zlib，**异常**) |

bus/notifier 对比 harness：**待实现**。

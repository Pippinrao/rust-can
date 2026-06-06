# rust-can-io 测试报告

> 设计文档：[../../design/details/io.md](../../design/details/io.md)

## 测试范围与环境

- Crate：`rust-can-io`
- 真实语料：`data/extracted/`（10 个 ASC，约 900MB）、`data/generated/`（BLF/JSONL fixture）
- 统计：2,384,077 classic CAN、11,303,173 CANFD、705,176 LIN-like
- 环境：2026-06-06，Windows，Rust 1.96.0，python-can `491a691`

## 单元/集成测试执行结果

| 指标 | 结果 |
| --- | --- |
| io 测试数 | 24 |
| 通过 | 24 |
| 失败 | 0 |

关键用例：

- ASC：真实最小语料、CAN/CANFD/LIN 解析、CANFD 十进制 DLC、channel 映射、roundtrip writer
- ASC：`scan_can_stats` / `collect_*` limit API、坏 payload 检测
- BLF：python-can 生成 `real_can_canfd_100000.blf` 可读、rust-can 无压缩 fixture roundtrip
- reader：扩展名与 BLF 魔数探测

覆盖率：asc 87%、blf 81%、event 92%。

## E2E 测试

| 场景 ID | 描述 | 状态 | 证据 |
| --- | --- | --- | --- |
| E2E-IO-001 | ASC write → read roundtrip（CAN/CANFD/LIN） | 通过 | `integration-tests/tests/e2e_io_roundtrip.rs` |
| E2E-IO-002 | BLF write → read roundtrip（CAN/CANFD） | 通过 | 同上 |
| E2E-IO-003 | format detect（扩展名 + BLF magic） | 通过 | 同上 |

| E2E-IO-004 | 真实语料 BLF/ASC 计数与抽样 | `integration-tests/tests/e2e_real_corpus.rs` | 通过 |
| E2E-IO-005 | `real_log_io --assert-count` | `integration-tests/tests/e2e_real_log_io_assert.rs` | 通过 |

## 性能测试

### 真实 ASC/BLF IO（2026-06-06）

命令：

```powershell
python benchmarks\python\prepare_real_log_fixtures.py --limit 100000 --lin-limit 1000
cargo run --release -p rust-can-benchmarks --bin prepare_rust_blf -- "<ASC>" "data\generated\rust_can_canfd_100000.blf" 100000
target\release\real_log_io.exe "<ASC>" "data\generated\real_can_canfd_100000.blf" 100000 5
python benchmarks\python\real_log_io.py --blf data\generated\real_can_canfd_100000.blf --asc-limit 100000 --runs 5
```

ASC 源：`data/extracted/.../CDC_VHR_LZ4CCAN_1_20260529091108_20260529091332(UTC+8)_58143.asc`

| 场景 | python-can mean | rust-can mean | 提升 | 20x | 100x |
| --- | ---: | ---: | ---: | --- | --- |
| ASC 100k CAN/CANFD | 275,862 msg/s | 6,746,798 msg/s | **24.46x** | 达标 | 否 |
| BLF zlib fixture | 573,005 msg/s | 10,606,543 msg/s | **18.51x** | **异常** | 否 |
| BLF 无压缩 fixture | 599,179 msg/s | 53,886,448 msg/s | **89.93x** | 达标 | 否 |

原始 JSON：`benchmarks/results/2026-06-06/*.json`  
摘要：[../real-log-io-report.md](../real-log-io-report.md)

### Min 吞吐（同次运行）

| 场景 | min 提升 |
| --- | ---: |
| ASC | 24.42x |
| BLF zlib | 10.43x（异常） |
| BLF 无压缩 | 77.51x |

## 与 python-can 功能/性能差距

| 格式/能力 | python-can | rust-can | 性能 |
| --- | --- | --- | --- |
| ASC CAN/CANFD/LIN | 有（ASC dialect 已 patch） | 已实现 | ASC **24.46x** |
| BLF CAN/CANFD | 有 | 已实现 | 无压缩 **89.93x**；zlib **14.85x 异常** |
| TRC/CSV/MF4/SQLite | 有 | 未实现 | — |
| Logger/Player | 有 | 未实现 | — |

### BLF zlib 优化（2026-06-06 补全测试系统）

优化项：`BlfReader` 复用 `decompress_buf` / `body_buf`；`NO_COMPRESSION` 零拷贝切片；zlib 预分配 + `read_exact`；scan 路径走 `scan_container_with_tail` 零 LogEvent 分配。

| 阶段 | python-can mean | rust-can mean | 提升 |
| --- | ---: | ---: | ---: |
| 优化前 | 580,077 msg/s | 8,616,803 msg/s | **14.85x** |
| 优化后 | 573,005 msg/s | 10,606,543 msg/s | **18.51x** |

仍低于 **20x** 门禁，保持 **异常** 状态；解压后端（如 zlib-ng）可作为后续项。

## 结论与后续行动

- ASC 与无压缩 BLF 满足 **20x**；zlib BLF 从 **14.85x → 18.51x**，仍为 documented exception。
- LIN BLF 缺真实样本；Logger rotation 未测。
- 建议在 `data/extracted/` 全量 ASC 上跑回归 scan 基准。

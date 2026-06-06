# rust-can-core 测试报告

> 设计文档：[../../design/details/core.md](../../design/details/core.md)

## 测试范围与环境

- Crate：`rust-can-core`
- 模块：message、frame、filter、listener、bit_timing、error、protocol、**bus（FilteredBus）**、**cyclic（TokioCyclicTask）**
- 环境（2026-06-06）：Windows 10.0.26200，Rust 1.96.0，python-can commit `491a691`
- 数据来源：单元测试内存 fixture；性能数据来自 `benchmarks/results/2026-06-03/`

## 单元/集成测试执行结果

2026-06-06 `cargo test --workspace --all-features`（存档：`benchmarks/results/2026-06-06/cargo-test-workspace.txt`）：

| 指标 | 结果 |
| --- | --- |
| core 测试数 | 55 |
| 通过 | 55 |
| 失败 | 0 |

重点用例：

- message：CAN 2.0/FD/XL validation、flag、remote、error frame、长 XL payload
- filter：mask、extended、OR 语义、空 filter 匹配全部
- listener：BufferedReader 超时、stop 后 drain、PrinterListener
- bus：FilteredBus 过滤跳过/超时/空 filter；send/shutdown 委托
- cyclic：TokioCyclicTask start/stop/modify/Drop、CyclicTask trait

覆盖率（llvm-cov 2026-06-06）：

| 文件 | 行覆盖率 |
| --- | ---: |
| message.rs | 85.42% |
| filter.rs | 76.47% |
| frame.rs | 100% |
| listener.rs | 96.10% |
| bit_timing.rs | 98.63% |
| bus.rs | 81.25% |
| cyclic.rs | 88.82% |

Workspace 行覆盖率 **81.35%**，达标。

## E2E 测试

| 场景 ID | 描述 | 状态 | 证据 |
| --- | --- | --- | --- |
| E2E-NTF-001 | virtual → notifier → BufferedReader | 通过 | `integration-tests/tests/e2e_virtual_notifier.rs` |
| E2E-COR-001 | bus recv 软件过滤兜底 | 通过 | `integration-tests/tests/e2e_filter_fallback.rs` |
| E2E-CYC-001 | TokioCyclicTask 周期发送 | 通过 | `integration-tests/tests/e2e_cyclic_send.rs` |
| E2E-COR-BTM-001 | BitTiming 计算/寄存器 roundtrip | 通过 | `integration-tests/tests/e2e_bit_timing.rs` |
| E2E-COR-PRT-001 | PrinterListener 输出 | 通过 | `integration-tests/tests/e2e_printer_listener.rs` |

### E2E 缺口

- `CanMessage` 独立跨 crate 场景（E2E-COR-MSG-001）仍待建

## 性能测试

命令：

```powershell
cargo run -p rust-can-benchmarks --release --bin perf_compare -- 1000000
python benchmarks\python\perf_compare.py 1000000
```

结果（`benchmarks/results/2026-06-03/rust.json` vs `python.json`）：

| 场景 | Rust ns/iter | python-can ns/iter | 提升 | 20x | 100x |
| --- | ---: | ---: | ---: | --- | --- |
| classic 8B create | 9.876 | 362.038 | 36.7x | 达标 | 否 |
| CAN FD 64B create | 10.713 | 400.237 | 37.4x | 达标 | 否 |
| 8B clone | 1.115 | 594.139 | 532.9x | 达标 | 是 |
| 8B validate | 0.848 | 186.935 | 220.4x | 达标 | 是 |
| 4-filter match | 1.792 | 212.592 | 118.6x | 达标 | 是 |

Criterion：`cargo bench -p rust-can-benchmarks --bench message_bench`（结果同目录 pytest-benchmark 交叉验证）。

## 与 python-can 功能/性能差距

| 能力 | python-can | rust-can | 性能 |
| --- | --- | --- | --- |
| Message 创建/校验/过滤 | 有 | 部分实现 | **20x–533x 已验证** |
| BusABC recv 过滤兜底 | 有 | 未实现 | 未验证 |
| 周期发送 | 有 | 部分实现 | 未验证 |
| ThreadSafeBus | 有 | 未实现 | — |
| tolerant equality | 有 | 未实现 | — |

## 结论与后续行动

- 消息与过滤热路径**全面超过 20x**，clone/validate/filter 接近或超过 **100x**。
- 需补 bus/cyclic 集成测试与软件过滤兜底，并建立 bus 路径 python-can 对照 benchmark。
- allocation 统计尚未纳入报告。

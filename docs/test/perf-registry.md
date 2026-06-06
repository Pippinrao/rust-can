# 性能场景登记表

> 权威要求：[REQUIREMENTS.md](REQUIREMENTS.md) PERF-01～PERF-05  
> 原始数据：`benchmarks/results/YYYY-MM-DD/`

**状态说明**：`已验证` · `异常`（&lt;20x） · `未验证` · `待建`

---

## 微基准（message / filter）

| 场景 ID | 描述 | Harness | python-can 对照 | 提升 | 状态 | 归档 |
| --- | --- | --- | --- | ---: | --- | --- |
| PERF-MSG-001 | classic 8B message create | `perf_compare.rs` | ✓ | 36.7x | 已验证 | `2026-06-03/rust.json` |
| PERF-MSG-002 | CAN FD 64B message create | `perf_compare.rs` | ✓ | 37.4x | 已验证 | `2026-06-03/rust.json` |
| PERF-MSG-003 | 8B message clone | `perf_compare.rs` | ✓ | 532.9x | 已验证 | `2026-06-03/rust.json` |
| PERF-MSG-004 | 8B message validate | `perf_compare.rs` | ✓ | 220.4x | 已验证 | `2026-06-03/rust.json` |
| PERF-FLT-001 | 4-filter match | `perf_compare.rs`, `bus_bench.rs` | ✓ | 118.6x | 已验证 | `2026-06-03/rust.json` |
| PERF-ALLOC-001 | message create 分配次数 | `alloc_bench.rs`, `alloc_compare.py` | ✓ | — | 已验证 | `2026-06-06/rust-alloc-smoke.json` |

---

## 真实日志 IO

| 场景 ID | 描述 | Harness | python-can 对照 | 提升 | 状态 | 归档 |
| --- | --- | --- | --- | ---: | --- | --- |
| PERF-IO-001 | ASC 读取 100k CAN/CANFD | `real_log_io.rs` | ✓ | 24.46x | 已验证 | `2026-06-06/rust-can-real-log-io.json` |
| PERF-IO-002 | BLF 无压缩 100k | `real_log_io.rs` | ✓ | 89.93x | 已验证 | `2026-06-06/rust-can-real-log-io-rust-blf-100k.json` |
| PERF-IO-003 | BLF python-can zlib 100k | `real_log_io.rs` | ✓ | 18.51x（优化后） | **异常** | `2026-06-06/`（优化前 14.85x） |

---

## 总线 / 分发（热路径扩展）

| 场景 ID | 描述 | Harness | python-can 对照 | 状态 | 备注 |
| --- | --- | --- | --- | --- | --- |
| PERF-ADP-001 | virtual bus send/recv 吞吐 | `virtual_bench.rs`, `virtual_perf_compare.rs` | ✓ | 已验证 | `virtual_io.py` 对照 |
| PERF-NTF-001 | notifier 多 listener dispatch | `virtual_bench.rs` | 待建 | 未验证 | python-can 无等价热路径 |
| PERF-CYC-001 | cyclic / send_periodic | E2E-CYC-001 | — | 未验证 | 功能已接线，缺 perf harness |
| PERF-BUS-001 | bus recv 软件过滤兜底 | E2E-COR-001 | — | 未验证 | 功能已实现 |

---

## Criterion benches

| Bench 文件 | 场景 | 状态 |
| --- | --- | --- |
| `message_bench.rs` | message create/clone/validate | 已验证（Rust-only，交叉 perf_compare） |
| `bus_bench.rs` | 4-rule filter match | 已验证 |
| `virtual_bench.rs` | virtual throughput + notifier | 已验证（Rust）；python 对照 smoke |

---

## 覆盖率公式

```
性能场景覆盖率 = 已测且归档场景数 / 本表「已实现场景」行数 × 100%
```

**已实现场景** = 状态非「未验证（功能未实现）」的行。

---

## Python 对照脚本

| 脚本 | 对应 Rust harness | 状态 |
| --- | --- | --- |
| `benchmarks/python/perf_compare.py` | `perf_compare.rs` | 可用 |
| `benchmarks/python/real_log_io.py` | `real_log_io.rs` | 可用 |
| `benchmarks/python/virtual_io.py` | `virtual_perf_compare.rs` | 可用 |
| `benchmarks/python/alloc_compare.py` | `alloc_bench.rs` | 可用 |

---

## 维护流程

1. 新增热路径 → 登记场景 ID + harness 路径。
2. 跑 benchmark → 归档 JSON 至 `benchmarks/results/YYYY-MM-DD/`。
3. 更新 `docs/test/details/benchmarks.md` 与对应模块报告。
4. &lt;20x 标 **异常**，不得改口径；优化后重跑并更新归档日期。

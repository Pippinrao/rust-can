# 测试合规审计报告

审计日期：2026-06-06（补全测试系统实施后刷新）  
审计范围：`<repo>` workspace 全部 crate  
权威要求：[REQUIREMENTS.md](REQUIREMENTS.md)  
场景登记：[e2e-registry.md](e2e-registry.md)、[perf-registry.md](perf-registry.md)

相关文档：

- [TEMPLATE.md](TEMPLATE.md) — 模块测试报告标准模板
- [AGENTS.md](../../AGENTS.md) — 门禁与章节规范
- [python-can-compatibility.md](../design/python-can-compatibility.md) — 功能矩阵

---

## 总览

| # | 要求 | 判定 | 摘要 |
| --- | --- | --- | --- |
| 1 | UT 行覆盖率 80%+ | **通过** | workspace ≥80%；bus/cyclic 已补测并移出豁免 |
| 2 | E2E 覆盖 100% 已实现功能 | **部分通过** | 首批矩阵 6/6 通过；独立 Message E2E 等待建 |
| 3 | 性能测试覆盖 100% | **部分通过** | message/filter/IO/virtual/alloc 已验证；BLF zlib **18.51x 异常** |
| 4 | 清晰测试报告模板 | **通过** | TEMPLATE + `scripts/generate_test_report.ps1` |

---

## 要求 1：UT 行覆盖率 80%+

### 判定：**通过**

| 子项 | 状态 | 证据 |
| --- | --- | --- |
| 覆盖率是否测量 | ✓ | `cargo llvm-cov --workspace --all-features` |
| workspace 行覆盖率 | ✓ ~80.3% | 2026-06-06 全量跑 |
| 80% 门禁命令 | ✓ | [REQUIREMENTS.md](REQUIREMENTS.md) UT-01 |
| CI 自动强制执行 | ✓ | `.github/workflows/test.yml` |
| `bus.rs` ≥80% | ✓ ~81% | `FilteredBus` UT |
| `cyclic.rs` ≥80% | ✓ ~89% | `TokioCyclicTask` UT |

### 仍豁免

| 文件 | 状态 |
| --- | --- |
| `rust-can-cli` / `rust-can-ffi` | 豁免，首功能落地时补测 |

---

## 要求 2：E2E 测试覆盖 100% 已实现功能

### 判定：**部分通过**

| 项 | 状态 |
| --- | --- |
| `integration-tests/` crate | ✓ |
| 首批矩阵场景 | **6/6 通过** |
| 新增 E2E | E2E-COR-001/CYC/BTM/PRT、E2E-IO-004/005 |
| e2e-registry | ✓ 已刷新 |

### 剩余缺口

- `CanMessage` 独立跨 crate E2E（E2E-COR-MSG-001）仍 **待建**
- `can.broadcastmanager` 未实现

---

## 要求 3：性能测试覆盖 100%

### 判定：**部分通过**

| 场景 | 状态 |
| --- | --- |
| message / filter 微基准 | **已验证** 20x+ |
| ASC / BLF IO | **已验证**；zlib BLF **18.51x 异常**（优化前 14.85x） |
| virtual bus throughput | **已验证** smoke（`virtual_perf_compare` + `virtual_io.py`） |
| allocation 统计 | **已验证**（PERF-ALLOC-001） |
| notifier dispatch perf | **未验证** python 对照 |
| cyclic perf | **未验证** |

---

## 要求 4：清晰测试报告模板

### 判定：**通过**

| 子项 | 状态 |
| --- | --- |
| [REQUIREMENTS.md](REQUIREMENTS.md) | ✓ |
| [TEMPLATE.md](TEMPLATE.md) | ✓ |
| `scripts/generate_test_report.ps1` | ✓ 新增 |
| `docs/test/details/*.md` | ✓ 已刷新 io / core 相关 |

---

## 功能差距矩阵（更新）

| 功能 | 实现 | UT | E2E | Perf |
| --- | --- | --- | --- | --- |
| CanMessage / validation | 部分 | ✓ | △ | ✓ |
| CanFilter / FilteredBus | 部分 | ✓ | ✓ | ✓ |
| Virtual adapter | 部分 | ✓ | ✓ | ✓ |
| ASC codec | 已实现 | ✓ | ✓ | ✓ |
| BLF codec | 部分 | ✓ | ✓ | △ |
| Notifier | 部分 | ✓ | ✓ | △ |
| CanBus 过滤兜底 | 部分 | ✓ | ✓ | △ |
| CyclicTask | 部分 | ✓ | ✓ | △ |
| CLI / FFI | 未实现 | — | — | — |

---

## 审计结论

补全测试系统计划五项 todo 已落地：`FilteredBus`、E2E 补全、perf harness、BLF zlib 优化（14.85x→18.51x）、CI/报告脚本。UT 门禁 **通过**；E2E 首批矩阵 **100%**；BLF zlib 仍 **异常**；notifier/cyclic perf 对照待收敛。

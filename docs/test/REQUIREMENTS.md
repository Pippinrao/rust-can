# 测试覆盖要求

> 本文件为 rust-can 测试体系的**单一权威来源**。CI 门禁、AGENTS.md、模块测试报告与 PR checklist 均引用本文规则 ID。

**范围分母**：[python-can-compatibility.md](../design/python-can-compatibility.md) 中状态为 **已实现** 或 **部分实现** 的条目；**硬件后端**、**未实现** CLI/FFI/TRC 等不计入分母。

---

## 1. 单元测试（UT）要求

| 规则 ID | 要求 | 度量方式 |
| --- | --- | --- |
| UT-01 | workspace 行覆盖率 ≥ **75%** | `cargo llvm-cov --workspace --all-features --fail-under-lines 75` |
| UT-02 | 含业务逻辑的 crate（core/adapters/io/notifier）行覆盖率 ≥ **75%** | `cargo llvm-cov -p <crate> --summary-only` |
| UT-03 | 新增/修改代码不得降低 workspace 覆盖率 | CI 或 merge 前全量跑 |
| UT-04 | 仅 trait 定义/未接线代码须标注豁免或 2 周内补测计划 | 见下方豁免表 |
| UT-05 | 每个 public API 至少 1 个正向 + 1 个错误/边界用例 | code review checklist |

### UT 豁免 / 补测表

| 模块 | 当前状态 | 处理 |
| --- | --- | --- |
| `rust-can-core/src/filter.rs` | 曾 76.47%，须 ≥80% | **补测**（P1） |
| `rust-can-core/src/bus.rs` | 0%（trait 未接线） | 实现 BusHandle 后 **2 周内补测** |
| `rust-can-core/src/cyclic.rs` | 0%（周期发送未接线） | 实现 task registry 后 **2 周内补测** |
| `rust-can-cli` | 0%（入口 stub） | 首个子命令落地时同步 UT |
| `rust-can-ffi` | 0%（仅版本导出） | 首个 C API 落地时同步 UT |
| `benchmarks` bin 工具 | 部分分支未覆盖 | 非门禁阻塞；建议 smoke test |

---

## 2. E2E 测试要求

**定义**：跨 crate、用户可见工作流或 CLI/FFI 边界；单文件 `#[test]` 不计入 E2E。

| 规则 ID | 要求 |
| --- | --- |
| E2E-01 | compatibility 矩阵中 **已实现/部分实现** 功能，每条至少 1 个 E2E 场景 |
| E2E-02 | 场景 ID 格式 `E2E-<domain>-<nnn>`，登记于 [e2e-registry.md](e2e-registry.md) |
| E2E-03 | 集成测试位于 workspace `integration-tests/` 或 crate 级 `tests/integration_*.rs` |
| E2E-04 | 真实语料 E2E 须断言**功能正确性**（计数、抽样 payload、roundtrip），不仅测吞吐 |

### 覆盖率公式

```
E2E 覆盖率 = 已注册且通过的 E2E 场景数 / compatibility 矩阵（已实现+部分实现，硬件除外）条目数 × 100%
```

**目标**：**100%**（未实现 bus 兜底等场景标 **未实现**，不计入通过分母时需注明）。

### 首批必注册场景

| 场景 ID | 链路 | 涉及 crate |
| --- | --- | --- |
| E2E-IO-001 | ASC write → read roundtrip（CAN/CANFD/LIN） | io |
| E2E-IO-002 | BLF write → read roundtrip（CAN/CANFD） | io |
| E2E-IO-003 | format detect（扩展名 + BLF magic） | io/reader |
| E2E-ADP-001 | virtual send → recv 多接收者 | adapters |
| E2E-NTF-001 | virtual bus → notifier → BufferedReader | adapters + notifier + core |
| E2E-COR-001 | filter 在 bus recv 路径兜底 | core + adapters（**未实现**） |

---

## 3. 性能测试要求

| 规则 ID | 要求 |
| --- | --- |
| PERF-01 | 已实现热路径须有 python-can **同数据**对照（硬件除外） |
| PERF-02 | 声明 20x+ 须有 `benchmarks/results/YYYY-MM-DD/` 原始 JSON + 模块报告更新 |
| PERF-03 | 低于 20x 标 **异常**，不得调整口径 |
| PERF-04 | allocation-sensitive 路径须记录分配次数（dhat 或 Criterion custom measurement） |
| PERF-05 | 场景登记于 [perf-registry.md](perf-registry.md)，与 `benchmarks/` harness 一一对应 |

### 覆盖率公式

```
性能场景覆盖率 = 已测且归档场景数 / perf-registry 中已实现场景数 × 100%
```

**目标**：**100%**（异常场景保持 **异常** 标注，仍计为「已测」）。

---

## 4. 测试报告要求

| 规则 ID | 要求 |
| --- | --- |
| RPT-01 | 模块报告使用 [TEMPLATE.md](TEMPLATE.md)，含 **E2E 独立章节** |
| RPT-02 | 每次 benchmark/重大测试变更：更新 `docs/test/details/<module>.md` + 归档原始数据 |
| RPT-03 | 每季度或 release 前更新 [AUDIT.md](AUDIT.md) 合规矩阵 |
| RPT-04 | PR 模板 checklist 引用 REQUIREMENTS 相关规则 ID |

### 报告生成流程

1. 复制 [TEMPLATE.md](TEMPLATE.md) → `docs/test/details/<module>.md`
2. 运行 UT / E2E / perf 命令，填结果表
3. 引用 `benchmarks/results/YYYY-MM-DD/` 文件名
4. 更新 [AUDIT.md](AUDIT.md) 矩阵中该模块行
5. 同步 [e2e-registry.md](e2e-registry.md) / [perf-registry.md](perf-registry.md) 状态

---

## 5. Merge 前 Checklist（摘要）

- [ ] UT-01：`cargo test --workspace --all-features`
- [ ] UT-01：`cargo llvm-cov --workspace --all-features --fail-under-lines 75`
- [ ] 相关 E2E 场景在 registry 中标记 **通过**
- [ ] 性能变更已归档并更新 perf-registry
- [ ] 模块测试报告已按 RPT-01 更新

---

## 6. 相关文档

| 文档 | 说明 |
| --- | --- |
| [e2e-registry.md](e2e-registry.md) | E2E 场景登记表 |
| [perf-registry.md](perf-registry.md) | 性能场景登记表 |
| [TEMPLATE.md](TEMPLATE.md) | 模块测试报告模板 |
| [AUDIT.md](AUDIT.md) | 合规审计 |
| [python-can-compatibility.md](../design/python-can-compatibility.md) | 功能矩阵（覆盖率分母） |

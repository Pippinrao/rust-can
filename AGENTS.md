# AGENTS.md

> 面向 AI 代理与自动化工具。人类读者请从 [README.md](README.md) 进入。

本仓库是性能敏感的 CAN 系统库 workspace，不是松散原型。目标是构建比 python-can 更完整、关键路径显著更快的 Rust CAN 工具链。

## 主要使命

- 功能对标 python-can 核心 API、日志格式与 CLI（**硬件后端映射除外**，当前只保留注入接口与能力声明）。
- 关键路径性能：同数据、同机器、同场景下相对 python-can **至少 20x**，目标 **100x**。
- 当前重点：真实 ASC/BLF 日志 IO（CAN、CANFD、LIN）。

## 文档体系

| 路径 | 受众 | 语言 |
| --- | --- | --- |
| `README.md` / `README.zh-CN.md` / `README.en.md` | 人类开发者 | 中文默认，英文链接 |
| `AGENTS.md` | AI 代理 | 中文 |
| `docs/design/` | 架构与设计 | 中文默认，`docs/design/en/` 为英文 |
| `docs/design/details/` | 模块级设计 | 中文 |
| `docs/test/` | 测试与性能报告 | 中文 |
| `docs/test/details/` | 模块级测试报告 | 中文 |
| `benchmarks/results/YYYY-MM-DD/` | 原始 benchmark 数据 | 英文文件名可接受 |

**禁止**在 `docs/` 外新增独立设计/测试文档（README 与 AGENTS 除外）。Git 工作流**不得**单独成文。

## 文档书写规范（严格管控）

后续所有文档变更必须遵守：

1. **语言**：正文默认简体中文；需要英文时在文首提供 `[English](docs/design/en/...)` 链接，或在 `docs/design/en/` / `docs/test/en/` 维护对应版本。
2. **位置**：
   - 架构总览 → `docs/design/overview.md`
   - 模块设计 → `docs/design/details/<module>.md`（`<module>` 与 workspace crate 名对应，如 `core`、`adapters`、`io`）
   - 测试报告 → `docs/test/details/<module>.md`（结构见 [docs/test/TEMPLATE.md](docs/test/TEMPLATE.md)）
   - 原始 benchmark JSON/txt → `benchmarks/results/YYYY-MM-DD/`（在测试报告中引用，不内联大段原始数据）
3. **模块设计文档必含章节**（按此顺序，标题可微调但不可省略）：
   - 架构设计
   - 接口设计
   - DFX 设计（可靠性、可维护性、可测试性、可观测性、可扩展性）
   - 性能指标（含与 python-can 对比、20x/100x 达标状态）
4. **模块测试报告必含章节**（与 [docs/test/TEMPLATE.md](docs/test/TEMPLATE.md) 对齐）：
   - 测试范围与环境
   - 单元/集成测试执行结果
   - **E2E 测试**（跨 crate / CLI / FFI 边界；单文件 UT 不计入）
   - 性能测试（命令、数据集、结果表）
   - 与 python-can 功能/性能差距
   - 结论与后续行动
5. **性能声明**：无同数据 benchmark 证据不得写“已达标”；低于 20x 必须标为**异常**；未实测标为**未验证**。
6. **功能声明**：未实现必须写“未实现”或“部分实现”，不得暗示已完成。
7. **链接**：移动文档后必须更新所有引用；旧路径用一行重定向 stub，不保留完整副本。
8. **变更同步**：改 API 或 benchmark 时，同步更新对应 `docs/design/details/` 与 `docs/test/details/`。

## Git 工作流

采用 `feature/*` 或 `bugfix/*` → `dev` → `master`。从 `dev` 切分支开发，合并前跑测试与文档更新，仅 release-ready 时 `dev` 合入 `master`。不提交 build 产物、外部 checkout、解压日志语料或生成 fixture。

## 基本规则

- 无 benchmark 证据不得宣称性能达标。
- 未实现功能必须明确标注。
- 热路径优先零拷贝/单拷贝；classic CAN 与 CAN FD 消息创建避免堆分配。
- 当前不实现真实硬件后端，除非明确要求；提供 adapter 注入、能力 metadata、mock/virtual 一致性测试。
- 行覆盖率目标 80%+（coverage harness 可用时）。
- **E2E 覆盖率**目标 **100%**（[python-can-compatibility.md](docs/design/python-can-compatibility.md) 已实现/部分实现条目，硬件除外）；场景登记 [docs/test/e2e-registry.md](docs/test/e2e-registry.md)。
- **性能场景覆盖率**目标 **100%**（已实现热路径）；场景登记 [docs/test/perf-registry.md](docs/test/perf-registry.md)。

## 测试权威文档

正式四维度要求（UT / E2E / Perf / 报告）见 **[docs/test/REQUIREMENTS.md](docs/test/REQUIREMENTS.md)**。规则 ID（UT-01、E2E-01、PERF-01、RPT-01 等）用于 CI 与 PR checklist。

## 验证命令

完成改动前运行最窄证明命令：

```powershell
cargo test --workspace --all-features
cargo bench --workspace
cargo llvm-cov --workspace --all-features --fail-under-lines 80
```

仅运行与变更相关的命令；无法运行须说明原因。

### Merge 前 checklist

- [ ] UT-01：`cargo test --workspace --all-features`
- [ ] UT-01：`cargo llvm-cov --workspace --all-features --fail-under-lines 80`
- [ ] 相关 E2E 场景已跑且 [e2e-registry.md](docs/test/e2e-registry.md) 状态正确（E2E-01）
- [ ] [e2e-registry.md](docs/test/e2e-registry.md) / [perf-registry.md](docs/test/perf-registry.md) 状态已更新（registry 同步）
- [ ] 性能变更已归档并更新 [perf-registry.md](docs/test/perf-registry.md)（PERF-02）
- [ ] 模块测试报告已更新（RPT-01）

2026-06-06 实测行覆盖率 81.35%，已通过 80% 门禁。

## 性能 benchmark 要求

- Rust 与 python-can 使用同一份生成/真实数据。
- 记录工具版本、机器信息；结果存 `benchmarks/results/YYYY-MM-DD/`。
- allocation-sensitive 场景记录分配次数或内存流量。

**已验证 20x+ 场景**（2026-06-03 微基准，2026-06-06 真实日志 IO）：

| 场景 | 提升 | 状态 |
| --- | ---: | --- |
| classic 8B message create | 36.7x | 达标 |
| CAN FD 64B message create | 37.4x | 达标 |
| message clone / validate / filter match | 118–533x | 达标 |
| ASC 读取 100k CAN/CANFD | 24.46x | 达标 |
| BLF 无压缩 fixture | 89.93x | 达标 |
| BLF python-can zlib | 18.51x（优化后；优化前 14.85x） | **异常** |

bus、notifier dispatch perf、CLI、FFI、hardware adapter 路径**未验证**或**部分验证**。

## 架构参考

变更前阅读：

- [docs/design/overview.md](docs/design/overview.md) — 总体架构
- [docs/design/real-log-io.md](docs/design/real-log-io.md) — IO、ASC、BLF、fixture
- [docs/design/python-can-compatibility.md](docs/design/python-can-compatibility.md) — 功能矩阵
- [docs/design/details/](docs/design/details/) — 模块设计
- [docs/test/details/](docs/test/details/) — 模块测试报告

当前主要缺口：bus 非 async fast path、CLI 命令族、C FFI、Python bindings、完整周期发送与 notifier reactor。

## 编辑指引

Crate 边界：

- `rust-can-core` — 协议无关核心类型与 trait
- `rust-can-adapters` — adapter 注入、registry、virtual
- `rust-can-io` — LogEvent、ASC/BLF、streaming codec
- `rust-can-notifier` — 多 bus listener 分发
- `rust-can-cli` — 用户 CLI
- `rust-can-ffi` — C ABI
- `benchmarks` — Criterion 与同数据对比 harness

不随意新增 crate；unsafe 隔离在边界并文档化；硬件后端用 mock/virtual 测试，真实硬件测试挂 feature 或环境变量。

# 模块测试报告模板

> 复制本模板到 `docs/test/details/<module>.md`（`<module>` 与 workspace crate 名对应，如 `core`、`adapters`、`io`）。  
> 设计文档链接：`docs/design/details/<module>.md`  
> 规范来源：[AGENTS.md](../../AGENTS.md) 第 4 节「模块测试报告必含章节」

---

# rust-can-<module> 测试报告

> 设计文档：[../../design/details/<module>.md](../../design/details/<module>.md)

## 测试范围与环境

| 项 | 内容 |
| --- | --- |
| Crate | `rust-can-<module>` |
| 覆盖模块 | （列出本报告涉及的源码模块/子系统） |
| 测试日期 | YYYY-MM-DD |
| 操作系统 | （如 Windows 10.0.26200 / Linux） |
| Rust 工具链 | （如 `rustc 1.xx`、`cargo-llvm-cov 0.x.x`） |
| python-can 基线 | commit `491a691`（或注明不适用） |
| 数据集 | （内存 fixture / `data/generated/` / `data/extracted/` 路径） |
| 原始数据归档 | `benchmarks/results/YYYY-MM-DD/`（列出引用的 txt/json 文件名） |

**范围外说明**（可选）：硬件后端、未实现 CLI 子命令等。

## 单元/集成测试

### 执行命令

```powershell
cargo test -p rust-can-<module> --all-features
# 或 workspace 全量：
cargo test --workspace --all-features
```

### 结果摘要

| 指标 | 结果 |
| --- | --- |
| 测试数 | |
| 通过 | |
| 失败 | |
| 忽略 | |

### 重点用例

- （按功能点列举已覆盖的测试场景，含边界与错误路径）

### 覆盖率（`cargo llvm-cov`）

```powershell
cargo llvm-cov -p rust-can-<module> --all-features --summary-only
```

| 文件/模块 | 行覆盖率 | 备注 |
| --- | ---: | --- |
| | | 未测 / 低于 80% 须标注 |

Workspace 行覆盖率门禁：`--fail-under-lines 80`（**达标 / 未达标 / 未验证**）。

## E2E 测试

> E2E 指跨 crate、跨子系统或 CLI/FFI 边界的端到端场景；单文件单元测试不计入本节。

### 执行命令

```powershell
# 示例：真实日志 IO harness
cargo run --release -p rust-can-benchmarks --bin real_log_io -- <args>
# 示例：未来 CLI integration test
cargo test -p rust-can-cli --test e2e_*
```

### 场景与结果

| 场景 ID | 描述 | 状态 | 证据 |
| --- | --- | --- | --- |
| E2E-001 | | 通过 / 失败 / 未实现 | 测试名或归档路径 |

### E2E 缺口

- （未覆盖的已实现功能点）

## 性能测试

### 命令与数据集

```powershell
# 微基准
cargo run -p rust-can-benchmarks --release --bin perf_compare -- 1000000
python benchmarks\python\perf_compare.py 1000000

# Criterion
cargo bench -p rust-can-benchmarks --bench message_bench

# 真实日志 IO（如适用）
cargo run --release -p rust-can-benchmarks --bin real_log_io -- <asc> <blf> <limit> <runs>
```

| 数据集 | 路径 | 说明 |
| --- | --- | --- |
| | | |

### 结果表

| 场景 | Rust | python-can | 提升 | 20x | 100x | 状态 |
| --- | ---: | ---: | ---: | --- | --- | --- |
| | | | | 达标/否 | 达标/否 | 达标 / **异常** / **未验证** |

原始 JSON：`benchmarks/results/YYYY-MM-DD/<file>.json`

### 性能缺口

- （已实现但未建立同数据对照 benchmark 的模块/场景）

## 与 python-can 差距

> 对照 [python-can-compatibility.md](../design/python-can-compatibility.md)；硬件后端映射除外。

| 功能/API | python-can | rust-can 实现状态 | UT | E2E | 性能 |
| --- | --- | --- | --- | --- | --- |
| | 有/部分/无 | 已实现 / 部分实现 / 未实现 | ✓/△/✗ | ✓/△/✗ | 已验证 / 异常 / 未验证 |

图例：✓ 充分覆盖，△ 部分覆盖，✗ 未覆盖。

## 结论与后续行动

### 结论

- （本模块测试/性能总体判定：通过 / 部分通过 / 未通过）

### 后续行动（按优先级）

1. [ ] P0：（阻塞发布或门禁项）
2. [ ] P1：（覆盖率或 E2E 缺口）
3. [ ] P2：（性能对照扩展）

### 变更记录

| 日期 | 变更 |
| --- | --- |
| YYYY-MM-DD | 初版 |

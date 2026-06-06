# rust-can-notifier 测试报告

> 设计文档：[../../design/details/notifier.md](../../design/details/notifier.md)

## 测试范围与环境

- Crate：`rust-can-notifier`
- 依赖：`rust_can_core::Listener`，tokio 异步运行时

## 单元/集成测试执行结果

| 指标 | 结果 |
| --- | --- |
| 测试数 | 3 |
| 通过 | 3 |
| 失败 | 0 |

场景：listener 增删、消息与错误 dispatch。

覆盖率：91.52% 行覆盖（2026-06-06 llvm-cov）。

## E2E 测试

| 场景 ID | 描述 | 状态 | 证据 |
| --- | --- | --- | --- |
| E2E-NTF-001 | virtual bus → notifier → BufferedReader | 通过 | `integration-tests/tests/e2e_virtual_notifier.rs` |

### E2E 缺口

- 多 bus / fd reactor（功能未实现）

## 性能测试

**未执行**。无 python-can Notifier 多 bus 多 listener 对照 benchmark。

## 与 python-can 功能/性能差距

| 能力 | python-can | rust-can |
| --- | --- | --- |
| 基础 dispatch | 有 | 部分实现 |
| fd reactor | 有 | 未实现 |
| bus registry | 有 | 未实现 |
| 异常保存 | 有 | 未实现 |

性能：**未验证**。

## 结论与后续行动

- 原型功能测试通过，覆盖率良好。
- 实现 reactor + registry 后补充吞吐/latency 与 python-can 对比。

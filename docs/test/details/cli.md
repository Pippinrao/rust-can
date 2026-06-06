# rust-can-cli 测试报告

> 设计文档：[../../design/details/cli.md](../../design/details/cli.md)

## 测试范围与环境

- Crate：`rust-can-cli`（`cancli` binary）
- 当前仅入口，无子命令

## 单元/集成测试执行结果

| 指标 | 结果 |
| --- | --- |
| 测试数 | 0 |
| 通过 | — |

覆盖率：**0%**（`main.rs` 未测）。

## E2E 测试

未适用（CLI 子命令未实现）。首个子命令落地时登记 e2e-registry。

## 性能测试

未适用。

## 与 python-can 功能/性能差距

python-can CLI 全套（logger、player、bridge、viewer、logconvert、detect）均为 **未实现**。

## 结论与后续行动

- 实现首个子命令（建议 `player` 或 `logconvert`）时同步添加 integration test 与文档更新。

# rust-can-adapters 测试报告

> 设计文档：[../../design/details/adapters.md](../../design/details/adapters.md)

## 测试范围与环境

- Crate：`rust-can-adapters`
- 模块：adapter trait、config、registry、backends::virtual
- 环境：2026-06-06 workspace 测试存档

## 单元/集成测试执行结果

| 指标 | 结果 |
| --- | --- |
| 测试数 | 12 |
| 通过 | 12 |
| 失败 | 0 |

覆盖场景：

- `AdapterConfig` JSON 与 builder roundtrip
- `ADAPTER_REGISTRY` 注册/查找
- virtual：单/多接收者、own messages、timestamp 保留、有界队列满、超时、close 错误
- adapter 默认方法返回 NotSupported 而非 panic

覆盖率：virtual 92%、registry 100%、config 97%。

## E2E 测试

| 场景 ID | 描述 | 状态 | 证据 |
| --- | --- | --- | --- |
| E2E-ADP-001 | virtual send → recv 多接收者 | 通过 | `integration-tests/tests/e2e_virtual_notifier.rs` |
| E2E-NTF-001 | virtual bus → notifier 链路 | 通过 | 同上 |

### E2E 缺口

- factory / autodetect 跨 crate 场景（功能未实现）

## 性能测试

**未执行** virtual bus 与 python-can `can.interfaces.virtual` 的同数据吞吐对比。

当前阶段性能重点在 IO 与 message 微路径，adapter 性能标为 **未验证**。

## 与 python-can 功能/性能差距

| 能力 | python-can | rust-can |
| --- | --- | --- |
| virtual 收发 | 有 | 已实现 |
| 20+ 硬件后端 | 有 | **未实现**（范围外） |
| Bus() 工厂 | 有 | 未实现 |
| autodetect | 有 | 未实现 |
| entry points 插件 | 有 | 未实现 |

## 结论与后续行动

- 功能测试通过，virtual 行为与基础契约一致。
- 需实现 factory + detect，并为 virtual 建立 latency/吞吐 benchmark。
- 优化 virtual fan-out 锁与 clone 路径。

# rust-can-notifier 模块设计

> Test report: [../../test/details/notifier.md](../../test/details/notifier.md)

## 架构设计

`rust-can-notifier` 对应 python-can `can.notifier`：多 bus、多 listener 的消息与错误分发。

```
┌─────────┐   recv    ┌───────────┐  dispatch  ┌──────────┐
│ Bus(es) │──────────▶│ Notifier  │───────────▶│ Listener │
└─────────┘           │ (tokio)   │            └──────────┘
                      └───────────┘
```

当前为 **原型**：基于 tokio 的异步 dispatch，listener 增删与消息/错误回调可用。

**未实现**：bus registry、fd/handle reactor（`fileno` 集成）、async callback 异常保存、与 python-can 线程/event-loop 双模式对齐。

## 接口设计

| API | 说明 | 状态 |
| --- | --- | --- |
| `Notifier::new` | 创建分发器 | 已实现 |
| `add_listener` / `remove_listener` | listener 管理 | 已实现 |
| `run` / dispatch loop | 从 bus 读并分发 | 部分实现 |
| fd reactor | 多 bus 单 reactor | 未实现 |

依赖 `rust_can_core::Listener` trait 的 `on_message` / `on_error`。

## DFX 设计

| 维度 | 设计 |
| --- | --- |
| 可靠性 | listener 错误不应 silent drop（待补齐异常保存） |
| 可维护性 | 单 crate、依赖 core listener |
| 可测试性 | 3 个单元测试覆盖增删与 dispatch |
| 可观测性 | 无内置 metrics |
| 可扩展性 | 计划 registry + reactor 插件点 |

## 性能指标

**未验证**：无 python-can Notifier 同场景 throughput/latency 对照。

覆盖率 91%（2026-06-06），但性能目标 20x/100x **未验证**。

功能对标：基础 dispatch **部分实现**；registry、fd reactor、async listener **未实现**。

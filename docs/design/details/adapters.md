# rust-can-adapters 模块设计

> Test report: [../../test/details/adapters.md](../../test/details/adapters.md)

## 架构设计

`rust-can-adapters` 实现 CAN 硬件/虚拟后端的注入 SPI，对应 python-can 的 `can.interface`、`can.interfaces.*` 与 `_entry_points` 插件模型。

```
┌──────────────┐     ┌─────────────────┐     ┌──────────────┐
│ AdapterConfig│────▶│  CanAdapter     │────▶│ CanFrame I/O │
└──────────────┘     │  (7 methods)    │     └──────────────┘
                     └────────┬────────┘
                              │
                     ┌────────▼────────┐
                     │ ADAPTER_REGISTRY│
                     │ virtual backend │
                     └─────────────────┘
```

当前仅实现 **virtual** 后端；真实 SocketCAN、Vector、Kvaser 等**不在当前范围**（与用户要求一致：硬件映射除外）。

## 接口设计

| 类型 | 职责 |
| --- | --- |
| `CanAdapter` | `open`, `read_frame`, `write_frame`, `info`, `close` + 可选 capability hooks |
| `AdapterConfig` | 键值配置、JSON 反序列化、typed accessor |
| `AdapterInfo` | 名称、channel、capabilities bitmask |
| `ADAPTER_REGISTRY` | 编译期/运行时 adapter 注册与查找 |

`backends::virtual`：

- 内存队列 fan-out，支持多接收者、`receive_own_messages`、`preserve_timestamps`
- 有界队列满时返回 delivery failure

**未实现**：`Bus()` 工厂、`detect_available_configs`、动态 dlopen 插件、ThreadSafeBus wrapper。

## DFX 设计

| 维度 | 设计 |
| --- | --- |
| 可靠性 | 默认 trait 方法返回 `NotSupported` 而非 panic；virtual 超时/close 错误可测 |
| 可维护性 | 最小 7 方法 surface；vendor 逻辑隔离在独立 backend crate（未来） |
| 可测试性 | 12 个单元测试覆盖 registry、config、virtual 收发与边界 |
| 可观测性 | `AdapterInfo` 暴露 capabilities；无内置 metrics |
| 可扩展性 | registry + feature flag / dlopen 双路径规划 |

**缺口**：virtual fan-out 锁与 clone 开销待优化；无真实硬件 conformance suite。

## 性能指标

virtual adapter send/recv：**未建立**与 python-can `virtual` 的同数据 benchmark。

功能对标：virtual 基本收发已实现；`VALID_INTERFACES` 规模、autodetect、配置合并 **未实现**。

覆盖率（2026-06-06）：adapter 92%、virtual 92%、registry 100%、config 97%。

性能目标 20x/100x：**未验证**（非当前 IO 热路径重点）。

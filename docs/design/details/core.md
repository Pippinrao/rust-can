# rust-can-core 模块设计

> English summary: see [../en/overview.md](../en/overview.md). Test report: [../../test/details/core.md](../../test/details/core.md)

## 架构设计

`rust-can-core` 是协议无关的核心层，对应 python-can 的 `can.message`、`can.bus`、`can.filter`、`can.listener`、`can.bit_timing`、`can.broadcastmanager`（周期发送雏形）与 `can.exceptions`。

```
┌─────────────────────────────────────────┐
│           Application / CLI             │
├─────────────────────────────────────────┤
│  message │ frame │ filter │ protocol    │
│  bus (CanBus) │ listener │ cyclic       │
│  bit_timing │ error                     │
└─────────────────────────────────────────┘
```

设计原则：

- `CanMessage` 使用内联 `[u8; 64]` 存储 payload，classic CAN / CAN FD 热路径无堆分配。
- `CanFrame` 作为线与后端边界表示；`CanMessage` 为高层语义。
- `CanBus` 为 async trait，计划拆出同步 fast path（当前未实现）。
- 过滤、校验、listener 与 python-can 语义对齐为目标，当前多处为部分实现。

## 接口设计

| 类型 / Trait | python-can 对应 | 状态 |
| --- | --- | --- |
| `CanMessage` | `Message` | 部分实现：缺 tolerant equality、timestamp policy |
| `CanFrame` | 内部 frame | 已实现 |
| `CanFilter` / `CanFilters` | filter API | 部分实现 |
| `CanBus` | `BusABC` | 部分实现：trait 存在，wrapper 与 fast path 缺失 |
| `CyclicTask` | `CyclicSendTaskABC` | 部分实现 |
| `Listener` / `BufferedReader` | listener 模块 | 部分实现 |
| `BitTiming` / `BitTimingFd` | `bit_timing` | 部分实现 |
| `CanError` | exceptions | 部分实现 |

关键公开 API（`rust_can_core::`）：

- `message::CanMessage` — `new`, FD/XL builders, validation, display
- `frame::CanFrame` — wire-level frame, flags, channel
- `filter::{CanFilter, CanFilters}` — mask/exact/extended 匹配
- `bus::CanBus` — async `send`/`recv`/`set_filters`
- `listener::{Listener, BufferedReader, PrinterListener}`

## DFX 设计

| 维度 | 设计 |
| --- | --- |
| 可靠性 | `CanError` 统一错误；message validation 拒绝冲突 flag 与越界 ID/DLC |
| 可维护性 | 模块边界清晰；crate 内 42 个单元测试 |
| 可测试性 | 纯逻辑模块无 IO 依赖；filter/message 高覆盖率 |
| 可观测性 | 依赖 workspace `tracing`；core 本身不强制 logging |
| 可扩展性 | `CanProtocol` 含 CAN XL；`#[non_exhaustive]` 预留 |

**缺口**：`bus.rs`、`cyclic.rs` 行覆盖率 0%（仅 trait/类型定义，无集成测试）；软件过滤兜底、周期任务 registry 未实现。

## 性能指标

同数据 microbenchmark（2026-06-03，`benchmarks/results/2026-06-03/`，1M iterations）：

| 场景 | Rust ns/iter | python-can ns/iter | 提升 | 20x | 100x |
| --- | ---: | ---: | ---: | --- | --- |
| classic 8B create | 9.876 | 362.038 | **36.7x** | 达标 | 未达 |
| CAN FD 64B create | 10.713 | 400.237 | **37.4x** | 达标 | 未达 |
| 8B clone | 1.115 | 594.139 | **532.9x** | 达标 | 达标 |
| 8B validate | 0.848 | 186.935 | **220.4x** | 达标 | 达标 |
| 4-filter match | 1.792 | 212.592 | **118.6x** | 达标 | 达标 |

bus send/recv、周期发送：**未验证**（无 python-can 同数据对照）。

功能对标 python-can：message/filter 核心路径领先；bus 行为契约、周期发送、ThreadSafeBus、AsyncBufferedReader **未实现**。

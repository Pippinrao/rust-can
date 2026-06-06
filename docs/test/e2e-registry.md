# E2E 场景登记表

> 权威要求：[REQUIREMENTS.md](REQUIREMENTS.md) E2E-01～E2E-04  
> 功能分母：[python-can-compatibility.md](../design/python-can-compatibility.md)（**已实现 / 部分实现**，硬件除外）

**状态说明**：`通过` · `待建` · `未实现`（功能未落地，E2E 不适用）

**覆盖率**（2026-06-06 补全测试系统后）：见 [AUDIT.md](AUDIT.md)

---

## 首批矩阵场景（AUDIT 定义）

| 场景 ID | 描述 | 涉及 crate | 测试位置 | 状态 |
| --- | --- | --- | --- | --- |
| E2E-IO-001 | ASC write → read roundtrip（CAN/CANFD/LIN） | io | `integration-tests/tests/e2e_io_roundtrip.rs` | 通过 |
| E2E-IO-002 | BLF write → read roundtrip（CAN/CANFD） | io | `integration-tests/tests/e2e_io_roundtrip.rs` | 通过 |
| E2E-IO-003 | format detect（扩展名 + BLF magic） | io | `integration-tests/tests/e2e_io_roundtrip.rs` | 通过 |
| E2E-ADP-001 | virtual send → recv 多接收者 | adapters | `integration-tests/tests/e2e_virtual_notifier.rs` | 通过 |
| E2E-NTF-001 | virtual bus → notifier → BufferedReader | adapters + notifier + core | `integration-tests/tests/e2e_virtual_notifier.rs` | 通过 |
| E2E-COR-001 | filter 在 bus recv 路径兜底 | core + adapters | `integration-tests/tests/e2e_filter_fallback.rs` | 通过 |

---

## 公开 API 对应（已实现 / 部分实现）

| 功能 / API | 矩阵状态 | 场景 ID | 状态 | 备注 |
| --- | --- | --- | --- | --- |
| `CanMessage` | 部分实现 | E2E-COR-MSG-001 | 待建 | 跨 crate 校验链；当前 UT + perf 覆盖 |
| `CanProtocol` | 已实现 | E2E-NTF-001 | 通过 | 经 virtual bus 间接验证 |
| `BusABC` / `CanBus` | 部分实现 | E2E-NTF-001, E2E-COR-001 | 通过 | virtual + FilteredBus |
| `BusState` | 部分实现 | — | 待建 | setter 未实现 |
| `CanError` 层次 | 部分实现 | E2E-NTF-001 | 通过 | notifier 错误 dispatch |
| `BitTiming` / `BitTimingFd` | 部分实现 | E2E-COR-BTM-001 | 通过 | `e2e_bit_timing.rs` |
| `CyclicTask` | 部分实现 | E2E-CYC-001 | 通过 | `TokioCyclicTask` + virtual |
| `Notifier` | 部分实现 | E2E-NTF-001 | 通过 | |
| `Listener` / `BufferedReader` | 部分实现 | E2E-NTF-001 | 通过 | |
| `LogReader` / format detect | 部分实现 | E2E-IO-003 | 通过 | |
| `MESSAGE_READERS` registry | 部分实现 | E2E-IO-003 | 通过 | ASC/BLF 扩展名 + magic |
| `MESSAGE_WRITERS` registry | 部分实现 | E2E-IO-001/002 | 通过 | writer roundtrip |
| `ASCReader` / `ASCWriter` | 已实现 | E2E-IO-001 | 通过 | |
| `BLFReader` / `BLFWriter` | 部分实现 | E2E-IO-002, E2E-IO-004 | 通过 | LIN BLF 待样本 |
| `Printer` | 部分实现 | E2E-COR-PRT-001 | 通过 | `e2e_printer_listener.rs` |
| `VALID_INTERFACES` / registry | 部分实现 | E2E-ADP-001 | 通过 | virtual 注册 + 收发 |
| `virtual` backend | 部分实现 | E2E-ADP-001, E2E-COR-001 | 通过 | 含软件 filter |

---

## 核心模块对应

| 模块 | 矩阵状态 | 场景 ID | 状态 |
| --- | --- | --- | --- |
| `can.message` | 部分实现 | — | 待建 |
| `can.bit_timing` | 部分实现 | E2E-COR-BTM-001 | 通过 |
| `can.broadcastmanager` | 部分实现 | — | 未实现 |
| `can.bus` | 部分实现 | E2E-COR-001 | 通过 |
| `can.exceptions` | 部分实现 | E2E-NTF-001 | 通过 |
| `can.listener` | 部分实现 | E2E-NTF-001, E2E-COR-PRT-001 | 通过 |
| `can.notifier` | 部分实现 | E2E-NTF-001 | 通过 |
| `can.typechecking` | 部分实现 | — | 待建 |
| IO ASC/BLF | 已实现/部分 | E2E-IO-001/002/003/004/005 | 通过 |

---

## BusABC 行为契约（部分实现条目）

| 行为 | 场景 ID | 状态 |
| --- | --- | --- |
| `send` / adapter write | E2E-ADP-001 | 通过 |
| `recv` 软件过滤兜底 | E2E-COR-001 | 通过 |
| `set_filters` / 硬件 hook | E2E-COR-001 | 通过 | virtual `apply_hardware_filters` |
| `send_periodic` | E2E-CYC-001 | 通过 | `TokioCyclicTask` |
| context / `Drop` cleanup | E2E-ADP-001 | 通过 | virtual close |

---

## 周期发送（部分实现条目）

| 能力 | 场景 ID | 状态 |
| --- | --- | --- |
| `CyclicTask::stop` | E2E-CYC-001 | 通过 |
| `CyclicTask::modify` | E2E-CYC-001 | 通过 | UT 覆盖 |
| 线程 / tokio fallback | E2E-CYC-001 | 通过 | `TokioCyclicTask` |

---

## 后端（当前阶段）

| 后端 | 矩阵状态 | 场景 ID | 状态 |
| --- | --- | --- | --- |
| `virtual` | 部分实现 | E2E-ADP-001, E2E-NTF-001, E2E-COR-001 | 通过 |
| 硬件后端（socketcan 等） | 未实现 | — | —（范围外） |

---

## 维护流程

1. 更新 [python-can-compatibility.md](../design/python-can-compatibility.md) 状态时，同步本表新增行。
2. 新增 E2E 测试后，将对应行改为 **通过** 并填写测试路径。
3. 功能未落地标 **未实现**；已落地无 E2E 标 **待建**。
4. Release 前核对：所有 **待建** 须有 issue 或里程碑。

# rust-can 架构设计

本文档基于当前仓库源码和上游 `python-can` 源码分析编写。上游源码已拉取到 `.external/python-can`，分析基准提交为 `491a691fd1faffab1c48956bafd711e7c653db54`。

## 目标

rust-can 的目标不是简单移植 python-can，而是提供一个覆盖面更广、性能更强、接口更容易扩展的 CAN 工具链。

核心目标如下：

- 功能覆盖 python-can 的核心 API、日志格式和 CLI 工具；当前阶段重点是 ASC/BLF 日志 IO、真实日志测试语料和同数据性能对比。真实硬件后端实现不是当前目标，只保留可扩展的注入接口和能力声明。
- 在相同输入数据、相同机器、相同测试场景下，关键路径性能相对 python-can 达到 20x 以上提升。没有实测数据不得宣称达标。
- 默认减少内存分配和内存拷贝。经典 CAN 和 CAN FD 消息必须走固定容量内联存储；日志解析、后端读写和分发路径优先使用借用、缓冲池或 `Bytes`/slice 语义。
- 保持接口易用。顶层 API 应接近 `python-can` 的 `Bus`、`Message`、`Notifier`、`Logger`、`LogReader`，但内部实现必须允许静态分发、特性开关和后端扩展。
- 测试全覆盖。行覆盖率目标 80%+，核心消息、过滤、日志解析、后端适配、Notifier 和 benchmark 对比工具必须有自动化测试。

## 当前阶段重点

2026-06-04 的目标边界如下：

- P0 实现 ASC reader/writer，必须支持真实 `data/` 样本中的 CAN、CANFD、LIN 三类记录。
- P0 设计 `LogEvent` 事件模型，预留未来新增 FlexRay、Ethernet、诊断事件、统计事件和厂商扩展 raw event。
- P0/P1 实现 BLF reader/writer。当前先以 CAN/CANFD fixture 建立对照；LIN BLF 需要真实样本或对象类型确认后推进。
- P0 建立真实 ASC/BLF 同数据 benchmark。当前 Rust ASC 和无压缩 BLF IO 已达 20x+；python-can zlib 压缩 BLF 低于 20x，必须标记为性能异常。
- 硬件后端实现延后；`rust-can-adapters` 当前只作为 adapter injection SPI、能力 metadata、mock/virtual conformance test 边界。

详细计划见 [REAL_LOG_IO_ARCHITECTURE.md](REAL_LOG_IO_ARCHITECTURE.md)。

## python-can 模块分析

python-can 的公开模块可以分为这些能力域：

| python-can 模块 | 主要职责 | rust-can 对应设计 |
| --- | --- | --- |
| `can.message` | `Message` 数据模型、校验、格式化、拷贝、相等比较 | `rust-can-core::message`，补充 tolerant equality、builder、borrowed view |
| `can.bus` | `BusABC`、过滤、周期发送、状态、上下文管理 | `rust-can-core::bus`，拆为同步 fast path 和异步 dyn path |
| `can.interface` | `Bus()` 工厂、配置加载、后端发现 | `rust-can-adapters::factory` 和 `config` |
| `can.interfaces.*` | 硬件和虚拟后端 | `rust-can-adapters::backends::*`，外加动态插件 ABI |
| `can.broadcastmanager` | 周期发送任务、可修改、可重启、多速率、限时 | `rust-can-core::cyclic` 和后端硬件周期发送扩展 |
| `can.notifier` | 多 bus 多 listener 分发，fd/event-loop 优先，线程兜底 | `rust-can-notifier`，需要补齐 registry、fd reactor、async listener |
| `can.listener` | Listener、RedirectReader、BufferedReader、AsyncBufferedReader | `rust-can-core::listener`，补齐 redirect 和 async reader |
| `can.thread_safe_bus` | 发送/接收独立锁的线程安全包装 | `rust-can-core::sync` 或 `rust-can-adapters::wrappers` |
| `can.io.*` | ASC、BLF、TRC、CSV、canutils、MF4、SQLite、printer | `rust-can-io::formats::*`，ASC CAN/CANFD/LIN 和 BLF CAN/CANFD 已实现 |
| `can.logger` / `can.player` / `can.viewer` / `can.bridge` / `can.logconvert` | CLI 工具和 TUI | `rust-can-cli`，目前只有命令入口，命令族待实现 |
| `can.bit_timing` | bit timing 和 oscillator tolerance | `rust-can-core::bit_timing`，当前只覆盖基础计算 |
| `can.util` | 配置、DLC 映射、通道解析、计时校准、弃用参数 | `rust-can-config` 或 `rust-can-adapters::config`，当前缺失 |
| `can.exceptions` | 错误类型 | `rust-can-core::error`，当前基础可用 |
| `can.ctypesutil` | C 动态库封装 | `rust-can-ffi` 和 vendor backend FFI helpers |

python-can 当前内置后端包括 `socketcan`、`virtual`、`udp_multicast`、`serial`、`slcan`、`kvaser`、`pcan`、`vector`、`ixxat`、`systec`、`usb2can`、`gs_usb`、`nixnet`、`nican`、`iscan`、`ics_neovi`、`neousys`、`etas`、`seeedstudio`、`cantact`、`robotell`、`canalystii`、`socketcand`。rust-can 当前只实现了 `virtual`。

## 推荐 workspace 模块

当前 workspace 已包含这些 crate：

- `rust-can-core`: 核心类型、错误、过滤、bus trait、bit timing、listener、cyclic。
- `rust-can-adapters`: 后端适配接口、配置、注册表、虚拟后端。
- `rust-can-io`: 日志事件、ASC/BLF 读写和格式探测。
- `rust-can-notifier`: 异步消息分发。
- `rust-can-cli`: CLI 入口，命令族待实现。
- `rust-can-ffi`: C FFI crate，目前导出版本信息。
- `benchmarks`: Criterion benchmark、同数据 Rust/Python 对比 harness、pytest-benchmark Python 对照。当前消息/过滤微路径和真实 ASC/BLF IO 有实测；bus 对比待实现。

完整 python-can API、IO、CLI、后端逐项对应见 [PYTHON_CAN_COMPATIBILITY.md](PYTHON_CAN_COMPATIBILITY.md)。

建议新增或拆分：

- `rust-can-config`: 统一处理配置文件、环境变量、CLI 参数合并、profile/context、DLC 映射和通道解析。这样 adapters 不需要承担配置解析全部职责。
- `rust-can-python`: PyO3 绑定，用 Rust 实现兼容 python-can 常用 API 的 Python 包。当前 workspace 只有 `pyo3` 依赖声明，没有独立 crate。
- `rust-can-codecs`: 专门处理二进制和文本日志格式的零拷贝 codec，可被 `rust-can-io` 和 CLI 复用。
- `rust-can-macros`: 可选。用于后端注册、配置 schema 和 FFI 导出，只有在重复样板明显增加后再引入。

## 核心架构

### 消息模型

`CanMessage` 应作为性能优先的 owned 消息类型。经典 CAN 和 CAN FD 使用固定 `[u8; 64]` 内联存储，避免堆分配。需要新增一个借用视图：

- `CanMessageRef<'a>`: 指向外部 payload 的只读消息视图，用于日志解析和后端读取。
- `CanMessageMut<'a>`: 可选，用于原地修改 payload。
- `CanMessageXL`: CAN XL 专用，payload 保存在 `Bytes` 中，header 内联前 64 字节；长 payload 截断问题已修复。

当前不符合目标的点：

- `CanMessageXL::new` 已先以 `usize` 校验完整 payload，再限制 header inline 长度；后续仍建议为 CAN XL 引入专用长度字段。
- `CanFrame` 使用 `Bytes`，但 `From<CanMessage> for CanFrame` 调用了 `Bytes::copy_from_slice`，这是一次拷贝。应提供 borrowed frame 或 inline frame fast path。
- `Display` 中的 hex 编码会为每个字节分配 `String` 并收集到 `Vec`，只能用于展示，不能进入热路径。
- `now_nanos()` 当前使用原子计数，不是真实时间。适合 benchmark 稳定性，但文档和 API 需要明确 timestamp source。

### Bus 接口

需要同时支持易用和高性能：

- `CanBus`: 面向应用的 trait object API，兼容动态后端。
- `CanBusSync`: 同步 fast path，避免 async_trait 装箱，适用于高频硬件读写。
- `CanBusAsync`: 基于 GAT 或显式 future 类型的异步 fast path，尽量避免 `async_trait`。
- `BusHandle`: 用户易用入口，内部可持有静态后端、动态后端或插件后端。

当前不符合目标的点：

- `#[async_trait]` 会让 `send`/`recv` 返回装箱 future，影响高吞吐路径。
- `CanBus::recv` 没有实现 python-can 的软件过滤兜底语义。python-can 的 `recv` 会持续读 `_recv_internal`，直到匹配过滤器或超时。
- `send_periodic` 缺少 `duration`、`autostart`、`store_task`、`modifier_callback`、多速率任务等完整语义。

### 后端适配

当前阶段不实现真实硬件后端。后端模块应先提供注入接口、能力声明和 mock/virtual 测试边界，让后续任何硬件都能接入而不污染 core/IO/CLI。

建议接口分层：

- `CanAdapter`: 最小 read/write/close/flush 接口。
- `CanAdapterFactory`: 从 typed config 创建 adapter。
- `AdapterCapabilities`: 声明协议、硬件过滤、fd/handle、周期发送、timestamp source、queue/backpressure。
- `AdapterRegistry`: 支持静态注册、动态插件注册和测试注入。
- `MockAdapter`/`VirtualAdapter`: 用于 conformance test 和性能 harness，不等同于真实硬件目标。

每个后端必须声明能力：

- 协议: CAN 2.0、CAN FD、CAN FD non-ISO、CAN XL。
- 过滤: 硬件过滤、软件过滤、混合过滤。
- 事件: fd、Windows handle、polling。
- 周期发送: 硬件 BCM、驱动周期发送、软件周期发送。
- 时间戳: 硬件时间戳、系统时间、单调时间。

当前不符合目标的点：

- `VirtualAdapter` 用全局 `Mutex<HashMap<String, Vec<Sender<CanFrame>>>>`，广播时持锁并逐接收者 clone frame。高 fan-out 时会放大锁竞争和拷贝。
- 注册表依赖 `LazyLock` 但 `virtual` 的 `_REGISTER` 没有被强制触发，存在注册不发生的风险。
- `AdapterInfo` 使用多个 `String`，可考虑 `Cow<'static, str>` 或静态描述降低常驻分配。

### Notifier 和 Listener

Notifier 应覆盖 python-can 行为并提升吞吐：

- 每个 bus 只能被一个活跃 Notifier 绑定，避免重复消费。
- 若 bus 提供 fd/handle，优先接入 reactor；否则使用专用接收任务。
- Listener 支持同步函数、异步函数、trait object 和 typed channel sink。
- BufferedReader 需要支持 bounded queue、背压策略和丢弃统计。
- RedirectReader 需要补齐，用于 bridge。

当前不符合目标的点：

- `rust-can-notifier` 未实现 bus registry，不能防止一个 bus 被多个 Notifier 消费。
- 当前分发时持 `listeners` 锁调用 listener，慢 listener 会阻塞 add/remove 和其他分发。
- `crossbeam::channel` 已导入但未使用；“lock-free ring buffer” 注释与实现不一致。

### IO 和日志格式

`rust-can-io` 应把 reader/writer 和 codec 分开：

- Reader/Writer: 面向文件、流、压缩、rotation、context manager 等用户接口。
- Codec: 面向 bytes/text chunk 的解析和编码，无文件依赖。
- Registry: 按扩展名和 MIME/魔数选择格式。

当前阶段必须覆盖：

- ASC: streaming reader/writer，支持 CAN、CANFD、LIN、metadata/raw/unknown event。
- BLF: streaming/block reader/writer，先支持 CAN/CANFD，预留 LIN 和未知 object roundtrip。
- `LogEvent`: 日志层事件模型，不能把 LIN 强塞进 `CanMessage`。
- Registry: 按扩展名、魔数和压缩后缀选择 ASC/BLF。

后续兼容目标：

- TRC、CSV、canutils log、SQLite、MF4。
- Printer、SizedRotatingLogger、LogReader、Logger、MessageSync。

当前状态：ASC CAN/CANFD/LIN、BLF CAN/CANFD、格式探测和基础 writer trait 已实现；TRC/CSV/canutils/MF4/SQLite 待实现。

### CLI

CLI 应包含：

- `cancli dump`: 实时打印 bus 消息。
- `cancli logger`: 记录到文件，支持格式、append、rotation。
- `cancli player`: 回放日志，支持 timestamp、gap、skip、loop、error frame 开关。
- `cancli bridge`: 多 bus 转发，支持过滤、方向、错误帧策略。
- `cancli logconvert`: 日志格式转换。
- `cancli detect`: 后端可用配置检测。
- `cancli viewer`: 可选 TUI。

当前状态：命令入口存在，logger/player/dump/bridge/logconvert/viewer/detect 命令族待实现。

### FFI 和 Python 兼容

FFI 目标：

- C ABI 提供稳定的 opaque handle、错误码、消息结构、bus open/send/recv/shutdown。
- PyO3 提供 Python 包，兼容 `python-can` 的高频 API，便于同数据 benchmark。
- Vendor SDK 后端应隔离 unsafe，并为每个 unsafe 边界添加测试或 mock。

当前状态：未实现，只导出 `version()`。

## 性能目标和验收

性能目标必须以实测数据判断：

| 场景 | rust-can 目标 | 对比方式 | 未达标处理 |
| --- | --- | --- | --- |
| 创建经典 CAN 消息 | python-can 20x+ | 同 ID、同 data、同校验策略，Criterion vs pytest-benchmark | 标记异常，分析分配和时间戳成本 |
| 创建 CAN FD 消息 | python-can 20x+ | 64 字节 payload，同校验策略 | 标记异常 |
| clone/copy 消息 | python-can 20x+ | 同 payload 长度，区分 owned 和 borrowed | 标记异常 |
| filter match | python-can 20x+ | 同过滤器集合、同消息集合 | 标记异常 |
| virtual bus 单生产单消费 | python-can 20x+ | 同消息数、同 payload、同 receive_own 配置 | 标记异常 |
| virtual bus 多接收者 fan-out | python-can 20x+ 或明确解释 | 1、4、16 listeners 同数据 | 未达标需优化锁和拷贝 |
| ASC CAN/CANFD/LIN 解析 | python-can 20x+ | 同真实 ASC 文件、同解析输出；LIN 以 rust-can golden corpus 验证 | 标记异常 |
| BLF CAN/CANFD 解析 | python-can 20x+ | 同 BLF fixture、同解析输出 | 标记异常 |
| Notifier 分发 | python-can 20x+ | 同 bus、同 listener 数、同消息数 | 标记异常 |

基准规则：

- 同一台机器、同一 CPU governor、电源策略、release build。
- Rust 使用 `cargo bench --workspace` 或指定 bench；Python 使用 `pytest-benchmark` 或 `pyperf`。
- 输入数据由仓库脚本生成，不能手工构造两套不同数据。
- 每次结果保存到 `benchmarks/results/YYYY-MM-DD/`，包含 Rust JSON、Python JSON、机器信息和异常列表。
- 若任何目标未实测，文档和 README 必须写“未验证”，不能写“已达到 20x”。

内存目标：

- 对热路径加入 allocation 计数 benchmark。
- 经典 CAN 和 CAN FD 创建消息的目标是 0 次堆分配。
- 日志解析应支持 streaming，不允许整文件读入作为默认行为。
- 后端分发路径应避免对每个 listener 无条件深拷贝 payload。

## 当前实测性能数据

测试日期：2026-06-03。原始结果保存在 `benchmarks/results/2026-06-03/`。

已安装并使用的工具：

- Rust 同数据 harness: `cargo run -p rust-can-benchmarks --release --bin perf_compare -- 1000000`
- Rust Criterion: `cargo bench -p rust-can-benchmarks --bench message_bench`
- Python pytest-benchmark: `python -m pytest benchmarks\python\test_python_can_benchmark.py --benchmark-only --benchmark-json=benchmarks\results\2026-06-03\pytest-benchmark-python.json`
- 覆盖率: `cargo llvm-cov --workspace --all-features --summary-only`

同数据 Rust/Python 对比结果：

| 场景 | Rust ns/iter | python-can ns/iter | 提升 | 状态 |
| --- | ---: | ---: | ---: | --- |
| classic 8B message create | 9.876 | 362.038 | 36.7x | 达到 20x |
| CAN FD 64B message create | 10.713 | 400.237 | 37.4x | 达到 20x |
| 8B message clone/copy | 1.115 | 594.139 | 532.9x | 达到 20x |
| 8B message validate | 0.848 | 186.935 | 220.4x | 达到 20x |
| 4-filter match | 1.792 | 212.592 | 118.6x | 达到 20x |

pytest-benchmark 对 python-can 的独立工具测量均值：

| pytest-benchmark 场景 | python-can mean ns/iter |
| --- | ---: |
| classic 8B message create | 565.215 |
| CAN FD 64B message create | 464.515 |
| 8B message clone/copy | 749.052 |
| 8B message validate | 216.827 |
| 4-filter match | 260.539 |

Criterion 对 Rust 的独立工具测量摘要：

| Criterion 场景 | Rust time |
| --- | ---: |
| `message_create_can20` | 10.755-10.810 ns |
| `message_create_canfd` | 8.556-8.626 ns |
| `message_clone` | 24.475-24.807 ns |
| `message_validate` | 827.42-829.46 ps |

解释和限制：

- 当前可宣称 20x+ 的只有消息创建、clone/copy、validate、filter match 这些微路径。
- `message_clone` 的 Criterion 数值高于同数据 harness，因为 Criterion bench 当前 clone 的消息和 payload 设置不同；最终发布前应统一 Criterion 和同数据 harness。
- bus send/recv、Notifier、CLI、FFI、硬件后端没有同数据实测，必须标记为未验证或未实现。ASC 和无压缩 BLF IO 已有 20x+ 同数据实测；python-can zlib 压缩 BLF 仍低于 20x，必须标记为性能异常。
- allocation 统计尚未实现，不能声称“0 分配已实测”，只能把它作为目标。

真实日志 IO 实测日期：2026-06-06。原始结果位于 `benchmarks/results/2026-06-06/`。

| 场景 | python-can | rust-can | 提升 | 状态 |
| --- | ---: | ---: | ---: | --- |
| ASC 读取，真实大 ASC 前 100,000 条 CAN/CANFD | 275,862 msg/s | 6,746,798 msg/s | 24.46x | 达到 20x |
| BLF 读取，python-can zlib `real_can_canfd_100000.blf` | 580,077 msg/s | 8,616,803 msg/s | 14.85x | 异常：低于 20x |
| BLF 读取，rust-can 无压缩 `rust_can_canfd_100000.blf` | 599,179 msg/s | 53,886,448 msg/s | 89.93x | 达到 20x |

## 测试策略

最低要求：

- 行覆盖率 80%+。推荐 `cargo llvm-cov --workspace --all-features --fail-under-lines 80`。
- 单元测试覆盖 core 类型、校验、filter、bit timing、DLC 映射、错误类型。
- 集成测试覆盖 virtual bus、Notifier、CLI dry-run、日志 roundtrip。
- 后端测试分为 mock、virtual、hardware-gated 三类。硬件测试必须通过 feature 或环境变量显式开启。
- FFI 测试必须验证 ABI smoke test 和错误传播。
- benchmark 不是替代测试。性能测试必须可重复运行，并且失败时能标记异常。

当前覆盖率实测结果：

| 指标 | 当前值 | 目标 | 状态 |
| --- | ---: | ---: | --- |
| Region coverage | 80.83% | 80%+ | 达标 |
| Function coverage | 80.99% | 80%+ | 达标 |
| Line coverage | 81.35% | 80%+ | 达标 |

当前行、region、function 覆盖率门禁均已达到 80%+；仍需继续提升未实现模块的覆盖质量。

## 当前仓库缺口

已实现或部分实现：

- `CanMessage`、`CanFrame`、`CanFilter`、`CanProtocol`、`BusState`、`CanError`。
- `BitTiming` 基础计算。
- `CanBus` trait 和 `CyclicTask` trait 雏形。
- `CanAdapter` trait、`AdapterConfig`、`AdapterInfo`、`VirtualAdapter`。
- `Notifier` 和 `BufferedReader` 雏形。
- `rust-can-io` ASC CAN/CANFD/LIN reader/writer、BLF CAN/CANFD reader/writer、格式探测。
- Criterion 消息 benchmark 和真实 ASC/BLF IO benchmark。

仍待实现：

- TRC、CSV、canutils、SQLite、MF4 等后续日志格式 reader/writer。
- `rust-can-cli` 的 dump/logger/player/bridge/logconvert/viewer/detect。
- C FFI 和 Python bindings。
- 除 virtual 外的所有硬件后端。
- 配置文件和环境变量加载。
- 后端 autodetect。
- ThreadSafeBus 包装。
- RedirectReader、AsyncBufferedReader。
- Notifier registry、fd/handle reactor、async callback 管理。
- 完整周期发送语义和硬件周期发送。
- 80%+ 覆盖率门禁当前已达标；region、function、line coverage 均已超过 80%，仍需继续提升未实现模块的覆盖质量。
- 与 python-can 的完整同数据性能对比工具。当前已有消息/过滤微路径和 ASC/BLF IO 对比；bus、Notifier、CLI、FFI 仍需补齐。

必须优化或修复：

- 为 CAN XL 引入专用长度字段，避免长期依赖 header inline 长度表达完整 payload。
- 为 bus trait 增加无 `async_trait` fast path。
- 为 `CanFrame` 和日志 codec 增加 borrowed/zero-copy 路径。
- 避免 VirtualAdapter 广播时持全局锁做 clone 和阻塞发送。
- 明确 timestamp source，并提供硬件、系统、单调和 benchmark timestamp 策略。
- 补齐 DLC 和 len 映射，包括 CAN FD DLC 编码。
- 让 adapter registry 注册可预测，避免惰性静态未触发。
- 为 hot path 建立 allocation benchmark。

## 路线图

### 阶段 0: 正确性和基线

- 修复 CAN XL 长度类型。
- 补齐 message equality、DLC 映射、timestamp policy。
- 建立 coverage、Criterion、pytest-benchmark、pyperf 基准框架。
- 生成同数据 benchmark corpus。
- README 中保持性能状态为“未验证”，直到有对比结果。

### 阶段 1: 核心 API 完整度

- 拆分 sync fast path 和 async ergonomic path。
- 实现 Bus 工厂、配置加载、软件过滤兜底。
- 补齐周期发送任务语义。
- 实现 ThreadSafeBus、RedirectReader、AsyncBufferedReader。

### 阶段 2: IO 和 CLI

- 实现 ASC 的 CAN/CANFD/LIN streaming reader/writer。
- 实现 `LogEvent`、payload borrowed view、格式 registry。
- 使用真实 `data/` ASC 和 generated BLF fixture 建立 roundtrip、golden corpus 和同数据 benchmark。
- 实现 `logconvert`/`player` 的 ASC/BLF 最小路径。

### 阶段 3: 后端注入接口

- 完善 adapter injection SPI、capability metadata、factory、registry。
- 加入 mock/virtual conformance tests。
- 为未来真实硬件后端定义 unsafe/FFI 边界模板，但不在当前阶段实现具体硬件。

### 阶段 4: 高性能和兼容包

- 实现 BLF CAN/CANFD，预留 LIN object 支持。
- 后续实现 TRC、SQLite、MF4。
- 实现 C FFI 和 PyO3 Python 包。
- 完成同数据 20x+ 对比报告。

### 阶段 5: 发布质量

- 80%+ 行覆盖率强制门禁。
- 文档、示例、后端能力矩阵、benchmark 报告公开。
- 建立异常处理流程：性能未达标、功能不兼容、硬件不可测均需显式记录。

## 决策原则

- 先正确，再快；快必须可测。
- 用户 API 保持简单，内部 API 允许精细化。
- 热路径不为动态扩展买单。动态插件和 trait object 应可用，但不应阻塞静态后端零成本路径。
- 后端能力要显式声明，不能靠运行时猜测。
- 未实现就是未实现，未验证就是未验证，不用乐观措辞掩盖状态。

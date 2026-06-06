# python-can 到 rust-can 功能对应矩阵

[English overview](en/overview.md)

上游参考源码：`hardbyte/python-can` commit `491a691fd1faffab1c48956bafd711e7c653db54`。

状态说明：

- 已实现：当前 rust-can 代码已有可运行实现。
- 部分实现：已有类型或雏形，但行为还未覆盖 python-can。
- 待实现：当前只有规划或接口入口。
- 扩展目标：python-can 没有或覆盖较弱，rust-can 应做得更好的方向。

## 公开 API 对应

| python-can API | python-can 来源 | rust-can 对应 | 状态 | 备注 |
| --- | --- | --- | --- | --- |
| `Message` | `can.message` | `rust_can_core::message::CanMessage` | 部分实现 | 缺 tolerant equality、Python 兼容 repr、timestamp policy；CAN XL 长 payload 截断已修复 |
| `Bus` | `can.interface.Bus` | `rust-can-adapters` factory + `BusHandle` | 未实现 | 当前没有统一工厂 |
| `BusABC` | `can.bus.BusABC` | `rust_can_core::bus::CanBus` | 部分实现 | 需补软件过滤兜底、迭代、上下文语义、fast path |
| `BusState` | `can.bus` | `rust_can_core::protocol::BusState` | 部分实现 | 枚举存在；state setter 和后端透传未实现 |
| `CanProtocol` | `can.bus` | `rust_can_core::protocol::CanProtocol` | 已实现 | rust-can 已加入 CAN XL 目标 |
| `CanError` | `can.exceptions` | `rust_can_core::error::CanError` | 部分实现 | 需细化后端错误码和 FFI 错误 |
| `CanInitializationError` | `can.exceptions` | `CanError::InitializationError` | 部分实现 | 只有枚举变体；缺 `error_code` 和 Python 类型层级 |
| `CanInterfaceNotImplementedError` | `can.exceptions` | `CanError::InterfaceNotImplemented` | 部分实现 | 只有枚举变体；缺 Python 兼容异常层级 |
| `CanOperationError` | `can.exceptions` | `CanError::OperationError` | 部分实现 | 只有枚举变体；缺 vendor status 映射 |
| `CanTimeoutError` | `can.exceptions` | `CanError::TimeoutError` | 部分实现 | 只有枚举变体；缺 Python 兼容异常层级 |
| `error_check` | `can.exceptions` | backend FFI status mapper | 未实现 | vendor SDK 错误检查 helper |
| `BitTiming` | `can.bit_timing` | `rust_can_core::bit_timing::BitTiming` | 部分实现 | 缺 oscillator tolerance 完整算法 |
| `BitTimingFd` | `can.bit_timing` | `rust_can_core::bit_timing::BitTimingFd` | 部分实现 | 当前只有 nominal/data 组合 |
| `CyclicSendTaskABC` | `can.broadcastmanager` | `rust_can_core::bus::CyclicTask` | 部分实现 | 缺 restart、duration、multi-rate、modifier data 语义 |
| `CyclicTask` | `can.broadcastmanager` | `rust_can_core::bus::CyclicTask` | 部分实现 | 只有 stop/modify/is_running 雏形 |
| `LimitedDurationCyclicSendTaskABC` | `can.broadcastmanager` | `rust_can_core::cyclic` | 未实现 | 需要 duration 精确停止 |
| `RestartableCyclicTaskABC` | `can.broadcastmanager` | `rust_can_core::cyclic` | 未实现 | 需要 start/stop 重启 |
| `ModifiableCyclicTaskABC` | `can.broadcastmanager` | `CyclicTask::modify` | 部分实现 | 需校验 ID/channel/长度不变 |
| `MultiRateCyclicSendTaskABC` | `can.broadcastmanager` | `MultiRateCyclicTask` | 未实现 | 需要 initial/subsequent period 和 count |
| `ThreadBasedCyclicSendTask` | `can.broadcastmanager` | software cyclic fallback | 未实现 | Rust 应优先 timer wheel / tokio interval / OS timer |
| `Notifier` | `can.notifier` | `rust_can_notifier::Notifier` | 部分实现 | 缺 registry、fd reactor、async callback、异常保存 |
| `Listener` | `can.listener` | `rust_can_core::listener::Listener` | 部分实现 | 基础 trait 已有 |
| `BufferedReader` | `can.listener` | `rust_can_core::listener::BufferedReader` | 部分实现 | 停止后行为和错误语义需对齐 |
| `AsyncBufferedReader` | `can.listener` | `AsyncBufferedReader` | 未实现 | 需要 async iterator 或 stream |
| `RedirectReader` | `can.listener` | `RedirectReader` | 未实现 | bridge 需要 |
| `ThreadSafeBus` | `can.thread_safe_bus` | `ThreadSafeBus` wrapper | 未实现 | 需要 send/recv 独立锁 |
| `Logger` | `can.io.logger` | `rust-can-io` + `rust-can-cli logger` | 待实现 | IO crate 已有 ASC/BLF 读写，logger orchestration 待实现 |
| `BaseRotatingLogger` | `can.io.logger` | `rust-can-io::logger::BaseRotatingLogger` | 未实现 | rotation 基类和 rollover 策略 |
| `SizedRotatingLogger` | `can.io.logger` | `rust-can-io::logger` | 未实现 | rotation 未实现 |
| `LogReader` | `can.io.player` | `rust-can-io::reader` | 部分实现 | 已有 ASC/BLF 基础格式探测，文件 source adapter 待补 |
| `MessageSync` | `can.io.player` | `rust-can-io::player::MessageSync` | 未实现 | replay timing 未实现 |
| `Printer` | `can.io.printer` | `PrinterListener` / `rust-can-io::Printer` | 部分实现 | core 有 stdout listener，IO Printer 未实现 |
| `MESSAGE_READERS` | `can.io.player` | `rust-can-io` reader registry | 部分实现 | 已有 ASC/BLF 扩展名和 BLF 魔数探测，完整 registry 待补 |
| `MESSAGE_WRITERS` | `can.io.logger` | `rust-can-io` writer registry | 部分实现 | 已有基础 writer trait，完整 registry 待补 |
| `TRCFileVersion` | `can.io.trc` | `rust-can-io::formats::trc::TrcFileVersion` | 未实现 | TRC v1.0/v1.1/v1.3/v2.0 |
| `detect_available_configs` | `can.interface` | `rust-can-adapters::detect` | 未实现 | 需并发探测和 timeout |
| `VALID_INTERFACES` | `can.interfaces` | `rust-can-adapters::registry` | 部分实现 | 当前 registry 不完整 |
| `set_logging_level` | `can.util` | tracing subscriber config | 未实现 | CLI 层需要 |
| `add_bus_arguments` | `can.cli` | `rust-can-cli` shared bus args | 未实现 | CLI bus 参数定义 |
| `create_bus_from_namespace` | `can.cli` | `rust-can-cli` config builder | 未实现 | argparse namespace 到 bus config |

## 核心模块对应

| python-can 模块 | 功能 | rust-can 模块 | 状态 |
| --- | --- | --- | --- |
| `can.__init__` | 顶层重导出 | 顶层 facade crate `rust-can` | 未实现 |
| `can._entry_points` | Python entry point 后端插件 | adapter dynamic registry | 未实现 |
| `can.bit_timing` | 位时序计算 | `rust-can-core::bit_timing` | 部分实现 |
| `can.broadcastmanager` | 周期发送 | `rust-can-core::cyclic` | 部分实现 |
| `can.bus` | bus 抽象、过滤、周期任务 | `rust-can-core::bus` | 部分实现 |
| `can.cli` | CLI 参数到 bus config | `rust-can-cli` + `rust-can-config` | 未实现 |
| `can.ctypesutil` | vendor C library helpers | `rust-can-ffi` + backend FFI helpers | 未实现 |
| `can.exceptions` | 错误层次 | `rust-can-core::error` | 部分实现 |
| `can.interface` | Bus 工厂和 autodetect | `rust-can-adapters::factory` | 未实现 |
| `can.listener` | listener 和 readers | `rust-can-core::listener` | 部分实现 |
| `can.message` | 消息模型 | `rust-can-core::message` | 部分实现 |
| `can.notifier` | 多 bus 分发 | `rust-can-notifier` | 部分实现 |
| `can.thread_safe_bus` | 线程安全 bus proxy | wrapper module | 未实现 |
| `can.typechecking` | Python 类型定义 | Rust strongly typed config/schema | 部分实现 |
| `can.util` | 配置、DLC、时间校准 | `rust-can-config` | 未实现 |

## BusABC 行为契约对应

| python-can 行为 | rust-can 对应 | 状态 | 说明 |
| --- | --- | --- | --- |
| `recv(timeout)` 软件过滤兜底 | `CanBus::recv` wrapper | 未实现 | 需要在未硬件过滤时循环读到匹配或超时 |
| `_recv_internal(timeout) -> (msg, already_filtered)` | adapter read + filter marker | 未实现 | 可用 typed enum 标记是否已过滤 |
| `send(msg, timeout)` | `CanBus::send` | 部分实现 | trait 存在，缺统一 bus 实现 |
| `set_filters` / `_apply_filters` | `CanFilters` + hardware filter hook | 部分实现 | filter 类型存在，bus 兜底未实现 |
| `filters` property | BusHandle filter state | 未实现 | 需要可读写 filter state |
| `send_periodic` | cyclic task factory | 部分实现 | 缺 store/remove/autostart/modifier |
| `stop_all_periodic_tasks` | task registry | 未实现 | Bus 内部需维护任务列表 |
| `__iter__` | sync/async stream iterator | 未实现 | Rust 可提供 iterator/Stream adapter |
| context manager / destructor cleanup | `Drop` + explicit `shutdown` | 部分实现 | 部分类型有 Drop，BusHandle 未实现 |
| `flush_tx_buffer` | `CanBus::flush_tx_buffer` | 部分实现 | 默认 NotSupported，后端未接入 |
| `state` getter/setter | `BusState` + backend setter | 部分实现 | getter 默认 active，setter 未实现 |
| `protocol` | `CanProtocol` | 部分实现 | 枚举存在，后端 protocol 配置未实现 |
| `fileno` | fd/handle reactor source | 未实现 | trait 方法存在，未接入 notifier reactor |
| `_detect_available_configs` | backend detect hook | 未实现 | 需并发 detect 和 timeout |

## 周期发送对应

| python-can 周期能力 | rust-can 对应 | 状态 | 改进目标 |
| --- | --- | --- | --- |
| 基础 stop | `CyclicTask::stop` | 部分实现 | 定义存在，缺完整 bus task registry |
| 限时发送 | `LimitedDurationCyclicTask` | 未实现 | 使用 monotonic deadline |
| 可重启 | `RestartableCyclicTask` | 未实现 | start/stop 状态机 |
| 可修改 data | `CyclicTask::modify` | 部分实现 | 需保持 arbitration ID/channel/消息数量不变 |
| 多速率发送 | `MultiRateCyclicTask` | 未实现 | initial count + subsequent period |
| 线程 fallback | software timer backend | 部分实现 | 当前 tokio task 雏形；需 OS timer 和 jitter 测试 |
| SocketCAN BCM | `socketcan` hardware cyclic task | 未实现 | Linux 下优先用 BCM 降低 jitter |
| error callback | cyclic task error policy | 未实现 | 继续/停止策略显式化 |

## util/config 对应

| python-can util API | rust-can 对应 | 状态 | 说明 |
| --- | --- | --- | --- |
| `load_file_config` | `rust-can-config::file` | 未实现 | 支持 can.conf / ini / toml/yaml 需要决策 |
| `load_environment_config` | `rust-can-config::env` | 未实现 | 环境变量合并 |
| `load_config` | `rust-can-config::load` | 未实现 | 参数、环境、文件按优先级合并 |
| `_create_bus_config` | typed config builder | 未实现 | 输出 adapter config + validation |
| `len2dlc` / `dlc2len` | core DLC mapping | 未实现 | CAN FD DLC 编码必须补齐 |
| `channel2int` | channel parser | 未实现 | 字符串通道归一化 |
| `check_or_adjust_timing_clock` | bit timing validator | 未实现 | 对齐硬件 clock |
| `time_perfcounter_correlation` | timestamp correlation | 未实现 | 硬件时间戳和系统时间换算 |
| `cast_from_string` | CLI/config value parser | 未实现 | bool/int/float/string |
| `deprecated_args_alias` | compatibility layer | 未实现 | Python binding 可选支持 |

## IO 格式对应

当前阶段 IO 重点是 ASC/BLF。ASC 必须支持真实 `data/` 样本中的 CAN、CANFD、LIN；LIN 不属于 python-can `Message` 模型，因此 rust-can 需要 `LogEvent::Lin`，不能只做 `CanMessage` adapter。

| python-can 模块/API | rust-can 对应 | 状态 | 改进目标 |
| --- | --- | --- | --- |
| `can.io.generic.MessageReader` | `rust-can-io::reader::MessageReader` | 未实现 | Reader 与 codec 分离 |
| `can.io.generic.MessageWriter` | `rust-can-io::writer::MessageWriter` | 未实现 | Writer 与 sink 分离 |
| `SizedMessageWriter` | `rust-can-io::writer::SizedMessageWriter` | 未实现 | file size / rotation 依赖 |
| `FileIOMessageWriter` | `rust-can-io::writer::FileWriter` | 未实现 | 文件 sink 适配 |
| `TextIOMessageWriter` | `rust-can-io::writer::TextWriter` | 未实现 | 文本格式公共写入 |
| `BinaryIOMessageWriter` | `rust-can-io::writer::BinaryWriter` | 未实现 | 二进制格式公共写入 |
| `FileIOMessageReader` | `rust-can-io::reader::FileReader` | 未实现 | 文件 source 适配 |
| `TextIOMessageReader` | `rust-can-io::reader::TextReader` | 未实现 | 文本格式公共读取 |
| `BinaryIOMessageReader` | `rust-can-io::reader::BinaryReader` | 未实现 | 二进制格式公共读取 |
| `ASCReader` / `ASCWriter` | `rust-can-io::formats::asc` | 已实现 | streaming parser，支持当前真实 ASC 的 CAN/CANFD/LIN 和 metadata/raw/unknown |
| `BLFReader` / `BLFWriter` | `rust-can-io::formats::blf` | 部分实现 | CAN/CANFD reader 和 writer roundtrip 已实现；LIN BLF object 待真实样本确认 |
| `TRCReader` / `TRCWriter` | `rust-can-io::formats::trc` | 未实现 | 版本枚举强类型 |
| `CSVReader` / `CSVWriter` | `rust-can-io::formats::csv` | 未实现 | serde/csv 或手写 fast path |
| `CanutilsLogReader` / `CanutilsLogWriter` | `rust-can-io::formats::canutils` | 未实现 | 与 can-utils 文本兼容 |
| `SqliteReader` / `SqliteWriter` | `rust-can-io::formats::sqlite` | 未实现 | 批量写入和 prepared statement |
| `MF4Reader` / `MF4Writer` | `rust-can-io::formats::mf4` | 未实现 | 明确依赖策略，不伪称兼容 |
| `Printer` | `rust-can-io::printer` | 未实现 | 可复用 formatting sink |
| `Logger` | `rust-can-io::logger::Logger` | 未实现 | 扩展名 registry |
| `SizedRotatingLogger` | `rust-can-io::logger::SizedRotatingLogger` | 未实现 | rotation 策略独立 |
| `LogReader` | `rust-can-io::reader::LogReader` | 部分实现 | ASC/BLF 扩展名和 BLF 魔数检测已实现，统一 LogReader 待补 |
| `MessageSync` | `rust-can-io::player::MessageSync` | 未实现 | replay timing 策略可注入 |
| suffix registry | `MESSAGE_READERS` / `MESSAGE_WRITERS` | 部分实现 | ASC/BLF 扩展名、常见压缩后缀和 BLF 魔数已覆盖，完整 writer registry 待补 |
| gzip/bzip2/xz decompress | reader source chain | 未实现 | 解压和格式解析解耦 |
| rotation rollover | rotating writer policy | 未实现 | 大小、时间和命名策略独立 |

## CLI 对应

| python-can CLI | rust-can 命令 | 状态 | 改进目标 |
| --- | --- | --- | --- |
| `can.logger` | `cancli logger` | 未实现 | 支持 rotation、append、format、filter |
| `can.logger --append` | `cancli logger --append` | 未实现 | 追加写入 |
| `can.logger --file-size` | `cancli logger --file-size` | 未实现 | SizedRotatingLogger |
| logger extra kwargs | `cancli logger --set key=value` | 未实现 | 透传 writer config |
| `can.player` | `cancli player` | 未实现 | 支持 loop、gap、skip、ignore timestamps |
| `can.player --loop` | `cancli player --loop` | 未实现 | 支持有限/无限循环 |
| `can.player --gap/--skip` | `cancli player --gap/--skip` | 未实现 | replay timing 策略 |
| `can.player --error-frames` | `cancli player --error-frames` | 未实现 | 错误帧回放 |
| `can.viewer` | `cancli viewer` | 未实现 | TUI，可按 ID 聚合 |
| `can.viewer --decode` | `cancli viewer --decode` | 未实现 | struct unpack + scaling |
| viewer sorting/pause/highlight | `cancli viewer` TUI controls | 未实现 | 排序、暂停、变化字节高亮 |
| `can.bridge` | `cancli bridge` | 未实现 | 多 bus、多方向、背压策略 |
| `can.logconvert` | `cancli logconvert` | 未实现 | streaming 格式转换 |
| `can.cli.create_bus_from_namespace` | shared CLI config builder | 未实现 | CLI/config/env 合并 |

## 后端对应

当前阶段不实现真实硬件后端。本表只作为未来注入候选和 python-can 兼容参考；当前要完成的是 adapter SPI、能力声明、factory/registry、mock/virtual conformance tests。

| python-can interface | rust-can backend | 状态 | 当前阶段 |
| --- | --- | --- | --- |
| `virtual` | `rust-can-adapters::backends::virtual` | 部分实现 | 保留为测试/benchmark 注入后端 |
| `socketcan` | `backends::socketcan` | 未实现 | 未来候选，不在当前目标内 |
| `socketcand` | `backends::socketcand` | 未实现 | 未来候选，不在当前目标内 |
| `udp_multicast` | `backends::udp_multicast` | 未实现 | 未来候选，不在当前目标内 |
| `serial` | `backends::serial` | 未实现 | 未来候选，不在当前目标内 |
| `slcan` | `backends::slcan` | 未实现 | 未来候选，不在当前目标内 |
| `gs_usb` | `backends::gs_usb` | 未实现 | 未来候选，不在当前目标内 |
| `pcan` | `backends::pcan` | 未实现 | 未来候选，不在当前目标内 |
| `kvaser` | `backends::kvaser` | 未实现 | 未来候选，不在当前目标内 |
| `vector` | `backends::vector` | 未实现 | 未来候选，不在当前目标内 |
| `ixxat` | `backends::ixxat` | 未实现 | 未来候选，不在当前目标内 |
| `systec` | `backends::systec` | 未实现 | 未来候选，不在当前目标内 |
| `usb2can` | `backends::usb2can` | 未实现 | 未来候选，不在当前目标内 |
| `nixnet` | `backends::nixnet` | 未实现 | 未来候选，不在当前目标内 |
| `nican` | `backends::nican` | 未实现 | 未来候选，不在当前目标内 |
| `iscan` | `backends::iscan` | 未实现 | 未来候选，不在当前目标内 |
| `ics_neovi` / `neovi` | `backends::ics_neovi` | 未实现 | 未来候选，不在当前目标内 |
| `neousys` | `backends::neousys` | 未实现 | 未来候选，不在当前目标内 |
| `etas` | `backends::etas` | 未实现 | 未来候选，不在当前目标内 |
| `seeedstudio` | `backends::seeedstudio` | 未实现 | 未来候选，不在当前目标内 |
| `cantact` | `backends::cantact` | 未实现 | 未来候选，不在当前目标内 |
| `robotell` | `backends::robotell` | 未实现 | 未来候选，不在当前目标内 |
| `canalystii` | `backends::canalystii` | 未实现 | 未来候选，不在当前目标内 |

后端公开 helper/能力补充：

| python-can 后端 helper | rust-can 对应 | 状态 | 说明 |
| --- | --- | --- | --- |
| `socketcan.build_can_frame` / `dissect_can_frame` | socketcan frame codec | 未实现 | 应独立于 socket IO 测试 |
| `socketcan` BCM helpers | socketcan BCM module | 未实现 | 周期发送低 jitter 路径 |
| `socketcan.utils.pack_filters` | socketcan filter packer | 未实现 | Linux kernel filter 编码 |
| `socketcand.detect_beacon` | socketcand discovery | 未实现 | daemon 自动发现 |
| socketcand ASCII conversion | socketcand codec | 未实现 | 网络协议 codec |
| `udp_multicast.pack_message` / `unpack_message` | udp multicast codec | 未实现 | msgpack feature-gated |
| `vector.get_channel_configs` | vector detect | 未实现 | vendor SDK detect |
| `kvaser.get_channel_info` | kvaser detect | 未实现 | vendor SDK channel info |
| `usb2can.serial_selector` | usb2can detect | 未实现 | serial discovery |
| vendor-specific error classes | backend error mapping | 未实现 | 映射为 typed vendor error + `CanError` |

## rust-can 应该优于 python-can 的架构点

1. 分层解耦

   python-can 把用户 API、配置、后端导入、日志格式和 CLI 行为大量放在同一 Python 包内。rust-can 应保持 crate 边界清晰：core 不依赖 adapter，adapter 不依赖 CLI，IO codec 不依赖文件系统，FFI 不污染安全 Rust API。

2. 静态 fast path 和动态扩展并存

   python-can 天然走动态对象和解释器调用。rust-can 应提供静态泛型后端用于高吞吐场景，同时保留 `dyn CanBus` 和动态插件用于易用性。

3. 后端能力显式化

   python-can 很多能力需要运行时尝试或查文档。rust-can 的 `AdapterInfo` 应声明协议、硬件过滤、fd/handle、周期发送、timestamp source、CAN XL 等能力。

4. 零拷贝日志和消息视图

   python-can `Message.data` 是 `bytearray`。rust-can 应增加 `CanMessageRef<'a>`、streaming codec、缓冲池和 borrowed frame，避免日志解析时反复分配。

5. 可测性能契约

   python-can 重功能兼容，性能不是强契约。rust-can 应把同数据 benchmark、allocation benchmark 和异常列表纳入发布门禁。

6. unsafe 隔离

   vendor SDK 调用不可避免需要 FFI。rust-can 应把 unsafe 限制在后端边界，并通过 typed wrapper、mock 和 smoke test 验证。

7. 背压和队列策略

   python-can 的 listener 队列多是无限队列或线程回调。rust-can 应支持 bounded queue、drop oldest/drop newest/block、丢弃计数和延迟指标。

8. 配置 schema 化

   python-can 配置以 dict/kwargs 为主。rust-can 可以保留灵活 map，但应提供 typed schema、serde 反序列化和后端配置校验。

## 已实测性能结果

测试日期：2026-06-03。机器信息和原始 JSON 位于 `benchmarks/results/2026-06-03/`。

命令：

```powershell
cargo run -p rust-can-benchmarks --release --bin perf_compare -- 1000000
python benchmarks\python\perf_compare.py 1000000
```

| 场景 | Rust ns/iter | python-can ns/iter | 提升 | 状态 |
| --- | ---: | ---: | ---: | --- |
| classic 8B message create | 9.876 | 362.038 | 36.7x | 达到 20x |
| CAN FD 64B message create | 10.713 | 400.237 | 37.4x | 达到 20x |
| 8B message clone/copy | 1.115 | 594.139 | 532.9x | 达到 20x |
| 8B message validate | 0.848 | 186.935 | 220.4x | 达到 20x |
| 4-filter match | 1.792 | 212.592 | 118.6x | 达到 20x |

限制：

- 这些是微基准，只证明当前消息/过滤热路径。
- bus send/recv、Notifier、CLI、FFI、真实硬件后端尚未实现或未建立同数据对照，因此不能宣称达到 20x。ASC 和 rust-can 无压缩 BLF 日志 IO 已达 20x+；python-can zlib 压缩 BLF 仍低于 20x，必须标为异常。
- Rust 当前 benchmark 使用手写 `Instant` harness，Criterion benchmark 仍需要补充同数据对照和 allocation 统计。

真实日志 IO 实测日期：2026-06-06。原始结果位于 `benchmarks/results/2026-06-06/`。

| 场景 | python-can | rust-can | 提升 | 状态 |
| --- | ---: | ---: | ---: | --- |
| ASC 读取，真实大 ASC 前 100,000 条 CAN/CANFD | 275,862 msg/s | 6,746,798 msg/s | 24.46x | 达到 20x |
| BLF 读取，python-can zlib `real_can_canfd_100000.blf` | 580,077 msg/s | 8,616,803 msg/s | 14.85x | 异常：低于 20x |
| BLF 读取，rust-can 无压缩 `rust_can_canfd_100000.blf` | 599,179 msg/s | 53,886,448 msg/s | 89.93x | 达到 20x |

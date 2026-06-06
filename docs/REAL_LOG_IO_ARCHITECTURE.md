# 真实日志 IO 架构与实施计划

本文档定义当前阶段的重点目标：优先实现 ASC/BLF 读写、真实日志测试语料、同数据性能对比，以及可扩展的硬件注入接口边界。硬件后端实现本身不是当前目标。

## 当前目标边界

- P0: ASC reader/writer，必须支持当前 `data/` 真实文件中的 CAN、CANFD、LIN 三类记录。
- P0: 日志事件模型要能预留未来格式，例如 FlexRay、Ethernet、诊断事件、统计事件、厂商自定义 raw event。
- P0: BLF reader/writer，先覆盖 CAN/CANFD 和 error/global marker 基础对象；LIN BLF 需要有真实样本或对象类型确认后进入 P1。
- P0: python-can 作为对照工具时允许针对当前 ASC dialect 修改 reader template，再使用同一真实数据生成 BLF fixture。
- P0: 硬件后端只设计注入接口、能力声明和 mock/virtual conformance tests，不在当前阶段实现 SocketCAN、Vector、Kvaser、PCAN 等具体后端。
- P0: 任何 20x+ 性能结论都必须来自同数据实测；当前 Rust ASC 和无压缩 BLF IO 已达 20x+，python-can zlib 压缩 BLF 仍低于 20x，必须标为性能异常。

## 真实数据语料

`data/` 中 5 个 ZIP 已解压到 `data/extracted/`。共 10 个 ASC 文件，文本总量约 900 MB。

| 类型 | 数量 |
| --- | ---: |
| ASC 文件 | 10 |
| 大 ASC 文件 | 5 |
| 小 ASC 文件 | 5 |
| classic CAN 记录 | 2,384,077 |
| CANFD 记录 | 11,303,173 |
| LIN-like 记录 | 705,176 |

典型 ASC header:

```text
date Fri May 29 09:08:31.774 AM 2026
base hex timestamps absolute
internal events logged
// version 10.0.0
Begin TriggerBlock Fri May 29 09:08:31.774 AM 2026
0.000000 Start of measurement
```

当前真实文件中必须兼容的记录：

```text
0.000080 2 1D1 Rx d 8 00 00 00 00 F8 00 82 D9
0.000000 CANFD 6 637 Rx 0 0 d 10 16 00 20 00 00 00 00 00 00 00 00 00 00 00 00 00 00
0.000030 L11 1 Rx 8 00 4F 3F FF FF C0 FE FE checksum = 00
```

解析规则注意点：

- `base hex` 影响 CAN ID 和 data bytes，但不能机械套用到所有字段。
- 当前 CANFD dialect 的 DLC token 是十进制 code，例如 `10` 表示 CAN FD DLC code 10，对应 16 bytes；不是 python-can writer 的 hex token `a`。
- CANFD 行中必须区分 DLC code 和实际 payload length，例如 `d 12 24`。
- LIN 行必须作为独立事件解析，不能误判为 CAN channel。
- 注释、trigger block、measurement event、未知行要进入 metadata/raw/unknown 事件或按策略跳过，不能导致 streaming parser 失败。

## 事件模型

`CanMessage` 只能表达 CAN/CANFD/CANXL 消息，不应承载 LIN。`rust-can-io` 应引入日志层事件模型：

```rust
#[non_exhaustive]
pub enum LogEvent<'a> {
    Can(CanEvent<'a>),
    CanFd(CanFdEvent<'a>),
    Lin(LinEvent<'a>),
    Metadata(MetadataEvent<'a>),
    Raw(RawEvent<'a>),
    Unknown(UnknownEvent<'a>),
}

#[non_exhaustive]
pub struct LinEvent<'a> {
    pub timestamp_ns: i64,
    pub channel: ChannelRef<'a>,
    pub frame_id: u8,
    pub direction: Direction,
    pub data: PayloadRef<'a>,
    pub checksum: Option<u8>,
}
```

设计要求：

- `LogEvent` 使用 `#[non_exhaustive]`，新增协议不破坏外部匹配。
- payload 优先使用 borrowed slice；需要持久化时再转 owned。
- ASC reader 输出 `LogEvent`；用户只想读取 CAN/CANFD 时可使用 `CanMessage` adapter。
- BLF reader 输出 `LogEvent`；不支持的 BLF object 保留为 `Raw` 或 `Unknown`，用于后续扩展和 roundtrip。
- writer 接受 `LogEventRef`，避免把 LIN 或 future event 强行转换为 CAN。

## rust-can-io 模块规划

| 模块 | 优先级 | 职责 | 当前状态 |
| --- | --- | --- | --- |
| `event` | P0 | `LogEvent`、`CanEvent`、`CanFdEvent`、`LinEvent`、metadata/raw/unknown | 已实现 owned 事件模型，后续优化 borrowed payload |
| `payload` | P0 | borrowed/inline/owned payload，减少分配和拷贝 | 已实现 owned payload，borrowed/inline 是性能优化项 |
| `formats::asc` | P0 | ASC streaming reader/writer，支持 CAN/CANFD/LIN | 已实现并通过真实 ASC 样本测试 |
| `formats::blf` | P0/P1 | BLF block/object reader/writer，先 CAN/CANFD，预留 LIN | 已实现 CAN/CANFD reader 和 writer roundtrip |
| `registry` | P0 | 按扩展名、魔数、压缩后缀选择格式 | 已实现基础扩展名和 BLF 魔数探测 |
| `reader` | P0 | file/stream/source adapter，不依赖具体格式 | 已实现 `LogFormat` 探测基础 |
| `writer` | P0 | sink adapter、flush、append、rotation hook | 已实现 `EventWriter` trait 基础 |
| `bench` | P0 | 同数据 IO benchmark harness | 已实现 `real_log_io` benchmark |
| `logger` | P1 | Logger、rotation、append | 待实现 |
| `player` | P1 | replay timing、gap、skip、loop | 已定义 replay item 基础，完整 player 待实现 |
| TRC/CSV/canutils/SQLite/MF4 | Later | 兼容目标，不是当前重点 | 未实现 |

## 硬件注入接口

硬件对接不是当前目标，但接口必须为后续注入保留。

建议把 `rust-can-adapters` 定义为 SPI，而不是硬件实现集合：

- `CanAdapter`: 最小同步 read/write/close/flush 接口。
- `CanAdapterFactory`: 从 typed config 创建 adapter。
- `AdapterCapabilities`: 声明协议、timestamp source、硬件过滤、poll handle、周期发送、queue/backpressure。
- `AdapterRegistry`: 接收静态注册、动态插件注册、测试注入。
- `MockAdapter`/`VirtualAdapter`: 用于 conformance test 和 benchmark，不等同于真实硬件目标。

能力声明应从 bool 扩展为 schema：

```rust
pub struct AdapterCapabilities {
    pub protocols: ProtocolSet,
    pub filters: FilterCapability,
    pub poll: PollCapability,
    pub cyclic_tx: CyclicTxCapability,
    pub timestamp: TimestampCapability,
    pub max_rx_queue: Option<usize>,
    pub max_tx_queue: Option<usize>,
}
```

## python-can 修改与 BLF fixture

本地 `.external/python-can/can/io/asc.py` 已修改 `ASCReader._process_fd_can_frame`，兼容两种 CANFD 格式：

- python-can writer 格式：`CANFD <channel> <dir> <id> ...`
- 当前真实文件格式：`CANFD <channel> <id> <dir> <brs> <esi> d <dlc> <len> <data...>`

已用修改后的 python-can 直接读取真实 ASC 并生成 BLF：

| fixture | 内容 | 验证 |
| --- | --- | --- |
| `data/generated/real_can_canfd_10000.blf` | 10,000 条 CAN/CANFD，8,514 CANFD + 1,486 classic CAN | `python-can BLFReader` 读回 10,000 条 |
| `data/generated/real_can_canfd_100000.blf` | 100,000 条 CAN/CANFD，85,245 CANFD + 14,755 classic CAN，python-can zlib BLF writer 输出 | `python-can BLFReader` 读回 100,000 条 |
| `data/generated/rust_can_canfd_100000.blf` | 100,000 条 CAN/CANFD，85,245 CANFD + 14,755 classic CAN，rust-can 无压缩 BLF writer 输出 | `python-can BLFReader` 读回 100,000 条 |
| `data/generated/real_lin_1000.jsonl` | 1,000 条 LIN event 样本 | 自定义 LIN event 解析，错误数 0 |
| `data/generated/fixture_stats.json` | fixture 生成统计 | 错误数 0 |

生成命令：

```powershell
python benchmarks\python\prepare_real_log_fixtures.py --limit 10000 --lin-limit 1000
python benchmarks\python\prepare_real_log_fixtures.py --limit 100000 --lin-limit 1000
cargo run --release -p rust-can-benchmarks --bin prepare_rust_blf -- "<ASC path>" "data\generated\rust_can_canfd_100000.blf" 100000
```

## 实测 IO 基线与 rust-can 对比

python-can 和 rust-can 对比测试日期：2026-06-06。结果保存于 `benchmarks/results/2026-06-06/`。

| 场景 | python-can 平均 | rust-can 平均 | 提升 | 状态 |
| --- | ---: | ---: | ---: | --- |
| ASC 读取，真实大 ASC 前 100,000 条 CAN/CANFD | 275,862 msg/s | 6,746,798 msg/s | 24.46x | 达到 20x |
| BLF 读取，python-can zlib `real_can_canfd_100000.blf` | 580,077 msg/s | 8,616,803 msg/s | 14.85x | 异常：低于 20x |
| BLF 读取，rust-can 无压缩 `rust_can_canfd_100000.blf` | 599,179 msg/s | 53,886,448 msg/s | 89.93x | 达到 20x |

Rust 验收目标：

- ASC reader: 同一真实 ASC、同一消息数量，相比 patched python-can 平均 throughput 目前为 24.46x，达标。
- BLF reader: python-can zlib fixture 平均 throughput 目前为 14.85x，低于 20x，标异常；rust-can 无压缩 BLF fixture 平均 throughput 为 89.93x，达标。
- LIN parser: 已有 ASC LIN 单测和真实小 ASC 样本覆盖；`real_lin_1000.jsonl` golden 对照仍可继续扩展。
- 内存: streaming 默认不整文件读入；ASC/BLF 解析热路径仍需加入 allocation count 或 peak RSS 报告。

## 测试覆盖要求

- ASC parser 单测覆盖 header、classic CAN、CANFD、LIN、unknown/raw、注释、trigger block、错误行恢复。
- ASC roundtrip 覆盖 CAN/CANFD/LIN；writer 输出必须可被 rust-can reader 读回。
- BLF reader/writer 覆盖 CAN message、CAN message2、CAN FD message、CAN FD message 64、error/global marker/raw unknown。
- fixture 集成测试使用 `data/generated/real_can_canfd_10000.blf` 和 `data/generated/real_lin_1000.jsonl`。
- coverage 仍以 80%+ 行覆盖率为门禁；2026-06-06 `cargo llvm-cov --workspace --all-features --summary-only --fail-under-lines 80` 已通过，line coverage 81.35%。

## 优先路线

1. P0: 在 `rust-can-io` 增加 `event`、`payload`、`formats::asc` 模块和真实 ASC parser tests。
2. P0: 实现 ASC CAN/CANFD/LIN streaming reader，不整文件读入。
3. P0: 实现 ASC writer 和 CAN/CANFD/LIN roundtrip。
4. P0: 建立 ASC vs patched python-can 同数据 benchmark，记录 allocation/throughput。
5. P0/P1: 实现 BLF CAN/CANFD reader/writer，使用已生成 BLF fixture 验证。
6. P1: BLF LIN object 支持，等待真实 LIN BLF 样本或对象类型确认。
7. P1: CLI `logconvert`/`player` 先围绕 ASC/BLF，不引入真实硬件依赖。
8. Later: TRC、CSV、canutils、SQLite、MF4。

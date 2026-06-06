# rust-can

rust-can 是一个早期 Rust CAN 工具链 workspace。当前重点是 ASC/BLF 日志 IO、真实日志测试语料、同数据性能对比，以及可扩展的硬件注入接口。

当前状态：架构和早期实现阶段。它还不是 python-can 的直接替代品；核心消息/过滤、virtual adapter、Notifier 原型、ASC CAN/CANFD/LIN reader/writer、BLF CAN/CANFD reader/writer 和真实日志 benchmark 已实现。CLI 命令族、C FFI、Python bindings 和真实硬件后端仍未完成。真实硬件后端实现不是当前阶段目标，只保留 adapter injection 接口。

## 项目目标

- 覆盖 python-can 核心能力：message、bus、filters、notifier、listeners、cyclic send tasks、logging、replay、bridge、viewer 和 backend discovery。
- 支持 CAN 2.0、CAN FD、CAN FD non-ISO 和 CAN XL。
- 当前优先支持 ASC 中的 CAN、CANFD、LIN 三类记录，并为未来新增格式预留 `LogEvent` 扩展模型。
- 为 virtual/mock 和未来硬件实现提供可注入 adapter SPI；SocketCAN、serial/slcan、UDP multicast、gs_usb、socketcand 和 vendor SDK 后端是后续候选，不是当前实现目标。
- 只在同数据、同机器、同场景 benchmark 已证明时，才声明关键路径相对 python-can 达到 20x+。
- 在 classic CAN 和 CAN FD 消息创建热路径避免堆分配，并优先使用零拷贝或单拷贝设计。
- 在 coverage harness 可用时维持 80%+ 行覆盖率。

## 当前状态

已实现或部分实现：

- `CanMessage`：classic CAN、CAN FD owned 消息类型；CAN XL payload 保留完整数据并修正了长 payload 截断问题。
- `CanFrame` 原始帧类型。
- `CanFilter` 和 `CanFilters`。
- `CanProtocol`、`BusState` 和基础 `CanError`。
- `BitTiming` 和 `BitTimingFd` 的基础结构。
- `CanBus`、`CyclicTask`、`CanAdapter` trait 雏形。
- `AdapterConfig`、`AdapterInfo`、adapter registry 和 virtual backend 原型。
- `Listener`、`BufferedReader` 和 `Notifier` 原型。
- `rust-can-io::event` 日志事件模型，支持 CAN、CANFD、LIN、metadata/raw/unknown 扩展。
- ASC reader/writer：支持当前真实 ASC 中的 CAN、CANFD、LIN，含 streaming limit/visitor API 和 roundtrip 测试。
- BLF reader/writer：支持 CAN/CANFD fixture 读取和 CAN/CANFD 写入 roundtrip。
- 消息和过滤微基准、真实 ASC/BLF IO benchmark，以及同数据 python-can 对比结果。

未实现或未验证：

- `Bus()` 工厂、配置文件/环境变量加载、后端 autodetect。
- 无 `async_trait` 的 bus fast path；当前 bus send/recv 尚未建立 20x 同数据实测。
- 完整软件过滤兜底、迭代、上下文管理和完整周期发送语义。
- `ThreadSafeBus`、`RedirectReader`、`AsyncBufferedReader`。
- Notifier registry、fd/handle reactor、async callback 管理；Notifier 性能未实测。
- TRC、CSV、canutils、MF4、SQLite 是后续兼容目标。
- `dump`、`logger`、`player`、`bridge`、`logconvert`、`viewer`、`detect` 等 CLI 命令。
- C FFI 和 Python bindings。
- 真实硬件后端实现；当前只规划注入接口、能力声明和 mock/virtual conformance tests。
- 80%+ coverage 门禁已建立；2026-06-06 `cargo llvm-cov --workspace --all-features --summary-only --fail-under-lines 80` 实测行覆盖率 81.35%，已达标。
- 完整 allocation 统计。

## Workspace Crates

- `rust-can-core`：核心类型、错误、过滤、bus trait、bit timing、listener 和 cyclic 基础。
- `rust-can-adapters`：后端适配接口、配置、注册表和 virtual backend。
- `rust-can-io`：日志事件模型、ASC CAN/CANFD/LIN reader/writer、BLF CAN/CANFD reader/writer、格式探测。
- `rust-can-notifier`：多 bus listener 分发；当前为原型，缺 registry、fd/handle reactor 和 async callback 管理。
- `rust-can-cli`：用户命令行工具入口；命令族仍需实现。
- `rust-can-ffi`：C ABI 目标 crate；目前仅导出版本信息。
- `benchmarks`：Criterion benchmark、消息/过滤对比、真实 ASC/BLF IO 对比 harness；bus 对比不完整。

设计文档见 [docs/design/](docs/design/)，测试报告见 [docs/test/](docs/test/)。完整架构见 [docs/design/overview.md](docs/design/overview.md)，真实日志 IO 计划见 [docs/design/real-log-io.md](docs/design/real-log-io.md)，python-can 功能矩阵见 [docs/design/python-can-compatibility.md](docs/design/python-can-compatibility.md)。各模块详细设计见 [docs/design/details/](docs/design/details/)，模块测试报告见 [docs/test/details/](docs/test/details/)。

Git 分支工作流见 [AGENTS.md](AGENTS.md#git-工作流)。

## 真实日志数据状态

- `data/` 中 5 个 ZIP 已解压到 `data/extracted/`，共 10 个 ASC 文件，约 900 MB 文本。
- 真实语料统计：2,384,077 条 classic CAN、11,303,173 条 CANFD、705,176 条 LIN-like 记录。
- 本地 `.external/python-can` 的 `ASCReader` 已调整，可读取当前 ASC 的 CANFD 格式。
- 已生成 `data/generated/real_can_canfd_10000.blf` 和 `data/generated/real_can_canfd_100000.blf`，均来自真实 ASC，python-can `BLFReader` 可读回对应 CAN/CANFD 消息。
- 已生成 `data/generated/rust_can_canfd_100000.blf`，由 rust-can BLF writer 写出，python-can 已验证读回 100,000 条、85,245 CANFD + 14,755 classic CAN。
- 已生成 `data/generated/real_lin_1000.jsonl`，包含 1,000 条 LIN event 样本。

## 性能实测摘要

只允许对下表场景声明 20x+ 已达标。bus、Notifier、CLI、FFI 和 hardware adapter 未建立同数据对照，必须视为未验证。真实日志 IO 中 ASC 和无压缩 BLF 已达 20x+；python-can 生成的 zlib 压缩 BLF 仍低于 20x，必须标为异常。

测试日期：2026-06-03。原始结果位于 `benchmarks/results/2026-06-03/`。

环境摘要：

- OS: Microsoft Windows 10.0.26200, X64
- Rust: `rustc 1.96.0 (ac68faa20 2026-05-25)`, `cargo 1.96.0 (30a34c682 2026-05-25)`
- Python: `Python 3.14.3`
- python-can: `491a691fd1faffab1c48956bafd711e7c653db54`
- Iterations: `1000000`

命令：

```powershell
cargo run -p rust-can-benchmarks --release --bin perf_compare -- 1000000
python benchmarks\python\perf_compare.py 1000000
cargo bench -p rust-can-benchmarks --bench message_bench
python -m pytest benchmarks\python\test_python_can_benchmark.py --benchmark-only --benchmark-json=benchmarks\results\2026-06-03\pytest-benchmark-python.json
cargo llvm-cov --workspace --all-features --summary-only
```

| 场景 | Rust ns/iter | python-can ns/iter | 提升 | 状态 |
| --- | ---: | ---: | ---: | --- |
| classic 8B message create | 9.876 | 362.038 | 36.7x | 达到 20x |
| CAN FD 64B message create | 10.713 | 400.237 | 37.4x | 达到 20x |
| 8B message clone/copy | 1.115 | 594.139 | 532.9x | 达到 20x |
| 8B message validate | 0.848 | 186.935 | 220.4x | 达到 20x |
| 4-filter match | 1.792 | 212.592 | 118.6x | 达到 20x |

限制：

- 这些是消息和过滤热路径微基准，不代表整个项目达到 20x。
- Rust 已运行 Criterion，Python 已运行 pytest-benchmark；同数据倍率表来自 `perf_compare` harness。
- allocation 统计仍需补充。
- 未实测场景必须写“未验证”，未实现功能必须写“未实现”。

真实日志 IO 对比：

测试日期：2026-06-06。原始结果位于 `benchmarks/results/2026-06-06/`。

| 场景 | python-can | rust-can | 提升 | 状态 |
| --- | ---: | ---: | ---: | --- |
| ASC 读取，真实大 ASC 前 100,000 条 CAN/CANFD | 275,862 msg/s | 6,746,798 msg/s | 24.46x | 达到 20x |
| BLF 读取，python-can zlib `real_can_canfd_100000.blf` | 580,077 msg/s | 8,616,803 msg/s | 14.85x | 异常：低于 20x |
| BLF 读取，rust-can 无压缩 `rust_can_canfd_100000.blf` | 599,179 msg/s | 53,886,448 msg/s | 89.93x | 达到 20x |

覆盖率实测：

| 指标 | 当前值 | 目标 | 状态 |
| --- | ---: | ---: | --- |
| Region coverage | 80.83% | 80%+ | 达标 |
| Function coverage | 80.99% | 80%+ | 达标 |
| Line coverage | 81.35% | 80%+ | 达标 |

## 开发、测试和 Benchmark

运行 workspace 测试：

```powershell
cargo test --workspace --all-features
```

运行 Rust benchmark：

```powershell
cargo bench --workspace
```

运行当前同数据性能对比：

```powershell
cargo run -p rust-can-benchmarks --release --bin perf_compare -- 1000000
python benchmarks\python\perf_compare.py 1000000
cargo run --release -p rust-can-benchmarks --bin real_log_io -- "<ASC path>" "data\generated\real_can_canfd_100000.blf" 100000 5
cargo run --release -p rust-can-benchmarks --bin prepare_rust_blf -- "<ASC path>" "data\generated\rust_can_canfd_100000.blf" 100000
cargo run --release -p rust-can-benchmarks --bin real_log_io -- "<ASC path>" "data\generated\rust_can_canfd_100000.blf" 100000 5
```

运行 Python 性能工具基准：

```powershell
python -m pytest benchmarks\python\test_python_can_benchmark.py --benchmark-only --benchmark-json=benchmarks\results\2026-06-03\pytest-benchmark-python.json
```

覆盖率目标：

```powershell
cargo llvm-cov --workspace --all-features --fail-under-lines 80
```

如果没有安装 `cargo llvm-cov`：

```powershell
cargo install cargo-llvm-cov
```

性能相关改动必须：

- 使用同一份生成数据喂给 Rust 和 python-can。
- 记录 Rust 与 Python 输出、机器信息、工具版本。
- 将结果保存到 `benchmarks/results/YYYY-MM-DD/`。
- 对 allocation-sensitive 场景记录分配次数或内存流量。
- 低于 20x 的目标场景标为例外，不调整声明口径。

## 上游 python-can 参考提交

架构分析和兼容矩阵使用的上游源码来自 [hardbyte/python-can](https://github.com/hardbyte/python-can)，commit `491a691fd1faffab1c48956bafd711e7c653db54`。

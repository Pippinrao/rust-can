# rust-can-io 模块设计

> Test report: [../../test/details/io.md](../../test/details/io.md)

## 架构设计

`rust-can-io` 负责日志格式读写，对应 python-can `can.io.*`、`LogReader`、`Logger` 的数据面。

```
  ASC/BLF file
       │
       ▼
┌──────────────┐    ┌─────────────┐    ┌──────────────┐
│ reader       │───▶│ formats     │───▶│ LogEvent     │
│ (detect)     │    │ asc / blf   │    │ Can/Lin/Raw  │
└──────────────┘    └─────────────┘    └──────────────┘
       ▲
┌──────────────┐
│ writer       │
└──────────────┘
```

`LogEvent` 枚举将 LIN、metadata、unknown 与 CAN 分离，避免强行塞入 `CanMessage`。

ASC：streaming parser，支持 CAN/CANFD/LIN、visitor/limit API、`scan_can_stats` 零分配计数路径。

BLF：CAN/CANFD object 读写；兼容 python-can 生成 fixture 与 rust-can 无压缩 fixture。

## 接口设计

| 模块 | 主要 API | python-can 对应 |
| --- | --- | --- |
| `event` | `LogEvent`, `CanEvent`, `LinEvent`, … | 扩展模型 |
| `formats::asc` | `AscReader`, `AscWriter`, `scan_can_stats`, `collect_*` | `ASCReader`/`ASCWriter` |
| `formats::blf` | `BlfReader`, `BlfWriter`, `scan_can_stats` | `BLFReader`/`BLFWriter` |
| `reader` | 扩展名/魔数探测 | `MESSAGE_READERS` |
| `writer` | writer trait | `MESSAGE_WRITERS` |
| `player` | replay timing（雏形） | `MessageSync` |

**未实现**：TRC、CSV、MF4、SQLite、rotating logger、完整 reader/writer registry。

## DFX 设计

| 维度 | 设计 |
| --- | --- |
| 可靠性 | 未知行 → `UnknownEvent` 或 skip；坏 payload token 在 scan 路径报告错误 |
| 可维护性 | ASC/BLF 分文件；真实语料驱动测试（`data/extracted/`） |
| 可测试性 | 24 个单元测试 + roundtrip + 真实最小 ASC + python-can BLF fixture |
| 可观测性 | scan API 可统计无效行；无结构化 trace |
| 可扩展性 | `LogEvent` non-exhaustive；新格式加 `formats::*` |

**风险**：python-can zlib BLF 解压路径性能低于 20x；LIN BLF 缺样本。

## 性能指标

真实数据 benchmark（2026-06-06，`data/extracted/` ASC + `data/generated/` BLF，100k CAN/CANFD）：

| 场景 | python-can | rust-can | 提升 | 20x | 100x |
| --- | ---: | ---: | ---: | --- | --- |
| ASC read 100k | 275,862 msg/s | 6,746,798 msg/s | **24.46x** | 达标 | 未达 |
| BLF zlib (python fixture) | 580,077 msg/s | 8,616,803 msg/s | **14.85x** | **异常** | 未达 |
| BLF 无压缩 (rust fixture) | 599,179 msg/s | 53,886,448 msg/s | **89.93x** | 达标 | 未达 |

ASC `scan_can_stats` 为零存储热路径，是对标 python-can reader 的主要优化点。

功能对标：ASC CAN/CANFD/LIN **已实现**；BLF CAN/CANFD **已实现**；Logger/Player orchestration **待实现**。

覆盖率：asc 87%、blf 81%、event 92%、reader 90%。

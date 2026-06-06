# rust-can-cli 模块设计

> Test report: [../../test/details/cli.md](../../test/details/cli.md)

## 架构设计

`rust-can-cli` 对应 python-can CLI 工具族：`can.logger`、`can.player`、`can.viewer`、`can.bridge`、`can.logconvert` 及 `can.cli` 共享参数。

当前仅有 **命令入口骨架**（`main.rs`），无子命令实现。

```
cancli (planned)
 ├── logger
 ├── player
 ├── bridge
 ├── logconvert
 ├── viewer
 └── detect
```

## 接口设计

| 命令 | python-can | 状态 |
| --- | --- | --- |
| `logger` | `can.logger` | 未实现 |
| `player` | `can.player` / `LogReader` | 未实现 |
| `bridge` | `can.bridge` | 未实现 |
| `logconvert` | `can.logconvert` | 未实现 |
| `viewer` | TUI viewer | 未实现 |
| `detect` | `detect_available_configs` | 未实现 |
| 共享 bus args | `add_bus_arguments` | 未实现 |

依赖规划：`rust-can-io`（读写）、`rust-can-adapters`（bus）、`ratatui`（viewer）。

## DFX 设计

| 维度 | 设计 |
| --- | --- |
| 可靠性 | CLI 应对 IO/adapter 错误给出明确 exit code |
| 可维护性 | clap derive；共享 `BusArgs` 模块 |
| 可测试性 | 计划 integration test + fixture 子进程 |
| 可观测性 | tracing-subscriber 集成（未实现） |
| 可扩展性 | 子命令模块化 |

## 性能指标

**未验证**。CLI 非当前 benchmark 重点。

覆盖率：**0%**（main 无测试）。

功能对标：**未实现**。

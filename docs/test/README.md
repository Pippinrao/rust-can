# 测试与性能报告

[English index](../test/en/README.md)

本目录存放测试执行摘要与性能报告。模块级详细报告见 [details/](details/)。

| 文档 | 说明 |
| --- | --- |
| [REQUIREMENTS.md](REQUIREMENTS.md) | **测试覆盖要求权威**（UT / E2E / Perf / 报告规则 ID） |
| [e2e-registry.md](e2e-registry.md) | E2E 场景登记表 |
| [perf-registry.md](perf-registry.md) | 性能场景登记表 |
| [TEMPLATE.md](TEMPLATE.md) | 模块测试报告标准模板（复制到 `details/<module>.md`） |
| [AUDIT.md](AUDIT.md) | 测试合规审计（覆盖率 / E2E / 性能 / 模板） |
| [real-log-io-report.zh-CN.md](real-log-io-report.zh-CN.md) | 2026-06-06 真实 ASC/BLF IO 对比（中文） |
| [real-log-io-report.md](real-log-io-report.md) | 同上（英文原始报告） |
| [details/core.md](details/core.md) | `rust-can-core` 测试报告 |
| [details/adapters.md](details/adapters.md) | `rust-can-adapters` 测试报告 |
| [details/io.md](details/io.md) | `rust-can-io` 测试报告 |
| [details/notifier.md](details/notifier.md) | `rust-can-notifier` 测试报告 |
| [details/cli.md](details/cli.md) | `rust-can-cli` 测试报告 |
| [details/ffi.md](details/ffi.md) | `rust-can-ffi` 测试报告 |
| [details/benchmarks.md](details/benchmarks.md) | benchmark harness 测试报告 |

原始 JSON/txt 数据位于 `benchmarks/results/YYYY-MM-DD/`。

## 报告生成

```powershell
.\scripts\generate_test_report.ps1 -Module core
```

按 [TEMPLATE.md](TEMPLATE.md) 输出 skeleton；人工补全 E2E/Perf 章节后写入 `details/<module>.md`。

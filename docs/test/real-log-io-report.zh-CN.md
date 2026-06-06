# 真实日志 IO 性能报告

日期：2026-06-06  
English: [real-log-io-report.md](real-log-io-report.md)

## 范围

在同一份真实日志数据集上对比 rust-can 与本地 patch 版 python-can。

- ASC 源：`data/extracted/.../CDC_VHR_LZ4CCAN_1_20260529091108_20260529091332(UTC+8)_58143.asc`
- ASC 限制：前 100,000 条 CAN/CANFD
- python-can zlib BLF：`data/generated/real_can_canfd_100000.blf`
- rust-can 无压缩 BLF：`data/generated/rust_can_canfd_100000.blf`
- 消息构成：85,245 CANFD + 14,755 classic CAN
- 每工具运行 5 次

## 结果

| 场景 | python-can 均值 | rust-can 均值 | 倍率 | 20x | 100x |
| --- | ---: | ---: | ---: | --- | --- |
| ASC 读取 100k | 275,862 msg/s | 6,746,798 msg/s | **24.46x** | 达标 | 否 |
| BLF zlib | 580,077 msg/s | 8,616,803 msg/s | **14.85x** | **异常** | 否 |
| BLF 无压缩 | 599,179 msg/s | 53,886,448 msg/s | **89.93x** | 达标 | 否 |

## 结论

- ASC fast scan 达标 20x。
- rust-can 无压缩 BLF 接近 100x，且与 python-can 对象布局兼容。
- python-can zlib BLF 为**文档化性能异常**，优化解压路径前不得宣称达标。

原始 JSON：`benchmarks/results/2026-06-06/`。模块报告：[details/io.md](details/io.md)

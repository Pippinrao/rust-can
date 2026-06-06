# rust-can-ffi 测试报告

> 设计文档：[../../design/details/ffi.md](../../design/details/ffi.md)

## 测试范围与环境

- Crate：`rust-can-ffi`
- 当前仅版本导出

## 单元/集成测试执行结果

| 指标 | 结果 |
| --- | --- |
| 测试数 | 0 |

覆盖率：**0%**。

## E2E 测试

未适用（C API 未实现）。

## 性能测试

未适用。

## 与 python-can 功能/性能差距

`can.ctypesutil` 与 vendor FFI：**未实现**。

## 结论与后续行动

- 定义 C API surface 后添加 cbindgen + C harness 测试。

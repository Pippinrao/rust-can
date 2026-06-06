# rust-can-ffi 模块设计

> Test report: [../../test/details/ffi.md](../../test/details/ffi.md)

## 架构设计

`rust-can-ffi` 对应 python-can `can.ctypesutil` 与 vendor SDK C ABI 封装目标。

当前仅导出 **版本信息** stub，无 bus/message/IO 的 C API。

```
C caller ──▶ rust-can-ffi ──▶ (planned) rust-can-core / adapters
```

## 接口设计

| 导出 | 状态 |
| --- | --- |
| 版本字符串 | 已实现 |
| `can_open` / `can_send` / `can_recv` | 未实现 |
| 错误码映射 | 未实现 |
| BLF/ASC C API | 未实现 |

## DFX 设计

| 维度 | 设计 |
| --- | --- |
| 可靠性 | C ABI 需明确 ownership/lifetime 文档 |
| 可维护性 | unsafe 隔离在 ffi 边界 |
| 可测试性 | 计划 cbindgen + C harness |
| 可观测性 | N/A |
| 可扩展性 | stable C header + semver |

## 性能指标

**未验证**。

功能对标：**未实现**（除版本信息）。

覆盖率：**0%**。

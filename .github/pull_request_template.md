## Summary

<!-- 简要说明变更目的 -->

## Test plan

- [ ] UT-01：`cargo test --workspace --all-features`
- [ ] UT-01：`cargo llvm-cov --workspace --all-features --fail-under-lines 80`
- [ ] UT-03：未降低 workspace 行覆盖率
- [ ] E2E-01/02：相关 E2E 场景已跑，[e2e-registry.md](docs/test/e2e-registry.md) 已更新
- [ ] PERF-02：性能变更已归档至 `benchmarks/results/YYYY-MM-DD/`
- [ ] RPT-01/02：模块测试报告 `docs/test/details/*.md` 已更新

## 规范引用

详见 [docs/test/REQUIREMENTS.md](docs/test/REQUIREMENTS.md)。

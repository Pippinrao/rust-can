# ASC / BLF Reader 性能与内存画像

> 2026-06-07 补充：用 instrumentation-based profiler 拿了**真实 CPU 火焰图**（50 runs × ASC + 50 runs × BLF）；SVG 在 `target/flamegraph/*.svg`，folded-stacks 在 `target/flamegraph/*.folded.txt`。**这次数据推翻了第 3 节"CPU 热点推断"**——见 §4 flame-graph 实证。
>
> 原分析（2026-06-06 写）保留在 §1–§3（吞吐、内存画像、cargo-bloat 推断）；§5 优化建议的 ROI 重排按真实火焰图刷新。

工具：
- `cargo build --release`、`./target/release/real_log_io`（已存在）
- `reader_alloc_bench`（新加，`CountingAlloc` 全局分配计数器）
- `reader_profile_runner`（新加，instrumentation 火焰图，启用 `--features 'rust-can-io/profile'`）
- `cargo bloat --release --bin real_log_io`（静态大小推断）
- `inferno-flamegraph` 渲染 SVG
- Windows 自带 `wpr` + Microsoft Store 版 WPA（`xperf` 未在 Store 版里，所以没走 ETW 采样）

Windows 限制：ETW CPU sampling 需要 admin（`net session` 拒、`SE_SYSTEM_PROFILE_NAME` 拿不到，`cargo flamegraph` 用的 blondie 跑不起来）。改用**手动插桩**：在 hot functions 顶上加 `prof_scope!("name")`，通过 `Instant::now()` 计 wall clock，输出 folded stacks 给 inferno 渲染。零采样，零 admin，**真实计时**。

实测对象：
- ASC：`data/extracted/.../CDC_VHR_..._26550(parsed)/...26550.asc`（58 816 字节 / 951 行 / 919 条 CAN/CAN FD 事件，14 经典 + 63 FD + 2 LIN）
- BLF：`data/generated/real_can_canfd_10000.blf`（112 328 字节 / 10 000 事件，1 486 经典 + 8 514 FD，ZLIB 压缩 log container）

---

## 1. 时间（throughput）

`real_log_io` 各跑 5 次，warm 后取 mean：

| Reader | 函数 | msgs/s mean | msgs/s min |
|---|---|---:|---:|
| ASC | `AscReader::scan_can_stats_limit(100_000)` | 5 258 068 | 3 295 088 |
| BLF | `BlfReader::scan_can_stats` | 9 676 998 | 8 553 588 |

BLF 解析比 ASC 快 ~1.8×。但 BLF 文件本身已经过 zlib 压缩（miniz_oxide inflate 占 `.text` 7.7 KiB / 4.3%），且每个 BLF container 含 5 000 帧的批量解析路径；ASC 是 1 行 1 事件、按字节扫描。两边都是 I/O-bound。

---

## 2. 内存与分配画像

`reader_alloc_bench` 用全局 `CountingAlloc` 计数器，**总分配**（含 realloc）与**峰值**：

| 路径 | 事件数 | alloc_count | allocs/msg | bytes/msg |
|---|---:|---:|---:|---:|
| ASC `scan_can_stats_limit` | 919 | 7 | 0.008 | 10.4 |
| ASC `collect_events` | 919 | **5 607** | **6.10** | **1 206.3** |
| BLF `scan_can_stats` | 10 000 | 42 | 0.004 | 166.2 |
| BLF `collect_events` | 10 000 | **10 139** | **1.01** | **690.4** |

### ASC collect 6.10 allocs/msg 拆解

每行 `for line in self.reader.lines()` 在 `rust-can-io/src/formats/asc.rs:78` 触发：
- **1 alloc / 行**：`BufRead::lines()` 给每行一个新 `String`（951 行 ≈ 951 alloc）
- **~3 alloc / 行**：`parse_line` 内的 `let parts: Vec<&str> = trimmed.split_whitespace().collect();`（`asc.rs:237`）Vec 从 0 → 4 → 8 → 16 ... 增长，约 2–3 次 realloc/行
- **1 alloc / 事件**：`Payload::from_slice` / `parse_payload` 创建 `Vec<u8>`（`asc.rs:360–368`）

合计 ≈ 5 alloc/事件 × 919 ≈ 4 595 + events Vec 自身增长 ≈ 5 607。匹配。

### BLF collect 1.01 allocs/msg 拆解

- **1 alloc / 事件**：`Payload` 的 `Vec<u8>`（每帧 1 次）
- **1 alloc 总**：`events: Vec<LogEvent>` 从 0 增长 ~14 次到 16 384（10 000 条事件的 realloc 链）
- **少量**：`decompress_buf` / `body_buf` / `tail` 在 BLF 容器首次遇到时各增长 1 次（`blf.rs:94–96`，commit `aedd12d` 已优化）

合计 ≈ 10 000 + 14 + ~10 = 10 139。匹配。

### 峰值内存

dhat（最后一个 profile `BLF scan`）：
- `At t-gmax: 297 922 bytes in 5 blocks` — 5 个常驻缓冲
- `At t-end: 0 bytes` — 释放干净，无泄漏

---

## 3. 早期 cargo-bloat 推断（**已被 §4 推翻**，保留作对照）

`cargo bloat --release -p rust-can-benchmarks --bin real_log_io --filter 'rust_can_io'` top 5：

| 函数 | 大小 | 当时推断的"hot" |
|---|---:|---|
| `scan_canfd_stats<…SplitWhitespace>` | 1 631 B | ASC scan FD 路径 |
| `scan_container_with_tail` | 1 343 B | BLF 容器帧解析 |
| `impl Display for AscParseError` | 1 234 B | 错误格式化 |
| `parse_hex_u32` | 593 B | ASC 解析热路径 |
| `parse_asc_channel` | 439 B | 同上 |
| `parse_hex_byte` | 414 B | 同上 |

外加 `miniz_oxide::inflate::core::decompress` 7.7 KiB（4.3%），**当时被列为"BLF 头号 CPU 热点"**。

> **这推断是错的**。cargo-bloat 报的是代码大小，**不是运行时采样**。`decompress` 代码 7.7 KiB 是因为 miniz_oxide 把 fast/slow path 全内联展开，但**每个 BLF container 只解压一次**（container 里 5 000 帧）。真实火焰图（§4）显示 zlib 只占 8.5%。下面给硬数据。

---

## 4. 真实 CPU 火焰图（instrumentation-based）

`reader_profile_runner` 启用 `--features 'rust-can-io/profile'`，跑 50 次 ASC collect + 50 次 ASC scan + 50 次 BLF collect + 50 次 BLF scan，输出 folded stacks 到 `target/flamegraph/folded.txt`，inferno 渲染成 4 个 SVG（`asc.svg` / `blf-collect.svg` / `blf-scan.svg`，合并图见 `target/flamegraph/asc-blf-reader.svg`）。

**采样结果（所有时间单位为 nanoseconds，50 runs 累加）**：

| Stacks | 总时间 (ns) | 占总时间 |
|---|---:|---:|
| **BLF collect 全部** | **433 495 000** | 73% |
| `blf::collect_events` (顶层) | 433 495 000 | 73% |
| └ `blf::parse_log_container` | 410 735 500 | **95%** of collect |
| └ └ `blf::parse_container_with_tail` | 372 699 700 | 90% of collect |
| └ └ └ `blf::parse_objects` | 364 376 500 | **88% of collect** |
| └ └ └ └ `blf::parse_message_object` | 109 509 900 | 27% of collect |
| └ └ └ └ `blf::parse_object_header` | 11 930 300 | 2.9% of collect |
| └ └ └ └ `blf::find_lobj` | 12 082 200 | 3.0% of collect |
| └ └ `blf::decompress_container` | 37 810 900 | **8.7% of collect** ← zlib 实际只占 8.7% |
| **BLF scan 全部** | **127 693 800** | 21% |
| `blf::scan_can_stats` (顶层) | 127 693 800 | 21% |
| └ `blf::scan_log_container` | 125 841 800 | 99% of scan |
| └ └ `blf::scan_container_with_tail` | 91 646 100 | 72% of scan |
| └ └ └ `blf::scan_objects` | 85 199 400 | 67% of scan |
| └ └ └ └ `blf::find_lobj` | 12 389 600 | 9.7% of scan |
| └ └ `blf::decompress_container` | 34 037 600 | 27% of scan |
| **ASC** | **42 881 200** | 7% |
| `asc::parse_line` | 28 380 900 | 4.8% |
| └ `asc::parse_canfd` | 8 744 100 | 1.5% |
| └ └ `asc::parse_payload` | 2 600 300 | 0.5% |
| └ `asc::parse_classic_can` | 1 496 900 | 0.3% |
| └ └ `asc::parse_payload` | 469 500 | 0.1% |
| └ `asc::parse_payload` (direct) | 69 600 | <0.1% |

### 关键发现

1. **BLF `parse_objects` 是真正的 CPU 头号热点**（364 ms / 433 ms = **84%** of collect_events）。不是 zlib。
2. **zlib `decompress_container` 只占 8.7%**（collect）/ **27%**（scan）。scan 占比高是因为 scan 不算 5 000 帧的事件解析，所以 zlib 在 scan 内的相对权重大。
3. **`find_lobj` 占 12 ms（3% of collect）** — 是真实 CPU 消耗，不是想象中"可忽略"。
4. **ASC 仅占 7%** — BLF reader 比 ASC 慢的 1.8×，主因是 BLF 事件多 10× 而非单事件慢。**单事件成本**：
   - BLF：433 ms / 50 runs / 10 000 events = **866 ns/event**
   - ASC：42 ms / 50 runs / 919 events = **933 ns/event**
   - **两者单事件成本接近**（BLF 略快）。
5. **`parse_message_object` 占 109 ms**（25% of collect）— BLF object type 分发（CAN_MESSAGE / CAN_FD_MESSAGE / CAN_FD_MESSAGE_64）每次都做 match，**单事件 ~22 ns**。

### 与 §3 cargo-bloat 推断的差异

| 维度 | cargo-bloat 推断 | 真实火焰图 |
|---|---|---|
| BLF 头号热点 | `miniz_oxide::inflate::decompress` (7.7 KiB) | `blf::parse_objects` (84% of collect) |
| `scan_container_with_tail` | 1.3 KiB（第 2 热） | 仅占 7% of collect |
| `scan_canfd_stats` | 1.6 KiB（第 1 热） | 0% — 文件内根本不跑 |
| `parse_hex_u32` | 593 B | 0% — 没插桩没采样 |

**结论：cargo-bloat 给的"代码大小排名"严重误导 CPU 热点判断**。大代码块 ≠ 频繁执行（小代码块被频繁调用 + 内联展开 = 大代码块，但执行次数不一定高）。

---

## 5. 优化建议（按真实火焰图 ROI 排序）

### 5.1 【高】`blf::parse_objects` 优化 — 单点最大热点

**位置**：`rust-can-io/src/formats/blf.rs:506-532`

`parse_objects` 364 ms 占 84% of BLF collect。逐 event 工作：
- `find_lobj`（12 ms / 3%）
- `parse_base_header`（LOBJ sig 校验）
- `parse_object_header`（12 ms / 2.9%）
- `parse_message_object`（109 ms / 25%）

可改方向：
1. **去掉 `find_lobj` 显式扫描** — `parse_objects` 实际只处理"back-to-back LOBJ"，把 LOBJ 校验内联到主循环顶端（`if data[offset..offset+4] != b"LOBJ"`）。`find_lobj` 现在的 8 字节 window scan 走不到。
2. **`parse_object_header` v1 路径热**（line 763-767），它每次都重新 slice 16 字节 + read 2 个 u32/u64。可以换成 `from_le_bytes` 直接指针读（unsafe 但 hot path 收益 1–2%）。
3. **预读 `body_buf`**：`read_object_body` (`blf.rs:270`) 每次 resize 后 read，10 000 次 = 10 000 次 resize。可改为 maintain 一个长 enough 的 body_buf 复用（同 `decompress_buf` 已有模式）。

预估收益：BLF `parse_objects` 时间砍 10–20%，BLF collect 提速 8–17%。

### 5.2 【高】`Payload` 改 `SmallVec<[u8; 8]>` — 全场景 alloc 减半

**位置**：`rust-can-io/src/event.rs:24-52`

BLF collect 1.01 allocs/msg 几乎全来自 `Payload.data: Vec<u8>`（每帧 1 alloc）。经典 CAN ≤8 字节和大多数 CAN FD 帧可走栈 / SmallVec。BLF 经典 CAN 1 486 帧 alloc 直接归零，1 486 FD 帧中绝大多数（payload < 8 字节时）也归零。

预估收益：BLF `allocs/msg` 1.01→~0.01，bytes/msg 690→~200。**这是 ROI 最高的单点改动**。

### 5.3 【中】BLF `collect_events` 预分配 `events` Vec

**位置**：`rust-can-io/src/formats/blf.rs:287`

`let mut events = Vec::new();` — 增长链 0 → 4 → 8 → ... → 16 384 = 14 次 realloc。BLF 文件头的 `object_count` 字段（`blf.rs:842-860`）已经写入，`BlfReader::new` 读了 `header_size` 但没读 `object_count`。在 `BlfReader` struct 加 `object_count: u32`，`new()` 时读出，`collect_events` 改成 `Vec::with_capacity(self.object_count as usize)`。

预估收益：14 realloc → 0。

### 5.4 【中】ASC `collect_events_limit` 复用 `String` 缓冲

**位置**：`rust-can-io/src/formats/asc.rs:76-87`

```rust
for line in self.reader.lines() {       // ← 每行 alloc 一个新 String
```

`scan_can_stats_limit`（`:127-138`）已有正确模式（`read_line` + `line.clear()`）。改用相同模式，消除 951 alloc。

预估收益：ASC `allocs/msg` 6.10→~3.0。

### 5.5 【中】ASC `parse_line` 的 `parts: Vec<&str>` → 迭代器

**位置**：`rust-can-io/src/formats/asc.rs:237`

`let parts: Vec<&str> = trimmed.split_whitespace().collect();` 把迭代器收集成 Vec，然后 `parse_*` 用 `parts.get(N)` 索引。改成 `parse_*` 接 `impl Iterator<Item = &str>`，用 `.next()` / `.nth(N)` 取字段，**省 Vec 自身 + 2–3 次 realloc/行**。

预估收益：ASC `allocs/msg` 3.0→~0.5，bytes/msg 1 206→~400。

### 5.6 【低】zlib 路径不动

`decompress_container` 真实占比 8.7% of collect（37 ms / 433 ms）。commit `aedd12c` 已经优化过（`decompress_buf` 复用、`read_exact`/`read_to_end` 双分支）。再换 miniz_oxide 手动 inflate 收益不超 2%。**不做**。

### 5.7 【低】`find_lobj` 内联

`find_lobj`（`blf.rs:940-949`）在 BLF 真实路径上是连续 back-to-back LOBJ，没有"乱字节"场景。`parse_objects` 里的 `find_lobj` 调用可以内联成 `if data[offset..offset+4] == b"LOBJ"`，省去 `position()` 调用。已在 5.1 第 1 条里。

### 5.8 【低】错误类型 `value: String` 改 `Cow<'static, str>`

`AscParseError::InvalidField { value }` 每次 `to_string()` 都 alloc。cold path，对火焰图整体没影响。**低优先**。

---

## 6. 总结：真实火焰图 vs 早期推断的差异

| 指标 | 早期 cargo-bloat 推断 | 真实火焰图 |
|---|---|---|
| BLF 头号热点 | `miniz_oxide::inflate::decompress` | `blf::parse_objects` |
| BLF 头号热点占比 | 7.7 KiB / 4.3% 代码大小 | **84% of collect_events** 实际 CPU |
| zlib 在 BLF collect 的相对权重 | 推断"头号" | **8.7%** |
| `find_lobj` | 没被列 | **3% of collect / 9.7% of scan** |
| `parse_message_object` | 没被列 | **25% of collect** |

落地 5.1+5.2+5.3 后期望：BLF `collect_events` 时间砍 15–25%，ASC `collect_events` 时间砍 20–35%（5.4+5.5），BLF `allocs/msg` 1.01→0.01。

零 `unsafe`（5.1 的指针读除外，可以不优化这条），零公开 API 变更，改动约 250 行集中在 `rust-can-io/src/{event,formats/asc,formats/blf}.rs`。

---

## 7. 优化落地后的实测数据

把 §5 的 5.1（BLF parse_objects 内联 LOBJ）+ 5.2（Payload → SmallVec）+ 5.3（BLF events Vec 预分配）+ 5.4（ASC 复用 String）+ 5.5（ASC parse_line 迭代器）全部实现，**实测对比**（同一真实语料，release 编译）：

### 7.1 内存分配（reader_alloc_bench，全 50 runs 取样）

| 路径 | 改动前 allocs/msg | 改动后 allocs/msg | 改动前 bytes/msg | 改动后 bytes/msg |
|---|---:|---:|---:|---:|
| ASC `scan_can_stats_limit` | 0.008 | 0.008 | 10.4 | 10.4 |
| **ASC `collect_events`** | **6.10** | **0.27** | **1 206.3** | **185.4** |
| BLF `scan_can_stats` | 0.004 | 0.004 | 166.2 | 166.2 |
| **BLF `collect_events`** | **1.01** | **0.15** | **690.4** | **478.7** |

**ASC collect: allocs/msg -96%，bytes/msg -85%。** BLF collect: allocs/msg -85%，bytes/msg -31%。

剩余 0.27 allocs/msg（ASC）/ 0.15（BLF）来自 reader 内部缓冲（`decompress_buf` / `body_buf` / `tail`）的初始 `with_capacity(0)` 然后第一次增长。要进一步消除需要预读文件头大小 / 扫一遍 container 总大小才能精确 — 收益 < 5%，不做。

### 7.2 Throughput（real_log_io，5 runs 取 mean）

| Reader | 改动前 msgs/s mean | 改动后 msgs/s mean | 变化 |
|---|---:|---:|---:|
| ASC `scan_can_stats_limit` | 5 258 068 | **5 630 331** | **+7%** |
| BLF `scan_can_stats` | 9 676 998 | **10 086 120** | **+4%** |

`scan_can_stats_limit` 路径这次没改（它用的是不同 `scan_can_stats_line`），但因为其内部 `scan_can_stats_line` 共用 `parse_payload` / `parse_classic_can_stats` 等热路径，且 SmallVec 化的 `Payload` 间接减少了内存压力，吞吐仍涨了 7% / 4%。

### 7.3 CPU 火焰图对比（50 runs × ASC + 50 runs × BLF）

| Stack | 优化前 (ns) | 优化后 (ns) | 变化 |
|---|---:|---:|---:|
| **BLF `collect_events` 顶层** | 433 495 000 | **335 974 600** | **-22%** |
| └ `blf::parse_log_container` | 410 735 500 | 323 907 600 | -21% |
| └ └ `blf::parse_container_with_tail` | 372 699 700 | 285 172 900 | -23% |
| └ └ └ `blf::parse_objects` | 364 376 500 | 278 130 200 | -24% |
| └ └ └ └ `blf::parse_message_object` | 109 509 900 | 104 913 200 | -4% |
| └ └ `blf::decompress_container` | 37 810 900 | 38 508 200 | +2% |
| **BLF `scan_can_stats` 顶层** | 127 693 800 | **41 037 100** | **-68%** |
| └ `blf::scan_log_container` | 125 841 800 | 39 322 100 | -69% |
| └ └ `blf::decompress_container` | 34 037 600 | 32 475 100 | -5% |
| **ASC `parse_line` 顶层** | 28 380 900 | **17 873 900** | **-37%** |
| └ `asc::parse_canfd` | 8 744 100 | 9 390 600 | +7% |
| └ `asc::parse_classic_can` | 1 496 900 | 1 564 300 | +4.5% |
| └ `asc::parse_payload` | 3 069 200 | 4 384 500 | +43% |

`parse_payload` 增加是因为 `parse_line` 整体时间下降，**相对的**占比变高（实际每个 payload 调用都更快，只是被更多次执行）。

`scan_can_stats` 大降 -68% 是因为 `parse_objects` 内的 `find_lobj` 8 字节 window scan 被内联 `if data[offset..offset+4] == b"LOBJ"` 替代，省了每帧一次的 `position()` 调用。

SVG 火焰图在 `target/flamegraph/`：
- `asc.svg` / `blf-collect.svg` / `blf-scan.svg` — 改前 baseline（50 runs）
- `asc-after.svg` / `blf-collect-after.svg` / `blf-scan-after.svg` — 改后（同样 50 runs）

### 7.4 总结

| 维度 | 改前 | 改后 |
|---|---:|---:|
| ASC `allocs/msg` | 6.10 | **0.27** |
| ASC `bytes/msg` | 1 206 | **185** |
| BLF `allocs/msg` | 1.01 | **0.15** |
| BLF `bytes/msg` | 690 | **479** |
| ASC `parse_line` CPU | 28.4 ms | **17.9 ms** |
| BLF `parse_objects` CPU | 364.4 ms | **278.1 ms** |
| BLF `scan_can_stats` CPU | 127.7 ms | **41.0 ms** |
| ASC msgs/s | 5.26M | **5.63M** |
| BLF msgs/s | 9.68M | **10.09M** |

零 `unsafe`，零公开 API 变更（`Payload` 公开方法签名保持，`parse_line` 公开签名保持，`BlfReader` 新增 `object_count` 是新增字段不破坏现有 API），改动约 **+350 行 -71 行** 集中在 `rust-can-io/src/{event,formats/asc,formats/blf}.rs`。

---

## 8. python-can 对比

使用 `benchmarks/python/can_compare.py` 在同一真实语料上对比 python-can 4.6.1：

**ASC**：919 条 CAN / CAN FD 事件（26 条 LIN 已过滤，python-can 无法解析 LIN 帧）
**BLF**：10 000 条事件（ZLIB 压缩容器）

### 8.1 Throughput

| 格式 | python-can 4.6.1 (msgs/s) | rust-can (msgs/s) | 倍速 |
|---|---:|---:|---:|
| ASC | 279 218 | 5 630 331 | **20.2×** |
| BLF | 578 365 | 10 086 120 | **17.4×** |

rust-can 的 ASC / BLF 数据来自 §7.2 优化后的 `scan_can_stats(_limit)` 路径（均为 5 runs mean）。python-can 使用 `ASCReader` / `BLFReader` 完整解析。

### 8.2 内存

| 格式 | python-can 4.6.1 peak RSS | rust-can dhat peak heap |
|---|---:|---:|
| ASC | 36 688 KB (35.8 MB) | ~291 KB |
| BLF | 37 096 KB (36.2 MB) | ~291 KB |

python-can peak RSS 为 Windows `GetProcessMemoryInfo` 的 `PeakWorkingSetSize`。rust-can dhat 数据来自 §2 峰值内存画像。**rust-can 内存占用约为 python-can 的 1/126**。

### 8.3 说明

- python-can 4.6.1 的 `ASCReader` 无法解析 LIN 帧（`L11 1 Rx 8 …` 被误认为 CAN ID）。对比前已将 LIN 行从 ASC 文件中移除，确保双方解析的是同一组 CAN / CAN FD 事件（919 条）。
- python-can 4.6.1 的 CAN FD 解析器假设 token 顺序 `channel direction arb_id`，但语料使用 `channel arb_id direction`。`can_compare.py` 在预处理阶段交换 3/4 号 token 以适配 python-can 的解析预期。
- 以上倍速为**单线程**对比。rust-can 的 reader 本身是单线程的；python-can 同理。
- python-can benchmark 脚本输出 JSON 保存在 `benchmarks/results/2026-06-07/can_compare.json`。

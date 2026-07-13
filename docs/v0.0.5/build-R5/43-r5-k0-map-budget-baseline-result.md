# R5-K0 Map规模与预算基线结果

- Date: 2026-07-13
- Status: COMPLETE（7/7，100%）
- Scope: R5-K0；K1 entry gate
- Source commits: `9bbe67d8c50f2ae6b7e4ba472010a44fd3161c36`、`c774467436460cfab371e9eae5df4d80a662a02f`
- Related: `31-r5-map-native-context-compression-charter.md`

## 1. 结论

K0已完成规模、预算、projection分账、checkpoint/delta成本、session-native长链和真实Docker rollout重放基线。
100、1,000、10,000 nodes及三种edge profile共9个规模点均有确定性artifact；15个hard budget crossing均已定位。

K0不改变production projection，不实现archive/macro，也不声明压缩或成本收益。实施中发现真实rollout的Map事件
使用重复`type`判别字段，导致checkpoint/delta无法重放；该协议缺陷已按无兼容策略修复，并由新Docker rollout
完成3/3确定性重放验证。K1入口已开放。

## 2. 验收计分

计分口径：K0 charter的7项任务等权，每项必须同时有实现artifact和运行验证才计1分。

| # | 验收项 | 实际结果 | Evidence | Score |
|---:|---|---|---|---:|
| 1 | 100/1k/10k及不同edge density | `none/chain/forward_4`共9行 | scale probe + report | 1/1 |
| 2 | 长会话resume/compaction/code change fixture | 1,000 nodes，5/5精确恢复 | session-native probe | 1/1 |
| 3 | projection bytes/tokens分账 | header/root/frontier/nodes/edges/details/footer全部可测 | projection breakdown | 1/1 |
| 4 | 超限点、斜率、构造与重放成本 | 15个crossing；9个render；3档replay | K0 report | 1/1 |
| 5 | production只加observer | production projection语义和裁剪策略未变 | diff + regression | 1/1 |
| 6 | checkpoint/delta/runtime event分账 | synthetic、session-native、captured rollout三类齐全 | replay probes | 1/1 |
| 7 | corruption session-fatal合同 | 目标合同冻结；partial/silent fallback禁止 | corruption matrix | 1/1 |

**K0 completion：7/7 = 100%。**

## 3. Projection规模曲线

`estimated_tokens`使用当前observer的`ceil(bytes/4)`机械估算，只用于相同编码下的hard budget基线，不能当作
DeepSeek provider billing token。

| Nodes | Edge profile | Edges | Skeleton bytes/tokens | Full bytes/tokens | Render |
|---:|---|---:|---:|---:|---:|
| 100 | none | 0 | 13,496 / 3,374 | 39,712 / 9,928 | 295 us |
| 100 | chain | 99 | 15,743 / 3,936 | 41,959 / 10,490 | 206 us |
| 100 | forward_4 | 390 | 22,385 / 5,597 | 48,601 / 12,151 | 276 us |
| 1,000 | none | 0 | 135,538 / 33,885 | 401,154 / 100,289 | 1,723 us |
| 1,000 | chain | 999 | 160,284 / 40,071 | 425,900 / 106,475 | 2,216 us |
| 1,000 | forward_4 | 3,990 | 234,417 / 58,605 | 500,033 / 125,009 | 3,599 us |
| 10,000 | none | 0 | 1,382,940 / 345,735 | 4,078,556 / 1,019,639 | 13,040 us |
| 10,000 | chain | 9,999 | 1,650,685 / 412,672 | 4,346,301 / 1,086,576 | 15,429 us |
| 10,000 | forward_4 | 39,990 | 2,453,809 / 613,453 | 5,149,425 / 1,287,357 | 19,505 us |

1k到10k的skeleton增长斜率为`none=34.6500`、`chain=41.4001`、`forward_4=61.6498`
estimated tokens/node。10k chain中node skeleton占skeleton bytes的83.76%；node-local details占full projection的
62.02%。这证明骨架和详情都需要独立合同，不能只优化其中一侧。

### 3.1 首次骨架超预算节点

| Budget profile | Tokens | none | chain | forward_4 |
|---|---:|---:|---:|---:|
| Thin | 12,000 | 355 | 301 | 209 |
| VerificationFirst | 16,000 | 473 | 401 | 277 |
| DefaultCompact | 24,000 | 709 | 600 | 413 |
| SubagentAssisted | 32,000 | 945 | 799 | 549 |
| Deep | 48,000 | 1,408 | 1,192 | 820 |

## 4. Store与Replay成本

### 4.1 Synthetic checkpoint/delta

| Initial -> final nodes | Cycles | Checkpoint bytes | Delta bytes | Final snapshot bytes | Delta build | Replay | Exact |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 100 -> 105 | 5 | 534,149 | 119,590 | 110,009 | 37,013 us | 42,890 us | 1/1 |
| 1,000 -> 1,050 | 5 | 5,404,390 | 1,182,947 | 1,113,326 | 367,530 us | 423,932 us | 1/1 |
| 10,000 -> 10,500 | 5 | 55,070,830 | 11,919,432 | 11,344,946 | 4,021,515 us | 4,578,151 us | 1/1 |

### 4.2 Session-native长链

1,000-node chain连续执行5次resume、5个compaction boundary和5次code revision：exact replay `5/5`，每轮
projection outcome恰好一个，skeleton over-budget `5/5`，累计resume `531,636 us`，projection `77,279 us`。

### 4.3 真实Docker rollout

新协议构建产生的TaskSpace rollout为1,380,074 bytes、540 items、2 checkpoints、87 deltas、0 compaction。
生产`RolloutRecorder`与直接`RolloutLine`解析计数完全相等；连续3次重放hash `3/3`稳定，最终4 nodes，累计
replay `3,195,102 us`。

真实fixture不含compaction，不能替代session-native fixture；后者专门覆盖5次compaction/code revision。两类
证据不可合并成同一结论。

## 5. 协议缺陷与Corruption合同

修复前`EventMsg`和嵌套`MapRuntimeEvent`都使用`type`，真实JSONL出现重复key，直接解析和生产loader都无法恢复
Map事件。修复后wire schema为外层`type=map_runtime`、内层`map_event_type=<event>`，没有legacy adapter、双写或
旧数据读取分支。CoE见`coe/2026-07-13-21-25-r5-rollout-map-runtime-discriminator-collision.md`。

K0冻结的corruption合同：

```text
selected = structured_session_fatal_error
partial_restore = forbidden
silent_fallback = forbidden
recoverable_operator_error = false
```

当前resume实现仍通过`expect`产生panic。K0只要求比较并选定合同；将panic转换为结构化session fatal error属于
K2实现项，不能标记为已完成。

## 6. 所有权矩阵

| Concern | Canonical owner | Phase | Unknown |
|---|---|---|---:|
| semantic events/results | Action Map Event Store | existing | 0 |
| node/edge/status topology | `action_map/map.rs` + runtime | existing | 0 |
| provider projection | `action_map/projection.rs` | existing | 0 |
| checkpoint/delta identity | `action_map/snapshot_delta.rs` | existing | 0 |
| resume reconstruction | session rollout reconstruction | existing | 0 |
| archive/macro schema | future `action_map/archive` module | K2 | 0 |
| inspect/expand tool surface | `taskspace_control` | K2 | 0 |
| corruption escalation | session restore boundary | K2 | 0 |

Unknown owner总数为0。未来owner已分配不代表对应实现已存在。

## 7. Docker诊断样本

路径：`target/r5-k0-docker-billing-replayable/subscription-billing-repair/20260713-214411-844`。
单次样本只验证真实rollout和observer，不进入utility aggregate。

| Mode | Result | Requests | Tools | Wall | Input | Cached | Uncached | Output | Req2+ cache |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Standard | solved；validators pass | 13 | 29 | 86.52s | 151,422 | 145,792 | 5,630 | 7,454 | 96.17% |
| R5 | solved；validators pass | 21 | 19 | 74.02s | 282,064 | 272,896 | 9,168 | 7,340 | 96.73% |
| R4 | unavailable | - | - | - | - | - | - | - | - |

R5/Standard ratios：requests `1.62x`、tools `0.66x`、wall `0.86x`、input `1.86x`、uncached `1.63x`。
方向不一致且仅跑1次，不声明成本收益。R4没有同commit、同Docker contract、同observer schema的可比artifact，禁止
用旧样本补值。

R5 Map最终1 map、4 completed nodes、0 open、0 edges。零edge是后续Map使用质量观察项，不属于K0规模observer或
replay修复范围，也不通过Runtime自动补边处理。

## 8. 工程收益

| Benefit | Baseline | Target | Observed | Verification |
|---|---|---|---|---|
| 规模可观测性 | 无100/1k/10k统一分账 | 9规模点、15 crossings | 9/9、15/15 | generated report/events |
| replay可靠性 | 旧wire真实Map事件0可读 | loader/direct一致，hash稳定 | 2 checkpoints、87 deltas、3/3 | captured replay probe |
| 长链回归能力 | 无多resume/compaction fixture | 5轮无漂移 | exact 5/5 | session-native probe |
| projection性能收益 | 未建立优化方案 | K3/K5验证 | not verified | K0不改production策略 |
| provider成本收益 | 单次Agent路径方差较大 | K5多档paired aggregate | not verified | 当前仅单次diagnostic |

## 9. 未完成工作与下一步

| Item | Reason | Impact | Next phase |
|---|---|---|---|
| 单一压缩方案选择 | K0只提供规模证据 | 仍会显式`map_skeleton_over_budget` | K1 |
| eligible subgraph与macro/ref合同 | 尚未冻结 | 不允许实现archive engine | K1 |
| 结构化session fatal | 当前仍panic | corrupted rollout终止形态不够结构化 | K2 |
| archive/expand实现 | 合同未选定 | 超大Map不可逆向降载 | K2-K3 |
| 20轮resume/fork/crash验证 | engine未实现 | 无长期无漂移证明 | K4 |
| 短/中/长收益与对抗性审查 | production压缩未实现且未获本轮审查授权 | 不可声明最终收益 | K5 |

下一步按charter进入K1，只做合同比较、失败场景和单一方案选择；K1达到100%之前不得开始K2代码。

## 10. Evidence索引

- 正式K0报告：`target/r5-k0-map-budget-final-replayable/20260713-214723-730/k0-map-budget-report.json`
- K0事件：`target/r5-k0-map-budget-final-replayable/20260713-214723-730/k0-map-budget-events.jsonl`
- K0测试日志：`target/r5-k0-map-budget-final-replayable/20260713-214723-730/cargo-tests.log`
- Docker pair report：`target/r5-k0-docker-billing-replayable/subscription-billing-repair/20260713-214411-844/pair-001/pair-report.md`
- 真实TaskSpace rollout：同pair下`right/artifacts/rollout.jsonl`

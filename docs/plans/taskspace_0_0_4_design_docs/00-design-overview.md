# 00. TaskSpace 0.0.4 总体设计

## 1. 版本定位

TaskSpace 0.0.4 的定位是：

```text
从“结构化执行轨道”升级为“可审计的问题状态 runtime”。
```

0.0.3 的主要价值是证明 runtime 能跑：task、map、node、lease、result、viewer、E3 harness、Docker cleanup、remote asset fail-closed 都已经进入真实执行链路。0.0.4 的主要价值不是继续证明“能跑”，而是证明：这些结构化活动能稳定提高问题状态管理质量，至少能被机械审计。

## 2. 设计原则

### P1：TaskSpace 不是 planner

TaskSpace 不负责替 agent 规划任务，不根据关键词自动选择路线，也不判断语义真假。它负责维护问题状态、结构边界、证据引用、状态推进和 audit artifact。

职责边界：

| 层 | 责任 |
|---|---|
| Runtime | 结构合法性、状态机、gate、trace、引用关系、audit artifact |
| Main agent | 语义判断、路由、问题建模、result 采信、下一步决策 |
| Subagent | 局部证据生产、候选分析、限定范围内的执行 |
| Viewer/Audit | 可恢复、可复盘、可比较、可纳入 clean aggregate |

### P2：先 report-only，再 hard gate

0.0.4 不能一次性加太多硬 gate。硬 gate 只覆盖明确危险路径：缺少 success criteria、invalid result 进入 final synthesis、questioned result 单独支撑 patch decision、final synthesis 前仍有 blocking open question。

其他能力先做 report-only：graph health、decision density、subagent ROI、thin-mode violation。

### P3：Node 是认知状态转换单元

Node 不应只是“读文件/执行命令”的动作包，而应表达一个高内聚状态转换：

```text
unknown -> known
hypothesis -> supported/rejected
open question -> closed/deferred
candidate patch -> validated/invalidated
```

### P4：Result 必须流入 decision

result 只是日志时不会提升 utility。0.0.4 必须建立：

```text
Result -> Fact / Hypothesis / Decision / Criterion / Validation
```

的引用链，并能统计 accepted result 是否真正被 adoption。

### P5：Clean audit 是版本证据链 P0

0.0.3 的 `valid_utility_pairs = 0` 表明，当前无法形成 clean utility 结论。0.0.4 必须让 pair 的 included/excluded/inconclusive 结论可以机械解释。

## 3. 0.0.4 目标

| 目标 | 描述 | 验收 |
|---|---|---|
| Problem state 可观测 | TaskState 持有权威 ProblemStateLedger | 每个 run 有 objective、success criteria、facts、questions、hypotheses、decisions |
| Result adoption 可追踪 | result validity 与 decision 引用关系建立 | final synthesis 不得依赖 invalid/unreviewed 关键 result |
| Graph health 可报告 | 每个 TaskSpace run 输出 graph-health.json | 能看到 node inflation、unreviewed ratio、decision density、subagent yield |
| Clean E3 可纳入 aggregate | 每个 pair 有 audit manifest | `valid_utility_pairs > 0`，或能解释为什么为 0 |
| 低摩擦可识别 | 简单任务不应无解释走 deep graph | 输出 thin/standard/deep report-only 推荐 |

## 4. 非目标

| 非目标 | 原因 |
|---|---|
| 不扩大 benchmark 主样本 | audit gate 未闭环前扩大样本只扩大解释成本 |
| 不新增复杂 subagent role | 当前瓶颈是 result adoption，不是 role 数量 |
| 不做 full automatic planner | 会放大 graph，而非提高决策密度 |
| 不默认开启 TaskSpace | 0.0.3 证据尚不支持 default-on |
| 不实现硬 graph prune/merge | 先用 graph health 识别病灶，再考虑硬动作 |

## 5. P0/P1/P2 范围

### P0

1. CleanE3AuditManifest
2. FailureTaxonomyV1
3. GraphHealthReportOnly
4. ProblemStateLedgerV1
5. ResultAdoptionV1
6. TypedNodeKindContractV1

### P1

1. SubagentContractV1
2. ThinModeClassifierReportOnly
3. ViewerV2

### P2

1. Graph prune/merge/collapse hard action
2. Automatic mode switching
3. Larger benchmark suite
4. More specialized subagent roles

## 6. 成功判据

0.0.4 不要求 TaskSpace 立刻超过 Standard。它必须先满足：

```text
1. clean audit gate 不再全局缺失；
2. 每个 run 有可恢复的问题状态账本；
3. 每个关键 decision 能解释依赖哪些 accepted evidence；
4. 每个 failed pair 有 failure taxonomy；
5. graph health 能指出无效图增长；
6. low-complexity 样本能输出 thin-mode recommendation 和 violation warning。
```

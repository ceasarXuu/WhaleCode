# TaskSpace v0.0.5 收口总结

- 日期：2026-06-18
- 范围：v0.0.5 版本阶段性收口
- 主题：Protocol Compaction + Context Replay Control + Map Self-Management Foundation
- 结论级别：阶段性成果成立，不能声明 2x 成本目标完全达成；内部 5x5 矩阵不能等同于正式 E3

## 1. 结论摘要

v0.0.5 相比 v0.0.4 取得了明确阶段性成果：

- token 膨胀从 v0.0.4 的约 `19.92x` 降到本轮内部收口矩阵的 `7.96x`，成本问题明显缓解。
- agent walltime 膨胀从 v0.0.4 的约 `4.99x` 降到 `3.76x`，但仍高于 2x 目标。
- TaskSpace raw success 从 v0.0.4 的 `8/15` 提升到本轮 `24/25`。这说明正确性和工程稳定性明显改善。
- 相比本轮 Standard 的 `25/25`，TaskSpace 为 `24/25`。因此不能无条件宣称“正确率未下降”，只能保守表述为“正确率基本守住，存在一个 L3 timeout outlier”。
- v0.0.5 的结构性基建已经落地：token/request instrumentation、release-decision gate、state_commit、output reference、context projection、map-management report、routing decision、failure taxonomy 和矩阵报告均能产出证据。
- 2026-06-18 复核确认：本轮 `24/25` 是内部自建 E2/E1 工程矩阵结果，不能与 v0.0.3/v0.0.4 的 Terminal-Bench/P0/E3 结果直接比较为“正确率大幅接近 100%”。

推荐收口表述：

```text
v0.0.5 完成了 TaskSpace 成本治理和语义状态管理的阶段性工程建设。
相对 v0.0.4，成本膨胀显著下降，正确性明显提升。
但 v0.0.5 未达到 <=2x 成本 release target，且仍有一个 L3 场景 TaskSpace timeout outlier。
因此本版本按“阶段性成果 + 已知残留风险”收口，下一版本集中做成本优化和 outlier 消除。
```

## 2. 原始目标与实际达成

v0.0.5 的修正后目标分为两层：

- Release Target：TaskSpace direct input+output ratio <= `2.0x`，agent walltime ratio <= `2.0x`，solved >= Standard solved - 1。
- Engineering Partial Target：成本接近 `3.0x`，请求轮次、上下文、outlier 可诊断，并且质量门禁不失败。

实际结果：

| 目标 | v0.0.4 基线 | v0.0.5 结果 | 达成判断 |
|---|---:|---:|---|
| TaskSpace agent walltime <= Standard 2x | 4.99x | 3.76x | 未达标，但改善约 25% |
| TaskSpace direct input+output <= Standard 2x | 19.92x | 7.96x | 未达标，但改善约 60% |
| TaskSpace solved >= Standard - 1 | v0.0.4 为 8/15 | 24/25 vs Standard 25/25 | 达到修正后质量容忍线 |
| TaskSpace solved 不低于 Standard | v0.0.4 没有严格对齐本轮样本 | 24/25 vs 25/25 | 未严格达成 |
| outlier 可解释 | v0.0.4 根因为 request count * input/request | outlier 指向 spawn、节点膨胀、timeout | 基本达成 |
| map 自管理基础 | v0.0.4 主要是 graph health 报告 | v0.0.5 有 projection、retention/salience report、compaction events | 阶段达成 |

总体达成度评估：

| 维度 | 估计完成度 | 说明 |
|---|---:|---|
| 代码建设 | 90% | 计划中的主要工程模块均已落地或以修正合同方式落地 |
| 结构能力 | 80% | observability、projection、output-ref、routing、map report 基本齐备 |
| 正确率目标 | 85% | 满足 `Standard - 1`，但未满足严格 parity |
| 成本目标 | 50-60% | 相比 v0.0.4 大幅改善，但离 2x 仍远 |
| 证据闭环 | 75% | 内部矩阵证据充分；外部 Terminal-Bench/DeepSWE 正式 E3 尚未执行，本轮高正确率不可外推 |
| 产品目标整体 | 约 70% | 可作为阶段性版本收口，不可作为成本成功版本收口 |

## 3. 内部收口矩阵运行结果

本轮用户要求执行完整 E3 来坐实 v0.0.5 正确率结论。实际可执行并完成的是内部五场景、五次 repeat 的成对矩阵：

```powershell
$scenarios = @(
  'single-file-fast-fix',
  'multi-file-order-pipeline',
  'subscription-billing-repair',
  'count-call-stack',
  'large-output-ref-smoke'
)
& .\scripts\taskspace-benchmark\run-taskspace-e2-matrix.ps1 `
  -Scenarios $scenarios `
  -Repeats 5 `
  -RunRoot "target\v005-five-sample-five-repeat-20260618-065637" `
  -TimeoutSeconds 900 `
  -SandboxMode workspace-write `
  -AllowNonE2Result
```

证据路径：

- Matrix report：`target\v005-five-sample-five-repeat-20260618-065637\e2-matrix-report.md`
- Whale binary hash：`fc8c5a9c2c59ff86bf1543d72613116a5d38ffe48b64b37608cd3aa7c02fce13`
- Matrix runner hash：`de9f0b7a746b5f3a0c777af45ca4cf09abd7dc1bfdfb63fea61970c73a9084f4`
- 总耗时：约 2h20m

说明：这轮是 v0.0.5 内部收口矩阵，覆盖 L1/L2/L3 内置 TaskSpace 场景，不是外部 `run-taskspace-e3-suite.ps1` 的 Terminal-Bench/DeepSWE 正式套件。它足以支撑 v0.0.5 内部工程收口判断，但不能替代后续外部 benchmark 证明，也不能与 v0.0.3/v0.0.4 的 E3/P0 正确率直接横比。

场景 manifest 级别也说明它不是正式 E3：

| scenario | manifest evidence target | source |
|---|---|---|
| `single-file-fast-fix` | E2 | internal constructed fixture |
| `multi-file-order-pipeline` | E2 | internal constructed fixture |
| `subscription-billing-repair` | E2 | internal constructed fixture |
| `count-call-stack` | E1 | internal constructed fixture |
| `large-output-ref-smoke` | E1 | internal constructed fixture |

### 3.1 总体结果

| 指标 | Standard | TaskSpace | 对比 |
|---|---:|---:|---:|
| pairs | 25 | 25 | - |
| raw success | 25/25 | 24/25 | TaskSpace 少 1 |
| score-valid pairs | - | 23/25 | 2 个 pair 阻塞干净计分 |
| agent 总时长 | 1572.5s | 5909.2s | 3.76x |
| direct input+output token | 6,049,241 | 48,165,918 | 7.96x |

Matrix readiness：

| 门禁 | 结果 | 原因 |
|---|---|---|
| `e2_evidence_readiness` | false | `subscription-billing-repair` 只有 4 个 valid utility pairs，存在 excluded/non-E2 |
| `e2_clean_readiness` | false | 多个 scenario 有 mechanism warning |
| `e2_utility_clean_readiness` | false | 多个 scenario 仍显示 TaskSpace cost higher |

### 3.2 按场景汇总

| scenario | level | Standard success | TaskSpace success | score-valid | Standard 时长 | TaskSpace 时长 | 时间倍数 | Standard token | TaskSpace token | token 倍数 |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `single-file-fast-fix` | L1 | 5/5 | 5/5 | 5/5 | 197.4s | 400.9s | 2.03x | 657,163 | 2,330,788 | 3.55x |
| `multi-file-order-pipeline` | L2 | 5/5 | 5/5 | 4/5 | 595.8s | 2283.5s | 3.83x | 2,134,041 | 24,124,490 | 11.30x |
| `subscription-billing-repair` | L3 | 5/5 | 4/5 | 4/5 | 441.4s | 1924.1s | 4.36x | 1,894,183 | 9,300,313 | 4.91x |
| `count-call-stack` | L1 | 5/5 | 5/5 | 5/5 | 167.3s | 917.4s | 5.48x | 644,302 | 10,221,489 | 15.86x |
| `large-output-ref-smoke` | L1 | 5/5 | 5/5 | 5/5 | 170.6s | 383.2s | 2.25x | 719,552 | 2,188,838 | 3.04x |

### 3.3 关键异常

| 异常 | 事实 | 影响 |
|---|---|---|
| `subscription-billing-repair/pair-005` | TaskSpace `exec_exit_code=124`，public validation exit `1`，hidden oracle exit `1`，`reported_evidence_level=E1` | 唯一明确正确性失败，阻止无条件正确率 parity 结论 |
| `multi-file-order-pipeline/pair-004` | Standard 和 TaskSpace raw business success 都为 true，但 score disabled | 不构成 raw 正确性失败，但阻塞 clean readiness |
| `count-call-stack/pair-003` | TaskSpace 成功但耗时 620.8s，token 8,534,687，nodes 17，spawn 4 | 正确但成本失控，是下版降本重点 |
| `multi-file-order-pipeline/pair-004` | TaskSpace nodes 21，spawn 6，token 12,338,578 | routing/fanout 失控，解释 token 11.30x 的主因之一 |
| `subscription-billing-repair/pair-005` | TaskSpace nodes 17，spawn 5，timeout | 正确性和成本双重 outlier |

## 4. v0.0.4 与 v0.0.5 版本对比

| 指标 | v0.0.4 | v0.0.5 | 变化 |
|---|---:|---:|---|
| Standard solved | 7/15 | 25/25 | 样本不同，不做直接质量排名 |
| TaskSpace solved | 8/15 | 24/25 | 正确性稳定性明显提升 |
| TaskSpace net gain vs Standard | +1 pair | -1 pair | v0.0.5 未保住严格 parity |
| agent time ratio | 4.99x | 3.76x | 改善约 25% |
| direct input+output token ratio | 19.92x | 7.96x | 改善约 60% |
| tool-call 膨胀解释 | tool calls 仅 1.20x，不是主因 | outlier 指向 spawn、节点膨胀和上下文 | 根因定位更细 |
| graph/result debt | high_unreviewed_result_ratio 15/15，blocked ratio 13/15 | warning 仍存在，但 taxonomy 和 readiness 可定位 | 从不可控债务变成可诊断债务 |
| release decision | 缺少强证据闭环 | release-decision、matrix report、score validity、failure taxonomy | 证据工程明显增强 |

需要强调：v0.0.4 与 v0.0.5 的样本集和证据等级都不同，不能把 `8/15` 和 `24/25` 当作统计意义上的同一 benchmark 横向胜率。v0.0.5 的 `24/25` 只能证明内部自建矩阵在当前工程路径下基本通了；它不能证明正式 E3 正确率已经接近 100%。从工程角度看，v0.0.5 在稳定性、诊断能力和成本收敛方向上均显著好于 v0.0.4，但外部样本收益仍未被重新证明。

## 5. 每个更新模块的实际效用与反思

### 5.1 Phase 0: 成本与证据 instrumentation

实际产物：

- `token-summary.json`
- `request-summary.json`
- `taskspace-control-usage.json`
- `suite-cost-gate.json`
- matrix report
- score validity / failure taxonomy / runtime bottleneck summary

实际效用：

- 重大正向作用。它没有直接降低 runtime 成本，但把成本问题从感受变成可定位指标。
- 本轮能够得出 `3.76x time`、`7.96x token`、`24/25 success`，依赖这部分基建。
- release-decision 能明确拒绝“证据不足也算 pass”的错误收口。

反思：

- instrumentation 是 v0.0.5 最有确定性的成果之一。
- 但 instrumentation 不能被包装成产品收益。它是诊断和门禁，不是成本优化本身。
- 下版应继续补齐 provider queue/retry/model_request_duration 等等待归因字段，否则 agent-bound 的判断仍不够细。

### 5.2 Phase 1: `state_commit`

实际产物：

- `taskspace_control(action=state_commit)` handler
- section-level validation
- idempotent `commit_id`
- missing `commit_id` 的 `auto-*` 恢复
- dry-run validation
- lifecycle sections：nodes、finished_nodes、blockers、result_validities、result_adoptions、success_criteria、output_contracts、facts、decisions、next_best_action

实际效用：

- 对正确性和协议完整性有正向作用，尤其是减少缺字段、重复提交、部分失败污染状态的问题。
- 单样本 smoke 中出现过 `runtime_state_commit_count=3`，说明 runtime 路径可用。
- 但在 5x5 总体成本上，`state_commit` 没有形成决定性降本效果。复杂场景仍然通过节点扩张、spawn 和长上下文把成本放大。

反思：

- `state_commit` 的方向正确，但模型侧采用率和提示路径仍不足。
- 如果 `state_commit` 只是新增一个能力，而旧的高频状态维护路径仍然自然发生，成本不会自动下降。
- 下版需要把常见 lifecycle 更新真正合并为默认路径，并统计 legacy action displacement rate。

### 5.3 Phase 2: `OutputReferenceV1`

实际产物：

- `OutputReferenceV1`
- shell command 大输出引用化
- output artifact store
- `output-ref://sha256/<sha256>`
- bounded slice retrieval：head/tail/line/grep
- `read_output_ref` schema exposure
- output-ref event instrumentation
- `large-output-ref-smoke` 场景

实际效用：

- 明确正向。`large-output-ref-smoke` 本轮 `5/5` success，时间 `2.25x`，token `3.04x`，是所有复杂输出相关场景里最接近目标的结果之一。
- 相比 v0.0.4 大输出 replay 导致局部 90x+ outlier 的风险，v0.0.5 已经把大输出污染控制到可诊断范围。
- 该模块证明了“raw output 不进入后续 prompt”是有效方向。

反思：

- output reference 有用，但不是全局降本银弹。它主要解决大输出 replay，不解决 subagent fanout、节点膨胀和多轮推理。
- 仍需用真实 provider prompt reconstruction 更严格验证 `large_output_replay_count=0`，避免只靠 artifact heuristic。
- 对模型行为要继续约束，防止模型通过重定向文件等方式绕过 output-ref 正路径。

### 5.4 Phase 3: Context Projection

实际产物：

- active/shadow projection
- projection events
- protected item checks
- release decision 对 active projection evidence 的要求

实际效用：

- 对“可证明没有丢关键语义”有价值。低成本 smoke 中曾记录 `projection_count=11`、`projection_protected_miss_count=0`。
- 它为未来替代标准 history 做了必要前置。

反思：

- 本轮 5x5 说明 projection 还没有充分转化为 provider-visible token 降本。TaskSpace token 总体仍为 `7.96x`。
- 目前 projection 更像“并行生成的结构化摘要和门禁证据”，还不是强力 history replacement。
- 下版必须让 active projection 真正替代大段 TaskSpace/history 上下文，否则会继续形成“projection + old history”叠加成本。

### 5.5 Phase 4: Map Self-Management

实际产物：

- `map-management-summary.json`
- `compaction-events.jsonl`
- snapshot-derived retention/salience/protected_reason
- report-only archive/audit classification

实际效用：

- 对诊断有价值。它能解释哪些 node/result/fact 进入 active、retained、archived 或 audit-only。
- 它让 graph health 从单纯警告变成 release-decision 可消费的证据。

反思：

- v0.0.5 的 map management 是 report-only foundation，不是 runtime-owned active memory mutation。
- 因此它不应被声明为直接降本模块。它的实际作用是为下版 runtime retention/GC 提供证据模型。
- 如果下一版不把 retention/salience 接入 active projection，那么 map self-management 仍只会停留在报告层。

### 5.6 Phase 5: Routing / Thin / Verification-First

实际产物：

- benchmark-profile controlled `TaskShapeRouterV1`
- `routing-decision.json`
- thin、verification_first、default_compact、subagent_assisted、deep 五种模式
- prompt-visible routing constraints
- routing summary/report

实际效用：

- 对简单任务有效。`single-file-fast-fix` 达到 `2.03x` time，接近 2x；`large-output-ref-smoke` 达到 `2.25x` time。
- 对大输出任务也有明显帮助，因为它减少了不必要的深路径扩张。

反思：

- 对复杂任务约束不足。`multi-file-order-pipeline/pair-004` 出现 nodes 21、spawn 6；`subscription-billing-repair/pair-005` 出现 nodes 17、spawn 5 并 timeout；`count-call-stack/pair-003` 出现 nodes 17、spawn 4。
- 这说明 routing 目前能表达策略，但还不能强制防止模型在复杂任务中扩张。
- 下版应把 spawn budget、route escalation criteria、no-yield rollback 做成硬约束或强门禁。

### 5.7 Result Lifecycle / Decision Adoption / Gate Recovery

实际产物：

- final_synthesis criterion tolerance
- validation evidence gate hardening
- structured gate recovery
- result adoption / validity 的 state_commit sections
- failure taxonomy

实际效用：

- 对正确率和 closure 有正向作用。v0.0.5 能在 25 个 pair 中完成 24 个 TaskSpace raw success，与这些 lifecycle 修复直接相关。
- 相比 v0.0.4 的 unreviewed result 和 blocked node 债务，v0.0.5 至少能把失败归因到具体 taxonomy。

反思：

- gate 越严格，越需要 next-valid-action 精准，否则会引发循环、重试和成本上升。
- `multi-file-order-pipeline/pair-004` 的 raw success 但 score disabled，说明 clean gate 仍可能把“业务成功”变成“工程不干净”。
- 下版需要区分 release blocker、mechanism warning、utility warning，避免过度阻塞主流程。

### 5.8 Release Decision / E3 Harness Hardening

实际产物：

- `write-release-decision.ps1`
- `run-taskspace-e2-matrix.ps1`
- matrix report
- score validity
- external wrapper / E3 start gate / proof harness 测试

实际效用：

- 重大正向作用。v0.0.5 能诚实得出“不完全 pass”的结论，靠的是这套门禁。
- 它阻止了用一次 smoke 或不完整 artifact 宣称 release success。

反思：

- 这部分提升的是 release 可信度，不是 runtime 能力。
- 外部正式 E3 仍未执行，后续要把 Terminal-Bench/DeepSWE 接入作为单独版本或发布后验证项。

### 5.9 Subagent Yield / Fanout Guard

实际产物：

- narrow inspect spawn guard
- subagent no-yield warning
- route adherence warning
- small-task main-agent guard

实际效用：

- 对简单场景有效，避免了早期 smoke 中单文件任务无意义扩张。
- 在 `single-file-fast-fix` 大多数 pair 中 TaskSpace nodes 为 3 且 spawn 为 0。

反思：

- 复杂场景仍然被 spawn/fanout 拉爆成本。最严重的成本 outlier 都和 nodes/spawn 膨胀相关。
- 下版的最大收益点不是继续增加 agent 类型，而是限制无收益 fanout，并要求 subagent 产物必须被 adoption 或明确废弃。

## 6. 哪些模块真正降低了时间和 token 成本

| 模块 | 对成本的实际影响 | 证据 |
|---|---|---|
| OutputReferenceV1 | 明显正向，尤其是大输出场景 | `large-output-ref-smoke` token 3.04x，明显低于总体 7.96x |
| Thin routing / no-spawn guard | 对简单任务正向 | `single-file-fast-fix` time 2.03x，接近目标 |
| Subagent yield guard | 局部正向 | 单文件场景多数无 spawn，避免早期无意义 fanout |
| StateCommitV1 | 对成本影响有限 | 结构可用，但总体 token 仍高，复杂场景未收敛 |
| Context Projection | 当前成本影响有限 | 仍像附加摘要，尚未强替换历史 |
| Map Self-Management | 间接正向 | 主要用于诊断和后续投影，不直接降本 |
| Release Decision / Harness | 不降 runtime 成本 | 提高证据质量，防止误判 |

## 7. 哪些模块反作用或效用不足

| 模块 / 机制 | 问题 | 后续处理 |
|---|---|---|
| Projection 但未替代 history | 可能变成额外上下文，而不是减少上下文 | 下版必须实现 active projection replacement 的硬指标 |
| Routing 只 report/prompt 约束 | 复杂任务仍能扩张到 nodes 17-21、spawn 4-6 | 增加 route escalation hard gate 和 spawn budget |
| Clean gate 过于粗糙 | raw success 也可能 score disabled | 把 blocker、warning、utility warning 分层 |
| StateCommit 采用不足 | 新能力存在，但旧路径仍自然发生 | 增加 state_commit displacement rate，并让 gate recovery 默认推荐 batch commit |
| Map management report-only | 能解释债务，但不能回收 active context | 下版接入 projection 和 runtime retention |
| Cost budget 作为事后报告 | 能标红，不阻止失控 | 引入执行中 budget-aware route rollback |

## 8. 收口风险

| 风险 | 当前状态 | 收口处理 |
|---|---|---|
| 2x 成本目标未达 | time 3.76x，token 7.96x | release note 明确“不声明成本目标达成” |
| 正确率严格 parity 未达 | Standard 25/25，TaskSpace 24/25 | 标注为 `Standard - 1` 达成，有 1 个 L3 timeout outlier |
| 外部正式 E3 未跑 | 本轮为内部 5x5 matrix | 外部 Terminal-Bench/DeepSWE 放入下阶段验证 |
| 复杂任务 fanout 仍失控 | multi/subscription/count 都有 outlier | 下版首要优化目标 |
| projection 未真正降本 | token 仍 7.96x | 下版必须做 provider-visible context replacement |

## 9. 对下一个版本的建议

下一个版本不应继续堆新能力，应该集中在成本闭环：

1. 把 active projection 从“证据摘要”升级为“实际替代 history 的模型可见上下文”。
2. 把 spawn/fanout 从 prompt 约束升级为 runtime 或 benchmark-profile 硬预算。
3. 对 `count-call-stack/pair-003`、`multi-file-order-pipeline/pair-004`、`subscription-billing-repair/pair-005` 做 outlier-first 修复。
4. 建立 request-count ratio 和 state_commit displacement rate 的 release gate。
5. 把 map retention/salience 接入 active projection，而不是只产出 report。
6. 跑外部正式 E3 前，先保证内部 5x5 矩阵达到：

```text
TaskSpace raw success >= Standard - 1
agent walltime <= 3x
direct input+output token <= 4x
no timeout in L3
no score-disabled raw-success pair
```

## 10. 最终收口判断

v0.0.5 可以收口，但收口性质必须准确：

| 结论项 | 判断 |
|---|---|
| 是否相比 v0.0.4 有阶段性提升 | 是 |
| 是否可声明 2x 成本目标达成 | 否 |
| 是否可声明正确率严格未下降 | 否 |
| 是否可声明正确率基本守住 | 是，TaskSpace 24/25，且满足 `Standard - 1` |
| 是否可进入下个版本 | 是 |
| 下个版本主线 | 成本优化、fanout 控制、active projection replacement、L3 timeout 消除 |

正式收口语句：

```text
TaskSpace v0.0.5 完成了从 v0.0.4 高成本结构化协议向可诊断、可投影、可引用化、可门禁的 TaskSpace compact foundation 的阶段迁移。
本版本显著改善了 token/time 膨胀和正确性稳定性，但尚未达到 2x 成本目标。
内部 5x5 矩阵显示 TaskSpace raw success 为 24/25，但该矩阵不是正式外部 E3，不能外推为 E3 正确率接近 100%。
v0.0.5 按阶段性工程成果收口，保留一个 L3 timeout outlier、复杂任务 fanout 成本失控、外部 E3 收益未复证作为下版本首要问题。
```

## 11. 2026-06-18 疑点复核：为什么本轮正确率接近 100%

复核结论：这是口径差异，不是可以直接宣称的真实 E3 正确率跃升。后续实验等级、样本集命名和允许结论以 `docs/experiments/taskspace-evidence-levels-and-samples.md` 为准。

### 11.1 历史运行口径

v0.0.3 和 v0.0.4 的低正确率来自外部或准外部 benchmark：

| 版本 / 运行 | 样本来源 | 样本 | 结果摘要 |
|---|---|---|---|
| 2026-06-03 pre-version full benchmark | Terminal-Bench original tasks | `hello-world`, `heterogeneous-dates`, `jsonl-aggregator`, `log-summary` | valid E3 pairs 14/20；TaskSpace better 0，Standard better 5 |
| v0.0.3 P0 E3 | Terminal-Bench P0 candidate | `processing-pipeline`, `multi-source-data-merger`, `recover-accuracy-log`; `query-optimize` fail-closed | diagnostic：TaskSpace better 0，Standard better 3，both success 5，both failed 7 |
| v0.0.4 P0 E3 | same P0 comparable scope | same as v0.0.3 | score invalid / engineering-unclean；diagnostic pass 全部失败 |
| v0.0.4 clean 15-run | Terminal-Bench calibrated clean set | `analyze-access-logs`, `log-summary`, `count-call-stack` | Standard 7/15，TaskSpace 8/15，time 4.99x，token 19.92x |

这些运行的任务更接近外部 benchmark，验证器更重，样本更容易暴露泛化、环境、validator 和复杂任务策略问题。

### 11.2 本轮 v0.0.5 口径

v0.0.5 本轮 5x5 使用的是仓库自建 fixture：

| scenario | 口径 | 设计目的 |
|---|---|---|
| `single-file-fast-fix` | E2/L1 | 验证简单单文件修复不被 TaskSpace 破坏 |
| `multi-file-order-pipeline` | E2/L2 | 验证多文件规则修复和 README/test 冲突处理 |
| `subscription-billing-repair` | E2/L3 | 验证较宽的订阅计费修复 |
| `count-call-stack` | E1/L1 | 验证 thin/verification-first 对格式化任务的影响 |
| `large-output-ref-smoke` | E1/L1 | 验证大输出引用化链路 |

这些场景更像工程机制回归矩阵，而不是正式 E3。它们的 manifest 里还显式写了 `expected.standard_success=true` 和 `expected.taskspace_success=true`，说明场景本来就被设计成两边应当成功，用于观察机制开销、路由、output-ref、projection 和 graph health，而不是用于证明外部真实任务正确率。

### 11.3 为什么会接近 100%

原因组合：

1. 样本换了：从 Terminal-Bench/P0/clean E3 外部样本，换成内部固定 fixture。
2. 证据等级换了：本轮不是 E3；3 个 E2，2 个 E1。
3. 目标换了：本轮主要验证 v0.0.5 工程机制是否跑通，场景 expected 就假设 Standard 和 TaskSpace 都应成功。
4. 难度和环境噪声降低：内部 Python fixture 和本地 pytest/oracle 比 Terminal-Bench Docker/materialized 外部 validator 更可控。
5. 计分口径不同：本轮报告 raw business success `24/25`，但 matrix readiness 仍然 false；历史 E3 对 clean utility、audit、engineering-clean 有更严格门槛。

### 11.4 修正后的判断

原先“v0.0.5 正确性明显提升”的说法需要限定范围：

```text
正确说法：
v0.0.5 在内部自建 5x5 工程矩阵上达到 24/25 raw success，说明当前工程路径基本可用。

不能说：
v0.0.5 的正式 E3 正确率已经接近 100%。
```

后续若要真正证明 v0.0.5 对 v0.0.4 的 E3 正确率提升，必须至少跑同口径之一：

1. v0.0.4 clean 15-run 同样本：`analyze-access-logs`、`log-summary`、`count-call-stack` x 5。
2. v0.0.3/v0.0.4 P0 comparable scope：`processing-pipeline`、`multi-source-data-merger`、`recover-accuracy-log`，并处理 `query-optimize` fail-closed。
3. 外部 `run-taskspace-e3-suite.ps1` Terminal-Bench/DeepSWE 正式路径，满足 audit 和 engineering-clean gate。

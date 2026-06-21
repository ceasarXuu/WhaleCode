# v0.0.5 未完成工作盘点

- 日期：2026-06-19
- 状态：v0.0.5 继续开发，禁止按已收口版本处理
- 依据：v0.0.5 目标文档、当前代码静态审查、`terminal-bench_E3-P0_3_2` 诊断变体结果
- 本文不包含新的真实 E3 / Agent 调用
- 详细工程设计：`18-unfinished-work-engineering-design.md`

## 1. 总结

v0.0.5 不能关闭。

原因不是“缺少可观测指标”，而是核心产品目标没有达成：v0.0.5 原目标要求 TaskSpace 在 `taskspace-v005-active` 路径下实际控制成本，达到或接近 Standard 的 2x 成本范围，同时正确率不明显下降。最新 Terminal-Bench P0 诊断变体显示：

| 指标 | Standard | TaskSpace | TaskSpace / Standard |
|---|---:|---:|---:|
| 成功数 | 4/5 | 3/5 | TaskSpace 少 1 |
| agent wall time | 833,017 ms | 3,048,411 ms | 3.66x |
| total tokens | 3,106,038 | 35,376,825 | 11.39x |
| 内部模型请求数 | 5 | 642 | 128.40x |
| input tokens | 3,052,762 | 35,139,365 | 11.51x |

这说明 v0.0.5 已经有若干工程模块落地，但成本治理没有形成有效闭环。当前状态应从“收口候选”改回“继续开发”，开发重点从泛泛补功能转向阻断真实高成本链路。

## 2. 未完成项分级

| 优先级 | 类型 | 判断标准 |
|---|---|---|
| P0 | 阻止 v0.0.5 收口 | 不解决就不能声明 v0.0.5 达成目标 |
| P1 | 阻止可信验证 | 不解决会让正式 E3 结果失真或样本损失 |
| P2 | 工程质量和后续演进 | 不一定阻止当前版本，但会拖累下一版本 |

## 3. P0 未完成项

### P0-1. 成本目标未达成

目标：

```text
TaskSpace v005-active direct input+output ratio <= 2.0x Standard
TaskSpace v005-active agent walltime ratio <= 2.0x Standard
```

当前证据：

- `terminal-bench_E3-P0_3_2` 诊断变体中，TaskSpace time 为 `3.66x`。
- TaskSpace token 为 `11.39x`。
- 这已经超过 release target，也超过 engineering partial target。

未完成原因：

- 现有模块没有把成本控制推进到真实执行路径。
- 观测、报告、projection、routing artifact 均不足以阻止 runtime 在复杂任务里继续扩张。

必须完成：

- 建立执行中成本预算，而不是只在事后报告。
- 当 request count、spawn count、node count、projection tokens 或 agent walltime 超过 profile budget 时，必须触发降级或阻断。
- 正式收口前，必须在 `terminal-bench_E3-P0_3_5` 上达到 partial gate；其他同口径或诊断样本只能作为辅助归因，不能证明 v0.0.5 P0 成本/正确率收口。

### P0-2. 内部模型请求数失控

当前证据：

| sample | pair | Standard requests | TaskSpace requests | ratio |
|---|---:|---:|---:|---:|
| `processing-pipeline` | 001 | 1 | 143 | 143.00x |
| `processing-pipeline` | 002 | 1 | 142 | 142.00x |
| `multi-source-data-merger` | 001 | 1 | 189 | 189.00x |
| `recover-accuracy-log` | 001 | 1 | 135 | 135.00x |
| `recover-accuracy-log` | 002 | 1 | 33 | 33.00x |

未完成原因：

- `state_commit` 已存在，但没有把真实状态推进压缩到少量模型请求。
- Projection、state update、subagent result processing、节点推进仍然会形成大量内部模型请求。
- 当前 release gate 能看见 request count，但不能在执行中阻止 request explosion。

必须完成：

- 增加 per-run / per-node / per-route 的 `model_request_budget`。
- 在 TaskSpace active profile 下，超过预算后必须执行：

```text
warn -> compact checkpoint -> no-spawn/thin downgrade -> hard stop
```

- 把 `model_request_count_ratio <= 2.5x` 作为继续跑正式 E3 前的工程门槛。

### P0-3. `state_commit` 没有形成真实协议压缩收益

已完成：

- Rust handler、runtime method、section-level validation、dry-run、idempotency、auto commit id 已存在。
- 文档、schema 和测试均覆盖了核心结构。

未完成：

- 复杂 benchmark 中仍出现大量状态推进和内部请求。
- 不能证明旧的细粒度控制动作已经被有效替代。
- 缺少 `legacy_action_displacement_rate` 和 `state_commit_adoption_rate` 的硬门槛。

必须完成：

- 把常见生命周期路径强制收敛为一个 `state_commit`：
  - create/finish node
  - result validity
  - result adoption
  - success criteria status
  - next best action
- 对 legacy fine-grained actions 增加 profile-level budget。
- 报告必须区分：
  - model-visible `state_commit`
  - runtime-synthesized state commit
  - legacy action fallback
  - rejected/partial commit retry

验收：

```text
state_commit_adoption_rate >= 80%
legacy_state_action_count <= 20% of v0.0.4 baseline
state_commit_rejection_rate <= 10%
```

### P0-4. Active Context Projection 没有证明正在替代高成本历史

已完成：

- `ContextProjectionV1` builder 和 projection event 路径存在。
- release decision 已要求 active projection evidence。

未完成：

- 最新诊断中 TaskSpace 每次请求平均仍有 35K-61K input tokens。
- 这说明 projection 没有把 provider-visible TaskSpace 上下文压到 v0.0.5 目标预算。
- 当前风险是“projection 作为附加摘要存在”，而不是“projection 替代旧历史”。

必须完成：

- 明确 active profile 的模型可见上下文组成。
- 禁止 `taskspace-v005-active` 同时携带完整旧 TaskSpace 历史和 projection。
- 增加 provider-visible prompt reconstruction 测试，直接检查：

```text
projection_tokens <= 12k
raw TaskSpace control history omitted
completed stale node history omitted
rejected subagent body omitted
large raw output omitted
protected evidence present
```

验收：

```text
avg_input_per_request_ratio <= 1.25x
projection_protected_miss_count = 0
active_context_replacement_confirmed = true
```

### P0-5. Subagent fanout 和节点扩张仍然失控

当前证据：

- `terminal-bench_E3-P0_3_2` TaskSpace 侧 5 次执行合计：
  - runtime events：`1,604`
  - maps/nodes/edges：`5 / 49 / 58`
  - subagent results：`46`
- `processing-pipeline/pair-001` 和 `multi-source-data-merger/pair-001` 分别出现大量 subagent results。

未完成原因：

- 现有 routing/no-spawn guard 对简单任务有效，但对 P0 外部样本约束不足。
- `record_subagent_plan` 的 yield gate 不足以阻止复杂任务中多次 fanout。

必须完成：

- 在 active profile 增加硬预算：

```text
max_spawn_agent_calls_per_task
max_parallel_subagent_results
max_nodes_per_route
max_open_leaf_nodes
```

- spawn 必须绑定明确 decision target，且结果必须被 adopt/reject/defer。
- 连续 no-yield 或未采用 subagent result 后，自动禁用同类 spawn。

验收：

```text
spawn_agent_call_count <= profile budget
subagent_no_decision_yield = 0
unreviewed_subagent_result_count = 0
```

### P0-6. Budget response 仍是事后诊断，不是执行控制

目标文档写过：

```text
warn -> compact -> state_commit checkpoint -> thin downgrade -> hard stop
```

当前状态：

- 报告能指出 cost gate FAIL。
- 但真实执行中没有阻止 `189` 个 TaskSpace 内部请求、`11.5M` token 的单 pair 继续扩大。

必须完成：

- budget manager 接入 runtime 或 benchmark active profile。
- 每次模型请求后更新预算状态。
- 超预算时不允许继续同一路径扩张：
  - 禁止新增 subagent。
  - 禁止新增非必要 inspect node。
  - 要求 finish current node 或进入 final/abort。

验收：

```text
budget_violation_detected_during_run = true
budget_response_action_taken = true
post_budget_spawn_count = 0
post_budget_request_count <= configured grace limit
```

### P0-7. 正确率未下降尚未坐实

当前事实：

- 内部 5x5 fixture 的 `24/25` 不是正式 E3。
- Terminal-Bench P0 诊断变体是 `Standard 4/5`，`TaskSpace 3/5`。
- 因此 v0.0.5 不能继续沿用“正确率未下降已基本坐实”的收口判断。

必须完成：

- 在同口径样本上重新验证：
  - 至少 `terminal-bench_E3-P0_3_5`。
  - 或 v0.0.4 clean 15-run 同样本。
- 在跑正式 E3 前，先完成 P0 成本与 harness 阻塞项，避免再次浪费真实 agent 调用。

验收：

```text
TaskSpace solved >= Standard solved - 1
score_valid = true
engineering_clean = true
no invalid_harness sample
```

## 4. P1 未完成项

### P1-1. `multi-source-data-merger` harness / validator eligibility 问题

当前证据：

`multi-source-data-merger` 在 `pair-001` 后 abort：

```text
e3_external_validator_fidelity_unproven
e3_external_validator_not_e3_eligible
no_tests_started_marker
public_validation_timeout
```

影响：

- 样本损失，导致计划 6 pair 只完成 5 pair。
- 正式 E3 如果不先修，会再次中止或产生 invalid_harness。

必须完成：

- 修复 external validator eligibility 判定。
- 确保 public validation 有稳定 started marker。
- 区分 validator timeout 与 agent timeout。
- 为这个样本单独跑非 agent harness self-test，再跑低成本 dry diagnostic。

### P1-2. 实验命名和证据等级已修正，但仍需严格执行

已完成：

- 命名规范改为 `数据集_子集_sample数量_repeats次数`。
- 文档明确 `terminal-bench_E3-P0_3_2` 是诊断变体，不是正式 E3。

未完成：

- 旧文档中仍有“内部 5x5 矩阵收口”的口径残留。
- 需要所有 v0.0.5 总结文档显式标记已废止或被新诊断覆盖。

必须完成：

- `15-closeout-summary.md` 标记为 superseded。
- README 把 `17-unfinished-work-inventory.md` 作为当前状态入口。
- 后续所有结果必须写明：
  - dataset
  - subset
  - samples
  - repeats
  - runner
  - evidence level
  - 是否正式 E3

### P1-3. Release decision 需要阻止 shadow/report-only 被误判为 active success

已完成：

- release decision 已经要求 active projection evidence。

未完成：

- 需要更强地检查 active profile 是否真的替换 provider-visible context，而不是仅生成 artifact。
- map management 当前是 report-only foundation，不能被计入 runtime cost control success。

必须完成：

- `release-decision` 增加：

```text
active_context_replacement_gate
runtime_budget_response_gate
state_commit_displacement_gate
spawn_budget_gate
```

### P1-4. Cost attribution 仍缺少执行阶段分解

当前可见：

- request count
- input/output/cached/uncached tokens
- runtime event count
- map/node/edge/subagent count

缺口：

- 每个 internal request 属于哪个阶段：
  - projection
  - node update
  - subagent spawn/result
  - synthesis
  - validation recovery
  - budget recovery
- 每个 request 的 latency、queue、provider retry、tool wait 分解。

必须完成：

- 在 request-summary 增加 `request_phase`。
- 在 runtime event 与 model request 之间建立 trace id join。
- 输出 top cost phases。

## 5. P2 未完成项

### P2-1. Map self-management 仍停留在 report-only foundation

这是 v0.0.5 修正文档允许的范围，但不能算成本控制完成。

后续需要：

- retention/salience 接入 active projection。
- archived/audit-only 项默认不进入模型可见上下文。
- protected evidence invariant 保持硬约束。

### P2-2. Routing 是 benchmark-profile controlled，不是产品 runtime routing

这是 v0.0.5 修正文档允许的范围，但不能算通用产品能力完成。

后续需要：

- 从 benchmark manifest routing 过渡到产品级 task classifier。
- 用户真实任务中也能产生 thin/default/deep route decision。
- route mistake report 可用于训练下一轮策略。

### P2-3. Clean gate / utility warning 分层仍需改进

问题：

- raw success、engineering clean、utility cost warning、release blocker 混在一起时容易误读。

后续需要：

- 明确四层：
  - business correctness
  - validation fidelity
  - engineering cleanliness
  - utility/cost
- 报告表格中默认分列展示，避免再次把内部 fixture 的 raw success 当作正式 E3 success。

## 6. 需要修正的旧判断

以下旧判断应视为被 2026-06-19 诊断推翻或至少暂停：

| 旧判断 | 修正 |
|---|---|
| v0.0.5 可以按阶段性成果收口 | 暂停。Terminal-Bench P0 诊断显示成本和正确率均未支持收口 |
| 主要代码建设约 90% 完成 | 改为“结构模块存在，但 active cost-control path 未完成” |
| 正确率基本守住 | 仅限内部 fixture；对 Terminal-Bench P0 不成立 |
| 成本优化可全部放到下一版本 | 不成立。v0.0.5 目标本身包含实际成本控制 |
| map/report/routing artifact 足以支撑 release | 不足。必须证明 active profile 影响 provider-visible execution |

## 7. 建议的继续开发顺序

### Step 1. 先关掉误收口风险

- 标记 `15-closeout-summary.md` 为 superseded。
- 更新 README 当前状态。
- 所有后续报告禁止把内部 E2/E1 fixture 称为正式 E3。

### Step 2. 修 P0 执行控制，不跑真实 E3

禁止真实 E3 / agent 调用，直到以下非 agent 验证通过：

- active context replacement prompt reconstruction test
- state_commit displacement synthetic test
- budget response synthetic test
- spawn budget synthetic test
- external harness eligibility test for P0 samples

### Step 3. 跑低成本 targeted diagnostic

只在代码完成后，先跑一个低成本但有针对性的诊断：

```text
terminal-bench_E3-P0_1_1
```

选择当前最能暴露问题的样本，例如 `processing-pipeline` 或 `recover-accuracy-log`。

目标不是证明 release，而是确认：

```text
request count 不再百倍膨胀
spawn 不再无预算扩张
active projection 确实降低 input/request
```

### Step 4. 再跑 `terminal-bench_E3-P0_3_5`

只有 Step 3 达到工程门槛后，才执行正式对比变体：

```text
terminal-bench_E3-P0_3_5
```

## 8. 当前收口判断

v0.0.5 当前状态：

```text
NOT READY TO CLOSE
```

允许说：

```text
v0.0.5 已经完成一批成本治理基础设施和诊断基建。
```

不能说：

```text
v0.0.5 已达到成本控制目标。
v0.0.5 已坐实正确率未下降。
v0.0.5 可以关闭并把成本优化全部放到下一版本。
```

下一步应该继续实现 P0 成本控制闭环，且在代码完成前不再运行真实 E3。

详细设计、代码落点、阶段门禁和非 agent 验证计划见：

```text
18-unfinished-work-engineering-design.md
```

# v0.0.5 未完成项工程设计

- Created: 2026-06-19
- Updated: 2026-06-19
- Version: v0.0.5-continuation-design
- Status: Draft
- Owner / Responsible: WhaleCode core engineering
- Related Systems: `action_map` runtime, `taskspace_control` handler, `spawn_agent` gates, TaskSpace context projection, benchmark cost/release scripts
- Related Links: `17-unfinished-work-inventory.md`, `10-implementation-plan.md`, `13-design-corrections-and-engineering-contract.md`, `16-terminal-bench_E3-P0_3_2-variant-run.md`
- Risk Level: High
- Plan Type: Full

## 1. 背景

v0.0.5 已经实现了若干基础模块：`state_commit`、output reference、context projection、map-management report、routing decision、cost instrumentation 和 release decision gate。但 `terminal-bench_E3-P0_3_2` 诊断变体显示，TaskSpace 仍然在真实 benchmark 路径上出现请求数、token 和 wall time 的数量级放大。

当前问题不是“没有记录指标”，而是记录到的指标没有反向控制执行。v0.0.5 继续开发必须把成本治理从 artifact/report 推进到 active execution path。

## 2. 问题定义

### 当前行为

- TaskSpace 能生成 projection、state_commit、output-ref、routing 和 cost artifact。
- 复杂 P0 样本仍能产生 33 到 189 次 TaskSpace 内部模型请求。
- subagent result、节点扩张、projection/context state update 仍可持续消耗模型请求。
- release decision 能判定 FAIL，但不能在执行中阻止继续扩张。

### 期望行为

- `taskspace-v005-active` 不是观测 profile，而是执行约束 profile。
- TaskSpace 在运行中知道预算并做出收缩动作。
- active projection 真正成为模型可见 TaskSpace surface，不能和完整历史叠加。
- subagent fanout、节点数、legacy action、request count 都有硬上限。
- 未达到工程门槛前，不再执行正式 E3。

### 差距

| 差距 | 影响 |
|---|---|
| cost budget 只在事后报告 | 失控 run 会完整烧完 token 和时间 |
| projection 不足以证明替换旧历史 | input/request 仍然很高 |
| `state_commit` 存在但 displacement 不明 | 协议压缩收益无法保证 |
| subagent/node budget 不够硬 | 复杂样本继续 fanout |
| release gate 缺少 active-effect 证明 | report-only 模块可能被误认为 runtime 成功 |

## 3. 目标

### Release Target

```text
TaskSpace solved >= Standard solved - 1
TaskSpace direct input+output ratio <= 2.0x Standard
TaskSpace agent walltime ratio <= 2.0x Standard
```

### Engineering Re-entry Target

在允许再次运行正式 `terminal-bench_E3-P0_3_5` 前，必须先达到：

```text
rollout_trace_model_request_count_ratio <= 2.5x on targeted diagnostic
avg_input_per_request_ratio <= 1.25x on targeted diagnostic
spawn_agent_call_count <= route budget
active_context_replacement_confirmed = true
budget_response_action_taken when budget exceeds threshold
```

### Diagnostic Target

每个 TaskSpace request 必须能归因到一个阶段：

```text
startup
projection
state_commit
legacy_state_action
ordinary_tool_recovery
subagent_spawn
subagent_result_processing
validation_recovery
final_synthesis
budget_recovery
unknown
```

## 4. 非目标

- 不在本轮引入新的 agent 角色或新 multi-agent 产品能力。
- 不移除 legacy `taskspace_control` action；只给 active profile 增加预算和 displacement gate。
- 不物理删除 audit artifact。
- 不把内部 E1/E2 fixture 当作正式 E3 结果。
- 不在代码未完成前运行真实 E3。

## 5. 约束和假设

| 项目 | 内容 |
|---|---|
| 用户约束 | 代码实际完成前禁止真实 E3 / 真实 agent 调用 |
| 工程约束 | 先用非 agent 单测、脚本 fixture、prompt reconstruction 测试验证 |
| 兼容约束 | v0.0.5 仍保留 legacy actions，避免破坏已有 TaskSpace 路径 |
| 安全约束 | 不删除 audit evidence；archive/audit-only 是投影策略，不是物理删除 |

| Assumption | Verification Method | If Assumption Fails |
|---|---|---|
| 现有 runtime event 可与模型请求建立 join | 检查 rollout trace、observability timeline、metrics extractor | Phase 0 先补 trace id 和 request phase 字段 |
| active projection 已有可复用 builder | Rust 单测和 prompt reconstruction | 若 builder 缺字段，先补 protected item source |
| spawn_agent gate 可在 runtime 层阻断 | `action_map/runtime.rs` 单测 | 若工具层先于 runtime 暴露，需在 tool registry 或 handler 增加 profile guard |
| benchmark profile 可传入 v005 budget | benchmark runner fixture | 若 profile 传递不稳定，先固定 config schema |

## 6. 总体技术设计

### 6.1 新增 TaskSpace Active Budget Contract

新增 `TaskSpaceActiveBudgetV1`，作为 active profile 的 runtime contract。

建议字段：

```text
profile_name
max_rollout_model_requests
max_model_requests_per_node
max_spawn_agent_calls
max_subagent_results
max_nodes
max_open_leaf_nodes
max_legacy_state_actions
max_projection_tokens
max_avg_input_tokens_per_request
post_budget_grace_requests
budget_response_policy
```

默认策略：

| 场景 | 初始预算 |
|---|---:|
| thin | requests <= 4, spawn = 0, nodes <= 4 |
| verification_first | requests <= 6, spawn = 0, nodes <= 5 |
| default_compact | requests <= 10, spawn <= 2, nodes <= 8 |
| subagent_assisted | requests <= 14, spawn <= 3, nodes <= 10 |
| deep | requests <= 20, spawn <= 4, nodes <= 14 |

这些值是工程保护阈值，不是最终产品承诺。若正式 P0 样本证明过紧，再通过 evidence 调整。

### 6.2 Budget State 进入 runtime

在 `ActionMapRuntimeState` 增加预算状态，而不是只在 benchmark 后处理：

```text
active_budget
budget_counters
budget_violations
budget_response_state
```

预算计数至少包含：

```text
rollout_model_request_count
node_model_request_count
spawn_agent_call_count
subagent_result_count
node_count
open_leaf_node_count
legacy_state_action_count
state_commit_count
projection_tokens_last
projection_tokens_max
```

预算状态机：

```text
normal -> warned -> compact_checkpoint_required -> thin_downgraded -> hard_stopped
```

### 6.3 Budget Response Gate

触发条件：

| Trigger | Response |
|---|---|
| request budget > 80% | warn + projection hint |
| request budget exceeded | require compact checkpoint / state_commit |
| spawn budget exceeded | block `spawn_agent` |
| node budget exceeded | block new node except validation/final recovery |
| legacy action budget exceeded | require `state_commit` |
| projection tokens exceeded twice | compact lower-priority items or block continuation |
| post-budget grace exceeded | hard stop with actionable final/abort instruction |

Gate 输出必须使用结构化 recovery：

```text
allowed
reason
budget_status
blocking_items
next_valid_actions
missing_evidence
```

### 6.4 Active Context Replacement Proof

新增 prompt reconstruction 测试和 artifact：

```text
active-context-replacement-report.json
```

字段：

```text
active_profile_enabled
projection_present
legacy_taskspace_history_present
raw_taskspace_control_history_tokens
completed_stale_node_history_tokens
rejected_subagent_body_tokens
large_raw_output_tokens
protected_items_present
projection_tokens
replacement_confirmed
violations
```

Release gate 只在 `replacement_confirmed=true` 时允许 cost PASS/PARTIAL。

### 6.5 StateCommit Displacement Gate

新增 `state-commit-displacement.json`：

```text
state_commit_count
runtime_state_commit_count
legacy_state_action_count
legacy_state_action_by_name
state_commit_adoption_rate
state_commit_rejection_rate
legacy_displacement_rate
```

active profile 规则：

- cognitive checkpoint 必须用 `state_commit`。
- lifecycle bundle 必须用 `state_commit`。
- legacy 单动作只允许作为 focused correction。
- legacy 超预算后，handler 返回 `state_commit_required`。

### 6.6 Spawn And Node Budget Gate

现有 runtime 已有若干 spawn/node guard，但它们是结构规则，不是 route budget。需要新增 profile budget gate：

```text
route_budget.max_spawn_agent_calls
route_budget.max_subagent_results
route_budget.max_nodes
route_budget.max_open_leaf_nodes
```

阻断规则：

- thin/verification-first 默认 `spawn=0`。
- 每个 spawn 必须引用 `record_subagent_plan`，且计划必须有 decision target。
- subagent result 必须在 N 个主 agent steps 内 `adopt/reject/defer`。
- no-yield result 达到阈值后，禁用同类 spawn。

### 6.7 Request Phase Attribution

在 cost instrumentation 中补足阶段归因：

```text
request_phase_summary.json
```

字段：

```text
phase
model_request_count
input_tokens
output_tokens
cached_input_tokens
uncached_input_tokens
wall_time_ms
representative_events
unknown_reason
```

需要 runtime event 与 model request 通过 trace id 或近邻时间窗关联。无法关联时必须标记 `unknown`，不能归零。

### 6.8 Release Decision 新门禁

`write-release-decision.ps1` 增加 blockers：

```text
active_context_replacement_gate_failed
runtime_budget_response_gate_failed
state_commit_displacement_gate_failed
spawn_budget_gate_failed
request_phase_attribution_missing
```

Clean release 必须同时满足：

```text
cost gate pass
quality gate pass
active replacement pass
budget response pass
state_commit displacement pass
spawn/node budget pass
required artifact pass
```

## 7. 代码落点

| 模块 | 文件 | 设计动作 |
|---|---|---|
| Runtime budget state | `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` | 增加 budget contract、counter、state machine、gate recovery |
| TaskSpace handler | `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs` | 对 legacy actions 增加 active profile budget 和 `state_commit_required` recovery |
| Tool schema/profile | `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`、`tool_config.rs`、`tool_registry_plan.rs` | 暴露 compact active profile，收窄 legacy action 诱导面 |
| Spawn tool gating | `third_party/codex-cli/codex-rs/tools/src/agent_tool.rs`、runtime spawn checks | profile 下隐藏/限制 spawn，或 runtime 统一阻断 |
| Context projection | `action_map/runtime.rs` projection builder | 增加 active replacement proof hooks、protected item report |
| Metrics extraction | `scripts/taskspace-benchmark/lib/metrics-extractor.ps1` | 解析新增 artifacts |
| Cost instrumentation | `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1` | 写 request phase、state commit displacement、active replacement |
| Release decision | `scripts/taskspace-benchmark/write-release-decision.ps1` | 加新 gate 和 blockers |
| Harness tests | `scripts/taskspace-benchmark/test-harness.ps1`、`test-release-decision.ps1` | 增加 synthetic pass/fail fixture |

## 8. 阶段计划

### Phase 0: Artifact And Trace Discovery

#### Objective

确认 runtime event、rollout trace、model request usage 之间能否建立足够稳定的关联。

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| 当前 P0 诊断 artifact 可读 | 读取 `request-summary.json` / pair metrics | artifact index | core engineering |
| runtime event 有时间或 trace 线索 | 静态检查 observability timeline | field coverage note | core engineering |

#### Implementation Tasks

- 写一个非 agent parser fixture，输入已存在诊断 artifact。
- 输出 request 与 runtime event 的可关联字段清单。
- 标记缺失字段，不猜测。

#### Deliverables

- `request-phase-field-audit.md`
- parser fixture

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| 缺字段处理 | synthetic JSONL | missing 不被当成 0 |
| phase enum | unit fixture | unknown 有 reason |

#### Exit Criteria

- 明确能否直接做 request phase attribution。
- 如果不能，列出 runtime 必须新增的 trace fields。

#### Review Plan

- 对照 `terminal-bench_E3-P0_3_2` 结果检查 phase 覆盖率。

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| 无法精确 join request/event | phase attribution 粗糙 | unknown > 30% | 增加 trace id | 暂用时间窗但标记 low confidence |

#### Gate To Next Phase

`request_phase_unknown_ratio` 有可解释上限，或新增 trace field 设计已定。

### Phase 1: Active Budget Runtime Contract

#### Objective

让 TaskSpace active profile 在运行中知道预算，并能阻断继续扩张。

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| Phase 0 字段清楚 | 查看 audit | phase audit | core engineering |
| existing runtime budget tests 可跑 | `cargo test -p codex-core budget` | test output | core engineering |

#### Design Approach

- 在 runtime 内维护 budget counters。
- Gate 不直接猜测业务语义，只限制动作类别和扩张行为。
- 超预算 recovery 必须给模型可执行下一步。

#### Implementation Tasks

- 增加 `TaskSpaceActiveBudgetV1` 类型。
- 增加 `TaskSpaceBudgetCounters`。
- 在 ordinary tool、spawn、create node、legacy action、projection build、state_commit 入口更新 counters。
- 增加 `BudgetViolation` trace event。
- 增加 hard stop 状态。

#### Deliverables

- Rust budget state implementation
- `budget-events.jsonl`
- budget gate unit tests

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| request budget | synthetic runtime test | 超预算后 ordinary expansion blocked |
| spawn budget | unit test | thin route spawn 被阻断 |
| legacy action budget | unit test | 超预算返回 `state_commit_required` |
| grace window | unit test | grace 后 hard stop |

#### Exit Criteria

- 非 agent Rust tests 覆盖 normal/warn/downgrade/hard_stop。
- budget event 可被 cost instrumentation 读取。

#### Review Plan

- 审查 gate 是否可能阻断必要 validation/final recovery。

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| 预算过紧导致正确率下降 | TaskSpace 提前停 | validation node 无法执行 | validation/final recovery 白名单 | profile 降级为 warn-only |

#### Gate To Next Phase

budget gate 单测全部通过，且没有破坏现有 `state_commit` / node contract 测试。

### Phase 2: Active Context Replacement Proof

#### Objective

证明 active projection 不是附加摘要，而是替代了高成本 TaskSpace 历史。

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| active projection builder 可用 | Rust projection tests | test output | core engineering |
| protected item list 明确 | 对照 contract | checklist | core engineering |

#### Design Approach

- 不直接改变业务语义；先增加 prompt reconstruction proof。
- active profile 下发现旧历史叠加时，release gate 失败。
- protected items missing 时，runtime gate 失败。

#### Implementation Tasks

- 增加 active prompt reconstruction helper。
- 输出 `active-context-replacement-report.json`。
- 增加 protected item enumerator。
- 增加 violations：
  - `legacy_taskspace_history_present`
  - `raw_output_replay_present`
  - `projection_over_budget`
  - `protected_item_missing`

#### Deliverables

- reconstruction helper
- report artifact
- release decision gate

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| active replacement | synthetic prompt fixture |旧 TaskSpace history 不出现 |
| protected item | fixture | user requirement / failed validator present |
| raw output | large output fixture | >50KB raw absent |

#### Exit Criteria

`active_context_replacement_confirmed=true` 能由非 agent fixture 证明。

#### Review Plan

- 审查是否把必要 validator evidence 错误 elide。

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| projection 丢上下文 | 正确率下降 | protected miss | hard fail active profile | 回退 shadow profile |

#### Gate To Next Phase

active replacement fixture PASS，release decision 能拒绝 shadow-only / replacement-failed run。

### Phase 3: StateCommit Displacement

#### Objective

让 `state_commit` 从“可用能力”变成 active profile 默认状态更新路径。

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| state_commit tests 当前通过 | `cargo test -p codex-core state_commit` | test output | core engineering |
| legacy action list 完整 | handler enum audit | action list | core engineering |

#### Design Approach

- 不删除 legacy actions。
- active profile 为 legacy actions 增加 budget。
- gate recovery 给出等价 `state_commit` 模板。

#### Implementation Tasks

- 统计 legacy action by name。
- 对 finish/adopt/validity/criteria/decision 等常见组合返回 batch commit hint。
- 增加 `state-commit-displacement.json`。
- release decision 增加 adoption/displacement gate。

#### Deliverables

- runtime counters
- displacement artifact
- release gate

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| legacy budget | unit test | 超预算后阻断并给 `state_commit` recovery |
| adoption rate | synthetic metrics | rate 正确 |
| release gate | fixture | low adoption fails |

#### Exit Criteria

active profile 下 legacy update 不再能无限发生。

#### Review Plan

- 审查 recovery template 是否完整覆盖 output_contract/fact_source/success_criteria。

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| 大 state_commit payload 导致重试 | request 增加 | rejection_rate > 10% | section templates and dry_run | focused legacy fallback |

#### Gate To Next Phase

`state_commit_adoption_rate` 和 `legacy_state_action_count` 可由 fixture gate。

### Phase 4: Spawn And Node Budget

#### Objective

阻止复杂样本中无预算 fanout 和节点膨胀。

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| route decision artifact 可用 | routing fixture | `routing-decision.json` | core engineering |
| spawn runtime tests 可跑 | `cargo test -p codex-core spawn_agent` | test output | core engineering |

#### Design Approach

- route 决定预算，不由模型自由扩张。
- runtime enforce，不只写 prompt。
- subagent result 必须快速进入 adoption lifecycle。

#### Implementation Tasks

- 在 budget contract 中接入 route mode。
- thin/verification-first 下默认 block spawn。
- default/deep 下限制 spawn/node/open leaf。
- 增加 no-yield result cooldown。
- 增加 unreviewed subagent result gate。

#### Deliverables

- route budget enforcement
- spawn budget artifact
- route adherence report 扩展

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| thin spawn | unit test | spawn blocked |
| subagent result adoption | fixture | unresolved result blocks further spawn |
| node budget | unit test | 超预算 create_node blocked |

#### Exit Criteria

复杂 route 也不能超过 profile budget，除非 explicit escalation event 记录原因。

#### Review Plan

- 审查 broad inspect delegation 是否仍有可行路径，不把所有复杂任务退化成单 agent 串行。

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| fanout 限制过强 | 复杂任务正确率下降 | hidden oracle fail | allow explicit deep escalation | 提高 deep budget |

#### Gate To Next Phase

spawn/node budget fixture PASS，release decision 能拒绝 budget exceeded run。

### Phase 5: Harness Eligibility And Release Gates

#### Objective

先修正式 E3 可信度问题，避免再次跑到 invalid_harness。

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| P0 runtime gates 已有 fixture | test outputs | pass logs | core engineering |
| P0 sample list 固定 | experiments doc | sample contract | core engineering |

#### Implementation Tasks

- 修 `multi-source-data-merger` validator eligibility/start marker/timeout 分类。
- `write-release-decision.ps1` 接入新 gate。
- `test-release-decision.ps1` 增加 pass/fail fixture。
- 所有正式结果强制显示 dataset/subset/sample/repeats/runner/evidence level。

#### Deliverables

- updated release decision
- updated harness tests
- P0 sample eligibility report

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| invalid harness fixture | script test | release fail reason 精确 |
| missing active gate | script test | clean release blocked |
| sample metadata | script test | 缺 metadata fails |

#### Exit Criteria

P0 samples 不会因为已知 eligibility/start marker 问题中途损失覆盖。

#### Review Plan

- 审查 release blocker 是否会混淆 raw correctness 和 utility warning。

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| harness gate 过严 | 可用结果被 invalid | false invalid | 分层 blocker/warning | 人审 override 但不能 clean pass |

#### Gate To Next Phase

非 agent harness tests PASS，P0 sample eligibility report clean。

### Phase 6: Targeted Diagnostic Then Formal E3

#### Objective

只在工程门槛达成后重新调用真实 agent。

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| Phase 1-5 gates PASS | test summary | non-agent validation report | core engineering |
| 用户批准真实 agent run | explicit instruction | chat record | user |

#### Execution Order

1. 先跑 `terminal-bench_E3-P0_1_1` targeted diagnostic。
2. 如果 request/token/spawn/budget gate 达标，再跑 `terminal-bench_E3-P0_3_5`。
3. 如果仍失败，先回到 Phase 0/1 做归因，不扩大样本。

#### Passing Standard

```text
targeted diagnostic:
  rollout_trace_model_request_count_ratio <= 2.5x
  avg_input_per_request_ratio <= 1.25x
  spawn/node budget pass
  active replacement pass

formal P0:
  TaskSpace solved >= Standard solved - 1
  direct input+output <= 3x for engineering partial
  walltime <= 3x for engineering partial
  no invalid_harness sample
```

## 9. 测试策略

必须先跑非 agent 测试：

```powershell
cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-core state_commit -- --nocapture
cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-core projection -- --nocapture
cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-core budget -- --nocapture
cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-core spawn_agent -- --nocapture
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\v005-continuation-harness-selftest
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1 -RunRoot target\v005-continuation-release-selftest
```

若新增测试名不同，以实际实现后的 focused test 名称为准，但必须覆盖同等门禁。

## 10. 风险与回滚

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| active budget 破坏正确率 | TaskSpace 提前停止 | correctness fail | validation/final recovery 白名单 | profile downgrade to warn-only |
| active projection 丢证据 | 错误 patch/final | protected miss | hard fail active replacement | shadow profile |
| state_commit 强制导致重试 | request 数增加 | rejection rate 高 | dry-run/template | legacy correction allowance |
| spawn 限制过严 | 复杂任务失败 | deep task fail | explicit escalation event | deep budget override |
| release gate 过严 | 无法收口 | false blocker | blocker/warning 分层 | 人审 override，不允许 clean pass |

## 11. 决策记录

| Decision | Reason |
|---|---|
| 先做 runtime budget，再跑 E3 | 当前问题是执行失控，继续跑只会消耗 token |
| active replacement 要有 proof artifact | projection artifact 本身不能证明降本 |
| `state_commit` 做 displacement gate | handler 存在不等于模型采用 |
| spawn/node 做硬预算 | P0 诊断显示 fanout 是主要放大器之一 |
| map self-management 仍不作为 P0 runtime mutation | v0.0.5 修正文档已定义为 report-only foundation |

## 12. 开放问题

1. model request 与 runtime event 的可靠 join key 是否已经存在，还是必须新增 trace id？
2. active budget 的默认阈值是否按 route mode 固定，还是 benchmark scenario 可覆盖？
3. budget hard stop 时应该返回用户可见中止，还是转 final_synthesis 输出 partial failure？
4. state_commit adoption gate 对不同任务复杂度是否需要不同阈值？
5. `terminal-bench_E3-P0_1_1` targeted diagnostic 首选样本应是 `processing-pipeline` 还是 `recover-accuracy-log`？

## 13. Plan Quality Checklist

- [x] 问题定义与目标分离。
- [x] 目标可测量。
- [x] 约束、假设、风险和开放问题分离。
- [x] 阶段计划有入口、任务、交付物、验证、退出和回退。
- [x] 明确禁止代码完成前真实 E3。
- [x] 包含 runtime、handler、tool schema、benchmark scripts 的代码落点。
- [x] 包含 release gate 和 rollback/fallback。

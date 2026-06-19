# v0.0.5 未完成项工程设计

- Created: 2026-06-19
- Updated: 2026-06-19
- Version: v0.0.5-continuation-design
- Status: Approved for Phase 0A-5 implementation after adversarial review; formal E3 remains forbidden until code-complete, non-agent gates, user approval, and start gate all pass.
- Owner / Responsible: WhaleCode core engineering
- Related Systems: `action_map` runtime, `taskspace_control` handler, `spawn_agent` gates, TaskSpace context projection, benchmark cost/release scripts
- Related Links: `17-unfinished-work-inventory.md`, `10-implementation-plan.md`, `13-design-corrections-and-engineering-contract.md`, `16-terminal-bench_E3-P0_3_2-variant-run.md`
- Risk Level: High
- Plan Type: Full

## 0.1 当前可执行状态

本文件已通过方案层对抗性审查，可作为 v0.0.5 Phase 0A-5 的工程实现入口。这里的“可执行”只允许进入代码实现、非 agent fixture、自测和 gate 建设；不允许把 v0.0.5 标记为可关闭，也不允许跳过 code-complete / non-agent gates / user approval / E3 start gate 去运行真实 E3。

后续 fresh agent 必须按以下顺序执行：

1. 先补齐 Phase 0A-5 的代码、producer-owned artifacts、negative fixtures 和 release/start gate。
2. 再生成 `v005_non_agent_gates.json`、`v005_code_complete.json`、`v005_user_approval.json` 所需的真实证据。
3. 只有 start gate 输出 `full_e3_allowed=true`，才允许运行 `terminal-bench_E3-P0_3_5`。
4. 任何 diagnostic-only 变体只能用于工程健康检查，不能作为 release proof。

## 0. 本文档优先级和本轮审查修正

本文档是 v0.0.5 未完成项继续开发的当前执行方案。若本文档与 `10-implementation-plan.md` 的 Phase 6 样本安排、PASS/PARTIAL/FAIL 发布口径、或 report-only routing 文字冲突，以本文档为准。

本轮对抗性审查后，新增以下硬约束：

1. v0.0.5 正式收口验证的主样本集必须是 `terminal-bench_E3-P0_3_5`。`terminal-bench_E3-v004-clean_3_5` 只能用于与 v0.0.4 clean 15-run 做同口径回归对比，不能替代 P0 成本/正确率结论。
2. 低成本诊断不得命名或报告为正式 E3。`terminal-bench_E3-P0_1_1`、`terminal-bench_E3-P0_3_1`、`terminal-bench_E3-P0_3_2` 均属于 `reported_evidence_level=diagnostic-only` 或 `E3-candidate`，不得进入 release_pass。
3. provider request lifecycle 的 canonical producer 是 `client.rs` / `ModelClientSession` provider dispatch 和 stream lifecycle。ActionMap 只能提供 request context 和预算策略，不能事后用 snapshot 推断 provider request 的 phase/node。
4. active context replacement 必须落在 `session/turn.rs` provider-visible history composition 边界，先改变实际 request input，再由 payload scan 证明结果；不得只生成 projection/report artifact。
5. release 和 E3 start gate 只能聚合 producer-owned typed artifacts；脚本不得成为事实来源。正式 release 必须绑定 sample set、repeats、runner、start-gate decision、user approval sample set 和 non-agent gate evidence。
6. 成本 hard stop 必须带质量补偿策略。任何 budget-induced validation skip、early final、final abort、thin downgrade 或 no-spawn 必须记录质量影响和 bounded recovery/escalation 结果。

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
TaskSpace model_request_count ratio <= 2.0x Standard
```

`model_request_count_ratio <= 2.5x` 只能进入 `blocked_partial`，不得产生 `release_pass`。固定 route/request/spawn/node 上限只是 safety cap；正式 P0 release proof 必须同时计算 TaskSpace/Standard ratio。

### Engineering Re-entry Target

在允许再次运行正式 `terminal-bench_E3-P0_3_5` 前，必须先达到：

```text
provider_request_hook_coverage >= 99%
request_phase_attribution_coverage >= 95%
rollout_trace_model_request_count_ratio <= 2.5x on targeted diagnostic
avg_input_per_request_ratio <= 1.25x on targeted diagnostic
spawn_agent_call_count <= route budget
active_context_replacement_confirmed = true
budget_response_action_taken when budget exceeds threshold
```

Re-entry 的 `2.5x` 是继续诊断或进入 formal P0 的工程门槛，不是 v0.0.5 clean release target。

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
| provider/model request lifecycle has an instrumentation hook | Inspect client/session provider-call path and add fixture | Phase 0A blocks all budget work until hook is implemented |
| 现有 runtime event 可与模型请求建立 join | 检查 rollout trace、observability timeline、metrics extractor | Phase 0B 补 trace id 和 request phase 字段；不能只记录字段清单后放行 |
| active projection 已有可复用 builder | Rust 单测和 prompt reconstruction | 若 builder 缺字段，先补 protected item source |
| spawn_agent gate 可在 runtime 层阻断 | `action_map/runtime.rs` 单测 | 若工具层先于 runtime 暴露，需在 tool registry 或 handler 增加 profile guard |
| benchmark profile 可传入 v005 budget | benchmark runner fixture | 若 profile 传递不稳定，先固定 config schema |

## 6. 总体技术设计

### 6.1 Provider Request Instrumentation Contract

预算控制必须锚定 provider/model request 生命周期，而不是只锚定 `action_map` 的 node/tool/spawn 状态。`action_map` runtime 可以做语义预算决策，但 provider-call hook 必须负责记录和触发 request 级预算检查。

新增 `TaskSpaceProviderRequestEventV1`：

```text
schema_version
request_id
parent_request_id optional
session_id
task_id optional
map_id optional
node_id optional
route_mode optional
request_phase
budget_state_before
budget_state_after
provider
model
input_tokens optional
output_tokens optional
cached_input_tokens optional
uncached_input_tokens optional
prompt_payload_sha256 optional
prompt_payload_artifact optional
exact_payload_scan_event_id optional
started_at
completed_at optional
status
```

必须实现的位置：

| Hook | 目的 | Gate |
|---|---|---|
| before provider request dispatch | 增加 request counter，检查 hard-stop/budget state | 若 active budget 已 hard_stopped，禁止发起非 recovery request |
| after provider response | 记录 token、latency、status，更新 budget state | 若超预算，产生 budget response event |
| provider request payload capture/reconstruction | 为 active replacement proof 提供精确 payload | 缺 payload hash 时 active replacement gate fail |
| exact payload scanner | 在 payload 捕获后、redaction/hash-only fallback 前做负向检查 | 缺 exact scan proof 时 active replacement gate fail |

request phase 必须由调用上下文显式传入；无法判断时填 `unknown` 并计入 coverage failure，不能默认为正常。

#### 6.1.1 Canonical Provider Lifecycle Producer

`TaskSpaceProviderRequestEventV1` 的唯一 canonical producer 是 provider request 生命周期本身：

- `client.rs` / `ModelClientSession` 在 HTTP 和 WebSocket provider dispatch 前产生 `dispatch_started`。
- 同一个 request id 必须贯穿 stream opened、response completed、provider error、cancellation、blocked、retry/fallback attempt。
- terminal event 必须在 stream completion、stream error 或 cancellation 时产生，不能把 `stream_opened` 当作 `response_completed`。
- terminal event 必须尽量携带 token usage、latency、provider/model、transport、status、attempt id、parent logical request id。
- payload hash、payload artifact 或 exact pre-redaction scan event 必须在 request payload 构建后、redaction/hash-only fallback 前绑定到同一个 request id。

ActionMap runtime 的职责：

- 在 request construction 前提供 `TaskSpaceProviderRequestContextV1`，包括 `task_id`、`map_id`、`node_id`、`route_mode`、`request_phase`、budget policy 和 context selection reason。
- 消费 provider lifecycle events 并补充 runtime budget state。
- 不得在 event drain 时通过 `current_main_node_id`、ready node 或时间窗推断 provider request 的 node/phase。缺失时必须写 `provider_request_context_missing_reason`，并计入 coverage failure。

request id 规则：

```text
provider_request_id = session_id + turn_id + logical_request_seq + attempt_seq
parent_request_id = logical request id for retry/fallback attempts
```

`provider-request-{n}` 只能作为单 turn 内部调试 id，不满足 release-grade lifecycle evidence。

WebSocket warmup / `generate=false` 请求必须二选一：

- 作为 `request_phase=startup` / `warmup` 的 provider lifecycle event 记录，且不进入 inference cost denominator；
- 或明确 excluded，并在 gate 中记录 exclusion reason。不得完全静默。

### 6.2 新增 TaskSpace Active Budget Contract

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

### 6.3 Budget State 进入 runtime

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

### 6.4 Budget Response Gate

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

预算状态下允许动作矩阵：

| Budget State | ordinary tools | `taskspace_control(state_commit)` | legacy state actions | create node | spawn | validation/final recovery | final/abort |
|---|---|---|---|---|---|---|---|
| normal | allow | allow | allow within budget | allow within route budget | allow within route budget | allow | allow |
| warned | allow | prefer | allow within budget | allow within route budget | allow within route budget | allow | allow |
| compact_checkpoint_required | block except bounded read needed for checkpoint | require | block except focused correction | block except validation/final node | block | allow | allow |
| thin_downgraded | allow only current-node bounded action | require | block | block broad expansion | block | allow | allow |
| hard_stopped | block | allow only final checkpoint | block | block | block | allow if already started and bounded | require final/abort |

hard stop 不能隐藏 correctness failure：如果 validation 已失败或未执行，final/abort 必须输出 blocked reason 和 missing validation evidence，不能伪装成成功 final。

#### 6.4.1 Budget-Induced Quality Protection

Clarified contract after adversarial review:

预算动作不能用“少做任务”伪造成成本下降。任何 hard stop、thin downgrade、no-spawn、early final、final abort、validation skip 都必须产生 `BudgetQualityImpactV1`。该事件的 canonical producer 是 runtime budget gate；metrics extractor 和 release decision 只能消费该事件，不能事后根据 summary 猜测质量影响。

输出位置：

```text
budget-quality-impact-events.jsonl
budget_induced_quality_impact_summary.json
```

`BudgetQualityImpactV1` 必须至少包含：

```text
schema_version
sample_id optional
request_id optional
task_id optional
map_id optional
node_id optional
budget_action
budget_state_before
budget_state_after
counter_name
counter_value
counter_limit
validator_status_before
validator_status_after
missing_evidence_count
protected_item_miss_count
solve_risk
bounded_recovery_allowed
bounded_recovery_used
route_escalation_allowed
route_escalation_used
manual_override_allowed
manual_override_used
final_classification
score_eligible
reason
```

质量补偿和计分规则：

- `validation_required` 节点不得因普通 budget hard stop 被静默跳过。
- 如果预算动作导致 validator 未执行、关键证据缺失、protected item 缺失或 final synthesis 提前结束，该样本必须进入 `bounded_recovery` 或 `blocked_by_budget`，不得直接计为 solved。
- 每个样本最多允许一次 bounded recovery request；该 request 必须标记 `request_phase=validation_recovery` 或 `budget_recovery`，并写回同一个 `sample_id` 的 quality impact summary。
- bounded recovery 后仍缺关键证据时，`final_classification` 必须是 `unsolved`、`blocked_by_budget` 或 `invalid_harness`，`score_eligible=false`。
- `early_final` 只有在 validator 已通过且 `missing_evidence_count=0` 时才允许 `score_eligible=true`；否则必须视为 `blocked_by_budget`。
- `final_abort`、`validation_skip`、`protected_item_miss_count>0`、`manual_override_used=true` 均不得进入 `release_pass` 的 solved 计数。
- human/manual override 只能把 run 标为 `accepted-risk` 或 diagnostic continuation，不能把 blocked sample 改写为 clean solved。
- release report 必须按样本输出 `budget_induced_quality_impact_summary`，说明预算动作是否导致 solve loss、是否使用 recovery、是否仍可计分。

release blocker 规则：

```text
budget_quality_impact_missing_count > 0 => release_pass blocked
budget_induced_validation_skip_count > 0 => release_pass blocked
budget_induced_score_ineligible_solved_count > 0 => release_pass blocked
blocked_by_budget_samples_count > 0 => release_pass blocked_partial or fail
manual_override_used_count > 0 => release_pass blocked
```

新增验收：

```text
budget_quality_impact_logged_for_every_budget_action = true
budget_induced_validation_skip_count = 0
blocked_by_budget_samples_count reported
bounded_recovery_request_count <= sample_count
budget_induced_score_ineligible_solved_count = 0
manual_override_used_count = 0 for release_pass
```

预算动作不能用“少做任务”伪造成成本下降。任何 hard stop、thin downgrade、no-spawn、early final、final abort、validation skip 都必须产生 `BudgetQualityImpactV1`：

```text
schema_version
sample_id optional
request_id optional
task_id optional
map_id optional
node_id optional
budget_action
validator_status_before
validator_status_after
missing_evidence_count
protected_item_miss_count
solve_risk
bounded_recovery_allowed
bounded_recovery_used
route_escalation_allowed
route_escalation_used
final_classification
reason
```

质量补偿规则：

- `validation_required` 节点不得因普通 budget hard stop 被静默跳过。
- 若预算导致验证证据缺失，必须进入 bounded recovery 或 `blocked_by_budget`，不得直接声明 solved。
- 每个样本最多允许一次 bounded recovery request；该 request 必须标记 `request_phase=validation_recovery` 或 `budget_recovery`。
- 若 recovery 后仍缺关键证据，样本结果必须显式标记为 unsolved / blocked，不能以低成本成功计入。
- release report 必须按样本输出 `budget_induced_quality_impact_summary`，包括预算动作是否导致 solve loss。

新增验收：

```text
budget_quality_impact_logged_for_every_budget_action = true
budget_induced_validation_skip_count = 0
blocked_by_budget_samples_count reported
bounded_recovery_request_count <= sample_count
```

### 6.5 Active Context Replacement Implementation And Proof

Active replacement 的实现入口必须是 provider-visible history composition，而不是 artifact 后处理。

实现合同：

- 在 `session/turn.rs` 的 request input / history assembly 边界新增 `build_active_provider_visible_history(...)`。
- `taskspace-v005-active` 下，`clone_history().for_prompt(...)` 之前必须先应用 active projection replacement。
- replacement policy 只允许保留当前用户约束、active projection、当前节点必要证据、失败验证证据、最终回答所需 protected items 和 bounded recovery 所需最小上下文。
- 旧 TaskSpace control history、stale node history、rejected subagent body、大 raw output replay 不得进入 provider-visible request。
- exact payload scan 必须扫描上述函数实际输出的 request payload，而不是扫描 projection artifact、summary artifact 或 post-run reconstructed text。
- `active-context-replacement-report.json` 必须引用 `provider_request_id`、`provider_payload_sha256` 和 `exact_payload_scan_event_id`。

对应测试：

| Test | Requirement |
|---|---|
| composition boundary fixture | active profile 的 provider-visible history 不包含 raw TaskSpace history |
| protected evidence fixture | protected items 仍可进入 prompt |
| shadow artifact fixture | 只有 projection artifact、实际 payload 未替换时 release fail |
| exact scan fixture | scan event 与 provider request id/hash 完全一致 |

### 6.5.1 Active Context Replacement Proof

新增 exact provider payload reconstruction 测试和 artifact：

```text
active-context-replacement-report.json
```

字段：

```text
active_profile_enabled
provider_payload_available
provider_payload_sha256
provider_payload_artifact
exact_payload_scan_event_id
exact_payload_scan_passed
exact_payload_scan_provenance
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

Release gate 只在以下条件同时满足时允许 cost pass：

```text
provider_payload_available = true
provider_payload_sha256 != empty
provider_payload_artifact != empty OR exact_payload_scan_event_id != empty
exact_payload_scan_passed = true
replacement_confirmed = true
legacy_taskspace_history_present = false
large_raw_output_tokens = 0
protected_items_present = true
```

禁止仅凭 `projection-events.jsonl`、projection count、protected miss count 或 regex 扫描 artifact 认定 active replacement 成功。

hash-only evidence 只能用于 privacy-safe audit，不能单独满足 active replacement release proof。若不能保留可搜索 exact payload artifact，必须在 provider request path 内生成 `exact-payload-scan-events.jsonl`，并证明 scan 发生在同一个 `provider_payload_sha256` 对应 payload 上、redaction/hash-only fallback 之前。

`ExactPayloadScanEventV1` 必须包含：

```text
scan_event_id
request_id
provider_payload_sha256
scanner_version
matcher_version
checked_byte_ranges
checked_token_ranges optional
negative_checks_performed
legacy_taskspace_history_present
large_raw_output_tokens
raw_taskspace_control_history_tokens
completed_stale_node_history_tokens
rejected_subagent_body_tokens
protected_items_present
passed
failure_reasons
```

`legacy_taskspace_history_present=false`、`large_raw_output_tokens=0` 和 `replacement_confirmed=true` 的 provenance 必须指向 exact payload artifact 或 exact payload scan event，不能指向 projection artifact、summary artifact 或 post-run regex scan。

### 6.6 StateCommit Displacement Gate

新增 `state-commit-displacement.json`：

```text
state_commit_count
runtime_state_commit_count
legacy_state_action_count
legacy_state_action_by_name
state_commit_adoption_rate
state_commit_rejection_rate
state_commit_retry_followup_request_count
legacy_displacement_rate
```

active profile 规则：

- cognitive checkpoint 必须用 `state_commit`。
- lifecycle bundle 必须用 `state_commit`。
- legacy 单动作只允许作为 focused correction。
- legacy 超预算后，handler 返回 `state_commit_required`。
- state_commit retry pressure 必须单独计数；如果 batch commit 降低 action count 但提高 follow-up model requests，不能判定为有效压缩。

### 6.7 Spawn And Node Budget Gate

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

route mode 必须定义最小有效并行语义，避免把 TaskSpace 简化成永远单 agent：

| Route Mode | Spawn Default | Minimum Useful Parallelism | Escalation Rule |
|---|---|---|---|
| thin | blocked | none | validator failure plus independent evidence surface |
| verification_first | blocked | none | local checker cannot explain validator failure |
| default_compact | allowed only with decision target | two independent evidence surfaces | explicit route escalation event |
| subagent_assisted | allowed within budget | at least two bounded tracks with adoption path | no-yield disables same-class spawn |
| deep | allowed within budget | independent high-risk tracks | requires route reason and budget override record |

### 6.8 Request Phase Attribution

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

需要 runtime event 与 model request 通过 request id / trace id 关联。时间窗 join 只能作为诊断 fallback，不能作为 release gate 依据。无法关联时必须标记 `unknown`，不能归零。

Release 前硬门槛：

```text
provider_request_hook_coverage >= 99%
request_phase_attribution_coverage >= 95%
unknown_request_phase_ratio <= 5%
```

### 6.9 Release Decision 新门禁

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

Clean release 的 required artifacts 至少包括：

```text
provider-request-events.jsonl
budget-events.jsonl
request-phase-summary.json
active-context-replacement-report.json
exact-payload-scan-events.jsonl OR searchable provider payload artifact
state-commit-displacement.json
spawn-node-budget-summary.json
v005-non-agent-gates.json
```

`write-release-decision.ps1` 不允许继续把 `context-projection-summary.json` 当作 active replacement proof。projection summary 只能作为诊断输入；release proof 必须来自 exact provider-visible payload artifact，或来自同一 `provider_payload_sha256` 上、redaction/hash-only fallback 之前生成的 exact payload scan event。

任何以下情况都必须阻断 `release_pass`：

```text
provider request event missing
budget event missing
request phase attribution missing or below threshold
runtime budget response missing
active replacement proof hash-only without exact scan
exact scan request_id/hash mismatch
state_commit displacement missing
spawn/node budget summary missing
v005 non-agent gate artifact missing or stale
```

#### 6.9.1 Formal E3 Identity Gate

`release_pass` 的 required artifacts 还必须包括：

```text
start-gate/e3-start-gate.json
start-gate/gate-decision.json
```

任何以下情况都必须阻断 `release_pass`：

```text
formal E3 identity missing or mismatched
sample_set_id/repeats/runner/start_gate identity mismatch
user approval sample set mismatch
non-agent gate evidence path missing or not local/verifiable
terminal-bench_E3-P0_1_1 / _3_1 / _3_2 diagnostic variant enters release_pass
```

Formal E3 identity gate:

```text
sample_set_id = terminal-bench_E3-P0_3_5
benchmark_family = terminal-bench
runner_entrypoint = run-taskspace-e3-suite.ps1
repeats_per_sample >= 5
sample_names exactly match registered terminal-bench_E3-P0_3_5 executable samples
run-status.evidence_target = E3
pair_completed.reported_evidence_level = E3 for every counted pair
start_gate.full_e3_allowed = true
start_gate.v005_markers_passed = true
start_gate.calibration_gate_passed = true
start_gate.task_list_hash/profile_hash/source_version match release artifacts
user_approval.approved_sample_set_id = terminal-bench_E3-P0_3_5
```

`terminal-bench_E3-P0_1_1`、`terminal-bench_E3-P0_3_1` 和 `terminal-bench_E3-P0_3_2` 必须在 release decision 中降级为 diagnostic-only / E3-candidate；即使所有 pair 成功，也不得产生 `release_pass`。

`v005-non-agent-gates.json` 中每个 gate 必须是 producer-owned structured gate：

```text
name
status
producer
command
exit_code
generated_at
git_commit
profile_hash
evidence_path
evidence_sha256
```

`evidence_path` 必须指向存在的本地 artifact，且位于 run root、repo `target/`、或明确允许的 evidence 目录内。`selftest://`、空路径、任意文本路径不得满足 release gate。

Release decision taxonomy 必须改为：

| Decision | Meaning | Closeable |
|---|---|---|
| `release_pass` | 2x release target and quality gate pass | yes |
| `blocked_partial` | engineering partial target may pass, but release target missed | no |
| `fail` | quality/cost/harness gates fail | no |

`blocked_partial` 允许作为工程进展记录，但禁止作为 v0.0.5 收口依据。

## 7. 代码落点

| 模块 | 文件 | 设计动作 |
|---|---|---|
| Provider request budget hook | `third_party/codex-cli/codex-rs/core/src/client.rs` HTTP/WebSocket provider dispatch 前后 | 执行中阻断/放行 request，记录 request id、phase、task/node context、budget before/after、payload hash；不得把 best-effort rollout trace 当作控制 hook |
| Provider request trace evidence | `third_party/codex-cli/codex-rs/rollout-trace/src/inference.rs`、`raw_event.rs` | 仅作为 exact payload / audit evidence 输入；不能负责预算阻断 |
| Runtime budget state | `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` | 增加 budget contract、counter、state machine、gate recovery |
| TaskSpace handler | `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs` | 对 legacy actions 增加 active profile budget 和 `state_commit_required` recovery |
| Tool schema/profile | `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`、`tool_config.rs`、`tool_registry_plan.rs` | 暴露 compact active profile，收窄 legacy action 诱导面 |
| Spawn tool gating | `third_party/codex-cli/codex-rs/tools/src/agent_tool.rs`、runtime spawn checks | profile 下隐藏/限制 spawn，或 runtime 统一阻断 |
| Provider-visible context composition | session/history/request input assembly path discovered in Phase 0A, plus `action_map/runtime.rs` projection builder | active profile 下省略 raw TaskSpace history、stale node history、rejected subagent body、large raw output；projection proof 只能证明，不是替换动作本身 |
| Metrics extraction | `scripts/taskspace-benchmark/lib/metrics-extractor.ps1` | 解析新增 artifacts |
| Cost instrumentation | `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1` | 写 request phase、state commit displacement、active replacement |
| Release decision | `scripts/taskspace-benchmark/write-release-decision.ps1` | 加新 gate 和 blockers |
| E3 start gate | `scripts/taskspace-benchmark/lib/e3-start-gate.ps1` | formal E3 必须依赖 v0.0.5 non-agent gates、code-complete marker 和用户批准 marker；缺任何一项时禁止 `full_e3` |
| Harness tests | `scripts/taskspace-benchmark/test-harness.ps1`、`test-release-decision.ps1` | 增加 synthetic pass/fail fixture |

### 7.1 修正后的关键落点覆盖

以下落点覆盖并收紧上表中仍然宽泛或待发现的条目：

| 模块 | 文件 | 设计动作 |
|---|---|---|
| Provider lifecycle canonical producer | `third_party/codex-cli/codex-rs/core/src/client.rs`、`third_party/codex-cli/codex-rs/core/src/session/turn.rs` | client/session 负责 start/terminal lifecycle、token、latency、payload hash/scan request id；ActionMap 不再事后推断 provider request attribution |
| Provider request context producer | `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`、session request construction path | 在 provider dispatch 前提供 `TaskSpaceProviderRequestContextV1`，包括 task/map/node/request_phase/context_selection_reason |
| Provider-visible context composition | `third_party/codex-cli/codex-rs/core/src/session/turn.rs` history/request input assembly boundary, plus `action_map/runtime.rs` projection builder | active profile 下在 `for_prompt` 前替换 provider-visible history，省略 raw TaskSpace history、stale node history、rejected subagent body、large raw output；projection proof 只能证明，不是替换动作本身 |
| Release formal identity gate | `scripts/taskspace-benchmark/write-release-decision.ps1` | 验证 sample_set_id、sample_names、repeats_per_sample、runner_entrypoint、runner_profile_hash、start gate decision、approval sample set 与 artifacts 一致 |
| E3 start sample-set gate | `scripts/taskspace-benchmark/lib/e3-start-gate.ps1`、`scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1` | 接收 expected sample set id，校验 user approval 的 approved_sample_set_id，并在 sample scheduling 前阻断不匹配运行 |
| Budget quality impact | runtime budget producer、metrics extractor、release decision | 对 hard stop / thin / no-spawn / validation skip / final abort 产出 `BudgetQualityImpactV1`，release report 按样本汇总 solve loss 风险 |

## 8. 阶段计划

### Phase 0A: Provider Request Hook Discovery And Implementation

#### Objective

先把预算控制锚到实际 provider/model request 生命周期。没有 provider request hook，就不能开始 runtime budget implementation。

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| provider request path 可定位 | code search over client/session/provider dispatch | exact file/function list | core engineering |
| TaskSpace active context 可在 request path 查询 | static code audit | context propagation note | core engineering |

#### Implementation Tasks

- 定位 provider request dispatch 前后 hook。
- 在 `client.rs` 的 HTTP `client.stream_request(...)` 和 WebSocket `websocket_connection.stream_request(...)` 前加入可阻断 budget check；响应/失败后更新 budget state。
- 禁止把 `InferenceTraceAttempt::record_started` 作为阻断 hook：它是 best-effort trace writer，失败时不会阻止 provider request。
- 增加 `TaskSpaceProviderRequestEventV1`。
- request 开始前记录 request id、route mode、task/map/node context、budget state before。
- request 完成后记录 token、latency、status、budget state after。
- 捕获或重建 exact provider-visible payload artifact；如出于隐私不能保留可搜索 artifact，则必须先生成 exact pre-redaction scan event。
- 无法获得 task/node context 时标记 missing，不允许静默 fallback。

#### Deliverables

- provider request event implementation
- `provider-request-events.jsonl`
- `budget-events.jsonl`
- exact request payload artifact support or exact pre-redaction scan event support

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| provider hook fires | non-agent fixture/mock provider request | every request emits start/end or terminal failed event |
| provider hook blocks | mock provider request with exceeded budget | non-recovery provider request is not dispatched |
| context propagation | fixture with active TaskSpace task/node | event has task_id/map_id/node_id |
| payload proof | fixture request | searchable payload artifact present, or exact pre-redaction scan event present and tied to payload sha256 |
| missing context | fixture without TaskSpace | explicit null/missing reason, not zero |

#### Exit Criteria

```text
provider_request_hook_coverage >= 99%
provider_request_context_missing_reason present for every missing context
payload hash available for active TaskSpace requests
payload artifact or exact pre-redaction scan event available for active replacement release proof
provider request over hard budget is blocked before network dispatch
```

#### Review Plan

- Review exact hook location before implementing runtime budget.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| provider payload capture exposes sensitive data | privacy/security risk | payload artifact contains secrets | hash-only by default plus in-process exact payload scan before redaction | hash-only evidence is audit-only and cannot satisfy release_pass without exact scan event |

#### Gate To Next Phase

Provider request hook implemented and covered by non-agent tests. A field audit alone is not sufficient.

### Phase 0B: Artifact And Trace Coverage

#### Objective

确认 runtime event、provider request event、rollout trace、model request usage 之间能否建立足够稳定的关联。

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| 当前 P0 诊断 artifact 可读 | 读取 `request-summary.json` / pair metrics | artifact index | core engineering |
| provider request event fixture 存在 | Phase 0A output | `provider-request-events.jsonl` | core engineering |

#### Implementation Tasks

- 写一个非 agent parser fixture，输入 provider request event、runtime event、rollout trace。
- 输出 request 与 runtime event 的 join coverage。
- 标记缺失字段，不猜测。
- 如果 coverage 未达标，阻止 Phase 1。

#### Deliverables

- `request-phase-field-audit.md`
- parser fixture

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| 缺字段处理 | synthetic JSONL | missing 不被当成 0 |
| phase enum | unit fixture | unknown 有 reason |

#### Exit Criteria

- request phase attribution coverage 达标。
- 如果不能达标，必须回到 Phase 0A 补字段；不能只列出字段后继续。

#### Review Plan

- 对照 `terminal-bench_E3-P0_3_2` 结果检查 phase 覆盖率。

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| 无法精确 join request/event | phase attribution 粗糙 | unknown > 30% | 增加 trace id | 暂用时间窗但标记 low confidence |

#### Gate To Next Phase

```text
request_phase_attribution_coverage >= 95%
unknown_request_phase_ratio <= 5%
```

### Phase 1: Active Budget Runtime Contract

#### Objective

让 TaskSpace active profile 在运行中知道预算，并能阻断继续扩张。

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| Phase 0A provider hook 已实现 | test output | provider request fixture pass | core engineering |
| Phase 0B trace coverage 达标 | parser fixture | coverage report | core engineering |
| existing runtime budget tests 可跑 | `cargo test -p codex-core budget` | test output | core engineering |

#### Design Approach

- 在 runtime 内维护 budget counters。
- Gate 不直接猜测业务语义，只限制动作类别和扩张行为。
- 超预算 recovery 必须给模型可执行下一步。

#### Implementation Tasks

- 增加 `TaskSpaceActiveBudgetV1` 类型。
- 增加 `TaskSpaceBudgetCounters`。
- 在 provider request hook、ordinary tool、spawn、create node、legacy action、projection build、state_commit 入口更新 counters。
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
| provider request budget | mock provider request fixture | grace 后不再发起非 recovery provider request |
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

### Phase 2: Exact Active Context Replacement Proof

#### Objective

证明 active projection 不是附加摘要，而是替代了高成本 TaskSpace 历史。

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| active projection builder 可用 | Rust projection tests | test output | core engineering |
| protected item list 明确 | 对照 contract | checklist | core engineering |

#### Design Approach

- active replacement 的核心动作是改变 provider-visible context composition；exact provider payload proof 只能证明替换是否发生。
- active profile 下发现旧历史叠加时，release gate 失败。
- protected items missing 时，runtime gate 失败。

#### Implementation Tasks

- 定位并修改 request input/history assembly，使 `taskspace-v005-active` 的 provider request 只携带 active projection、当前必要用户约束、当前节点必要证据、失败验证证据和最终回答所需 protected items。
- 在 active profile 中省略 raw TaskSpace control history、completed stale node history、rejected subagent body、large raw output replay。
- 增加 active provider payload capture/reconstruction helper。
- 输出 `active-context-replacement-report.json`。
- 增加 protected item enumerator。
- 增加 violations：
  - `legacy_taskspace_history_present`
  - `raw_output_replay_present`
  - `projection_over_budget`
  - `protected_item_missing`

#### Deliverables

- provider-visible context composition implementation
- reconstruction helper
- report artifact
- release decision gate

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| active replacement | synthetic prompt fixture |旧 TaskSpace history 不出现 |
| actual context composition | synthetic history fixture | request input assembly omits raw TaskSpace history before scan |
| fake projection artifact | fixture with projection event but legacy payload | release gate fails |
| hash-only fallback | fixture with hash/size but no payload artifact or exact scan | release gate fails |
| exact scan provenance | fixture with scan event | replacement fields point to same request_id and payload hash |
| protected item | fixture | user requirement / failed validator present |
| raw output | large output fixture | >50KB raw absent |

#### Exit Criteria

`active_context_replacement_confirmed=true` 能由 exact provider payload fixture 证明。

#### Review Plan

- 审查是否把必要 validator evidence 错误 elide。

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| projection 丢上下文 | 正确率下降 | protected miss | hard fail active profile | 回退 shadow profile |

#### Gate To Next Phase

active replacement fixture PASS，release decision 能拒绝 shadow-only、replacement-failed、projection-artifact-only run。

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
| retry pressure | fixture with rejected commit | follow-up request count is measured and gateable |
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
- 把 current `PASS/PARTIAL/FAIL` decision 改为 `release_pass/blocked_partial/fail`，并输出 `closeable` 字段；`blocked_partial.closeable=false`，markdown 必须明确不可用于 v0.0.5 收口。
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1` 接入 v0.0.5 non-agent gates、code-complete marker 和用户批准 marker。缺任何一项时 `full_e3_allowed=false`，`next_allowed_command_category` 只能是 `fixture_tests`、`targeted_diagnostic` 或 `blocked`。
- `run-taskspace-e3-suite.ps1` 必须暴露并转发 `V005NonAgentGatesPath`、`V005CodeCompleteMarkerPath`、`V005UserApprovalMarkerPath`；正式 scoring/full E3 模式下，如果 `gate_decision.full_e3_allowed=false`，即使 start gate `exit_code=0` 也必须在 sample scheduling 前中止。
- v0.0.5 marker 不允许只是存在即通过。三类 marker 都必须是结构化 JSON，并绑定当前 `task_list_hash`、`profile_hash`、`source_version`、生成时间和 schema version。
- `v005_non_agent_gates.json` 必须汇总 Phase 0A-5 的非 agent 证据，至少包含 provider hook、runtime budget response、active replacement exact scan、state_commit displacement、spawn/node budget、request phase attribution、release decision fixture、start gate fixture 的 pass/fail 和证据路径。
- `v005_code_complete.json` 必须列出代码完成范围、git commit、未完成 P0 项为 empty、非 agent test command/output path；空文件或任意文本必须 fail。
- `v005_user_approval.json` 必须记录 explicit approval source、approval timestamp、approved command category、approved sample set id、task_list_hash、profile_hash 和 source_version；不同样本、profile 或源码版本不得复用。
- `test-release-decision.ps1` 增加 pass/fail fixture。
- 增加 E3 start gate fixture：缺 v0.0.5 gate 时拒绝 formal E3；只有 non-agent gates PASS、code-complete marker 存在、用户批准 marker 存在时才允许 `full_e3`。
- 增加 spoofed marker fixture：任意文本 marker、过期 marker、hash/profile/source 不匹配 marker 必须 fail。
- 增加 suite runner fixture：complete calibration 但 `full_e3_allowed=false` 时不得 schedule samples。
- 所有正式结果强制显示 dataset/subset/sample/repeats/runner/evidence level。

#### Deliverables

- updated release decision
- updated E3 start gate
- updated harness tests
- P0 sample eligibility report

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| invalid harness fixture | script test | release fail reason 精确 |
| missing active gate | script test | clean release blocked |
| blocked partial | script test | 3x engineering partial produces `blocked_partial`, closeable=false |
| start gate formal E3 block | script test | missing v0.0.5 non-agent gates/code/user marker blocks `full_e3` |
| start gate decision honored | script test | `full_e3_allowed=false` aborts before sample scheduling even when gate json exists |
| marker schema | script test | arbitrary text / stale / mismatched hash marker fails |
| hash-only active replacement | script test | hash-only payload proof cannot produce `release_pass` |
| exact payload scan | script test | scan event can satisfy replacement proof only when request_id/hash match |
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

#### Phase 6 修正规则

Phase 6 必须拆成三个互不替代的证据轨道：

| 轨道 | 样本集 | reported_evidence_level | 用途 | 是否允许 release_pass |
|---|---|---|---|---|
| targeted diagnostic | `terminal-bench_E3-P0_1_1` 或 `terminal-bench_E3-P0_3_1/_3_2` 变体 | `diagnostic-only` 或 `E3-candidate` | 低成本检查 request/token/spawn/budget 是否明显改善 | no |
| formal P0 release proof | `terminal-bench_E3-P0_3_5` | `E3` | v0.0.5 当前 P0 成本/正确率收口判断 | yes, if all gates pass |
| v0.0.4 clean comparison | `terminal-bench_E3-v004-clean_3_5` | `E3` only after full audit | 与 v0.0.4 clean 15-run 做同口径回归对比 | no, cannot replace P0 proof |

执行顺序：

1. 先跑 diagnostic-only targeted diagnostic，默认首选 `processing-pipeline` 或当前最能暴露 request explosion 的 P0 样本。
2. diagnostic 必须输出 `reported_evidence_level=diagnostic-only`、`sample_set_id`、`repeats_per_sample`、`runner_entrypoint` 和 `not_release_proof=true`。
3. 只有 diagnostic 的 request/token/spawn/budget gate 达标，且 non-agent gates、code-complete marker、user approval marker 均有效，才允许跑 `terminal-bench_E3-P0_3_5`。
4. `terminal-bench_E3-v004-clean_3_5` 只能在 P0 release proof 之外补跑，用于说明相对 v0.0.4 clean 口径是否退化；不得替代 P0 结论。
5. release report 必须分开列出 diagnostic、formal P0、v004 clean comparison，禁止混表或用内部 fixture success 补足正式 E3 success。

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
  direct input+output <= 2x for release_pass
  walltime <= 2x for release_pass
  model_request_count <= 2x for release_pass
  direct input+output <= 3x only for blocked_partial
  walltime <= 3x only for blocked_partial
  model_request_count <= 2.5x only for blocked_partial
  no invalid_harness sample
```

## 9. 测试策略

必须先跑非 agent 测试：

```powershell
cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-core state_commit -- --nocapture
cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-core projection -- --nocapture
cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-core budget -- --nocapture
cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-core spawn_agent -- --nocapture
cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-core provider_request_budget -- --nocapture
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

1. provider request hook 的精确文件/函数位置在哪里？Phase 0A 必须回答后才能实现预算。
2. active budget 的默认阈值是否按 route mode 固定，还是 benchmark scenario 可覆盖？
3. 已决策：budget hard stop 可以返回用户可见 `blocked_by_budget`，也可以进入 `final_synthesis` 输出 partial failure；但只要 validator 未通过、关键证据缺失或 `score_eligible=false`，该样本不得计为 solved，也不得进入 `release_pass` 的 clean 成功统计。
4. state_commit adoption gate 对不同任务复杂度是否需要不同阈值？
5. `terminal-bench_E3-P0_1_1` targeted diagnostic 首选样本应是 `processing-pipeline` 还是 `recover-accuracy-log`？

## 12.1 Current Decisions For Accepted Review Blockers

1. Provider request hook location is no longer open for v0.0.5 implementation: the authoritative hook is `third_party/codex-cli/codex-rs/core/src/client.rs` / `ModelClientSession` before provider dispatch and across stream lifecycle. `ActionMap` may provide request context and budget policy, but release evidence must not infer provider request phase/node from a later runtime snapshot.
2. Active budget thresholds must be two-layered: Standard-baseline ratio gates decide `release_pass` / `blocked_partial`, while route fixed thresholds are only runtime safety caps. A run cannot close v0.0.5 merely because it stayed under an absolute route cap.
3. Release decision must derive formal P0 identity from task-list content, verify suite receipt hash chain, and require `suite-runner-attestation.json` produced by the suite runner path. A self-consistent copied JSON tree is not enough for `release_pass`.

## 13. Plan Quality Checklist

- [x] 问题定义与目标分离。
- [x] 目标可测量。
- [x] 约束、假设、风险和开放问题分离。
- [x] 阶段计划有入口、任务、交付物、验证、退出和回退。
- [x] 明确禁止代码完成前真实 E3。
- [x] 包含 runtime、handler、tool schema、benchmark scripts 的代码落点。
- [x] 包含 release gate 和 rollback/fallback。

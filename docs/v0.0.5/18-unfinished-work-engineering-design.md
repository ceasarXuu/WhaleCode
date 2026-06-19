# v0.0.5 未完成项工程设计

- Created: 2026-06-19
- Updated: 2026-06-19
- Version: v0.0.5-continuation-design
- Status: Draft - not approved for implementation until adversarial closure review passes
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
provider_request_hook_coverage >= 99%
request_phase_attribution_coverage >= 95%
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

### 6.5 Active Context Replacement Proof

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
| Provider request hook | provider client/session request path, exact file to be confirmed in Phase 0A | 记录 request id、phase、task/node context、budget before/after、payload hash |
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
- 增加 `TaskSpaceProviderRequestEventV1`。
- request 开始前记录 request id、route mode、task/map/node context、budget state before。
- request 完成后记录 token、latency、status、budget state after。
- 捕获或重建 exact provider-visible payload artifact；如出于隐私不能保留可搜索 artifact，则必须先生成 exact pre-redaction scan event。
- 无法获得 task/node context 时标记 missing，不允许静默 fallback。

#### Deliverables

- provider request event implementation
- `provider-request-events.jsonl`
- exact request payload artifact support or exact pre-redaction scan event support

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| provider hook fires | non-agent fixture/mock provider request | every request emits start/end or terminal failed event |
| context propagation | fixture with active TaskSpace task/node | event has task_id/map_id/node_id |
| payload proof | fixture request | searchable payload artifact present, or exact pre-redaction scan event present and tied to payload sha256 |
| missing context | fixture without TaskSpace | explicit null/missing reason, not zero |

#### Exit Criteria

```text
provider_request_hook_coverage >= 99%
provider_request_context_missing_reason present for every missing context
payload hash available for active TaskSpace requests
payload artifact or exact pre-redaction scan event available for active replacement release proof
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

- 不直接改变业务语义；先增加 exact provider payload proof。
- active profile 下发现旧历史叠加时，release gate 失败。
- protected items missing 时，runtime gate 失败。

#### Implementation Tasks

- 增加 active provider payload capture/reconstruction helper。
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
- 把 current `PARTIAL` decision 改为 `blocked_partial`，并确保 exit/report 文案明确不可收口。
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
| blocked partial | script test | 3x engineering partial produces `blocked_partial`, closeable=false |
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
  direct input+output <= 3x only for blocked_partial
  walltime <= 3x only for blocked_partial
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

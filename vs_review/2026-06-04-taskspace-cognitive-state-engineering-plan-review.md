# Subagent VS Review: TaskSpace Cognitive State Engineering Plan

- Created: 2026-06-04T03:24:27+08:00
- Updated: 2026-06-04T04:08:05+08:00
- Task: 制定 TaskSpace 问题状态与模型管理工程化落地方案，并执行对抗性审查。
- Report path: `vs_review/2026-06-04-taskspace-cognitive-state-engineering-plan-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: closed

## Round 1: Engineering Plan Adversarial Review

### Review Input

#### Objective

Review whether the proposed TaskSpace cognitive-state engineering plan is concrete, grounded in the existing Whale/Codex codebase, implementable in phases, and sufficient to address the negative E3 benchmark findings without over-designing a parallel runtime.

#### Review Target

Architecture and engineering plan documentation for TaskSpace cognitive-state management.

#### Target Locations

- `docs/plans/2026-06-04-taskspace-cognitive-state-runtime-after-e3.md`
- `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md`
- `docs/testing/2026-06-03-taskspace-full-benchmark-run.md`
- `third_party/codex-cli/codex-rs/core/src/action_map/map.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs`
- `third_party/codex-cli/codex-rs/app-server-protocol/src/protocol/common.rs`
- `scripts/run-action-map-regression.ps1`
- `scripts/action-map-graph-health-lib.ps1`

#### Change Introduction

The plan reframes TaskSpace from a planner/task-map mechanism into a cognitive-state management runtime. It proposes phased implementation of direct trace, light kernel, output-contract/data-provenance/failed-hypothesis sentinels, expanded task/node/result schemas, explicit result validity, promotion from direct execution to TaskSpace, viewer/audit upgrades, and E2/E3 benchmark metric updates. It explicitly aims to reuse existing ActionMap/TaskSpace code paths instead of creating a parallel runtime.

#### Risk Focus

- Whether the plan still hides a second runtime behind new terminology.
- Whether the proposed schema and phases are too broad for a first implementation.
- Whether runtime can realistically detect output contract, data provenance, and failed hypothesis risks without semantic overreach.
- Whether compatibility with existing snapshots, rollout replay, tool schema, and app-server protocol is sufficiently addressed.
- Whether tests can prove the mechanism rather than reintroduce self-deceptive checks.
- Whether observability is specific enough to diagnose E3-style failures after the fact.
- Whether the plan preserves the user constraint that runtime must not perform semantic task routing or quality scoring.

#### Verification Status

- Current task is documentation and engineering plan only; no code implementation has been made in this round.
- Local read-only code inspection confirmed existing core paths:
  - `map.rs` contains current `TaskState`, `ActionMapInstance`, `MapNode`, `NodeResult`.
  - `runtime.rs` owns gate/lease/result/barrier/snapshot behavior.
  - `taskspace_tool.rs` and `taskspace_control.rs` currently expose summary-oriented control fields.
  - app-server protocol already exports `ActionMapSnapshot*`.
  - existing PowerShell E2/E3 scripts provide graph-health and benchmark foundations.
- No automated code tests were run because this round only changes docs and review artifacts.

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Focus on concrete blocking flaws, over-design, missing engineering steps, missing tests, and observability gaps.
- Output must include summary, blocking findings, non-blocking risks, required fixes, missing tests, missing logs/observability, and evidence.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| architecture-adversary | The plan changes core TaskSpace abstractions, runtime responsibilities, data model, and protocol shape. | architecture boundaries, migration, avoiding parallel runtime, over-design |
| test-validity-adversary | The plan exists because E2/E3 previously proved mechanism but not product utility; validation quality is central. | benchmark validity, non-self-deceptive tests, regression coverage |
| observability-adversary | The new direction depends on diagnosing facts, assumptions, contracts, evidence, and promotion after failures. | logs, snapshots, viewer, audit artifacts, failure diagnosis |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| architecture-adversary | `multi_agent_v1.spawn_agent` (`explorer`) | `019e8ef2-7d20-7821-b595-649189edad34` / Singer | spawn_agent tool result | no | Round 1 Review Input | main-agent history, hidden reasoning, drafts, conclusions, full diff persuasion | yes |
| test-validity-adversary | `multi_agent_v1.spawn_agent` (`explorer`) | `019e8ef2-cc09-7830-8eb2-3eeee56823d9` / Hubble | spawn_agent tool result | no | Round 1 Review Input | main-agent history, hidden reasoning, drafts, conclusions, full diff persuasion | yes |
| observability-adversary | `multi_agent_v1.spawn_agent` (`explorer`) | `019e8ef3-18d1-7532-8df2-81c5e52e295c` / James | spawn_agent tool result | no | Round 1 Review Input | main-agent history, hidden reasoning, drafts, conclusions, full diff persuasion | yes |

### Reviewer Outputs

#### architecture-adversary

##### Summary

方向总体正确，围绕当前 ActionMap/TaskSpace 渐进改造，但首轮实施边界、sentinel 输入来源、result validity 依赖模型、snapshot/generated schema 兼容仍不够可实施。当前不能直接进入实现。

##### Blocking Findings

- Hidden second runtime risk：`direct_trace/taskspace_trace` 加 `LightKernelState` 可能和 `TaskState.cognitive_state` 形成两套状态源。必须明确 direct trace 只是 append-only observation log，promotion 后唯一权威状态进入 `TaskState.cognitive_state`。
- Sentinel 触发条件会迫使 runtime 做语义/启发式判断。若不新增结构化 tool trace event，runtime 只能解析 shell 文本或模型 preview。
- `Questioned/Invalid` result guard 现在无法按方案执行。当前 graph 只有 node dependency，没有 result dependency；要么新增 result/evidence refs，要么把该 enforcement 降级为 audit-only。
- Snapshot/protocol 兼容方案不足，且当前 generated schema 可能已有漂移。新增 cognitive/result 字段前，必须先修复协议生成链和 CI 校验。
- 首轮 schema/phase 过大，且 prompt/context 更新顺序太晚。没有先定义一个最小闭环会导致字段机械填充、忽略或静默丢失。

##### Non-blocking Risks

- 新状态不应继续堆进超大 `runtime.rs`，应拆 helper module，但这不是第二 runtime。
- `contracts.rs` 是实际 action-class gate，应前移为核心落点。
- 首轮验收不要宣称 TaskSpace 产品收益，只证明 output contract、provenance、validity 闭环能阻止同形失败。
- `GeneratedForTestOnly` 规则容易误伤合法 fixture/self-test，应按 final fact dependency 拦截。

##### Required Fixes

- 写清 direct trace lifecycle：append-only、非权威、可 promotion、promotion 后 materialize 到 `TaskState.cognitive_state`。
- 定义 sentinel 输入契约：由 tool/session 层产生结构化 trace event，runtime 只看显式 event tag。
- 把 MVP 缩到一个闭环：output contract + provenance + result claims/evidence/validity。
- 新增 result dependency/evidence ref 模型，或把 validity dependency enforcement 改为 audit-only。
- 先修复 protocol generated schema 漂移，再增加新 snapshot 字段。
- schema、handler、developer context、测试必须同一薄切片落地。

##### Missing Tests

- Old snapshot restore defaults.
- Generated schema CI/freshness.
- Sentinel source tests proving no string parsing.
- Result dependency/evidence-ref tests.
- Promotion/collapse roundtrip.
- Viewer/browser smoke.

##### Missing Logs / Observability

- `taskspace_trace_event_recorded`
- `sentinel_warning_raised`
- `sentinel_barrier_raised/cleared`
- `taskspace_promoted/collapsed`
- `fact_source_recorded`
- `output_contract_recorded`
- `result_validity_changed`
- `cognitive_state_updated`

##### Evidence

- `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md`
- `docs/testing/2026-06-03-taskspace-full-benchmark-run.md`
- `third_party/codex-cli/codex-rs/core/src/action_map/map.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs`
- `third_party/codex-cli/codex-rs/app-server-protocol/src/protocol/common.rs`

#### test-validity-adversary

##### Summary

方案方向正确，但测试设计还不能证明 TaskSpace 问题状态管理真实有效。现有测试能证明自然 prompt、真实执行、hidden oracle 隔离、map/node/edge/subagent 归属和成本差异，但还不能证明 output contract、data provenance、failed hypothesis sentinel、promotion、claims/evidence/validity 真的改变行为并阻断 E3 失败形态。

##### Blocking Findings

- 测试矩阵仍主要证明结构存在和业务测试通过，不证明认知机制闭环。
- Sentinel 验证路径不足。没有 negative test 证明“没有契约不能写最终输出”“GeneratedForTestOnly 不能进 facts”“失败假设循环会被阻断”。
- Promotion 是核心设计但没有可判定测试，E2/E3 report 没有 promotion_trigger/promotion_latency 的实际抽取和 fail gate。
- claims/evidence/validity 目前只被设计为字段和规则，缺少“下游依赖被阻断”的验证。

##### Non-blocking Risks

- prompt contamination guard 目前主要检查用户 prompt 静态文本，还应检查模型最终回答和用户可见内容。
- 外部 E3 适配器默认 node/spawn 上限过宽，对发现过度生长不敏感。
- 当前内置三场景缺少专门诱发 provenance、contract、hypothesis 失败的场景。

##### Required Fixes

- readiness 分成机制可用、行为健康、产品收益三层。
- 扩展 metrics extractor 和 pair/matrix report，实际抽取并 fail gate cognitive MVP 指标。
- 为三个 sentinel 各做最小负例。
- 增加 promotion replay，验证 trace refs 被继承到初始状态。

##### Missing Tests

- Negative tests：缺 claims/evidence 的 `Accepted`、`Questioned` result 被 implementation 唯一依赖、`GeneratedForTestOnly` 写入 facts、unknown provenance 写入 final fact。
- Replay tests：复放 `jsonl-aggregator`、`heterogeneous-dates`、`hello-world`。
- Compatibility tests：老 snapshot/rollout 缺新字段时 restore 默认值。
- Prompt contamination tests：检查用户可见输出不出现内部概念。

##### Missing Logs / Observability

- Sentinel raised/cleared events。
- Output contract trace。
- Provenance transitions。
- Result validity transitions。
- Promotion/collapse events。

##### Evidence

- `docs/testing/2026-06-03-taskspace-full-benchmark-run.md`
- `docs/plans/2026-06-04-taskspace-cognitive-state-runtime-after-e3.md`
- `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md`
- `scripts/taskspace-benchmark/lib/matrix-report.ps1`
- `scripts/taskspace-benchmark/lib/metrics-extractor.ps1`
- `scripts/taskspace-benchmark/lib/pair-report.ps1`

#### observability-adversary

##### Summary

方案方向正确，但还不足以让工程师在 E3-style 失败后稳定诊断 facts、assumptions、output contracts、provenance、result validity、promotion 为什么出错。主要缺口是可机读 schema、事件生命周期、viewer/audit 断言和 artifact join key。

##### Blocking Findings

- Cognitive state 仍可能停留在计划/内部状态，尚未形成 snapshot/control 闭环。
- Result validity 没有可审计 transition。当前事件和 snapshot 仍以 result body 为核心。
- Sentinel / promotion / barrier 生命周期不可追责，缺 sentinel id、trigger refs、promotion id、clear action、cleared_by 或 inherited trace refs。
- Viewer 自动刷新保留状态的验证不足，E2E 还不是 Playwright 级真实交互断言。
- Benchmark artifact 能说明谁赢/谁输，但不能解释 TaskSpace 内部为什么接受、质疑或污染了结果。

##### Non-blocking Risks

- Rust/TS schema 可能已经漂移。
- 自然语言字段短期可行，但削弱 audit 指标稳定计算。
- E3 流程仍有人工补跑和 PowerShell 参数拆词问题。

##### Required Fixes

- 定义并生成 versioned snapshot schema。
- 扩展 `taskspace_control` 的记录类动作。
- 新增 runtime events。
- audit artifact 必须按 `result_id / claim_id / evidence_ref / artifact_hash / validator_ref / promotion_id` 串起 why-chain。
- viewer 增加 cognitive side panel，并能从 graph/node/result 跳到相关 facts/contracts/validity/provenance。

##### Missing Tests

- Snapshot Rust/JSON/TS schema round-trip 和 generated schema freshness test。
- Accepted 缺 claims/evidence、GeneratedForTestOnly 进入 facts、questioned result 被下游唯一依赖的拒绝测试。
- 三个 sentinel fixture。
- Direct trace promotion 继承测试。
- Playwright 级 viewer refresh state test。
- E3 audit fixture：必须能回答某个 result 为什么 accepted/questioned/invalid。

##### Missing Logs / Observability

- `trace_id`
- `sentinel_id/type/severity/status`
- `trigger_event_ids`
- `fact_source_id/provenance/confidence`
- `output_contract_id/path/format/encoding/schema/validator_refs`
- `result_validity_prev/new/reason/reviewer`
- `promotion_id/trigger/latency/inherited_trace_refs`
- `barrier_id/clear_action/cleared_by`
- `audit_schema_version`

##### Evidence

- `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md`
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs`
- `third_party/codex-cli/codex-rs/app-server/src/codex_message_processor.rs`
- `scripts/export-action-map-observability.ps1`
- `scripts/run-tui-taskspace-viewer-e2e.ps1`
- `target/benchfull-20260603-182527/.../jsonl-aggregator/.../pair-005`

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| architecture-adversary | direct trace / LightKernel may become a second state source | blocking | accept | This conflicts with the single-runtime goal. | Added “single authoritative state” and changed trace to append-only non-authoritative log in the engineering plan. | Closure review Round 2 |
| architecture-adversary | sentinel trigger conditions may force semantic string parsing | blocking | accept | Runtime currently only has structured action class and result body; string parsing would be hidden semantic runtime. | Added sentinel input contract: runtime consumes structured trace event tags only and does not parse shell strings/preview. | Closure review Round 2 |
| architecture-adversary | questioned/invalid dependency cannot be enforced without result refs | blocking | accept | Current graph is node dependency only. | Added `EvidenceRef`; MVP hard gate applies to state updates, while implementation sole-dependency enforcement is audit-only until result dependency model exists. | Closure review Round 2 |
| architecture-adversary | protocol/generated schema drift must be fixed first | blocking | accept | New fields would worsen existing drift risk. | Moved schema freshness to Phase 0/0.5 and made it a blocker before new snapshot fields. | Closure review Round 2 |
| architecture-adversary | first schema/phase too large | blocking | accept | A large optional schema would invite mechanical filling. | Shrunk MVP to output contract + provenance + result evidence/validity. | Closure review Round 2 |
| test-validity-adversary | tests do not prove cognitive mechanism closure | blocking | accept | Existing metrics focus on map/node/cost. | Added MVP hard gates and three-layer readiness model. | Closure review Round 2 |
| test-validity-adversary | sentinel verification lacks negative tests | blocking | accept | Needed to prevent same E3 failures. | Added negative tests and replay fixtures for output/provenance/result evidence. | Closure review Round 2 |
| test-validity-adversary | promotion has no判定测试 | blocking | accept with scope deferral | Promotion is not an MVP capability. | Final closure moved promotion/collapse/barrier to v1.1, added `promotion_not_in_mvp=true`, and kept promotion metrics report-only. | Closure review Round 4 |
| test-validity-adversary | claims/evidence/validity lacks dependency validation | blocking | accept | Same as architecture result dependency issue. | Added `EvidenceRef`, MVP state-update gates, and final artifact dependency hard gate for `Questioned` / `Invalid` result usage. | Closure review Round 4 |
| observability-adversary | cognitive state lacks snapshot/control closure | blocking | accept | Viewer/audit need machine-readable state. | Added versioned snapshot schema, MVP control actions, and cognitive side panel requirements. | Closure review Round 2 |
| observability-adversary | result validity lacks auditable transition | blocking | accept | Events must show why accepted/questioned/invalid. | Added `result_validity_changed` and required transition fields. | Closure review Round 2 |
| observability-adversary | sentinel/promotion/barrier lifecycle not traceable | blocking | accept | Failure analysis needs ids and clear actions. | Added required lifecycle payload schemas; promotion/barrier lifecycle events are v1.1, not MVP. | Closure review Round 4 |
| observability-adversary | viewer refresh verification insufficient | blocking | accept | Existing smoke is not enough for UI state. | Added Playwright-level viewer refresh state test. | Closure review Round 2 |
| observability-adversary | benchmark artifacts lack why-chain | blocking | accept | Pair artifacts must explain why results were trusted. | Added audit join key requirements across result/claim/evidence/artifact/validator/promotion. | Closure review Round 2 |
| all reviewers | Non-blocking risks about prompt contamination, generated data false positives, schema drift, and broad thresholds | major | accept | These are valid implementation risks. | Added risk mitigations and MVP hard-gate scoping. | Closure review Round 2 |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes, after Round 4 scope and audit closure
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2
  - Round 3
  - Round 4
- Blocking re-review launch records:
  - `019e8f02-0c6e-7d50-9d0f-37822c5ba075` / Dalton
  - `019e8f02-4b28-73f2-91b8-ce811a3035d9` / Aristotle
  - `019e8f02-88cc-7cf2-8544-203215dac94b` / Russell
  - `019e8f0d-771c-70e3-8c71-67551ae64158` / Hypatia
  - `019e8f0d-ba1a-7713-8185-e5d52eee9513` / Lovelace
  - `019e8f13-df5b-7d81-8a0d-8e7d731edebc` / Wegener
- Rejected findings backed by evidence: none
- Deferred findings documented: yes, promotion/collapse/barrier explicitly v1.1 with MVP report-only metrics
- Allowed to proceed: yes

## Final Conclusion

Final adversarial review status: allowed to proceed into implementation. Architecture, test-validity, and observability blocking findings were accepted and closed through Round 4. Deferred scope is explicit: promotion/collapse/barrier are v1.1 and must not be counted as MVP success.

## Round 2: Closure Review For Accepted Blocking Findings

### Review Input

#### Objective

Verify whether the accepted blocking findings from Round 1 were adequately addressed in the engineering plan before implementation begins.

#### Review Target

Closure review of the revised TaskSpace cognitive-state engineering plan and this review report.

#### Target Locations

- `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md`
- `docs/plans/2026-06-04-taskspace-cognitive-state-runtime-after-e3.md`
- `vs_review/2026-06-04-taskspace-cognitive-state-engineering-plan-review.md`

#### Change Introduction

The engineering plan was revised after Round 1 to:

- make direct trace append-only and non-authoritative;
- declare `TaskState.cognitive_state` the single authoritative problem state;
- require structured trace events as sentinel inputs;
- prevent runtime from parsing shell strings or previews for semantic meaning;
- shrink the MVP to output contract, data provenance, and result evidence/validity;
- add `EvidenceRef` and move full implementation dependency enforcement to audit-only for MVP;
- add protocol/generated schema freshness as a Phase 0 blocker;
- add versioned snapshot schema, runtime events, audit join keys, and Playwright viewer refresh testing.

#### Risk Focus

- Did the revised plan actually remove the hidden second-runtime risk?
- Did it stop runtime from doing hidden semantic string parsing?
- Is the MVP now small enough to implement?
- Are result validity and evidence refs now enforceable at the right level?
- Does the plan now include enough schema, event, viewer, and benchmark closure to diagnose E3-style failures?
- Are there any remaining blocking gaps before implementation?

#### Verification Status

- Documentation updated only; no code implementation has been made.
- No automated code tests were run because this is still design and review work.

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Focus only on closure of accepted Round 1 blocking findings.
- Output must include summary, remaining blocking findings, non-blocking risks, required fixes, missing tests, missing logs/observability, and evidence.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| architecture-adversary-closure | Verifies architecture blocking fixes: single state source, no semantic runtime, MVP scope, protocol order. | architecture closure |
| test-validity-adversary-closure | Verifies test blocking fixes: negative tests, hard gates, readiness layers. | validation closure |
| observability-adversary-closure | Verifies observability blocking fixes: schema, events, viewer, audit join keys. | observability closure |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| architecture-adversary-closure | `multi_agent_v1.spawn_agent` (`explorer`) | `019e8f02-0c6e-7d50-9d0f-37822c5ba075` / Dalton | spawn_agent tool result | no | Round 2 Review Input | main-agent history, hidden reasoning, drafts, conclusions, full diff persuasion | yes |
| test-validity-adversary-closure | `multi_agent_v1.spawn_agent` (`explorer`) | `019e8f02-4b28-73f2-91b8-ce811a3035d9` / Aristotle | spawn_agent tool result | no | Round 2 Review Input | main-agent history, hidden reasoning, drafts, conclusions, full diff persuasion | yes |
| observability-adversary-closure | `multi_agent_v1.spawn_agent` (`explorer`) | `019e8f02-88cc-7cf2-8544-203215dac94b` / Russell | spawn_agent tool result | no | Round 2 Review Input | main-agent history, hidden reasoning, drafts, conclusions, full diff persuasion | yes |

### Reviewer Outputs

#### architecture-adversary-closure

结论：architecture closure passed。

剩余 blocking：无。

非阻塞风险：

- 基线文档仍把 `Light Kernel` 写成携带 `objective/success_criteria/observed_facts/open_questions` 的状态容器，可能误导实现者。
- `FactSource` / `OutputContract` 的 evidence refs 与 `NodeResultEvidencePackage` 的 `EvidenceRef` 需要统一 join-key 语义。

证据：

- 工程方案已声明 `TaskState.cognitive_state` 是唯一权威状态源。
- sentinel 输入契约已限制为结构化 trace event tag，不解析 shell 字符串或 preview。
- schema freshness 已被前置为 Phase 0/0.5 blocker。

#### test-validity-adversary-closure

结论：NOT CLOSED。

剩余 blocking：

- promotion finding 没有真正以可判定测试闭环，而是被移出 MVP。这个处理可以成立，但不能继续标记为 fixed，必须显式写成 MVP scope deferral，并在报告中输出 `promotion_not_in_mvp=true`。
- claims/evidence/validity 的下游依赖阻断仍不足。方案只阻断 `update_cognitive_state`，但没有定义 `Questioned` / `Invalid` result 进入 final artifact dependency 时的 hard gate。

要求修复：

- 明确 MVP hard gate 的 pass/fail 契约，区分 warning-only 与 fail gate。
- 增加 `Questioned` / `Invalid` result 被 final artifact chain 消费时的失败 fixture。
- 增加 run-level gate record 字段：`gate_name`、`expected`、`observed`、`source_artifact`、`fixture_id`、`pass/fail`。

#### observability-adversary-closure

结论：NOT CLOSED。

已闭合：

- versioned snapshot / schema freshness。
- result validity transition。
- viewer refresh state retention 测试要求。

剩余 blocking：

- sentinel / barrier / promotion lifecycle 事件没有完整 required payload schema。
- benchmark / audit why-chain 仍缺可机械 join 的 artifact row/schema，尤其是 `artifact_hash`、`promotion_id`、`result_validity_event_id` 和 final-artifact dependency edges。

要求修复：

- 为 `sentinel_warning_cleared`、`sentinel_barrier_raised`、`sentinel_barrier_cleared`、`taskspace_promoted`、`taskspace_collapsed`、`promotion_aborted` 增加 required fields。
- 定义 audit artifact schema，能从 final artifact path/hash 串到 result、claim、evidence、validator、validity transition、sentinel、promotion。

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Action Taken | Follow-up |
|---|---|---|---|---|---|
| architecture-adversary-closure | Architecture blockers closed | blocking closure | accept | 不再补架构 blocker。 | Round 3 不再重复架构审查。 |
| architecture-adversary-closure | baseline doc Light Kernel may mislead | major | accept | 将基线文档的 Light Kernel 改为 Light Checkpoint View，并明确它只由 trace 派生、不是权威状态源。 | Round 3 可顺带抽查。 |
| architecture-adversary-closure | evidence refs type mismatch | minor | accept | 将 `FactSource` / `OutputContract` 的 `evidence_refs` 统一为 `Vec<EvidenceRef>`。 | Round 3 可顺带抽查。 |
| test-validity-adversary-closure | promotion was deferred, not fixed | blocking | accept with scope correction | 新增 MVP Pass / Fail 契约，明确 `promotion_not_in_mvp=true`，promotion/collapse 是 v1.1，不作为 MVP 成功声明。 | Round 3 validation closure |
| test-validity-adversary-closure | Questioned/Invalid downstream dependency not closed | blocking | accept | 增加 final artifact dependency hard gate：`questioned_or_invalid_final_artifact_dependency`；增加对应 fixture。 | Round 3 validation closure |
| test-validity-adversary-closure | hard gate contract unclear | blocking | accept | 增加 hard fail gate、warning/report-only 指标表和 run-level gate record schema。 | Round 3 validation closure |
| observability-adversary-closure | lifecycle events lack required payload schemas | blocking | accept | 补齐 `sentinel_warning_cleared`、barrier、promotion、collapse、abort 的 required fields，并标注 v1.1 scope。 | Round 3 observability closure |
| observability-adversary-closure | audit why-chain underspecified | blocking | accept | 新增 `CognitiveAuditRecord`、`FinalArtifactDependencyEdge`、`RunGateRecord`，包括 `audit_schema_version`、`artifact_hash`、`result_validity_event_id`、`promotion_not_in_mvp`。 | Round 3 observability closure |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes, revised again after Round 2
- Blocking re-review completed: yes
- Blocking re-review passed: no
- Blocking re-review round links:
  - Round 2
- Blocking re-review launch records:
  - `019e8f02-0c6e-7d50-9d0f-37822c5ba075` / Dalton
  - `019e8f02-4b28-73f2-91b8-ce811a3035d9` / Aristotle
  - `019e8f02-88cc-7cf2-8544-203215dac94b` / Russell
- Rejected findings backed by evidence: none
- Deferred findings documented: yes, promotion/collapse explicitly marked v1.1 with `promotion_not_in_mvp=true`
- Allowed to proceed: no, Round 3 closure required

## Round 3: Closure Review After Scope And Audit Fixes

### Review Input

#### Objective

Verify whether the Round 2 test-validity and observability blockers were adequately addressed after the plan added MVP pass/fail gates, explicit promotion scope deferral, final artifact dependency hard gates, lifecycle event schemas, and audit artifact schema.

#### Review Target

Closure review of the revised TaskSpace cognitive-state engineering plan and this report.

#### Target Locations

- `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md`
- `docs/plans/2026-06-04-taskspace-cognitive-state-runtime-after-e3.md`
- `vs_review/2026-06-04-taskspace-cognitive-state-engineering-plan-review.md`

#### Change Introduction

After Round 2, the plan was revised to:

- define explicit MVP hard fail gates versus warning/report-only metrics;
- mark promotion/collapse as v1.1 and require MVP reports to include `promotion_not_in_mvp=true`;
- add final artifact dependency hard gates for `Questioned` / `Invalid` result usage;
- add run-level gate records with expected/observed/source artifact/fixture/pass fields;
- add required payload schemas for sentinel warning clear, v1.1 barrier, v1.1 promotion, collapse, and abort lifecycle events;
- add `CognitiveAuditRecord`, `FinalArtifactDependencyEdge`, and `RunGateRecord` schemas;
- update the baseline doc so Light Kernel becomes a derived `Light Checkpoint View`, not a second authoritative state source.

#### Risk Focus

- Whether promotion is honestly scoped out of MVP instead of being counted as fixed.
- Whether `Questioned` / `Invalid` result usage can fail the run when it reaches authoritative state or final artifacts.
- Whether benchmark/audit artifacts can mechanically reconstruct why a final artifact was accepted, questioned, invalid, or contaminated.
- Whether observability fields are concrete enough to implement and test.

#### Verification Status

- Documentation updated only; no code implementation has been made.
- No automated code tests were run because this remains design and review work.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| test-validity-adversary-closure-round3 | Rechecks Round 2 validation blockers after MVP gate and fixture changes. | validation closure |
| observability-adversary-closure-round3 | Rechecks Round 2 observability blockers after lifecycle/audit schema changes. | observability closure |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| test-validity-adversary-closure-round3 | `multi_agent_v1.spawn_agent` (`explorer`) | `019e8f0d-771c-70e3-8c71-67551ae64158` / Hypatia | spawn_agent tool result | no | Round 3 Review Input | main-agent history, hidden reasoning, drafts, conclusions, full diff persuasion | yes |
| observability-adversary-closure-round3 | `multi_agent_v1.spawn_agent` (`explorer`) | `019e8f0d-ba1a-7713-8185-e5d52eee9513` / Lovelace | spawn_agent tool result | no | Round 3 Review Input | main-agent history, hidden reasoning, drafts, conclusions, full diff persuasion | yes |

### Reviewer Outputs

#### test-validity-adversary-closure-round3

结论：CLOSED。

剩余 blocking：无。

确认闭合点：

- promotion 已明确降级为 MVP scope deferral，而不是伪装成 fixed。
- `Questioned` / `Invalid` 进入 cognitive state 或 final artifact chain 有 hard fail。
- MVP hard fail / warning / report-only 契约和 run-level gate record 字段已定义。
- fixture 已要求构造失败路径，而不只是检查字段存在。

非阻塞建议：

- 基线文档仍保留 P4/P8 的旧口径，建议同步 MVP 边界。
- 实现时最好参数化覆盖 `Questioned` / `Invalid` × `cognitive_state_update` / `final_artifact_dependency` 四种组合。
- Phase 4 把完整 `update_cognitive_state` 延后，但 hard gate 使用该名称；实现时应明确 MVP 路径是 `record_fact` / `cognitive_state_updated` 还是部分 `update_cognitive_state`。

#### observability-adversary-closure-round3

结论：NOT CLOSED。

已闭合：

- lifecycle required fields 已写入工程方案。
- v1.1 标注已写入工程方案。
- audit schema 和 why-chain join keys 已写入工程方案。

剩余 blocking：

- `runtime-after-e3` 仍把 promotion / collapse / maintenance barrier 写成“第一版”或当前阶段行为，和工程方案里 “promotion/collapse/barrier 延后到 v1.1、MVP 只 report-only” 的口径冲突。

非阻塞建议：

- `FinalArtifactDependencyEdge.from_kind/to_kind` 最好收敛为 enum。
- `result_validity_changed` required fields 中也显式列出 `result_id`。
- `sentinel_warning_cleared.clear_action` 固定枚举，承载风险接受和契约修正。

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Action Taken | Follow-up |
|---|---|---|---|---|---|
| test-validity-adversary-closure-round3 | Test-validity closure passed | blocking closure | accept | 无需继续修 test blocking。 | 无 |
| test-validity-adversary-closure-round3 | baseline doc P4/P8 old scope may mislead | major | accept | 更新 `runtime-after-e3` 顶部和 P4/P8/P5/设计取舍/工程入口，明确 promotion/collapse/barrier 属于 v1.1，MVP 只 report-only。 | Round 4 observability closure |
| test-validity-adversary-closure-round3 | parameterized validity dependency fixture | minor | accept | 增加 `result-validity-dependency-matrix` fixture，覆盖 `Questioned` / `Invalid` × `cognitive_state_update` / `final_artifact_dependency`。 | Round 4 可抽查 |
| observability-adversary-closure-round3 | runtime baseline scope conflicts with MVP | blocking | accept | 更新 `runtime-after-e3`，以工程方案为 MVP 口径；P4 promotion、P8 collapse、P5 maintenance barrier 均标注 v1.1/future scope。 | Round 4 observability closure |
| observability-adversary-closure-round3 | dependency edge kind is free text | minor | accept | 将 `FinalArtifactDependencyEdge.from_kind/to_kind` 改成 `AuditDependencyKind` enum。 | Round 4 可抽查 |
| observability-adversary-closure-round3 | result validity event should repeat result_id | minor | accept | 在 `result_validity_changed` required fields 中显式加入 `result_id`。 | Round 4 可抽查 |
| observability-adversary-closure-round3 | clear action should be normalized | minor | accept | 固定 `sentinel_warning_cleared.clear_action` 为 `FixApplied` / `RiskAcceptedByMainAgent` / `ContractRevised`。 | Round 4 可抽查 |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes, revised after Round 3
- Blocking re-review completed: yes
- Blocking re-review passed: no
- Blocking re-review round links:
  - Round 3
- Blocking re-review launch records:
  - `019e8f0d-771c-70e3-8c71-67551ae64158` / Hypatia
  - `019e8f0d-ba1a-7713-8185-e5d52eee9513` / Lovelace
- Rejected findings backed by evidence: none
- Deferred findings documented: yes, promotion/collapse/barrier are v1.1 with MVP report-only metrics
- Allowed to proceed: no, Round 4 observability closure required

## Round 4: Observability Closure After MVP Scope Sync

### Review Input

#### Objective

Verify whether the Round 3 observability blocker was closed after syncing MVP scope across the baseline runtime doc and engineering plan.

#### Review Target

Closure review of MVP scope consistency and observability schema refinements.

#### Target Locations

- `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md`
- `docs/plans/2026-06-04-taskspace-cognitive-state-runtime-after-e3.md`
- `vs_review/2026-06-04-taskspace-cognitive-state-engineering-plan-review.md`

#### Change Introduction

After Round 3, the docs were revised to:

- declare in `runtime-after-e3` that MVP scope is controlled by the engineering plan;
- mark promotion/collapse/maintenance hard barrier as v1.1 or future scope;
- keep `promotion_trigger`, `promotion_latency`, and `collapse_rate` report-only in MVP;
- add parameterized result-validity dependency fixture and `mvp-scope-regression`;
- add `result_id` to `result_validity_changed` required fields;
- normalize audit dependency edge kinds as `AuditDependencyKind`;
- normalize `sentinel_warning_cleared.clear_action` values.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| observability-adversary-closure-round4 | Rechecks the final Round 3 observability blocker and schema refinements. | observability closure |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| observability-adversary-closure-round4 | `multi_agent_v1.spawn_agent` (`explorer`) | `019e8f13-df5b-7d81-8a0d-8e7d731edebc` / Wegener | spawn_agent tool result | no | Round 4 Review Input | main-agent history, hidden reasoning, drafts, conclusions, full diff persuasion | yes |

### Reviewer Outputs

#### observability-adversary-closure-round4

结论：CLOSED。

剩余 blocking：无。

确认闭合点：

- `runtime-after-e3` 已明确首轮 MVP 以 engineering plan 为准。
- P4 promotion、P8 collapse、P5 maintenance barrier 不再是 MVP 成功条件。
- engineering plan 已补 `result_id`、v1.1 lifecycle schema、`AuditDependencyKind`、why-chain records、`clear_action` 允许值。

非阻塞风险：

- `runtime-after-e3` 底部“优先改造路径 / 第一阶段成功标准”仍可能把实现者推向完整 cognitive fields。已在主 agent 后续修正中收紧为 MVP 与 v1.1 分层。
- Round 4 区块原本尚未补写；本次主 agent 已补回闭环记录。

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Action Taken | Follow-up |
|---|---|---|---|---|---|
| observability-adversary-closure-round4 | Round 3 observability blocker closed | blocking closure | accept | 记录 CLOSED。 | 无 |
| observability-adversary-closure-round4 | bottom implementation path may still encourage scope creep | minor | accept | 收紧 `runtime-after-e3` 的“优先改造路径 / 第一阶段成功标准”，明确 MVP 与 v1.1 分层。 | 无 |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 4
- Blocking re-review launch records:
  - `019e8f13-df5b-7d81-8a0d-8e7d731edebc` / Wegener
- Rejected findings backed by evidence: none
- Deferred findings documented: yes, promotion/collapse/barrier are v1.1 with MVP report-only metrics
- Allowed to proceed: yes

## Round 5: Preflight Fold-In Second Check

### Review Input

#### Objective

二次检查 TaskSpace cognitive-state 工程计划在吸收 preflight 验证结论后是否自洽，是否仍有阻塞性设计或文档问题。

#### Review Target

Preflight 结论回填后的工程计划一致性审查。

#### Target Locations

- `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md`
- `vs_review/2026-06-04-taskspace-cognitive-preflight-tests-review.md`
- `docs/plans/2026-06-04-taskspace-cognitive-state-runtime-after-e3.md`

#### Risk Focus

- 新增的“最小化预实验后的计划修正”和原计划 Phase、测试矩阵、完成定义是否一致。
- contract-sketch / preflight guard 是否仍可能被误当成生产覆盖或 E3 utility 证据。
- promotion/collapse/barrier 是否被混入 MVP 完成定义或 viewer/audit hard gate。
- schema freshness、tool schema gap、snapshot restore、viewer 空态这些预实验暴露问题是否已经被写成明确 blocker。
- 是否还有 runtime 做语义判断、新造平行 runtime、仅靠 prompt 解决的残留表述。

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| preflight-fold-in-consistency-reviewer | Rechecks plan consistency after folding preflight findings into the engineering plan. | phase ownership, MVP boundary, test self-deception |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| preflight-fold-in-consistency-reviewer | `multi_agent_v1.spawn_agent` (`explorer`) | `019e91a1-12d8-7d11-92ff-d94ec8d94236` / Godel | spawn_agent tool result | no | Round 5 Review Input | main-agent history, hidden reasoning, drafts, conclusions, full diff persuasion | yes |

### Reviewer Outputs

#### preflight-fold-in-consistency-reviewer

结论：NOT CLOSED。

阻塞问题：

- Phase 0.1 / 0.5 归属不自洽：文档一处把 `taskspace_control` tool schema gap test 写到 Phase 0.5，另一处又把真实 schema gap 和 snapshot join-key 归到 Phase 0.1。建议 Phase 0.1 = preflight contract-sketch + real `taskspace_control` gap + snapshot join-key；Phase 0.5 = generated schema freshness。
- `state_delta_intent` 漏入 MVP 实现面：计划其他位置已说它是 report-only / v1.1，但 Phase 3 测试仍要求 invalid `state_delta_intent` 返回明确错误，容易诱导 MVP parser/schema/action 实现。

非阻塞风险：

- Phase 8 标题最好显式标 `v1.1 / non-MVP`，避免排期误读。
- Viewer 段应说明 MVP viewer 只展示 output contracts、fact sources、result evidence、validity、sentinel records；完整 facts/assumptions/decisions/open_questions 面板不属于 MVP。

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Action Taken | Follow-up |
|---|---|---|---|---|---|
| preflight-fold-in-consistency-reviewer | Phase 0.1 / 0.5 ownership conflict | blocking | accept | 将 tool schema gap 明确归到 Phase 0.1；Phase 0.5 只负责 generated JSON/TypeScript/Rust schema freshness。 | Round 6 closure |
| preflight-fold-in-consistency-reviewer | `state_delta_intent` leaked into MVP parser/schema tests | blocking | accept | 将 Phase 3 测试改为只校验 invalid `validity`；`state_delta_intent` 标为 report-only / v1.1 测试债务，不进入 MVP parser/schema/action hard requirement。 | Round 6 closure |
| preflight-fold-in-consistency-reviewer | Phase 8 title may be misread as MVP | minor | accept | 将 Phase 8 标题改为 `Phase 8 (v1.1 / non-MVP)`。 | Round 6 closure can spot-check |
| preflight-fold-in-consistency-reviewer | Viewer section should not imply full cognitive graph in MVP | minor | accept | 将 viewer 段改为“只展示 MVP 字段，不是完整 cognitive graph 面板”，并明确完整 facts/assumptions/decisions/open_questions 延后到 v1.1。 | Round 6 closure can spot-check |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 6
- Blocking re-review launch records:
  - `019e91a4-fbfc-77b0-8233-7144433c3e09` / Euclid
- Rejected findings backed by evidence: none
- Deferred findings documented: none
- Allowed to proceed: yes

## Round 6: Preflight Fold-In Closure Review

### Review Input

#### Objective

检查 Round 5 的两个 accepted blocking findings 是否已经闭合。只聚焦 closure，不重新展开大范围设计审查。

#### Review Target

Round 5 closure after doc fixes.

#### Target Locations

- `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md`
- `vs_review/2026-06-04-taskspace-cognitive-state-engineering-plan-review.md`

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| preflight-fold-in-closure-reviewer | Closure check for Round 5 accepted blockers. | phase ownership, MVP scope leakage |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| preflight-fold-in-closure-reviewer | `multi_agent_v1.spawn_agent` (`explorer`) | `019e91a4-fbfc-77b0-8233-7144433c3e09` / Euclid | spawn_agent tool result | no | Round 6 Closure Input | main-agent history, hidden reasoning, drafts, conclusions, full diff persuasion | yes |

### Reviewer Outputs

#### preflight-fold-in-closure-reviewer

结论：CLOSED。

闭合确认：

- Phase 0.1 / 0.5 归属已自洽：Phase 0.1 明确包含 contract-sketch、真实 `taskspace_control` gap、snapshot join-key；Phase 0.5 只负责 generated JSON/TypeScript/Rust schema freshness。
- `state_delta_intent` 不再是 MVP parser/schema/action hard requirement；Phase 3 只要求 invalid `validity` 错误，并把 `state_delta_intent` 标为 report-only / v1.1 测试债务。
- Phase 8 已标为 `v1.1 / non-MVP`。
- Viewer 段明确 MVP 只展示 MVP 字段，不是完整 cognitive graph 面板，完整面板延后到 v1.1。

剩余非阻塞风险：

- Viewer 展示列表有少量重复字段表述；主 agent 后续已整理成唯一 MVP 字段清单。

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Action Taken | Follow-up |
|---|---|---|---|---|---|
| preflight-fold-in-closure-reviewer | Round 5 blockers closed | blocking closure | accept | 记录 CLOSED。 | 无 |
| preflight-fold-in-closure-reviewer | Viewer list has minor duplicate wording | minor | accept | 整理为唯一 MVP 字段清单：output contracts、fact sources/provenance、result claims/evidence/validity、sentinel warning records、`promotion_not_in_mvp` report marker。 | 无 |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 6
- Blocking re-review launch records:
  - `019e91a4-fbfc-77b0-8233-7144433c3e09` / Euclid
- Rejected findings backed by evidence: none
- Deferred findings documented: none
- Allowed to proceed: yes

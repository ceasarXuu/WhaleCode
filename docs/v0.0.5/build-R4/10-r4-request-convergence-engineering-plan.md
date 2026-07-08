# R4 请求轮数收敛工程计划

- Created: 2026-07-08
- Updated: 2026-07-08
- Version: v0.0.5 build-R4 post-closeout
- Status: Active - Phase 1-4 implemented; H-200 runtime response-recovery boundary fix verified on targeted sample; Phase 5 targeted diagnostics benefit no-go; H-202 fact-source coverage repair in progress
- Owner / Responsible: WhaleCode core runtime
- Related Systems: TaskSpace runtime, ActionMapRuntime, session turn loop, action-contract feedback, active projection, context compiler, benchmark harness
- Related Links:
  - `docs/v0.0.5/build-R4/00-r4-tools-chain-special-project.md`
  - `docs/v0.0.5/build-R4/05-phase-benefit-evidence.md`
  - `docs/v0.0.5/build-R4/06-r4-engineering-closeout.md`
  - `docs/v0.0.5/build-R4/09-r4-takeover-progress-audit-20260703.md`
  - `coe/2026-07-03-05-03-r4-durable-evidence-gates.md`
  - `vs_review/2026-07-07-r4-runtime-boundary-hard-stop-audit.md`
- Risk Level: High
- Plan Type: Full

## 1. Task Classification

| Dimension | Classification |
|---|---|
| Work type | bug fix + architecture change + performance optimization |
| Primary failure | TaskSpace request-count amplification from incomplete state-flow convergence |
| Production surface | CLI provider sampling loop, TaskSpace runtime state, model-visible feedback, benchmark reporting |
| Required evidence | unit tests, loop-level replay fixtures, trace/log artifacts, paired sample reruns, adversarial review |

## 2. Background

R4 已经把 tools 链路的能力层、反馈层、projection/runtime 边界推进到工程可审计状态。最新 runtime-stop closure 删除了非预算 hard-stop，并确认生产 provider-sampling hard stop 只剩 `TaskSpaceProviderBudgetHardStopV1`。

但 R4 public-10 和后续 keyed rerun 证明 TaskSpace utility parity 仍未达成。请求轮数放大是当前成本问题的主因。R4 public-10 的历史基线如下：

```text
heterogeneous-dates:
  standard_model_request_count=1
  taskspace_model_request_count=12
  request_2_plus_cache_hit_rate=0.98556
  taskspace_token_ratio=11.082
```

该证据说明成本放大不是 DeepSeek cache prefix 失效，而是 TaskSpace 在已有工具结果、验证证据或 final readiness 事实后，仍继续触发下一轮 provider request。

后续 H-029 已把 `heterogeneous-dates` 的执行层从 900s timeout 推进到 solved：`taskspace_wall_time_ratio=1.84`、`taskspace_tool_call_ratio=0.46`、`request_2_plus_hit_rate=0.981959`。该 solved run 的 durable closeout 片段没有记录模型请求数，所以 Phase 0 必须重新抽取或复跑得到当前 request-count baseline。public-10 的 `12x` 只能作为历史放大基线，不能作为最新收益对比的唯一 baseline。

## 3. Problem Definition

### 3.1 Current Behavior

TaskSpace 在以下情况下会继续请求模型：

| Case | Current Symptom | Evidence |
|---|---|---|
| inspect 已有成功诊断和工作证据 | 仍继续低价值 read/search | `heterogeneous-dates` result-4 已输出正确数值后继续读 CSV |
| 实现和验证已成功 | success criteria / output contract 仍 open，final answer 被拒 | `heterogeneous-dates` public/hidden validation pass 后 provider request 到 57 |
| tool failure / validation failure / patch failure 已有具体信号 | 下一轮反馈被泛化、降级或 projection 冲淡 | R4 COE H-181/H-188/H-189 相关证据 |
| provider budget 达到阈值 | 历史上 `over_profile_hint` 只是 telemetry；当前已修成 hard baseline | H-009/E-025 |
| recovery 存在但不闭环 | generic recovery 后继续重复无效动作 | public-10 timeout/wrong rows |

### 3.2 Expected Behavior

TaskSpace 不替 Agent 做业务语义决策，但必须把状态账本、工具反馈和上下文证据忠实、可追踪地交给 Agent，并在硬基线处停止。

每一轮 provider request 必须能回答：

```text
why_request_was_needed:
  active_node / pending_tool_result / final_gate_rejection / recovery / budget_grace / user_turn_start
what_evidence_was_already_available:
  latest tool/test/edit/read result ids and visibility
what_adoption_gap_remained:
  open success criteria / output contract / lifecycle result / node completion / none
what_feedback_path_was_used:
  tool_feedback_recovery / no_action_follow_up / final_rejected / empty_follow_up / actionable
```

### 3.3 Gap

当前 runtime 有足够多局部 guard 和 focused tests，但缺少一条贯穿 provider request 的闭环账本：

1. 不知道每一轮请求的真实触发理由是否合理。
2. 不知道已有 evidence 为什么没有被采纳为节点完成、success criteria、output contract 或 final readiness。
3. 不知道下一轮 projection/feedback 是否忠实携带了上一轮 tool result。
4. 真实样本里只能事后肉眼追 trace，无法用机器门禁防止 request-count 回归。

## 4. Goals

| Goal | Benefit | Baseline | Target | Verification |
|---|---|---|---|---|
| G1: provider request reason ledger | 每轮请求可归因，减少肉眼追 trace | request amplification 只能事后从 rollout/provider trace 推断 | 每轮 provider request 有结构化 reason/event，未知 reason 为 0 | focused unit + replay fixture |
| G2: evidence adoption closure | 避免已有成功证据后继续请求 | `heterogeneous-dates` validation pass 后 request 到 57 | final readiness 证据可采纳，合法 final 不再因 open criteria 空转 | targeted rerun + final gate fixture |
| G3: feedback/projection 语义透传 | 减少 generic recovery 和重复低价值动作 | closed-action / failed-tool 可降级为 no-action 或被 projection 稀释 | failed tool/read/test/patch feedback 保持分类、result ref、visibility | action-contract/projection tests |
| G4: request-count benefit proof | 证明修复不是只增加日志 | historical public-10: `heterogeneous-dates` TaskSpace 12 requests vs standard 1; latest H-029 solved baseline must be extracted in Phase 0 | targeted solved sample TaskSpace request ratio <= 3x against the current baseline，且不牺牲 correctness | paired sample rerun |
| G5: runtime boundary preservation | 避免用 runtime 策略控制替代 Agent 智能 | 历史多次 hard-stop 越界 | 新 stop 只允许总预算、协议、权限、工具合同、状态机硬基线 | adversarial review + text audit |

## 5. Non-goals

| Non-goal | Reason |
|---|---|
| 不通过增加语义 hard-stop 强迫 Agent 选择某动作 | 违反 R4 runtime boundary：runtime 只守底线，不替 Agent 做上限策略 |
| 不把 single-sample pass 当成 E3 readiness | R4 closeout 已明确 E3 no-go |
| 不用 cache hit 掩盖 request amplification | high cache hit 已被证明不能抵消多轮请求成本 |
| 不删除 replay/debug 所需 tool trace | R4 tools 链路仍要求可审计和可回放 |
| 不引入自然语言模板绕过模型 | 项目约束禁止固定自然语言智能回复 |

## 6. Constraints And Assumptions

| Type | Item | Handling |
|---|---|---|
| Boundary | TaskSpace 是不可绕过工具和规则化账本，不是语义控制器 | 所有修复必须以 evidence/log/projection fidelity 或硬基线为依据 |
| Context | projection 层只做事实构造、透明裁剪、引用暴露 | 不在 projection 注入思考提示或策略性 next-action coaching |
| Evidence | generated `target/` artifacts 不默认提交 | 将关键 report snapshot 或摘要写入 docs/COE |
| Validation | public benchmark rerun 成本较高 | 先做 loop-level fixture，再跑 3 个 targeted samples |
| Secrets | `.env.local` 可供本机使用，但不得写入仓库 | 文档只记录 preflight 状态，不记录 key |

### Assumptions

| Assumption | Verification Method | If Assumption Fails |
|---|---|---|
| request amplification 主要来自状态流不闭环，而不是 provider/cache 故障 | compare request ledger, cache trace, tool result sequence | 退回 Phase 0，重新分类 provider/cache 层故障 |
| `heterogeneous-dates` 是低噪声 request-count benefit 样本 | rerun with latest binary and compare standard/taskspace | 如果环境污染，换同类 solved-by-both sample |
| 当前 hard-stop boundary closure 仍成立 | text audit only provider budget hard-stop production path | 若发现非预算 stop，先修 boundary 再推进 convergence |

## 7. Dependencies

| Dependency | Type | Current Status | Blocking Risk | Handling Plan |
|---|---|---|---|---|
| DeepSeek API key | environment | Local `.env.local` available by user instruction | real keyed rerun unavailable without key | provider credential preflight; never commit key |
| fresh Whale binary build | environment | build generally available but prior build timeout appeared in docs | targeted rerun cannot prove benefit | require build command pass before sample gate |
| public sample harness | system | R4 harness exists | generated artifacts may be non-durable | archive summary to docs/COE, keep raw under target |
| existing full `codex-core --lib` gate | test | 12 residual failures unrelated to runtime-stop closure | cannot use full suite as sole gate | use focused gates plus record full-suite residuals |

## 8. Current State Summary

| Area | Current State | Remaining Gap |
|---|---|---|
| Provider budget | pre-dispatch hard gate exists | budget stops cap cost but do not prove good convergence |
| Non-budget hard stops | removed in latest closure | loop-level tests still need hardening |
| Feedback classification | `ToolFeedbackRecovery` split from no-action; response actionability is now observational only | H-200 targeted rerun passed: ordinary response actionability no longer creates model-visible recovery guidance |
| Projection | shifted toward factual constructor | some historical paths still require text audits and fixtures |
| Sample evidence | public-10 reports request multipliers; three targeted reruns now complete diagnostically | benefit no-go: org/sqlite paired reruns standard solved but TaskSpace wrong; full public-10 deferred |

## 9. Overall Technical Design

### 9.1 Request Reason Ledger

Introduce a structured request-level trace event, tentatively named `TaskSpaceProviderRequestReasonV1`.

Minimum fields:

| Field | Purpose |
|---|---|
| `request_id` / `turn_id` / `task_id` / `node_id` | Correlate provider request to TaskSpace state |
| `node_kind` / `request_phase` | Explain the phase that requested model sampling |
| `trigger_kind` | `user_turn_start`, `active_node_work`, `tool_feedback_recovery`, `final_gate_rejection`, `empty_follow_up`, `budget_recovery`, `provider_budget_hard_stop` |
| `response_actionability_previous` | Previous response classification when applicable |
| `latest_tool_result_refs` | Result ids and tool names visible before this request |
| `adoption_blockers` | Open criteria, output contract, lifecycle, unreviewed result, missing validation, missing fact source |
| `projection_bundle_hash` | Detect whether provider-visible context changed |
| `model_visible_feedback_refs` | Recovery/projection items expected to be visible |
| `request_reason_delta` | New evidence refs, changed blocker, changed projection, model-emitted control, or none |
| `repeated_same_reason_count` | Count repeated requests with same trigger/blocker and no new evidence/projection delta |
| `reason_confidence` | `direct`, `derived`, `unknown` |

Hard rule: `reason_confidence=unknown` is allowed during Phase 1 discovery but becomes a gate failure after Phase 2.

Hard rule: after Phase 4, a repeated same-reason request with `request_reason_delta=none` must fail the loop fixture unless it is the single documented budget-recovery grace or a hard-baseline stop. This is a detection gate, not a runtime strategy decision.

### 9.2 Evidence Fact Adoption And Blocker Accuracy

Adoption means recording facts into the ledger, not choosing the Agent's next strategy. The repair must distinguish three actors:

| Actor | Allowed Effect |
|---|---|
| `runtime_fact_recorded` | attach result refs to declared criteria, output contracts, lifecycle records, or exact blockers |
| `model_action` | finish nodes, create next nodes, select implementation strategy, choose whether to read/edit/test |
| `hard_baseline` | reject or stop only for protocol, permission, tool contract, total budget, or explicit state-machine invariant |

Hard rule: Phase 2 must not introduce runtime-created semantic finish/transition behavior. Any existing auto-finish / next-node creation path must be audited separately and either tied to a hard baseline with negative tests, converted back to model-emitted `taskspace_control`, or excluded from this plan.

Add focused adoption checks before sending another provider request only to record declared evidence and exact blockers:

| Adoption Surface | Required Behavior |
|---|---|
| successful edit | lifecycle and changed artifact evidence attach to implementation node |
| successful validation | validation result can satisfy related success criteria and output contract |
| final answer candidate | final readiness reports exact unsatisfied criteria, not generic rejection |
| validation rework | blocked validation origin and changed artifact target remain attached |
| inspect success | successful diagnostic and working evidence are recorded with result refs; missing fact sources and unresolved blockers are explicit; runtime does not create an implementation node from this fact alone |

This is not runtime deciding business truth. It is ledger bookkeeping: if a tool result already proves a declared criterion, the ledger records that proof; if it does not, the ledger records the exact blocker. The Agent remains responsible for emitting `taskspace_control` transitions and choosing the next semantic action.

### 9.3 Feedback And Projection Semantics

Keep projection as a constructor:

| Allowed | Not Allowed |
|---|---|
| raw/bounded tool result excerpts | hidden strategy prompts |
| result ids and artifact refs | subjective "you already know enough" assertions |
| truncation boundaries and retrieval refs | closed action-space injection unless state machine hard baseline requires it |
| failure kind and exact command/path | generic no-action downgrade when tool feedback exists |

### 9.4 Loop-Level Regression Harness

Add replay fixtures that model request sequences without requiring full public benchmark cost:

| Fixture | Proves |
|---|---|
| inspect success then repeated read | request reason records available evidence and unchanged blockers; no same-reason request may repeat without new evidence, changed projection, model-emitted control, or hard-baseline stop |
| same blocker and unchanged projection | ledger is not logging-only; fixture fails when another provider request has no delta |
| validation pass with output contract | final readiness closes or reports exact blocker |
| failed tool feedback | stays `tool_feedback_recovery`, not generic no-action |
| duplicate complete read | feedback stays visible and request reason remains auditable |
| provider budget boundary | only total/node budget hard baseline stops sampling |

### 9.5 Benefit Validation Samples

Use three targeted samples before public-10 rerun:

| Sample | Reason | Initial Target |
|---|---|---|
| `heterogeneous-dates` | solved-by-both; isolates request-count cost | TaskSpace solved, current model request count measured, request ratio <= 3x |
| `organization-json-generator` | exposes feedback/projection/rework chain | TaskSpace solved when standard solves; model request count measured; request ratio <= 3x when standard request count is measured; no 900s timeout; no provider-budget terminal counted as pass; over-threshold solved runs are no-go/diagnostic, not benefit pass |
| `sqlite-db-truncate` | long-flow recovery path | TaskSpace solved when standard solves; model request ratio <= 3x or explicit no-go; no timeout; no generic provider-budget terminal counted as pass |

## 10. Phase Gate Overview

| Phase | Independent Verification | Forbidden Future Dependency | Exit Evidence | Completion Required Before Next Phase | Proceed Decision |
|---|---|---|---|---|---|
| Phase 0 Baseline | docs/COE + report snapshot audit | no implementation needed | request amplification taxonomy and baseline table | 100% or explicit residual risk | pause until complete |
| Phase 1 Ledger | focused unit tests and trace fixture | no real sample rerun required | `TaskSpaceProviderRequestReasonV1` event emitted for synthetic paths | 100% | pause until complete |
| Phase 2 Adoption | adoption unit tests and loop fixture | no public benchmark needed | success criteria/output contract/lifecycle adoption passes fixtures | 100% | pause until complete |
| Phase 3 Feedback/projection | text audit + projection tests | no sample benefit needed | no generic downgrade for known feedback classes | 100% | pause until complete |
| Phase 4 Loop harness | sequence replay tests | no real sample needed | request count bounded in fixtures | 100% | pause until complete |
| Phase 5 Targeted samples | paired rerun artifacts | no public-10 rerun needed | 3 sample benefit table | 100% or recorded no-go | pause until complete |
| Phase 6 Public gate update | public-10 report | no E3 needed | updated R4 decision | 100% | decide E3/no-go |

### Phase 1 Implementation Status - 2026-07-08

Phase 1 已落地为观测账本，不是新的语义控制策略。

已实现内容：

1. `provider_request_budget` trace event 追加 `schema:taskspace-provider-request-reason-v1`。
2. 每轮 provider request 记录 `trigger_kind`、`response_actionability_previous`、`latest_tool_result_refs`、`model_visible_feedback_refs`、`adoption_blockers`、`projection_bundle_hash`、`request_reason_delta`、`repeated_same_reason_count`、`reason_confidence`。
3. provider budget pre-dispatch hard baseline 触发时，也记录同一套 reason tags，便于复盘最后一轮为什么停在硬基线。
4. `repeated_same_reason_count` 只是 detector：它揭示相同原因、相同 projection/evidence 下的连续请求，不拒绝、不改写、不替 Agent 选择动作。

边界说明：

| Item | Decision |
|---|---|
| Runtime behavior | 不新增非预算 hard-stop，不新增动作纠正，不新增 strategy prompt |
| Event visibility | 常规 provider lifecycle 继续返回 `TaskspaceTraceEventRecorded`；pre-dispatch hard-stop reason 写入 ActionMap snapshot trace，用于回放和报告 |
| Next dependency | Phase 2 必须基于该账本识别 evidence adoption gap，不能直接加语义 stop |

### Phase 2-4 Implementation Status - 2026-07-08

本轮继续执行后的结论：Phase 2-4 的代码层收敛以“减少 runtime 语义干预、加强可观测账本”为主，而不是新增动作约束。

已实现内容：

1. 生产路径删除 `record_main_tool_result` 中 inspect 读/搜证据达到阈值后自动调用 `force_finish_inspect_for_provider_budget(..., "inspect_progress_convergence")` 的行为。
2. `force_finish_inspect_for_provider_budget`、`force_finish_implement_for_provider_budget`、`force_finish_validation_after_successful_tool` 已收窄为 `#[cfg(test)]`，生产 runtime 不再暴露这些自动阶段迁移入口。
3. validation rework 中 exact duplicate complete target read 不再被 runtime 硬拒绝为 `validation_rework_duplicate_artifact_read`；重复读取是否有价值由 Agent 决策，成本上限仍由 provider budget 硬基线负责。
4. active projection 的 `next_valid_actions` 去掉 `do not repeat`、`do not finish`、`retry edits only`、`do not substitute weaker validation` 等策略指令，改为事实型 marker：`inspect_finish_blocker`、`edit_action_contract`、`finish_node_blocker`、`validation_recovery_fact`。
5. 保留成功验证结果的 adoption 账本能力，但不把 adoption 当作 runtime 自动 finish/transition 的理由。
6. request reason ledger 和 public report extractor 增加 request-reason coverage / unknown / repeated-no-delta 汇总字段，供 Phase 5/6 判断请求放大是否来自语义传递问题。
7. 对抗性审查发现 action-contract transport 仍残留 validation rework duplicate target read 硬拒绝；已删除 `taskspace_closed_validation_rework_read_reject_reason` 生产分支，并把相关 contract/recovery 文案改为低信息量事实 marker，重复 read_file 仍保持 state-machine-legal。
8. H-196 补充清理 feedback/projection 里残留的动作建议句式：`available_actions`、`apply_patch is available`、条件式 `read_file ... only if ...` 等生产文案改为 `state_machine_requirement`、`validation_command_source`、`action_space_source`、`validation_rework_target_read_result`、`duplicate_complete_target_read_signal` 等事实字段。
9. H-200 删除 response actionability 驱动的 model-visible recovery 注入：`response_actionability` 只记录 trace，不再构造 no-action/path-correction/apply-patch/rework/transition/validation recovery item；final/blocked gate rejection 只返回中性 state error。

边界审计：

| Path | Previous Behavior | Current Classification | Current Status |
|---|---|---|---|
| inspect progress convergence | read/search 后 runtime 可自动 finish inspect 并创建 implement node | runtime overreach candidate | production path removed; test-only historical helper retained |
| validation rework duplicate complete read | exact duplicate read 被硬拒绝并生成 recovery | runtime overreach candidate | production block removed; duplicate read remains agent-controlled |
| action-contract duplicate target read | cache-optimized transport 可在 shell read 前硬拒绝重复 target read | runtime overreach candidate | production reject removed after adversarial review; retained only factual low-information marker |
| feedback/projection action-suggestion wording | `available_actions`/`apply_patch is available`/条件式 target read 文案可能像 runtime 策略建议 | feedback constructor overreach candidate | production wording converted to fact/source fields; tests assert old phrases absent |
| provider response actionability recovery | runtime 按 actionability 分类向模型注入 developer recovery / transition-available / validation-closeout item | runtime overreach candidate | production injection removed; actionability remains trace-only |
| successful validation adoption | 成功 test/build 可自动标记 validation result validity | ledger adoption | retained; does not create semantic next node by itself |
| provider budget pre-dispatch | 达到总请求预算停止本 turn provider sampling | hard baseline | retained with reason ledger tags |
| active projection next actions | 混合事实和策略指令 | feedback constructor | rewritten to factual markers plus state-machine tool facts |

已验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib inspect_progress_convergence_records_evidence_without_runtime_transition --locked
  passed: 1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback --locked
  passed: 1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib projection_ --locked
  passed: 22 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib validation_rework_duplicate_read --locked
  passed: 7 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib request_convergence --locked
  passed: 1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib provider_request --locked
  passed: 11 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib action_contract_prompt --locked
  passed: 31 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib implementation_recovery --locked
  passed: 9 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib provider_response_actionability --locked
  passed: 12 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib terminal_gate_rejection_feedback --locked
  passed: 1 test
```

当前解释：

- Phase 2 的 adoption 方向已收敛到“证据账本记录，不自动语义迁移”。
- Phase 3 的 projection/feedback 方向已收敛到“事实 marker + result refs，不注入动作策略”。
- Phase 4 的 loop harness 已落地 request reason no-delta fixture；Phase 5 targeted samples 已跑完并记录 benefit no-go，Phase 6 因 targeted no-go 停在 public-10/E3 之前。

### Phase 5 Targeted Sample Status - 2026-07-08

三个 targeted samples 已按诊断模式跑完。`heterogeneous-dates` 使用 right-only 复跑验证 H-192；`organization-json-generator` 和 `sqlite-db-truncate` 使用 paired 模式验证标准侧与 TaskSpace 侧差异。

```text
target/r4-phase5-stale-final-readiness-fix-20260708-heterogeneous/runs/terminal_bench__heterogeneous-dates/20260708-043442-428/pair-001
target/r4-phase5-request-convergence-org-20260708/runs/terminal_bench__organization-json-generator/20260708-044045-953/pair-001
target/r4-phase5-request-convergence-sqlite-20260708/runs/terminal_bench__sqlite-db-truncate/20260708-044047-534/pair-001
```

计数口径：

- Phase 5 表格里的 `TaskSpace Provider Requests` 使用 canonical 诊断口径：`right/artifacts/request-phase-summary.json.provider_request_distinct_count`。该字段只在 provider lifecycle phase hooks 可用时使用，表示 distinct provider request payload 数；同文件中的 `provider_request_terminal_count` 和 `expected_model_request_count` 在这三次 TaskSpace run 中与它一致。
- 标准侧 paired baseline 没有 TaskSpace provider lifecycle phase events，因此标准侧请求数仍以 `left/artifacts/request-summary.json.model_request_count=1` 作为 baseline。
- `right/artifacts/request-summary.json.model_request_count=1` 是 top-level summary，不代表 TaskSpace provider lifecycle 请求总量，不能作为 TaskSpace 侧请求放大判断口径。
- `right/artifacts/request-summary.json.rollout_trace.model_request_count` 在三次最新 TaskSpace run 中分别为 `12/21/21`，比 canonical distinct provider count `11/20/20` 多 1。该字段保留为 legacy/fallback rollout trace telemetry；当 `request-phase-summary.provider_request_distinct_count` 存在且大于 0 时，R4 public report extractor 现在优先使用 `request_phase_summary_provider_distinct`。

样本结果：

| Sample | Mode | Standard | TaskSpace | TaskSpace Provider Requests | Request Count Source | Request Reason | Terminal / Failure |
|---|---|---|---|---:|---|---|---|
| `heterogeneous-dates` | right-only | not run | solved: public=0, hidden=0 | 11 | `request_phase_summary_provider_distinct` | unknown=0, repeated-no-delta=0 | `final_answer`; exact payload scan clean |
| `organization-json-generator` | paired | solved: public=0, hidden=0 | wrong: public=1, hidden=0 | 20 | `request_phase_summary_provider_distinct` | unknown=0, repeated-no-delta=0 | hard budget after repeated failed `apply_patch`; no `organization.json` generated |
| `sqlite-db-truncate` | paired | solved: public=0, hidden=0 | wrong: public=1, hidden=0 | 20 | `request_phase_summary_provider_distinct` | unknown=0, repeated-no-delta=0 | hard budget after late incorrect `recover.py`; `recover.json` matches only 2 rows |

结论：

- H-192 的具体失败链路已收敛：旧 run 在 final readiness 后继续重复 `state_commit` 并触发 20 request hard budget；新 run 以 `final_answer` 结束。
- 该修复没有新增 runtime action block，也没有强制 Agent final；只是当最新 projection 已关闭旧缺口时，不再把陈旧 recovery 保留进 provider-visible history。
- Phase 5 不支持 benefit pass：两个 paired 样本都证明标准侧可解而 TaskSpace 侧未解，并且均耗尽 20/20 provider request。
- 这两个 no-go 样本不是 request-reason 观测盲区：request reason coverage 为 100%，unknown 为 0，exact payload scan clean。剩余问题更接近 failed-edit/large-inspect feedback 的可用性和 Agent 对反馈的采纳效率，而不是 H-192 stale recovery 或 repeated-no-delta loop。

### H-199 Feedback Fidelity Repair - 2026-07-08

H-193/H-194 后续收敛不采用“让 projection 更会总结修法”的方向。修复原则是反馈层只做事实构造：保留工具原始失败、目标定位、可见性/截断元数据和工具语法事实，不注入 `correction_options`、动作路径建议或 schema repair synthesis。

已落地：

- `apply_patch` 失败反馈改为 `tool_feedback_locator`、`content_visibility_source`、`patch_format_facts` 和 `raw_output`，删除 `correction_options` / `Available correction paths` 类策略字段。
- validation rework patch-only feedback 删除 schema repair synthesis，只复制当前上下文中的 schema contract 证据片段。
- recent tool feedback projection 增加 `body_chars`、`excerpt_chars`、`excerpt_truncated`、`body_omitted_chars`，长输出裁剪变成透明机械事实。
- 新增长输出 projection 回归断言，防止 H-194 类大 evidence 被裁剪但缺少可见性元数据。

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib action_contract_prompt --locked
  passed: 31 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib implementation_recovery --locked
  passed: 9 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib apply_patch --locked
  passed: 55 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib projection_ --locked
  passed: 22 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib validation_rework --locked
  passed: 33 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib taskspace_action_contract --locked
  passed: 77 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale --locked
  passed
```

本机健康构建环境已补全，无需 `CODEX_SKIP_VENDORED_BWRAP=1` 的 `cargo build -p codex-cli --bin whale --locked` 已通过。`CODEX_SKIP_VENDORED_BWRAP=1` 仅作为 focused test 的提速/隔离选项使用。

H-199 后验 targeted sample 诊断：

| Sample | Run Root | Standard | TaskSpace | Key Finding |
|---|---|---|---|---|
| `organization-json-generator` | `target/r4-h199-postfix-org-20260708/.../pair-001` | wrong | wrong | TaskSpace 已生成 `organization.json`，但 `members` 使用姓名而非 id；该 paired run 不能作为 standard-solved 对比 |
| `sqlite-db-truncate` | `target/r4-h199-postfix-sqlite-20260708/.../pair-001` | solved | wrong | TaskSpace 20/20 provider requests 后仍未生成 `recover.json`；trace 暴露 `developer_recovery` 与非 cap recovery guidance 注入 |

该后验结果说明 H-199 已改善一部分反馈可见性，但未解决 runtime response-recovery 越界。尤其是 `sqlite-db-truncate` 中，失败已经不是“没有 request reason”或“exact payload scan 不干净”：`unknown=0`，active projection small 且 replacement confirmed；直接问题是 runtime 把 actionability 分类继续转化成 model-visible recovery guidance，污染了 Agent 的上下文和动作选择。

### H-200 Runtime Response-Recovery Boundary Closure - 2026-07-08

根因：

- `response_actionability.needs_recovery()` 原本不仅用于 trace，还会构造 model-visible developer recovery item。
- 这些 recovery item 包括 no-action、path correction、apply_patch grammar、implementation recovery、inspect transition available、validation closeout available、validation infra recovery 等，已经超出“工具反馈忠实透传 + 状态机硬基线”的边界。
- terminal gate rejection 文字也曾包含“Continue / Correct ...”类动作性提示，属于 runtime 对 Agent 思考层的干扰。

修复：

1. `response_actionability` 保留为观测账本；生产路径不再因 actionability 分类生成 developer recovery。
2. provider response trace 里的 `recovery_action` 只有实际存在 recovery item 时才为 `developer_recovery`；普通 actionability recovery 现在记录为 `none`。
3. 删除 post-completed 阶段基于成功 edit/test/inspect progress 自动插入 transition/validation closeout guidance 的路径；仅保留 provider budget hard-stop。
4. `TaskSpaceFinalAnswerRejectedV1` / `TaskSpaceBlockedResponseRejectedV1` 改为中性 state error，只说明 accepted=false、rejection_reason 和 state_effect。
5. 删除 `Session` 层旧语义查询 wrapper；ActionMap 中只服务历史 recovery prompt 的查询 helper 降为 test-only 或删除。
6. 旧 recovery text builder 降为 `#[cfg(test)]` fixture，普通 `codex-core` lib 构建不再编译这些提示构造器。

边界结论：

- 保留：真实工具结果、state-machine/tool contract 错误、权限/协议错误、provider budget hard baseline。
- 删除：runtime 根据“Agent 可能没理解”而注入下一步建议、纠正路径、transition availability、validation closeout availability。
- 设计倾向更新：当 Agent 低级失败或重复动作时，优先怀疑上下文语义传递、裁剪、引用和工具反馈可见性，而不是优先新增 runtime 约束或提示。

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo check -p codex-core --lib --locked
  passed, no warnings

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib provider_response_actionability --locked
  passed: 12 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib terminal_gate_rejection_feedback --locked
  passed: 1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib taskspace_action_contract --locked
  passed: 77 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib action_contract_prompt --locked
  passed: 31 tests
```

H-200 targeted sample rerun 已完成：

- RunDir: `target/r4-h200-boundary-sqlite-right-20260708/runs/terminal_bench__sqlite-db-truncate/20260708-183418-789/pair-001`
- right-only diagnostic；TaskSpace business success passed，`recover.json` 生成且 public/hidden validation exit code 都为 0。
- `TaskSpaceProviderResponseActionabilityV1 recovery_action=none`，普通 actionability 不再产生 `developer_recovery`。
- 未再出现 “TaskSpace inserted non-cap TaskSpace recovery guidance”。
- 唯一 runtime 插入项是允许保留的 provider budget hard-stop：`TaskSpaceProviderBudgetHardStopV1 reason=provider_request_hard_limit_exceeded request_count=20/20`。

该结果验证 H-200 的边界修复目标已经达成。剩余失败形态已经转移：业务产物正确，但 TaskSpace 生命周期没有收敛，原因不是 response-recovery 注入，而是 H-202 的 fact-source coverage 证据采纳问题。

### H-202 Fact-Source Coverage Alias / Diagnostic Evidence Gap - 2026-07-08

现象：

- `sqlite-db-truncate` right-only rerun 中，Agent 两次显式发出 `taskspace_control(action=finish_node, next_node_kind=implement_solution)`。
- `rollout.jsonl` 确认第二次 `finish_node` 有对应 `function_call_output`，不是控制工具结果丢失。
- runtime 拒绝原因是：`/app/trunc.db` 这个 declared fact-source artifact 仍未有 read/search evidence。
- 同一 run 中，Agent 已多次通过 workspace-relative `trunc.db` 执行二进制诊断读取，并成功解析出 10 行，还写出了正确的 `recover.json`。
- active projection 已能显示 `fact_source_coverage: /app/trunc.db status=not_observed workspace_relative_alias_from_failed_path=trunc.db`，说明 alias 信息存在，但 coverage gate 没有把成功的二进制诊断读取采纳为 source coverage。

根因假设：

- `inspect_missing_required_fact_source_artifacts` 只把成功 `read/search` 结果计为 observed artifact。
- action-contract 下二进制诊断命令通过 `run_test` 发出，因此结果 `action_class=test`，即使命令和输出已经机械证明读取了 `trunc.db`，也不会进入 fact-source coverage。
- 对二进制/结构化输入来说，“必须 read/search”这个实现细节过窄；底线应是“有具体工具证据证明 declared input 被观察过”，而不是限定某个 action class。

边界结论：

- 不应恢复 runtime 强制 transition 或 recovery guidance。
- 修复方向是让 ledger/coverage 忠实承认已有工具证据：成功的二进制/结构化诊断读取可以满足 fact-source coverage；路径 listing、stat-only 或失败的 absolute path 不应满足。
- 这是反馈/账本 fidelity 问题，不是 Agent 智能不足，也不是需要给 Agent 增加新约束。

下一步：

- 在 `inspect_node_observed_artifact_refs` 或等价 helper 中增加保守的 diagnostic-source evidence 判定。
- 添加 fixture：`/app/trunc.db` required，`xxd trunc.db` 成功并输出 hex/cell evidence 后，`finish_node -> implement_solution` 应通过。
- 添加负例：`rg --files`、`ls/wc/file`、失败 `/app/trunc.db` 不应满足 coverage。

### H-201 Candidate: Provider Payload Attribution

H-199/H-200 诊断还暴露一个独立成本问题：active projection 本身较小，但 provider payload 每轮仍约 448KB-474KB，且 `input_tokens` 大部分来自稳定 cached prefix。

当前判断：

- 这不是 H-200 的直接 runtime recovery 注入问题。
- 直接付费成本部分被 cache 缓解，但 request count、latency、provider budget 仍受影响。
- 后续需要单独追踪 provider payload composition：区分 stable prefix、dynamic suffix、active projection、tool schema、history/cache anchor 的占比，避免误把大 cached prefix 当作 projection 失败。

H-201 暂不进入当前修复闭环；H-200 的优先验证目标仍是“非 cap recovery guidance 是否消失”。

### Phase 6 Report Gate Status - 2026-07-08

public-10 report/gate 已支持 request reason coverage 字段：

| Field | Purpose |
|---|---|
| `standard_model_request_count_source` / `taskspace_model_request_count_source` | 请求数来源；TaskSpace 侧优先 `request_phase_summary_provider_distinct`，再 fallback 到 rollout trace / provider cache / summary / metrics |
| `request_reason_coverage_status` | 区分 measured、missing、legacy unavailable |
| `request_reason_event_count` | 当前 run 中 request reason 事件数量 |
| `request_reason_unknown_count` | 未知归因数量，measured run 必须为 0 |
| `request_reason_attribution_coverage` | measured run 的归因覆盖率 |
| `repeated_same_reason_no_delta_count` | 相同原因且无 evidence/projection delta 的重复请求数量 |
| `request_reason_trigger_kind_counts` | 按 trigger_kind 聚合 |
| `request_reason_delta_counts` | 按 request_reason_delta 聚合 |

已验证：

```text
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-cost-instrumentation.ps1
  passed

pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-usage-accounting-gate.ps1
  passed
```

Phase 6 记录 no-go：full public-10 rerun 尚未执行，且 Phase 5 targeted paired samples 已经出现 standard solved / TaskSpace wrong，因此没有继续运行 full public-10 或 E3 的工程依据。后续应先开新的 failed-edit / long-inspect feedback 收敛切片，再重新进入 targeted sample gate。

## 11. Phased Execution Plan

### Phase 0: Baseline And Request Taxonomy

#### Objective

Create a durable baseline that classifies each extra provider request in known R4 samples by trigger reason and evidence adoption gap.

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| R4 docs and COE available | `rg` / file read | source list | core runtime |
| latest runtime boundary closure known | inspect `vs_review/2026-07-07-r4-runtime-boundary-hard-stop-audit.md` | only provider budget hard-stop remaining | core runtime |

#### Design Approach

Build a table from existing artifacts before writing code. The goal is to prevent implementing a ledger that cannot classify known failures.

#### Implementation Tasks

1. Extract request-count baselines from `r4-public-10-tool-stress-report.snapshot.json`.
2. Extract the latest H-029 `heterogeneous-dates` solved-run request count from raw run artifacts if available, or mark it unavailable and require a fresh baseline rerun before Phase 5 can claim benefit.
3. Extract known request amplification root-cause classes from R4 COE.
4. Define `trigger_kind`, `adoption_blocker`, `request_reason_delta`, and `repeated_same_reason_count` semantics in documentation before code.
5. Mark unknown classes explicitly.

#### Deliverables

| Deliverable | Location |
|---|---|
| request amplification taxonomy | this document or follow-up addendum |
| baseline table | R4 docs/COE |
| unknown-class list | R4 docs/COE |

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| taxonomy only | n/a | n/a | doc review | existing report snapshot | none | planned |

#### Logging And Observability Design

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason Field | Correlation / Trace Field | Log Level | Consumer |
|---|---|---|---|---|---|---|---|
| baseline classification | classified | all known samples mapped | unknown class remains | `unknown_reason` | sample id / run dir | n/a | R4 owner |

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | taxonomy covers known evidence | manual audit + review | every cited sample has class or explicit unknown |
| Benefit | baseline can measure request-count reduction | compare target/baseline table | baseline includes standard/taskspace request counts where available |
| Observability | unknowns are visible | doc table | no silent "misc" class |

#### Exit Criteria

- `heterogeneous-dates`, `organization-json-generator`, and `sqlite-db-truncate` have baseline rows.
- Each row has request count or explicit `request_count_unavailable`, terminal reason, likely adoption gap, and evidence path.
- `heterogeneous-dates` latest H-029 baseline is either extracted or marked as requiring fresh rerun before benefit claims.

#### Review Plan

Architecture or benefit review before implementation.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| stale generated artifacts | wrong baseline | snapshot differs from raw target | cite durable snapshot and mark raw unavailable | rerun sample before Phase 5 |

#### Gate To Next Phase

| Gate Condition | Verification Evidence | Completion Status | User Approval Required | Proceed Decision |
|---|---|---|---|---|
| baseline taxonomy complete | doc table | planned | no | pause |

### Phase 1: Provider Request Reason Ledger

#### Objective

Emit a structured reason event for every provider request and every pre-dispatch provider-budget hard stop.

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| Phase 0 taxonomy approved | doc gate | trigger/adoption enum list | core runtime |
| provider request entry point identified | source inspection | `session/turn.rs` request path | core runtime |

#### Design Approach

Record the reason before provider dispatch. The event is observational first; it must not change action legality.

#### Implementation Tasks

1. Add request reason builder near provider dispatch.
2. Capture active node, request phase, previous response actionability, latest tool result refs, adoption blockers.
3. Emit trace/log event before `stream_with_provider_request_budget`.
4. Emit a separate terminal reason event when provider-budget hard stop blocks dispatch.
5. Track `request_reason_delta` and `repeated_same_reason_count`.
6. Add tests for user start, active node work, recovery, final rejection, repeated same-reason requests, and budget hard stop.

#### Deliverables

| Deliverable | Location |
|---|---|
| request reason event | `third_party/codex-cli/codex-rs/core/src/session/turn.rs` or adjacent trace module |
| reason event tests | `codex-core --lib provider_request_reason` |
| report extractor update | `scripts/taskspace-benchmark` if needed |

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| request reason event | session provider dispatch path | CLI model sampling | focused unit tests | trace event in whale-exec | none | planned |
| hard-stop reason event | pre-dispatch budget gate path | CLI model sampling | provider budget tests | `TaskSpaceProviderBudgetHardStopV1` correlated event | none | planned |
| repeated same-reason detector | session/request reason builder | CLI model sampling | repeated-reason unit tests | `request_reason_delta` and count fields | none | planned |

#### Logging And Observability Design

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason Field | Correlation / Trace Field | Log Level | Consumer |
|---|---|---|---|---|---|---|---|
| provider request dispatch | pre_dispatch | `reason_confidence=direct` | `reason_confidence=unknown` | `unknown_reason` | `request_id`, `turn_id`, `node_id` | info | R4 report / diagnostics |
| repeated reason detection | pre_dispatch | new evidence or changed projection recorded | same trigger/blocker with no delta | `request_reason_delta` | `request_id`, `previous_request_id`, `projection_bundle_hash` | warn | R4 report / diagnostics |
| provider hard stop | blocked | `trigger_kind=provider_budget_hard_stop` | missing correlated reason | `budget_reason` | `request_id`, `node_id` | warn | R4 report / diagnostics |

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | each provider request has reason | unit/fixture | no request without reason event |
| Benefit | request amplification diagnosable | synthetic sequence report | report can group requests by trigger/adoption blocker |
| Benefit | logging-only loop rejected | repeated same-reason fixture | unchanged reason/projection/no-new-evidence cannot pass as convergence |
| Observability | trace fields present | snapshot/test | required fields non-empty except documented none |

#### Exit Criteria

- Focused tests pass.
- Text scan shows no new non-budget hard-stop marker.
- Reason event can be extracted from a synthetic or small local run.

#### Review Plan

Implementation review after code lands.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| event logs too much context | payload/log bloat | large event body | store refs/hashes, not raw output | disable verbose fields |
| event changes control flow | boundary regression | tests show action legality changed | keep builder read-only | revert ledger commit |

#### Gate To Next Phase

| Gate Condition | Verification Evidence | Completion Status | User Approval Required | Proceed Decision |
|---|---|---|---|---|
| reason ledger lands read-only | tests + trace snapshot | planned | no | pause |

### Phase 2: Evidence Fact Adoption And Blocker Accuracy

#### Objective

Ensure successful tool/test/edit evidence is recorded into node lifecycle, success criteria, output contract, and final readiness when it already satisfies declared ledger requirements, without runtime-created semantic finish or next-node decisions.

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| Phase 1 reason ledger available | test/log | request reason events | core runtime |
| known adoption gaps listed | Phase 0 table | adoption blocker enum | core runtime |

#### Design Approach

Fix ledger adoption gaps. Do not infer unstated business truth. Adoption must cite declared criteria, output contract, changed artifacts, validation results, or lifecycle dependencies. If a next semantic step is needed, the runtime may expose exact blockers and available state, but the Agent must emit the transition/control action unless a separate hard-baseline rule applies.

#### Implementation Tasks

1. Add/repair adoption checks for output contract validation success.
2. Attach implementation edit/lifecycle evidence to dependent validation closeout when state contract allows.
3. Emit exact final readiness blockers when final is rejected.
4. Add negative tests proving missing declared fact sources or open output contracts cannot be auto-finished or auto-transitioned by runtime.
5. Audit existing inspect auto-finish / next-node creation paths and classify each as `model_action`, `hard_baseline`, or `runtime_overreach_candidate`.
6. Add regression tests for `heterogeneous-dates`-style validation pass with open criteria.

#### Deliverables

| Deliverable | Location |
|---|---|
| adoption code | `action_map/runtime.rs`, `session/turn.rs` as needed |
| boundary audit table for finish/transition paths | R4 docs/COE |
| final readiness tests | `codex-core --lib final_readiness` / existing focused filters |
| adoption reason fields | request reason ledger |

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| output contract adoption | ActionMap final readiness | `taskspace_control` / final gate | focused tests | request reason no longer says output contract open after pass | none | planned |
| success criteria adoption | ActionMap success criteria state | final gate | focused tests | criterion evidence refs populated | none | planned |
| finish/transition boundary audit | action-contract/session transition helpers | taskspace_control and pre-dispatch checks | boundary negative tests | `adoption_actor` trace field | none | planned |

#### Logging And Observability Design

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason Field | Correlation / Trace Field | Log Level | Consumer |
|---|---|---|---|---|---|---|---|
| evidence adoption | evaluated | criterion/output contract satisfied with result ref | blocker remains open | `adoption_failure_reason` | result id / criterion id / `adoption_actor` | info/warn | R4 diagnostics |
| transition boundary | evaluated | model-emitted control or hard-baseline classification | runtime-created semantic transition | `transition_actor_violation` | node id / request id | warn/error in tests | R4 diagnostics |
| final gate | rejected | exact blocker emitted | generic final rejection | `final_blocker_kind` | node id / result id | warn | Agent + diagnostics |

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | no false adoption | negative tests | criteria remain open when evidence does not satisfy declared contract |
| Correctness | no runtime semantic transition | boundary property tests | runtime does not create finish/next-node from partial evidence |
| Benefit | fewer post-validation requests | replay fixture | final pass does not generate repeated model requests |
| Observability | blockers exact | snapshot | no generic rejection for known blocker classes |

#### Exit Criteria

- Focused adoption tests pass.
- Boundary audit classifies all finish/transition paths touched by this plan.
- Negative tests prove missing declared fact sources cannot be auto-finished or auto-transitioned.
- Request reason fixture shows final readiness blocker disappears after satisfying evidence.

#### Review Plan

Implementation and test-validity review if adoption logic expands.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| false positive final readiness | wrong answer accepted | negative final gate test fails | require declared contract/result refs | revert adoption rule |

#### Gate To Next Phase

| Gate Condition | Verification Evidence | Completion Status | User Approval Required | Proceed Decision |
|---|---|---|---|---|
| adoption closure covered | tests + request reason fixture | planned | no | pause |

### Phase 3: Feedback And Projection Semantic Integrity

#### Objective

Prevent tool feedback from being distorted, lost, or downgraded between tool result, recovery item, active projection, and next provider payload.

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| Phase 1 reason ledger available | logs/tests | model-visible feedback refs | core runtime |
| known feedback classes listed | COE taxonomy | class list | core runtime |

#### Design Approach

Keep feedback factual. If content is too large, expose bounded excerpt plus ref. If a recovery item is inserted, classify it specifically.

#### Implementation Tasks

1. Add feedback-class preservation tests across failed tool, patch failure, validation failure, duplicate read, closed-action rejection.
2. Make projection scanner assert factual sections and forbid strategy-injection phrases in active projection.
3. Ensure generic `TaskSpaceNoActionRecoveryV1` is only used when no actionable tool/control/final result exists.
4. Add request reason correlation to projection bundle hash and feedback refs.

#### Deliverables

| Deliverable | Location |
|---|---|
| feedback class tests | `codex-core --lib action_contract_prompt`, `validation_rework`, `provider_response_actionability` |
| projection text audit | test or script |
| reason-feedback correlation | trace event |

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| feedback class preservation | session recovery/projection paths | provider payload | focused tests | feedback refs in reason ledger | none | planned |
| projection strategy audit | active projection builder | provider payload | text audit tests | payload scan | none | planned |

#### Logging And Observability Design

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason Field | Correlation / Trace Field | Log Level | Consumer |
|---|---|---|---|---|---|---|---|
| feedback projection | included | result ref visible or retrievable | missing expected feedback ref | `projection_omission_reason` | result id / projection hash | warn | R4 diagnostics |
| recovery classification | classified | specific recovery kind | generic no-action for actionable output | `recovery_misclassification` | response item id | warn/error in tests | R4 diagnostics |

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | feedback class preserved | unit/snapshot | no known class downgraded |
| Benefit | repeated low-value loops reduced in fixture | loop fixture | next request sees exact feedback |
| Observability | omission reason explicit | scan/log test | no silent omission |

#### Exit Criteria

- Focused feedback/projection suites pass.
- Text scan finds no new strategy-control projection phrases outside hard baseline tests.

#### Review Plan

Architecture review focused on runtime boundary and projection semantics.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| projection grows too large | cost regression | payload token increase | bounded excerpts + refs | fold low-priority logs |
| feedback becomes directive | boundary regression | review/text audit finding | factual wording rules | revert wording |

#### Gate To Next Phase

| Gate Condition | Verification Evidence | Completion Status | User Approval Required | Proceed Decision |
|---|---|---|---|---|
| semantic integrity proven in fixtures | tests + text audit | planned | no | pause |

### Phase 4: Loop-Level Request Regression Harness

#### Objective

Prove the repaired flow reduces unnecessary provider requests in controlled sequences before paying for real sample reruns.

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| Phase 1-3 focused gates pass | test output | CI/local commands | core runtime |
| request reason extractor available | command/test | grouped reason report | core runtime |

#### Design Approach

Build compact deterministic fixtures that simulate sequence states and assert request count/reason transitions.

#### Implementation Tasks

1. Create loop fixture harness or extend existing session/action-map tests.
2. Add assertions for max request count per sequence.
3. Assert terminal reason is exact, not generic timeout/budget drain.
4. Wire fixture output into R4 evidence docs.

#### Deliverables

| Deliverable | Location |
|---|---|
| loop regression tests | `codex-core --lib request_convergence` or existing module |
| request reason summary artifact | `target/` plus docs summary |

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| loop fixtures | session/action-map test harness | provider request simulation | request convergence tests | reason summary artifact | test-only fixture, blocks sample gate if absent | planned |

#### Logging And Observability Design

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason Field | Correlation / Trace Field | Log Level | Consumer |
|---|---|---|---|---|---|---|---|
| loop fixture | replayed | request count within threshold | extra request reason present | `unexpected_request_reason` | fixture id | test output | developers |

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | fixtures model known failures | test review | each fixture maps to COE evidence |
| Benefit | request count reduced in fixture | test assertion | no unnecessary request after satisfying state |
| Observability | reason summary generated | artifact check | each request has reason |

#### Exit Criteria

- All loop fixtures pass.
- Each fixture links to a real R4 failure class.

#### Review Plan

Test-validity review if fixtures become primary gate.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| fixtures overfit implementation | false confidence | sample rerun regresses | map every fixture to runtime trace and include black-box sample gate | add real rerun earlier |

#### Gate To Next Phase

| Gate Condition | Verification Evidence | Completion Status | User Approval Required | Proceed Decision |
|---|---|---|---|---|
| loop-level no-extra-request proof | tests + artifact | planned | no | pause |

### Phase 5: Targeted Sample Benefit Validation

#### Objective

Measure whether request convergence improves real tasks without hiding correctness regressions.

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| Phase 4 loop fixtures pass | test output | request convergence tests | core runtime |
| fresh binary built | build command | Whale binary | core runtime |
| provider preflight passes | harness preflight | no missing credential | core runtime |

#### Design Approach

Run right-only where appropriate for cost control, then paired rerun if a benefit claim needs standard comparison.

#### Implementation Tasks

1. Rerun `heterogeneous-dates` with latest binary and current model request count extraction.
2. Rerun `organization-json-generator` paired when standard solves; if TaskSpace does not solve or exceeds request ratio <= 3x, record no-go and do not count as benefit pass.
3. Rerun `sqlite-db-truncate` paired when standard solves; if TaskSpace does not solve, record no-go and do not count as benefit pass.
4. Extract request reason summary, request counts, tool counts, token/cache, outcome, terminal marker.
5. Update R4 evidence docs.

#### Deliverables

| Deliverable | Location |
|---|---|
| targeted sample reports | `target/...` with durable docs summary |
| benefit comparison table | `docs/v0.0.5/build-R4/05-phase-benefit-evidence.md` or addendum |
| no-go/go decision | R4 docs |

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| targeted rerun | whale CLI benchmark harness | Terminal-Bench sample run | harness gates | three targeted pair reports/request reason summaries | none | diagnostic complete; benefit no-go |

#### Logging And Observability Design

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason Field | Correlation / Trace Field | Log Level | Consumer |
|---|---|---|---|---|---|---|---|
| sample run | completed | solved or exact blocker with request reasons | timeout or unknown terminal reason | `terminal_reason` | sample id / run id | report | R4 owner |
| sample benefit gate | evaluated | `sample_pass_eligibility=pass` with request/wall/token threshold fields | over-threshold, missing threshold, or diagnostic no-go | `request_ratio_result` / `wall_time_ratio_result` / `token_ratio_result` / `diagnostic_no_go` | sample id / run id | report | R4 owner |

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | public validation | sample validator | no solved claim without validator evidence |
| Benefit | `heterogeneous-dates` request ratio | compare current baseline | TaskSpace solved, request count measured, ratio <= 3x |
| Benefit | `organization-json-generator` request/correctness | paired rerun | if standard solves, TaskSpace must solve with measured request ratio <= 3x and no provider-budget terminal; otherwise no-go |
| Benefit | `sqlite-db-truncate` request/correctness | paired rerun | if standard solves, TaskSpace must solve with measured request ratio <= 3x or record no-go |
| Benefit | wall/token side effects | report comparison | side-effect thresholds must be declared before rerun; if unavailable, wall/token are observational and `sample_pass_eligibility` cannot be `pass` on them |
| Observability | request reason coverage | report extractor | unknown reason count 0 |

#### Exit Criteria

- `heterogeneous-dates` benefit target passes or no-go is recorded with exact reason.
- `organization-json-generator` and `sqlite-db-truncate` either pass correctness/request gates, including request ratio <= 3x when standard request count is measured, or are recorded as no-go; exact blocker alone is diagnostic evidence, not benefit pass.
- All sample outputs have durable summary entries.

#### Review Plan

Benefit-realization review before updating public-10 gate.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| sample cost high | slows iteration | repeated 900s timeout | right-only first, stop on exact blocker | return to Phase 2/3 |
| standard side also wrong | weak comparison | standard public validation fails | mark evidence E1/E2-candidate only | choose alternate paired sample |

#### Gate To Next Phase

| Gate Condition | Verification Evidence | Completion Status | User Approval Required | Proceed Decision |
|---|---|---|---|---|
| targeted sample pass/no-go table complete with request, wall, token, correctness, and eligibility fields | sample reports + doc summary | complete as diagnostic no-go | no | stop before full public-10/E3 |

### Phase 6: Public Gate Update And E3 Decision

#### Objective

Update R4 public-10 evidence only after targeted samples prove the mechanism works.

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| Phase 5 targeted evidence complete | docs/run artifacts | benefit table | core runtime |
| report generator supports request reason fields | script test | report gate pass | core runtime |

#### Design Approach

Run or update public-10 report with request reason fields. A full public-10 rerun is required for any go/E3 progression. A selected subset rerun is allowed only as diagnostic evidence or no-go evidence; omitted rows must be marked `not_evaluated_for_go` and cannot contribute to a pass.

#### Implementation Tasks

1. Add request reason columns to public-10 report if not already present.
2. Rerun full public-10 before any go decision. If cost requires a selected subset first, label it diagnostic/no-go only and record omitted rows.
3. Update closeout decision.
4. Record E3 go/no-go with explicit evidence.

#### Deliverables

| Deliverable | Location |
|---|---|
| updated public-10 report | `target/...` plus durable snapshot if accepted |
| updated R4 closeout addendum | docs |
| E3 decision | docs |

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| public report update | benchmark scripts | public-10 report generation | report gate tests | no full public-10 rerun after Phase 5 no-go | none | report fields implemented; release no-go recorded |

#### Logging And Observability Design

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason Field | Correlation / Trace Field | Log Level | Consumer |
|---|---|---|---|---|---|---|---|
| public gate | evaluated | go/no-go with measured ratios | missing request reason, unknown terminal reason, omitted row, or explicitly unavailable row in go decision | `gate_failure_reason` / `omission_reason` | sample id | report | release owner |

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | public-10 completeness | report gate | all planned rows complete; explicitly unavailable rows force blocked/no-go for any go decision |
| Benefit | request/cost improvement | report comparison | no solved sample with unbounded request amplification |
| Benefit | no subset cherry-pick | report gate | go decision requires full public-10; subset reports can only be diagnostic/no-go |
| Observability | reason coverage | report gate | unknown request reason count 0 for measured rows |

#### Exit Criteria

- Public gate records R4 request convergence outcome.
- E3 decision remains no-go unless utility and request-count gates pass.

#### Review Plan

Adversarial release/benefit review.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| public-10 still negative | E3 blocked | utility/cost no-go | record no-go and open next plan | do not run E3 |

#### Gate To Next Phase

| Gate Condition | Verification Evidence | Completion Status | User Approval Required | Proceed Decision |
|---|---|---|---|---|
| release decision evidenced | Phase 5 no-go + report-field tests + review | no-go recorded; full public-10 deferred | no | stop |

## 12. Implementation Completeness Matrix

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|---|
| Request reason ledger | every provider request has reason | `action_map/runtime.rs` trace enrichment | provider sampling lifecycle | `provider_request` 11/11; `provider_response_actionability` 10/10 | `schema:taskspace-provider-request-reason-v1` reason tags | none | implemented 2026-07-08 |
| Repeated same-reason detector | unchanged reason/projection/evidence is observable without runtime blocking | `action_map/runtime.rs` reason delta builder | provider sampling lifecycle | `provider_request_reason_ledger_counts_repeated_no_delta_requests` | `request_reason_delta`, `repeated_same_reason_count` | none | detector implemented; no gate by design |
| Evidence fact adoption repair | satisfied criteria/contracts record result refs without runtime semantic transition | `action_map/runtime.rs` | final gate / taskspace_control | boundary negative tests | criterion/output contract refs, `adoption_actor` | none | implemented for current slices; org/sqlite expose next feedback-actionability slice |
| Feedback semantic integrity | known tool feedback not downgraded | session recovery/projection paths | provider payload | action-contract/projection tests | feedback refs and projection hash | none | implemented for active projection / duplicate-read overreach slice |
| Loop regression harness | known loops do not create extra requests | test harness | codex-core tests | request convergence tests | reason summary artifact | test-only but required before sample gate | implemented initial no-delta fixture; H-192 sample passed; utility no-go recorded |
| Targeted sample reruns | real correctness/request benefit passes or no-go is recorded | benchmark harness | Terminal-Bench samples | harness gates | heterogeneous/org/sqlite pair reports | none | diagnostic complete; benefit no-go |
| Public gate update | full public-10 protects go/E3 decision from cherry-pick | report scripts | public-10 report | report gate | no full public-10 rerun after targeted no-go | none | no-go recorded; rerun deferred |

## 13. Risks, Dependencies, And Mitigations

| Risk | Probability | Impact | Trigger Signal | Mitigation | Fallback |
|---|---:|---:|---|---|---|
| Runtime boundary regression | Medium | High | new non-budget hard-stop or strategy projection text | text audit + adversarial review | revert offending stop/projection wording |
| False final readiness | Medium | High | validator negative test fails or wrong output accepted | require declared criteria and result refs | disable adoption rule |
| Observability bloat | Medium | Medium | trace size increases materially | hash/ref large fields | sample only low-priority detail |
| Fixture overfitting | Medium | Medium | public sample still amplifies requests | link fixtures to real traces and run targeted samples | add missing failure class |
| Provider/environment noise | Medium | Medium | standard side wrong or timeout before first event | preflight and evidence-level downgrade | rerun or choose alternate sample |

## 14. Testing And Validation Strategy

| Layer | Command / Artifact | Passing Standard |
|---|---|---|
| Formatting | `cargo fmt --check` in Rust workspace | pass |
| Diff hygiene | `git diff --check` | pass |
| Focused unit | `cargo test -p codex-core --lib provider_request --locked` | all pass |
| Existing R4 regression | `provider_response_actionability`, `no_action_recovery`, `validation_rework`, `action_contract_prompt`, `provider_budget` | no regression |
| Loop fixture | `request_convergence` filter | request count/reason assertions pass |
| Targeted samples | harness output | solved or exact terminal no-go, no unknown reason |
| Public gate | R4 public-10 report gate | request fields complete for measured rows |

## 15. Release, Rollback, And Fallback Strategy

| Stage | Release Strategy | Rollback / Fallback |
|---|---|---|
| Ledger only | land as observational trace first | revert ledger commit if trace breaks or bloats output |
| Adoption changes | land behind focused tests and targeted rerun | revert specific adoption rule; keep ledger for diagnosis |
| Projection/feedback changes | land with text audit | revert wording/classification change |
| Report updates | add fields while preserving old fields | keep old report parser path until new gate passes |

No E3 progression is allowed from this plan until Phase 6 records a measured go decision.

## 16. Observability And Success Metrics

| Metric | Baseline | Target | Measurement |
|---|---|---|---|
| `request_reason_unknown_count` | unavailable | 0 for measured targeted samples | request reason report |
| `taskspace_model_request_ratio` for `heterogeneous-dates` | historical 12x; latest right-only diagnostic rerun solved at 11 distinct provider requests | <= 3x against a paired current standard baseline; until paired baseline exists, diagnostic closure only | paired rerun with baseline comparison; not enough for go |
| targeted org/sqlite correctness | public-10 negative: org timeout, sqlite wrong | solved when standard solves and request ratio <= 3x when standard request count is measured; otherwise explicit no-go | paired rerun result: both no-go |
| targeted sample pass eligibility | unavailable | every targeted row has `request_ratio_threshold`, `request_ratio_result`, `wall_time_ratio_threshold`, `wall_time_ratio_result`, `token_ratio_threshold`, `token_ratio_result`, `sample_pass_eligibility`, `diagnostic_no_go`, `standard_solved`, `taskspace_solved` | targeted report |
| repeated same-reason no-delta requests | unavailable | 0 passing fixtures | request convergence fixture |
| generic no-action after actionable tool feedback | historically present | 0 in focused fixtures | actionability/recovery tests |
| provider budget hard-stop terminal rate in targeted samples | present in many traces | not used as generic convergence substitute; exact reason recorded | targeted sample reports |
| cache hit | request 2+ around 0.98556 in baseline sample | no material regression while request count drops | provider cache trace |

## 17. Security And Permission Review

| Area | Rule |
|---|---|
| Secrets | Do not log API keys, env values, or `.env.local` content |
| Tool output | Do not expand large raw stdout/stderr in request reason events; use result refs/hashes |
| File paths | Paths may be logged when already part of model-visible tool feedback; avoid private external paths in durable docs |
| Provider payload | Ledger must not add hidden prompt content or user-secret data |

## 18. API / Compatibility Strategy

| Surface | Compatibility Rule |
|---|---|
| Existing trace consumers | add fields/events without removing existing event names |
| Public report scripts | preserve existing fields; add request reason fields as optional until gate migration |
| Replay artifacts | include schema version in new event |
| Provider API | no provider request protocol change |

## 19. Open Questions

| Question | Why It Matters | Resolution Path |
|---|---|---|
| Should `TaskSpaceProviderRequestReasonV1` live in session trace or ActionMap trace? | ownership and dependency direction | Phase 1 design check |
| What exact current request-count baseline should replace historical `heterogeneous-dates` 12x after H-029? | benefit gate strictness | partially resolved: TaskSpace right-only current count is 11; paired standard baseline still required for benefit ratio |
| Should the initial <=3x request-ratio threshold be tightened after a clean org/sqlite baseline? | benefit gate strictness | Phase 5 baseline table; until then >3x is no-go, not pass |
| Should public-10 full rerun happen before or after three targeted samples pass? | cost control | resolved for this slice: full public-10 is deferred because Phase 5 targeted paired samples produced benefit no-go |
| Which full-suite residual failures block release independently of R4? | release readiness | separate full-suite cleanup plan |

## 20. Decision Log

| Date | Decision | Rationale |
|---|---|---|
| 2026-07-08 | Treat request amplification as state-flow closure, not budget tuning | high cache hit plus high request ratio proves cache is not primary cause |
| 2026-07-08 | Keep runtime boundary: no new semantic hard-stop as primary fix | aligns with R4 boundary clarification and latest adversarial closure |
| 2026-07-08 | Require request reason ledger before more broad reruns | avoids paying for samples without knowing why requests continue |
| 2026-07-08 | Treat stale final-readiness recovery as context-fidelity bug, not Agent-action violation | latest projection had closed the missing ledger ids, so preserving old recovery text was contradictory feedback |
| 2026-07-08 | Stop before full public-10 after targeted paired no-go | org/sqlite standard solved while TaskSpace wrong, so full public-10 cannot produce a go signal without another repair slice |

## 21. Change Log

| Date | Change |
|---|---|
| 2026-07-08 | Added H-199 feedback fidelity repair: failed-edit / long-inspect feedback now preserves facts, locators, and truncation metadata without projection repair synthesis |
| 2026-07-08 | Ran `organization-json-generator` and `sqlite-db-truncate` paired diagnostics; both recorded as Phase 5 benefit no-go |
| 2026-07-08 | Added H-192 stale final-readiness recovery repair and `heterogeneous-dates` right-only diagnostic rerun results |
| 2026-07-08 | Implemented Phase 1 provider request reason ledger in ActionMap trace; repeated same-reason is detector-only |
| 2026-07-08 | Initial engineering plan drafted for R4 request convergence |

## 22. Plan Quality Checklist

- [x] Problem definition separates current behavior, expected behavior, and gap.
- [x] Goals include measurable benefit targets.
- [x] Non-goals preserve runtime boundary.
- [x] Phases are independently verifiable.
- [x] Phase gates pause unless evidence is complete or residual risk is approved.
- [x] Logging design covers request trigger, adoption, feedback, and public gate chain.
- [x] Implementation completeness matrix distinguishes planned work from landed production paths.
- [x] Rollback and fallback are recorded.
- [x] Security handling avoids secret/raw-output leakage.
- [x] E3 remains blocked until measured evidence changes.

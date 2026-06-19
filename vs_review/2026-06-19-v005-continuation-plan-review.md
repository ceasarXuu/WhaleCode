# Subagent VS Review: v0.0.5 Continuation Plan

- Created: 2026-06-19T12:30:00+08:00
- Updated: 2026-06-19T12:58:00+08:00
- Task: 对 v0.0.5 继续开发方案执行对抗性审查，确认方案是否足以支撑实际成本控制、E3 门禁和版本收口判断。
- Report path: `vs_review/2026-06-19-v005-continuation-plan-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: blocked

## Round 1: Plan Viability Review

### Review Input

#### Objective
确认 v0.0.5 继续开发方案是否能修正此前误判：v0.0.5 目标包含实际成本控制，不能只做可观测；正式 E3 只能在代码完整、非 agent 门禁通过、用户批准后运行；最终版本结论必须基于合格的 Terminal-Bench 证据。

#### Review Target
架构方案、实验制度、E3/release 门禁和当前实现落点。

#### Target Locations
- `docs/v0.0.5/17-unfinished-work-inventory.md`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- `docs/experiments/README.md`
- `docs/experiments/taskspace-evidence-levels-and-samples.md`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1`
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1`
- `scripts/taskspace-benchmark/write-release-decision.ps1`
- `scripts/taskspace-benchmark/test-e3-start-gate.ps1`
- `scripts/taskspace-benchmark/test-release-decision.ps1`
- `third_party/codex-cli/codex-rs/core/src/client.rs`

#### Change Introduction
方案把 v0.0.5 从“收口候选”改回“继续开发”，要求把成本治理从 artifact/report 推进到 active execution path。已先落地 release decision 和 E3 start gate 的部分脚本门禁；Rust provider request budget hook 正在实现中但尚未提交。

#### Risk Focus
- 方案是否仍然可能把 observability 当成成本控制，从而无法阻断真实 provider/model request。
- Phase 0A/Phase 1 是否有足够明确的 runtime hook、budget state 和 recovery contract。
- active context replacement proof 是否证明 provider-visible context 真正替换高成本历史，而不是只生成旁路证据。
- E3/release 门禁是否能防止未完成代码、错误样本或候选证据被误当作正式 E3。
- 方案是否有过度硬停导致正确率下降且缺少 recovery/final synthesis 路径。
- 当前已落地脚本门禁和未提交 `client.rs` 改动是否暴露方案/实现不一致。

#### Verification Status
- 已通过非 agent 脚本自测：
  - `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1 -RunRoot target\v005-release-decision-selftest`
  - `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1 -RunRoot target\v005-e3-start-gate-selftest`
- 当前仍未运行正式 E3；在代码完整和门禁通过前禁止运行真实 E3。
- 当前工作区存在未提交 `third_party/codex-cli/codex-rs/core/src/client.rs` 改动，属于 provider request budget hook 的进行中实现。

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Produce blocking findings first; do not try to confirm the existing plan.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| architecture-adversary | 方案触及 client/session/runtime/action_map/benchmark gate 边界，风险是 hook 放错层或职责混乱。 | architecture, state flow, module boundaries |
| test-validity-adversary | 此前出现过 E3 证据误判，当前最重要风险是测试/门禁再次允许错误结论。 | experiment validity, release gate, self-deceptive tests |
| observability-adversary | 成本治理依赖 budget event、trace、artifact 和诊断闭环，风险是日志不能证明或不能诊断真实失控。 | logging, budget evidence, diagnostics |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| architecture-adversary | `multi_agent_v1.spawn_agent` explorer | `019ede1f-10aa-7213-9e2d-2e3aa33fd65d` / Gauss | spawn tool result in current Codex session | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| test-validity-adversary | `multi_agent_v1.spawn_agent` explorer | `019ede1f-807f-7fa1-9122-e79d1efc4c18` / Turing | spawn tool result in current Codex session | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| observability-adversary | `multi_agent_v1.spawn_agent` explorer | `019ede1f-e73d-7b82-a360-ad26f32a3c72` / Avicenna | spawn tool result in current Codex session | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### architecture-adversary / Gauss

##### Summary
方案方向正确，但 provider budget state、TaskSpace runtime state、request phase attribution 和 final/recovery policy 还没有完全接成一条可执行架构链。当前 `client.rs` 已开始的 budget context 代码仍是 standalone counter，尚未接入 dispatch 或 ActionMap/TaskSpace state。

##### Blocking Findings
- Provider request budget code exists but is currently orphaned. `ProviderRequestBudgetContext` and `before_dispatch` exist in `client.rs`, but HTTP/WebSocket dispatch still directly calls provider `stream_request`.
- The plan puts budget state in `ActionMapRuntimeState`, but provider dispatch lacks a deliberate path to that state.
- Current release gates can still pass report-only projection, not active replacement.
- E3 start gate markers are too weak to prove runtime cost control.
- Final synthesis and recovery are named, but the exact runtime owner and final/abort contract are not pinned.

##### Non-blocking Risks
- Default budgets are plausible but arbitrary and need calibration fixtures.
- Legacy action allowlist for focused correction is still under-specified.
- WebSocket warmup accounting must be explicitly decided.

##### Required Fixes
- Wire provider budget enforcement at HTTP and WebSocket dispatch points before `stream_request`.
- Pass TaskSpace/request budget context into `ModelClientSession::stream(...)` or equivalent request context.
- Make release decision require active replacement, runtime budget response, state_commit displacement, spawn budget and request phase gates.
- Replace marker-only E3 start checks with content validation of the non-agent gates artifact.
- Define a single runtime owner for budget hard-stop recovery and final/abort behavior.

##### Missing Tests
- HTTP/WebSocket provider request budget blocks before dispatch.
- Budget state shared across retries/transports.
- Active payload scan fails when projection artifact exists but raw TaskSpace history remains in provider payload.
- Start gate rejects stale/empty non-agent gate marker.
- Hard stop produces blocked final/abort when validation evidence is missing.

##### Missing Logs / Observability
- `request_phase` on every provider request.
- Same request id / trace id across provider event, rollout trace, runtime event and cost summary.
- Budget state before/after, response action, grace count and post-budget request/spawn counts.
- Active replacement proof tied to exact provider payload hash.

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/client.rs` - reviewer found provider budget types but no dispatch callers.
- `scripts/taskspace-benchmark/write-release-decision.ps1` - reviewer found projection summary still used as gate evidence.
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1` - reviewer found marker existence path too weak for full E3.

#### test-validity-adversary / Turing

##### Summary
文档已经识别“candidate evidence 被误判成 E3/release evidence”的历史错误，但当前脚本还没有强制执行该标准。友好 fixture 仍可产生 `release_pass`，E3 start gate 也可能在 v0.0.5 marker 缺失时让 suite 继续。

##### Blocking Findings
- Missing v0.0.5 markers do not block suite execution because marker gates can be `blocked` while suite runner only checks start gate `exit_code`.
- Suite runner does not expose or forward `V005NonAgentGatesPath`、`V005CodeCompleteMarkerPath`、`V005UserApprovalMarkerPath`.
- Marker spoofing is trivial: current marker check only verifies file exists.
- Release decision still accepts projection artifacts as active replacement proof.
- Release decision lacks new P0 gates for runtime budget response, state_commit displacement, spawn/node budget and request phase attribution.
- Invalid one-pair smoke can be treated as pass; it should route to harness repair, not formal E3.

##### Non-blocking Risks
- Sample identity is under-specified in release gating.
- `score_valid` is trusted from aggregate JSON rather than recomputed from pair-level artifacts.
- `blocked_partial` semantics exist but tests do not yet prove 3x partial vs 2x release thresholds.

##### Required Fixes
- Treat missing/invalid v0.0.5 markers as formal scoring/full E3 blockers, or make runner honor `gate_decision.full_e3_allowed=false` before scheduling samples.
- Add marker path parameters to suite runner and validate marker schema/hash/source/profile/freshness/user approval provenance.
- Replace projection-count release proof with exact payload scan/request hash matching.
- Add release blockers/artifacts for runtime budget response, state_commit displacement, spawn/node budget and request phase attribution.
- Make invalid harness smoke block formal E3 until clean smoke or explicit lower command category.

##### Missing Tests
- Complete calibration but missing v005 markers must not schedule samples.
- Spoofed arbitrary marker files must fail.
- Projection artifact present but provider payload contains legacy history must fail release.
- Hash-only active replacement must fail release.
- Exact scan request_id/hash mismatch must fail release.
- Duplicate or mismatched sample_set/source/profile metadata must fail.

##### Missing Logs / Observability
- Gate decision logs why `full_e3_allowed=false` and whether runner honored it.
- Release decision emits per-gate evidence paths.
- User approval marker records approval source, timestamp, command category, task list hash, source version and profile hash.

##### Evidence
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1` - reviewer found marker checks and failure aggregation too weak.
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1` - reviewer found marker paths not forwarded and runner only checks exit code.
- `scripts/taskspace-benchmark/write-release-decision.ps1` - reviewer found missing P0 blockers.

#### observability-adversary / Avicenna

##### Summary
方案文档比当前脚本/代码更强。当前证据仍不足以做 release-grade claim，因为 provider-visible payload proof、request-to-runtime attribution、active budget enforcement 和 release gate 仍未全部接通。

##### Blocking Findings
- Provider request budget code exists but is not wired into dispatch.
- Current budget event schema is too thin for the stated objective; it lacks task/map/node, phase, token, latency and payload proof fields.
- Release gate still allows report-only success paths.
- Provider-visible payload proof is specified but not available in current instrumentation.
- Runtime state lacks active budget contract/counters.

##### Non-blocking Risks
- Locally synthesized request ids may not join to provider response ids, rollout trace attempts or runtime events.
- Failed, blocked, retried, unauthorized and cancelled requests need terminal accounting events, not only completed responses.
- `unknown` request phase must be a thresholded failure.

##### Required Fixes
- Wire budget checks directly before HTTP/WebSocket provider network dispatch and emit terminal events for success/failure/retry/blocked/cancelled/fallback.
- Replace thin `ProviderRequestBudgetEvent` with schema close to `TaskSpaceProviderRequestEventV1`.
- Add provider request events, budget events, request phase summary, active replacement report and exact payload scan events to release-required artifacts.
- Make release fail on missing/partial/hash-only proof or attribution below threshold.
- Add runtime `TaskSpaceActiveBudgetV1` counters/state into `ActionMapRuntimeState`.

##### Missing Tests
- Mock provider request blocked before network dispatch.
- HTTP/WebSocket both emit start/end/blocked request events.
- Failed/retried/unauthorized requests emit terminal status and preserve counters.
- Active TaskSpace request event includes task_id/map_id/node_id/request_phase.
- Exact payload scan proves old TaskSpace history and large raw output absent before redaction/hash fallback.
- Release decision fails when provider request, budget, phase summary or exact payload artifacts are absent.

##### Missing Logs / Observability
- `request_phase` on every provider request.
- Join key from provider request event to runtime event and rollout trace attempt.
- Budget response action event and post-budget counts.
- Latency split where available.
- Context missing reason fields, not silent null/zero defaults.

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/client.rs` - reviewer found current event schema too thin and not wired.
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1` - reviewer found current diagnostics are post-run and heuristic, not exact payload proof.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` - reviewer found no active budget fields yet.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Gauss / Avicenna | Provider budget hook is currently orphaned and not wired to dispatch/runtime state | blocking | accept | Current worktree has in-progress `client.rs` code; it is not complete and must not be treated as implemented. | Keep implementation phase blocked until provider hook is wired before HTTP/WebSocket dispatch and bridged to ActionMap runtime. | Code implementation + post-code review required before E3. |
| Gauss / Avicenna | Budget event schema too thin for release-grade evidence | blocking | accept | v0.0.5 requires task/map/node/phase/token/payload evidence. | `18-unfinished-work-engineering-design.md` already defines `TaskSpaceProviderRequestEventV1`; accepted as implementation contract, not optional. | Implementation must expand current Rust event before review closure. |
| Gauss / Turing / Avicenna | Release gate can still accept report-only projection | blocking | accept | Reviewers found `write-release-decision.ps1` still relies on projection summary style gates. | Updated `18-unfinished-work-engineering-design.md` to list release-required artifacts and forbid projection summary as active replacement proof. | Script implementation must enforce these blockers. |
| Turing | E3 start gate markers are weak and runner does not honor `full_e3_allowed=false` | blocking | accept | Runner currently forwards no marker paths and checks exit code only. | Updated Phase 5 design to require runner marker params, structured JSON markers, schema/hash/source/profile validation, and pre-scheduling abort on `full_e3_allowed=false`. | Script implementation must add tests for spoofed/stale/mismatched markers. |
| Gauss | Final synthesis/recovery owner not pinned | blocking | accept | Hard-stop behavior is named but not sufficiently assigned to a runtime owner. | Keep as accepted blocker for implementation design: budget hard stop must map to explicit final/abort contract and no further non-recovery provider calls. | Add runtime owner and tests during Phase 1 implementation. |
| Turing | Invalid one-pair smoke can satisfy preflight | blocking | accept | Classified invalid harness should not unlock formal E3. | Phase 5 design now requires invalid smoke to route to harness repair/lower category, not formal E3. | Implement in start gate/suite runner tests. |
| Reviewers | Default budget thresholds need calibration | major | accept | The thresholds are engineering guardrails, not product proof. | Existing doc already states thresholds are not final product promises. | Keep calibration fixture before release. |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: partially at design-document level; code/script implementation still pending
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - Required after design/code/script fixes are implemented.
- Blocking re-review launch records:
  - pending
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: no for E3; yes for implementing accepted fixes

## Final Conclusion

The continuation plan is directionally correct but not yet closed. It may proceed into implementation of the accepted fixes, but it is not allowed to proceed to formal E3 or v0.0.5 closeout. Accepted blockers require code/script implementation, validation, and a fresh closure review.

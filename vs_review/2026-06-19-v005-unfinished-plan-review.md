# Subagent VS Review: v0.0.5 Unfinished Engineering Plan

- Created: 2026-06-19T13:11:58+08:00
- Updated: 2026-06-19T13:43:00+08:00
- Task: 对 v0.0.5 未完成项工程方案执行对抗性审查，确认方案是否足以继续开发并避免再次误判 E3 / 成本控制结果。
- Report path: `vs_review/2026-06-19-v005-unfinished-plan-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: passed

## Round 1: Plan Viability Review

### Review Input

#### Objective

审查 v0.0.5 未完成项工程方案是否能真实支撑产品目标：在 v0.0.5 内完成实际成本控制，而不是只做可观测；代码完成前禁止真实 E3；最终需要在 `terminal-bench_E3-P0` 口径验证正确率不下降且成本进入门槛。

#### Review Target

v0.0.5 未完成项工程设计、未完成项盘点、原实施计划、实验制度和相关门禁脚本。

#### Target Locations

- `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- `docs/v0.0.5/17-unfinished-work-inventory.md`
- `docs/v0.0.5/10-implementation-plan.md`
- `docs/experiments/`
- `third_party/codex-cli/codex-rs/core/src/client.rs`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1`
- `scripts/taskspace-benchmark/write-release-decision.ps1`
- `scripts/taskspace-benchmark/test-e3-start-gate.ps1`
- `scripts/taskspace-benchmark/test-release-decision.ps1`

#### Change Introduction

当前方案把 v0.0.5 从“收口候选”改回“继续开发”，要求先完成 provider request lifecycle hook、runtime budget hard stop、active provider-visible context replacement、state_commit displacement、fanout/node budget、E3 start gate 和 release decision gate，再允许真实 E3。方案还要求用 exact provider payload artifact 或 pre-redaction exact scan event 证明 active replacement，而不能用 projection summary 或 hash-only 证据替代。

#### Risk Focus

- 方案是否与 v0.0.5 的实际成本控制目标一致，而不是只加强可观测。
- provider/client/session/action_map/runtime/release gate 的职责边界是否能落地。
- active replacement 是否既有 proof，也有实际 provider-visible context composition 实现任务。
- E3 targeted diagnostic 与 formal E3 顺序是否会再次误导判断。
- marker、non-agent gates、release artifacts 是否能防止 forged/stale/shape-only 证据。
- 成本 hard stop 是否有正确率补偿和 recovery 设计。

#### Verification Status

- 本轮是方案审查，不运行真实 E3 / Agent benchmark。
- 已有相关报告：
  - `vs_review/2026-06-19-v005-continuation-plan-review.md`
  - `vs_review/2026-06-19-v005-code-gates-review.md`
- 当前已知代码门禁审查仍指出 provider lifecycle / payload proof 方向存在未闭合 blocker。

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| `product-logic-adversary` | v0.0.5 的核心争议是目标是否包含实际成本控制以及是否能防止再次误判。 | 产品目标、验收口径、正确率补偿 |
| `architecture-adversary` | 方案跨 provider client、session、ActionMap runtime、脚本门禁和 artifacts，存在职责错配风险。 | 架构边界、生命周期、状态归因 |
| `test-validity-adversary` | 此版本已经发生过内部 fixture / Terminal-Bench 口径混淆，需要专门挑战验证制度。 | 测试有效性、实验命名、marker freshness |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| `product-logic-adversary` | `multi_agent_v1.spawn_agent` explorer | `019ede4a-5458-7c61-a108-ba6f43394c6c` / Newton | spawn_agent result | no | Round 1 Review Input with product-logic focus | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| `architecture-adversary` | `multi_agent_v1.spawn_agent` explorer | `019ede4a-9099-76e2-b7e6-28c6e043d3c6` / Beauvoir | spawn_agent result | no | Round 1 Review Input with architecture focus | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| `test-validity-adversary` | `multi_agent_v1.spawn_agent` explorer | `019ede4a-cffc-7961-ad59-806d896a8284` / Epicurus | spawn_agent result | no | Round 1 Review Input with test-validity focus | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### product-logic-adversary / Newton

##### Summary

方案总体方向正确：它承认 v0.0.5 不能收口，成本问题不是缺指标而是缺 runtime 执行控制，并且把正式 E3 放在代码完成、非 agent gate、targeted diagnostic 之后。主要阻塞是 Phase 6 正式验证样本口径仍可能跑偏，低成本诊断命名仍可能误导，以及成本收缩后的正确率补偿策略不足。

##### Blocking Findings

- Phase 6 仍可能跑错正式验证口径，不能证明用户指定的 P0 目标。`10-implementation-plan.md` Phase 6 使用 `analyze-access-logs`、`log-summary`、`count-call-stack`，这对应 `terminal-bench_E3-v004-clean_3_5`，不是当前目标 `terminal-bench_E3-P0_3_5`。
- `terminal-bench_E3-P0_1_1` targeted diagnostic 命名仍有误导风险，必须显式登记为 diagnostic-only 或 E3-candidate。
- 成本硬停后的质量补偿策略不足，缺少 bounded escalation、second-pass verification、manual blocked classification、sample-level retry budget 等产品层补偿验收。

##### Non-blocking Risks

- `10-implementation-plan.md`、`17-unfinished-work-inventory.md`、`18-unfinished-work-engineering-design.md` 并存时，读者可能不知道当前优先级。
- 旧 plan 中 report-only routing 文字容易和当前 active cost control 主线冲突。
- P0 样本数、pair 数和 diagnostic 变体粒度容易继续混淆。

##### Required Fixes

- 明确 `18-unfinished-work-engineering-design.md` supersedes `10-implementation-plan.md` 的 Phase 6。
- 把正式 P0 release proof 和 v0.0.4 clean comparison 拆开。
- 将 `_1_1/_3_1/_3_2` 诊断显式登记为 diagnostic-only / E3-candidate。
- 增加预算收缩后的质量补偿 gate。

##### Missing Tests

- Phase 6 sample-set guard。
- Diagnostic naming test。
- Quality-under-budget tests。

##### Missing Logs / Observability

- `budget-induced quality impact` summary。
- Formal E3 start-gate decision fields: requested/actual sample set id、reported evidence level、approval marker、code-complete marker、non-agent gate artifact hash。

##### Evidence

- `docs/v0.0.5/10-implementation-plan.md:477-489`
- `docs/v0.0.5/17-unfinished-work-inventory.md:421-445`
- `docs/experiments/taskspace-evidence-levels-and-samples.md:84-96`

#### architecture-adversary / Beauvoir

##### Summary

工程方向正确，但还不是干净的 architecture contract。provider lifecycle 和 active context replacement 仍部分像 proof/reporting system；真正执行边界是 `session/turn.rs` history composition 和 `client.rs` provider dispatch。若不让这些边界成为结构化 lifecycle 和 replacement artifacts 的 producer，release/E3 gate 会变成与真实 model request 因果关系不足的脚本校验。

##### Blocking Findings

- Provider lifecycle contract 比当前 hook 更宽，但没有明确 terminal usage/latency/payload proof 的 owner。
- ActionMap 当前通过 provider request 之外的 snapshot 归因，存在 node/phase 误归因风险。
- Active context replacement 没有足够钉在实际 context composition 点上，容易只做 payload scan/report 而不改变 `clone_history().for_prompt(...)` 输入。
- Release/E3 gates 仍依赖 marker 和 summary artifacts，而不是 producer-side schema guarantees。
- Phase ordering 在 provider lifecycle 和 runtime budget state 上有循环依赖：到底 provider lifecycle 拥有 budget state，还是 ActionMap 拥有并传 context，需要先明确。

##### Non-blocking Risks

- `provider-request-{n}` 不是 release-grade global request id。
- WebSocket warmup bypass 必须明确 excluded 或单独 tracking。
- request phase taxonomy 丰富但当前实现可能仍硬编码为 `model_sampling`。

##### Required Fixes

- 定义 canonical provider lifecycle producer 在 `client.rs` / `ModelClientSession`。
- ActionMap 作为 consumer/annotator，只在 request construction 前提供 context，不事后推断。
- 增加显式 request context object。
- 把 active replacement 放到 `session/turn.rs` composition boundary。
- producer-side schema 先行，脚本只做 gate aggregation。

##### Missing Tests

- lifecycle start/terminal exactly-once。
- token usage joins terminal lifecycle event。
- retry/fallback attempt id。
- phase attribution fixture。
- active composition fixture。
- misattribution test。

##### Missing Logs / Observability

- terminal provider lifecycle event with latency/token usage。
- payload hash/artifact/scan tied to dispatch path。
- explicit dispatch failure / stream open / stream completion / provider error / cancellation status。

##### Evidence

- `third_party/codex-cli/codex-rs/core/src/client.rs`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `scripts/taskspace-benchmark/write-release-decision.ps1`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1`

#### test-validity-adversary / Epicurus

##### Summary

验证制度方向变严，但仍不足以防止误判。主要问题集中在 `write-release-decision.ps1`：形状正确的合成 JSON、单 repeat、自称 E3 event 仍可能推到 `release_pass`，没有强制 formal E3 的 sample set、repeat、runner、start-gate identity 和真实 evidence path 绑定。

##### Blocking Findings

- `write-release-decision.ps1` 没有强制 `repeats >= 5` 或样本集口径，单 pair fixture 可通过 `release_pass`。
- release decision 可把 internal/synthetic fixture 当 formal E3，因为只信 `evidence_target=E3` 和 `reported_evidence_level=E3`，不验证 `sample_set_id`、`benchmark_family`、`runner_entrypoint`、`runner_profile_hash`。
- `v005-non-agent-gates.json` 仍是 shape gate，测试 fixture 使用 `selftest://...` 而不是可验证 artifact。
- release decision 没有绑定 start gate decision artifact。
- user approval marker 没有校验 approved sample set。

##### Non-blocking Risks

- `terminal-bench_E3-P0_3_5` 文档定义清楚，但 diagnostic 变体未登记为 diagnostic-only。
- start gate 有 freshness，release decision 对 non-agent gates 没有同等 freshness/commit/profile/task-list hash 校验。

##### Required Fixes

- release decision 增加 formal E3 identity gate。
- required artifacts 增加 start-gate outputs。
- non-agent gate evidence_path 必须是存在的本地 artifact，并记录 command、exit_code、generated_at、git_commit、profile_hash。
- user approval 校验 `approved_sample_set_id`。
- release PASS fixture 改为 3 samples x 5 repeats 的 formal shape，单 pair fixture 只能测 blocked/diagnostic。

##### Missing Tests

- `repeats=1` 自称 E3 必须 fail。
- forged internal runner family 必须 fail。
- non-existent / `selftest://` evidence path 必须 fail。
- 缺 start-gate decision artifact 必须 fail。
- approval sample set mismatch 必须 fail。
- `_3_1/_3_2` 诊断变体进入 release decision 必须 fail 或降级。

##### Missing Logs / Observability

- release decision 输出 sample_set_id、repeats_per_sample、runner_family、runner_profile_hash、start_gate_decision_path。
- non-agent gate summary 输出 evidence hash、mtime/freshness、command exit code。

##### Evidence

- `scripts/taskspace-benchmark/test-release-decision.ps1`
- `scripts/taskspace-benchmark/write-release-decision.ps1`
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1`
- `docs/experiments/taskspace-evidence-levels-and-samples.md`

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| product-logic | Phase 6 may use v004 clean samples instead of P0 formal proof | blocking | accept | Old plan retained v004 clean samples; current target is P0 | Added supersession note and Phase 6 evidence tracks in `18`; added supersession note in `10` | Implement sample-set guard in scripts |
| product-logic | `_1_1/_3_1/_3_2` diagnostic naming may mislead | blocking | accept | Diagnostic variants could be reported as E3 | Added diagnostic-only rule to `18`; registered diagnostic-only variants in experiments doc | Add release/start gate tests |
| product-logic | Budget hard stop lacks quality compensation | blocking | accept | Cost gates alone can hide solve loss | Added `BudgetQualityImpactV1`, bounded recovery and validation-skip rules | Implement runtime event and release summary |
| architecture | Provider lifecycle canonical producer unclear | blocking | accept | Current budget event is narrower than required lifecycle | Added `6.1.1 Canonical Provider Lifecycle Producer` and code landing table | Implement client/session lifecycle producer |
| architecture | ActionMap snapshot can misattribute request | blocking | accept | Request phase/node inferred after request lifecycle | Defined ActionMap as context producer/consumer, not lifecycle producer | Refactor runtime bridge |
| architecture | Active replacement not grounded at composition boundary | blocking | accept | Proof can exist without changing prompt input | Added active replacement implementation contract at `session/turn.rs` composition boundary | Implement `build_active_provider_visible_history` |
| architecture | Gates rely on marker artifacts, not producer schemas | blocking | accept | Script layer cannot be source of truth | Added producer-owned structured gates and formal E3 identity gate | Harden scripts and fixtures |
| architecture | Phase ordering circular around lifecycle/budget | blocking | accept | Partial budget exists, full lifecycle still missing | Clarified lifecycle producer owns request lifecycle; ActionMap supplies context/budget policy | Implement in that dependency order |
| test-validity | Release accepts single-repeat synthetic E3 | blocking | accept | Existing fixture uses repeats=1 | Added formal E3 identity gate requiring `repeats_per_sample >= 5` and P0 sample set | Harden `write-release-decision.ps1` |
| test-validity | Release does not validate runner/sample identity | blocking | accept | Current artifacts can self-label E3 | Added sample_set/benchmark/runner/profile/start-gate identity requirements | Add negative fixtures |
| test-validity | non-agent gates are shape-only | blocking | accept | `selftest://` path could pass | Added local evidence path, command, exit_code, git_commit, evidence hash requirements | Implement path/hash checks |
| test-validity | release not bound to start gate decision | blocking | accept | Release artifacts did not require start-gate outputs | Added start-gate required artifacts and identity matching | Implement required artifact check |
| test-validity | user approval lacks sample-set binding | blocking | accept | Approval category alone is too broad | Added `approved_sample_set_id=terminal-bench_E3-P0_3_5` requirement | Add start gate parameter/test |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes, in plan documents
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - Required before treating plan as closed, because all blocking findings were accepted and docs were changed.
- Blocking re-review launch records:
  - pending closure round
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: no, closure re-review required

## Final Conclusion

Round 1 found accepted blocking issues. The plan was updated, and Round 2 closure review passed. 方案审查层面可以进入开发执行；代码层面仍必须完成 producer lifecycle、active replacement、release identity gate、non-agent tests 后，才能运行真实 E3。

## Round 2: Closure Review

### Review Input

#### Objective

检查 Round 1 中已接受的 blocking findings 是否已经在方案文档中被充分修正。

#### Review Target

文档修正后的 closure review。

#### Target Locations

- `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- `docs/v0.0.5/10-implementation-plan.md`
- `docs/experiments/taskspace-evidence-levels-and-samples.md`
- `vs_review/2026-06-19-v005-unfinished-plan-review.md`

#### Risk Focus

- `18` 是否明确 supersedes `10` 的 Phase 6 样本安排。
- formal P0 proof 是否钉死为 `terminal-bench_E3-P0_3_5`。
- diagnostic variants 是否不能 release_pass。
- provider lifecycle、active replacement、release formal identity gate 和 budget quality compensation 是否闭合。

#### Verification Status

- 文档静态检查已确认关键条款可检索。
- 未运行真实 E3。

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| documentation-skill-adversary + architecture/test-validity closure focus | Round 1 blocker 都是方案可执行性、文档优先级、架构边界和验证制度问题。 | closure review |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| documentation-skill-adversary + architecture/test-validity closure focus | `multi_agent_v1.spawn_agent` explorer | `019ede51-3b7e-71e0-9821-6187b5c9642d` / Descartes | spawn_agent result | no | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### Descartes

##### Summary

Closure review scope only. The accepted Round 1 blocking findings are sufficiently closed at the plan-document level. `18-unfinished-work-engineering-design.md` now clearly supersedes old Phase 6 guidance in `10`, separates diagnostic/P0/v004 tracks, pins formal P0 proof to `terminal-bench_E3-P0_3_5`, moves provider lifecycle ownership to client/session, places active replacement at `session/turn.rs` composition, and adds formal identity plus quality-compensation gates.

##### Blocking Findings

- None found for closure scope.

##### Non-blocking Risks

- `docs/experiments/taskspace-evidence-levels-and-samples.md` still said the next same-scope correctness check is `terminal-bench_E3-v004-clean_3_5`; this could mislead readers unless it says the comparison cannot replace P0 release proof.
- Several dense lines in `18` pack multiple must-rules into one physical line. Semantics are present, but reviewability is weak.

##### Required Fixes

- None required before treating Round 1 documentation blockers as closed.

##### Missing Tests

- Tests are specified but not implemented in this closure scope: release/start gate negative fixtures, provider lifecycle fixtures, payload proof fixtures, active replacement fixtures.

##### Missing Logs / Observability

- Specified, not yet implemented: `BudgetQualityImpactV1`, provider lifecycle token/latency/payload scan binding, release identity outputs and non-agent evidence constraints.

##### Evidence

- Supersession closed: `docs/v0.0.5/18-unfinished-work-engineering-design.md`, `docs/v0.0.5/10-implementation-plan.md`
- Formal P0 proof pinned: `docs/v0.0.5/18-unfinished-work-engineering-design.md`, `docs/experiments/taskspace-evidence-levels-and-samples.md`
- Diagnostic-only variants blocked from release pass: `docs/experiments/taskspace-evidence-levels-and-samples.md`, `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- Provider lifecycle producer fixed to client/session and ActionMap demoted to context/consumer: `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- Active replacement placed at `session/turn.rs` provider-visible composition boundary: `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- Release formal identity gate includes sample set, repeats, runner, start-gate decision, approval sample set, and local evidence constraints: `docs/v0.0.5/18-unfinished-work-engineering-design.md`

##### Verdict

pass. Round 1 accepted blocking findings are closed in the plan documents.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Descartes | No closure-scope blockers | n/a | accept | Closure review verdict pass | Mark report passed | Proceed to implementation only after user confirms; true E3 remains blocked until code/tests complete |
| Descartes | v004 clean comparison wording could still mislead | minor | accept | Non-blocking but cheap to clarify | Added sentence to `docs/experiments/taskspace-evidence-levels-and-samples.md` that v004 clean comparison cannot replace P0 release proof | none |
| Descartes | Dense lines reduce reviewability | minor | defer | Semantics are present; broad formatting pass is not needed for closure | Track as documentation polish, not implementation blocker | optional later cleanup |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2
- Blocking re-review launch records:
  - Descartes / `019ede51-3b7e-71e0-9821-6187b5c9642d`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Allowed to proceed: yes, for implementation planning/code work only; not for true E3 before code and non-agent gates complete.

# Subagent VS Review: v0.0.5 Unfinished Engineering Plan

- Created: 2026-06-19T13:11:58+08:00
- Updated: 2026-06-19T13:52:26+08:00
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

## Round 3: Fresh Executability Review After Design Refinement

### Review Input

#### Objective

对当前完善后的 v0.0.5 未完成项工程方案执行 fresh 对抗性审查，确认方案是否足够可执行，是否仍会导致后续 agent 误跑/误报 E3，是否能防止“代码未完成就验证收口”，以及是否能支撑实际成本控制而不只做观测。

#### Review Target

v0.0.5 未完成项工程方案、实验命名与证据等级制度、旧方案 supersession 关系、formal E3 / diagnostic-only 边界、non-agent gates / code-complete / user approval / release gate 设计。

#### Target Locations

- `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- `docs/v0.0.5/17-unfinished-work-inventory.md`
- `docs/v0.0.5/10-implementation-plan.md`
- `docs/v0.0.5/README.md`
- `docs/experiments/taskspace-evidence-levels-and-samples.md`
- `vs_review/2026-06-19-v005-unfinished-plan-review.md`

#### Change Introduction

当前方案已经把 v0.0.5 从历史收口状态改回继续开发状态，并要求先完成 provider lifecycle、active context replacement、budget hard stop、state_commit displacement、spawn/node budget、formal E3 start/release gate 等工程代码和非 agent 证据，再允许真实 E3。此轮审查不验证代码实现，只攻击方案是否仍存在执行歧义、验收伪造路径或实验结论误导风险。

#### Risk Focus

- 文档优先级和执行入口是否足够清楚，fresh agent 是否会跟错旧计划。
- Phase 0A-5 未完成项是否每项都有可执行代码落点、测试、artifact 和 release gate。
- formal E3 与 diagnostic-only 的边界是否仍可能被混淆。
- non-agent gates / code-complete / user approval / sample identity 是否还有伪造或过期复用风险。
- provider lifecycle、active context replacement、budget hard stop、state_commit displacement、spawn/node budget 的职责边界是否可执行。
- 成本控制是否会牺牲正确率且缺乏质量补偿。

#### User-Perspective Review Focus

- 后续 fresh agent 能否从文档中明确知道先做代码、再做非 agent gate、最后才可请求真实 E3。
- 用户能否从方案和实验制度中区分 diagnostic、E3-candidate、formal E3 和 release proof。
- 若 run 被 gate 阻止，报告是否能清楚说明缺什么、下一步只能做什么。

#### Assumptions To Attack

- 只要文档写了 supersession，后续执行者就不会读错旧计划。
- marker/hash/profile/source 字段足够防止伪造或过期复用。
- payload hash / scan / report 能证明 active replacement，而不会变成另一套 report-only 证据。
- hard stop 能降低成本，同时不会把未验证样本错误计为 solved。
- `_3_1/_3_2` 这类低成本变体不会被再次误称为 E3。

#### Adversarial Lenses

- documentation
- comprehension
- test validity
- observability
- release operations
- maintenance

#### Verification Status

- 本轮是方案对抗性审查，不运行真实 E3 / agent benchmark。
- 相关 release/start gate 代码已有部分实现和测试，但当前审查范围主要是方案是否可执行和不误导。
- 代码实现仍未全部完成，真实 E3 仍应保持禁止状态。

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 12 minutes | bounded extension if alive | 2 | accepted blocking findings require a fresh closure review |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| `documentation-skill-adversary` | 本轮最高风险是方案文档和实验制度能否被 fresh agent 正确执行，避免再次把诊断结果误报为 E3 或在代码未完成前进入验证收口。 | 文档优先级、fresh-session 可执行性、实验命名、证据等级、验收路径 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| `documentation-skill-adversary` | `multi_agent_v1.spawn_agent` explorer | `019ede68-6fc3-73c3-9984-07c9067bdf0e` / Pauli | spawn_agent result | no | Round 3 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| Pauli | `documentation-skill-adversary` | 1 | `019ede68-6fc3-73c3-9984-07c9067bdf0e` | ~12 minutes | completed | reviewer returned findings | completed |

### Reviewer Outputs

#### Pauli

##### Summary

当前方案比第一轮更可执行：`18-unfinished-work-engineering-design.md` 已把 v0.0.5 定义为继续开发，把 formal P0 proof 固定到 `terminal-bench_E3-P0_3_5`，并把 provider lifecycle、active replacement、non-agent gates、code-complete/user approval marker、diagnostic-only 边界写成工程合约。但还不能说后续 agent 不会误跑/误报。主要 blocker 是入口文档仍有旧入口冲突，以及质量补偿 / hard stop 的产品语义仍留为开放问题。

##### Blocking Findings

- README 入口仍可能把 fresh agent 带回旧计划链路。
  - Broken assumption: 只要 `18` 写了 supersession，fresh agent 就会自然跟 `18`。
  - Failure scenario: agent 从 README 进入，看到当前状态入口是 `17`，但修正说明仍说 2026-06-17 后工程执行以 `13`、`10`、acceptance checklist 为准；跑 E3 前读的是 `08/09/checklist`，没有强制先读 `18`。
  - Trigger condition: fresh agent 只读 README 的修正说明 / 跑 E3 前读段落，或按 `10-implementation-plan.md` 执行历史 Phase 6。
  - Impact: 仍可能跟错旧 plan、复用旧 Phase 6 或把旧 release gate 当当前入口，导致 premature E3 或错误 closeout。
  - Proof needed: README 必须把 `18` 明确列为当前 canonical execution entry，并声明 `18` supersedes `10` 的 Phase 6、release taxonomy、report-only routing。

- Budget hard stop 的质量语义仍未闭合，可能用省钱掩盖未完成。
  - Broken assumption: hard stop + recovery whitelist 足以同时降成本和保持正确率。
  - Failure scenario: runtime 为满足 2x 成本触发 thin/no-spawn/hard stop，跳过 validation 或提前 final synthesis；release 报告只看到成本改善和没有继续扩张，但实际样本未被充分验证。
  - Trigger condition: budget threshold 被触发且任务仍需验证/修复；`final_synthesis` 走 partial failure 或 early final 路径。
  - Impact: 可能把少做了工作报告成成本控制成功，牺牲正确率，尤其在 P0 样本上污染 formal E3 结论。
  - Proof needed: 明确 `BudgetQualityImpactV1` 的字段、producer、release blocker 规则；定义 validation skip、early final、final abort、manual blocked、bounded escalation 后的计分语义。

##### Non-blocking Risks

- `18` 自身状态还是 Draft，可能让后续 agent 误判为不能实现或忽略审查状态。
- Phase 0A-5 的落点足够具体，但仍多为待实现合约；真实 E3 禁止状态应保持。
- diagnostic-only 边界文档已清楚，但命名仍带 E3，必须靠 gate 执行。

##### User-Perspective Checks

- 可用性：工程师能从 `18` 找到阶段、代码落点、测试和 gate，但从 README 进入仍有歧义。
- 易用性：Phase 0A-5 每阶段都有 entry criteria、tasks、deliverables、tests、gate；缺少一张只按此顺序执行的短 checklist。
- 易理解性：E3 taxonomy 已明显改善，但 README 的旧入口会削弱清晰度。

##### Required Fixes

- 更新 README：当前 canonical execution entry 应为 `18`，并明确 `18` supersedes `10` 的 Phase 6/release taxonomy/report-only routing。
- 把 `18` 的 Draft 状态改成审查后可执行状态，或明确可开始 Phase 0A 实现但 formal E3 仍禁止。
- 补全 `BudgetQualityImpactV1` contract：字段、producer、artifact path、release blocker/warning 规则、quality compensation 计分语义。
- 明确 hard stop 产品行为：用户可见 blocked、partial final、manual approval override 各自何时允许，以及是否可纳入 solved。
- 把 README 的跑 E3 前读列表加入 `18` 和 experiments evidence taxonomy。

##### Missing Tests

- README/canonical-entry lint。
- Formal E3 start gate negative fixtures。
- Diagnostic result cannot release-pass fixture。
- Budget quality impact fixtures。
- Active replacement negative fixture。
- Provider lifecycle fixture。

##### Missing Logs / Observability

- `BudgetQualityImpactV1` event and per-sample summary。
- Provider lifecycle terminal event with status、latency、token usage、payload hash/scan id、request id。
- Start-gate decision artifact with requested/actual sample set、approval sample set、profile/source/task hash、next allowed command category。
- Hard-stop reason chain。
- Active replacement exact scan provenance。

##### Evidence

- `docs/v0.0.5/README.md:6-13`
- `docs/v0.0.5/README.md:49-55`
- `docs/v0.0.5/README.md:80-84`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:13-16`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:63-78`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:666`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:704`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:1203`
- `docs/experiments/taskspace-evidence-levels-and-samples.md:84-104`
- `vs_review/2026-06-19-v005-unfinished-plan-review.md:336-338`

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| Pauli | README 入口仍可能把 fresh agent 带回旧计划链路 | README 仍把 `13/10/checklist` 表述为执行依据，跑 E3 前读列表缺 `18` | blocking | accept | Reviewer evidence valid | Rewrote `docs/v0.0.5/README.md` in Chinese; made `18` the canonical execution entry; stated `18` supersedes old Phase 6/release taxonomy/report-only routing; added formal E3 prohibition and reading order | Closure review required |
| Pauli | Budget hard stop 的质量语义仍未闭合 | hard stop 可能少做验证却被报告为成本成功 | blocking | accept | Reviewer evidence valid | Updated `docs/v0.0.5/18-unfinished-work-engineering-design.md`: status now implementation-approved with E3 still forbidden; added clarified `BudgetQualityImpactV1` producer/artifact/fields/release blocker rules; resolved hard-stop open question into scoring rule | Closure review required |
| Pauli | `18` 状态还是 Draft | 后续 agent 可能误判执行授权 | non-blocking | accept | Status line was stale | Changed status to Phase 0A-5 implementation approved after adversarial review, formal E3 forbidden until gates pass | Closure review covers |
| Pauli | Phase 0A-5 仍多为待实现合约 | 文档可执行不等于代码完成 | non-blocking | accept | This is intentionally true | Kept true E3 forbidden; README and `18` now explicitly restrict work to code/non-agent gates until code-complete | Track in implementation |
| Pauli | diagnostic-only 命名仍带 E3 | Manual summary may mislabel diagnostic | non-blocking | accept | Needs gate enforcement | README now repeats diagnostic-only prohibition; existing release/start gate negative tests remain implementation requirement | Track in implementation |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes, in plan documents
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - Round 4 required
- Blocking re-review launch records:
  - pending
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: no, Round 4 closure review required before treating plan review as closed.

## Round 4: Closure Review For Round 3 Fixes

### Review Input

#### Objective

只验证 Round 3 的两个 accepted blocking findings 是否已在文档中闭合：

1. README 入口可能把 fresh agent 带回旧计划链路。
2. Budget hard stop / `BudgetQualityImpactV1` 质量语义未闭合，可能用省钱掩盖未完成。

#### Review Target

Round 3 修复后的文档闭环，不重新扩展全量方案审查。

#### Target Locations

- `docs/v0.0.5/README.md`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- `docs/experiments/taskspace-evidence-levels-and-samples.md`
- `vs_review/2026-06-19-v005-unfinished-plan-review.md`

#### Risk Focus

- README 是否明确 `18-unfinished-work-engineering-design.md` 是当前 canonical execution entry，并说明它 supersedes `10` 的 Phase 6/release taxonomy/report-only routing。
- README 的 E3 前置阅读和禁止误用是否能防止 premature E3 / diagnostic-only 误报。
- `18` 的状态是否清楚允许 Phase 0A-5 实现但仍禁止 formal E3。
- `BudgetQualityImpactV1` 是否有 producer、artifact path、字段、release blocker、hard stop 计分语义。
- hard stop 用户可见 blocked / partial final / manual override 是否不会被算作 clean solved。

#### Verification Status

- 已修复 `docs/v0.0.5/README.md` 和 `docs/v0.0.5/18-unfinished-work-engineering-design.md`。
- 未运行真实 E3。

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| normal | 8 minutes | bounded extension if alive | 2 | cannot pass if review unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| `documentation-skill-adversary` closure reviewer | Round 3 blockers are documentation executability and evidence-contract clarity issues. | closure validity |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| `documentation-skill-adversary` closure reviewer | `multi_agent_v1.spawn_agent` explorer | `019ede6e-1d84-78b0-959a-12328b170cc4` / Ramanujan | spawn_agent result | no | Round 4 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| Ramanujan | `documentation-skill-adversary` closure reviewer | 1 | `019ede6e-1d84-78b0-959a-12328b170cc4` | ~8 minutes | completed | closure reviewer returned pass | completed |

### Reviewer Outputs

#### Ramanujan

##### Summary

Read-only closure review completed for the two Round 3 accepted blocking findings. Both are closed at the documentation/plan level. The reviewer did not modify files.

##### Closure Verdict

pass

##### Blocking Findings Remaining

none

##### Non-blocking Risks

- The contracts are still documentation-level until implementation and gate fixtures exist. This does not block closure scope because Round 3 asked whether the docs now close the two accepted findings.
- Diagnostic variants still contain `E3` in their names, so enforcement must come from the start/release gates when implemented.

##### Evidence

- README now makes `18-unfinished-work-engineering-design.md` the canonical execution entry and supersedes old paths: `docs/v0.0.5/README.md:16-25`.
- README now prevents premature E3 / diagnostic-only misuse: `docs/v0.0.5/README.md:27-32`, `docs/v0.0.5/README.md:82-92`.
- `18` clearly allows Phase 0A-5 implementation while forbidding formal E3: `docs/v0.0.5/18-unfinished-work-engineering-design.md:6`, `docs/v0.0.5/18-unfinished-work-engineering-design.md:13-21`.
- Diagnostic-only taxonomy is reinforced: `docs/experiments/taskspace-evidence-levels-and-samples.md:108-126`, `docs/experiments/taskspace-evidence-levels-and-samples.md:262-264`.
- `BudgetQualityImpactV1` now has producer, artifact paths, fields, release blockers, and hard-stop scoring semantics: `docs/v0.0.5/18-unfinished-work-engineering-design.md:318-325`, `docs/v0.0.5/18-unfinished-work-engineering-design.md:327-346`, `docs/v0.0.5/18-unfinished-work-engineering-design.md:360-376`, `docs/v0.0.5/18-unfinished-work-engineering-design.md:278-312`, `docs/v0.0.5/18-unfinished-work-engineering-design.md:1288`.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| Ramanujan | Round 3 blocking findings closed | n/a | n/a | accept | Closure verdict pass | Mark Round 3 closure passed | Proceed with implementation; true E3 remains forbidden until code and gates complete |
| Ramanujan | Contracts still documentation-level until implementation | Implementation not complete | non-blocking | accept | This is expected current state | Keep true E3 forbidden; continue code implementation next | Track in implementation plan |
| Ramanujan | Diagnostic names still include E3 | Manual reporting can still mislabel | non-blocking | accept | Naming convention intentionally includes evidence/subset field | Enforce through start/release gates and report fields | Track in gate implementation |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 4
- Blocking re-review launch records:
  - Ramanujan / `019ede6e-1d84-78b0-959a-12328b170cc4`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: yes, for implementation and non-agent verification only; not for true E3 before code-complete, non-agent gates, user approval, and start gate all pass.

# Subagent VS Review: R4 runtime boundary hard-stop audit

- Created: 2026-07-07 22:44:47 +0800
- Updated: 2026-07-08 00:11:49 +0800
- Report schema: adversarial-v1
- Task: 审计 R4 TaskSpace runtime hard-stop / recovery 逻辑是否仍越过“只守硬基线、不替 Agent 做语义策略决策”的边界。
- Report path: `vs_review/2026-07-07-r4-runtime-boundary-hard-stop-audit.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context; reviewer received only the review navigation packet
- Status: passed

## Round 1: hard-stop boundary audit

### Review Input

#### Objective
确认 R4 runtime 边界问题是否已收干净，并枚举仍存在的 hard-stop / recovery 越界残留。

#### Review Target
代码实现、测试策略、反馈层 recovery 文案和 TaskSpace runtime 边界。

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `coe/2026-07-03-05-03-r4-durable-evidence-gates.md`

#### Change Introduction
R4 已完成 H171-H187 多轮边界修复。本轮审计不改代码，只验证剩余 hard-stop 是否属于硬基线，或仍在替 Agent 选择策略。

#### Risk Focus
- runtime 因“Agent 应该 patch / validate / block”而阻断合法工具动作。
- recovery 文案注入 closed action space 或 next-action prompt。
- dead marker / old tests 让已禁用 hard-stop 被未来改动误复活。
- block_node/final readiness 校验从 ledger 底线扩大到语义策略判断。

#### User-Perspective Review Focus
- Agent 收到的反馈是否忠实、清晰、不过度指挥。
- 错误是否归因到工具/上下文/Agent，而不是被 runtime 伪装成策略约束。

#### Implementation Completeness Focus
- 每个 `TaskSpace*HardStopV1` 是否有可达触发路径。
- 每个拒绝路径是否只对应节点动作矩阵、工具合同、总预算、交付完整性等硬基线。
- 测试是否覆盖“合法动作不得因语义策略被拒绝”的性质。

#### Target Benefit Focus
- 目标收益：降低 runtime 语义控制、提高反馈层语义透传。
- Baseline：H171-H187 前存在 forced transition、path correction gate、implementation-needs-edit hard stop、per-node budget hard stop。
- Measurement：源码静态审计、聚焦单测、目标 sample 运行记录、独立复核。

#### Assumptions To Attack
- “重复失败后 hard-stop”一定是成本硬基线。
- “已有证据足够，所以不能再读/不能 block”是状态机底线。
- recovery 文案里写“必须/不得”只是提示，不会污染 Agent 上下文。

#### Adversarial Lenses
- architecture
- state
- failure
- implementation-completeness
- testing
- observability

#### Verification Status
- 本地已完成源码 hard-stop 枚举。
- 独立 subagent 已完成只读复核。
- 聚焦测试 `validation_rework_patch_only_schema_repair_gets_one_extra_recovery_before_hard_stop` 失败，证明存在陈旧测试/旧语义残留。
- 本轮未执行代码修复。

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.

### Internal Subagent Unavailable Fallback

- Internal subagent unavailable reason: n/a
- Local CLI discovery commands: n/a
- Discovered CLI candidates: n/a
- User-recommended alternative agent requested: n/a
- User approval requested: n/a
- Fallback outcome: n/a

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 15 min | none | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| architecture-adversary | 最高风险是 runtime/TaskSpace 边界和长期维护方向，而不是单个语法 bug。 | module boundary, state-machine responsibility, semantic-control overreach |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1.spawn_agent` | `019f3d4d-bddc-70b2-bbca-29af813465a6` | spawn_agent tool result and subagent notification | `fork_context=false` | Round 2 Review Input | main-agent history, hidden reasoning, drafts, conclusions, full diff | yes |
| architecture-adversary | `multi_agent_v1.spawn_agent` | `019f3d07-c0d9-71f0-8a06-2c3c0e445a2a` | subagent notification | `fork_context=false` | Round 1 Review Input plus explicit target files | main-agent history, hidden reasoning, drafts, conclusions, full diff | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| reviewer-1 | architecture-adversary | 1 | `019f3d07-c0d9-71f0-8a06-2c3c0e445a2a` | <15 min | completed | returned findings via subagent notification | completed |

### User Decision After Failed Review

- Decision: n/a
- User-visible reason: review completed.

### Reviewer Outputs

#### reviewer-1

##### Summary
复核认为 R4 主线已清掉 per-node budget、path correction、implementation-needs-edit 等已知 blocker，但 hard-stop / recovery 体系仍有两类阻塞残留：duplicate validation rework read 和 apply_patch recovery 仍在注入策略或累计失败后终止；另有 dead marker / dead hard-stop 代码和测试残留会污染后续维护。

##### Blocking Findings
- Duplicate-read recovery 仍在注入语义策略，越过“语义透传”边界。
  - Broken assumption: exact duplicate complete target read 的硬基线可以扩展为“现在必须 patch，不得再 read/search/list/validate”。
  - Failure scenario: Agent 想执行状态机合法的其他 read/search/list 或 block，但上下文被 recovery 文案强制引导为 exactly one apply_patch。
  - Trigger condition: validation rework target read 已存在且 runtime 识别重复读或 implementation recovery。
  - Impact: runtime 替 Agent 决定修复策略，可能掩盖真实上下文缺失或工具反馈问题。
  - Proof needed: recovery 文案改为事实/合同透传；性质测试证明除 exact duplicate complete target read 外其他合法动作不被拒绝。
- ApplyPatchRecoveryHardStop 把工具失败恢复次数升级为语义停机，且 recovery 文案禁止合法后续动作。
  - Broken assumption: apply_patch 失败多次后，runtime 可以终止 provider sampling 或禁止 read/search/list/test/control。
  - Failure scenario: Agent 需要重新读取上下文或选择 blocked，但 recovery/hard-stop 要求继续 patch 或停止。
  - Trigger condition: apply_patch/edit failure recovery count 达到 2 或 3。
  - Impact: 工具合同错误从“单次拒绝并透传错误”扩大为策略性终止。
  - Proof needed: 删除或降级 hard-stop；保留单次 patch grammar/target 合同拒绝；测试证明失败后合法工具动作仍可继续采样。

##### Non-blocking Risks
- ProviderBudgetHardStop 基本回到硬基线，但 validation rework patch feedback grace 仍需实证。
  - Broken assumption: 专用 grace 一定只是预算宽限。
  - Failure scenario: grace 变成特定语义路径的隐式优先级。
  - Trigger condition: total budget exhausted but validation rework artifacts present.
  - Impact: 低到中，当前未见直接阻断。
  - Proof needed: 明确 grace 只影响总预算宽限，不生成策略性 next action。
- PathCorrectionHardStop 已不可达，但 marker/测试壳残留。
  - Broken assumption: 不可达代码不会影响后续边界判断。
  - Failure scenario: 未来维护误复活旧 hard-stop。
  - Trigger condition: 复用旧 marker 或测试 helper。
  - Impact: 维护风险。
  - Proof needed: 删除 dead marker/helper 或显式改为 advisory-only 测试。
- ValidationReworkPatchOnlyHardStop 已不可达，但构造函数和旧测试残留。
  - Broken assumption: predicate 返回 false 即足够。
  - Failure scenario: 旧测试/构造函数继续表达 hard-stop 正当性，后续改动可能复活。
  - Trigger condition: 维护者按旧测试修复失败。
  - Impact: 维护风险，且本地主线程已复现聚焦测试失败。
  - Proof needed: 删除构造函数/marker，更新测试为 evidence-only。

##### User-Perspective Checks
- Usability: risk - recovery 文案中的 “exactly one apply_patch” 和 “Do not call read_file” 会让 Agent/操作者误以为这是状态机规则。
- Ease of use: risk - dead marker 和旧测试让边界状态难以判断。
- Ease of understanding: finding - 当前 hard-stop 名称和 recovery 文案混合了工具合同、预算底线和策略建议。

##### Implementation Completeness Checks

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status | Finding Link |
|---|---|---|---|---|---|---|---|---|
| Provider budget | total rollout budget only hard-stops | `runtime.rs:1299` | pre-dispatch gate | budget tests pass in prior run | H-186/E-357 | none | landed | non-blocking risk |
| Path correction | advisory only | `turn.rs:2087`, `turn.rs:2138`, `turn.rs:3122` | action-contract recovery | tests show no hard-stop | H-168/H-187 context | dead marker | partial | non-blocking risk |
| Patch-only recovery | evidence-only, no hard-stop | `turn.rs:2599`, `turn.rs:3095` | recovery item | focused old test fails | no live blocker | dead constructor | partial | non-blocking risk |
| Duplicate rework read | only exact duplicate complete read may be hard contract | `runtime.rs:2237`, `turn.rs:2489`, `turn.rs:3085` | prepare_main_tool_call + recovery loop | tests cover old hard-stop behavior | no fresh live isolation | none | partial | blocking finding |
| Apply patch recovery | single tool-contract rejection only | `turn.rs:3103`, `turn.rs:2815` | recovery loop | tests assert hard-stop | no fresh live isolation | none | partial | blocking finding |

##### Target Benefit Checks

| Claimed Benefit | Baseline | Target | Measurement Method | Comparison Evidence | Result | Regression / Side Effect | Status | Finding Link |
|---|---|---|---|---|---|---|---|---|
| Runtime stops semantic overcontrol | H171-H187 exposed forced transitions/path correction/implementation hard stops | only hard baselines remain | static audit + target sample + tests | H-187 target sample passed | partial | remaining recovery strategy injection | weak-evidence | blocking findings |
| Feedback layer semantic passthrough | prior projection/recovery injected next actions | factual constructor / tool feedback only | static text audit | `rg` found many “exactly one”/“Do not call” recovery strings | partial | Agent context pollution | weak-evidence | blocking findings |

##### Required Fixes
- Remove or downgrade `TaskSpaceApplyPatchRecoveryHardStopV1`; keep only single-action apply_patch contract rejection.
- Rewrite duplicate-read recovery as factual duplicate-read/tool-contract feedback, not “exactly one apply_patch” strategy.
- Remove dead `TaskSpaceValidationReworkPatchOnlyHardStopV1` and `TaskSpacePathCorrectionHardStopV1` markers/helpers or make their advisory-only status explicit.
- Re-audit `block_node` rejection heuristics that say implementation cannot be blocked because runtime thinks patch is available.

##### Missing Tests
- Property test: every state-machine-legal tool action remains allowed unless it violates explicit hard contract.
- Text test: recovery items must not contain closed action-space wording such as “Do not call read_file/list_files/search” unless that action is itself a hard node-policy violation.
- Duplicate-read test: only exact duplicate complete target read is rejected; different read/search/list remains legal.
- Apply-patch failure test: after patch grammar failure, legal read/search/list/control can still be emitted in later sampling.

##### Missing Logs / Observability
- Boundary audit metric for recovery item class: `hard_baseline`, `tool_contract`, `advisory_hint`, `semantic_strategy`.
- Trace field indicating whether a rejection blocked a tool dispatch, terminal action, or only emitted a hint.

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:2527` - duplicate-read recovery text uses “Current required behavior” and `exactly one apply_patch`.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:2268` - duplicate-read gate blocks read/search before successful edit.
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:3103` - apply_patch recovery hard-stop predicate.
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:2815` - apply_patch recovery hard-stop constructor.
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:3095` - patch-only hard-stop predicate currently returns false.
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:3122` - path-correction hard-stop helper is test-only and returns false.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| architecture-adversary | duplicate-read recovery injects strategy | exact duplicate read gate became “must patch now” context | blocking | accept | Local audit confirms `runtime.rs:2237` blocks duplicate read/search and `turn.rs:2527` injects “exactly one apply_patch” plus “Do not call read_file/list_files/search”. | Recorded as H-188/E-360/E-361. No code fix in audit round. | Next repair pass should rewrite as factual duplicate-read feedback and narrow hard contract. |
| architecture-adversary | ApplyPatchRecoveryHardStop escalates tool failures into semantic stop | repeated patch failure stops sampling and forbids other legal recovery actions | blocking | accept | Local audit confirms `turn.rs:3103` and `turn.rs:2815`; text audit finds multiple “Do not call read_file/list_files/search” recovery strings. | Recorded as H-188/E-360/E-361. No code fix in audit round. | Next repair pass should delete/downgrade hard-stop and preserve tool-contract rejections. |
| architecture-adversary | ProviderBudgetHardStop grace needs proof | validation-rework grace could become hidden strategy preference | non-blocking | defer | Total budget hard-stop itself is hard baseline; no direct live failure found. | Tracked in H-188 as non-blocking follow-up. | Add focused test/audit when touching budget gate. |
| architecture-adversary | PathCorrectionHardStop dead marker | unavailable runtime path but stale marker can mislead maintenance | non-blocking | accept | Local audit confirms marker remains while hard-stop helper is `#[cfg(test)]` and false. | Recorded as H-188/E-360/E-361. | Remove dead marker/helper during cleanup. |
| architecture-adversary | PatchOnlyHardStop dead marker / tests | predicate false but constructor and old tests remain | non-blocking | accept | Focused test failed on old expectation, proving stale test state. | Recorded as H-188/E-362. | Update/delete stale tests and dead constructor during cleanup. |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: no
- Blocking re-review passed: n/a
- Blocking re-review round links:
  - n/a
- Blocking re-review launch records:
  - n/a

### Repair Update

- Repair date: 2026-07-07
- Main repair summary:
  - Removed/downgraded stale apply_patch, validation-rework-patch-only, and path-correction hard-stop paths that converted tool feedback into provider stop conditions.
  - Rewrote duplicate validation rework read feedback as duplicate-evidence semantics only; it no longer tells the Agent that apply_patch is the only valid next action.
  - Rewrote apply_patch/action-contract feedback summaries from `next_valid_action`/`mandatory_next_action` wording to `tool_feedback_facts`, `correction_options`, `available_actions`, and explicit `state_machine_requirement` only where the state machine has a hard baseline.
  - Removed runtime block_node strategy heuristics that rejected blockers because runtime believed patching, retesting, or avoiding internal-policy blockers was the better semantic strategy; retained fact-contradiction and validation-node hard baselines.
  - Added/synchronized tests so implement-solution edit pressure remains advisory: read/test/tool actions are not rejected solely because runtime thinks an edit is preferable.
- Verification:
  - `cargo fmt` completed; rustfmt emitted existing stable-toolchain warnings for unstable `imports_granularity`.
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib validation_rework --locked` passed: 33 tests.
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib apply_patch --locked` passed: 55 tests.
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib action_contract_prompt --locked` passed: 29 tests.
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib blocker --locked` passed: 20 tests.
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib path_correction --locked` passed: 14 tests.
  - Targeted action-map snapshot/boundary tests passed after updating stale fixed-string expectations.
  - Full `codex-core --lib` run reached 2389 passed / 12 failed / 3 ignored. Remaining failures are outside this R4 boundary repair surface: guardian/env model-review cases, file watcher state-lock cases, thread manager provider-refresh count, and one existing final-gate wording assertion.
- Text audit:
  - Removed the audited stale hard-stop markers/predicates from production code.
  - The remaining matches for `next_valid_action`, `Current required behavior`, and `Do not call read_file` are negative test assertions only.
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Implementation completeness gaps resolved or accepted by user: yes
- Target benefit warnings recorded: yes
- Blocked reason: n/a after repair; accepted blocking findings were fixed locally.
- Allowed to proceed: yes

## Final Conclusion

The accepted Round 1 blocking findings have been repaired locally. Duplicate-read recovery now preserves duplicate-evidence facts without forcing `apply_patch`, apply-patch recovery no longer escalates repeated tool failures into strategy hard-stop, dead path-correction / patch-only hard-stop residue has been removed or downgraded, and block_node strategy rejection heuristics have been narrowed to hard evidence contradictions and validation baselines.

Residual risk remains around unrelated full-suite failures and the still-intentional exact duplicate complete-read hard baseline, but the audited runtime-boundary overreach cases are closed by the repair evidence above.

## Round 2: reachable runtime stop closure audit

### Review Input

#### Objective
Continue R4 runtime-stop cleanup under the clarified boundary: TaskSpace/runtime may enforce hard baselines and tool contracts, but must not stop provider sampling or reject legal Agent actions because runtime believes a semantic strategy is better.

#### Review Target
Post-`62993d8` runtime stop implementation, recovery loop, stop predicates, and tests for currently reachable hard-stop paths.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `coe/2026-07-03-05-03-r4-durable-evidence-gates.md`
- this report, especially Round 1 Repair Update

#### Change Introduction
Round 1 accepted blocking findings were repaired: apply_patch recovery hard-stop was removed/downgraded, duplicate-read recovery no longer forces `apply_patch`, stale path-correction / patch-only hard-stop residue was removed or downgraded, and block_node strategy heuristics were narrowed. The current question is whether remaining reachable runtime stops still cross the boundary.

#### Risk Focus
- `TaskSpaceProviderBudgetHardStopV1`: should enforce only total provider request budget, not hidden semantic preference.
- `TaskSpaceNoActionRecoveryHardStopV1`: should stop only repeated no-action / no-progress responses, not failed tool feedback or legal state-machine actions.
- `TaskSpaceValidationReworkDuplicateReadHardStopV1`: should not terminate the turn for an Agent's wrong but state-machine-legal behavior unless this is an explicit hard no-progress duplicate-evidence baseline.
- Validation / closed-validation action narrowing should be checked as node-policy hard baselines, not strategy guidance.
- Warning/recovery text must not inject “next best strategy” or prompt-like directives beyond hard contracts.

#### User-Perspective Review Focus
- Agent-facing feedback should be faithful, minimal, and understandable.
- A user or future maintainer should be able to distinguish hard baseline, tool contract, advisory hint, and semantic strategy.
- Runtime stop messages should not obscure whether the issue is Agent behavior, tool feedback, context projection, or hard budget exhaustion.

#### Implementation Completeness Focus
- Enumerate every reachable `*HardStopV1` path after `62993d8`.
- For each stop, identify trigger predicate, production entry point, tests, and whether non-stop legal alternatives remain allowed.
- Check whether accepted Round 1 fixes have actual production-path coverage rather than test-only assertions.

#### Target Benefit Focus
- Claimed benefit: runtime boundary is cleaner and feedback layer is closer to semantic passthrough.
- Baseline: Round 1 found runtime strategy injection and recovery hard-stop overreach.
- Target: only hard baselines stop provider sampling; recovery items preserve facts and available actions without deciding strategy.
- Measurement: static code audit, focused tests, and text scan.

#### Assumptions To Attack
- Repeated exact duplicate complete-read stop is a hard no-progress baseline rather than semantic strategy enforcement.
- No-action hard-stop cannot be reached after failed tools or valid but rejected state-machine actions.
- Provider budget grace cannot become an implicit semantic priority.
- “Do not call …” wording appears only in true node-policy hard baselines.
- Text tests are strong enough to prevent strategy-injection regression.

#### Adversarial Lenses
- state
- failure
- implementation-completeness
- testing
- observability
- maintenance

#### Verification Status
- Local focused tests passed for `validation_rework`, `apply_patch`, `action_contract_prompt`, `blocker`, `path_correction`, `implement_`, `implementation_recovery`, and selected snapshot tests.
- Full `codex-core --lib` remains at `2389 passed / 12 failed / 3 ignored`; residual failures are currently classified outside the R4 boundary repair surface.
- No fresh post-repair target sample / E3 run has been performed after `62993d8`.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Output must include summary, blocking findings, non-blocking risks, user-perspective checks, implementation completeness checks, target benefit checks, required fixes, missing tests, missing logs/observability, and evidence.
- Focus on falsifying the boundary claim. Do not repeat Round 1 unless the code still contains the issue.

### Internal Subagent Unavailable Fallback

- Internal subagent unavailable reason: n/a
- Local CLI discovery commands: n/a
- Discovered CLI candidates: n/a
- User-recommended alternative agent requested: n/a
- User approval requested: n/a
- Fallback outcome: n/a

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| high-risk | 20 min | 10 min if alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | Highest remaining risk is production stop-predicate behavior and whether legal actions are terminated under edge states. | state flow, failure handling, tests, production-path completeness |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| reviewer-2 | implementation-adversary | 1 | `019f3d4d-bddc-70b2-bbca-29af813465a6` | <20 min | completed | returned findings via subagent notification | completed |

### Reviewer Outputs

#### reviewer-2

##### Summary
Read-only adversarial review completed against commit `62993d8`. Reviewer found one blocking boundary regression: `TaskSpaceNoActionRecoveryHardStopV1` could still be reached through failed tool/path-correction feedback because the response classifier treated `tool_failure_recovery_message_present` as no-action even when a failed tool result was recorded as actionable output.

Round 1 apply_patch hard-stop and path-correction / patch-only hard-stop residues appeared removed from production paths. Duplicate-read feedback was cleaner, but duplicate-read hard-stop escalation still needed stronger same-action fingerprint proof.

##### Blocking Findings
- B1: failed tool feedback can still consume no-action recovery and hard-stop the turn.
  - Broken assumption: `TaskSpaceNoActionRecoveryHardStopV1` cannot be reached after failed tools or valid rejected state-machine actions.
  - Failure scenario: Agent emits a read/tool call that fails with path-correction feedback. Runtime records actionable failed tool feedback, but classifier returns `NoActionFollowUp`; repeated occurrences can exceed the advisory cap and stop provider sampling.
  - Trigger condition: `tool_path_correction_feedback.is_some()` is passed as `tool_failure_recovery_message_present=true` regardless of `saw_actionable_output=true`.
  - Impact: runtime can terminate sampling because a tool failed, not because the provider produced repeated no-action/no-progress text.
  - Proof needed: focused test proving failed tool/path-correction feedback with `saw_actionable_output=true` does not produce no-action classification or `TaskSpaceNoActionRecoveryHardStopV1`.

##### Non-blocking Risks
- Duplicate-read hard-stop is node-scoped count based: `previous_recovery_count > 0` hard-stops any later duplicate-read recovery on the same node. Reviewer asked for a direct test proving different legal actions avoid semantic escalation.
- Provider budget has a special validation-rework patch feedback grace beyond total rollout budget. Reviewer considered it probably acceptable as a final recovery allowance, but it should remain documented as a budget contract exception rather than a strategy preference.
- Some `Do not call ...` text remains in validation/inspect node-policy prompts. Most appear to be hard baselines; text-regression coverage should classify allowed vs forbidden contexts.

##### User-Perspective Checks
- Usability: finding B1. Failed tools being called no-action would look arbitrary to a user.
- Ease of use: duplicate-read recovery text is clearer after Round 1 because it says duplicate evidence only and leaves legal actions available.
- Ease of understanding: budget hard-stop wording is understandable, but `quality_impact_required` and `next_valid_actions` inside budget stop can still read like strategy guidance.

##### Implementation Completeness Checks

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status | Finding Link |
|---|---|---|---|---|---|---|---|---|
| Provider budget hard stop | only total rollout budget stops sampling | `runtime.rs` pre-dispatch budget gate, `turn.rs` provider budget item | sampling pre-dispatch gate | provider-budget tests | warning event in sampling gate | unit snapshots | partial | non-blocking risk |
| No-action hard stop | only repeated no-action/no-progress stops | classifier and recovery loop in `turn.rs` | sampling response classification | contradictory tests found by reviewer | warning/hard-stop branch in `turn.rs` | unit-level | failing | B1 |
| Duplicate complete-read stop | only repeated exact duplicate complete read stops | duplicate read gate plus recovery loop | action-map prepare tool call | different-read tests exist | repeated block JSON | unit-level | partial | non-blocking risk |
| Apply_patch recovery | failed edit feedback does not hard-stop | `turn.rs` recovery generation | recovery item generation | negative assertions | advisory warning only | unit-level | pass | none |
| Closed validation narrowing | validation nodes enforce current test/build baseline | action-contract feedback in `turn.rs` | validation node policy | focused tests reported passed | state-machine requirement text | unit-level | pass | none |

##### Target Benefit Checks

| Claimed Benefit | Baseline | Target | Measurement Method | Comparison Evidence | Result | Regression / Side Effect | Status | Finding Link |
|---|---|---|---|---|---|---|---|
| Runtime stops only hard baselines | prior strategy hard-stops | only total budget / hard node policy stop | static audit + tests | Round 1 repair plus Round 2 review | partial before fix | failed tools could be no-action | B1 |
| Failed tools remain legal progress | failed tool feedback should not count as no-action | actionable failed tool feedback is not no-action | classifier unit tests | contradictory tests found | failing before fix | arbitrary stop risk | B1 |
| Duplicate-read is duplicate evidence only | old forced patch directive | factual duplicate feedback | static text and tests | Round 1 repair | partial before fix | stop escalation remained | non-blocking risk |

##### Required Fixes
- Change `classify_taskspace_provider_response_actionability` so `saw_actionable_output=true` wins over failed-tool feedback, or split failed tool feedback recovery from no-action accounting.
- Update the failed-tool classifier test so failed tool feedback is actionable progress unless no tool result was actually recorded.
- Ensure path-correction recovery items do not increment `TaskSpaceNoActionRecoveryV1` counts when they came from recorded failed tool results.

##### Missing Tests
- Failed read/path-correction feedback with `saw_actionable_output=true` must not produce no-action classification.
- Repeated failed tool/path-correction feedback must not produce `TaskSpaceNoActionRecoveryHardStopV1`.
- Duplicate-read hard-stop should require the same complete read target/fingerprint, or hard-stop escalation should be removed.
- Text-regression test: `Do not call ...` only allowed in explicit node-policy hard baselines.

##### Missing Logs / Observability
- Add structured recovery class tags: `no_action`, `failed_tool_feedback`, `gate_rejection`, `hard_baseline`, `advisory`.
- Log whether a recovery item consumed the no-action cap.
- For duplicate-read escalation, log target artifact, previous result id, and repeated action fingerprint.

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs` classifier returned `NoActionFollowUp` for `tool_failure_recovery_message_present` before checking `saw_actionable_output`.
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs` response handling passed `tool_path_correction_feedback.is_some()` into that classifier and created recovery.
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs` old no-action hard-stop text included recoverable action-contract output.
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs` old classifier test asserted tool failure feedback is no-action recovery despite `saw_actionable_output=true`.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` duplicate complete-read gate preserved facts and allowed other legal actions.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| implementation-adversary | B1 failed tool feedback can consume no-action recovery and hard-stop | failed tool feedback was classified as no-action before actionable output | blocking | accept | Local inspection confirmed classifier order and old test expectation. | Added `ToolFeedbackRecovery`; actionable gate/tool failure feedback now classifies as `tool_feedback_recovery`, not `no_action_follow_up`. Removed `TaskSpaceNoActionRecoveryHardStopV1` marker, constructor, loop branch, warning branch, and tests. | Round 3 closure re-review required. |
| implementation-adversary | Duplicate-read hard-stop escalation lacks same-action proof | node-scoped duplicate-read count could terminate a later duplicate without fingerprint proof | major | accept | Local scan confirmed stop was count/item-text based. | Removed `TaskSpaceValidationReworkDuplicateReadHardStopV1` marker, constructor, loop branch, warning branch, predicate, and hard-stop tests; duplicate-read remains recoverable feedback only. | Round 3 closure re-review required. |
| implementation-adversary | Provider budget validation-rework grace needs documentation | semantic-looking exception could be mistaken for strategy preference | major | defer | Provider budget tests still show only rollout hard stop; grace remains narrow and tested. | Kept as budget contract exception; COE will record residual risk. | Revisit when budget policy is finalized. |
| implementation-adversary | `Do not call ...` text classification gap | node-policy baselines and strategy directives can be confused | major | defer | Current production hard-stop scan now only finds provider budget hard-stop; node-policy text remains for validation/closed-validation baselines. | No code change in this patch. | Add a dedicated text-policy audit after runtime stop cleanup. |

### Repair Update

- Code changes:
  - Removed non-budget provider-sampling stops: `TaskSpaceNoActionRecoveryHardStopV1` and `TaskSpaceValidationReworkDuplicateReadHardStopV1`.
  - Kept no-action and duplicate-read recovery as feedback/advisory items; they continue sampling until total provider budget hard-stop or another hard baseline.
  - Added `TaskspaceProviderResponseActionability::ToolFeedbackRecovery` so actionable failed tool/gate feedback can request recovery without being mislabeled as no-action.
  - Updated tests from hard-stop expectations to recoverable-feedback expectations.
- Validation:
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib provider_response_actionability --locked`: passed, 10 tests.
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib no_action_recovery --locked`: passed, 6 tests.
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib validation_rework_duplicate_read --locked`: passed, 7 tests.
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib provider_budget --locked`: passed, 23 tests.
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib path_correction --locked`: passed, 14 tests.
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib action_contract_prompt --locked`: passed, 29 tests.
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib validation_rework --locked`: passed, 33 tests.
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib apply_patch --locked`: passed, 55 tests.
  - Text scan for `TaskSpace.*HardStopV1`, `HARD_STOP`, `hard stop`, and provider stop text now finds only `TaskSpaceProviderBudgetHardStopV1` in production code.

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - pending Round 3
- Blocking re-review launch records:
  - pending Round 3
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Implementation completeness gaps resolved or accepted by user: partial
- Target benefit warnings recorded: yes
- Blocked reason: accepted blocking finding fixed locally; closure re-review still required.
- Allowed to proceed: no

## Round 3: closure re-review for non-budget runtime stop removal

### Review Input

#### Objective
Verify closure of Round 2 accepted blocking findings. Confirm that post-repair runtime stop behavior aligns with the boundary: only hard baselines, especially total provider budget, may stop provider sampling; failed tool feedback, no-action follow-up, and duplicate-read mistakes remain feedback/recovery, not runtime stop.

#### Review Target
Post-Round-2 implementation after removing `TaskSpaceNoActionRecoveryHardStopV1` and `TaskSpaceValidationReworkDuplicateReadHardStopV1`, plus adding `ToolFeedbackRecovery`.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `coe/2026-07-03-05-03-r4-durable-evidence-gates.md`
- this report Round 2 Main Agent Response and Repair Update

#### Change Introduction
The main agent accepted Round 2 B1 and duplicate-read escalation risk. Repair removed the two non-budget provider-sampling hard-stops and split actionable failed tool/gate feedback into `tool_feedback_recovery` instead of `no_action_follow_up`.

#### Risk Focus
- Confirm `TaskSpaceNoActionRecoveryHardStopV1` and `TaskSpaceValidationReworkDuplicateReadHardStopV1` are absent from production code and tests do not preserve them as desired behavior.
- Confirm failed tool/path-correction feedback with `saw_actionable_output=true` is not no-action classification.
- Confirm duplicate-read recovery no longer stops provider sampling after repeated feedback.
- Confirm `TaskSpaceProviderBudgetHardStopV1` remains as the only production `*HardStopV1` provider stop.
- Identify any replacement path that still stops sampling for non-budget recovery.

#### User-Perspective Review Focus
- Agent-visible feedback should distinguish `no_action_follow_up` from `tool_feedback_recovery`.
- Repeated bad Agent actions should remain visible feedback unless total budget or hard node/tool contract applies.

#### Implementation Completeness Focus
- Check marker constants, constructors, predicates, loop branches, warning text, and tests.
- Check response actionability traces use the new `tool_feedback_recovery` classification.
- Check validation coverage named in Round 2 Repair Update.

#### Target Benefit Focus
- Claimed benefit: runtime no longer has non-budget recovery hard-stops.
- Baseline: Round 2 found no-action hard-stop after failed tool feedback.
- Target: production text scan finds only provider budget hard-stop; tests verify failed-tool feedback and duplicate-read remain recoverable.

#### Assumptions To Attack
- Removing marker/constructor is enough; no alternate branch still breaks the provider loop.
- Tests were updated to meaningful behavior, not only weaker string checks.
- `ToolFeedbackRecovery` still generates required feedback and does not silently drop path-correction / gate feedback.
- Provider budget grace is still the only exception after total budget pressure.

#### Adversarial Lenses
- state
- failure
- implementation-completeness
- testing
- observability

#### Verification Status
- Focused tests reported passing in Round 2 Repair Update:
  - `provider_response_actionability`: 10 tests
  - `no_action_recovery`: 6 tests
  - `validation_rework_duplicate_read`: 7 tests
  - `provider_budget`: 23 tests
  - `path_correction`: 14 tests
  - `action_contract_prompt`: 29 tests
  - `validation_rework`: 33 tests
  - `apply_patch`: 55 tests
- Text scan reported only `TaskSpaceProviderBudgetHardStopV1` production matches.
- Full `codex-core --lib` has not been rerun after Round 2 repair yet.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Focus on closure: either pass the accepted blocking fixes or identify remaining blocking gaps.

### Internal Subagent Unavailable Fallback

- Internal subagent unavailable reason: n/a
- Local CLI discovery commands: n/a
- Discovered CLI candidates: n/a
- User-recommended alternative agent requested: n/a
- User approval requested: n/a
- Fallback outcome: n/a

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| high-risk | 20 min | 10 min if alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | Closure risk is concrete production behavior and tests around runtime stop predicates. | state flow, recovery classification, tests |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1.spawn_agent` | `019f3d56-4eb1-7391-85ff-6909b3f9bad6` | spawn_agent tool result | `fork_context=false` | Round 3 Review Input | main-agent history, hidden reasoning, drafts, conclusions, full diff | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| reviewer-3 | implementation-adversary | 1 | `019f3d56-4eb1-7391-85ff-6909b3f9bad6` | <20 min | completed | returned closure findings via subagent notification | completed |

### Reviewer Outputs

#### reviewer-3

##### Summary
Closure is verified for the Round 2 accepted blocking findings. Reviewer found no remaining production runtime path that stops provider sampling for failed tool feedback, no-action follow-up, or duplicate validation-rework reads. The only live `*HardStopV1` production marker in reviewed code is `TaskSpaceProviderBudgetHardStopV1`, and the session loop only breaks on that marker.

##### Blocking Findings
- none

##### Non-blocking Risks
- Stale historical hard-stop names remain in the COE document, including `TaskSpaceNoActionRecoveryHardStopV1` and `TaskSpaceValidationReworkDuplicateReadHardStopV1`. This is documentation/history, not production behavior, but future audits need to distinguish historical evidence from current code.
- Some duplicate-read recovery tests still used old fixture text such as “apply the smallest fix with apply_patch” or `next_valid_actions:["call apply_patch ..."]`. Reviewer noted this did not preserve hard-stop behavior, but could confuse maintainers.

##### User-Perspective Checks
- Failed tool/path-correction feedback with an actionable tool result now classifies as `tool_feedback_recovery`, not `no_action_follow_up`.
- Repeated no-action recovery can exceed the advisory threshold, but the turn continues.
- Duplicate-read recovery increments advisory counters and emits feedback, but does not stop sampling.
- Provider sampling stops only when the pre-dispatch provider budget gate returns the provider-budget hard-stop item.

##### Implementation Completeness Checks

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status | Finding Link |
|---|---|---|---|---|---|---|---|---|
| Remove no-action hard-stop | no marker/constructor/branch for `TaskSpaceNoActionRecoveryHardStopV1` | `turn.rs` recovery loop | sampling recovery handling | `no_action_recovery` tests | warning says turn continues | none | landed | none |
| Remove duplicate-read hard-stop | duplicate-read is counted but not terminal | `turn.rs` recovery loop and `runtime.rs` duplicate-read gate | action-map prepare tool call + recovery handling | `validation_rework_duplicate_read` tests | duplicate-read remains tool feedback | none | landed | none |
| Split tool feedback from no-action | actionable failed tool/gate feedback is `tool_feedback_recovery` | `turn.rs` response classifier | response actionability trace | `provider_response_actionability` tests | actionability logging records classification | none | landed | none |
| Keep provider budget hard-stop | only total provider budget stops sampling | `runtime.rs` pre-dispatch gate and `turn.rs` budget item | sampling pre-dispatch | `provider_budget` tests | provider budget warning | none | landed | none |

##### Target Benefit Checks

| Claimed Benefit | Baseline | Target | Measurement Method | Comparison Evidence | Result | Regression / Side Effect | Status | Finding Link |
|---|---|---|---|---|---|---|---|
| Failed tool feedback remains recovery | B1 found failed tools could become no-action hard-stop | `tool_feedback_recovery` classification | code review + focused tests | closure review | achieved | none found | proven | none |
| No-action follow-up remains advisory | no-action hard-stop stopped sampling | advisory feedback continues turn | code review + tests | closure review | achieved | repeated no-action now relies on total budget | proven | none |
| Duplicate-read remains feedback | duplicate-read hard-stop stopped sampling | duplicate-read recovery continues until budget/hard baseline | code review + tests | closure review | achieved | none found | proven | none |
| Total provider budget remains hard stop | budget stop should remain hard baseline | only provider-budget `*HardStopV1` remains | text scan + tests | closure review | achieved | provider grace remains documented risk | proven | none |

##### Required Fixes
- none for closure

##### Missing Tests
- No blocking missing tests.
- Useful additions:
  - Loop-level test showing repeated `ToolFeedbackRecovery` path-correction items never increment `TaskSpaceNoActionRecoveryV1`.
  - Loop-level test showing repeated duplicate-read recovery continues until provider budget rather than breaking early.
  - Refresh stale duplicate-read fixture strings so tests do not embed old “call apply_patch” recovery language unless deliberately testing legacy parsing.

##### Missing Logs / Observability
- No blocking observability gap.
- Useful improvement: log recovery marker type separately from generic `developer_recovery`.

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`: only production hard-stop marker in session code is `TaskSpaceProviderBudgetHardStopV1`.
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`: provider response classifier maps actionable gate/tool feedback to `ToolFeedbackRecovery`.
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`: only provider-budget hard-stop breaks the sampling loop.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`: provider budget pre-dispatch stop remains total-budget gated.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`: duplicate-read gate remains tool/action feedback and leaves other legal actions available.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| implementation-adversary | No remaining blocking findings | accepted Round 2 fixes might have left an alternate non-budget stop path | blocking closure | accept | Closure reviewer found no non-budget runtime stop path; local text scan confirms only provider budget hard-stop remains in production. | Marked Round 2 blocking closure passed. | none |
| implementation-adversary | COE contains historical hard-stop names | historical evidence can be mistaken for current behavior | minor | accept | COE is append-only historical evidence; E-366 states current code removed these stops. | No deletion; final status will explicitly label them historical. | none |
| implementation-adversary | Stale duplicate-read fixture strings | old strategy wording in tests could confuse maintainers | minor | accept | Local scan found fixture-only old `call apply_patch` / `apply the smallest fix` strings. | Rewrote fixture text to neutral `reuse result / choose state-machine-legal action / record blocked` language and reran focused duplicate-read tests. | completed |
| implementation-adversary | More loop-level tests could be useful | current tests are focused/unit level, not full recovery loop property tests | major | defer | Not blocking per closure reviewer; current focused tests and code review prove closure. | Track as follow-up test hardening. | add when building recovery-loop property suite |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 3
- Blocking re-review launch records:
  - `019f3d56-4eb1-7391-85ff-6909b3f9bad6`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Implementation completeness gaps resolved or accepted by user: yes
- Target benefit warnings recorded: yes
- Blocked reason: n/a
- Allowed to proceed: yes

### Post-Closure Verification

```text
cargo fmt --check
  passed

git diff --check
  passed

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib provider_response_actionability --locked
  passed: 10 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib no_action_recovery --locked
  passed: 6 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib validation_rework_duplicate_read --locked
  passed: 7 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib provider_budget --locked
  passed: 23 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib action_contract_prompt --locked
  passed: 29 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib validation_rework --locked
  passed: 33 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib apply_patch --locked
  passed: 55 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked
  failed: 2390 passed; 12 failed; 3 ignored
```

Full-suite residual failures are outside this runtime-stop closure surface: two `file_watcher` tests, guardian / MCP guardian permission tests requiring a working DeepSeek guardian review environment, `session::tests::action_map_final_gate_failure_records_developer_followup`, and `thread_manager::tests::new_uses_active_provider_for_model_refresh`.

## Final Conclusion

R4 runtime-stop boundary closure passed adversarial re-review. Production runtime now has only `TaskSpaceProviderBudgetHardStopV1` as a provider-sampling hard stop. No-action recovery, failed tool/path-correction feedback, and duplicate validation-rework reads remain feedback/recovery paths rather than runtime stop paths.

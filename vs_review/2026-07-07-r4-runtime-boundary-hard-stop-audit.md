# Subagent VS Review: R4 runtime boundary hard-stop audit

- Created: 2026-07-07 22:44:47 +0800
- Updated: 2026-07-07 22:44:47 +0800
- Report schema: adversarial-v1
- Task: 审计 R4 TaskSpace runtime hard-stop / recovery 逻辑是否仍越过“只守硬基线、不替 Agent 做语义策略决策”的边界。
- Report path: `vs_review/2026-07-07-r4-runtime-boundary-hard-stop-audit.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context; reviewer received only the review navigation packet
- Status: fixed-after-repair

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

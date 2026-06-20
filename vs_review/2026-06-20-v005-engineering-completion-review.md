# Subagent VS Review: v0.0.5 Engineering Completion

- Created: 2026-06-20T04:02:00+08:00
- Updated: 2026-06-20T04:02:00+08:00
- Report schema: adversarial-v1
- Task: 审查 v0.0.5 工程缺口是否真的补齐，重点寻找未实现、漏做、只有入口/框架、fixture/mock 替代真实实现的问题。
- Report path: `vs_review/2026-06-20-v005-engineering-completion-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: open

## Round 1: 工程完成声明对抗性审查

### Review Input

#### Objective
验证 v0.0.5 “只看工程代码和非 agent gate 已完成”的声明是否站得住。重点不是评估真实 E3 效果，而是找出工程实现是否存在入口已写但 runtime 未接入、脚本 fixture 代替真实 producer、mock 数据冒充证据、文档完成但代码缺失、测试只测 happy path 等问题。

#### Review Target
代码实现、测试策略、release/start gate、v0.0.5 工程完成审计文档。

#### Target Locations
- `docs/v0.0.5/17-unfinished-work-inventory.md`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- `docs/v0.0.5/20-v005-engineering-code-complete-audit-2026-06-20.md`
- `docs/v0.0.5/checklists/acceptance-checklist.md`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/client.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs`
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1`
- `scripts/taskspace-benchmark/write-release-decision.ps1`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1`
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1`
- `scripts/taskspace-benchmark/test-cost-instrumentation.ps1`
- `scripts/taskspace-benchmark/test-release-decision.ps1`
- `scripts/taskspace-benchmark/test-e3-start-gate.ps1`

#### Change Introduction
v0.0.5 后续开发补齐了 provider request budget hook、active context replacement、tool-call/output 成对过滤、state_commit displacement artifact、spawn/node budget、release/start gate marker 校验和相关非 agent gates。主线结论声称：工程代码和非 agent gate 已 CODE-COMPLETE，但真实 Terminal-Bench E3 效果尚未证明。

#### Risk Focus
- 是否存在 `TaskSpaceProviderRequestEventV1` 只由后处理脚本从 trace 推断，而不是 provider dispatch lifecycle 真实产生。
- 是否存在 active budget 只记录事件或报告，不在真实 provider dispatch / runtime expansion 前阻断。
- 是否存在 release/start gate 接受 fixture/mock marker，而不要求真实 evidence 与当前 commit/source/profile/sample set 绑定。
- 是否存在 active context replacement 只覆盖测试构造文本，没有覆盖真实 ChatCompletions provider-visible message 组成边界。
- 是否存在 state_commit、spawn/node budget、output reference 只是 parser/report 可见，没在 active profile 默认路径形成约束。
- 是否存在 “工程完成” 文档把尚未真实接入的 Phase 0A-5 需求误判为完成。

#### User-Perspective Review Focus
- 用户关心的是“有没有没实现、漏做、mock 方案”，不是测试列表。
- 审查输出必须能明确指出：哪些是 blocking，哪些只是仍需真实 E3 证明。

#### Assumptions To Attack
- focused tests 覆盖真实 runtime 入口。
- fixture gates 足以证明 producer-owned artifacts。
- provider request hook 是 canonical producer，而不是后处理重建。
- budget hard stop 会阻止真实请求继续发出。
- start gate 会在正式 E3 scheduling 前阻断，且不能被空 marker/错样本 marker 绕过。
- 当前 `CODE-COMPLETE` 只限工程口径，没有混入效果口径。

#### Adversarial Lenses
- implementation
- testing
- observability
- release gating
- state
- failure
- maintenance

#### Verification Status
- 主 agent 已运行并通过：
  - `cargo test -p codex-core active_context_replacement -- --nocapture`
  - `cargo test -p codex-core provider_request_budget -- --nocapture`
  - `cargo test -p codex-core state_commit -- --nocapture`
  - `cargo test -p codex-core output_reference -- --nocapture`
  - `cargo test -p codex-core projection -- --nocapture`
  - `cargo test -p codex-core budget -- --nocapture`
  - `scripts/taskspace-benchmark/test-cost-instrumentation.ps1`
  - `scripts/taskspace-benchmark/test-release-decision.ps1`
  - `scripts/taskspace-benchmark/test-harness.ps1`
  - `scripts/taskspace-benchmark/test-e3-start-gate.ps1`
- 没有运行真实 Terminal-Bench E3。
- 没有运行真实 agent benchmark sample。

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Focus on high-impact missing implementation, mock/fixture substitution, and false completion evidence.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 15 minutes | one bounded extension if alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | 最高风险是工程实现是否真实接入 runtime，而不是文档或 UX。 | missing implementation, runtime integration, fake/mock evidence |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | multi_agent_v1.spawn_agent | `019ee368-9db7-7f73-8a36-5b2a5bf50af6` | `multi_agent_v1.spawn_agent` result in current Codex thread | fork_context=false | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless independently inspected | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| reviewer-output-1 | implementation-adversary | 1 | `019ee368-9db7-7f73-8a36-5b2a5bf50af6` | 15 minutes | completed | reviewer returned findings | completed |

### Reviewer Outputs

#### reviewer-output-1

##### Summary

只读审查结论：不能把 v0.0.5 继续无限定称为 `CODE-COMPLETE`。更准确的口径是：provider lifecycle hook 和 active context replacement 的主入口确实接到了 runtime/provider 边界，不是纯脚本后处理；但 active budget response、`state_commit` 默认替代路径、以及 release/start gate 的当前 commit 绑定仍有工程缺口。

##### Blocking Findings

- Active budget 只在 hard cap 阻断，中间预算状态没有真实 runtime response。
  - Broken assumption: active budget response 已经在真实 provider dispatch/runtime expansion 前形成控制。
  - Failure scenario: 请求数进入 `warned` / `thin_downgraded` / `compact_checkpoint_required` 状态时，系统只继续观察并放行，直到 `request_count >= max_requests` 才阻断。
  - Trigger condition: active profile 下 provider request count 超过 soft threshold 但未达到 max。
  - Impact: E3 成本膨胀可继续发生；报告里会出现 budget state，但没有真正执行 downgrade、compact checkpoint、bounded recovery 或质量补偿路径。
  - Proof needed: 一个 runtime/provider 测试证明 soft budget 状态会改变下一次 dispatch/expansion 行为，而不仅仅记录事件。
  - Evidence: `third_party/codex-cli/codex-rs/core/src/client.rs:329-375`、`third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1702-1704`、`third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1736-1769`

- `state_commit` 仍不是 active profile 的强制默认状态更新路径。
  - Broken assumption: legacy state update 已被 `state_commit` displacement 取代。
  - Failure scenario: 模型继续调用 legacy `record_fact` / `record_output_contract` / `record_success_criteria` 等动作，runtime 仍接受；只要 fixture 或某次样本调用过 `state_commit`，displacement 报告就可能显示通过。
  - Trigger condition: active profile 中模型没有自然采用 `state_commit`，或者混用 legacy update。
  - Impact: `state_commit displacement complete` 可能只是 artifact 可见，不代表真实 agent state path 已迁移。
  - Proof needed: active profile 下 legacy state mutation 超预算或被禁止的负向测试；真实 sample 中 `state_commit` adoption/displacement denominator 来自同一轮 `taskspace_control` 调用流。
  - Evidence: `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs:89-198`、`third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:3705-3743`、`docs/v0.0.5/18-unfinished-work-engineering-design.md:1021-1060`

- non-agent gate evidence 的 `git_commit` 没有强制等于当前 HEAD。
  - Broken assumption: release/start gate 的所有 evidence 都绑定当前 commit/source/profile/sample set。
  - Failure scenario: 旧 commit 生成的 non-agent gate evidence 被复制到当前 run root，只要 evidence hash、profile hash、source_version 等字段匹配，gate 可接受；当前 `code_complete_marker.git_commit` 可以是 HEAD，但子 gate evidence 是旧 commit。
  - Trigger condition: non-agent gate subgate JSON 中 `git_commit` 非空但不等于当前 HEAD。
  - Impact: 代码改动后可以复用旧的非 agent gate 证据，工程完成声明不再可靠。
  - Proof needed: fixture test：subgate `git_commit` stale 时 start gate 和 release decision 必须失败。
  - Evidence: `scripts/taskspace-benchmark/lib/e3-start-gate.ps1:220-237`、`scripts/taskspace-benchmark/lib/e3-start-gate.ps1:240-260`、`scripts/taskspace-benchmark/write-release-decision.ps1:727-763`

##### Non-blocking Risks

- Provider lifecycle 不是纯后处理脚本，这点基本成立。真实 hook 在 dispatch 前调用并记录 payload，见 `third_party/codex-cli/codex-rs/core/src/client.rs:1906-1910`、`third_party/codex-cli/codex-rs/core/src/client.rs:2217-2284`。但 `CODEX_RS_SSE_FIXTURE` 会在 budget dispatch 前直接返回，见 `third_party/codex-cli/codex-rs/core/src/client.rs:1853-1865`，所以 fixture-backed runs 不能被当成 provider lifecycle evidence。
- Active context replacement 接在 provider-visible prompt composition 边界，不是纯报告逻辑。入口在 `third_party/codex-cli/codex-rs/core/src/session/turn.rs:443-458`，实际过滤在 `third_party/codex-cli/codex-rs/core/src/session/turn.rs:1373-1478`。风险是 exact scan 仍是字符串扫描，`taskspace_control` 等词如果出现在受保护用户输入里可能误判 legacy history，见 `third_party/codex-cli/codex-rs/core/src/client.rs:745-780`。
- acceptance checklist 仍未闭环，尤其是 active/shadow、state_commit、raw prompt omission、projection token 等项仍是未勾选状态，见 `docs/v0.0.5/checklists/acceptance-checklist.md:7-8`、`:27-39`、`:49-51`。这不等同于代码缺失，但和 `CODE-COMPLETE` 口径冲突。

##### Missing Tests

- stale subgate `git_commit` 必须失败的 start gate / release decision 负向测试。
- soft budget 状态触发实际 response 的测试：warn/thin/compact 不应只是 observe。
- active profile 下 legacy state mutation 被拒绝、限流或强制转 `state_commit` 的测试。
- ChatCompletions provider-visible payload 的端到端 replacement 测试，而不只测内部 `ResponseItem` 构造文本。
- fixture-backed provider request evidence 被拒绝的 release/start gate 测试。
- 真实 `whale exec --taskspace` smoke：同一 run 中验证 Experiment mode、active projection、provider payload event、budget event、state_commit displacement 来自真实执行流。

##### Missing Logs / Observability

- budget event 没有清楚区分“状态被观察到”和“系统实际采取了 response action”。
- `state_commit_displacement` 缺少和同一真实 `taskspace_control` 调用序列绑定的 denominator provenance。
- release/start gate 日志没有输出每个 subgate 的 `git_commit` 与 current HEAD 对比。
- active replacement exact scan 没有指出 offending legacy token 的来源类别，难以区分真实 legacy history 和 protected user text false positive。

##### Evidence

- `third_party/codex-cli/codex-rs/core/src/client.rs:329-375`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1702-1769`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs:89-198`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1:220-260`
- `scripts/taskspace-benchmark/write-release-decision.ps1:727-763`

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| implementation-adversary | Active budget 只在 hard cap 阻断，中间预算状态没有真实 runtime response | soft budget 状态未改变 dispatch/expansion 行为 | blocking | accept | 当前代码确实只有 hard stop 前置阻断；非 blocked 状态主要记录为 observe。 | 修复：`ProviderRequestBudgetContext::before_dispatch` 在 `compact_checkpoint_required` 状态阻断普通请求，只允许 `budget_recovery` / `final_synthesis` / `final_abort`；budget quality action 区分 `compact_checkpoint_required`。验证：`cargo test -p codex-core provider_request_budget -- --nocapture`、`cargo test -p codex-core budget -- --nocapture` 通过。 | fresh closure review |
| implementation-adversary | `state_commit` 仍不是 active profile 的强制默认状态更新路径 | legacy state actions 仍可在 active profile 无约束使用 | blocking | accept | legacy actions 仍暴露；displacement 事件不能证明 denominator 覆盖所有 legacy mutation。 | 修复：`taskspace_control` handler 拒绝 direct legacy state mutation action，要求 `state_commit`；`start_task` initial fields 和 `state_commit` 仍允许。验证：`active_profile_rejects_direct_legacy_state_action`、`active_profile_allows_state_commit_action`、`state_commit` suite 通过。 | fresh closure review |
| implementation-adversary | non-agent gate evidence 的 `git_commit` 没有强制等于当前 HEAD | stale non-agent evidence 可复用 | blocking | accept | start gate / release decision 当前只要求 subgate `git_commit` 非空。 | 修复：E3 start gate 与 release decision 均要求 non-agent subgate `git_commit` 等于当前 HEAD；release decision 也要求 code-complete marker `git_commit` 等于当前 HEAD；补 stale commit negative fixture。验证：`test-release-decision.ps1`、`test-e3-start-gate.ps1` 通过。 | fresh closure review |
| implementation-adversary | fixture-backed runs 不能当成 provider lifecycle evidence | fixture path 可绕过 provider dispatch budget hook | major | accept | `CODEX_RS_SSE_FIXTURE` 在 client path 早返回。 | 部分处理：非 agent marker subgate 现在必须绑定当前 HEAD、local evidence sha 和 identity；仍需后续把 fixture-backed provider evidence 与 real provider lifecycle evidence 分级。 | 记录为 non-blocking residual risk |
| implementation-adversary | exact scan 字符串误判风险 | 用户文本包含 taskspace_control 可能误判 | major | defer | 当前任务重点是未实现/漏做；该项不阻止工程路径接入，但需要后续改成结构化 provenance。 | defer to v0.0.6 observability hardening | 不作为本轮 blocking |
| implementation-adversary | acceptance checklist 未闭环导致口径冲突 | 未勾选 checklist 和 CODE-COMPLETE 文档冲突 | major | accept | 文档状态会误导工程完成判断。 | pending doc correction | 修复后更新审计文档 |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: pending
- Blocking re-review passed: pending
- Blocking re-review round links:
  - pending
- Blocking re-review launch records:
  - pending
- Rejected findings backed by evidence: pending
- Deferred findings documented: pending
- Blocked reason: accepted blocking fixes require fresh closure review
- Allowed to proceed: no

## Round 3: Remaining Blocking Closure Review

### Review Input

#### Objective
复核 Round 2 后 active budget runtime attribution 修复是否关闭，并确认 HEAD binding 负向测试补齐。

#### Review Target
剩余 blocking closure：provider request phase 真实 runtime attribution；HEAD binding negative fixtures。

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `third_party/codex-cli/codex-rs/core/src/client.rs`
- `third_party/codex-cli/codex-rs/core/src/client_tests.rs`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1`
- `scripts/taskspace-benchmark/write-release-decision.ps1`
- `scripts/taskspace-benchmark/test-e3-start-gate.ps1`
- `scripts/taskspace-benchmark/test-release-decision.ps1`

#### Change Introduction
Round 2 found that the compact checkpoint allowlist was helper-only because `turn.rs` hard-coded every provider request as `model_sampling`. The follow-up fix moved request phase into `ActionMapProviderRequestBudgetSnapshot`, derives `final_synthesis` from the active node kind, and makes `turn.rs` use the snapshot phase. Additional stale code-complete git commit fixtures were added.

#### Risk Focus
- Whether final/recovery phase is now reachable from real session provider attribution.
- Whether stale code-complete marker commits are rejected in both start gate and release decision.

#### User-Perspective Review Focus
- Confirm no remaining “only helper/fixture” gap for the accepted blocking fixes.

#### Assumptions To Attack
- `final_synthesis` request phase comes from action map runtime, not a unit-test-only attribution.
- `turn.rs` no longer hard-codes `model_sampling`.
- HEAD binding has negative tests for subgates and code-complete markers.

#### Adversarial Lenses
- implementation
- testing
- release gating

#### Verification Status
- `cargo test -p codex-core provider_request_budget -- --nocapture` passed.
- `cargo test -p codex-core state_commit -- --nocapture` passed.
- `cargo test -p codex-core budget -- --nocapture` passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1 -RunRoot target\v005-review-release-selftest-2` passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1 -RunRoot target\v005-review-start-gate-selftest-2` passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1` passed.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 15 minutes | one bounded extension if alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | Closure target is runtime/gate implementation correctness. | accepted blocking closure |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | multi_agent_v1.spawn_agent | `019ee38d-f29d-7060-9bac-b260c204440d` | `multi_agent_v1.spawn_agent` result in current Codex thread | fork_context=false | Round 3 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless independently inspected | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| reviewer-output-3 | implementation-adversary | 1 | `019ee38d-f29d-7060-9bac-b260c204440d` | 15 minutes | completed | closure reviewer passed remaining fixes | completed |

### Reviewer Outputs

#### reviewer-output-3

##### Summary

Read-only closure review only. No blocking regressions were found in the scoped files. Active budget runtime attribution now appears wired through the real session path, HEAD binding is enforced in both start gate and release decision, and `state_commit` direct legacy rejection has no obvious rollback.

##### Blocking Findings

- none

##### Non-blocking Risks

- Active budget session-path closure is covered by code flow and unit tests, but there is no full integration test that drives an actual streamed turn while the current main node is `final_synthesis` and asserts provider lifecycle event attribution end-to-end.
- `final_synthesis` attribution depends on `current_main_node_id` being set to the final node before provider dispatch. Runtime supports that path, but a higher-level session fixture would make future regressions harder.

##### User-Perspective Checks

- Usability: pass - Review target is internal engineering gate behavior.
- Ease of use: pass - No user-facing workflow changed.
- Ease of understanding: risk - internal audit docs must retain clear boundary between engineering completion and real E3 effect proof.

##### Required Fixes

- none

##### Missing Tests

- Full streamed-turn integration test for final_synthesis provider lifecycle attribution would be useful but is not blocking for this closure.

##### Missing Logs / Observability

- none blocking

##### Evidence

- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:407` - snapshot includes `request_phase`.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1556` - runtime snapshot derives phase from current main node.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1582` - `NodeKind::FinalSynthesis` maps to `final_synthesis`.
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:2555` - real turn path obtains snapshot and passes `snapshot.request_phase`.
- `third_party/codex-cli/codex-rs/core/src/client.rs:289` - compact checkpoint allows recovery/final phases.
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1:186` - start gate reads current HEAD.
- `scripts/taskspace-benchmark/write-release-decision.ps1:458` - release decision requires code-complete marker commit equals current HEAD.
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs:451` - handler calls legacy rejection before dispatch.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| implementation-adversary | No blocking findings | n/a | n/a | accept | Round 3 closure reviewer found all blocking fixes closed. | No further blocking code action required. | Commit and push review report and fixes. |
| implementation-adversary | Full streamed-turn integration test missing | Higher-level integration coverage would catch future attribution regressions. | major | defer | Current code-flow and focused runtime tests prove the closure target; full streamed-turn fixture is useful but heavier than this closure requirement. | Document as residual risk. | Track for future validation hardening. |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 3
- Blocking re-review launch records:
  - implementation-adversary / `019ee38d-f29d-7060-9bac-b260c204440d`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Blocked reason: n/a
- Allowed to proceed: yes

## Final Conclusion

Round 1 found three blocking implementation gaps. Round 2 found one remaining active budget runtime attribution gap. Round 3 closure review passed with no blocking findings. v0.0.5 engineering completion may proceed under the limited engineering/non-agent-gate scope; real E3/product effect remains unproven.

## Final Conclusion

Round 1 found blocking implementation gaps. Main agent accepted and fixed the blocking findings. The review remains open until a fresh closure review validates the fixes.

## Round 2: Blocking Closure Review

### Review Input

#### Objective
复核 Round 1 的 3 个 accepted blocking fixes 是否真正关闭，没有 mock/fixture 替代真实 runtime。

#### Review Target
实现修复、测试修复、release/start gate 修复。

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/client.rs`
- `third_party/codex-cli/codex-rs/core/src/client_tests.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1`
- `scripts/taskspace-benchmark/write-release-decision.ps1`
- `scripts/taskspace-benchmark/test-e3-start-gate.ps1`
- `scripts/taskspace-benchmark/test-release-decision.ps1`

#### Change Introduction
Round 1 accepted 3 blocking findings and implemented fixes:

- provider request budget now blocks regular dispatch at `compact_checkpoint_required`, while allowing recovery/final phases.
- direct legacy state mutation actions in `taskspace_control` handler are rejected in active profile, forcing `state_commit`.
- start gate and release decision require v0.0.5 non-agent subgate `git_commit` values to match current HEAD; release decision also binds code-complete marker to HEAD.

#### Risk Focus
- 修复是否只是测试 helper 或 fixture，不是 runtime/provider path。
- 是否仍存在 direct model-callable legacy state mutation path。
- stale non-agent gate evidence 是否仍可绕过 release/start gate。

#### User-Perspective Review Focus
- 用户要确认“没实现、漏做、mock 方案”这类问题是否真正关闭。

#### Assumptions To Attack
- `compact_checkpoint_required` 会改变 dispatch 行为。
- legacy state mutation 不能被模型通过 `taskspace_control` 直接调用。
- gate evidence 与当前源码 commit 绑定。

#### Adversarial Lenses
- implementation
- testing
- release gating
- failure

#### Verification Status
- `cargo test -p codex-core provider_request_budget -- --nocapture` passed.
- `cargo test -p codex-core active_profile_rejects_direct_legacy_state_action -- --nocapture` passed.
- `cargo test -p codex-core active_profile_allows_state_commit_action -- --nocapture` passed.
- `cargo test -p codex-core state_commit -- --nocapture` passed.
- `cargo test -p codex-core budget -- --nocapture` passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1 -RunRoot target\v005-review-release-selftest` passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1 -RunRoot target\v005-review-start-gate-selftest` passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1` passed.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 15 minutes | one bounded extension if alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | Closure target is runtime/gate implementation correctness. | accepted blocking closure |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | multi_agent_v1.spawn_agent | `019ee37f-7ccc-7722-badc-7402248fcae7` | `multi_agent_v1.spawn_agent` result in current Codex thread | fork_context=false | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless independently inspected | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| reviewer-output-2 | implementation-adversary | 1 | `019ee37f-7ccc-7722-badc-7402248fcae7` | 15 minutes | completed | closure reviewer returned partial closure finding | completed |

### Reviewer Outputs

#### reviewer-output-2

##### Summary

Read-only closure review completed. Two fixes look closed at runtime. The active budget soft-response fix is only partially closed because its allowlist exists, but the real session path never appears to set `request_phase` to the allowed recovery/final phases.

##### Blocking Findings

- Active budget allowlist is not wired into real runtime attribution.
  - Broken assumption: `budget_recovery` / `final_synthesis` / `final_abort` phases are reachable from production `ProviderRequestAttribution`.
  - Failure scenario: `ProviderRequestBudgetContext` allows recovery/final phases, but `session/turn.rs` hard-codes every real request as `model_sampling`.
  - Trigger condition: compact checkpoint is reached and a real final synthesis request is attempted.
  - Impact: compact-checkpoint ordinary dispatch is blocked, but the intended soft escape phases are not reachable through real session dispatch.
  - Proof needed: runtime snapshot/turn attribution must carry a real final/recovery phase; test must prove final_synthesis snapshot phase is produced.
  - Evidence: `third_party/codex-cli/codex-rs/core/src/client.rs:289-295`, `third_party/codex-cli/codex-rs/core/src/client.rs:338-399`, `third_party/codex-cli/codex-rs/core/src/session/turn.rs:2555-2568`

##### Non-blocking Risks

- `CODEX_RS_SSE_FIXTURE` bypasses budget handling in fixture mode; fixture-based tests can hide budget behavior.
- `action_map/basemap.rs` still says legacy `record_*` actions are available for focused corrections, but runtime now rejects them.

##### Missing Tests

- Missing integration-style budget test through `session/turn.rs` or fake client transport proving compact checkpoint runtime allows only recovery/final phases.
- Missing negative runtime test proving normal `model_sampling` is blocked while real final/recovery phase is allowed.
- Missing full handler invocation test for legacy state action rejection after normalization.
- Missing stale `v005-code-complete.json.git_commit` fixtures for start gate and release decision.

##### Missing Logs / Observability

- No additional blocking observability finding beyond Round 1 residuals.

##### Evidence

- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:2555-2568`
- `third_party/codex-cli/codex-rs/core/src/client.rs:289-399`
- `third_party/codex-cli/codex-rs/core/src/client_tests.rs:149-161`

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| implementation-adversary | Active budget allowlist is not wired into real runtime attribution | `turn.rs` hard-coded `model_sampling`, so final/recovery phases were helper-only | blocking | accept | Reviewer evidence is correct. | 修复：`ActionMapProviderRequestBudgetSnapshot` 新增 `request_phase`；runtime 根据 current node kind 推导 phase，`FinalSynthesis` 产生 `final_synthesis`；`turn.rs` 使用 snapshot phase，而不是硬编码 `model_sampling`。新增 `provider_request_budget_snapshot_uses_final_synthesis_phase`。验证：`provider_request_budget`、`budget` 通过。 | fresh Round 3 closure review |
| implementation-adversary | Missing stale code-complete git fixtures | HEAD binding缺少 code-complete stale commit 负向覆盖 | major | accept | 负向测试覆盖不足。 | 修复：`test-e3-start-gate.ps1` 和 `test-release-decision.ps1` 增加 stale code-complete `git_commit` fixtures。验证：release/start gate self-test 通过。 | fresh Round 3 closure review |
| implementation-adversary | `CODEX_RS_SSE_FIXTURE` bypasses budget handling | fixture mode hides provider budget behavior | major | defer | 该路径是 test fixture transport；本轮 blocking 已通过 production attribution 修复和 non-agent evidence HEAD/hash gate 降低误用风险。 | Defer to fixture evidence labeling hardening. | Track as residual non-blocking risk |
| implementation-adversary | `basemap.rs` stale prompt says legacy record actions are available | prompt may induce rejected legacy calls | major | defer | Runtime now rejects direct legacy state mutation. Prompt cleanup is desirable but not required to close implementation enforcement. | Defer to prompt cleanup after closure. | Track as residual non-blocking risk |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2
  - Round 3
- Blocking re-review launch records:
  - implementation-adversary / `019ee37f-7ccc-7722-badc-7402248fcae7`
  - implementation-adversary / `019ee38d-f29d-7060-9bac-b260c204440d`
- Rejected findings backed by evidence: yes
- Deferred findings documented: yes
- Blocked reason: none after Round 3
- Allowed to proceed: yes, for non-agent engineering closeout only; real E3 still requires explicit start-gate/user approval

## Round 3 Closure Review

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | multi_agent_v1.spawn_agent | `019ee38d-f29d-7060-9bac-b260c204440d` | `multi_agent_v1.spawn_agent` result in current Codex thread | fork_context=false | Round 3 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless independently inspected | yes |

### Reviewer Outputs

#### reviewer-output-3

##### Summary

Fresh closure review passed. The reviewer found no remaining blocking implementation gaps for the accepted findings from Round 1 and Round 2.

##### Closure Judgement

- Active budget soft response: closed. `compact_checkpoint_required` blocks ordinary provider dispatch while permitting the intended final/recovery phases, and runtime attribution now carries the phase instead of hard-coding all requests as `model_sampling`.
- `state_commit` active-profile path: closed. Direct legacy state mutation actions are rejected by the handler for active profiles; `state_commit` and `start_task` initial fields remain available.
- HEAD-bound non-agent gates: closed. E3 start gate and release decision require current-HEAD evidence, including stale code-complete negative fixtures.

##### Non-blocking Residual Risk

- There is still no full streamed-turn integration test for final-synthesis provider lifecycle attribution. Current coverage is focused unit/fixture coverage rather than an end-to-end session transport test.

### Final Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Deferred findings documented: yes
- Allowed to proceed: yes, for engineering-code-complete documentation and commit. Real E3 remains outside this review and must pass the separate start gate.

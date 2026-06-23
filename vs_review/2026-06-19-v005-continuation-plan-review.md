# Subagent VS Review: v0.0.5 continuation plan

- Created: 2026-06-19T17:57:40+08:00
- Updated: 2026-06-19T17:57:40+08:00
- Task: 对 v0.0.5 未完成项工程方案执行对抗性审查，并修正发现的方案/gate 问题。
- Report path: `vs_review/2026-06-19-v005-continuation-plan-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: blocked

## Round 1: continuation design and gate review

### Review Input

#### Objective
审查 v0.0.5 继续开发方案是否真实覆盖用户目标：不能只做观测，必须实际降低 terminal-bench E3 TaskSpace 相比 Standard 的时间/token/request 成本，同时保证正确率不下降；代码完成前禁止真实 E3。

#### Review Target
方案、实验制度、release/start gate、以及当前 runtime producer 现状。

#### Target Locations
- `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- `docs/v0.0.5/17-unfinished-work-inventory.md`
- `docs/v0.0.5/13-design-corrections-and-engineering-contract.md`
- `docs/experiments/README.md`
- `docs/experiments/taskspace-evidence-levels-and-samples.md`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1`
- `scripts/taskspace-benchmark/write-release-decision.ps1`
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1`

#### Change Introduction
当前方案把 v0.0.5 从错误收口状态拉回继续开发，并设计 Phase 0A-5 runtime/gate 补齐路径。本轮审查挑战其是否仍有可绕过的样本、receipt、producer、成本阈值和 runtime ownership 漏洞。

#### Risk Focus
- 是否把成本控制降级为可观测。
- 是否允许用 diagnostic-only 或替代样本证明 v0.0.5 收口。
- active provider-visible history composer 是否仍是字符串过滤。
- spawn/node/state_commit budget 是否仍由脚本事后推断。
- start gate / release decision 是否能防止错误 E3 再次误导项目判断。

#### Verification Status
- 本轮审查前已有部分 release/start gate 加固。
- 本轮审查后已补充文档口径和低成本 gate 修复。
- 未运行真实 E3，未调用真实 agent benchmark。

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| product-logic-adversary | 检查 v0.0.5 是否仍满足用户产品目标和不收口约束。 | 产品目标、样本口径、成本/正确率结论 |
| architecture-adversary | 检查方案与实际代码是否仍是文档/gate 补丁而非 runtime-owned 架构闭环。 | runtime producer、context composer、预算职责 |
| test-validity-adversary | 检查 E3 实验制度是否还能被 weak smoke 或错误 receipt 绕过。 | start gate、release decision、样本/repeat/receipt |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| product-logic-adversary | `multi_agent_v1.spawn_agent` explorer | `019edf43-b293-7ff2-8c0a-52a13aeacd6b` | tool result and notification | no | Round 1 Review Input, product subset | main-agent history, reasoning, drafts, conclusions, full diff | yes |
| architecture-adversary | `multi_agent_v1.spawn_agent` explorer | `019edf43-e623-7662-abd8-c2462e9f4e83` | tool result and notification | no | Round 1 Review Input, architecture/code subset | main-agent history, reasoning, drafts, conclusions, full diff | yes |
| test-validity-adversary | `multi_agent_v1.spawn_agent` explorer | `019edf44-1c42-7543-95b3-e6398dbc142d` | tool result and notification | no | Round 1 Review Input, test/gate subset | main-agent history, reasoning, drafts, conclusions, full diff | yes |

### Reviewer Outputs

#### product-logic-adversary

##### Summary
方案主线覆盖用户目标，没有整体把成本控制降级成可观测；但有两个阻塞口径漏洞会让后续执行者绕过关键 gate 或误用替代样本收口。

##### Blocking Findings
- Phase 6 entry gate 写成 `Phase 1-5 gates PASS`，与前文 Phase 0A-5 和 provider hook 前置要求冲突。
- inventory 仍允许 `terminal-bench_E3-P0_3_5` 或同口径样本作为阶段性成本成功，口径不够硬。

##### Non-blocking Risks
- `Approved for Phase 0A-5 implementation` 可能被摘要误读成可收口。
- 旧 closeout 文档仍需 supersede。
- 已关闭的 provider hook 位置仍在开放问题区留下歧义。

##### Required Fixes
- Phase 6 entry criteria 改为 Phase 0A-5 non-agent gates PASS。
- 删除 inventory 中“或同口径样本”。
- README/closeout 入口应指向 unfinished inventory。

##### Missing Tests
- start gate must abort before scheduling when `full_e3_allowed=false`.
- marker spoofing fixture.
- exact payload scan join fixture.
- formal pair ledger fixture.

##### Missing Logs / Observability
- provider lifecycle canonical event.
- runtime-produced `BudgetQualityImpactV1`.
- request/walltime phase attribution.
- experiment header with evidence level/sample set/runner/score validity.

##### Evidence
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:19`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:1233`
- `docs/v0.0.5/17-unfinished-work-inventory.md:59`
- `docs/experiments/taskspace-evidence-levels-and-samples.md:154`

#### architecture-adversary

##### Summary
方案方向正确，但当前实现仍未完全从文档性补丁/脚本门禁跨到 runtime-owned 架构闭环。关键阻塞是 active composer 仍是字符串过滤，spawn/node 和 state_commit displacement 仍是脚本聚合/推断事实源。

##### Blocking Findings
- `prepare_provider_visible_prompt_items` 仍是 marker/string filtering，不是结构化 provider-visible context composer。
- spawn/node budget 仍由 `cost-instrumentation.ps1` 从 observability/tool calls 事后统计，不是 runtime hard budget。
- state_commit displacement gate 仍由脚本从 taskspace_control usage count 推断，不能证明模型路径被协议压缩替代。
- provider request budget 已有 runtime 事件层，但 `BudgetQualityImpactV1` 分类过粗，不能表达 thin/no-spawn/final-abort/validation-skip 的真实质量影响。

##### Non-blocking Risks
- release gate 绑定正式 E3 的逻辑强，但只能验证 artifact；如果上游 producer 弱，gate 会变成“强门禁 + 弱 producer”。

##### Required Fixes
- 将 provider-visible prompt 组装重构为 typed composer，输出 include/omit reason 和 protected invariant。
- 在 runtime/taskspace_control/spawn/create_node 入口加入 active budget state 和 hard cap。
- `spawn-node-budget-summary.json`、`state-commit-displacement.json` 改为消费 runtime producer events。
- 扩展 `BudgetQualityImpactV1` 真实记录质量影响来源。

##### Missing Tests
- typed composer fixture，含普通用户文本中出现 `TaskSpace` 的负例。
- spawn/node runtime budget unit tests。
- state_commit displacement runtime tests。
- budget quality impact hard-stop / validation-skip / final-abort fixtures。

##### Missing Logs / Observability
- provider-visible composition decision log。
- runtime spawn/node budget events。
- state_commit displacement events。
- quality impact fields must come from runtime path, not fixed defaults。

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:1322`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1509`
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1:291`
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1:307`

#### test-validity-adversary

##### Summary
实验制度已明显降低错误 E3/弱 smoke 误判风险，但 start gate 库函数、suite receipt 完整性和 direct/walltime cost blocker 仍有制度缺口。

##### Blocking Findings
- `Invoke-TaskspaceE3StartGate` 未提供 `TaskListPath` 时可让 task list gate skipped，非 suite 直接调用可能绕过正式样本集约束。
- suite receipt 只要求 `sample_scheduled >= 1` 和 `sample_completed >= 1`，不能证明 3 samples x 5 repeats 全部由 suite runner 调度完成。

##### Non-blocking Risks
- `09-e3-validation-plan.md` 仍保留旧口径，虽然已被新文档取代。
- one-pair smoke 仍偏弱，但不会直接授权 full E3。

##### Required Fixes
- formal/full E3 场景下缺 `TaskListPath`、`Benchmark=terminal-bench`、`Repeats=5` 必须 fail。
- suite receipt gate 必须验证 scheduled/completed 覆盖 P0 三样本且每样本 5 pair。
- direct input/output ratio 和 walltime ratio 超过 3x 时输出独立 blocker。

##### Missing Tests
- 完整 markers/calibration 但不传 `TaskListPath` 不得 full_e3。
- receipt 只含 1 个 sample completed 但 events 有 15 pairs 必须 fail。
- direct/walltime > 3x 且 correctness pass 时必须有 cost blocker。

##### Missing Logs / Observability
- markdown 输出 task list derivation details。
- release summary 输出结构化 direct/walltime blockers。

##### Evidence
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1:299`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1:476`
- `scripts/taskspace-benchmark/write-release-decision.ps1:495`
- `scripts/taskspace-benchmark/write-release-decision.ps1:736`

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| product-logic | Phase 6 omitted Phase 0A/0B gates | blocking | accept | `18` had Phase 1-5 wording only | Changed entry criterion to Phase 0A-5 non-agent gates with explicit producer/gate list | Covered by docs diff |
| product-logic | inventory allowed substitute same-mouth sample | blocking | accept | `17` allowed “或同口径样本” | Removed substitute-sample release wording; only `terminal-bench_E3-P0_3_5` can prove P0 closeout | Covered by docs diff |
| architecture | active composer still string filtering | blocking | accept | current `turn.rs` implementation remains weak | Not fixed in this review patch; remains P0 implementation blocker | Must be fixed before code-complete/E3 |
| architecture | spawn/node budget not runtime-owned | blocking | accept | current cost summary consumes observability-derived counts | Not fixed in this review patch; remains P0 implementation blocker | Must be fixed before code-complete/E3 |
| architecture | state_commit displacement not runtime-owned | blocking | accept | current cost summary derives from control usage counts | Not fixed in this review patch; remains P0 implementation blocker | Must be fixed before code-complete/E3 |
| architecture | quality impact too coarse | blocking | accept | current runtime quality event lacks real thin/no-spawn/final-abort taxonomy | Not fixed in this review patch; remains P0 implementation blocker | Must be fixed before code-complete/E3 |
| test-validity | missing formal TaskListPath can still allow full_e3 path | blocking | accept | start gate task list row was skipped and gate decision did not force `full_e3_allowed=false` on failed gate | Added formal task list hard fail and made `full_e3_allowed` depend on total gate pass; added test | `test-e3-start-gate.ps1` PASS |
| test-validity | suite receipt did not prove full P0 scheduling | blocking | accept | receipt pass required only one scheduled/completed event | Added receipt formal sample coverage check: all P0 samples scheduled and >=5 completed pairs each; added test | `test-release-decision.ps1` PASS |
| test-validity | direct/walltime over 3x lacks explicit blocker | major | accept | model request had blocker, direct/walltime did not | Added `formal_p0_direct_input_output_ratio_gate_failed` and `formal_p0_walltime_ratio_gate_failed`; added tests | `test-release-decision.ps1` PASS |

### Validation

- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1 -RunRoot target\v005-review-start-gate-selftest-2` -> PASS
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1 -RunRoot target\v005-review-release-selftest-3` -> PASS

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: partial
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - n/a
- Blocking re-review launch records:
  - n/a
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: no

## Final Conclusion

本轮方案审查未通过关闭条件。已修复文档口径和低成本 gate 漏洞，但 v0.0.5 仍存在 runtime producer 阻塞：typed provider-visible composer、runtime-owned spawn/node budget、runtime-owned state_commit displacement、真实质量影响事件。代码完成前仍禁止真实 E3；当前只能继续做非 agent 实现和测试。

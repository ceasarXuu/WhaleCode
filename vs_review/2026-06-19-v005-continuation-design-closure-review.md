# Subagent VS Review: v0.0.5 continuation design closure readiness

- Created: 2026-06-19 15:53:03 +08:00
- Updated: 2026-06-19 15:53:03 +08:00
- Task: 对 v0.0.5 未完成项工程方案执行对抗性审查，判断是否足以进入正式 E3 或版本收口。
- Report path: `vs_review/2026-06-19-v005-continuation-design-closure-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: blocked
- Current commit inspected: `955a9abcc`

## Round 1: 继续开发方案与收口门禁审查

### Review Input

#### Objective

审查 v0.0.5 未完成项工程方案是否已经足以支撑继续开发、低成本诊断、正式 `terminal-bench_E3-P0_3_5` 和版本收口。重点挑战方案是否仍存在“观测替代控制”“脚本自证”“样本口径误判”的风险。

#### Review Target

- v0.0.5 未完成项工程方案。
- 当前 provider lifecycle、active context replacement、budget state、release/start gate 实现状态。
- 实验命名、样本制度、diagnostic-only 与 formal E3 的区分。

#### Target Locations

- `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- `docs/v0.0.5/17-unfinished-work-inventory.md`
- `docs/v0.0.5/13-design-corrections-and-engineering-contract.md`
- `docs/v0.0.5/10-implementation-plan.md`
- `docs/experiments/taskspace-evidence-levels-and-samples.md`
- `docs/v0.0.5/README.md`
- `docs/v0.0.5/16-terminal-bench_E3-P0_3_2-variant-run.md`
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1`
- `scripts/taskspace-benchmark/write-release-decision.ps1`
- `scripts/taskspace-benchmark/test-release-decision.ps1`
- `scripts/taskspace-benchmark/test-e3-start-gate.ps1`
- `third_party/codex-cli/codex-rs/core/src/client.rs`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`

#### Change Introduction

v0.0.5 方案已经把版本状态从“可阶段性收口”改为“继续开发”，并要求在真实 E3 前补齐 provider-owned lifecycle artifacts、active context replacement exact proof、budget quality impact、non-agent gates、code-complete marker、user approval marker 和 release/start gates。

#### Risk Focus

- provider request lifecycle identity 是否仍可能是局部 counter 或后验归因。
- request phase attribution 是否只是统一标签，不能解释成本来源。
- active context replacement 是否真实改变 provider-visible request payload，而不是 marker/report proof。
- budget state machine 是否有 warn、downgrade、hard stop 和质量补偿。
- release/start gate 是否会被 copied JSON、synthetic fixture、自报 run-status 或弱 marker 欺骗。
- `terminal-bench_E3-P0_3_2` 等 diagnostic-only 变体是否可能被误当正式 E3。

#### Verification Status

- 本轮只读审查。
- 未运行真实 E3。
- 未调用真实 agent benchmark。
- 审查基于当前 repo 文件和 subagent 直接读取结果。

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| architecture-adversary | v0.0.5 方案跨 `client.rs`、`session/turn.rs`、`action_map/runtime.rs`、benchmark scripts，核心风险是事实源和控制边界错误。 | provider lifecycle、context replacement、budget state、module ownership |
| test-validity/release-ops adversary | 历史误判来自 E3 口径和 release proof 混淆，必须挑战实验制度和 gate 是否可伪造。 | sample identity、start gate、release decision、artifact provenance |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| architecture-adversary | `multi_agent_v1.spawn_agent` explorer | `019ededc-48f9-7232-a705-09fe7e4d4f38` | spawn/wait tool result in current Codex thread | no | Round 1 architecture review packet | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| test-validity/release-ops adversary | `multi_agent_v1.spawn_agent` explorer | `019ededc-b32d-7f61-94f4-2d1cde8c281a` | spawn/wait tool result in current Codex thread | no | Round 1 experiment/release review packet | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### architecture-adversary

##### Summary

方案正确指出成本控制必须进入 provider-visible execution path，而不是停留在报告层。但当前实现证据仍不满足这个标准：identity、phase attribution、budget state semantics、source-of-truth ownership 都不够 release-grade。

##### Blocking Findings

- Provider request lifecycle identity 不是 release-grade。方案拒绝 `provider-request-{n}`，但代码仍在 `client.rs:286` 和 `client.rs:321` 生成这种局部 id。
- Request phase attribution 只是通用标签。`turn.rs:2140` 硬编码 `request_phase: Some("model_sampling")`，`runtime.rs:1550` 把缺失 phase 默认为 `model_sampling`，会制造虚高 coverage。
- Active context replacement 有真实 composition 边界，但 proof 仍是 marker-based。`turn.rs:442` 和 `turn.rs:452` 说明 provider-visible prompt construction 位置存在；`client.rs:619` 只检查 marker 字符串和若干 legacy 文本缺失，不能证明旧 TaskSpace history 不会通过其他 item 进入请求。
- Budget state machine 不满足方案 contract。当前主要是 count-based allow/block，`runtime.rs:1656` 只在 blocked 时产生 tag-level `hard_stop` quality impact，看不到 first-class normal/warn/downgrade/hard_stop transition。
- Source-of-truth 仍混乱。方案要求脚本不做事实源，但 `write-release-decision.ps1` 仍聚合 marker JSON 和 aggregates 做 release decision。

##### Non-blocking Risks

- WebSocket warmup 在 `client.rs:1838` 被排除在 budget lifecycle 外，方案要求记录为 `startup/warmup` 或显式 exclusion reason。
- `client.rs:624` 把 `protected_items_present` 等同于 `active_projection_present`，不是 semantic protected-evidence verification。
- `runtime.rs:1510` 仍可从 `current_main_node_id` 或 first ready node 选 node，有后验归因风险。

##### Required Fixes

- 用 `session_id + turn_id + logical_request_seq + attempt_seq` 替代 `provider-request-{n}`，并支持 retry/fallback parent linkage。
- request phase 必须由 call-site/context producer 显式传入；缺失时写 `unknown` 并计入 coverage failure。
- 引入真实 budget state machine：`normal`、`warn`、`compact_checkpoint`、`downgrade_thin/no_spawn`、`hard_stop`、`recovery/final_abort`。
- hard stop 必须有显式质量补偿：recovery path、score eligibility、validation skip reason、bounded final behavior，补偿失败不得计 solved。
- release source 必须是 producer-owned typed artifacts；脚本只验证 typed artifacts，不能合成核心事实。

##### Missing Tests

- Retry/fallback identity test。
- Missing phase becomes `unknown` and fails coverage。
- Legacy TaskSpace text enters via tool output/summary/non-marker wording 的 provider payload negative test。
- Budget transition test 覆盖 warn、downgrade、hard_stop、recovery。
- Release rejects marker-only or script-reconstructed evidence without producer-owned lifecycle artifacts。

##### Missing Logs / Observability

- `provider_request_id`、`logical_request_id`、`attempt_id`、`parent_request_id`。
- `budget_state_before`、`budget_state_after`、`budget_transition_reason`。
- `phase_source`、`phase_missing_reason`、`phase_confidence`。
- `payload_scan_scope`。
- `quality_compensation_action`、`recovery_result`、`score_eligible`。

##### Evidence

- `docs/v0.0.5/18-unfinished-work-engineering-design.md:32` - 方案要求 provider lifecycle canonical producer，并禁止 ActionMap 后验推断。
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:33` - active replacement 必须落在 provider-visible composition。
- `docs/v0.0.5/17-unfinished-work-inventory.md:52` - 当前模块仍未阻止 runtime expansion。
- `third_party/codex-cli/codex-rs/core/src/client.rs:1734` - HTTP dispatch 前已有 provider dispatch hook。
- `third_party/codex-cli/codex-rs/core/src/client.rs:1838` - WebSocket warmup 被跳过。
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1668` - budget quality impact 仍主要来自 provider events/tag。
- `scripts/taskspace-benchmark/write-release-decision.ps1:562` - release script 聚合 gate 并决定 release state。

#### test-validity / release-ops adversary

##### Summary

方案比早先误判路径强很多，但仍不能视为 release-grade。最大缺口是 formal release path 仍信任 run directory 内 producer-side JSON/JSONL；hash 多数是自洽性检查，不足以证明产物来自真实 suite runner 或真实 agent execution。

##### Blocking Findings

- Start gate 可能用弱 v0.0.5 markers 授权正式 E3。`lib/e3-start-gate.ps1:173-197` 只要求 non-agent gate 有 `status=pass` 和非空 `evidence_path`，不要求 `evidence_sha256`、`exit_code`、`git_commit`、本地文件存在或 current profile binding。`test-e3-start-gate.ps1:135-151` self-test fixture 使用 `selftest://...` evidence paths。
- Code-complete 和 approval marker 的实现弱于设计。Start gate 对 code-complete 只检查 `git_commit` 和空 `unfinished_p0_items`，不检查 `code_complete=true`、`sample_set_id`、test output paths；approval 不检查 `approval_timestamp`。Release decision 也不检查 approval timestamp。
- Release decision 仍可能被 copied/fabricated internally consistent run tree 欺骗。`test-release-decision.ps1:23-309` 能构造 synthetic artifacts 并在 `test-release-decision.ps1:311-316` 得到 `release_pass`；这说明 gate 接受手工构造的自洽 JSON tree。
- Provider/budget gates 太浅。`write-release-decision.ps1:431` 的 providerRequestPass 只要求至少一个 provider request event schema；`write-release-decision.ps1:432` 的 budgetResponsePass 只要求一个 budget event `status=pass` 或 `budget_response_action_taken`，不能证明所有 accepted pairs / TaskSpace requests / exact run ids 覆盖。

##### Non-blocking Risks

- 命名规范对 formal P0 基本已被 `Get-TaskspaceE3SampleSetDerivation` 强绑定；`terminal-bench_E3-P0_3_5` 才是 formal release proof。
- Suite runner 在 start gate fail 或 `full_e3_allowed=false` 时会在 sample scheduling 前阻断，相关测试已验证无 sample dirs 创建；但这依赖 marker checks 继续增强。

##### Required Fixes

- Start gate marker validation 必须与 release decision 一样严格：每个 non-agent sub-gate 要求 local `evidence_path`、`evidence_sha256`、matching file hash、`exit_code=0`、`git_commit`、`profile_hash`、当前 `task_list_hash/source_version`。
- Start gate 和 release decision 都要要求 `code_complete=true`、`sample_set_id=terminal-bench_E3-P0_3_5`、非空 test output paths、current HEAD。
- Start gate 和 release decision 都要要求并解析 `approval_timestamp`，设置 freshness，并绑定 `approved_sample_set_id`、`task_list_hash`、`profile_hash`、`source_version`。
- 增加 runner-owned immutable receipt：在 scheduling 前写 suite manifest hash，并建立覆盖 `run_initialized`、sample scheduling、pair completion、aggregate、release inputs 的 event-chain hash。

##### Missing Tests

- copied JSON but runner/event-chain receipt absent 的 negative release fixture。
- `v005_non_agent_gates.json` 使用 `selftest://` 或 missing evidence hashes 的 negative start-gate fixture。
- `status=pass` 但 missing `code_complete=true` 的 negative code-complete fixture。
- missing `approval_timestamp` 的 negative approval fixture。
- 一个 fake provider event 但 15 completed pairs 的 negative provider fixture。

##### Missing Logs / Observability

- start-gate audit log：逐字段说明 marker 为什么 pass/fail。
- release-decision 输出每个 non-agent gate 的 verified marker/evidence hashes。
- event-chain hash 或 runner receipt 绑定 scheduling 到 completed artifacts。
- per-pair provider request coverage summary 绑定 accepted pair dirs。

##### Evidence

- `docs/experiments/taskspace-evidence-levels-and-samples.md:82-104` - diagnostic variants 不允许作为 release proof。
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:943-965` - 设计要求 structured markers 和 gate evidence。
- `docs/v0.0.5/README.md:19-21` - v0.0.5 当前不能关闭。
- `docs/v0.0.5/16-terminal-bench_E3-P0_3_2-variant-run.md:3-28` - `_3_2` 是 diagnostic/candidate。
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1:193-233` - suite runner 可在 scheduling 前阻断。
- `scripts/taskspace-benchmark/write-release-decision.ps1:305-400`, `:431-525`, `:560-568` - release identity、provider/budget pass 和 final decision 逻辑。
- `scripts/taskspace-benchmark/test-release-decision.ps1:23-316` - synthetic pass fixture。
- `scripts/taskspace-benchmark/test-e3-start-gate.ps1:203-230`, `:331-347`, `:391-398` - start gate tests。

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| architecture-adversary | Provider request lifecycle identity is not release-grade. | blocking | accept | 当前 `client.rs` 仍存在 `provider-request-{n}`，与 `18` 中 request id 规则冲突。 | 记录为 P0 blocker，不允许正式 E3。 | 实现 session/turn/logical/attempt id，并补 retry/fallback test。 |
| architecture-adversary | Request phase attribution is a generic label. | blocking | accept | `turn.rs` 和 `runtime.rs` 的 `model_sampling` 默认会掩盖 unknown phase。 | 记录为 P0 blocker，不允许正式 E3。 | phase 由 call-site/context producer 传入；unknown 计入 coverage failure。 |
| architecture-adversary | Active context replacement proof is marker-based. | blocking | accept | 目前 scan 不足以证明旧历史不会通过其他 item 进入 provider payload。 | 记录为 P0 blocker，不允许正式 E3。 | exact payload semantic scan 与负例 fixture。 |
| architecture-adversary | Budget state machine is not expressive enough. | blocking | accept | 当前实现以 request count 和 hard_stop tag 为主，未覆盖方案状态机。 | 记录为 P0 blocker，不允许正式 E3。 | 实现 typed transitions 和 quality compensation。 |
| architecture-adversary | Source-of-truth boundaries remain muddled. | blocking | accept | release script 仍承担较多事实聚合；typed artifacts authoritative boundary 不够清晰。 | 记录为 P0 blocker，不允许正式 E3。 | release script 改为验证 producer-owned typed artifacts。 |
| test-validity/release-ops adversary | Start gate can authorize formal E3 from weak markers. | blocking | accept | start gate 早于正式 E3 调度，弱 marker 会直接造成昂贵误跑。 | 记录为 P0 blocker，不允许正式 E3。 | start gate 逐字段校验 evidence/hash/profile/source/task。 |
| test-validity/release-ops adversary | Code-complete and approval marker checks are weaker than design. | blocking | accept | 缺 `code_complete=true`、sample binding、timestamp freshness 等字段校验。 | 记录为 P0 blocker，不允许正式 E3。 | 补 marker schema 和 negative fixtures。 |
| test-validity/release-ops adversary | Release decision can be fooled by copied/fabricated consistent run tree. | blocking | accept | synthetic pass fixture 是合理单测，但同时暴露 release proof 缺少 runner-owned immutable receipt。 | 记录为 P0 blocker，不允许 release_pass。 | 增加 runner event-chain / receipt，并要求 release decision 验证。 |
| test-validity/release-ops adversary | Provider/budget gates are too shallow for release provenance. | blocking | accept | 至少一个 event 不能证明所有 accepted pairs/request 覆盖。 | 记录为 P0 blocker，不允许 release_pass。 | per-pair/provider coverage denominator 与 exact run id 绑定。 |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: no
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - n/a; fixes not implemented in this round
- Blocking re-review launch records:
  - n/a; fixes not implemented in this round
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: yes for implementation of accepted fixes; no for formal E3, no for release closeout

## Final Conclusion

本轮对抗性审查没有通过 v0.0.5 收口门槛。`18-unfinished-work-engineering-design.md` 的方向仍然可作为继续开发入口，但当前方案和实现状态仍有 blocking gaps：

1. provider lifecycle identity / phase attribution / retry attempt lineage 未达到 release-grade。
2. active context replacement proof 仍偏 marker scan，不能证明实际 provider payload 无旧 TaskSpace history 和大 raw output。
3. budget state machine 仍未完整表达 warn、downgrade、hard_stop、quality compensation。
4. start gate 对 non-agent/code-complete/user-approval markers 的调度前校验弱于 release decision 和设计要求。
5. release decision 缺 runner-owned immutable receipt / event-chain，仍可能被 copied internally consistent JSON tree 欺骗。
6. provider/budget gate 需要 per-pair / per-request denominator，不能以至少一个 event 代表整轮覆盖。

结论：可以继续开发 accepted fixes；禁止把当前状态用于正式 `terminal-bench_E3-P0_3_5`，也禁止据此关闭 v0.0.5。

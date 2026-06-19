# Subagent VS Review: v0.0.5 unfinished engineering plan

- Created: 2026-06-19T14:06:34+08:00
- Updated: 2026-06-19T14:31:00+08:00
- Task: 对 v0.0.5 未完成项方案执行对抗性审查，确认方案是否足以继续开发并避免再次误导实验结论。
- Report path: `vs_review/2026-06-19-v005-unfinished-plan-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: blocked

## Round 1: plan viability review

### Review Input

#### Objective
审查 `docs/v0.0.5` 下的当前方案，重点判断 v0.0.5 是否已经把“实际成本控制”从可观测指标推进到可执行工程方案，并且是否能防止再次出现 E3 数据集、样本、repeat 或成功率口径误判。

#### Review Target
设计方案、实验制度、工程实施计划和门禁策略。

#### Target Locations
- `docs/v0.0.5/README.md`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- `docs/v0.0.5/17-unfinished-work-inventory.md`
- `docs/v0.0.5/14-implementation-gap-audit.md`
- `docs/v0.0.5/13-design-corrections-and-engineering-contract.md`
- `docs/v0.0.5/10-implementation-plan.md`
- `docs/v0.0.5/09-e3-validation-plan.md`
- `docs/experiments/`
- `vs_review/2026-06-19-v005-unfinished-plan-review.md`

#### Change Introduction
当前 v0.0.5 已经从早期“阶段性收口”改为“版本不能关闭，继续开发”。方案要求在真实 E3 前补齐成本控制相关代码、非 agent 门禁、预算影响质量判定、实验命名和样本制度，避免把非 terminal-bench 或口径不清的测试误当成 E3 结论。

#### Risk Focus
- 方案是否仍然偏可观测，缺少真正减少 token/时间的控制闭环。
- 是否有明确的 producer、artifact、gate、release blocker 和测试命令，而不是只写目标。
- 是否能防止 `terminal-bench_E3-P0_3_5`、`_3_2`、`_3_1` 等变体混淆，尤其 sample 数和 repeats 数。
- 是否有硬停止、预算跳过、验证跳过后仍算 solved 的质量风险定义。
- 是否能按现有工程状态实际落地，而不是要求一个过大的版本重写。
- 是否有遗漏的非 agent 测试、日志、回放和失败诊断基建。

#### Verification Status
- 本轮只审查方案，不执行 E3，不调用真实 agent benchmark。
- 已知当前方案仍在开发前审查阶段，未声明 v0.0.5 可关闭。
- 请只读检查，不修改文件。

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- 输出必须包含 summary、blocking findings、non-blocking risks、required fixes、missing tests、missing logs/observability、evidence。

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| product-logic-adversary | v0.0.5 的产品目标曾被误收口，需要挑战目标是否真实覆盖成本控制和正确率不下降。 | 产品目标、范围、成功口径 |
| architecture-adversary | 方案涉及 runtime、projection、budget、artifact、release gate 多模块，需要挑战边界和可维护性。 | 架构边界、职责、落地顺序 |
| test-validity-adversary | 历史问题核心是 E3 口径误判，需要挑战实验制度和验证门禁。 | 实验有效性、测试自欺、门禁 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| product-logic-adversary | `multi_agent_v1.spawn_agent` explorer | `019ede7d-db65-7fe3-b75c-03911e708a3c` / Archimedes | spawn_agent result in current Codex thread | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| architecture-adversary | `multi_agent_v1.spawn_agent` explorer | `019ede7e-1537-7d13-a3d3-36b5ca2c479f` / Hume | spawn_agent result in current Codex thread | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| test-validity-adversary | `multi_agent_v1.spawn_agent` explorer | `019ede7e-4bdc-7d03-b82b-0120381a4c1a` / Boyle | spawn_agent result in current Codex thread | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### product-logic-adversary / Archimedes

##### Summary
当前方案已经把“实际成本控制”从观测指标推进到了可执行工程方案，但只是方案层成立，不能说明 v0.0.5 已达成。核心证据是 `18-unfinished-work-engineering-design.md` 明确要求 provider dispatch 前可阻断 budget hook、runtime budget state、active context replacement、spawn/node hard budget、release/start gate，并禁止在 code-complete、non-agent gates、user approval 前跑正式 E3。

##### Blocking Findings
- 无针对当前 canonical 方案的 blocking finding。
- 接近 blocking 的文档治理风险：旧文档仍保留较宽松口径。`09-e3-validation-plan.md:108-116` 仍写 “Engineering success but product partial ... acceptable for v0.0.5”，`10-implementation-plan.md:496-498` 仍有 PASS/PARTIAL/FAIL 旧分类。虽然 `README.md:23-25` 和 `18-unfinished-work-engineering-design.md:26-35` 已声明以 `18` 为准，但这些旧段落如果被单独引用，仍可能复活“partial 可收口”的误判。

##### Non-blocking Risks
- v0.0.5 范围很大：provider hook、session context replacement、runtime budget、spawn gate、release/start gate 都进 P0；实际实现风险高。
- “成本下降且正确率不下降”的口径是可执行的，但仍是 focused P0 proof，不是广泛产品质量证明。
- hard stop 可能降低正确率；实现时必须防止 budget-induced skip 被算 solved。

##### Required Fixes
- 给 `09`、`10` 的旧 PASS/PARTIAL 段落加醒目 superseded banner，或直接改成引用 `18` 的 `release_pass/blocked_partial/fail`。
- release decision 必须强制校验 `sample_set_id=terminal-bench_E3-P0_3_5`、`repeats_per_sample>=5`、每个 counted pair 的 `reported_evidence_level=E3`。
- 正式 E3 runner 必须在 sample scheduling 前阻断 `full_e3_allowed=false`，不能只让 start gate 产出警告。

##### Missing Tests
- provider hook：超预算 request 不发出网络请求。
- runtime budget：normal/warn/downgrade/hard_stop、spawn blocked、legacy action budget。
- active context replacement：synthetic history fixture 证明旧 TaskSpace history 不进 provider-visible input。
- release/start gate negative fixtures：缺 marker、sample set 不匹配、`_3_2` 试图 release_pass、缺 metadata 均 fail。

##### Missing Logs / Observability
- `provider-request-events.jsonl`、`budget-events.jsonl`、payload hash 或 exact pre-redaction scan event。
- request phase attribution、top cost phase、runtime event 与 model request trace id join。
- active replacement report：`legacy_taskspace_history_present`、`raw_output_replay_present`、`projection_over_budget`、`protected_item_missing`。
- budget quality impact：hard stop/thin/no-spawn/validation skip/final abort 对 solve 风险的记录。

##### Evidence
- `docs/v0.0.5/README.md:10-12` - 当前状态明确不能关闭。
- `docs/v0.0.5/README.md:29-32` and `docs/experiments/taskspace-evidence-levels-and-samples.md:98-114` - 禁止误用 `_1_1/_3_1/_3_2`。
- `docs/v0.0.5/17-unfinished-work-inventory.md:35-88` - 成本失控现状。
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:39-58`, `:759-803`, `:886-935` - 从观测转执行闭环。
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:48-66` and `docs/v0.0.5/13-design-corrections-and-engineering-contract.md:50-68` - 成功/失败定义。

#### architecture-adversary / Hume

##### Summary
`18-unfinished-work-engineering-design.md` 方向正确：它定位真实 provider dispatch path，拒绝事后脚本真相，要求 active context replacement 在 request composition 生效，并增加 release/start gate identity checks。但它还没有安全落到当前代码。最大架构缺口是当前 runtime 仍从 ActionMap snapshot 归因 provider request，而 provider lifecycle 真相在 `client.rs` / `ModelClientSession`。

##### Blocking Findings
- Provider request attribution 仍是 snapshot-derived，可能误归因 node/phase。`turn.rs` 在 dispatch 前取 `action_map_provider_request_budget_snapshot()`，之后把 drained provider events 都用同一 snapshot 记录；`runtime.rs` 从 `current_main_node_id` 或 first ready node 选 node，并给每个 event stamped `snapshot.node_id` 和硬编码 `request_phase:model_sampling`。除非 Phase 0A 在 provider request construction time 创建真实 `TaskSpaceProviderRequestContextV1`，否则违反目标。
- Active context replacement 没有被当前 request composition 证明。`turn.rs` 先把 ActionMap developer context 写进 history，再发送 `clone_history().for_prompt(...)`；这只移除旧 projection item，不证明 raw TaskSpace history、大输出 replay 或 stale node history 已从 provider-visible input 中移除。
- `14-implementation-gap-audit.md` 虽被 README supersede，但 stale 到有危险。它仍把 active compact projection 标成 Implemented，把 routing 标成 benchmark-profile contract；需要在文件内加醒目的 historical/stale banner。
- Budget runtime state 太薄。当前只有 `provider_request_count` 和固定 max request，不代表计划里的 `normal -> warned -> compact_checkpoint_required -> thin_downgraded -> hard_stopped` 状态机，也没有 per-node/spawn/legacy/projection counters。

##### Non-blocking Risks
- WebSocket warmup 排除在 inference trace 和 budget dispatch 之外是合理的，但 artifacts 需要 explicit warmup exclusion records。
- Release gate 形状正确，但仍是 script-mediated；producer-owned artifact 规则只有在 artifacts 来自 provider/request path 时才成立。
- Phase 1 budget enforcement 和 Phase 2 active replacement 互相支撑；若先 hard-stop 而未证明 payload replacement，可能减少 request 数但不解决 `avg_input/request`。

##### Required Fixes
- 把 request identity production 移到 provider-visible request boundary：在 `client_session.stream_with_provider_request_budget(...)` 之前产生 `provider_request_id`、`task_id`、`map_id`、`node_id`、`request_phase`、`route_mode`、budget state、context-selection reason。
- 用 dispatch-time request-context object 替代 snapshot attribution；phase 来自 runtime intent，不再硬编码 `model_sampling`。
- 在 provider request construction 内生成 exact payload scan 或 searchable payload artifact，并和 payload hash 绑定。
- `taskspace-v005-active` 必须改变 `for_prompt` input composition，而不是只注入 developer context。
- 在 `14-implementation-gap-audit.md` 文件内标记 historical/stale。

##### Missing Tests
- Provider request attribution test：snapshot 后 `current_main_node_id` 改变，event 仍保留 dispatch-time context。
- Negative active replacement fixture：projection 存在但 raw TaskSpace history 仍在 provider payload，release 必须 fail。
- Hash-only payload fixture：没有 exact scan event 时 release 必须 fail。
- Budget lifecycle fixture：覆盖 warn、compact-required、hard-stop、one allowed recovery request。
- Start gate fixture：`full_e3_allowed=false` 必须在 sample scheduling 前 abort。

##### Missing Logs / Observability
- producer events 生成的 `request_phase_summary.json`。
- `exact-payload-scan-events.jsonl` keyed by `provider_request_id` and `provider_payload_sha256`。
- Budget state transition events：previous/new state、trigger、allowed recovery、quality impact。
- Warmup/startup provider lifecycle events with explicit denominator exclusion。

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:1902-1908`, `:1924-1929`, `:2217-2224` - 当前 provider event 记录依赖 snapshot。
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1493-1516`, `:1545`, `:1577-1584` - 当前 node/phase attribution 的弱点。
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:435-450` - 当前 provider-visible input composition 边界。
- `docs/v0.0.5/14-implementation-gap-audit.md:38`, `:42` - stale implemented 声明。
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:441`, `:497`, `:520`, `:642`, `:663-691`, `:1142-1149` - canonical 方案要求。

#### test-validity-adversary / Boyle

##### Summary
当前方案已经在文档上明确区分：`terminal-bench_E3-P0_3_5` 是 formal release proof，`_1_1/_3_1/_3_2` 是 diagnostic-only，不能支撑 `release_pass`。脚本也有一部分硬门禁：`run-taskspace-e3-suite.ps1` 禁止 scoring 模式下 `-SkipStartGate`，并会在 `full_e3_allowed=false` 时调度样本前退出。但仍不足以防止 E3 误判，核心问题是 release decision 和 non-agent gates 仍过度依赖可伪造 JSON shape，而不是强绑定真实 runner 产物、真实用户批准和真实外部样本执行链路。

##### Blocking Findings
- `write-release-decision.ps1` 的 self-test 证明 synthetic fixture 可以产生 `release_pass`。`test-release-decision.ps1:258-263` 构造 `New-FixtureRun "pass"` 后断言输出 `release_pass`，该 fixture 是脚本内合成的 15 pair 文件和 JSON，不是真实 Terminal-Bench runner。
- release identity gate 只校验 `run-status.json` 自报字段，未把 runner identity 绑定到实际 runner invocation。`write-release-decision.ps1:292-336` 校验的 `sample_set_id`、样本名、`runner_entrypoint`、`repeats_per_sample` 都来自 `run-status.json`；`lib/e3-identity.ps1:22-63` 的 profile hash 未包含 RunnerPath 或 runner file hash。
- non-agent gates 仍主要是 shape gate。start gate 只要求各 gate `status=pass` 和 `evidence_path` 非空；release decision 虽要求 evidence path、command、exit_code、generated_at、git_commit、profile_hash、evidence_sha256 非空，但没有校验 `evidence_sha256` 等于文件实际 hash，也没有要求 gate-level profile/source/task hash 等于当前 run。
- release decision 没有直接验证 user approval marker，而是信任可复制的 gate-decision artifact。

##### Non-blocking Risks
- `run-taskspace-e3-suite.ps1` 允许自定义 `RunnerPath`，但 release decision 只接受自报 `runner_entrypoint="run-taskspace-e3-suite.ps1"`，没有真实 child runner hash。
- `docs/experiments/taskspace-evidence-levels-and-samples.md:80-104` 存在明显编码损坏/乱码，会降低人工审查可靠性。
- `ExpectedSampleSetId` 在 start gate 默认是 `terminal-bench_E3-P0_3_5`，但 suite 没有显式从 task list 推导 sample set id；release 依赖后置 `run-status.json`。

##### Required Fixes
- release decision 必须拒绝 synthetic/internal fixture：要求真实 suite manifest/run-status 由 `run-taskspace-e3-suite.ps1` 生成，并校验 command line、script path、script hash、child runner path/hash、task list hash、sample dirs 和 pair dirs 一一对应。
- non-agent gates 从“字段存在”升级为“证据校验”：重算 `evidence_sha256`，并要求每个 gate 记录且匹配当前 `task_list_hash`、`source_version`、`profile_hash`、`sample_set_id`。
- release decision 直接读取并校验 `v005_user_approval.json`、`v005_code_complete.json`、`v005_non_agent_gates.json`，不能只信任 `gate-decision.json`。
- `runner_profile_hash` 应包含 runner entrypoint、runner file hash、task list path/hash、sample set id、source version、scoring flags。
- `terminal-bench_E3-P0_3_5` 应由 task list 内容反推：必须恰好 3 个正式样本、每个 5 repeats、无 diagnostic-only 标记、无 skipped/excluded pairs。

##### Missing Tests
- synthetic full-shape fixture must fail release decision unless explicitly marked `fixture_test`。
- internal matrix artifacts with `sample_set_id=terminal-bench_E3-P0_3_5` must fail。
- `_1_1/_3_1/_3_2` diagnostic-only run with all other gates passing must fail `release_pass`。
- forged `gate-decision.json` without real approval marker must fail。
- non-agent gate with wrong evidence hash must fail。
- custom `RunnerPath` or changed runner file hash must fail formal E3 identity。
- task list with 3 names but wrong source/task hash must fail。

##### Missing Logs / Observability
- release decision should output `artifact_origin=real_suite|fixture|unknown`。
- record `runner_script_sha256`, `child_runner_sha256`, `task_list_sha256`, `approval_marker_sha256`。
- record `diagnostic_only_rejected_count` and rejected sample set ids。
- record per gate hash verification status, not just pass/fail。

##### Evidence
- `docs/experiments/taskspace-evidence-levels-and-samples.md:86-104` - diagnostic-only docs boundary。
- `docs/experiments/taskspace-evidence-levels-and-samples.md:190-200` - formal sample id docs。
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1:53-64` - suite forbids `SkipStartGate` with scoring。
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1:155-190` - suite blocks scheduling when `full_e3_allowed=false`。
- `scripts/taskspace-benchmark/write-release-decision.ps1:292-336` - release formal identity gate。
- `scripts/taskspace-benchmark/write-release-decision.ps1:463-471` - release pass condition。
- `scripts/taskspace-benchmark/test-release-decision.ps1:258-263` - synthetic pass fixture。

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| product-logic-adversary | `09`/`10` 仍保留旧 PASS/PARTIAL 口径，可能复活 partial 可收口误判。 | major | accept | 旧文档可能被单独引用；与当前 `18` canonical release taxonomy 存在治理风险。 | 本轮记录为必须修正文档治理项。 | 给 `09`、`10` 加 superseded banner 或改写旧 PASS/PARTIAL 段落。 |
| product-logic-adversary | release decision 必须强制校验 formal P0 identity。 | major | accept | 历史误判根因就是样本、repeat、E3 口径混淆；方案要求必须落到脚本 gate。 | 保持为 Phase 5 实施项。 | 在 release/start gate 中补强 sample_set/repeats/reported evidence level 校验及 negative fixtures。 |
| architecture-adversary | Provider request attribution 仍 snapshot-derived，可能误归因 node/phase。 | blocking | accept | 评审引用 `turn.rs` 和 `runtime.rs` 当前代码；这与 Phase 0A dispatch-time context 目标一致，是尚未完成的代码工作。 | 不关闭审查；列为实施 blocker。 | 实现 `TaskSpaceProviderRequestContextV1` dispatch-time context，替代 snapshot 归因，并补测试。 |
| architecture-adversary | Active context replacement 没有被当前 request composition 证明。 | blocking | accept | 当前 `clone_history().for_prompt(...)` 边界仍可能 replay raw history；这正是 v0.0.5 成本根因之一。 | 不关闭审查；列为实施 blocker。 | 在 provider-visible composition 边界实现 active replacement，并以 exact payload scan 验证。 |
| architecture-adversary | `14-implementation-gap-audit.md` stale implemented 声明危险。 | major | accept | README supersession 不足以防止读者直接打开 `14`。 | 列为文档治理修复。 | 给 `14` 加 historical/stale banner，避免作为当前执行依据。 |
| architecture-adversary | Budget runtime state 太薄。 | blocking | accept | 当前 provider request count 不等于计划状态机和 per-node/spawn/legacy/projection counters。 | 不关闭审查；列为实施 blocker。 | 实现预算状态机、budget transition events、quality impact artifact。 |
| test-validity-adversary | synthetic fixture 可以产生 `release_pass`。 | blocking | accept | 这会直接复现“形状完整 artifact 被误当正式结论”的风险。 | 不关闭审查；列为 release gate blocker。 | release decision 增加 artifact origin、real suite provenance、runner/script/task hashes；fixture 只能产生 fixture/test verdict。 |
| test-validity-adversary | release identity gate 只信 `run-status.json` 自报字段。 | blocking | accept | 自报字段不足以证明真实 runner invocation 和 task list。 | 不关闭审查；列为 release gate blocker。 | profile hash 纳入 runner script hash、child runner hash、task list hash、source/scoring flags。 |
| test-validity-adversary | non-agent gates 仍主要是 shape gate。 | blocking | accept | evidence hash 未重算，gate hash 未与当前 run 绑定。 | 不关闭审查；列为 non-agent gate blocker。 | 重算 evidence hash，要求 gate-level hash 与当前 run identity 一致。 |
| test-validity-adversary | release decision 没有直接验证 user approval marker。 | blocking | accept | 只信任 gate-decision artifact 不足以防复制/归档误用。 | 不关闭审查；列为 release gate blocker。 | release decision 直接读取并校验 approval/code-complete/non-agent marker。 |
| test-validity-adversary | experiments 文档局部乱码降低人工审查可靠性。 | minor | accept | 不直接破坏 gate，但影响审查和交接。 | 列为文档清理项。 | 修复 `docs/experiments/taskspace-evidence-levels-and-samples.md` 编码/乱码段落。 |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: no
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - pending after fixes
- Blocking re-review launch records:
  - pending after fixes
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: no for formal E3/release closeout; yes for implementing the accepted fixes

## Final Conclusion

本轮对抗性审查没有通过收口。方案方向可继续开发，但不能把 v0.0.5 视为已完成，也不能运行或引用正式 E3 作为 release proof。

必须先完成以下 blocker，再做 closure re-review：

1. 文档治理：`09`、`10`、`14` 必须明确降级为 historical/superseded，避免旧 PASS/PARTIAL 和 implemented 声明复活。
2. 架构落地：provider request identity/phase/node 归因必须从 dispatch-time request context 产生，不能依赖 ActionMap snapshot。
3. 成本根因：active context replacement 必须在 provider-visible composition 边界生效，并用 exact payload scan 证明。
4. 预算控制：实现 budget runtime state machine、per-node/spawn/legacy/projection counters 和 `BudgetQualityImpactV1`。
5. 实验门禁：release decision 必须绑定真实 suite provenance、runner/script/task hashes、真实 approval/code-complete/non-agent markers，并拒绝 synthetic/internal/diagnostic-only artifact 进入 `release_pass`。

# v0.0.5 方案继续开发对抗性审查

- Created: 2026-06-19T16:20:00+08:00
- Task: 对 v0.0.5 未完成项方案执行新一轮对抗性审查，判断方案是否足以继续开发，以及是否仍存在会误导 E3 / release 判断的方案级问题。
- Report path: `vs_review/2026-06-19-v005-plan-continuation-adversarial-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Real E3 / real agent benchmark: not run
- Status: blocked

## Round 1: continuation plan review

### Review Input

#### Objective

审查 `D:\whalecode-alpha` 的 v0.0.5 未完成项方案是否已经足以继续开发，尤其检查用户要求“v0.0.5 不能关闭、继续开发、完善未完成项设计”之后，方案是否仍有产品目标、架构落地或实验制度 blocker。

#### Review Targets

- `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- `docs/v0.0.5/17-unfinished-work-inventory.md`
- `docs/v0.0.5/10-implementation-plan.md`
- `docs/v0.0.5/09-e3-validation-plan.md`
- `docs/experiments/taskspace-evidence-levels-and-samples.md`
- `scripts/taskspace-benchmark/write-release-decision.ps1`
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1`
- `scripts/taskspace-benchmark/lib/e3-identity.ps1`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1`
- `third_party/codex-cli/codex-rs/core/src/client.rs`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `vs_review/2026-06-19-v005-unfinished-plan-review.md`

#### Risk Focus

- v0.0.5 是否明确包含实际成本控制，而不只是可观测。
- `terminal-bench_E3-P0_3_5` 与 `_1_1/_3_1/_3_2` diagnostic-only 变体是否无法混淆。
- 旧文档是否仍保留 `PARTIAL` 可收口或旧 PASS/PARTIAL/FAIL 口径。
- provider request attribution 是否真正进入 dispatch-time context。
- active context replacement 是否真正落在 provider-visible composition，而不是 projection/report 自证。
- `BudgetQualityImpactV1` 是否来自 runtime canonical producer。
- release/start gate provenance 是否足够防止 synthetic/self-reported artifacts。
- 当前未提交 `turn.rs` 中间态是否构成工程 blocker。

#### Verification Status

- 本轮只读审查。
- 未跑 E3。
- 未调用真实 agent benchmark。
- 当前工作区存在未提交 `third_party/codex-cli/codex-rs/core/src/session/turn.rs` 中间态，reviewer 被要求不要把它当作已完成成果。

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| product-goal-adversary | v0.0.5 曾出现收口口径误判，需要重新挑战产品目标和成功定义。 | 目标、范围、release 口径 |
| architecture-adversary | 方案涉及 provider、runtime、projection、budget、release gate，需要挑战架构事实源。 | runtime producer、上下文替换、预算状态 |
| experiment-validity-adversary | 历史核心问题是 E3 / diagnostic / sample / repeat 混淆。 | 实验制度、门禁、防伪造 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| product-goal-adversary | `multi_agent_v1.spawn_agent` explorer | `019ede9d-1a92-7131-98a8-b711cde5eaa6` / Ampere | spawn_agent result in current Codex thread | no | Round 1 product-goal review packet | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| architecture-adversary | `multi_agent_v1.spawn_agent` explorer | `019ede9d-5abe-7851-970c-cc3853363ceb` / Singer | spawn_agent result in current Codex thread | no | Round 1 architecture review packet | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| experiment-validity-adversary | `multi_agent_v1.spawn_agent` explorer | `019ede9d-9684-7ee1-a5bf-738bb1b44c18` / Halley | spawn_agent result in current Codex thread | no | Round 1 experiment-validity review packet | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

## Reviewer Outputs

### product-goal-adversary / Ampere

#### Summary

方案已经足以继续开发，但还不足以宣称 v0.0.5 可以关闭。当前 canonical 设计在产品目标上基本纠偏：明确要求实际成本控制进入 provider/runtime 执行路径，formal release proof 使用 `terminal-bench_E3-P0_3_5`，diagnostic-only 变体不能进 `release_pass`，并把 `blocked_partial` 定义为不可收口。

#### Blocking Findings

1. 旧文档正文仍可能误导 “PARTIAL 可收口”。`docs/v0.0.5/09-e3-validation-plan.md:80-82` 仍保留 PASS/PARTIAL/FAIL 旧分类，`docs/v0.0.5/09-e3-validation-plan.md:123` 仍写 “acceptable for v0.0.5”。虽然页首 `docs/v0.0.5/09-e3-validation-plan.md:5-8` 已声明 superseded，但正文被单独引用时仍会复活旧口径。`docs/v0.0.5/10-implementation-plan.md:485`、`:492`、`:498-500` 也仍保留 PASS/PARTIAL/FAIL 和 documented PARTIAL 表述，尽管 `:10-12` 有 supersession note。
2. 已记录的对抗审查本身指出 release gate 仍有可伪造 artifact 风险。`vs_review/2026-06-19-v005-unfinished-plan-review.md:160-164` 指出 synthetic fixture、run-status 自报字段、shape gate、未直接校验 user approval marker 等风险。若这些仍未实现修复，方案不能进入正式 E3/release proof，只能继续做代码和非 agent gate 开发。

#### Non-blocking Risks

- 范围大但不必拆目标：provider hook、active context replacement、runtime budget、spawn/node gate、release/start gate 都是 P0，执行风险高，但它们都对应当前成本失控根因，不是任意扩 scope。
- 成本 hard stop 可能损害正确率；`docs/v0.0.5/18-unfinished-work-engineering-design.md:35`、`:365-375`、`:1288` 已要求记录质量影响并禁止把 budget-induced skip/abort 计为 clean success。
- `docs/v0.0.5/17-unfinished-work-inventory.md:59` 仍说正式收口前至少达到 partial gate，否则不能称为阶段性成本成功；这句话本身可接受，但应避免被理解为 partial 可关闭 v0.0.5。

#### Required Fixes

- 把 `09`、`10` 的旧 PASS/PARTIAL 正文改成明确历史段落，或直接替换为 `release_pass / blocked_partial / fail`。
- 按旧审查报告补强 release provenance：真实 suite manifest、runner/script/task hash、sample dirs/pair dirs 对应关系、真实 approval/code-complete/non-agent markers。
- 保持 `docs/v0.0.5/18-unfinished-work-engineering-design.md:717-721` 的口径为唯一收口口径：`blocked_partial.closeable=false`。
- formal P0 release proof 必须继续绑定 `terminal-bench_E3-P0_3_5`，不能用 `_1_1/_3_1/_3_2`。

#### Missing Tests / Logs

- release/start gate negative fixtures：synthetic fixture、internal matrix 伪装 P0、diagnostic-only 全绿、伪造 gate-decision、错误 evidence hash、custom RunnerPath 都必须 fail。
- provider budget 日志：`provider-request-events.jsonl`、`budget-events.jsonl`、request phase attribution、runtime event 与 model request trace join。
- active replacement proof：不能只生成 projection artifact，必须证明 provider-visible input 里旧 TaskSpace history 被替换。

### architecture-adversary / Singer

#### Summary

当前状态仍有架构 blocker，不能进入真实 E3，也不能声明 v0.0.5 工程闭环完成。方案文档方向正确，release/start gate 脚本相较旧审查已有明显补强；但 Rust runtime 落地仍停在中间态，尤其 provider request attribution、active context replacement、runtime budget state / BudgetQualityImpact 都没有达到 `18-unfinished-work-engineering-design.md` 的 canonical producer 要求。

#### Blocking Findings

1. Provider request attribution 仍未从 snapshot-derived 推进到 dispatch-time context。`third_party/codex-cli/codex-rs/core/src/session/turn.rs:1905-1917` 在 provider dispatch 前取 `action_map_provider_request_budget_snapshot()`，然后把 `task_id/map_id/node_id` 和硬编码 `request_phase = "model_sampling"` 塞进 `ProviderRequestAttribution`。`third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1497-1520` 的 snapshot 又来自 `current_main_node_id` 或 first ready node。这仍是“发请求前拍一张 runtime 状态照”，不是每个 provider request construction/dispatch 时生成的 canonical `TaskSpaceProviderRequestContextV1`。
2. Active context replacement 当前代码不是 provider-visible composition 级别的证明，而且未提交改动疑似无法编译。`third_party/codex-cli/codex-rs/core/src/session/turn.rs:436-450` 仍是把 ActionMap developer context 写入 history，然后 `clone_history().for_prompt(...)` 构造输入。未提交 diff 只新增两个 `prepare_provider_visible_prompt_items(...)` 调用，没有对应函数定义。
3. Budget runtime state / BudgetQualityImpact 仍不是 runtime canonical producer。`third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:440-458` 当前 runtime state 只看到 `provider_request_count` 级别字段，没有设计文档要求的 `active_budget / budget_counters / budget_violations / budget_response_state` 状态机。`third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1591-1593` 只在 blocked event tag 上记录 `budget_response_action_taken:true`，没有 runtime 产生的 BudgetQualityImpact canonical event。
4. 当前 `turn.rs` 未提交中间态是明显工程风险。`git status --short` 显示 `M third_party/codex-cli/codex-rs/core/src/session/turn.rs`，且 diff 只新增调用点。

#### Non-blocking Risks

- Release/start gate provenance 有进步：`scripts/taskspace-benchmark/lib/e3-identity.ps1:64-69` 已把 runner entrypoint、runner script hash、child runner hash、task list hash、sample set id 纳入 profile hash；`scripts/taskspace-benchmark/write-release-decision.ps1:361-380` 也要求 `artifact_origin = real_suite` 和 marker hash 匹配。
- 剩余风险是 sample set id 在 `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1:79` 仍由 benchmark + repeats 推导，不是由 task list 内容强校验“恰好 3 个正式样本、每个 5 repeats、无 diagnostic-only”。

#### Required Fixes

- 先修复 `turn.rs` 中间态：补齐或移除 `prepare_provider_visible_prompt_items`，保证 `cargo check/test` 能过。
- 把 provider attribution 改成 dispatch-time request context：request id、phase、node、route、budget state 必须在 request construction boundary 生成。
- active replacement 必须改 `for_prompt` 输入组成，并生成 exact payload scan event，绑定 `request_id + provider_payload_sha256`。
- BudgetQualityImpact 必须由 runtime 在 budget action 发生时产出，不能只由 benchmark/release 脚本汇总推断。
- Formal E3 identity gate 增加 task list 内容校验，而不只信 `sample_set_id` 字段。

#### Missing Tests / Logs

- 编译测试：当前 `turn.rs` 必须先通过 `cargo check -p codex-core`。
- Provider attribution negative test：dispatch 后改变 `current_main_node_id`，event 仍保留 dispatch-time node/phase。
- Active replacement negative fixture：projection 存在但 raw TaskSpace history 仍进 payload 时 release fail。
- Budget lifecycle fixture：warn、compact-required、thin downgrade、hard stop、allowed recovery request。
- `TaskSpaceProviderRequestContextV1` dispatch-time event。
- `exact-payload-scan-events.jsonl`，按 `request_id` 和 `provider_payload_sha256` join。
- runtime budget transition event：previous/new state、trigger、allowed recovery、quality impact。

### experiment-validity-adversary / Halley

#### Summary

v0.0.5 的实验制度比上一轮明显更严格，文档层已经把 `terminal-bench_E3-P0_3_5`、`_3_2/_3_1/_1_1 diagnostic-only`、E1/E2 internal matrix 和 release proof 分开；start gate 也能在 `full_e3_allowed=false` 时于 sample scheduling 前阻断。但还不能判定严格实验制度已建立完成。核心 blocker 仍在 release decision 的 provenance：它校验了很多 marker/hash 字段，但仍主要相信 run root 中的自报 artifact 形状，且 self-test 里手工合成的 “real_suite” fixture 可以产出 `release_pass`。

#### Blocking Findings

1. `release_pass` 仍可由 synthetic fixture 产生，只要伪装成 `artifact_origin=real_suite`。`scripts/taskspace-benchmark/test-release-decision.ps1:288` 构造 `New-FixtureRun "pass"` 后断言 exit 0、`decision=release_pass`、`closeable=true`。该 fixture 手工写入 15 个 pair、`run-status.json`、hash 字段和 marker 文件，并非真实 suite runner 产物。
2. release decision 没有重算 runner/task-list 的真实来源 hash，只校验 run-status 中相关字段是 64 位 hex。`scripts/taskspace-benchmark/write-release-decision.ps1:361` 校验 `runner_script_sha256`、`child_runner_sha256`、`task_list_sha256` 只匹配 hex 格式；只有 approval/code-complete marker hash 会对本地文件重算。
3. `terminal-bench_E3-P0_3_5` 的 sample set identity 仍不是从 task list 强反推。`scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1:79` 只要 `Benchmark=terminal-bench` 且 `Repeats=5` 就设置 `sampleSetId=terminal-bench_E3-P0_3_5`，不验证 task list 恰好是 `processing-pipeline`、`multi-source-data-merger`、`recover-accuracy-log` 三个正式样本。
4. start gate 的 task-list gate 只验证存在性和 source_version，不验证 formal P0 样本集合。`scripts/taskspace-benchmark/lib/e3-start-gate.ps1:229-253` 仅检查 task list 可解析、非空、`task_dir` 存在、source_version 存在。

#### Non-blocking Risks

- 文档层局部乱码严重，尤其 `docs/experiments/taskspace-evidence-levels-and-samples.md` 附近，人工审查成本高，容易误读。
- `09-e3-validation-plan.md` 已加 superseded banner，但正文仍保留旧 PASS/PARTIAL 口径。
- `run-taskspace-e3-suite.ps1` 支持自定义 `RunnerPath`，profile hash 包含 child runner hash，但 release 没有对当前 runner path/hash 做外部重算。

#### Required Fixes

- release decision 必须从真实 suite manifest 或 suite runner 产物重算：runner script hash、child runner hash、task list hash、task list 样本名、task dirs、source version、sample count、repeat count。
- `terminal-bench_E3-P0_3_5` 必须由 task list 内容反推，而不是由 `Benchmark + Repeats` 推导。
- self-test 中 synthetic pass fixture 不应产出正式 `release_pass`；应改为 `fixture_release_pass` 或只验证 gate mechanics，正式 `release_pass` 必须来自真实 suite-produced manifest。
- release decision 必须拒绝 `run-status.json` 自报 `real_suite` 但缺少 suite-runner signed/derived manifest 的结果。

#### Missing Tests / Logs

- 伪造 `run-status.json` 把 `_3_2` 改成 `terminal-bench_E3-P0_3_5`，必须 fail。
- task list 不是三大 P0 样本但 `sample_names` 自报正确，必须 fail。
- `runner_script_sha256` 与当前 `run-taskspace-e3-suite.ps1` 实际 hash 不匹配，必须 fail。
- `child_runner_sha256` 与实际 child runner 不匹配，必须 fail。
- synthetic fixture 即使 `artifact_origin=real_suite` 也必须 fail。
- internal matrix artifact 自报 `benchmark_family=terminal-bench` 必须 fail。
- release decision 输出 `artifact_origin_verification=derived_from_suite_manifest|self_reported|failed`。
- `task_list_sample_set_derivation`：实际 task list 样本、样本数、repeats、excluded/skipped pair 是否进入 formal identity。
- runner hash verification 状态：`runner_script_hash_verified`、`child_runner_hash_verified`、`task_list_hash_verified`。

## Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| product-goal-adversary | `09`/`10` 旧 PASS/PARTIAL 正文仍可能误导 partial 可收口。 | major | accept | 页首 supersession 不足以防止正文被单独引用。 | 本报告记录为未关闭 blocker。 | 改写旧段落为 `release_pass/blocked_partial/fail` 或显式 historical-only。 |
| product-goal-adversary | release gate 仍有可伪造 artifact 风险。 | blocking | accept | 与上一轮审查和本轮实验 reviewer 结论一致。 | 本报告记录为未关闭 blocker。 | 引入 suite-produced manifest / derived provenance。 |
| architecture-adversary | provider request attribution 仍未达到 dispatch-time canonical context。 | blocking | accept | 当前实现仍在 request 前取 snapshot，不能证明每个 provider request 的 phase/node 真实归因。 | 本报告记录为未关闭 blocker。 | 实现 `TaskSpaceProviderRequestContextV1` dispatch-time producer。 |
| architecture-adversary | active context replacement 未在 provider-visible composition 级别闭环，且 `turn.rs` 中间态疑似不可编译。 | blocking | accept | `prepare_provider_visible_prompt_items` 只有调用点没有定义。 | 本报告记录为立即工程 blocker。 | 先修复 `turn.rs` 可编译性，再补 exact payload scan。 |
| architecture-adversary | BudgetQualityImpact 仍不是 runtime canonical producer。 | blocking | accept | release 脚本检查 artifact 不等于 runtime 在预算动作发生时产出事实。 | 本报告记录为未关闭 blocker。 | 在 runtime budget gate 中产出 quality impact event。 |
| experiment-validity-adversary | synthetic fixture 可伪装 `real_suite` 并产出 `release_pass`。 | blocking | accept | `test-release-decision.ps1` 的 pass fixture 仍是手工合成 run tree。 | 本报告记录为未关闭 blocker。 | 正式 `release_pass` 必须依赖 suite-derived manifest；fixture 只能验证机制。 |
| experiment-validity-adversary | release decision 未重算 runner/task-list 真实来源 hash。 | blocking | accept | 当前只校验 hex 形状，不能防伪造。 | 本报告记录为未关闭 blocker。 | release decision 重算 runner、child runner、task list hash。 |
| experiment-validity-adversary | `terminal-bench_E3-P0_3_5` 仍由 benchmark+repeats 推导，不是 task list 反推。 | blocking | accept | 任意 terminal-bench repeats=5 可能被标成 formal P0。 | 本报告记录为未关闭 blocker。 | 增加 formal P0 sample-set derivation gate。 |

## Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed in this round: no
- Blocking re-review completed: no
- Real E3 allowed: no
- Allowed next work: code implementation and non-agent gate/test fixes only

## Final Conclusion

本轮对抗性审查不通过收口。v0.0.5 canonical 方案方向可以继续开发，但仍不能声明工程完成、不能运行真实 E3、不能把任何现有结果作为 release proof。

下一步必须先处理四类 blocker：

1. 文档口径：彻底清理 `09`、`10` 的旧 PASS/PARTIAL 正文。
2. 代码完整性：修复当前 `turn.rs` 中间态，恢复可编译、可测试状态。
3. runtime 事实源：provider request context、active context replacement、BudgetQualityImpact 必须由 runtime/provider-visible 边界产生。
4. 实验防伪：release decision 必须从 suite-produced manifest / task list / runner 文件重算 provenance，禁止 synthetic/self-reported tree 进入 `release_pass`。

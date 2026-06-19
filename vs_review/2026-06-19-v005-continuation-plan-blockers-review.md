# v0.0.5 继续开发方案对抗性审查

- Created: 2026-06-19
- Report schema: adversarial-v1
- Review target: `docs/v0.0.5/17-unfinished-work-inventory.md`, `docs/v0.0.5/18-unfinished-work-engineering-design.md`, provider request trace path, cost/release scripts, E3 gate path
- Review mode: fresh internal subagent plus main-agent adversarial audit
- Source session policy: subagent did not inherit main-agent context
- Subagent: `019ede0a-4876-7a92-8c4a-c36623b97c03` (`Aquinas`)
- Status: blocked - design must be corrected before implementation and before any real E3 run

## Review Input

### Objective

审查 v0.0.5 未完成项工程方案是否足以支撑当前产品目标：

- v0.0.5 必须实现实际成本控制，不只是可观测。
- 代码完成前禁止真实 E3 / 真实 agent benchmark。
- 后续只有在工程门禁通过后，才能执行 `terminal-bench_E3-P0`。

### Target Locations

- `docs/v0.0.5/17-unfinished-work-inventory.md`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- `third_party/codex-cli/codex-rs/core/src/client.rs`
- `third_party/codex-cli/codex-rs/rollout-trace/src/inference.rs`
- `third_party/codex-cli/codex-rs/rollout-trace/src/raw_event.rs`
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1`
- `scripts/taskspace-benchmark/write-release-decision.ps1`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1`

### Risk Focus

- 方案是否仍把事后 artifact 当成 runtime 控制。
- provider request hook 是否真正位于 provider dispatch 前后，并能阻断请求。
- exact payload scan 是否能证明 provider-visible history 被替换。
- release decision 是否仍允许 `PARTIAL` 被误用为可收口状态。
- E3 start gate 是否会绕过 v0.0.5 non-agent gates。

## Reviewer Launch Record

- Reviewer role: independent implementation/readiness adversary
- Mechanism: internal subagent
- Agent id: `019ede0a-4876-7a92-8c4a-c36623b97c03`
- Nickname: `Aquinas`
- Context mode: `fork_context=false`
- Read-only instruction: yes
- Explicitly excluded: main-agent chat history, conclusions, hidden reasoning, implementation drafts
- Input packet: the target locations and risk focus listed above

## Reviewer Output Summary

独立 reviewer 结论：方向正确，但当前方案仍有阻塞缺口，会让 implementation 继续误跑真实 E3，或把 v0.0.5 做成“更强可观测”而不是“实际成本控制”。

## Blocking Findings

### B1. E3 start gate 没有纳入 v0.0.5 禁跑真实 E3 的硬闸门

Evidence:

- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1` 的 start gate 当前主要检查磁盘、Docker、路径、task list、one-pair smoke、calibration、cheap self-tests。
- `docs/v0.0.5/18-unfinished-work-engineering-design.md` 写明 Phase 6 入口需要 Phase 1-5 gates PASS 和用户批准，但代码落点没有明确要求修改 `e3-start-gate.ps1`。

Impact:

真实 E3 仍可能绕过 v0.0.5 runtime gate，被错误启动，从而再次造成实验误判和真实 agent 成本浪费。

Main-agent response: accept.

Required correction:

- 在方案中把 `scripts/taskspace-benchmark/lib/e3-start-gate.ps1` 列为 P0 代码落点。
- `full_e3` 必须依赖 v0.0.5 non-agent gates、code-complete marker、用户批准 marker。
- 缺任何一个 marker 时，只允许 `targeted_diagnostic` 或 `blocked`，不能允许正式 E3。

### B2. 现有 provider trace 是旁路观察，不是 runtime budget 控制点

Evidence:

- `third_party/codex-cli/codex-rs/core/src/client.rs` 中 HTTP 路径在 request 构建后调用 `inference_trace_attempt.record_started(&request)`，随后才发送 provider request。
- WebSocket 路径同样先 `record_started(&ws_request)`，再发送 request。
- `third_party/codex-cli/codex-rs/rollout-trace/src/inference.rs` 的 `record_started` 是 best-effort trace writer；写 payload 失败会直接返回，不会阻断请求。
- `third_party/codex-cli/codex-rs/rollout-trace/src/raw_event.rs` 的 `InferenceStarted` 没有 budget state、request phase、task/node context、阻断结果。

Impact:

如果 implementation 直接复用 rollout trace 作为“provider request hook”，v0.0.5 会继续停留在观测层，无法实现 budget hard stop。

Main-agent response: accept.

Required correction:

- 明确 `InferenceTraceAttempt` 只能作为 exact payload evidence / observability input。
- 新增真正的 provider request budget hook，位于 `client.rs` 的 HTTP/WebSocket provider dispatch 前后，且不能放在 best-effort trace writer 内。
- dispatch 前必须能返回 block/recovery decision；dispatch 后必须更新 budget state 和事件。

### B3. Active replacement proof 设计有报告和测试，但缺少实际替换 provider-visible history 的实现落点

Evidence:

- `docs/v0.0.5/18-unfinished-work-engineering-design.md` Phase 2 的任务集中在 payload capture/reconstruction、report、protected item enumerator、violations。
- 方案没有明确指出要修改哪个 request construction / message history assembly / context surface builder 来省略 raw TaskSpace history。

Impact:

实现可能只做到“发现旧历史叠加并 fail release gate”，而不是完成 v0.0.5 成本控制必需的实际替换。

Main-agent response: accept.

Required correction:

- Phase 2 增加 provider-visible context composition 修改任务。
- 明确 raw TaskSpace control history、completed stale node history、rejected subagent body、large raw output 从 active provider request 中被省略的位置。
- exact payload scanner 只能作为证明，不是替换动作本身。

### B4. Release decision 当前仍允许 `PARTIAL` 作为一级状态

Evidence:

- `scripts/taskspace-benchmark/write-release-decision.ps1` 当前会在 cost status 为 `PARTIAL` 且 blockers 为空时输出 `PARTIAL`。
- 方案要求 taxonomy 改为 `release_pass` / `blocked_partial` / `fail`，且 `blocked_partial` 不可收口。

Impact:

后续报告仍可能把 partial 当作可收口信号，重复 v0.0.5 误判。

Main-agent response: accept.

Required correction:

- `write-release-decision.ps1` 必须输出 `release_pass`、`blocked_partial`、`fail`。
- JSON 中增加 `closeable` 布尔字段。
- `blocked_partial.closeable=false`，且 markdown 文案必须明确“不可用于 v0.0.5 收口”。

## Non-Blocking Risks

### R1. `warn-only` 只能作为调试退路，不能进入 release evidence

Main-agent response: accept.

Correction:

- 在方案中明确 warn-only 不可生成 `release_pass` 或 `blocked_partial`。

### R2. request phase attribution 的时间窗 fallback 不能计入 release coverage

Main-agent response: accept.

Correction:

- phase coverage 只统计 request id / trace id 直接关联结果。
- 时间窗 join 只能作为 diagnostic low-confidence evidence。

## Required Missing Tests

- E3 start gate fixture：缺 v0.0.5 gates 时必须阻止 `full_e3`。
- Provider hook fixture：超过 budget 后，非 recovery provider request 不得发出。
- Exact payload fixture：projection event 存在但 provider payload 仍含 legacy history 时必须 fail。
- Release decision fixture：3x engineering partial 只能生成 `blocked_partial`，且 `closeable=false`。
- Runtime budget logs fixture：必须输出 `budget-events.jsonl`，证明 `budget_violation_detected_during_run` 和 `budget_response_action_taken` 来自执行中事件，不是 post-run 推断。

## Closure Decision

Initial status: not closed.

Accepted blocking findings required a design correction before implementation. After the design correction, a fresh closure review was run focused only on these blockers:

1. E3 start gate is part of Phase 0/5 and blocks formal E3.
2. provider request budget hook is not confused with rollout trace.
3. active replacement includes actual provider-visible context composition changes.
4. release decision taxonomy is corrected to non-closeable `blocked_partial`.

## Round 2: Closure Review

### Reviewer Launch Record

- Reviewer role: closure reviewer
- Mechanism: internal subagent
- Agent id: `019ede0e-d7c9-71a0-ac6f-e87e69196c0f`
- Nickname: `Copernicus`
- Context mode: `fork_context=false`
- Read-only instruction: yes
- Explicitly excluded: main-agent chat history, implementation drafts, hidden reasoning
- Review scope: only the four accepted blocking findings above

### Closure Reviewer Verdict

Verdict: pass.

The reviewer found that the four prior blocking findings are closed at the engineering-plan level:

- `docs/v0.0.5/18-unfinished-work-engineering-design.md` now makes `scripts/taskspace-benchmark/lib/e3-start-gate.ps1` a first-class P0 landing point and requires formal E3 to depend on v0.0.5 non-agent gates, code-complete marker, and user approval marker.
- The plan now separates the real provider request budget hook in `client.rs` from best-effort rollout trace evidence.
- Phase 2 now includes actual provider-visible context composition changes, not only scan/report artifacts.
- Phase 5 now requires `release_pass` / `blocked_partial` / `fail` taxonomy and `blocked_partial.closeable=false`.

### Residual Implementation Risks

The closure pass is plan-level only. Current implementation still needs to catch up:

- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1` still derives `full_e3_allowed` from calibration only.
- `scripts/taskspace-benchmark/write-release-decision.ps1` still emits `PASS/PARTIAL/FAIL`.
- rollout trace remains best-effort evidence and cannot block provider dispatch.
- `client.rs` still needs a real budget hook around HTTP/WebSocket provider dispatch.

### Main-Agent Closure Response

accept.

The design blocker is closed. Implementation may proceed, but formal E3 remains forbidden until the implementation-level gates and review are complete.

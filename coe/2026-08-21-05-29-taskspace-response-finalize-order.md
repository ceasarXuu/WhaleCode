# Problem P-001: Codex 0.147 工具调度抢跑 TaskSpace 响应终结
- Status: open
- Created: 2026-08-21 05:29
- Updated: 2026-08-21 05:35
- Objective: 保证 `taskspace_exec` 仅在当前 Provider response 完成并通过 response scope 校验后开始执行。
- Symptoms:
  - 真实 map-request 首个响应返回一个合法 `taskspace_exec`，运行时先以 `provider response did not complete before TaskSpace Exec` 拒绝，约 2ms 后才记录 response finalized accepted。
- Expected behavior:
  - response scope 先 finalized，再允许 `taskspace_exec` claim 并执行；响应最终可 reconciliation。
- Actual behavior:
  - tool future 构造时立即 `tokio::spawn`，后台 handler 在响应流处理循环完成前抢先 claim；随后 scope 虽成功 finalized，但已无法 reconciliation。
- Impact:
  - TaskSpace 真实 Agent 在第一次外层 Exec 调用即失败，阻塞 0.147 rebase 的 TaskSpace 合入验收。
- Reproduction:
  - 提案 `CBP-818AC8EF81FB25DD` 的 map-request arm，记录 `WAR-20260821-052740-CACHE-REGRESSION-8AF3D2BC-CACHE-002`。
- Environment:
  - Linux；提交 `93ef626dda6254ca94cb39ad266baeef5415f5d4`；`deepseek-v4-flash` Responses API。
- Known facts:
  - Provider response 含一个 `taskspace_exec`，scope 记录同一 outer call id、provider request/response identity，且 finalized accepted。
  - `FuturesOrdered::push_back` 不轮询 future；实际抢跑来自 `ToolCallRuntime::handle_tool_call_with_source` 在返回 future 前立即 `tokio::spawn`。
  - 0.147 已提供精确 runtime 的 `wait_until_ready` 钩子，并在取得工具并行锁前 await。
- Ruled out:
  - Provider wire trace、usage 和 response identity 缺失：本次三者均完整。
  - Provider 返回多次 Exec 或非法顶层 client Tool：scope 记录 `exec_call_count=1` 且 finalized accepted。
- Fix criteria:
  - 根因由真实事件时序和代码调度路径两类独立证据确认。
  - TaskSpace runtime 使用 0.147 readiness 钩子等待本 response scope terminal，不改变 Standard/其他工具调度。
  - integration test 必须覆盖 function call 到达后、response completed 前 spawned task 不得 claim，完成后成功产生 tool output。
  - 定向离线测试和一次预算内 map-request 真实复验通过。
- Current conclusion: H-001 confirmed；应在 TaskSpace runtime readiness 边界做局部同步。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: 0.147 eager tool spawn 绕过 response finalize 顺序
- Status: confirmed
- Parent: P-001
- Claim: `handle_tool_call_with_source` 构造 future 时立即 spawn，TaskSpace handler 因而早于 `scope.finalize` 执行。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 真实日志严格呈现 claim rejected 在前、finalized accepted 在后。
- Falsifiable predictions:
  - If true: tool runtime 在 future 被 drain 前已 spawn，且没有 TaskSpace readiness wait。
  - If false: handler 只会在 `drain_in_flight` 轮询后启动，或已有 finalize barrier。
- Diagnostic evidence plan:
  - Prediction or clause under test: 工具后台任务的实际启动点是否早于 response loop 结束。
  - Signal: spawn、readiness、finalize、drain 的代码顺序及带时间戳真实事件。
  - Capture method: 静态代码路径检查与既有真实运行日志。
  - Event name or marker:
    - `taskspace.exec.rejected`
    - `taskspace.exec.response_finalized`
  - Correlation keys:
    - `call_00_0xHyhfXGPklqS3ONXRph0067`
  - Differentiates from:
    - H-002
    - H-003
  - Supports if:
    - spawn 在 future 构造期发生，且 rejection 时间早于 finalize。
  - Refutes if:
    - spawn 仅在 drain 后发生或 finalize 已先完成。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
- Conclusion: eager spawn 与缺失 TaskSpace readiness barrier 共同造成确定性竞态。
- Repair design readiness: ready
- Next step: 为 TaskSpace runtime 提供只等待当前 response terminal 的 readiness future。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: Provider response identity 绑定晚于 Exec 且最终仍缺失
- Status: refuted
- Parent: P-001
- Claim: Provider trace 适配没有及时或正确绑定 response identity，导致 claim 永远不能成功。
- Layer: interaction
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 本轮刚恢复 provider wire producer，需要排除其与 TaskSpace scope 的交互错误。
- Falsifiable predictions:
  - If true: finalize 事件缺少 request/response identity 或被 contract rejected。
  - If false: finalize accepted 且 identity 完整，只是时序晚于 claim。
- Diagnostic evidence plan:
  - Prediction or clause under test: finalize 的 identity 与 contract 结果。
  - Signal: correlated finalize event。
  - Capture method: 真实 stderr 与 provider wire trace。
  - Event name or marker:
    - `taskspace.exec.response_finalized`
  - Correlation keys:
    - `provider-wire:01a02113-2775-7e12-bce0-fc38f95e0a2a:0:logical-1:attempt-1`
  - Differentiates from:
    - H-001
  - Supports if:
    - identity 为空或 finalize rejected。
  - Refutes if:
    - identity 完整且 accepted。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: finalize identity 完整且 accepted，故 refuted。
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: none
- Blocker:
  - none
- Close reason:
  - E-001 refuted

## Hypothesis H-003: DeepSeek 返回非法多 Exec 或顶层 client Tool
- Status: refuted
- Parent: P-001
- Claim: Provider 输出本身违反 TaskSpace response contract，reconciliation 失败是预期拒绝。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - TaskSpace response contract 会拒绝多 Exec 与逃逸 client Tool。
- Falsifiable predictions:
  - If true: scope finalize 应记录多 Exec/非法 Tool 并 rejected。
  - If false: 仅一个 Exec 且 scope finalized accepted。
- Diagnostic evidence plan:
  - Prediction or clause under test: response item 与 scope cardinality。
  - Signal: rollout function call、finalize exec count/accepted。
  - Capture method: 真实 rollout 与 stderr。
  - Event name or marker:
    - `taskspace.exec.response_finalized`
  - Correlation keys:
    - `call_00_0xHyhfXGPklqS3ONXRph0067`
  - Differentiates from:
    - H-001
  - Supports if:
    - exec count >1、非法 Tool 或 finalize rejected。
  - Refutes if:
    - exec count=1 且 accepted。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: response 仅含一个合法 outer Exec，故 refuted。
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: none
- Blocker:
  - none
- Close reason:
  - E-001 refuted

## Evidence E-001: 真实运行显示 claim 严格早于 finalize
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports
- Type: reproduction
- Source: `target/cache-hit-regression/WAR-20260821-052740-CACHE-REGRESSION-8AF3D2BC/.../pair-001/right/artifacts/whale-exec.stderr.log` 与 `rollout.jsonl`
- Prediction or plan link:
  - 三项假设的真实响应时序与 contract 结果。
- Matched signal:
  - `05:28:21.294628` claim rejected；`05:28:21.296782` 同一 call finalized accepted；rollout 随后写入 rejected tool output 并 turn failed。
- Correlation keys:
  - `call_00_0xHyhfXGPklqS3ONXRph0067`
- Raw content:
  ```text
  21:28:21.294628 response_claim_rejected: provider response did not complete
  21:28:21.296782 response_finalized: exec_call_count=1 accepted=true
  ```
- Interpretation: response 合法且 identity 完整，失败是约 2ms 的运行时竞态而非 Provider contract 错误。
- Time: 2026-08-21 05:35

## Evidence E-002: future 入队不 poll，但 runtime 在构造期立即 spawn
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs`、`core/src/session/turn.rs` 与 futures-util `FuturesOrdered::push_back`
- Prediction or plan link:
  - H-001 的实际启动点预测。
- Matched signal:
  - `handle_output_item_done` 构造 tool future；`handle_tool_call_with_source` 在返回 future 前执行 `tokio::spawn`；`push_back` 明确不 poll；TaskSpace runtime 未实现 `wait_until_ready`；scope 直到 response loop 后才 finalize。
- Correlation keys:
  - upstream vendor commit `8991de284`
- Raw content:
  ```text
  AbortOnDropHandle::new(tokio::spawn(async move { ... dispatch ... }))
  FuturesOrdered::push_back: This function will not call poll
  scope.finalize(...) precedes drain_in_flight(...), but not the eager spawn
  ```
- Interpretation: 0.147 的 runtime readiness hook 是最小且精确的同步点，可保留通用工具 eager dispatch 而只阻止 TaskSpace 抢跑。
- Time: 2026-08-21 05:35

## Evidence E-003: TaskSpace readiness 单元与流式集成回归通过
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `codex-core` 定向 nextest 运行
- Prediction or plan link:
  - P-001 离线修复标准：barrier 必须等待 finalize，且完整 Responses 流须在下一请求收到非拒绝的 Map Tool output。
- Matched signal:
  - readiness 单元测试通过；完整 `taskspace_exec` 集成测试通过；`taskspace_exec` 相关 69 项全部通过。
- Correlation keys:
  - nextest run `27c25a2d-a47b-405f-a293-28f969bb9ec4`
  - nextest run `1f03724d-0b38-41b5-9e40-e0ad7d1378bb`
  - nextest run `c066a44e-5cdb-43f0-99e4-9163e752c289`
- Raw content:
  ```text
  readiness_waits_until_response_is_finalized: 1 passed
  taskspace_exec_waits_for_response_finalization: 1 passed
  taskspace_exec filtered suite: 69 passed, 0 failed
  ```
- Interpretation: 局部 barrier 在 deterministic 测试中消除了 eager spawn 竞态，且未破坏其余 TaskSpace Exec 合同；仍需预算内真实 map-request 复验后关闭问题。
- Time: 2026-08-21 05:43

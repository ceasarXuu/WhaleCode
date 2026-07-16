# Problem P-001: R6 control 失败被性能观察器漏计
- Status: fixed
- Created: 2026-07-16 08:15
- Updated: 2026-07-16 08:20
- Objective: 让成本与性能观察器忠实统计 `TaskSpaceControlResultR6V1` 的 protocol/state/nested failure，不把真实拒绝误报为零。
- Symptoms:
  - R6 complex pair-002 的 live trace 存在一次 `state_machine_failed`，但 `taskspace-control-usage.json` 报告 `control_failure_count=0`、`control_state_failure_count=0`。
- Expected behavior:
  - 同一 control call ID 的失败 output 应按 `status`/`error.class` 计入对应分类。
- Actual behavior:
  - 统计器只认可旧 `TaskSpaceControlResultV1` 与 `TaskSpaceControlResultV2` schema version。
- Impact:
  - E6 报告会把 Agent 的错误操作和 Runtime 的忠实拒绝隐藏，破坏 trace 成本与反馈链判断。
- Reproduction:
  - `target/r6-phase-e/e6-live-path-fix-final/subscription-billing-repair/20260716-080544-210/pair-002/left`。
- Environment:
  - R6 `0ce775278`、Docker hard boundary、current performance observer。
- Known facts:
  - `task-event-95` 是 canonical `function_call_output`，`toolSuccess=false`，body 为合法 `TaskSpaceControlResultR6V1`。
  - body 的 `status=state_machine_failed`、`success=false`、`state_commit=false`。
- Ruled out:
  - rollout 丢失 control output。
  - canonical response-item extractor 没有暴露 output/call ID。
- Fix criteria:
  - fixture 覆盖 R6 typed state failure 并计数为 1。
  - 重放原 pair-002 后 state failure 从 0 修正为 1。
  - V1/V2 protocol/nested 分类保持不变。
- Current conclusion: H-001 已修复并通过 fixture、harness 与 6 个 R6 artifact 离线重放；observer 现在忠实报告 1 次 protocol 和 2 次 state failure。
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - E-003
- Close reason:
  - R6V1 failure classification verified on fixture and captured live traces

## Hypothesis H-001: schema allowlist 缺少 R6V1
- Status: confirmed
- Parent: P-001
- Claim: `cost-instrumentation.ps1` 只有在 schema 为 V1/V2 时才读取 `success=false`，因此完整的 R6V1 failure 被跳过。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 实际 artifact 与代码条件逐字段匹配该机制。
- Falsifiable predictions:
  - If true: output/call ID 均存在，唯一不满足条件的是 schema allowlist；补入 R6V1 后原 artifact 可离线纠正。
  - If false: 补入 R6V1 后仍为零，或 extractor 没有返回 output。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对齐 task-event-94/95 与统计器条件。
  - Signal: call ID、schema_version、status、success 和计数结果。
  - Capture method: jq trace probe、代码检查、fixture 和 artifact 重放。
  - Event name or marker:
    - `task_context_event_recorded`
  - Correlation keys:
    - `call_00_ET_WWY3bZsQ7vQUhqGaK7rM5847`
  - Differentiates from:
    - H-002
  - Supports if:
    - canonical item 完整且只被 schema allowlist 排除。
  - Refutes if:
    - canonical item 本身缺字段。
  - Instrumentation status: permanent regression
  - Instrumentation lifecycle:
    - 保留 R6 fixture。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
- Conclusion: confirmed
- Repair design readiness: ready; Phase E implementation is already authorized
- Next step: closed.
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: canonical response-item 提取丢失 R6 output
- Status: refuted
- Parent: P-001
- Claim: R6 rollout 使用 `task_context_event_recorded` 包装后，observer 没有提取到 function output。
- Layer: alternative
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - 若 output 不可见，schema allowlist 不是充分原因。
- Falsifiable predictions:
  - If true: probe 无法得到 task-event-95 的 output/call ID。
  - If false: canonical payload 含完整 JSON body 和相同 call ID。
- Diagnostic evidence plan:
  - Prediction or clause under test: 直接读取 canonical task event。
  - Signal: eventType、callId、toolSuccess、rawPayload.output。
  - Capture method: jq trace probe。
  - Event name or marker:
    - `task_context_event_recorded`
  - Correlation keys:
    - `task-event-95`
  - Differentiates from:
    - H-001
  - Supports if:
    - output 缺失。
  - Refutes if:
    - output 完整。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: refuted
- Repair design readiness: not applicable
- Next step: closed as alternative.
- Blocker:
  - none
- Close reason:
  - canonical payload is complete

## Evidence E-001: live canonical event 保留完整 R6 failure
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: runtime-trace
- Source: complex pair-002 left rollout
- Prediction or plan link:
  - H-001/H-002 canonical payload prediction。
- Matched signal:
  - task-event-95 的 call ID 与调用一致，`toolSuccess=false`，output 包含 R6V1/state_machine_failed/success=false/state_commit=false。
- Correlation keys:
  - `task-event-95`
  - `call_00_ET_WWY3bZsQ7vQUhqGaK7rM5847`
- Raw content:
  ```text
  TaskSpaceControlResultR6V1 / state_machine_failed / success=false
  ```
- Interpretation: 执行反馈没有丢失，漏计发生在后处理分类。
- Time: 2026-07-16 08:15

## Evidence E-002: observer 只允许旧 V1/V2
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1`
- Prediction or plan link:
  - H-001 schema allowlist prediction。
- Matched signal:
  - `$schemaVersion -in @("TaskSpaceControlResultV1", "TaskSpaceControlResultV2")` 是读取失败状态的前置条件。
- Correlation keys:
  - none
- Raw content:
  ```text
  R6V1 not in failure schema allowlist
  ```
- Interpretation: 这是确定性的版本收敛缺口，不是 Agent 或 Runtime 行为问题。
- Time: 2026-07-16 08:15

## Evidence E-003: R6 fixture 与 6-run artifact 重放纠正失败计数
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: benchmark self-tests and Phase E6 artifacts
- Prediction or plan link:
  - H-001 修复后原 artifact 可离线纠正。
- Matched signal:
  - `test-cost-instrumentation.ps1`、`test-performance-observation.ps1`、`test-harness.ps1` 全部通过。
  - simple pair-001 恢复 1 次 protocol failure：空 `mutate_graph`，反馈明确，后续完成。
  - complex pair-001/pair-002 各恢复 1 次 state failure：未 bind 即 complete，均为 `state_commit=false/partial_commit=0`，Agent 随后 bind/complete。
  - 其余 3 个 R6 run 维持 0 failure；6/6 均成功 `finish_end`。
- Correlation keys:
  - `call_00_u0gP7HPvDTvuZ2dRQQMM7195`
  - `call_00_cjTUyJLA4VLQwuqtQetE0055`
  - `call_00_ET_WWY3bZsQ7vQUhqGaK7rM5847`
- Raw content:
  ```text
  simple:  protocol=1 state=0 nested=0
  complex: protocol=0 state=2 nested=0
  ```
- Interpretation: 统计口径已恢复，三次拒绝都是清晰硬规则反馈且没有 partial commit 或纠错循环。
- Time: 2026-07-16 08:20

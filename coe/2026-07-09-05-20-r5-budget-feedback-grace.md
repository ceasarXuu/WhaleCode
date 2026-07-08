# Problem P-001: R5 PhaseB 工具失败反馈被预算 hard stop 截断
- Status: open
- Created: 2026-07-09 05:20
- Updated: 2026-07-09 05:36
- Objective: 修复 R5 PhaseB 中工具失败反馈已经记录但没有交付给 Agent 修正的预算边界问题。
- Symptoms:
  - `count-call-stack` 三向样本中，R5-B TaskSpace 执行 `apply_patch` 后仍未修改文件，最终验证失败。
  - 日志出现 `TaskSpaceProviderBudgetHardStopV1 reason=provider_request_hard_limit_exceeded request_count=6/6`。
- Expected behavior:
  - 若模型请求已经产生工具调用，工具结果必须被保留并至少有一次机会进入下一次模型请求，由 Agent 自行决定修正、继续或阻塞。
- Actual behavior:
  - `apply_patch` 工具失败反馈被记录为 `node-event-5`，但下一次请求被 hard stop 阻断。
- Impact:
  - R5 PhaseB 反馈层验收失败；TaskSpace 相比 standard 更容易因为预算门截断工具反馈而失败。
- Reproduction:
  - 使用当前 R5-B 构建运行 `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1 -Scenario count-call-stack -Repeats 1 -RunSide both`。
- Environment:
  - branch `whalecode-alpha`，DeepSeek `deepseek-v4-flash`，R5 PhaseB NodeEvent 直接路径改动后。
- Known facts:
  - E-001
  - E-002
  - E-003
  - E-004
  - E-005
  - E-006
  - E-007
  - E-008
- Ruled out:
  - none
- Fix criteria:
  - 工具失败反馈不被 pre-dispatch hard stop 吞掉；相关单元测试通过。
  - `count-call-stack` 单样本重新运行时，R5 当前阶段至少能把失败反馈交付给 Agent 并不再停在同一 hard stop 点。
- Current conclusion: H-001/H-002 已修复并通过单测；live sample 已越过原反馈截断点，但仍暴露 H-003 请求预算生命周期截断。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - H-001 fixed by post-budget grace counting correction
  - H-002 fixed by state-machine rejection feedback follow-up
  - H-003 remains open
- Close reason:
  - not closed

## Hypothesis H-001: budget_recovery 请求过早消耗 post-budget grace
- Status: confirmed
- Parent: P-001
- Claim: 第 6 次请求虽然标记为 `budget_recovery`，但它只是从 `5/6` 到 `6/6`，尚未越过 hard limit；当前实现把它计入 `post_budget_grace_request_count`，导致真正需要把工具失败反馈交给 Agent 的下一次请求被 hard stop 阻断。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - none
- Rationale:
  - R5-B 样本中请求 6 是 implementation 节点第一轮，随后工具失败；如果 post-budget grace 没被提前消耗，下一轮可作为反馈交付请求。
- Falsifiable predictions:
  - If true: 请求 6 的 budget event 应显示 `request_count_before=5 request_count_after=6 max_requests=6 request_phase=budget_recovery`，随后工具失败记录存在，再下一轮 hard stop 显示 `post_budget_grace` 已无可用余量。
  - If false: 请求 6 不应是 pre-limit recovery，或工具失败反馈没有被记录，或 hard stop 与 post-budget grace 无关。
- Diagnostic evidence plan:
  - Prediction or clause under test: 请求 6 pre-limit recovery 被计入 post-budget grace，并导致工具失败反馈无法再触发一次模型请求。
  - Signal: benchmark `provider-request-events.jsonl`、`rollout.jsonl`、TaskSpace snapshot 中的 `node-event-5`。
  - Capture method: 读取 R5-B `count-call-stack` 样本 artifacts。
  - Event name or marker:
    - `TaskSpaceProviderRequestBudgetEventV1`
    - `node-event-5`
    - `TaskSpaceProviderBudgetHardStopV1`
  - Correlation keys:
    - `provider-request:019f4381-b79c-77b2-b3b9-dc78957e0533:logical-6:attempt-1`
    - `taskspace-action-contract-6-apply_patch`
  - Differentiates from:
    - Agent 未产出 patch
    - 工具结果未进入 TaskSpace
    - projection 丢失工具反馈
  - Supports if:
    - 日志同时显示 pre-limit budget recovery、工具失败 NodeEvent、下一轮 hard stop。
  - Refutes if:
    - 任一关键事件缺失，或 hard stop 发生在工具执行前。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: 保持修复；把 H-003 交给后续预算生命周期收敛，不在 PhaseB 放宽状态机。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 请求 6 是 pre-limit budget_recovery
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: `target/r5-phaseB-samples/count-call-stack/20260709-045241-275/pair-001/right/artifacts/provider-request-events.jsonl`
- Prediction or plan link:
  - H-001 If true
- Matched signal:
  - request 6 started with `request_count_before=5`, `request_count_after=6`, `max_requests=6`, `request_phase=budget_recovery`
- Correlation keys:
  - `provider-request:019f4381-b79c-77b2-b3b9-dc78957e0533:logical-6:attempt-1`
- Raw content:
  ```text
  request_phase=budget_recovery request_count_before=5 request_count_after=6 max_requests=6 node_kind=implement_solution
  ```
- Interpretation: 请求 6 到达 hard limit，但不是 post-limit 请求，不应消耗 post-budget grace。
- Time: 2026-07-09 05:20

## Evidence E-004: pre-limit budget_recovery 不再消耗 post-budget grace
- Related hypotheses:
  - H-001
- Direction: supports
- Type: unit-test
- Source: `cargo test -p codex-core post_budget_grace_counter_ignores_pre_limit_budget_recovery_request -- --nocapture`
- Prediction or plan link:
  - H-001 repair validation
- Matched signal:
  - test passed
- Correlation keys:
  - `provider_request_counts_against_post_budget_grace`
- Raw content:
  ```text
  test session::turn::...post_budget_grace_counter_ignores_pre_limit_budget_recovery_request ... ok
  ```
- Interpretation: 只有 `request_count_before >= max_requests` 的 `budget_recovery` 才计入 post-budget grace。
- Time: 2026-07-09 05:28

## Evidence E-005: actionable feedback 临界点会标记下一轮 budget recovery
- Related hypotheses:
  - H-001
- Direction: supports
- Type: unit-test
- Source: `cargo test -p codex-core actionable_feedback_at_rollout_limit_requests_budget_recovery_followup -- --nocapture`
- Prediction or plan link:
  - H-001 repair validation
- Matched signal:
  - test passed
- Correlation keys:
  - `taskspace_feedback_needs_budget_recovery_followup`
- Raw content:
  ```text
  actionable_feedback_at_rollout_limit_requests_budget_recovery_followup ... ok
  ```
- Interpretation: 工具反馈类 actionable output 在 rollout limit 点不会被立刻 hard stop 吞掉。
- Time: 2026-07-09 05:29

## Hypothesis H-002: 状态机硬拒绝反馈在预算临界点也会被截断
- Status: fixed
- Parent: P-001
- Claim: 即使没有 ordinary tool 结果，只要 runtime 产生 `TaskSpaceActionV1 rejected` 等硬底线反馈，也必须给 Agent 一次接收反馈的机会；否则 Agent 无法基于状态机错误修正动作。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - H-001
- Rationale:
  - R5-B second rerun 中 inspect 节点尝试 `apply_patch` 被正确拒绝，但拒绝反馈后立即 hard stop。
- Falsifiable predictions:
  - If true: rejection message 后跟随 `TaskSpaceProviderBudgetHardStopV1`，没有下一轮 provider request。
  - If false: 拒绝反馈已进入下一轮，Agent 能进行状态转移或合法动作。
- Diagnostic evidence plan:
  - Prediction or clause under test: 硬拒绝反馈本身需要 budget recovery follow-up。
  - Signal: `TaskSpaceActionV1 rejected`、`TaskSpaceProviderBudgetHardStopV1`、下一轮 request。
  - Capture method: 读取 R5-B second/third rerun artifacts。
  - Event name or marker:
    - `TaskSpaceActionV1 rejected`
    - `taskspace_feedback_after_provider_budget_limit`
  - Correlation keys:
    - `node_policy_violation:inspect_code_context:apply_patch`
  - Differentiates from:
    - 放宽 inspect edit 权限
    - semantic recovery guidance
  - Supports if:
    - 单测覆盖 hard rejection feedback follow-up，third rerun 不再停在同一拒绝点。
  - Refutes if:
    - hard rejection 后仍无后续 request。
  - Instrumentation status: existing
  - Instrumentation lifecycle:
    - provider request events
- Evidence gate: satisfied
- Related evidence:
  - E-006
  - E-007
- Conclusion: fixed
- Repair design readiness: implemented
- Next step: 保持状态机拒绝不放宽；只保留反馈投递窗口。
- Blocker:
  - none
- Close reason:
  - fixed by `taskspace_feedback_needs_budget_recovery_followup`

## Evidence E-006: hard rejection feedback follow-up 单测通过
- Related hypotheses:
  - H-002
- Direction: supports
- Type: unit-test
- Source: `cargo test -p codex-core state_machine_rejection_at_rollout_limit_requests_budget_recovery_followup -- --nocapture`
- Prediction or plan link:
  - H-002 repair validation
- Matched signal:
  - test passed
- Correlation keys:
  - `TaskSpaceActionV1 rejected`
  - `taskspace_message_has_state_machine_rejection`
- Raw content:
  ```text
  state_machine_rejection_at_rollout_limit_requests_budget_recovery_followup ... ok
  ```
- Interpretation: 状态机拒绝反馈能触发一次 budget recovery follow-up。
- Time: 2026-07-09 05:30

## Evidence E-007: third rerun 不再停在状态机拒绝点
- Related hypotheses:
  - H-002
- Direction: supports
- Type: benchmark-log
- Source: `target/r5-phaseB-samples-after-rejection-grace/count-call-stack/20260709-051151-818/pair-001/right/artifacts/whale-exec.jsonl`
- Prediction or plan link:
  - H-002 live validation
- Matched signal:
  - Agent 在 request_count=7/6 输出 `finish_node`，进入 `implement_solution`
- Correlation keys:
  - `node-1`
  - `node-2`
- Raw content:
  ```text
  action=finish_node node_id=node-1 next_node_kind=implement_solution
  ```
- Interpretation: 拒绝/反馈临界点后，Agent 可以继续推进状态，不再在同一 hard stop 点失败。
- Time: 2026-07-09 05:35

## Hypothesis H-003: verification_first 全局请求预算会截断多节点生命周期
- Status: open
- Parent: P-001
- Claim: `verification_first` 的 `max_rollout_model_requests=6` 对 inspect -> implement 的 TaskSpace 生命周期过紧；当 Agent 在 inspect 中完成必要读取后才创建 implement node，runtime 会在 implement 第一轮模型请求前 hard stop。
- Layer: contributing-factor
- Factor relation: all_of
- Depends on:
  - H-001
  - H-002
- Rationale:
  - 第三次 rerun 中上下文反馈没有丢失，Agent 也识别根因并完成状态转移，但 patch 没有机会执行。
- Falsifiable predictions:
  - If true: 日志显示 `finish_node` 已创建 `implement_solution`，随后 `TaskSpaceProviderBudgetHardStopV1 request_count=7/6 node_kind=implement_solution`。
  - If false: implement 节点内应至少有一次 provider request 或 tool call。
- Diagnostic evidence plan:
  - Prediction or clause under test: 状态转移后 hard stop 发生在 implement 节点第一轮 request 前。
  - Signal: `finish_node` followed by hard stop with `node_request_count=0/2`.
  - Capture method: third rerun artifacts。
  - Event name or marker:
    - `finish_node`
    - `TaskSpaceProviderBudgetHardStopV1`
  - Correlation keys:
    - `request_count=7/6`
    - `node_kind=implement_solution`
  - Differentiates from:
    - NodeEvent 丢失
    - projection 丢失工具反馈
    - 状态机拒绝 apply_patch
  - Supports if:
    - implement node exists but `changed_paths` 为空且没有 patch tool call。
  - Refutes if:
    - implement 节点有合法 patch tool call。
  - Instrumentation status: existing
  - Instrumentation lifecycle:
    - benchmark artifacts
- Evidence gate: satisfied
- Related evidence:
  - E-008
- Conclusion: open
- Repair design readiness: not ready
- Next step: 在后续预算生命周期收敛中处理，优先保证资源硬底线不截断已完成状态转移；不得新增语义策略约束。
- Blocker:
  - 需要和 R5-C/E 的 projection/gate 简化边界对齐。
- Close reason:
  - not closed

## Evidence E-008: implement 节点创建后被 hard stop
- Related hypotheses:
  - H-003
- Direction: supports
- Type: benchmark-log
- Source: `target/r5-phaseB-samples-after-rejection-grace/count-call-stack/20260709-051151-818/pair-001/right/artifacts/whale-exec.jsonl`
- Prediction or plan link:
  - H-003 If true
- Matched signal:
  - `finish_node` 后 `TaskSpaceProviderBudgetHardStopV1 request_count=7/6 node_request_count=0/2 node_kind=implement_solution`
- Correlation keys:
  - `node_kind=implement_solution`
  - `request_count=7/6`
- Raw content:
  ```text
  TaskSpaceProviderBudgetHardStopV1 reason=provider_request_hard_limit_exceeded request_count=7/6 node_request_count=0/2 state=over_profile_hint node_kind=implement_solution phase=model_sampling
  ```
- Interpretation: PhaseB 反馈承载路径已越过原失败点；当前 blocker 是预算 hard baseline 对新节点第一轮执行的截断。
- Time: 2026-07-09 05:36

## Evidence E-002: 工具失败反馈已进入 NodeEvent
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: `target/r5-phaseB-samples/count-call-stack/20260709-045241-275/pair-001/right/artifacts/rollout.jsonl`
- Prediction or plan link:
  - H-001 If true
- Matched signal:
  - `node-event-5` recorded for `taskspace-action-contract-6-apply_patch`
- Correlation keys:
  - `node-event-5`
  - `taskspace-action-contract-6-apply_patch`
- Raw content:
  ```text
  nodeEventId=node-event-5 actionClass=edit toolSuccess=false
  apply_patch verification failed: Failed to find expected lines in src/call_stack_counter.py
  ```
- Interpretation: 反馈层保存了失败语义；问题不是工具结果丢失，而是后续交付被预算门截断。
- Time: 2026-07-09 05:20

## Evidence E-003: 下一轮被 hard stop 阻断
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: `target/r5-phaseB-samples/count-call-stack/20260709-045241-275/pair-001/right/artifacts/whale-exec.jsonl`
- Prediction or plan link:
  - H-001 If true
- Matched signal:
  - `TaskSpaceProviderBudgetHardStopV1`
- Correlation keys:
  - `node-2`
- Raw content:
  ```text
  TaskSpaceProviderBudgetHardStopV1 reason=provider_request_hard_limit_exceeded request_count=6/6 node_request_count=1/2 state=over_profile_hint node_kind=implement_solution phase=model_sampling
  ```
- Interpretation: hard stop 发生在工具失败反馈记录之后、Agent 下一次看到反馈之前。
- Time: 2026-07-09 05:20

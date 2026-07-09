# Problem P-001: R5 PhaseB 工具失败反馈被预算 hard stop 截断
- Status: fixed
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
  - E-009
  - E-010
  - E-011
  - E-012
  - E-013
- Ruled out:
  - none
- Fix criteria:
  - 工具失败反馈不被 pre-dispatch hard stop 吞掉；相关单元测试通过。
  - `count-call-stack` 单样本重新运行时，R5 当前阶段至少能把失败反馈交付给 Agent 并不再停在同一 hard stop 点。
- Current conclusion: H-001/H-002 已修复并通过单测；C0 修复 H-003/H-004 的预算账本边界；H-005 修复 action-contract patch 语法归一化缺陷；`count-call-stack` 复验 standard/R5 均 solved。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
- Resolution basis:
  - H-001 fixed by post-budget grace counting correction
  - H-002 fixed by state-machine rejection feedback follow-up
  - H-003 fixed by fresh executable node first-request grace
  - H-004 fixed by excluding fresh executable node first request from post-budget feedback grace accounting
  - H-005 fixed by stripping trailing-only `*** End Patch` before unified diff normalization
- Close reason:
  - fixed by H-001/H-002/H-003/H-004/H-005 with E-013 live validation

## Hypothesis H-001: budget_recovery 请求过早消耗 post-budget grace
- Status: fixed
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
- Conclusion: fixed
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
- Status: fixed
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
- Conclusion: fixed
- Repair design readiness: implemented
- Next step: 保持修复限定在硬预算账本；不得让 runtime 替 Agent 选择下一步动作。
- Blocker:
  - none
- Close reason:
  - fixed by `provider_fresh_node_first_request_grace`

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

## Evidence E-009: fresh executable node first-request grace 单测通过
- Related hypotheses:
  - H-003
- Direction: supports
- Type: unit-test
- Source: `cargo test -p codex-core taskspace_active_budget_allows_fresh_executable_node_first_request -- --nocapture`
- Prediction or plan link:
  - H-003 repair validation
- Matched signal:
  - test passed
- Correlation keys:
  - `provider_fresh_node_first_request_grace`
  - `node_kind=implement_solution`
- Raw content:
  ```text
  taskspace_active_budget_allows_fresh_executable_node_first_request ... ok
  ```
- Interpretation: 已完成状态转移后的可执行新节点，即使全局 rollout 请求数超过 profile hint，也不会在首轮模型请求前被 hard stop 截断。
- Time: 2026-07-09 07:19

## Hypothesis H-004: fresh executable node 首轮请求错误占用 post-budget feedback grace
- Status: fixed
- Parent: P-001
- Claim: C0 初步放开 implement 节点首轮请求后，该请求仍被标记为 `budget_recovery`，并在 `request_count_before >= max_requests` 时消耗唯一的 post-budget grace，导致随后真实的工具失败反馈没有下一轮交付窗口。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - H-003
- Rationale:
  - `count-call-stack` C0 样本中 implement 首轮请求已经发出，Agent 输出 `apply_patch`，工具返回 `apply_patch verification failed`；紧接着 request_count=7/6、node_request_count=1/2 被 hard stop。
- Falsifiable predictions:
  - If true: fresh implement 首轮请求的 provider budget event 显示 `request_phase=budget_recovery request_count_before=6 max_requests=6 node_request_count=1`，随后工具失败反馈存在，再下一轮 hard stop。
  - If false: fresh 首轮请求不消耗 post-budget grace，工具失败反馈应能触发一次后续请求，或 hard stop 与 post-budget grace 无关。
- Diagnostic evidence plan:
  - Prediction or clause under test: fresh executable node first request 不应计入 post-budget feedback grace。
  - Signal: provider request budget trace、tool failure stderr、hard stop warning、unit test counter。
  - Capture method: 读取 C0 sample artifacts 并补 focused unit test。
  - Event name or marker:
    - `TaskSpaceProviderRequestBudgetEventV1`
    - `fresh_node_first_request_grace:true`
    - `post_budget_grace_counted:false`
    - `TaskSpaceProviderBudgetHardStopV1`
  - Correlation keys:
    - `provider-request:019f43ec-a421-7030-93ec-7d088d282252:logical-7:attempt-1`
    - `node_kind=implement_solution`
  - Differentiates from:
    - projection 语义丢失
    - runtime 自动修补 patch
    - 放宽工具语法
  - Supports if:
    - 单测证明 fresh first request 不增加 `post_budget_grace_request_count`，同节点后续 budget recovery 才增加。
  - Refutes if:
    - fresh first request 后 counter 仍为 1，或后续 feedback request 仍无法被计入/触发。
  - Instrumentation status: existing plus permanent trace tag
  - Instrumentation lifecycle:
    - `fresh_node_first_request_grace:true` 和 `post_budget_grace_counted:false` 保留为预算账本观测标签。
- Evidence gate: satisfied
- Related evidence:
  - E-010
  - E-011
- Conclusion: fixed
- Repair design readiness: implemented
- Next step: 保持 fresh first request 与 feedback grace 分账；后续 Phase C 继续处理 action-contract 一步一请求的结构性成本。
- Blocker:
  - none
- Close reason:
  - fixed by excluding fresh executable node first request from post-budget grace accounting.

## Evidence E-010: fresh first request 不再扣 post-budget grace
- Related hypotheses:
  - H-004
- Direction: supports
- Type: unit-test
- Source: `cargo test -p codex-core post_budget_grace_counter_ignores_fresh_executable_node_first_request -- --nocapture`
- Prediction or plan link:
  - H-004 repair validation
- Matched signal:
  - test passed
- Correlation keys:
  - `fresh_node_first_request_grace:true`
  - `post_budget_grace_counted:false`
  - `post_budget_grace_request_count`
- Raw content:
  ```text
  post_budget_grace_counter_ignores_fresh_executable_node_first_request ... ok
  ```
- Interpretation: budget accounting 已将“新可执行节点首轮请求”与“工具失败反馈交付请求”拆成两个不同窗口；后者仍受 post-budget grace 硬限制。
- Time: 2026-07-09 07:19

## Evidence E-011: C0 首轮复验越过原 hard stop，但暴露 patch trailing End 归一化缺陷
- Related hypotheses:
  - H-004
  - H-005
- Direction: supports
- Type: benchmark-log
- Source: `target/r5c0runs2/count-call-stack/20260709-070112-493/pair-001/right/artifacts/whale-exec.jsonl`
- Prediction or plan link:
  - H-004 live validation
  - H-005 If true
- Matched signal:
  - implement 节点获得首轮 read 和后续 apply_patch 请求；patch 工具反馈进入 `node-event-7`；失败为 `invalid hunk ... *** End Patch`，不是原 implement 首轮 hard stop。
- Correlation keys:
  - `node-event-7`
  - `request_count=8/6`
  - `node_request_count=2/2`
- Raw content:
  ```text
  action=read_file node_id=node-2 path=./src/call_stack_counter.py
  action=apply_patch node_id=node-2
  apply_patch verification failed: invalid hunk at line 14, '*** End Patch' is not a valid hunk header
  ```
- Interpretation: H-003/H-004 的预算窗口修复推进了失败点；新的 blocker 是 action-contract patch 归一化把缺失 Begin、带尾部 End 的 unified diff 变成双 End。
- Time: 2026-07-09 07:03

## Hypothesis H-005: action-contract patch 归一化把 trailing-only End Patch 复制进 hunk
- Status: fixed
- Parent: P-001
- Claim: `normalize_taskspace_unified_diff_patch` 只在 patch 首尾同时是 `*** Begin Patch` / `*** End Patch` 时剥外壳；当 Agent 输出缺少 Begin 但带尾部 End 的 unified/native 混合 patch 时，normalizer 会把尾部 `*** End Patch` 当作 hunk 内容复制，再追加一个新的 End，导致 apply_patch 工具失败。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - H-004
- Rationale:
  - C0 首轮复验中 action-contract 输出的 patch string 没有 `*** Begin Patch`，但有 `*** End Patch`；工具执行预览显示两个 `*** End Patch`。
- Falsifiable predictions:
  - If true: 对样本 patch 直接调用 `taskspace_action_to_tool_call` 会产生只含一个尾部 End 的 native payload；修复前会产生双 End 或工具失败。
  - If false: 双 End 来自 apply_patch runtime 或 provider 输出本身，而不是 normalizer。
- Diagnostic evidence plan:
  - Prediction or clause under test: trailing-only End Patch 必须在 unified diff normalization 前剥离。
  - Signal: focused unit test payload assertions。
  - Capture method: 增加样本形态单测并运行。
  - Event name or marker:
    - `taskspace_action_contract_normalizes_unified_diff_with_trailing_end_only`
  - Correlation keys:
    - `*** End Patch` count
    - `--- a/src/call_stack_counter.py`
  - Differentiates from:
    - Agent 选择了错误目标
    - apply_patch 工具本身损坏
    - projection 丢失文件内容
  - Supports if:
    - normalized payload 以 Begin 开头、以单个 End 结尾，且移除 unified file headers。
  - Refutes if:
    - normalized payload 仍有两个 End 或保留 `--- a/...`。
  - Instrumentation status: unit test
  - Instrumentation lifecycle:
    - permanent regression test
- Evidence gate: satisfied
- Related evidence:
  - E-012
  - E-013
- Conclusion: fixed
- Repair design readiness: implemented
- Next step: 后续 Phase C 继续收敛 transport/tool loop，而不是通过 projection 强提示 patch 格式。
- Blocker:
  - none
- Close reason:
  - fixed by stripping trailing-only `*** End Patch` before unified diff normalization.

## Evidence E-012: trailing-only End Patch 归一化单测通过
- Related hypotheses:
  - H-005
- Direction: supports
- Type: unit-test
- Source: `cargo test -p codex-core taskspace_action_contract_normalizes_unified_diff_with_trailing_end_only -- --nocapture`
- Prediction or plan link:
  - H-005 repair validation
- Matched signal:
  - test passed
- Correlation keys:
  - `normalize_taskspace_unified_diff_patch`
  - `taskspace_action_to_tool_call`
- Raw content:
  ```text
  taskspace_action_contract_normalizes_unified_diff_with_trailing_end_only ... ok
  ```
- Interpretation: action-contract patch converter 不再把孤立尾部 `*** End Patch` 复制进 hunk 内容。
- Time: 2026-07-09 07:05

## Evidence E-013: C0 二次复验 standard/R5 均 solved
- Related hypotheses:
  - H-003
  - H-004
  - H-005
- Direction: supports
- Type: benchmark-log
- Source: `target/r5c0runs3/count-call-stack/20260709-070533-898/pair-001/pair-report.md`
- Prediction or plan link:
  - C0 live validation
- Matched signal:
  - `outcome_standard: solved`
  - `outcome_taskspace: solved`
  - right `business_success: True`
  - right `public_validation_exit_code: 0`
  - right `hidden_oracle_exit_code: 0`
- Correlation keys:
  - `target/r5c0runs3/count-call-stack/20260709-070533-898`
  - `src/call_stack_counter.py`
- Raw content:
  ```text
  utility_direction: both_success
  outcome_standard: solved
  outcome_taskspace: solved
  right / taskspace business_success: True
  changed_paths: src/call_stack_counter.py
  ```
- Interpretation: C0 修复关闭了本 case 的 live sample gate；该 run 为 `Repeats=1` 诊断证据，不计入 aggregate utility。
- Time: 2026-07-09 07:08

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

## Hypothesis H-006: action-contract 单动作承载放大 provider request
- Status: fixed
- Parent: P-001
- Claim: TaskSpace action-contract 原先一次 provider response 只能承载一个 action，导致无依赖读文件、测试/结束等连续动作被拆成多个 provider request；这不是 Agent 智能问题，也不是 runtime 应该强行替 Agent 决策的问题，而是 transport 承载能力不足。
- Layer: contributing-factor
- Factor relation: all_of
- Depends on:
  - H-003
  - H-004
- Evidence gate: satisfied
- Related evidence:
  - E-014
  - E-021
- Conclusion: fixed
- Repair design readiness: implemented
- Repair:
  - 增加 `taskspace-action-sequence-v1`，最多承载 8 个 Agent 明确动作。
  - runtime 按顺序执行，不合并、不重排、不生成动作。
  - 遇到 action 拒绝、edit/test 失败、final/blocked 或 gate recovery 即停止当前 sequence。
- Close reason:
  - fixed by action sequence parser/executor and sequence-indexed call ids.

## Hypothesis H-007: active projection marker mismatch 导致薄投影未替换
- Status: fixed
- Parent: P-001
- Claim: R5-C 初跑中 active projection 已生成，但 context compiler 仍依赖旧 `active compact profile` marker，导致 provider-visible payload 没有用最新 thin projection 替换旧上下文，历史 bootstrap/status 内容继续污染上下文。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - H-006
- Evidence gate: satisfied
- Related evidence:
  - E-015
  - E-016
- Conclusion: fixed
- Repair:
  - active replacement 改为结构识别 `ContextProjectionV1 active replacement:`。
  - 增加测试 `active_context_replacement_recognizes_thin_projection_without_legacy_profile_marker`。
- Close reason:
  - fixed by structural active projection detection.

## Hypothesis H-008: sequence 内失败后继续执行后续动作会扭曲反馈
- Status: fixed
- Parent: P-001
- Claim: action sequence 若在 `apply_patch` 或 `run_test` 失败后继续执行后续动作，会让 Agent 收到混合反馈并误以为后续步骤也有执行意义；runtime 应停止当前 Agent 给出的 sequence，但不替 Agent 决定下一步。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - H-006
- Evidence gate: satisfied
- Related evidence:
  - E-017
  - E-018
- Conclusion: fixed
- Repair:
  - 增加 `taskspace_sequence_failure_feedback_from_response_item`。
  - `apply_patch verification failed` 或 `Exit code:` 非零测试反馈后停止 sequence。
- Close reason:
  - fixed by sequence fail-stop on failed edit/test.

## Hypothesis H-009: 状态机拒绝反馈只在 trace 中可见，未进入 active projection
- Status: fixed
- Parent: P-001
- Claim: inspect 节点尝试 edit 被正确拒绝，但拒绝语义只在 provider_response_actionability/whale-exec trace 中可见，未作为 node-local feedback 进入 active projection，Agent 下一轮容易重复或误判。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - H-007
- Evidence gate: satisfied
- Related evidence:
  - E-019
  - E-021
- Conclusion: fixed
- Repair:
  - 新增 `runtime_feedback` node event。
  - action-contract parse/reject branch 调用 `record_action_map_runtime_feedback`。
  - active projection recent events 显示 `source=runtime_feedback`。
- Close reason:
  - fixed by runtime feedback node event path.

## Hypothesis H-010: prose/f-string braces 抢占 JSON 起点导致合法 action sequence 解析失败
- Status: fixed
- Parent: P-001
- Claim: Agent 输出合法 action sequence 前若有 prose 或 f-string 示例 `{count_stack_depth()}`，旧 parser 会从第一个 `{` 开始解析，返回 malformed JSON；这是 parser 起点选择缺陷，不应通过提示 Agent 不写 prose 解决。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - H-006
- Evidence gate: satisfied
- Related evidence:
  - E-020
- Conclusion: fixed
- Repair:
  - 增加 `taskspace_prefixed_action_json_start`，只接受首字段为 `schema_version` 的 JSON object 起点。
  - 增加测试 `taskspace_action_contract_parser_recovers_sequence_after_prose_with_braces`。
- Close reason:
  - fixed by schema-version anchored JSON start scanning.

## Hypothesis H-011: unified diff hunk context 过脆导致正确 patch 被 native apply_patch 拒绝
- Status: fixed
- Parent: P-001
- Claim: Agent 给出的 unified diff hunk header 带行号/函数上下文且包含多余空行上下文，native apply_patch 对这些上下文要求更严格，导致语义正确的 patch 失败。
- Layer: capability-gap
- Factor relation: all_of
- Depends on:
  - H-006
- Evidence gate: satisfied
- Related evidence:
  - E-018
  - E-021
- Conclusion: fixed
- Repair:
  - unified hunk header 归一为 native `@@`。
  - update hunk 最小化为最近稳定锚点和 change lines，避免函数上下文/空行数量误差。
- Close reason:
  - fixed by native patch hunk normalization.

## Hypothesis H-012: benchmark routing prompt 仍在 model-visible 注入策略
- Status: fixed
- Parent: P-001
- Claim: `TaskShapeRouterV1` 虽然标记为 report-only，但 benchmark 仍把 routing constraints 追加到 TaskSpace prompt，造成 Phase C active projection 之外的 model-visible 策略注入。
- Layer: boundary-violation
- Factor relation: all_of
- Depends on:
  - H-007
- Evidence gate: satisfied
- Related evidence:
  - E-021
- Conclusion: fixed
- Repair:
  - `New-TaskspaceRoutingPrompt` 保留函数但返回空串。
  - routing decision 继续写 artifact/report，不再进入模型上下文。
  - PowerShell harness tests 改为防回归断言 report-only。
- Close reason:
  - fixed by report-only routing prompt.

## Hypothesis H-013: bootstrap context 仍暴露旧 compact/cognitive 语义要求
- Status: fixed
- Parent: P-001
- Claim: 即使 active projection 已 thin，TaskSpace bootstrap/transition context 仍出现 `active compact profile`、`cognitive_preflight_requirement`、`result_validity_requirement` 等旧语义要求，违反 R5 的薄构造器边界。
- Layer: boundary-violation
- Factor relation: all_of
- Depends on:
  - H-012
- Evidence gate: satisfied
- Related evidence:
  - E-021
- Conclusion: fixed
- Repair:
  - bootstrap 文案改为 `TaskSpace v0.0.5 thin bootstrap`。
  - transition notice 只声明 task path/current node 硬入口和 map runtime boundary。
  - 移除旧 compact/result-validity/cognitive preflight model-visible 文案。
- Close reason:
  - fixed by thin bootstrap and transition notice cleanup.

## Hypothesis H-014: patch 成功后 Agent 仍消耗预算重试环境不可用测试
- Status: open
- Parent: P-001
- Claim: R5-C 最新样本中 patch 已正确落地且 public/hidden validation 均通过，但 Agent 仍反复尝试环境不可用的测试路径并最终触发 provider budget hard stop；这是残余反馈使用/执行节奏效率问题，不应通过 runtime 语义约束禁止 Agent 行为。
- Layer: residual-efficiency
- Factor relation: all_of
- Depends on:
  - H-006
  - H-009
- Evidence gate: satisfied
- Related evidence:
  - E-021
- Conclusion: open
- Repair design readiness: not-ready
- Next step:
  - 后续优先检查 test feedback 的上下文呈现、native tool-loop carrier、以及环境/validator 反馈的效率，不新增 runtime 策略性 hard-stop。
- Blocker:
  - none

## Evidence E-014: action sequence parser/executor focused tests passed
- Related hypotheses:
  - H-006
- Direction: supports
- Type: unit-test
- Source: `cargo test -p codex-core taskspace_action_contract -- --nocapture`
- Matched signal:
  - 82 passed
- Raw content:
  ```text
  taskspace_action_contract_parser_accepts_action_sequence ... ok
  taskspace_action_contract_sequence_call_ids_are_unique ... ok
  ```
- Interpretation: action-contract 可解析 sequence，并为 sequence 内 action 生成唯一 call id。
- Time: 2026-07-09 18:30

## Evidence E-015: C phase1 暴露 active projection replacement marker mismatch
- Related hypotheses:
  - H-007
- Direction: supports
- Type: benchmark-log
- Source: `target/r5cphase1/count-call-stack/20260709-175326-337`
- Matched signal:
  - `active_projection_present=false`、`context_bundle=false`、`legacy_taskspace_history_present=true`
- Interpretation: thin projection 未被 context compiler 识别为 active replacement，旧历史污染 provider payload。
- Time: 2026-07-09 17:53

## Evidence E-016: structural active projection replacement focused test passed
- Related hypotheses:
  - H-007
- Direction: supports
- Type: unit-test
- Source: `cargo test -p codex-core active_projection -- --nocapture`
- Matched signal:
  - 11 passed
- Interpretation: active projection replacement 不再依赖旧 compact marker，并保留反馈 excerpt/ref。
- Time: 2026-07-09 18:30

## Evidence E-017: failed edit/test sequence fail-stop focused test passed
- Related hypotheses:
  - H-008
- Direction: supports
- Type: unit-test
- Source: `cargo test -p codex-core taskspace_action_contract -- --nocapture`
- Matched signal:
  - `action_sequence_failure_feedback_detects_failed_edit_and_test ... ok`
- Interpretation: sequence 内失败反馈可被识别，后续依赖动作不继续执行。
- Time: 2026-07-09 18:30

## Evidence E-018: C phase2 暴露 patch hunk 归一化和失败后续执行问题
- Related hypotheses:
  - H-008
  - H-011
- Direction: supports
- Type: benchmark-log
- Source: `target/r5cphase2/count-call-stack/20260709-180007-217`
- Matched signal:
  - Agent 输出 `apply_patch + run_test + validate` sequence；`apply_patch` 因 hunk context/header 失败，旧路径仍继续执行后续动作。
- Interpretation: failure fail-stop 与 unified hunk normalization 都是能力/反馈层问题，不应通过 projection 提示修复。
- Time: 2026-07-09 18:00

## Evidence E-019: runtime feedback projection focused test passed
- Related hypotheses:
  - H-009
- Direction: supports
- Type: unit-test
- Source: `cargo test -p codex-core runtime_feedback_records_node_event_and_active_projection -- --nocapture`
- Matched signal:
  - test passed
- Interpretation: 状态机拒绝/parse 拒绝反馈可进入 node event，并在 active projection 可见。
- Time: 2026-07-09 18:30

## Evidence E-020: prose braces parser recovery focused test passed
- Related hypotheses:
  - H-010
- Direction: supports
- Type: unit-test
- Source: `cargo test -p codex-core taskspace_action_contract_parser_recovers_sequence_after_prose_with_braces -- --nocapture`
- Matched signal:
  - test passed
- Interpretation: parser 不再被 action JSON 前的 f-string braces 抢占。
- Time: 2026-07-09 18:30

## Evidence E-021: R5-C live sample standard/R5 both solved with old prompt hits removed
- Related hypotheses:
  - H-006
  - H-009
  - H-011
  - H-012
  - H-013
  - H-014
- Direction: supports
- Type: benchmark-log
- Source: `target/r5cphase6/count-call-stack/20260709-183144-389/pair-001/pair-report.md`
- Matched signal:
  - `outcome_standard: solved`
  - `outcome_taskspace: solved`
  - `taskspace_tool_call_ratio: 1`
  - right `public_validation_exit_code: 0`
  - right `hidden_oracle_exit_code: 0`
  - old routing/compact prompt hits: 0
- Correlation keys:
  - `target/r5cphase6/count-call-stack/20260709-183144-389`
  - `src/call_stack_counter.py`
- Raw content:
  ```text
  standard current: solved, 15135ms, 10 tools
  R5-C current: solved, 45228ms, 10 tools
  right rollout_trace.model_request_count: 8
  agent_messages: 8
  agent_actions: 15
  multi_action_messages: 4
  TaskShapeRouterV1 active profile constraints: 0
  active compact profile: 0
  cognitive_preflight_requirement: 0
  ```
- Interpretation: Phase C 修复通过 targeted live gate。残余 hard stop 发生在 patch 正确落地且 external validation 通过之后，归入 H-014 后续效率问题。
- Time: 2026-07-09 18:32

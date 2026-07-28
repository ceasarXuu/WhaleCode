# Problem P-001: 普通 Tool 结果提交后的 canonical revision 未进入 Agent 反馈
- Status: open
- Created: 2026-07-29 04:11
- Updated: 2026-07-29 04:56
- Objective: 让一次合法 TaskSpace response 完成后，Agent 忠实获得下一次控制操作所需的最终 canonical revision
- Symptoms:
  - `taskspace_control` 成功返回 `revision_after=N`
  - 同 response 的每个普通 Tool 结果释放 reservation 时继续提交 Map，使实际 revision 变成 `N+K`
  - 普通 Tool 输出只保留原生结果，不携带最终 revision；下一次 `execute/finish_map` 稳定提交旧 revision
  - A2-C 51 个状态失败 request 中 36 个是 `stale_revision`
- Expected behavior:
  - 原生普通 Tool 结果保持完整、无扭曲
  - Runtime 自己产生的 Map 状态变化有对应的机械事实反馈
  - 下一 provider request 能看到完整 response 执行后的唯一 canonical revision
- Actual behavior:
  - control result 只描述 reservation prepare 后的中间 revision
  - result attribution 的后续 revision 只进入 Store/日志，没有进入 provider-visible feedback
- Impact:
  - 每次成功的工作 response 很容易紧跟一次 stale reject 和一次纠正重试
  - request、input、output 和 wall time 被结构性放大
  - Agent 被误判为不能管理 revision，实际是反馈缺失
- Reproduction:
  - 检查 A2-C 任一成功 `initialize_and_execute/execute + ordinary sibling` response
  - 对照 control output、普通 Tool outputs、下一次 submitted revision 和 Store revision
- Environment:
  - source commit `abe2b872b6708e666293d0018ecd3654bf5a65cc`
  - run root `target/r7-five-layer-matrix/a2-c/abe2b872b/20260729-0315`
- Known facts:
  - `prepare_response_for_main` 先提交 reservations 并生成 `TaskSpaceResponseCommitV1`
  - `release_main_action_result` 对每个 ordinary result 再提交一次 canonical Map
  - `record_taskspace_bound_tool_result` 返回 `Result<(), String>`，不把最终 revision 返回 sequence
  - ordinary Tool 的 model-visible output 不包含 TaskSpace revision
- Ruled out:
  - Agent 忽略普通 Tool 输出中的 revision；输出中不存在该字段
  - 单一 projection policy；三种 policy 都复现，且一个 user turn 内 projection 不随每次 provider loop 重建
  - Store revision 未推进；事务测试和导出 Map 均证明 result attribution 会推进 revision
- Fix criteria:
  - control prepare revision 与 response-final canonical revision 语义明确分离
  - ordinary Tool 原生输出字节和语义保持不变
  - response 完成后 Agent 可直接获得最终 canonical revision，无需先触发 stale reject
  - 三种 projection policy 共用同一反馈实现
  - A2-C repeat-3 中由隐藏 result attribution 引起的 stale retry 为零
- Current conclusion: 这是反馈层语义缺失，不是 Agent 智能问题；修复应在 response 执行完成时补充机械 commit receipt，不应让 Runtime替 Agent 选择动作
- Related hypotheses:
  - H-001
- Resolution basis:
  - Store-backed `TaskSpaceResponseFinalReceiptV1` 工程修复已实施
  - prepare revision、普通 Tool 输出和 response-final revision 保持三个独立事实
  - 定向事务与 sequence 回归通过，A2-C live rerun pending
- Close reason:
  - not closed

## Hypothesis H-001: control output 暴露了中间 revision，result attribution 的最终 revision 未暴露
- Status: confirmed
- Parent: P-001
- Claim: sequence 在 sibling 执行前构造 control output，而 sibling result release 继续修改 canonical Map；最终 revision 没有返回 model-visible result
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - stale submitted revision 等于前一 control output，canonical revision 等于该 response 全部结果提交后的 revision
- Falsifiable predictions:
  - If true: 同 response 中 control `revision_after=N`，K 个 sibling 完成后 Store revision 至少为 `N+K`，下一请求仍提交 N
  - If false: 普通 Tool feedback 已携带最终 revision，或 result release 不推进 Map
- Diagnostic evidence plan:
  - Prediction or clause under test: 对齐一次完整 response 的 control、siblings、Store 和下一 request
  - Signal: revision_before/revision_after、reservation count、result release commits、submitted_expected_revision
  - Capture method: raw rollout + canonical Store export + production call graph
  - Event name or marker:
    - `TaskSpaceResponseCommitV1`
    - `taskspace_native_tool_result_attributed`
    - `taskspace_response_preflight_rejected`
  - Correlation keys:
    - control call_id
    - reservation_id
    - map_id
  - Differentiates from:
    - Agent 自行选择错误 revision
  - Supports if:
    - 最终 revision 只存在于 Store/日志
  - Refutes if:
    - provider-visible feedback 已提供最终 revision
  - Instrumentation status: existing-observability-sufficient
  - Instrumentation lifecycle:
    - 修复后保留 response-final revision 和 attribution count
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: production call graph 和 live trace 一致确认
- Repair design readiness: implemented
- Next step: 重跑 A2-C，确认隐藏 result attribution 引起的 stale retry 为零
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 成功初始化后的下一控制请求使用了不可避免的旧 revision
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: A2-C `single-file-fast-fix / map-always / repeat 1`
- Prediction or plan link:
  - H-001 的完整 response revision 预测
- Matched signal:
  - control 返回 revision 1，一个 ordinary result 后 canonical revision 为 2；下一 execute 提交 1 并被拒绝
- Correlation keys:
  - map `map-019faa26-f0f3-7732-abf2-68615563a309`
- Raw content:
  ```text
initialize_and_execute: revision_before=0, revision_after=1
ordinary result: native output only
next execute: expected_revision=1
reject: current_revision=2, violation=stale_revision
  ```
- Interpretation: Agent 使用了它最后被明确告知的 revision
- Time: 2026-07-29 04:11

## Evidence E-002: production result attribution 推进 Map 但不返回 revision
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source:
  - `core/src/tools/sequence.rs`
  - `core/src/tools/parallel.rs`
  - `core/src/session/taskspace_response.rs`
  - `core/src/action_map/runtime/transactions.rs`
- Prediction or plan link:
  - H-001 的 call graph 预测
- Matched signal:
  - control output 在 sibling dispatch 前由 `prepared.model_visible_result()` 构造
  - 每个 sibling 通过 `release_main_action_result` 提交 result ref
  - record API 仅返回 `Result<(), String>`
- Correlation keys:
  - `ActionMapPreparedResponse.revision_after`
  - `release_action_reservation`
- Raw content:
  ```text
outputs = [control_output]
handle_taskspace_bound_tool_call_for_sequence(...)
record_taskspace_bound_tool_result(...) -> Result<(), String>
release_reservation(expected_revision = current map revision)
  ```
- Interpretation: Runtime 产生了 Agent 不可见的后续 canonical revision
- Time: 2026-07-29 04:11

## Evidence E-003: response-final canonical revision 已作为独立事实返回
- Related hypotheses:
  - H-001
- Direction: supports
- Type: test
- Source:
  - `core/src/action_map/response.rs`
  - `core/src/action_map/runtime/transactions.rs`
  - `core/src/tools/sequence.rs`
  - `core/src/tools/sequence_taskspace_tests.rs`
- Prediction or plan link:
  - P-001 Fix criteria
- Matched signal:
  - sibling result attribution 全部结束后从 persistent Map Store 读取 canonical revision
  - developer factual message 追加 `TaskSpaceResponseFinalReceiptV1`
  - 下一次 execute 使用回执 revision 可直接提交
  - 普通 Tool 原生 output 不被包装或改写
- Correlation keys:
  - map_id
  - control_call_id
  - reservation_revision_after
  - canonical_revision
- Raw content:
  ```text
tools::sequence::taskspace_tests: 5 passed
response_final_receipt_revision_is_accepted_by_the_next_execute: passed
taskspace_response_final_receipt_emitted
codex-core --lib with repository .env.local: 1915 passed, 3 ignored
  ```
- Interpretation: 反馈缺失已在工程路径补齐；产品问题保持 open，直到 live trace 证明 stale amplification 消失
- Time: 2026-07-29 04:56

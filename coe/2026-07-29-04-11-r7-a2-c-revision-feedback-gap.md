# Problem P-001: 普通 Tool 结果提交后的 canonical revision 未进入 Agent 反馈
- Status: open
- Created: 2026-07-29 04:11
- Updated: 2026-07-29 06:20
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
- Current conclusion: 原始根因仍是反馈层语义缺失，不是 Agent 智能问题；但把 receipt 作为独立 developer/system
  历史项追加是错误载体，会确定性破坏 DeepSeek 缓存。最终事实必须回到同一 `taskspace_control` 调用的结果语义，
  不应让 Runtime 替 Agent 选择动作，也不应新增中途 system 消息
- Related hypotheses:
  - H-001
- Resolution basis:
  - Store-backed `TaskSpaceResponseFinalReceiptV1` 工程修复已实施
  - prepare revision、普通 Tool 输出和 response-final revision 保持三个独立事实
  - 定向事务与 sequence 回归通过
  - A2-C live rerun 证明独立 developer/system receipt carrier 确定性破坏缓存，产品修复失败
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
- Next step: 保留 Store-backed final revision，重构为同一 control call 的最终 Tool result，再重跑 A2-C
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
- Interpretation: 反馈缺失已在工程路径补齐，但本证据只验证值可用，没有验证 carrier 对 Provider 缓存和 wire
  角色的影响；后续 live evidence 已证明当前 carrier 不合格
- Time: 2026-07-29 04:56

## Hypothesis H-002: 独立 developer receipt 在 DeepSeek wire 上变成中途 system 消息并破坏缓存
- Status: confirmed
- Parent: P-001
- Claim: sequence 在 Tool siblings 后追加 developer factual message；DeepSeek wire 将其表示为历史中途的 system
  message，导致下一请求只复用基础缓存前缀
- Layer: repair-regression
- Factor relation: dependent_on
- Depends on:
  - H-001
- Rationale:
  - 修复前 `map-append` 保持约 95% request-2+ cache；修复后 exact message prefix 仍保持，但每个 receipt 后缓存
    稳定退回约 7K 以下
- Falsifiable predictions:
  - If true: 紧跟 receipt 的请求稳定低缓存，不紧跟 receipt 的同一 run 请求恢复高缓存
  - If false: 缓存低点与 receipt 无时间关联，或 Tool hash/tool choice/message prefix 同时变化
- Diagnostic evidence plan:
  - Prediction or clause under test: 在保持 projection policy、Tool hash、tool choice 和消息前缀不变时，对齐 receipt
    与下一 token_count
  - Signal: receipt-before、input、cached input、wire role、prefix preservation
  - Capture method: A2-C repair rerun raw rollout + provider wire trace
  - Event name or marker:
    - `TaskSpaceResponseFinalReceiptV1`
    - `token_count`
    - `provider_wire_request`
  - Correlation keys:
    - run path
    - request index
    - control_call_id
  - Differentiates from:
    - `map-append` projection 累加的正常 input 成本
    - Tool schema/tool_choice 动态变化
    - 单次 Provider 冷启动
  - Supports if:
    - receipt-before 请求系统性低缓存且无 receipt 请求高缓存
  - Refutes if:
    - 两组命中无显著路径差异
  - Instrumentation status: existing-raw-evidence-sufficient / aggregate-report-gap
  - Instrumentation lifecycle:
    - 后续矩阵报告永久增加 receipt-before cache 分组
- Evidence gate: satisfied
- Related evidence:
  - E-004
  - E-005
- Conclusion: 35/35 与 1/39 的分组结果及 wire role 共同确认
- Repair design readiness: ready
- Next step: 设计同一 control call 的最终 Tool result carrier；实施前保持五层职责和普通 Tool 零侵入
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-004: map-append receipt-before 与缓存坍缩完全对齐
- Related hypotheses:
  - H-002
- Direction: supports
- Type: reproduction
- Source: A2-C repair rerun，全部 6 个 `map-append` runs
- Prediction or plan link:
  - H-002 的 receipt 时间关联预测
- Matched signal:
  - receipt-before：35 requests，35 个 cached input <= 7,000，命中率 23.64%
  - no-receipt-before：39 requests，仅 1 个 cached input <= 7,000，命中率 93.69%
- Correlation keys:
  - run root `target/r7-five-layer-matrix/a2-c-repair/445499582/20260729-0546`
- Raw content:
  ```text
receipt_before=false requests=39 collapse=1 input=860042 cached=805760 hit=93.69%
receipt_before=true  requests=35 collapse=35 input=901614 cached=213120 hit=23.64%
  ```
- Interpretation: 同一 policy、同一 binary、同一矩阵内的请求级对照排除了普通 projection 累加和随机离群
- Time: 2026-07-29 06:20

## Evidence E-005: wire 保持前缀和 Tool 身份，但 receipt 作为 system 历史出现
- Related hypotheses:
  - H-002
- Direction: supports
- Type: diagnostic-log
- Source: A2-C repair rerun provider wire trace
- Prediction or plan link:
  - H-002 的 role 与 prefix 判别
- Matched signal:
  - `prefix_preserved=true`
  - `message_prefix_preserved=true`
  - `tool_choice_changed=false`
  - `tools_hash` 不变
  - `TaskSpaceResponseFinalReceiptV1` 对应 message role 为 `system`
- Correlation keys:
  - `single-file-fast-fix / map-append / repeat 1`
  - receipt control call IDs
- Raw content:
  ```text
request 4: prefix_preserved=true, cached=5888
message[14].role=system, content=TaskSpaceResponseFinalReceiptV1
request 5 without new receipt: cached=15744
request 6 after new receipt: cached=5888
  ```
- Interpretation: 缓存回归来自 receipt carrier，而不是 Tool schema 或自然历史前缀重写
- Time: 2026-07-29 06:20

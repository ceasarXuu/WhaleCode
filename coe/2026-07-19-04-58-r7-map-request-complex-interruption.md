# Problem P-001: R7 map-request 复杂样本执行后未闭合 Map 并中断
- Status: investigating
- Created: 2026-07-19 04:58
- Updated: 2026-07-19 04:58
- Objective: 证明复杂样本在 map-request 下只初始化 Map、持续执行普通工具却未闭合并中断的具体因果机制。
- Symptoms:
  - `subscription-billing-repair` 的 Standard 臂 solved，R7 map-request 臂执行 18 个 provider request 和 34 个普通工具尝试后为 `interrupted`。
  - TaskSpace 只调用一次 `initialize_map`，没有后续 control 或 `read_map`，最终 Map 保留 1 个 open leaf。
- Expected behavior:
  - Agent 可自行决定何时读取 Map，但 TaskSpace 硬门禁不可绕过；任务完成时应由 Agent 显式闭合节点并提交 `finish_end`。
- Actual behavior:
  - Agent 在初始化后完成大量普通工具动作，但没有再次维护 Map，也没有产生合法终局。
- Impact:
  - R7 Phase D 的复杂样本退出门禁未通过，不能据此宣布 Phase D 完成。
- Reproduction:
  - Docker hard boundary，`deepseek-v4-flash`，reasoning max，`taskspace_projection_policy="map-request"`，运行 `subscription-billing-repair` 1 次。
- Environment:
  - Linux；branch `whalecode-alpha`；commit `8202c3a1a`；run `target/r7-phase-d/request/complex/subscription-billing-repair/20260719-045442-647`。
- Known facts:
  - 两臂 harness 有效；Standard solved；TaskSpace interrupted。
  - TaskSpace 首次 control 初始化成功，自动 provider projection 最大值为 0，普通工具前有合法 binding。
  - TaskSpace provider message prefix preservation 为 100%，不是已知的 projection 替换缓存断裂。
  - 第 18 次 provider request 正常完成；Agent 返回普通最终总结后，本地 R6 终局门禁产生 `taskspace_terminal_protocol_violation` 并以 exit 1 结束。
  - `plain_provider_final_is_nonterminal_and_does_not_retry` 集成测试明确冻结了“普通 final 非终局且不重试”的行为。
- Ruled out:
  - provider、timeout 或预算先行中断。
- Fix criteria:
  - 根因通过最后 provider response、interrupt source、终局判定和 Map trace 的一致证据确认。
  - 修复后同一复杂样本至少 1 次 solved、Map 闭合，且 Phase D 自动 projection 和硬门禁合同不回归。
- Current conclusion: 直接失败机制已确认：Agent 未闭合 Map 就输出普通 final，R6 冻结终局门禁按设计 fatal 且不发起纠正请求。map-request 缺少持续可见当前 Map 是否导致 Agent 遗忘，仍只有相关性证据，不能据此设计修复。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: 外部预算或 provider 中断终止了仍在正常推进的 Agent
- Status: refuted
- Parent: P-001
- Claim: Agent 尚未尝试结束任务时，provider、预算或执行器中断先终止了会话，因此 Map 未闭合只是中断结果。
- Layer: root-cause
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - completion 状态为 `interrupted` 而非普通失败，且执行了 18 个 provider request。
- Falsifiable predictions:
  - If true: trace 中应存在明确 interrupt/timeout/budget/provider terminal 信号，最后 response 不包含终局意图或被 runtime 拒绝的终局动作。
  - If false: 最后 response 已表达终局或工具调用已完成，但本地终局处理错误地转成 interrupted。
- Diagnostic evidence plan:
  - Prediction or clause under test: 中断信号先于任何终局尝试发生。
  - Signal: metrics interruption fields、provider request terminal、rollout 尾部和 stderr。
  - Capture method: 对同一 run 的结构化 artifacts 做时序关联。
  - Event name or marker:
    - interruption_source
    - provider.chat_wire_request_terminal
  - Correlation keys:
    - request_id
    - call_id
  - Differentiates from:
    - H-002
    - H-003
  - Supports if:
    - 明确外部中断发生且不存在终局尝试。
  - Refutes if:
    - Agent 已提交终局动作，或 runtime 主动制造了无外部来源的 interrupted。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: refuted；provider 正常完成，exec_nonzero 来自本地终局协议 fatal。
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: none；该分支已排除。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: map-request 缺少持续可见的机械 Map 事实，导致 Agent 初始化后遗忘生命周期
- Status: unverified
- Parent: P-001
- Claim: 普通请求完全不再携带当前 Map 事实，而初始化 control feedback 随历史后移，Agent 因上下文显著性不足停止维护 Map。
- Layer: interaction
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - 只出现一次初始化 control；初始化前 2 个普通工具被 hard gate 拒绝，随后 32 个普通工具由初始 binding 承载，没有 read 或 transition。
- Falsifiable predictions:
  - If true: provider payload 保留初始化历史但后续没有任何当前 Map handle/projection；Agent 在自然语言或动作中不再引用 Map 生命周期。
  - If false: 当前 Map 机械状态持续可见，或 Agent 明确知道未闭合但因其他错误无法提交 control。
- Diagnostic evidence plan:
  - Prediction or clause under test: 初始化后 provider 上下文没有当前 Map 事实，且 Agent 行为不再体现生命周期意识。
  - Signal: provider wire section/message shape、rollout assistant/tool-call 序列、control feedback 可见性。
  - Capture method: 对 request 2 到末请求逐请求检查 taskspace 载体和 Agent 动作。
  - Event name or marker:
    - provider.chat_wire_shape_recorded
    - taskspace_control
  - Correlation keys:
    - request_index
    - request_id
  - Differentiates from:
    - H-001
    - H-003
  - Supports if:
    - 当前机械 Map 事实在初始化后完全不可见，且 Agent 无终局/Map 动作。
  - Refutes if:
    - Agent 持续收到当前 Map 状态并明确尝试闭合。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: pending
- Related evidence:
  - E-001
  - E-003
  - E-004
- Conclusion: unverified
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: 重建逐 request 上下文与动作表。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-003: 终局硬门禁只有在 Agent 主动结束时才反馈，当前中断路径绕过了该反馈机会
- Status: confirmed
- Parent: P-001
- Claim: 状态机仍拒绝非法终局，但 provider/执行器中断路径没有触发终局尝试，因此 Agent 从未收到“Map 未闭合”的机械错误。
- Layer: interaction
- Factor relation: part_of
- Depends on:
  - H-001
- Rationale:
  - TaskSpace hard gate 是工具/终局动作上的底线规则，不应主动提醒；如果执行被外部切断，它可能没有合法反馈时机。
- Falsifiable predictions:
  - If true: trace 中没有 `finish_end` 或 plain final gate rejection，状态机 snapshot 仍合法且只因外部中断保持 open。
  - If false: 存在终局尝试但 gate 没有执行、反馈丢失或错误映射。
- Diagnostic evidence plan:
  - Prediction or clause under test: 中断前不存在任何终局 carrier 或 gate result。
  - Signal: rollout terminal response、TaskSpace terminal events、control action counts、last response actionability。
  - Capture method: 关联 rollout 尾部与 observability trace。
  - Event name or marker:
    - finish_end
    - taskspace_terminal_committed
  - Correlation keys:
    - call_id
    - request_id
  - Differentiates from:
    - H-002
  - Supports if:
    - 无终局动作且外部中断直接结束。
  - Refutes if:
    - 已有终局动作却没有得到硬门禁反馈。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-005
- Conclusion: confirmed；Agent 的普通 final 触发本地协议 fatal，冻结合同明确禁止自动重试，因此没有向 Agent 返回可纠正反馈的下一轮请求。
- Repair design readiness: ready，但该行为属于 R6 冻结终局合同，Phase D 不改动；是否改变应进入后续独立设计决策。
- Next step: 在 Phase D 结果中作为跨策略风险记录，不在本阶段修复。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: Docker 复杂样本复现
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: neutral
- Type: reproduction
- Source: `target/r7-phase-d/request/complex/subscription-billing-repair/20260719-045442-647/performance-observation.json`
- Prediction or plan link:
  - 建立症状基线，尚不能区分三个假设。
- Matched signal:
  - Standard solved；TaskSpace interrupted；initialize_map=1；read_map=0；control=1；ordinary tool attempts=34（初始化前 2、初始化后 32）；open leaf=1。
- Correlation keys:
  - run_id `20260719-045442-647`
- Raw content:
  ```text
  Standard: requests=14, business_success=true, completion=complete
  TaskSpace: requests=18, business_success=false, completion=interrupted
  TaskSpace controls: initialize_map=1, other controls=0
  TaskSpace map: nodes=4, edges=4, open_leaf_nodes=1
  ```
- Interpretation: 证明 Phase D 复杂样本失败和 Map 未闭合，但不证明中断来源或反馈缺失机制。
- Time: 2026-07-19 04:58

## Evidence E-002: 最后 provider 响应完成后本地终局门禁 fatal
- Related hypotheses:
  - H-001
  - H-003
- Direction: refutes
- Type: diagnostic-log
- Source: `pair-001/right/artifacts/metrics.json`、`provider-request-events.jsonl`、`whale-exec.stderr.log`
- Prediction or plan link:
  - H-001/H-003：区分外部中断与本地终局拒绝。
- Matched signal:
  - logical request 18 `response_completed`；随后 `taskspace_terminal_protocol_violation`；`exec_exit_code=1`，`exec_timed_out=false`。
- Correlation keys:
  - request_count `18`
  - map_id `map-1`
  - revision `2`
- Raw content:
  ```text
  last_provider_response_actionability=protocol_violation
  interruption_source=exec_nonzero
  exec_timed_out=false
  taskspace_terminal_protocol_violation control_mode="work_active" map_id="map-1" revision=Some(2)
  provider_assistant_message_present=true saw_actionable_output=false
  ```
- Interpretation: refutes H-001，supports H-003；中断是本地 hard gate 的直接结果，不是 provider 或预算先行终止。
- Time: 2026-07-19 05:03

## Evidence E-003: map-request 全程没有当前 projection 但保留初始化反馈
- Related hypotheses:
  - H-002
- Direction: supports
- Type: diagnostic-log
- Source: `pair-001/right/artifacts/provider-cache-trace-summary.json`
- Prediction or plan link:
  - H-002：检查初始化后当前 Map 事实的 provider 可见性。
- Matched signal:
  - 18 个 request 的 active projection 均为 0；request 3-18 均保留 687 bytes 初始化 control feedback；工具 schema 13 个且 hash 恒定。
- Correlation keys:
  - run_id `20260719-045442-647`
- Raw content:
  ```text
  active_projection request_bytes=[0 x 18]
  taskspace_control_feedback request_bytes=[0,0,687 x 16]
  tools bytes_per_request=27377
  prefix_preserved_count=17/17
  ```
- Interpretation: 证明 Agent 没有收到当前 Map projection，但没有发生上下文丢失或缓存形状断裂；单凭此证据不能证明它导致遗忘。
- Time: 2026-07-19 05:03

## Evidence E-004: map-append 近期复杂样本 3 次均闭合
- Related hypotheses:
  - H-002
- Direction: supports
- Type: experiment
- Source: `benchmarks/taskspace/r7/phase-c-static-bootstrap-contract-result.json`
- Prediction or plan link:
  - H-002：比较持续投影存在时同一复杂样本的生命周期完成情况。
- Matched signal:
  - Phase C map-append 的 `subscription-billing-repair` TaskSpace 3/3 solved；当前 map-request 0/1。
- Correlation keys:
  - Phase C run `20260719-030418-675`
  - Phase D run `20260719-045442-647`
- Raw content:
  ```text
  map-append: solved=3/3
  map-request: solved=0/1, terminal protocol violation
  ```
- Interpretation: 支持“持续当前 Map 可见性可能影响生命周期遵循”，但样本数和版本窗口不足以确认因果。
- Time: 2026-07-19 05:03

## Evidence E-005: R6 冻结合同明确普通 final 不重试
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-location
- Source: `core/src/session/turn.rs` 与 `core/tests/suite/taskspace_terminal_contract.rs`
- Prediction or plan link:
  - H-003：确认 gate 后是否存在 provider 纠正轮次。
- Matched signal:
  - `terminal_protocol_violation` 直接 `break Err(CodexErr::Fatal(...))`；集成测试名为 `plain_provider_final_is_nonterminal_and_does_not_retry`。
- Correlation keys:
  - commit `106440d65`
- Raw content:
  ```text
  break Err(CodexErr::Fatal(TASKSPACE_TERMINAL_PROTOCOL_VIOLATION.to_string()))
  plain_provider_final_is_nonterminal_and_does_not_retry
  ```
- Interpretation: H-003 的机制和设计来源均被直接代码及测试证据确认；它不是 Phase D 新引入的行为。
- Time: 2026-07-19 05:03

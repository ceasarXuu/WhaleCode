# Problem P-001: R5 正常进展被请求次数 hard stop 截断
- Status: open
- Created: 2026-07-10 01:54
- Updated: 2026-07-10 01:54
- Objective: 移除 TaskSpace 正常执行路径中的低请求次数 hard stop，只允许语义无关、可证明为严重资源异常的外部硬基线终止采样，并忠实保留未完成语义。
- Symptoms:
  - `count-call-stack` R5-D 样本在第 7 次 provider request 成功修改目标文件后，被 `TaskSpaceProviderBudgetHardStopV1` 截断，Agent 没有获得执行验证和最终回答的机会。
  - benchmark 外部 validator 随后通过，并把样本报告为 solved，掩盖了 Agent 被中断且任务未自行收尾的事实。
- Expected behavior:
  - 正常、有新反馈和有效进展的 Agent 执行不应被 TaskSpace 自定义请求次数上限终止。
  - profile 请求数只能用于观测、告警和成本分析，不能代表严重异常。
  - 若用户、provider 或系统资源硬边界确实终止执行，runtime 只能报告资源中断并保留证据，不能生成 Agent 最终答复或把中断记为正常完成。
- Actual behavior:
  - `verification_first` profile 将 rollout 上限固定为 6；达到上限且 grace 用尽后，pre-dispatch gate 无条件拒绝下一次模型请求。
  - hard stop developer message 随后被转成 `task_complete`，并用 runtime 生成文本覆盖 `last_agent_message`。
- Impact:
  - runtime 根据任务路由和普通请求次数干预 Agent 的执行上限，违反 R5 的工具边界。
  - Agent 的验证、状态提交和最终回答可能被截断，benchmark 又可能因外部验证通过而产生虚假的性能收益。
- Reproduction:
  - 读取 `target/r5d-ledger-deactivation/count-call-stack/20260710-002316-050/pair-001/right/artifacts/rollout.jsonl` 的 `node-event-11`、`TaskSpaceProviderBudgetHardStopV1` 和 `task_complete` 连续事件。
- Environment:
  - branch `whalecode-alpha`，commit `da69670`，DeepSeek `deepseek-v4-flash`，R5 Phase D，route mode `verification_first`。
- Known facts:
  - E-001
  - E-002
  - E-003
  - E-004
- Ruled out:
  - 本次不是无进展循环：hard stop 前的最新工具结果是成功 edit，且目标文件 diff 正确。
  - 本次不是 provider、上下文窗口、进程或 validator 故障：provider 请求成功完成，CLI 正常退出，外部 public/hidden validation 均通过。
- Fix criteria:
  - TaskSpace profile 的 rollout/node request 阈值不再参与正常 pre-dispatch 拒绝，只保留观测和告警。
  - 若保留 emergency stop，其触发必须是语义无关且可客观证明的严重异常或显式用户配置，不得由 route mode 的普通请求次数推导。
  - 资源中断不得生成 Agent 身份的最终答复，不得发出正常 `task_complete`，benchmark 必须区分外部验证成功与 Agent 完整完成。
  - focused 单测覆盖“超过 profile 仍可继续”“显式严重资源边界可中断”“中断不伪装完成”；原样本或等价样本中 Agent 能自行验证并结束。
- Current conclusion: H-001/H-002 已由代码路径和 R5-D 运行证据确认。现有 hard stop 不是严重异常保护，而是普通请求 profile 的强制执行器；旧的 grace 修复只延后错误边界，没有修正抽象。修复已具备设计条件，但尚未获得本轮实施授权。
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: route profile 将正常请求次数错误提升为硬终止条件
- Status: confirmed
- Parent: P-001
- Claim: `verification_first` 的固定 6 次 rollout profile 被 pre-dispatch gate 当作 hard limit；该 gate 不验证严重资源异常或无进展，只依据请求计数和 grace，因此会截断正常、有进展的 Agent 执行。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - profile 名称和 `over_profile_hint` 表明它本应是经验提示，但 gate 返回 `allowed: false` 并终止 provider sampling。
- Falsifiable predictions:
  - If true: 代码应将 `verification_first.max_rollout_model_requests` 设为 6，并在 `request_count >= max_requests` 且 grace 不可用时直接 hard stop，不检查新工具结果、进展或真实资源耗尽。
  - If false: hard stop 应要求 provider/context/process/cost 等独立严重异常证据，或只产生 advisory warning 而继续请求。
- Diagnostic evidence plan:
  - Prediction or clause under test: 普通 profile 请求计数本身就是 hard stop 的充分条件。
  - Signal: profile 配置和 `gate_provider_request_pre_dispatch` 条件表达式。
  - Capture method: 静态检查 `runtime.rs` 对 route budget 的构造与 pre-dispatch gate。
  - Event name or marker:
    - `provider_request_hard_limit_exceeded`
  - Correlation keys:
    - `route_mode:verification_first`
  - Differentiates from:
    - provider 真实限流或失败
    - context window 耗尽
    - 无进展循环检测
  - Supports if:
    - 达到固定 6 次请求即可触发拒绝，且条件不含上述严重异常信号。
  - Refutes if:
    - 6 次仅告警，hard stop 由独立严重异常触发。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留请求计数观测，移除其正常路径控制权。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: 经用户确认后，将 profile budget 降级为 observe/advisory，并把 emergency resource stop 拆成独立、显式的外部硬基线。
- Blocker:
  - repair authorization pending
- Close reason:
  - not closed

## Evidence E-001: verification_first 固定 6 次 rollout 请求
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:717-766`
- Prediction or plan link:
  - H-001 If true
- Matched signal:
  - `VerificationFirst` 设置 `max_rollout_model_requests = 6`、`max_model_requests_per_node = 2`。
- Correlation keys:
  - `taskspace-v005-verification_first`
- Raw content:
  ```text
  TaskSpaceRouteMode::VerificationFirst => {
      budget.max_rollout_model_requests = 6;
      budget.max_model_requests_per_node = 2;
  }
  ```
- Interpretation: 上限来自 TaskSpace route profile，不是用户显式限制或底层资源故障。
- Time: 2026-07-10 01:54

## Evidence E-002: pre-dispatch gate 只凭计数阻止模型请求
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1311-1369`
- Prediction or plan link:
  - H-001 diagnostic evidence plan
- Matched signal:
  - `request_count >= max_requests` 且 grace 不可用时返回 `allowed: false`，reason 为 `provider_request_hard_limit_exceeded`。
- Correlation keys:
  - `gate_provider_request_pre_dispatch`
- Raw content:
  ```text
  let rollout_limit_exceeded = snapshot.max_requests > 0
      && snapshot.request_count >= snapshot.max_requests;
  if rollout_limit_exceeded && !grace_remaining
      && !validation_rework_patch_feedback_grace
      && !fresh_node_first_request_grace
  {
      return TaskSpaceBudgetGateDecision { allowed: false, ... };
  }
  ```
- Interpretation: gate 未证明死循环、无进展、provider 故障、上下文耗尽或显式成本边界；普通计数是终止的核心依据。
- Time: 2026-07-10 01:54

## Hypothesis H-002: hard stop 把资源中断扭曲为 Agent 正常完成
- Status: confirmed
- Parent: P-001
- Claim: pre-dispatch hard stop 不仅停止采样，还注入 developer message、覆盖 `last_agent_message` 并发出 `task_complete`，把 runtime 中断扭曲为 Agent 完成；benchmark 的外部 validator 又把它归类为 solved。
- Layer: interaction
- Factor relation: all_of
- Depends on:
  - H-001
- Rationale:
  - 本样本没有 Agent 验证和最终总结，但报告仍显示 business success。
- Falsifiable predictions:
  - If true: 成功 edit 后应立即出现 runtime hard-stop developer message 和 `task_complete`，其 last message 不是 Agent 输出；metrics 仍为 `business_success: true`。
  - If false: 中断应保留为 incomplete/interrupted，且外部验证不得把 Agent completion 改写为完成。
- Diagnostic evidence plan:
  - Prediction or clause under test: hard stop 事件链会覆盖 Agent 完成语义并被 benchmark 误计成功。
  - Signal: rollout 末尾事件、last-message 和 metrics 分类。
  - Capture method: 对照 R5-D right artifacts 的最后工具结果、hard stop、task_complete、validation 和 metrics。
  - Event name or marker:
    - `node-event-11`
    - `TaskSpaceProviderBudgetHardStopV1`
    - `task_complete`
  - Correlation keys:
    - `turn_id:019f47b1-3449-7052-985d-33b7caf8201a`
  - Differentiates from:
    - Agent 主动输出 blocked/final
    - Agent 自行执行验证并完成
  - Supports if:
    - runtime hard stop 紧跟成功 edit，Agent 没有验证/最终答复，但 `task_complete` 和 `business_success:true` 仍出现。
  - Refutes if:
    - Agent 在 hard stop 前已验证并明确完成，或系统把结果标记为 interrupted/incomplete。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留 hard resource interruption 事件，但不得映射成正常 completion。
- Evidence gate: satisfied
- Related evidence:
  - E-003
  - E-004
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: 经用户确认后，拆分 provider sampling interruption、Agent completion 和 harness validation 三类状态。
- Blocker:
  - repair authorization pending
- Close reason:
  - not closed

## Evidence E-003: 成功 edit 后立即 hard stop 并发出 task_complete
- Related hypotheses:
  - H-002
- Direction: supports
- Type: reproduction
- Source: `target/r5d-ledger-deactivation/count-call-stack/20260710-002316-050/pair-001/right/artifacts/rollout.jsonl:201-203`
- Prediction or plan link:
  - H-002 If true
- Matched signal:
  - active projection 已记录 `node-event-11 action_class=edit tool_success=true`；下一事件是 `request_count: 7/6` hard stop，再下一事件是 runtime 文本作为 last message 的 `task_complete`。
- Correlation keys:
  - `turn_id:019f47b1-3449-7052-985d-33b7caf8201a`
- Raw content:
  ```text
  node-event-11 ... action_class=edit tool_success=true
  TaskSpaceProviderBudgetHardStopV1 ... request_count: 7/6 ...
  task_complete ... last_agent_message="TaskSpace provider budget hard stop: provider_request_hard_limit_exceeded"
  ```
- Interpretation: 最新动作是有效进展，不是严重异常；runtime 截断了 Agent 后续验证，并伪造了完成出口。
- Time: 2026-07-10 01:54

## Evidence E-004: 外部验证通过掩盖 Agent 未完成
- Related hypotheses:
  - H-002
- Direction: supports
- Type: diagnostic-log
- Source: `target/r5d-ledger-deactivation/count-call-stack/20260710-002316-050/pair-001/right/artifacts/metrics.json`
- Prediction or plan link:
  - H-002 diagnostic evidence plan
- Matched signal:
  - `public_validation_exit_code=0`、`hidden_oracle_exit_code=0`、`business_success=true`，同时 map 仍有 `open_leaf_nodes=1`、`state_commit_count=0`，last message 是 hard stop。
- Correlation keys:
  - `pair-001/right`
- Raw content:
  ```text
  business_success: true
  open_leaf_nodes: 1
  state_commit_count: 0
  public_validation_exit_code: 0
  hidden_oracle_exit_code: 0
  ```
- Interpretation: 代码修改正确只能证明业务 patch 有效，不能证明 Agent 完整执行；当前结果分类把两者混为一谈。
- Time: 2026-07-10 01:54

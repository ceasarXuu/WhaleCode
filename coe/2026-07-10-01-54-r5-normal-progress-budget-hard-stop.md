# Problem P-001: R5 正常进展被请求次数 hard stop 截断
- Status: fixed
- Created: 2026-07-10 01:54
- Updated: 2026-07-10 04:40
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
  - E-005
  - E-006
  - E-007
  - E-008
- Ruled out:
  - 本次不是无进展循环：hard stop 前的最新工具结果是成功 edit，且目标文件 diff 正确。
  - 本次不是 provider、上下文窗口、进程或 validator 故障：provider 请求成功完成，CLI 正常退出，外部 public/hidden validation 均通过。
- Fix criteria:
  - TaskSpace profile 的 rollout/node request 阈值不再参与正常 pre-dispatch 拒绝，只保留观测和告警。
  - 若保留 emergency stop，其触发必须是语义无关且可客观证明的严重异常或显式用户配置，不得由 route mode 的普通请求次数推导。
  - 资源中断不得生成 Agent 身份的最终答复，不得发出正常 `task_complete`，benchmark 必须区分外部验证成功与 Agent 完整完成。
  - focused 单测覆盖“超过 profile 仍可继续”“显式严重资源边界可中断”“中断不伪装完成”；原样本或等价样本中 Agent 能自行验证并结束。
- Current conclusion: H-001/H-002/H-003 的修复均已通过 focused tests 和等价 live sample 验证。普通 profile request count 只观测；completion/interruption/external validation 独立分类；当前 final/gate rejection 以 provider-visible 机械反馈进入下一轮。最终 R5 样本在 13 次 request 后由 Agent 自行完成、map 闭合、外部验证通过，且 39 次 exact payload scan 全部通过，无 budget hard stop 或禁用语义标记。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - H-001 + E-007 + E-008：超过旧 profile 后继续采样并完成，无 profile hard stop。
  - H-002 + E-007 + E-008：Agent completion、sampling interruption、external validation 独立记录，最终样本真实 complete。
  - H-003 + E-007 + E-008：当前拒绝成对保留到 provider history；live 样本收到机械拒绝后自行纠正。
- Close reason:
  - 原始截断与反馈丢失症状不再复现；性能成本回退作为 R5-F/G 独立问题继续跟踪，不恢复 runtime 语义控制。

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
- Next step: 已完成；后续只保留观测，资源中断复用显式 substrate 边界。
- Blocker:
  - none
- Close reason:
  - repaired and validated by E-007/E-008

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
- Next step: 已完成；benchmark 和 pair report 分项输出三类状态。
- Blocker:
  - none
- Close reason:
  - repaired and validated by E-007/E-008

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

## Hypothesis H-003: final gate 拒绝结果未进入下一轮 Agent 上下文
- Status: confirmed
- Parent: P-001
- Claim: `active_node_open` 拒绝被包装为临时 `last_agent_message` 和 actionability trace preview，但在返回 follow-up 前被清空，也没有记录为 provider-visible conversation item；Agent 下一轮看不到拒绝结果，只能继续基于原上下文重复 final。
- Layer: feedback
- Factor relation: single
- Depends on:
  - H-001
- Rationale:
  - 移除 profile hard stop 后，Agent 已正确 edit 和验证，却从第 12 次请求起连续 44 次输出 final；每次 trace 都记录同一 rejection，provider 行为没有任何纠正迹象。
- Falsifiable predictions:
  - If true: rollout 会出现连续 `final_rejected`，但不存在对应 developer/user/tool feedback item；代码会在设置 rejection 文本后清空 `last_agent_message`，外层只因 `needs_follow_up` 重采样。
  - If false: 下一次 provider prompt 应包含完整 `TaskSpaceFinalAnswerRejectedV1`，或 Agent 至少能看到等价的机械错误 item。
- Diagnostic evidence plan:
  - Prediction or clause under test: final rejection 只可观测、不进入 provider-visible history。
  - Signal: 首次 final、actionability event、后续 assistant response 序列，以及 turn 中 rejection 构造/清空/重采样代码。
  - Capture method: 对照 live `whale-exec.jsonl` 和 `turn.rs` final rejection 路径。
  - Event name or marker:
    - `TaskSpaceFinalAnswerRejectedV1`
    - `TaskSpaceProviderResponseActionabilityV1 actionability=final_rejected`
  - Correlation keys:
    - `pair-001/right`
    - `request_count:12..56`
  - Differentiates from:
    - `taskspace_control` 未注册
    - provider 拒绝 native tool schema
    - Agent 已看到错误但选择忽略
  - Supports if:
    - rejection 仅存在于 trace preview，conversation history 没有对应输入 item，且下一轮继续原样 final。
  - Refutes if:
    - payload 证明完整拒绝已进入下一轮上下文。
  - Instrumentation status: permanent
  - Instrumentation lifecycle:
    - 保留 actionability trace，同时增加 provider-visible 机械错误 item 的回归测试。
- Evidence gate: satisfied
- Related evidence:
  - E-005
  - E-006
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: 已完成；当前 rejection/tool pair 强制保留，反馈只含 exact reason、gate class 和 state unchanged。
- Blocker:
  - none
- Close reason:
  - repaired and validated by E-007/E-008

## Evidence E-005: 移除 hard stop 后出现 44 次不可见拒绝循环
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: reproduction
- Source: `target/r5e-phase-e-final/count-call-stack/20260710-031757-998/pair-001/right/artifacts/whale-exec.jsonl`
- Prediction or plan link:
  - H-003 If true
- Matched signal:
  - R5 在请求 6/6 后继续 edit、执行 CLI 和 public validator；第 12 次请求首次 final 被 `active_node_open` 拒绝，随后到 56/6 共记录 44 次 `final_rejected`，期间没有 `taskspace_control` call，Agent 每轮继续输出完成总结。
- Correlation keys:
  - `pair-001/right`
  - `request_count:12..56`
- Raw content:
  ```text
  request_count=7/6: successful file_change
  request_count=11/6: validator_contract=passed
  request_count=12/6: actionability=final_rejected ... hard_state: active_node_open
  request_count=56/6: actionability=final_rejected ... hard_state: active_node_open
  ```
- Interpretation: profile 已不再控制执行，但机械拒绝没有形成反馈闭环；这是上下文传递缺失，不是 Agent 在已知错误上主动选择重复。
- Time: 2026-07-10 03:24

## Evidence E-006: rejection 在重采样前被清空且未记录 conversation item
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/session/turn.rs:14561-14630`
- Prediction or plan link:
  - H-003 diagnostic evidence plan
- Matched signal:
  - `record_action_map_main_final_response` 失败后，代码只把 `TaskSpaceFinalAnswerRejectedV1` 放入局部 `last_agent_message`；actionability trace 记录 preview 后，`final_response_rejected` 分支把它设为 `None`。外层看到 `needs_follow_up=true` 后直接进入下一次 sampling，没有记录 provider-visible item。
- Correlation keys:
  - `final_response_rejected`
  - `needs_follow_up`
- Raw content:
  ```text
  last_agent_message = Some(taskspace_final_answer_gate_rejection_followup(&error));
  ...
  if final_response_rejected {
      last_agent_message = None;
  }
  ```
- Interpretation: trace 能看到错误不等于 Agent 上下文能看到错误；当前链路在反馈层丢失了 rejection 语义。
- Time: 2026-07-10 03:24

## Evidence E-007: focused tests 验证 hard stop 退场、生命周期拆分和当前拒绝可见
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports
- Type: fix-validation
- Source: `third_party/codex-cli/codex-rs/core` 与 `scripts/taskspace-benchmark` 测试输出
- Prediction or plan link:
  - P-001 Fix criteria
- Matched signal:
  - `active_context_replacement_preserves_current_gate_feedback_pair`、`final_gate_rejection_item_is_provider_visible_mechanical_feedback`、hard-gate taxonomy、no-auto-rework、action-contract no-reinterpret focused tests 实际执行通过。
  - benchmark metrics/cost/E3-validity/harness 四个 selftest 全部通过；interruption、completion、external validation 和 request/control source 分开计量。
- Correlation keys:
  - `R5-E-focused-tests-20260710`
- Raw content:
  ```text
  cargo test -p codex-core --no-run: PASS
  codex-features: 37 passed
  codex-tools: 139 passed, 1 ignored
  benchmark selftests: 4/4 PASS
  cargo build -p codex-cli --bin whale: PASS
  ```
- Interpretation: 修复不是依赖 live 偶然行为；活动路径和报告分类都有独立回归保护。
- Time: 2026-07-10 04:33

## Evidence E-008: 等价 live sample 完成且拒绝反馈闭环正常
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports
- Type: fix-validation
- Source: `target/r5e-phase-e-final-clean/count-call-stack/20260710-043411-389/pair-001`
- Prediction or plan link:
  - P-001 Fix criteria
- Matched signal:
  - R5 在 13 次内部 request 后 `agent_completion_status=complete`、`agent_final_observed=true`、`sampling_interrupted=false`、`external_validation_status=passed`、`open_leaf_nodes=0`。
  - 首次 `finish_node` 成功后 Agent 错误重复 finish；runtime 返回 `no_current_node_binding` 机械拒绝，下一轮 provider history 同时包含拒绝和 `current_node:none/status=completed` projection，Agent 随后自行纠正并 final。
  - 39 次 exact payload scan 全部 `passed=true`；legacy history、budget hard stop、`next_valid_actions`、semantic summary 和其他禁用标记均未出现。
- Correlation keys:
  - `pair-001/right`
  - `provider_request:logical-1..13`
- Raw content:
  ```text
  agent_completion_status=complete
  model_request_count=13 source=rollout_trace
  taskspace_control_count=2 source=rollout_trace
  open_leaf_nodes=0
  exact_payload_scan_event_count=39; failed=0; forbidden=0
  ```
- Interpretation: profile 计数不再截断执行，runtime 不伪装完成，当前拒绝语义能够进入 Agent 上下文并形成反馈闭环。样本的 3.11x wall-time 回退是独立成本问题，不否定本问题的修复，也不能通过恢复 hard stop 处理。
- Time: 2026-07-10 04:36

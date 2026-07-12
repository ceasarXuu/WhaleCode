# Problem P-001: TaskSpace final rejection 自动放大 provider requests
- Status: open
- Created: 2026-07-12 23:17
- Updated: 2026-07-13 00:28
- Objective: 删除Runtime对未闭合Map最终回答的自动重采样控制，使硬状态反馈只忠实暴露一次，不在无新动作或状态变化时重复请求provider。
- Symptoms:
  - `subscription-billing-repair` repeat-2 R5产生120 requests、56 controls、15 nodes和3,280,395 input tokens，仍最终solved。
  - requests 54-93连续记录相同`final_rejected`，同一running node状态没有变化。
- Expected behavior:
  - Runtime可以记录Map未闭合的机械状态，但不得替Agent决定继续推理并自动反复请求provider。
  - 同一拒绝反馈不应在状态无变化时重复写入上下文。
- Actual behavior:
  - final gate拒绝Agent回答后设置`needs_follow_up=true`，外层turn loop立即再次sampling。
  - Agent继续给出final时，Runtime重复拒绝、重复写developer feedback并继续sampling。
- Impact:
  - 单个complex任务从常见12-20 requests放大到120；uncached input达到1,391,371，wall达到385.69s。
  - G阶段3-repeat收益门禁失败，J6.7和J7保持blocked。
- Reproduction:
  - 读取`target/r5-j6-7-7-repeat3-final/subscription-billing-repair/20260712-225957-211/pair-002/left/artifacts`。
  - 对照`rollout.jsonl`中的`provider_response_actionability:final_rejected`和`TaskSpaceFinalAnswerRejectedV1`。
- Environment:
  - branch `whalecode-alpha`，binary commit `84979fe`，Docker hard boundary，`deepseek-v4-flash`。
- Known facts:
  - E-001：3-repeat中只有complex repeat-2发生120-request outlier，全部validator仍通过。
  - E-002：50次final rejection均由Runtime标为follow-up；52份拒绝feedback进入canonical context。
  - E-003：119个相邻请求中118个保持message prefix；不存在provider transport retry证据。
  - E-004：触发前Agent创建了两个未绑定的final/inspect nodes，形成open-node hard state。
- Ruled out:
  - provider transport retry不是主因；120个distinct logical provider requests均有terminal lifecycle。
  - projection latest replacement不是主因；active projection为0且prefix preserved为118/119。
- Fix criteria:
  - plain Agent final或final gate失败不得由Runtime自动触发无界provider follow-up。
  - 同一hard state和相同feedback下新增provider request数为0。
  - Agent回答、Map未闭合状态和硬错误均忠实保留，不伪造task completed。
  - focused/complex各3 repeats全部solved，无final-rejection loop；request/cache/wall重新进入门禁。
- Current conclusion: 用户已授权修复。`session/turn.rs`已删除plain final拒绝注入、`final_rejected`分类和自动follow-up；显式`finish_then_end`硬校验及Map未闭合状态保持不变，正在执行sample验证。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: final gate rejection被Runtime转换为自动sampling循环
- Status: confirmed
- Parent: P-001
- Claim: `session/turn.rs`在final gate失败后设置`needs_follow_up=true`，导致外层turn loop在没有用户输入的情况下继续sampling；相同状态可无限重复该路径。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - outlier含50次final rejection，且请求理由集中为`final_synthesis`和`response_recovery`。
- Falsifiable predictions:
  - If true: 每次final gate error后都记录developer rejection、清空last Agent message并返回`needs_follow_up=true`。
  - If false: rejection后turn应结束，或只有新的Agent/user动作才能再次sampling。
- Diagnostic evidence plan:
  - Prediction or clause under test: final gate error直接驱动下一次provider request。
  - Signal: `final_response_rejected`、`needs_follow_up`和下一logical request的连续trace。
  - Capture method: 对照repeat-2 rollout与`session/turn.rs`控制流。
  - Event name or marker:
    - `provider_response_actionability:final_rejected`
    - `TaskSpaceFinalAnswerRejectedV1`
  - Correlation keys:
    - pair-002/left
    - provider request_count 54..93
  - Differentiates from:
    - provider retry、cache miss、Agent ordinary tool loop
  - Supports if:
    - code设置follow-up且trace显示无用户输入的连续distinct logical requests。
  - Refutes if:
    - 连续请求来自transport attempt retry或新pending user input。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留request reason、response actionability和重复feedback计数。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
- Conclusion: confirmed
- Repair design readiness: implemented; awaiting runtime validation
- Next step: run focused and complex Docker samples
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: partial multi-finish与Agent创建孤立节点触发open-node状态
- Status: confirmed
- Parent: P-001
- Claim: Agent提交`implement -> verify -> final_synthesis`时，首步成功、第二步因目标不存在失败；随后Agent创建node-1/node-2但未把当前verify正确过渡到这些节点，形成terminal gate持续拒绝的open nodes。
- Layer: interaction
- Factor relation: part_of
- Depends on:
  - H-001
- Rationale:
  - control step反馈显示result-2已提交且verify已bound，紧接着出现两个create_node和open-node terminal failure。
- Falsifiable predictions:
  - If true: 首次异常terminal前Map已经存在verify current lease及node-1/node-2 open nodes。
  - If false: open nodes应由Runtime自行创建或feedback映射错误产生。
- Diagnostic evidence plan:
  - Prediction or clause under test: open nodes可逐项追溯到Agent control calls和成功step。
  - Signal: task-event-72到124的control call/output及Map snapshot。
  - Capture method: 按event sequence重放control lifecycle。
  - Event name or marker:
    - `TaskSpaceControlResultV1`
    - `active_map_has_open_nodes`
  - Correlation keys:
    - task-event-72..124
  - Differentiates from:
    - Runtime隐式建图、projection状态扭曲
  - Supports if:
    - 每个orphan node都有Agent create_node call且成功反馈。
  - Refutes if:
    - node没有对应Agent-authored call或成功step未进入上下文。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留control step和Map lifecycle关联。
- Evidence gate: satisfied
- Related evidence:
  - E-004
- Conclusion: confirmed trigger；不能单独解释Runtime自动产生50次provider follow-up
- Repair design readiness: no separate repair required before H-001 boundary decision
- Next step: retain as reproduction trigger
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-003: provider retry或projection缓存破坏造成120 requests
- Status: refuted
- Parent: P-001
- Claim: 120 requests主要来自provider transport retries或projection变化导致的缓存重试。
- Layer: environment
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - outlier cache下降明显，需要排除外部provider和projection结构。
- Falsifiable predictions:
  - If true: logical request distinct count应明显低于terminal attempts，或message prefix频繁断裂。
  - If false: 120个请求均为distinct logical requests且prefix基本保持。
- Diagnostic evidence plan:
  - Prediction or clause under test: transport attempts和wire prefix解释request放大。
  - Signal: distinct request、terminal lifecycle、prefix preserved和shape transition计数。
  - Capture method: request-phase和provider-cache trace。
  - Event name or marker:
    - `provider.chat_wire_prefix_preserved`
  - Correlation keys:
    - pair-002/left
  - Differentiates from:
    - H-001 Runtime follow-up
  - Supports if:
    - retries或prefix breaks接近新增请求数量。
  - Refutes if:
    - distinct=terminal=120且prefix preserved=118/119。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留wire prefix与logical request分账。
- Evidence gate: satisfied
- Related evidence:
  - E-003
- Conclusion: refuted
- Repair design readiness: not applicable
- Next step: none
- Blocker:
  - none
- Close reason:
  - refuted

## Evidence E-001: 3-repeat复现单组120-request outlier
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: `target/r5-j6-7-7-repeat3-final/subscription-billing-repair/20260712-225957-211/performance-observation.json`
- Prediction or plan link:
  - H-001关于同一状态重复sampling的预测
- Matched signal:
  - repeat-2 R5为120 requests、56 controls、15 nodes、385.69s、3,280,395 input，最终solved。
- Correlation keys:
  - pair-002/left
- Raw content:
  ```text
  model_request_count=120
  taskspace_control_count=56
  nodes=15
  input_tokens=3280395
  wall_time_ms=385691
  business_success=true
  ```
- Interpretation: 正确性最终恢复，但TaskSpace控制路径可发生数量级request放大。
- Time: 2026-07-12 23:12

## Evidence E-002: 50次final rejection均驱动follow-up
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: pair-002/left `rollout.jsonl`及`third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Prediction or plan link:
  - H-001 diagnostic evidence plan
- Matched signal:
  - rollout含50个`response_actionability:final_rejected`和52个rejection feedback items；代码在gate error分支设置`needs_follow_up=true`并清空`last_agent_message`。
- Correlation keys:
  - request_count 54..93
  - `TaskSpaceFinalAnswerRejectedV1`
- Raw content:
  ```text
  final_rejected=50
  final_gate_feedback_items=52
  needs_follow_up = true
  final_response_rejected = true
  if final_response_rejected { last_agent_message = None; }
  ```
- Interpretation: Runtime不是只返回一次硬错误，而是主动把拒绝转成下一次model sampling。
- Time: 2026-07-12 23:16

## Evidence E-003: provider retry与projection prefix不是主因
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: diagnostic-log
- Source: pair-002/left `request-phase-summary.json`和`provider-cache-trace-summary.json`
- Prediction or plan link:
  - H-003 falsifiable prediction
- Matched signal:
  - distinct request=120、terminal=120、prefix preserved=118/119、shape transition=1。
- Correlation keys:
  - pair-002/left
- Raw content:
  ```text
  provider_request_distinct_count=120
  provider_request_terminal_count=120
  prefix_preserved_count=118/119
  cache_shape_transition_count=1
  ```
- Interpretation: 新增请求由Runtime逻辑发起，不是同一logical request的transport attempt，也不是projection重排。
- Time: 2026-07-12 23:16

## Evidence E-004: open nodes可追溯到Agent control sequence
- Related hypotheses:
  - H-002
- Direction: supports
- Type: diagnostic-log
- Source: pair-002/left `rollout.jsonl` task-event-72..124
- Prediction or plan link:
  - H-002 diagnostic evidence plan
- Matched signal:
  - 首个finish step已提交并bound verify，第二步因`final_synthesis`不存在失败；随后两个Agent `create_node`成功，terminal反馈明确列出node-1/node-2 open。
- Correlation keys:
  - task-event-72..124
- Raw content:
  ```text
  steps[0]: result_id=result-2 binding_status=bound
  steps[1]: transition_rejected next node `final_synthesis` does not exist
  TaskSpace node created: node-1
  TaskSpace node created: node-2
  active_map_has_open_nodes: node-1,node-2
  ```
- Interpretation: Agent错误操作触发了hard state，但50次自动重采样由H-001 Runtime边界行为造成。
- Time: 2026-07-12 23:16

## Evidence E-005: plain final自动重采样路径已从Runtime删除
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `third_party/codex-cli/codex-rs/core/src/session/turn.rs`、`core/src/action_map/runtime.rs`
- Prediction or plan link:
  - H-001关于final gate error不得驱动下一次provider request的修复判据
- Matched signal:
  - plain final记录Map失败时仅写`taskspace.plain_final_delivered_with_open_map`结构化日志；不设置`needs_follow_up`、不改写或清空Agent message、不写developer rejection item。
  - `FinalRejected` actionability及`response_recovery`中的`final_rejected`分支均已删除。
- Correlation keys:
  - `taskspace.plain_final_delivered_with_open_map`
- Raw content:
  ```text
  cargo test -p codex-core active_context_replacement_tests
  12 passed; 0 failed
  ```
- Interpretation: 单元层已证明plain final保持`FinalCandidate`且不要求recovery；仍需Docker sample证明原始症状不再出现。
- Time: 2026-07-13 00:28

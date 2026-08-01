# Problem P-001: R6 live sample enters transition rejection loop
- Status: fixed
- Created: 2026-07-15 08:26
- Updated: 2026-07-15 09:50
- Objective: 找出并修复 R6 生产纵向切换后两个 Docker 样本共同进入状态转换拒绝循环的根因。
- Symptoms:
  - single-file-fast-fix 的 R6 侧达到约 79 次 provider request，profile hint 为 8。
  - multi-file-order-pipeline 的 R6 侧达到约 51 次 provider request，profile hint 为 14。
  - 两侧反复收到 transition_invalid，revision 多数停留在 1。
- Expected behavior:
  - Agent 初始化合法 Root -> Work -> Finish 图，按 work frontier 执行普通工具并显式 finish_end。
  - 机械拒绝忠实返回当前 revision、图状态和 violation，不形成无状态变化的无限 provider loop。
- Actual behavior:
  - 两个独立样本均持续声明不同 node ID 的 transition，但 Runtime 反复拒绝。
  - provider budget 只记录 over_profile_hint，没有终止循环；运行由工程侧主动终止。
- Impact:
  - R6 Phase C Docker live gate 失败；当前候选不可进入 Phase D。
  - 两条并行运行产生显著额外 token、时间和 provider 成本。
- Reproduction:
  - 当前提交 e744c9c4d 构建并 attested 的 debug Whale。
  - Docker hard boundary，deepseek-v4-flash，reasoning effort max。
  - 分别运行 single-file-fast-fix 与 multi-file-order-pipeline 的 Standard/R6 pair。
- Environment:
  - Linux host，Docker Server 29.6.1，image digest sha256:55a8ac465c574efb57d8bd53f286812a77f41fd428de1c3b0b18b7c5165ee0ca。
  - Branch whalecode-alpha，candidate e744c9c4d。
- Known facts:
  - 两个样本的 Standard 侧均正常结束，只有当前 R6 TaskSpace 侧进入循环。
  - credential、binary attestation、container image 和 pair disk preflight 全部通过。
  - repeated reject 的 state_commit=false，revision 多数为 1。
  - simple 的 75 次、branch-join 的 52 次 TaskSpace 调用全部是 `initialize_map`，且每次都令旧 `current_node_id == root.node_id`。
  - 下一 provider request 忠实保留了上一 assistant tool call 与同 call_id tool output；不存在消息条目丢失或重复。
- Ruled out:
  - 暂无。
- Fix criteria:
  - 根因通过 trace、代码路径与定向失败测试中的至少两类证据确认。
  - 修复后原始两个 Docker 样本 R6 侧均完成外部验证，无 transition reject loop。
  - 拒绝反馈保持忠实，不增加 Runtime 语义决策或兼容分支。
- Current conclusion: Root 被旧合同错误暴露为可绑定 current 节点、拒绝回执又报告候选 revision，是循环的根因与反馈放大因素；新合同和单层真实回执已通过两个原始 Docker 样本复验。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - simple 与 branch-join 的 R6 侧均完成业务执行、显式终结和外部验证，control/state/protocol failure 为 0。
  - 两个样本分别形成 5/4 节点、4/3 边的完整 Root -> Finish 路径，没有 transition reject loop。
- Close reason:
  - 原始失败路径已不可表达，机械反馈真实性与 live 修复门禁均满足。

## Hypothesis H-001: initialization accepts a non-actionable current root
- Status: confirmed
- Parent: P-001
- Claim: initialize_map 接受 current_node_id=root，但 Root 不承载 ordinary tool lease 且不能执行 Agent 随后请求的 transition，形成合法初始化与可执行状态之间的合同断层。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - none
- Rationale:
  - 首个可见拒绝的 subject 是 root，后续日志还出现 no_task_path。
- Falsifiable predictions:
  - If true: 首次成功初始化 payload 的 current node 是 Root，且下一次普通工具或 Root transition 无合法执行路径。
  - If false: 首次成功初始化已经绑定 Ready/Running Work，Root 从未成为 current execution binding。
- Diagnostic evidence plan:
  - Prediction or clause under test: 首次成功 state commit 后的 canonical map 与 current binding。
  - Signal: rollout 中 initialize 参数、R6 result、snapshot/projection 和紧随其后的 tool call。
  - Capture method: 解析两条已保留 rollout/provider trace 的前 5 次 control interaction。
  - Event name or marker:
    - map_runtime_graph_revision_committed
  - Correlation keys:
    - pair side、call_id、revision
  - Differentiates from:
    - H-002
  - Supports if:
    - accepted initialization records Root as current while no Work lease/binding exists。
  - Refutes if:
    - accepted initialization records a valid Work binding before the first reject。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: 旧 schema 把 `current_node_id` 暴露为无角色裸引用，解析层还明确接受 Root；Runtime 却无条件按 `Work/Ready -> Running` 执行 Bind。两条 trace 的每次调用均选择 Root 并在候选 revision 1 失败，外层原子事务随后回滚。
- Repair design readiness: satisfied
- Next step: none
- Blocker:
  - none
- Close reason:
  - implemented and validated by E-005/E-006

## Hypothesis H-002: control feedback hides or distorts the committed graph state
- Status: confirmed
- Parent: P-001
- Claim: initialize/reject 的 provider-visible feedback 没有忠实暴露 canonical node IDs、statuses、current binding 或 revision，导致 Agent 在每次请求中猜测新 node ID。
- Layer: amplifier
- Factor relation: all_of
- Depends on:
  - none
- Rationale:
  - Agent 连续使用 node_1、n1、a 等不同 ID，router error 又把 typed R6 JSON 嵌入另一层 error.message。
- Falsifiable predictions:
  - If true: provider-visible tool result 缺少 map state，或 R6 JSON 被字符串化嵌套而无法作为原结构读取。
  - If false: 每次 provider request 都能看到未扭曲的完整 current map state 与 exact violation subjects。
- Diagnostic evidence plan:
  - Prediction or clause under test: model input 中 control result 的原始 wire shape。
  - Signal: provider-wire trace 的 request payload、function call output、sequence aggregation。
  - Capture method: 按 call_id 对齐 provider response、Runtime tool output 和下一 request input。
  - Event name or marker:
    - TaskSpaceControlResultR6V1
  - Correlation keys:
    - call_id、request sequence、revision
  - Differentiates from:
    - H-001
    - H-003
  - Supports if:
    - exact graph state 不在下一 request，或 typed result 被再次包装为 message string。
  - Refutes if:
    - next request 含一次且结构正确的 map state、node IDs/statuses/revision。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: 消息条目没有丢失，但拒绝语义发生两处失真：候选 revision 1 被表述为实际 current revision；完整 R6 rejection 又被字符串化到另一层 `error.message`。真实生产状态始终是空 map/revision 0。
- Repair design readiness: satisfied
- Next step: none
- Blocker:
  - none
- Close reason:
  - implemented and validated by E-005/E-006

## Hypothesis H-003: exposed transitions do not cover the canonical state path
- Status: refuted
- Parent: P-001
- Claim: tool schema 暴露的 bind/complete/block/unblock 与 initialized node status/readiness 的转换表不一致，使 Agent 无法通过合法 sequence 从 revision 1 推进。
- Layer: root-cause
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - 大量不同 node subject 均得到同一 transition_invalid，而 revision 不变。
- Falsifiable predictions:
  - If true: 用首次 committed map 执行 schema 允许的 transition，找不到到达 ordinary-tool-ready Work 的合法路径。
  - If false: 至少存在一个工具 schema 可表达且领域核心接受的确定 transition。
- Diagnostic evidence plan:
  - Prediction or clause under test: production initialization state 到 first Work Running 的可达路径。
  - Signal: 领域 transition table、handler mapping和使用 live initialize payload 的定向测试。
  - Capture method: 从 trace 提取 initialize 参数，构造纯 transaction/handler probe。
  - Event name or marker:
    - transition_rejected
  - Correlation keys:
    - node ID、status、revision
  - Differentiates from:
    - H-001
    - H-002
  - Supports if:
    - 所有 exposed transition 均被 validator 拒绝，且不是 stale revision 或 wrong ID。
  - Refutes if:
    - trace 中存在明确合法 transition，但 Agent 未选择。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: `Root/Open -> first Work/Ready -> Bind -> Work/Running` 是工具 schema 可表达且领域核心接受的确定路径；合法初始化定向测试稳定提交到 revision 2。失败来自旧 schema 允许选择 Root，不是 transition table 缺路。
- Repair design readiness: not applicable
- Next step: none
- Blocker:
  - none
- Close reason:
  - transition-table root-cause claim refuted

## Evidence E-001: two independent TaskSpace samples reproduce the same loop
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports
- Type: reproduction
- Source: target/r6-phase-c-live/*/*/20260715-082257-*/pair-001/right/artifacts
- Prediction or plan link:
  - P-001 stable reproduction requirement
- Matched signal:
  - repeated transition_invalid with revision 1 in both samples
- Correlation keys:
  - simple run 20260715-082257-633
  - branch run 20260715-082257-656
- Raw content:
  ~~~text
  single-file-fast-fix: transition_invalid subjects root/node_1/n1/...; request_count ~79/8
  multi-file-order-pipeline: transition_invalid across changing node IDs; request_count ~51/14
  ~~~
- Interpretation: 该现象跨两个不同复杂度样本稳定存在，不能归为一次 Agent 随机选择。
- Time: 2026-07-15 08:26

## Evidence E-002: budget telemetry confirms state-free request amplification
- Related hypotheses:
  - H-002
- Direction: supports
- Type: diagnostic-log
- Source: both whale-exec.jsonl artifacts
- Prediction or plan link:
  - H-002 provider-visible feedback fails to enable stateful recovery
- Matched signal:
  - request_dispatched_without_state_change repeated after each actionable response
- Correlation keys:
  - request_count 74->79 simple
  - request_count 46->51 branch
- Raw content:
  ~~~text
  TaskSpaceProviderRequestBudgetEventV1 ... state=over_profile_hint->over_profile_hint ... reason=request_dispatched_without_state_change
  TaskSpaceProviderResponseActionabilityV1 actionability=actionable ... assistant_message_present=false saw_actionable_output=true
  ~~~
- Interpretation: Runtime 将每个被拒绝的 control call 视为 actionable output 并继续 provider loop，state 未推进；这描述循环，不单独证明反馈为何无效。
- Time: 2026-07-15 08:26

## Evidence E-003: every live bootstrap selected Root as current node
- Related hypotheses:
  - H-001
- Direction: supports
- Type: trace-correlation
- Source: 两条 preserved rollout JSONL
- Prediction or plan link:
  - H-001 首次及后续 initialization payload
- Matched signal:
  - simple 75/75、branch-join 52/52 调用均为 initialize_map，且 `current_node_id == root.node_id`
- Correlation keys:
  - call_id、pair side、logical request
- Raw content:
  ~~~text
  simple first call: root=root, current_node_id=root
  branch first call: root=understand, current_node_id=understand
  all calls: state_commit=true count=0
  ~~~
- Interpretation: 现象由工具合同稳定诱发，不是一次随机 Agent 选择。
- Time: 2026-07-15 08:35

## Evidence E-004: candidate revision and committed state diverge on rejection
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: code-path-and-test
- Source: action_map/runtime.rs; action_map/runtime_tests.rs
- Prediction or plan link:
  - 外层 clone transaction 的原子回滚
- Matched signal:
  - domain initialize 先产生 revision 1，Root Bind 拒绝后 outer runtime 不写回；修复前反馈仍报告 revision 1
- Correlation keys:
  - current_revision、state_commit、snapshot
- Raw content:
  ~~~text
  before snapshot == after rejected initialization snapshot
  repaired rejection.current_revision == 0
  repaired rejection.state_commit == false
  ~~~
- Interpretation: 旧反馈把未提交候选状态当成当前状态；修复后的 revision 与真实 pre-state 一致。
- Time: 2026-07-15 08:40

## Evidence E-005: repaired contract and feedback pass focused and subsystem regression
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports H-001/H-002 repair; refutes H-003
- Type: test
- Source: local cargo and R6 contract test output
- Prediction or plan link:
  - fix criteria local gate
- Matched signal:
  - schema requires `current_work_node`; old `current_node_id` is rejected; non-ready initial Work is atomic revision 0 rejection; rooted rejection remains one JSON object
- Correlation keys:
  - codex-tools 142; action_map 64; control 16; sequence 12; reconstruction 30; multi_agents 85
- Raw content:
  ~~~text
  codex-tools: 141 passed, 1 ignored
  action_map: 64 passed
  taskspace control: 16 passed
  sequence: 12 passed
  rollout reconstruction: 30 passed
  multi_agents: 85 passed
  R6 rooted DAG contract: passed
  ~~~
- Interpretation: 修复收紧的是机械输入角色和反馈真实性，没有让 Runtime 选择 Work 或注入纠错策略。
- Time: 2026-07-15 08:43

## Evidence E-006: 两个原始 Docker 样本均无 transition reject loop
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports H-001/H-002 repair; refutes H-003
- Type: fix-validation
- Source: `target/r6-phase-c-epoch/simple/single-file-fast-fix/20260715-094309-889`; `target/r6-phase-c-epoch/branch-join/multi-file-order-pipeline/20260715-094519-735`
- Prediction or plan link:
  - P-001 Docker fix criteria
- Matched signal:
  - 两个 R6 arm 均 solved、external validation passed、Task completed；control/state/protocol failure 为 0
- Correlation keys:
  - simple run `20260715-094309-889`
  - branch-join run `20260715-094519-735`
- Raw content:
  ~~~text
  simple: requests=13, map=1, nodes=5, edges=4, open=0
  branch-join: requests=13, map=1, nodes=4, edges=3, open=0
  exact payload scan failures: 0
  ~~~
- Interpretation: 修复后 Agent 能初始化并推进合法 Work，Root 仅在显式 finish_end 事务中与 Finish 一同闭合；旧 transition 拒绝循环未复现。
- Time: 2026-07-15 09:50

# Problem P-001: TaskSpace 内部 action 被高频提升为顶层 Function Call
- Status: open
- Created: 2026-08-19 07:24
- Updated: 2026-08-19 07:24
- Objective: 解释原生 action 协议下 `shell` 被提升为未声明顶层 Function Call 的发生机制，并在证据不足时阻止继续通过改名、提示词堆叠或 Runtime 自动包裹掩盖问题。
- Symptoms:
  - 最新复杂样本的三次有效 TaskSpace 运行中，两次运行生成顶层 `shell`，合计五个调用。
  - Pair 001 先正确执行 `taskspace_exec(actions.kind=shell)`，下一请求却并行生成三个顶层 `shell`。
  - Pair 003 首次工作前并行生成两个顶层 `shell`，随后在拒绝反馈后改用 `taskspace_exec`。
- Expected behavior:
  - Agent 只将 `taskspace_exec` 作为 client/Map 动作的顶层 Function Tool；`shell` 只能作为其内部 action identity 出现。
- Actual behavior:
  - Provider 接受并返回了未在顶层 Tool 集合声明的 `function_call(name=shell)`；Runtime 在副作用前拒绝，Agent 下一请求恢复。
- Impact:
  - 每次异常响应至少增加一次 Provider 请求和整轮 input；同一响应的并行调用会形成成组失败。
  - 当前业务正确性未受损，但三次 TaskSpace 中两次命中，不能视为低频边缘错误。
- Reproduction:
  - `release-dispatch-repair`，实际 TaskSpace pairs 001/003/005，`deepseek-v4-flash`，`map-request`。
  - Evidence root: `target/whale-agent-runs/WAR-20260819-064028-R8-NATIVE-ACTION-R5/release-dispatch-repair/20260819-065901-736`。
- Environment:
  - branch `whalecode-alpha`；commit `e8024ec779f2884a41f15db9792aafe65854ccce`；TaskSpace base `3.1.0`；action protocol commit `3750a3932`。
- Known facts:
  - 三次 TaskSpace 请求面的顶层普通 `exec_command` 为零，顶层 Tool 集合没有声明 `shell`。
  - Base instructions 和 `taskspace_exec` description 都明确说明 action 不是独立顶层 Tool。
  - 内层 `shell` action 分支完整暴露原生命令参数 schema，并复用原生 imperative description。
  - Pair 001 的非法参数是内部 action 的机械扁平化：`kind=shell` 变为 Function 名称，`parameters.cmd` 被提升，wrapper-only `node_id` 被保留。
  - Pair 003 的非法参数来自同一内层参数合同，但漏掉 `node_id`，因此 Runtime 无法忠实自动包回 Exec。
  - 所有五次调用均在副作用前拒绝，Agent 在下一请求纠正。
- Ruled out:
  - Runtime 将内层 action 提升为顶层调用。
  - TaskSpace 顶层重新声明了普通 client Tool。
  - 文字协议明确允许顶层调用 `shell`。
  - 拒绝反馈丢失或错误；两轮均在下一请求准确恢复。
- Fix criteria:
  - 候选必须消除或显著降低“内层 action 被理解为独立 Function”的结构诱因，而不修改普通 Tool 原生合同、不让 Runtime 推断节点归属、不增加 Standard 分支。
  - 离线必须证明最终顶层 Tool 集合仍只有 `taskspace_exec` 与 Provider-hosted Tool，内部能力仍可完整机械路由。
  - 真实复验必须在同等复杂样本上覆盖首次初始化和后续工作，且不再出现任何顶层 client/action identity 逃逸；真实预算另行批准。
- Current conclusion: 根因已收敛为两个共同条件：TaskSpace action schema 仍把 `shell` 暴露为具有完整原生参数和命令式说明的可调用式分支，模型会把该分支机械提升为顶层 Function；DeepSeek Responses 路径又没有在 Provider 侧把 Function 名称硬限制在已声明集合。禁止文字存在且在线可见，因此继续增加同义提示不是主修复方向。成功反馈再次突出 `action=shell` 可能是后续调用的放大因素，但不是必要条件，因为 Pair 003 在任何成功反馈前已逃逸。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
- Resolution basis:
  - direct mechanism satisfied；repair candidate not selected
- Close reason:
  - not closed

## Hypothesis H-001: 内层 action schema 仍被模型识别为可调用 Function
- Status: confirmed
- Parent: P-001
- Claim: `taskspace_action.anyOf` 的每个分支以 `kind=shell` 作为判别名，携带完整原生参数 schema，并复用“Runs a command...”说明；模型据此把分支提升为顶层 `function_call(name=shell)`。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - H-003
- Rationale:
  - 旧 `tool=exec_command/input` 方案发生同型提升；改为 `kind=shell/parameters` 后，被提升的名称随之从 `exec_command` 变成 `shell`。
- Falsifiable predictions:
  - If true: 非法顶层调用的名称和参数应可由内层 action 分支机械导出，并携带内层独有字段或参数结构。
  - If false: 非法调用应使用 schema 中不存在的名称或参数，或顶层 `shell` 应来自独立声明/Runtime 改写。
- Diagnostic evidence plan:
  - Prediction or clause under test: 比较最终 schema、真实 Function Call 参数、顶层 Tool 集合和 Runtime dispatch 路径。
  - Signal: `kind` 枚举、原生参数字段、非法调用参数、provider tool count、Runtime rejection。
  - Capture method: 静态读取 `catalog.rs`/`sequence_schema.rs`/`protocol.rs`，并逐调用核对 pairs 001/003 rollout。
  - Event name or marker:
    - `function_call(name=shell)`
    - `taskspace.exec.request_started`
  - Correlation keys:
    - outer call ID
    - pair number
  - Differentiates from:
    - H-004 文字协议冲突
    - Runtime 名称改写
  - Supports if:
    - 名称由 `kind=shell` 导出，参数由 `parameters` 导出，且顶层未声明 `shell`。
  - Refutes if:
    - 顶层声明包含 `shell`，或 Runtime 在 Provider response 后生成该调用。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
- Conclusion: 两种非法参数形状都来自同一内层 action 合同；改名仅改变了被提升的 Function 名称，没有消除抽象层级混淆。
- Repair design readiness: ready for candidate design after user confirmation
- Next step: 设计最小结构单变量，优先移除“可调用式 action identity + 原生 imperative branch description”的组合信号，不做 Runtime 自动包裹。
- Blocker:
  - 核心协议候选需要用户确认；真实复验需要独立预算。
- Close reason:
  - not closed

## Hypothesis H-002: 成功反馈中的 action identity 放大后续提升
- Status: unverified
- Parent: P-001
- Claim: 成功反馈把 `action_results[].action="shell"` 作为突出身份再次写入自然历史，提高下一请求把 `shell` 当作独立可调用 Tool 的概率。
- Layer: interaction
- Factor relation: part_of
- Depends on:
  - H-001
- Rationale:
  - Pair 001 首次正确调用后，成功反馈明确返回 `action=shell`，下一响应立即产生三个顶层 `shell`。
- Falsifiable predictions:
  - If true: 在输入 schema 不变时，仅降低成功反馈中独立 action 名的可调用显著性，会降低“正确一次后再逃逸”。
  - If false: 逃逸频率不变，或多数逃逸都发生在首次成功反馈前。
- Diagnostic evidence plan:
  - Prediction or clause under test: 区分 first-turn escape 与 post-success escape，并做反馈单变量前后对照。
  - Signal: 逃逸前一条 `taskspace_exec` result 是否含 action identity。
  - Capture method: 当前 trace 分类；若进入修复实验，只做一个反馈变量并另行申请预算。
  - Event name or marker:
    - `action_results[].action`
  - Correlation keys:
    - previous outer call ID
  - Differentiates from:
    - H-001 schema 本身足以诱发首次逃逸
  - Supports if:
    - post-success 逃逸显著下降且 first-turn 不变。
  - Refutes if:
    - 反馈变化没有作用或 first-turn 已解释全部频率。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: pending
- Related evidence:
  - E-004
  - E-005
- Conclusion: Pair 001 支持其作为候选放大因素；Pair 003 证明它不是必要根因，当前不能据此直接修改反馈合同。
- Repair design readiness: blocked until isolated
- Next step: 先解决 H-001 的输入合同结构，再决定是否仍需反馈单变量。
- Blocker:
  - 缺少隔离实验。
- Close reason:
  - not closed

## Hypothesis H-003: Provider 未硬限制 Function 名称
- Status: confirmed
- Parent: P-001
- Claim: DeepSeek Responses 路径接受并返回了未在请求顶层 Tool 集合声明的 Function 名称，使模型的内外层混淆能够到达 Runtime。
- Layer: interaction
- Factor relation: all_of
- Depends on:
  - none
- Rationale:
  - 若 Provider 严格限制名称，`function_call(name=shell)` 应在 Provider 边界被拒绝而不进入 rollout。
- Falsifiable predictions:
  - If true: wire 的顶层普通 client Tool 数为零，但 rollout 含完整的 `function_call(name=shell)`。
  - If false: 请求实际声明了 `shell`，或该 item 由本地 Runtime 合成。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对照 provider wire breakdown、rollout 原始 response item 和本地拒绝事件。
  - Signal: `tools_count=2`、`native_client_tool=0`、`function_call(name=shell)`。
  - Capture method: 读取 pairs 001/003 provider wire trace 与 rollout。
  - Event name or marker:
    - `provider.chat_wire_request_payload`
    - `function_call(name=shell)`
  - Correlation keys:
    - logical request ID
    - call ID
  - Differentiates from:
    - Runtime 重新暴露或改写 Tool
  - Supports if:
    - 未声明名称原样出现在 Provider response。
  - Refutes if:
    - 最终请求顶层包含 `shell`。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - retain current wire and response traces
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-006
- Conclusion: Provider 名称约束缺口是异常到达 Runtime 的必要条件，但不解释模型为何选择 `shell`；生成诱因仍在 H-001。
- Repair design readiness: no local Provider-side repair assumed
- Next step: Runtime 继续 fail-closed；不要以此为理由推断或自动修复节点绑定。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-004: Base 或 Tool 文字协议错误允许顶层 shell
- Status: refuted
- Parent: P-001
- Claim: Agent 高频逃逸是因为 L1/L2 明确或含糊地允许将 action kind 作为顶层 Tool 调用。
- Layer: sub-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 若禁止规则缺失，优先修复文字协议比改结构更小。
- Falsifiable predictions:
  - If true: Base 或 Tool description 应缺少 sole-entry 规则，或存在直接调用 action kind 的冲突说明。
  - If false: 两层都明确禁止，且 Agent在拒绝后能准确复述正确规则并恢复。
- Diagnostic evidence plan:
  - Prediction or clause under test: 当前生产 hash 对应的 Base、Tool description 和拒绝后 reasoning。
  - Signal: sole top-level、not independently callable、do not emit action kind、Agent correction text。
  - Capture method: 静态读取当前源码并核对 Pair 001 rollout。
  - Event name or marker:
    - Base version `3.1.0`
  - Correlation keys:
    - Pair 001 request after rejection
  - Differentiates from:
    - H-001 结构信号压过文字规则
  - Supports if:
    - 禁止文字缺失或互相冲突。
  - Refutes if:
    - 禁止规则完整且 Agent能在反馈后准确应用。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-007
  - E-008
- Conclusion: 当前没有“文字协议告诉 Agent 可以顶层调用 shell”的错误；再增加同义句不会解决已观测的结构提升。
- Repair design readiness: not applicable
- Next step: none
- Blocker:
  - none
- Close reason:
  - refuted by current contract and recovery trace

## Evidence E-001: 顶层未声明 shell 但 Provider 返回五个 shell Function Call
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: reproduction
- Source: `WAR-20260819-064028-R8-NATIVE-ACTION-R5` provider wire trace and pairs 001/003 rollout
- Prediction or plan link:
  - H-001/H-003 未声明名称到达 Runtime
- Matched signal:
  - 三次 TaskSpace 顶层 `exec_command=0`；pairs 001/003 共五个 `function_call(name=shell)`。
- Correlation keys:
  - pair-001
  - pair-003
- Raw content:
  ```text
  TaskSpace runs: 3
  top-level exec_command calls: 0
  top-level shell calls: 5
  affected runs: 2/3
  ```
- Interpretation: 异常不是顶层 schema 暴露普通 Tool；Provider 允许未声明名称进入本地 Runtime。
- Time: 2026-08-19 07:24

## Evidence E-002: Pair 001 非法参数是完整内层 action 的机械扁平化
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: pair-001 `rollout.jsonl`
- Prediction or plan link:
  - H-001 参数应可由内层 action 机械导出
- Matched signal:
  - `kind=shell` 成为 Function 名，`parameters.cmd` 被提升，`node_id` 被保留。
- Correlation keys:
  - `call_00_M4P8Mpyc3GLqFPPzGtY84163`
  - `call_01_hOYQny1iXn7N2xypXJMa1179`
  - `call_02_ox5yOI3bdmWkObG0kWhv1846`
- Raw content:
  ```text
  expected inner: {"kind":"shell","node_id":"inspect","parameters":{"cmd":"..."}}
  actual outer: function_call(name="shell", arguments={"cmd":"...","node_id":"inspect"})
  ```
- Interpretation: 输出不是无关幻觉，而是对内层 action 分支进行稳定的层级提升。
- Time: 2026-08-19 07:24

## Evidence E-003: Pair 003 在首次反馈前提升内层参数合同
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: diagnostic-log
- Source: pair-003 `rollout.jsonl`
- Prediction or plan link:
  - H-001 输入 schema 本身足以诱发；H-002 反馈不是必要条件
- Matched signal:
  - 首次 `taskspace_exec` 前生成两个顶层 `shell`，参数为内层 command schema 的 `cmd/workdir`，但无 `node_id`。
- Correlation keys:
  - `call_00_Ad4RaDJPYKWCyUmGd7D90497`
  - `call_01_ndwLT4g7yt2CASpaO9xu7308`
- Raw content:
  ```text
  function_call(name="shell", arguments={"cmd":"pwd && ls -la ...","workdir":"/workspace"})
  function_call(name="shell", arguments={"cmd":"cat README.md ...","workdir":"/workspace"})
  ```
- Interpretation: 成功反馈不是逃逸必要条件；缺少 `node_id` 也证明 Runtime 不能无语义推断地自动包裹。
- Time: 2026-08-19 07:24

## Evidence E-004: Pair 001 正确调用后反馈再次突出 action=shell
- Related hypotheses:
  - H-002
- Direction: supports
- Type: observation
- Source: pair-001 first `taskspace_exec` output
- Prediction or plan link:
  - H-002 post-success exposure
- Matched signal:
  - 下一请求前上下文包含独立字段 `"action":"shell"`。
- Correlation keys:
  - `call_00_3KRCk3SeX873FP6wS5FY9976`
- Raw content:
  ```text
  "action_results":[{"node_id":"inspect","action":"shell","outcome":"succeeded",...}]
  ```
- Interpretation: 该字段可能放大 action identity 的可调用显著性，但单条顺序证据不能证明因果。
- Time: 2026-08-19 07:24

## Evidence E-005: Pair 003 无成功反馈也发生逃逸
- Related hypotheses:
  - H-002
- Direction: refutes
- Type: observation
- Source: pair-003 first Provider response
- Prediction or plan link:
  - H-002 若为必要根因，首次成功反馈前不应逃逸
- Matched signal:
  - 首响应直接产生两个顶层 `shell`。
- Correlation keys:
  - pair-003 first response
- Raw content:
  ```text
  first TaskSpace client response items: shell, shell
  prior taskspace_exec success output: none
  ```
- Interpretation: H-002 只能是候选放大因素，不能作为主根因。
- Time: 2026-08-19 07:24

## Evidence E-006: Runtime 对五次逃逸均零副作用拒绝
- Related hypotheses:
  - H-003
- Direction: supports
- Type: diagnostic-log
- Source: pairs 001/003 function call outputs
- Prediction or plan link:
  - H-003 未声明 Function 到达 Runtime 后由本地硬边界拒绝
- Matched signal:
  - 每个 call ID 都收到配对失败输出，未启动 Tool future。
- Correlation keys:
  - five shell call IDs
- Raw content:
  ```text
  TaskSpace rejects undeclared top-level Function Tool `shell`. It was not executed.
  ```
- Interpretation: 当前硬边界正确；问题位于 Agent 可见生成合同与 Provider 名称约束，不是执行旁路。
- Time: 2026-08-19 07:24

## Evidence E-007: 两层文字合同都明确禁止独立 action Tool
- Related hypotheses:
  - H-004
- Direction: refutes
- Type: code-location
- Source: `whalecode_taskspace.md:214` and `taskspace_exec/protocol.rs:15-21`
- Prediction or plan link:
  - H-004 文字允许或缺失
- Matched signal:
  - Base 写明 sole top-level entry；Tool description 写明 action not independently callable、不得把 kind 作为顶层 Tool。
- Correlation keys:
  - Base `3.1.0`
- Raw content:
  ```text
  An action is part of that capability, not an independently callable top-level Tool.
  Do not emit an action `kind` as a top-level Tool call.
  ```
- Interpretation: 禁止规则不是缺失；结构信号与模型 Function Call 偏向压过了文字规则。
- Time: 2026-08-19 07:24

## Evidence E-008: Agent 收到拒绝后准确理解并恢复
- Related hypotheses:
  - H-004
- Direction: refutes
- Type: observation
- Source: pair-001 reasoning after three rejections
- Prediction or plan link:
  - H-004 若规则不可理解，拒绝后仍应继续错误调用
- Matched signal:
  - Agent明确识别自己使用了顶层 `<invoke name="shell">`，下一请求改用 `taskspace_exec.actions`。
- Correlation keys:
  - pair-001 request after three shell calls
- Raw content:
  ```text
  I invoked the top-level `shell` tool directly instead of using `taskspace_exec`.
  I need to always use `taskspace_exec` with actions.
  ```
- Interpretation: 反馈语义有效且 Agent具备正确规则；首选错误来自生成偏置而非规则完全不可理解。
- Time: 2026-08-19 07:24

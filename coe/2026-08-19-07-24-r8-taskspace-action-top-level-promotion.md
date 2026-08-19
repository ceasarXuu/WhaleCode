# Problem P-001: TaskSpace 内部 exec_command 被提升为顶层 Function Call
- Status: open
- Created: 2026-08-19 07:24
- Updated: 2026-08-19 11:20
- Objective: 解释原生 `tools[]` 协议下 `exec_command` 被提升为未声明顶层 Function Call 的发生机制，并在证据不足时阻止继续通过改名、提示词堆叠或 Runtime 自动包裹掩盖问题。
- Symptoms:
  - 当前有效原生 `tools[]` 协议的历史四臂基线显示，三个 TaskSpace 臂中 `7/9` 轮发生顶层 `exec_command` 逃逸，共 11 次。
  - 已回退的错误 `shell` 候选和已删除的 Hosted 双写协议不计入当前产品问题的 Tool 类型统计。
- Expected behavior:
  - Agent 只将 `taskspace_exec` 作为 client/Map 动作的顶层 Function Tool；内部 client Tool 保持原生 identity 和输入合同，但不会被提升为顶层 Function Call。
- Actual behavior:
  - Provider 接受并返回了未在顶层 Tool 集合声明的 `function_call(name=exec_command)`；Runtime 在副作用前拒绝，Agent 下一请求恢复。
- Impact:
  - 每次异常响应至少增加一次 Provider 请求和整轮 input；同一响应的并行调用会形成成组失败。
  - 当前业务正确性未受损，但四臂有效 TaskSpace 中 `7/9` 轮命中，不能视为低频边缘错误。
- Reproduction:
  - `subscription-billing-repair × map-always/map-append/map-request × repeat=3`，`deepseek-v4-flash`。
  - Evidence root: `target/r8-four-arm/WAR-20260818-055427-R8-FOUR-ARM-R3/data`。
- Environment:
  - branch `whalecode-alpha`；错误 action protocol commit `3750a3932` 已由 `ab23d8f5b` 回退；当前恢复原生 `tools[]` 协议。
- Known facts:
  - 四臂 TaskSpace 请求面的顶层 Tool 集合没有声明 `exec_command`；Runtime 和 Provider wire 没有重新暴露它。
  - Base instructions 和 `taskspace_exec` description 都明确说明 client Tool 只能位于 outer Exec 内部。
  - 非法参数是内部调用的机械扁平化：`tool=exec_command` 变为 Function 名称，`input.cmd` 被提升，wrapper-only `node_id` 被保留。
  - Runtime 对非法调用均在副作用前拒绝；可继续反馈生效后 Agent 能在下一请求纠正。
  - `taskspace_exec` 替代的是 Codex 顶层 `exec` 超级工具，不替代或重命名 Exec 内部原生 Tool；内部 `exec_command` 暴露本身符合产品定义。
  - 历史四臂基线 subject commit `4fe2f3557` 使用 TaskSpace base `3.0.5`；Standard 只作为正常顶层 `exec_command` 对照，不计逃逸。
  - 四臂基线的 9 次 TaskSpace 中 7 次发生逃逸；这 7 次的第一次逃逸都紧跟成功 `taskspace_exec` output，且 output 包含 `client_results[].tool="exec_command"`。
  - Codex `exec` 也暴露原生内部 Tool，但通过 `tools.exec_command(...)` 命名空间表达；其 outer result 默认突出 `Script completed/failed`，不会机械回显独立的内部 Tool identity 字段。
  - 对现存 885 份 rollout 复扫并排除离线 fixture、错误 `shell` 候选和已删除 Hosted 双写协议后，当前有效设计下只有 `exec_command` 发生过顶层逃逸；没有发现 `apply_patch`、`write_stdin`、`view_image` 或其他原生 client Tool 逃逸。
- Ruled out:
  - Runtime 将内部 `exec_command` 提升为顶层调用。
  - TaskSpace 顶层重新声明了普通 client Tool。
  - 文字协议明确允许顶层调用 `exec_command`。
  - `web_search` 和 `shell` 能证明当前有效协议下存在其他 Tool 逃逸；两者分别属于已删除和已回退设计。
- Fix criteria:
  - 候选必须消除或显著降低内部 `exec_command` 被理解为独立顶层 Function 的诱因，而不修改普通 Tool 原生合同、不让 Runtime 推断节点归属、不增加 Standard 分支。
  - 离线必须证明最终顶层 Tool 集合仍只有 `taskspace_exec` 与 Provider-hosted Tool，内部能力仍可完整机械路由。
  - 真实复验必须在同等复杂样本上覆盖首次初始化和后续工作，且不再出现任何顶层 client/action identity 逃逸；真实预算另行批准。
- Current conclusion: 当前有效产品设计中只坐实了 `exec_command` 逃逸，不能泛化为所有内部 client Tool 的共同作用域缺陷。直接路径已确认：模型把内部 `exec_command` 名称、原生参数和 wrapper `node_id` 组合成未声明顶层 Function Call，Provider 的名称约束缺口使其到达 Runtime。`exec_command` 作为编码 Agent 最常用、通常首个执行、具有强直接调用先验的 Tool，又在成功反馈中以 `client_results[].tool="exec_command"` 再次出现；这组特殊性是当前根因候选。反馈字段与下一轮逃逸存在 `7/7` 顺序关联，但尚无单变量 A/B，不能宣称唯一因果。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
- Resolution basis:
  - direct mechanism satisfied；repair candidate not selected
- Close reason:
  - not closed

## Hypothesis H-001: 内层 action schema 仍被模型识别为可调用 Function
- Status: closed
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
- Conclusion: 两种非法参数形状都来自同一内层 action 合同，证明“内部判别值可能被提升”的机制；但该证据不支持把原生内部 Tool 替换成 TaskSpace 自建 action。`shell` 候选已被产品层级澄清推翻并回退。
- Repair design readiness: not ready; this hypothesis cannot select a replacement vocabulary
- Next step: none；该错误候选只作为反例保留。
- Blocker:
  - none
- Close reason:
  - superseded by product-layer clarification and revert `ab23d8f5b`

## Hypothesis H-002: 成功反馈中的 exec_command identity 放大后续提升
- Status: unverified
- Parent: P-001
- Claim: 成功反馈把 `client_results[].tool="exec_command"` 作为裸 identity 再次写入自然历史，提高下一请求把 `exec_command` 当作独立顶层 Tool 的概率。
- Layer: interaction
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 恢复原生协议的历史四臂证据中，7 个受影响 TaskSpace 运行的第一次 `exec_command` 逃逸全部紧跟一个含 `client_results[].tool="exec_command"` 的成功反馈。
- Falsifiable predictions:
  - If true: 在输入 schema 不变时，仅降低成功反馈中独立 Tool 名称的可调用显著性，会降低“正确一次后再逃逸”。
  - If false: 逃逸频率不变，或多数逃逸都发生在首次成功反馈前。
- Diagnostic evidence plan:
  - Prediction or clause under test: 区分 first-turn escape 与 post-success escape，并做反馈单变量前后对照。
  - Signal: 逃逸前一条 `taskspace_exec` result 是否含 `exec_command` identity。
  - Capture method: 当前 trace 分类；若进入修复实验，只做一个反馈变量并另行申请预算。
  - Event name or marker:
    - `client_results[].tool`
  - Correlation keys:
    - previous outer call ID
  - Differentiates from:
    - H-005 `exec_command` 的直接调用先验和首轮位置
  - Supports if:
    - post-success 逃逸显著下降且 first-turn 不变。
  - Refutes if:
    - 反馈变化没有作用或 first-turn 已解释全部频率。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: pending
- Related evidence:
  - E-011
  - E-013
- Conclusion: 对当前有效原生协议，该因素具有 `7/7` 受影响运行的顺序关联；但没有反馈单变量 A/B，且 `exec_command` 本身还有强直接调用先验和首轮高频位置，当前只能列为最强放大候选。
- Repair design readiness: blocked until isolated
- Next step: 若进入修复实验，只改变成功反馈对内部 Tool identity 的作用域表达，保持请求 schema、原生 Tool、Base、DAG 和拒绝逻辑不变。
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

## Hypothesis H-005: exec_command 的直接调用先验压过 TaskSpace 外层协议
- Status: unverified
- Parent: P-001
- Claim: `exec_command` 在编码 Agent 训练和工作流中具有比其他 client Tool 更强的顶层直接调用先验；它通常又是初始化后的首个、高频 Tool，并在成功反馈中再次出现裸名称，这些因素共同压过 `taskspace_exec` 的外层协议。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - H-003
- Rationale:
  - 当前有效设计的真实逃逸全部是 `exec_command`，没有其他原生 client Tool。
  - 逃逸名称和参数可由内部调用机械导出，且同时保留 TaskSpace wrapper 专属 `node_id`；顶层请求没有声明对应 Function。
  - `exec_command` 几乎总是首个探索动作，暴露次数和直接调用习惯显著高于 Patch、图片和 Tool Search。
  - Codex `exec` 保留相同原生能力，但用 `tools.exec_command(...)` 明确内部命名空间，并由 outer `exec` 返回脚本级状态。
- Falsifiable predictions:
  - If true: 在保持其他合同不变时，降低 `exec_command` 裸名称的重复显著性会降低逃逸；其他内部 Tool 在相当暴露机会下仍不会出现同类行为。
  - If false: 其他原生 client Tool 在有效设计下也出现同型逃逸，或调整 `exec_command` 反馈显著性后频率不变。
- Diagnostic evidence plan:
  - Prediction or clause under test: 内部调用到非法顶层调用的名称、参数和顺序映射。
  - Signal: `tools[].tool`、`client_results[].tool`、下一响应 Function name/arguments、顶层 Tool 集合。
  - Capture method: 四臂历史 trace、owner-state trace、全量 rollout Tool 类型复扫与 Codex `exec` 静态对照。
  - Event name or marker:
    - `function_call(name=exec_command)`
  - Correlation keys:
    - outer call ID
    - next provider request
  - Differentiates from:
    - 顶层 Tool 重新声明
    - Runtime 改写
    - Base 明确允许直接调用
  - Supports if:
    - 有效设计下逃逸持续只发生于 `exec_command`，且反馈单变量降低其频率。
  - Refutes if:
    - 其他原生 client Tool 出现同型逃逸，或反馈单变量无影响。
  - Instrumentation status: existing permanent traces sufficient for direct mechanism
  - Instrumentation lifecycle:
    - retain
- Evidence gate: pending; specificity is observed, causal factors are not isolated
- Related evidence:
  - E-009
  - E-010
  - E-011
  - E-012
  - E-013
- Conclusion: `exec_command` 的特殊性已经由全量历史分类确认，但“直接调用先验、首轮位置、反馈重复”三者各自贡献尚未隔离，不能先建设通用内部 Tool 作用域方案。
- Repair design readiness: blocked until one variable is isolated
- Next step: 优先设计只改变 `exec_command` 成功反馈重复表达的单变量候选；请求 schema、原生 Tool、Base、DAG 和拒绝逻辑保持不变。
- Blocker:
  - 缺少反馈单变量真实 A/B。
- Close reason:
  - not closed

## Evidence E-009: 错误 shell 候选已整体回退且原生协议离线通过
- Related hypotheses:
  - H-001
  - H-005
- Direction: supports
- Type: fix-validation
- Source: commit `ab23d8f5b` and focused test output
- Prediction or plan link:
  - 产品澄清要求恢复原生内部 Tool identity，不保留兼容分支。
- Matched signal:
  - `3750a3932` 的 `actions[].kind=shell` 生产改动被完整反向应用；`cargo test -p codex-core taskspace --lib --locked` 为 123 passed。
- Correlation keys:
  - `ab23d8f5b`
- Raw content:
  ```text
  Revert "fix(taskspace): expose native actions through exec"
  test result: ok. 123 passed; 0 failed
  ```
- Interpretation: 当前调查基线重新是原生 `tools[]`，后续结论不得继续以 `shell` 方案为目标设计。
- Time: 2026-08-19 11:20

## Evidence E-010: 历史四臂中 TaskSpace 逃逸为 7/9 轮和 11 次
- Related hypotheses:
  - H-002
  - H-005
- Direction: supports
- Type: historical-trace-audit
- Source: `WAR-20260818-055427-R8-FOUR-ARM-R3` twelve rollouts
- Prediction or plan link:
  - H-005 内部 identity 会被提升；H-002 统计 post-success 顺序。
- Matched signal:
  - Standard `a0` 的顶层 `exec_command` 是正常声明调用，不计逃逸。
  - Map Always `a1`: 2/3 轮、3 次逃逸。
  - Map Append `a2`: 3/3 轮、4 次逃逸。
  - Map Request `a3`: 2/3 轮、4 次逃逸。
- Correlation keys:
  - subject commit `4fe2f3557eab1ca07836dfdc9e0f909b73329ea7`
  - TaskSpace base `3.0.5`
- Raw content:
  ```text
  a1: 2/3 affected, 3 calls
  a2: 3/3 affected, 4 calls
  a3: 2/3 affected, 4 calls
  total: 7/9 affected, 11 calls
  ```
- Interpretation: 恢复后的原生协议存在跨 projection 模式的高频层级逃逸，不能把 owner-state 单轮复发视为孤例。
- Time: 2026-08-19 11:20

## Evidence E-011: 四臂每个受影响运行的第一次逃逸都紧跟 client result
- Related hypotheses:
  - H-002
- Direction: supports
- Type: sequence-analysis
- Source: `WAR-20260818-055427-R8-FOUR-ARM-R3` TaskSpace rollouts
- Prediction or plan link:
  - H-002 post-success exposure should precede the first escape in affected runs.
- Matched signal:
  - 7 个受影响运行的第一次非法 `function_call(name=exec_command)` 之前，最近的 Tool output 都是成功 `taskspace_exec` output，且都含 `client_results[].tool="exec_command"`。
  - 同一运行中的第二次逃逸通常紧跟第一次零副作用拒绝，属于一次错误选择的并行或连续放大，不另算首次触发。
- Correlation keys:
  - a1/r1
  - a1/r2
  - a2/r1
  - a2/r2
  - a2/r3
  - a3/r1
  - a3/r2
- Raw content:
  ```text
  first escape after successful client_results: 7/7 affected runs
  first-turn escape before any taskspace_exec success: 0/9 TaskSpace runs
  ```
- Interpretation: 成功反馈重复裸 Tool identity 是高可信放大候选；顺序关联本身仍不能替代隔离 A/B。
- Time: 2026-08-19 11:20

## Evidence E-012: Codex exec 通过命名空间和 outer 状态隔离内部 Tool
- Related hypotheses:
  - H-005
- Direction: supports
- Type: code-location
- Source: `codex-rs/code-mode/src/description.rs` and `core/src/tools/code_mode/mod.rs`
- Prediction or plan link:
  - 成熟 Exec 设计应保留原生 Tool 同时建立明确 outer/inner 作用域。
- Matched signal:
  - 内部调用写为 `await tools.exec_command(...)`；nested Tool 结果只存在于脚本内，除非 Agent 显式 `text(result)`；outer result 首先返回 `Script completed/failed`。
- Correlation keys:
  - `PUBLIC_TOOL_NAME=exec`
- Raw content:
  ```text
  await tools.exec_command(...)
  Script completed
  ```
- Interpretation: 这不是 TaskSpace 修复的直接证明，但反驳“必须隐藏原生内部 Tool”，并提供作用域隔离的成熟结构对照。
- Time: 2026-08-19 11:20

## Evidence E-013: 有效产品设计下只有 exec_command 发生逃逸
- Related hypotheses:
  - H-002
  - H-005
- Direction: supports
- Type: historical-trace-audit
- Source: repository-local 885 `rollout.jsonl` files
- Prediction or plan link:
  - 判定逃逸是否为所有内部 Tool 的通用问题，还是 `exec_command` 特有问题。
- Matched signal:
  - 只统计同一 trace 出现 `taskspace_exec` 的运行，并排除 `patch-observability-selftest` fixture。
  - 当前有效原生协议中发现 `exec_command` 21 次、涉及 17 个 rollout。
  - `shell` 5 次来自已回退错误 action 候选；`web_search` 2 次来自已删除 Hosted 双写协议，均不属于有效设计下的对照 Tool。
  - 未发现 `apply_patch`、`write_stdin`、`view_image`、Tool Search 或其他原生 client Tool 逃逸。
- Correlation keys:
  - 885 local rollouts
- Raw content:
  ```text
  valid-design escape identities: exec_command only
  exec_command: 21 calls / 17 rollouts
  other native client tools: 0
  ```
- Interpretation: 当前证据不支持“所有内部 Tool identity 都会逃逸”的泛化；调查范围收窄到 `exec_command` 特有生成倾向。
- Time: 2026-08-19 11:45

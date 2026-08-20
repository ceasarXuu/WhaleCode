# Problem P-001: TaskSpace 内部 exec_command 被提升为顶层 Function Call
- Status: open
- Created: 2026-08-19 07:24
- Updated: 2026-08-19 14:40
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
  - 受影响调用仍保留正确 `node_id`，说明 Agent 没有遗忘 Map 或节点归属；错误只发生在 outer/inner 调用层级。
  - Chat Completions 适配器始终用 outer `taskspace_exec` 的原 `call_id` 生成 `role=tool/tool_call_id`，不会把 `client_results[].tool` 改写成顶层 Function 名。
  - 可解析当前 `tools[]` 协议的历史样本中，成功 `exec_command` 后再次执行 `exec_command` 时发生 18/199 次顶层提升；成功 `apply_patch` 后执行 `exec_command` 为 0/102。
  - 同一统计中，`initialize_and_work` 后继续执行 `exec_command` 的提升率为 13/105，普通 `work` 后为 1/81；异常集中在初始化后的第一次连续探索。
  - Codex `exec` 也暴露原生内部 Tool，但通过 `tools.exec_command(...)` 命名空间表达；其 outer result 默认突出 `Script completed/failed`，不会机械回显独立的内部 Tool identity 字段。
  - 对现存 885 份 rollout 复扫并排除离线 fixture、错误 `shell` 候选和已删除 Hosted 双写协议后，当前有效设计下只有 `exec_command` 发生过顶层逃逸；没有发现 `apply_patch`、`write_stdin`、`view_image` 或其他原生 client Tool 逃逸。
- Ruled out:
  - Runtime 将内部 `exec_command` 提升为顶层调用。
  - TaskSpace 顶层重新声明了普通 client Tool。
  - 文字协议明确允许顶层调用 `exec_command`。
  - Chat Completions/Responses 适配层丢失 outer `taskspace_exec` 身份或把内部结果改写成新 Function Call。
  - Agent 因 Map、节点归属或 shell 输出被裁剪而只能重新猜测；逃逸调用仍携带正确 `node_id` 和新命令。
  - Tool output 过长是必要触发条件；逃逸样本的反馈长度中位数并不高于正确样本。
  - `web_search` 和 `shell` 能证明当前有效协议下存在其他 Tool 逃逸；两者分别属于已删除和已回退设计。
- Fix criteria:
  - 候选必须消除或显著降低内部 `exec_command` 被理解为独立顶层 Function 的诱因，而不修改普通 Tool 原生合同、不让 Runtime 推断节点归属、不增加 Standard 分支。
  - 离线必须证明最终顶层 Tool 集合仍只有 `taskspace_exec` 与 Provider-hosted Tool，内部能力仍可完整机械路由。
  - 真实复验必须在同等复杂样本上覆盖首次初始化和后续工作，且不再出现任何顶层 client/action identity 逃逸；真实预算另行批准。
- Current conclusion: 已坐实的直接根因是 **outer/inner 作用域只存在于 `taskspace_exec` 参数结构和文字合同中，Provider 的 Function Call 返回通道不校验名称必须属于本轮顶层声明；模型因此可把一个内部 `exec_command` 对象错误序列化为顶层 Function Call**。这不是 Runtime 改写、上下文丢失或 Map 丢失：错误调用保留正确 `node_id`，适配器也保留 outer call identity。为什么几乎只选择 `exec_command`，当前证据支持三个放大因素：它是高频连续探索动作、两个 canonical work 示例都使用它、成功反馈再次裸露其 identity。统计关联很强，但三个因素的独立贡献尚无单变量 A/B，不能把其中任一项单独写成唯一根因。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
  - H-006
  - H-007
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
- Claim: DeepSeek Chat/Responses 兼容路径接受并返回了未在请求顶层 Tool 集合声明的 Function 名称，使模型的内外层混淆能够到达 Runtime。
- Layer: interaction
- Factor relation: all_of
- Depends on:
  - none
- Rationale:
  - 若 Provider 严格限制名称，当前有效协议下的 `function_call(name=exec_command)` 应在 Provider 边界被拒绝而不进入 rollout。
- Falsifiable predictions:
  - If true: wire 的顶层普通 client Tool 数为零，但 rollout 含完整的 `function_call(name=exec_command)`。
  - If false: 请求实际声明了 `exec_command`，或该 item 由本地 Runtime 合成。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对照 provider wire breakdown、rollout 原始 response item 和本地拒绝事件。
  - Signal: `tools_count=2`、`native_client_tool=0`、`function_call(name=exec_command)`。
  - Capture method: 读取历史四臂 provider wire、rollout 和 Chat streaming parser。
  - Event name or marker:
    - `provider.chat_wire_request_payload`
    - `function_call(name=exec_command)`
  - Correlation keys:
    - logical request ID
    - call ID
  - Differentiates from:
    - Runtime 重新暴露或改写 Tool
  - Supports if:
    - 未声明名称原样出现在 Provider response，并由 parser 不加本地改名地转成 `ResponseItem::FunctionCall`。
  - Refutes if:
    - 最终请求顶层包含 `exec_command`，或本地代码合成该调用。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - retain current wire and response traces
- Evidence gate: satisfied
- Related evidence:
  - E-006
  - E-014
  - E-015
  - E-019
- Conclusion: Provider 名称约束缺口是异常到达 Runtime 的必要条件，但不解释模型为何只偏向 `exec_command`；选择倾向仍由 H-002/H-005 调查。
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

## Hypothesis H-006: Provider 顶层 Function 通道没有结构性承载 TaskSpace 内外层作用域
- Status: confirmed
- Parent: P-001
- Claim: TaskSpace 把 client Tool 作为 `taskspace_exec` Function 参数中的结构化对象暴露，但 Provider 的模型输出仍只有普通顶层 Function Call 通道，且不校验返回名称必须属于本轮顶层声明；因此内外层作用域只能依靠模型遵循嵌套 JSON 和文字合同，错误提升可以成为合法 Provider response。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - H-003
- Rationale:
  - 请求顶层没有 `exec_command`，但 Provider 原样返回 `function_call(name=exec_command)`。
  - 逃逸参数是内部 action 的混合扁平化：保留 `node_id`，同时把 `input.cmd` 提升为 Function arguments。
  - outer output 的 `call_id` 在 Chat Completions 适配中保持不变，本地没有生成第二个调用。
- Falsifiable predictions:
  - If true: 同一顶层 schema 下可同时出现正确 outer 调用和未声明的内部名称提升；提升调用应保留内部对象信息，而不是由本地改写生成。
  - If false: 最终 Provider 请求实际声明了 `exec_command`，或适配器/Runtime 在收到 `taskspace_exec` 后创建了顶层 `exec_command`。
- Diagnostic evidence plan:
  - Prediction or clause under test: 检查最终 Tool 声明、Provider response item、outer output 到 chat message 的适配和 Runtime 拒绝路径。
  - Signal: `native_client_tool=0`、相同 capability hash、原 outer `tool_call_id`、Provider 返回的未知 Function name、零副作用拒绝。
  - Capture method: 四臂 provider wire、rollout、`codex-api/src/endpoint/chat_completions.rs` 和 `core/src/tools/parallel.rs` 静态链路。
  - Event name or marker:
    - `provider.chat_wire_prefix_preserved`
    - `function_call(name=exec_command)`
  - Correlation keys:
    - provider logical request ID
    - outer and escaped call IDs
  - Differentiates from:
    - Runtime 提升
    - 适配器丢失 outer identity
    - Map/context 丢失
  - Supports if:
    - 顶层声明不含 `exec_command`，outer output 保持配对，Provider 后续仍返回新的顶层 `exec_command`。
  - Refutes if:
    - 任一中间层重新声明、改名或合成该调用。
  - Instrumentation status: existing permanent traces sufficient
  - Instrumentation lifecycle:
    - retain
- Evidence gate: satisfied
- Related evidence:
  - E-014
  - E-015
  - E-016
- Conclusion: 这是顶层提升能够发生的已证实结构机制。它解释“如何逃逸”，但不单独解释模型为什么优先提升 `exec_command`；后者由 H-002/H-005 继续描述为待隔离放大因素。
- Repair design readiness: ready for isolated candidate design, not yet authorized
- Next step: 在不更改原生 Tool identity、Standard、DAG 和 Runtime 归属权的前提下，隔离最小模型可见作用域表达变量。
- Blocker:
  - 修复候选和真实 A/B 预算尚未由用户选择。
- Close reason:
  - not closed

## Evidence E-014: Chat 适配保留 outer taskspace_exec 身份
- Related hypotheses:
  - H-002
  - H-006
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/codex-api/src/endpoint/chat_completions.rs:44-67`
- Prediction or plan link:
  - H-006 排除适配器把内部结果提升为顶层调用。
- Matched signal:
  - `ResponseItem::FunctionCallOutput` 只生成 `role=tool`、原 `tool_call_id=call_id` 和文本 content；不读取 content 内的 `client_results[].tool`，也不创建 Function Call。
- Correlation keys:
  - outer taskspace_exec call ID
- Raw content:
  ```text
  role = tool
  tool_call_id = outer taskspace_exec call_id
  content = serialized TaskSpace Exec result
  ```
- Interpretation: 内部 `tool="exec_command"` 只是 outer result content；后续顶层调用来自模型的新响应，不是本地改写。
- Time: 2026-08-19 14:40

## Evidence E-015: 相同顶层合同下同时存在逃逸与正确调用
- Related hypotheses:
  - H-005
  - H-006
- Direction: supports
- Type: controlled-historical-comparison
- Source: `WAR-20260818-055427-R8-FOUR-ARM-R3` a2/r1 and a1/r3 provider wire
- Prediction or plan link:
  - 区分确定性 request-shape 错误与模型选择波动。
- Matched signal:
  - 两轮使用相同 Base `3.0.5` hash、相同 TaskSpace capability identity、相同 tools hash、`tools_count=2` 和 `tool_choice=auto`。
  - a2/r1 初始化后生成顶层 `exec_command`；a1/r3 在相同成功结果形状后继续正确调用 `taskspace_exec`。
- Correlation keys:
  - base sha256 `e2f81354...`
  - capability `05b41a6b...`
  - tools hash `c47aafee...`
- Raw content:
  ```text
  affected:   taskspace_exec -> succeeded exec_command result -> top-level exec_command
  unaffected: taskspace_exec -> succeeded exec_command result -> taskspace_exec(work/exec_command)
  ```
- Interpretation: 合同允许模型稳定走正确路径，但不能结构性阻止未声明名称；症状是概率性生成错误，不是某个 projection 模式必然改写。
- Time: 2026-08-19 14:40

## Evidence E-016: 逃逸调用保留正确 node_id
- Related hypotheses:
  - H-005
  - H-006
- Direction: supports
- Type: diagnostic-log
- Source: four-arm affected rollouts
- Prediction or plan link:
  - 排除 Map/节点归属上下文丢失。
- Matched signal:
  - 典型错误调用为 `function_call(name="exec_command", arguments={"cmd":"...","node_id":"explore"})`；`node_id` 与当前 in-flight owner 一致。
- Correlation keys:
  - `call_00_rqNYN6QkxN6ZwBtP5JjY0660`
- Raw content:
  ```text
  {"cmd":"cat /workspace/README.md ...","node_id":"explore"}
  ```
- Interpretation: Agent掌握了动作内容和归属，只把调用包装层级选错；不能通过增强 Map 状态反馈解释或修复。
- Time: 2026-08-19 14:40

## Evidence E-017: exec-to-exec 和初始化后续显著集中逃逸
- Related hypotheses:
  - H-002
  - H-005
- Direction: supports
- Type: historical-transition-analysis
- Source: 可解析当前 `tools[]` 协议的本地 rollout
- Prediction or plan link:
  - H-005 若与 `exec_command` 特有连续调用和首轮位置相关，转换率应明显高于 Patch 后验证或普通 work。
- Matched signal:
  - 成功 `exec_command` 后下一意图仍为 `exec_command`：18/199 被提升，181/199 正确包裹。
  - 成功 `apply_patch` 后下一意图为 `exec_command`：0/102 被提升。
  - `initialize_and_work` 后继续 `exec_command`：13/105；普通 `work` 后继续 `exec_command`：1/81。
  - 探索性单侧 Fisher 值分别约为 `0.00044` 和 `0.00297`；由于历史运行并非独立同分布实验，只用于确认集中性，不作为精确总体概率。
- Correlation keys:
  - previous outer call ID
  - next Function Call
- Raw content:
  ```text
  exec success -> exec intent: 18 escape / 181 wrapped
  patch success -> exec intent: 0 escape / 102 wrapped
  initialize continuation: 13/105 escape
  ordinary work continuation: 1/81 escape
  ```
- Interpretation: 问题不是“任何下一步 shell 都会逃逸”，而是高度集中在成功 shell 的连续调用，尤其 Map 初始化后的第一次探索续步。
- Time: 2026-08-19 14:40

## Evidence E-018: 反馈长度不是必要触发条件
- Related hypotheses:
  - H-002
  - H-005
- Direction: neutral
- Type: historical-distribution-analysis
- Source: exec-to-exec transition dataset
- Prediction or plan link:
  - 排除 output 过长导致 outer 作用域提示被冲淡。
- Matched signal:
  - 逃逸样本 outer feedback 中位数约 1536 B，正确包裹样本约 1797 B；逃逸组并不更长。
- Correlation keys:
  - previous outer result
- Raw content:
  ```text
  escaped median outer feedback: 1536 B
  wrapped median outer feedback: 1797 B
  ```
- Interpretation: 裁剪结果长度不是当前根因方向；需要研究 identity/作用域表达，而非压缩更多语义。
- Time: 2026-08-19 14:40

## Evidence E-019: Provider response parser 原样接受未声明 Function 名称
- Related hypotheses:
  - H-003
  - H-006
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/codex-api/src/sse/chat_completions.rs:186-245`
- Prediction or plan link:
  - H-003/H-006 区分 Provider 返回与 Runtime 本地合成。
- Matched signal:
  - streaming parser 从 `delta.tool_calls[].function.name` 直接保存名称，并在响应结束时生成同名 `ResponseItem::FunctionCall`；该路径不按请求 Tool catalog 校验或改名。
- Correlation keys:
  - Provider tool call index and call ID
- Raw content:
  ```text
  state.name = function.name
  ResponseItem::FunctionCall { name, arguments, call_id }
  ```
- Interpretation: rollout 中的顶层 `exec_command` 是 Provider 模型响应的原始调用选择；本地第一次语义处理是后续 TaskSpace 零副作用拒绝。
- Time: 2026-08-19 14:40

## Hypothesis H-007: 成功反馈中的裸 client Tool 字段是可隔离放大因素
- Status: rejected
- Parent: P-001
- Claim: 在请求侧协议完全不变时，把成功结果中的 `tool="exec_command"` 改为明确的已执行 client Tool 作用域表达，会降低后续连续 shell 被提升为顶层 Function Call 的概率。
- Layer: interaction
- Factor relation: part_of
- Depends on:
  - H-002
  - H-006
- Rationale:
  - 逃逸集中发生在成功 `exec_command` 后继续 `exec_command`；Patch 后执行 shell 未出现同类行为。
  - outer call identity 没有丢失，因此只需要隔离 result content 的显著性，不应改请求、Runtime 或 Map。
- Falsifiable predictions:
  - If true: 同版本复杂样本中，F1 的顶层 `exec_command` 逃逸低于 B0，且业务、Map 和其他协议行为不下降。
  - If false: F1 逃逸不降、转为其他顶层名称，或引入新的结果理解错误。
- Diagnostic evidence plan:
  - Prediction or clause under test: 只改变 Agent-visible result 字段名，保持值、顺序、原生结果和请求 wire 不变。
  - Signal: 每 run 逃逸 call、完成结果、请求/token/cache、其他拒绝类型。
  - Capture method: `subscription-billing-repair × map-request × B0/F1 × repeat=5` Docker trace；F2 仅在 F1 不通过时执行。
  - Event name or marker:
    - `function_call(name=exec_command)`
    - `taskspace.exec.request_started`
  - Correlation keys:
    - run record ID
    - provider request ID
    - outer call ID
  - Differentiates from:
    - canonical 示例影响
    - Base 指令影响
    - Runtime/Map 状态影响
  - Supports if:
    - F1 逃逸明显下降，且没有新协议错误。
  - Refutes if:
    - F1 无改善或错误迁移。
  - Instrumentation status: existing permanent traces sufficient
  - Instrumentation lifecycle:
    - retain
- Evidence gate: satisfied
- Related evidence:
  - E-011
  - E-017
  - E-020
- Conclusion: 被证伪。B0 与 F1 的全部逃逸都在首个 `initialize_and_work` 响应中产生，早于任何 client result；F1 的 3/5 run 与 B0 相同，call 数 6 到 4 的变化不能建立因果关系。
- Repair design readiness: rejected; F1 reverted
- Next step: 不再修改后续结果 identity；转向首请求可见的协议与示例。
- Blocker:
  - none
- Close reason:
  - candidate refuted by temporal and controlled-run evidence

## Evidence E-020: 反馈字段实验不能解释首轮逃逸
- Related hypotheses:
  - H-007
- Direction: refutes
- Type: controlled-experiment
- Source: `WAR-20260819-084538-R8-EXEC-FEEDBACK-SCOPE-AB` B0/F1
- Prediction or plan link:
  - H-007 要求成功反馈字段先出现，随后才放大下一轮调用。
- Matched signal:
  - B0 为 3/5 run、6 call；F1 为 3/5 run、4 call；两臂 10 次业务与隐藏验证全部通过。
  - 两臂全部逃逸都与首个 `initialize_and_work` 同响应产生，早于任何 `client_results`。
- Correlation keys:
  - experiment arm and run index
  - first provider response
- Raw content:
  ```text
  B0: 3/5 escaped runs, 6 calls
  F1: 3/5 escaped runs, 4 calls
  escaped position: first response, before successful client feedback
  ```
- Interpretation: 成功反馈既不是首轮逃逸的必要条件，也不是本实验观察到的直接诱因；F2 若仍只修改后续反馈，同样不具备因果可行性。
- Time: 2026-08-19 09:00

## Hypothesis H-008: client Tool work 示例放大内部 action 的顶层提升
- Status: rejected
- Parent: P-001
- Claim: `taskspace_exec` description 中两段显式 `exec_command` work 示例，使模型在首响应正确生成 outer sequence 的同时，更容易把后续同类内部 action 另写为顶层 sibling Function Call。
- Layer: interaction
- Factor relation: part_of
- Depends on:
  - H-006
- Rationale:
  - 全部实验逃逸都在首响应发生，必须由首请求中已有信息解释。
  - description 有两段完整 `tools:[{tool:"exec_command",...}]` 示例；没有其他 client Tool 的同类示例。
  - 当前有效逃逸只涉及 `exec_command`，没有 `apply_patch` 顶层逃逸。
- Falsifiable predictions:
  - If true: 仅移除两段 work 示例后，首响应顶层 `exec_command` 逃逸明显低于 B0，且 schema 驱动的初始化和合法序列仍稳定。
  - If false: 逃逸不降，或只表现为正常随机波动。
- Diagnostic evidence plan:
  - Prediction or clause under test: E1 只移除两段 client work 示例，保留闭集 schema、协议正文、read/finish 示例和所有运行逻辑。
  - Signal: 首响应逃逸 run/call、初始化成功、业务/隐藏验证、Map 闭合、其他协议异常。
  - Capture method: 同一二进制构建流程与 `subscription-billing-repair × map-request × repeat=5`。
  - Event name or marker:
    - `function_call(name=exec_command)`
    - first `initialize_and_work`
  - Correlation keys:
    - experiment arm and run index
    - provider first response
  - Differentiates from:
    - 后续反馈显著性
    - Base instructions
    - Runtime 拒绝逻辑
  - Supports if:
    - E1 首响应逃逸清晰下降且无初始化或业务回归。
  - Refutes if:
    - 逃逸无清晰下降或错误迁移。
  - Instrumentation status: existing permanent traces sufficient
  - Instrumentation lifecycle:
    - retain
- Evidence gate: satisfied
- Related evidence:
  - E-020
  - E-021
- Conclusion: 未获得支持。E1 从 3/5 变为 2/5 escaped runs，但 escaped calls 仍为 6，且 5 次中后两次各集中出现 3 call；该变化不足以建立示例为因果放大因素。
- Repair design readiness: rejected; E1 reverted
- Next step: 保持生产 description 不变，转向首响应 Function 选择机制与结构约束。
- Blocker:
  - none
- Close reason:
  - candidate did not produce a clear directional improvement

## Evidence E-021: 移除 client work 示例没有降低逃逸总量
- Related hypotheses:
  - H-008
- Direction: refutes
- Type: controlled-experiment
- Source: `WAR-20260819-084538-R8-EXEC-FEEDBACK-SCOPE-AB` B0/E1
- Prediction or plan link:
  - H-008 要求只移除两段 work 示例后，首响应逃逸明显下降且无错误迁移。
- Matched signal:
  - B0 为 3/5 run、6 call；E1 为 2/5 run、6 call。
  - E1 前三次为 0 call，后两次各 3 call；15 次实验的业务、公开、隐藏验证和 Map 闭合全部通过。
- Correlation keys:
  - experiment arm and run index
  - first provider response
- Raw content:
  ```text
  B0 escape calls/run: [1,0,2,0,3]
  E1 escape calls/run: [0,0,0,3,3]
  ```
- Interpretation: 示例不是逃逸发生的必要条件；run 数的轻微变化不能覆盖相同的总 escape call 数和明显的批内波动。
- Time: 2026-08-19 09:08

## Hypothesis H-009: 内层 exec_command 的局部作用域合同可降低顶层提升
- Status: rejected
- Parent: P-001
- Claim: 只在 TaskSpace Exec 内部 catalog 的 `exec_command` description 中明确其作用域，可降低模型把该内部 action 另写为顶层 Function Call 的概率，同时不改变 Standard Tool 合同和其他内部 Tool。
- Layer: interaction
- Factor relation: part_of
- Depends on:
  - H-006
- Rationale:
  - 逃逸全部发生在首响应，候选必须作用于首请求已有的 Tool schema，而不能依赖后续反馈。
  - 当前有效设计只有 `exec_command` 出现同型逃逸，不支持扩大为所有内部 Tool 的通用协议。
  - `exec_command` 的原生输入 schema 和 identity 必须保留；最小候选是在其 TaskSpace 内层 description 中补足 outer/inner 作用域。
- Falsifiable predictions:
  - If true: 同版本复杂样本中，首响应顶层 `exec_command` 逃逸相对 B0 明显下降，且初始化、业务、Map 和其他 Tool 行为不回归。
  - If false: 逃逸频率不降、错误迁移到其他 Tool，或 Agent 无法正常构造内层 `exec_command`。
- Diagnostic evidence plan:
  - Prediction or clause under test: 仅为 TaskSpace 内层 `exec_command` 增加“只能位于 `taskspace_exec.tools`，不得作为顶层 Function Tool call”描述。
  - Signal: 首响应逃逸 run/call、合法内层调用、业务/oracle、Map 闭合、请求/token/cache 和其他拒绝类型。
  - Capture method: 使用与 B0 相同的复杂样本、projection、模型与 repeat 口径；真实运行须另行获得预算批准。
  - Event name or marker:
    - `function_call(name=exec_command)`
    - first `initialize_and_work`
  - Correlation keys:
    - experiment arm and run index
    - provider first response
  - Differentiates from:
    - 成功反馈字段
    - work 示例
    - Base instructions
    - Runtime 拒绝逻辑
  - Supports if:
    - 逃逸相对 B0 明显下降，且没有新协议错误或业务回归。
  - Refutes if:
    - 逃逸无清晰下降，或错误迁移、合法内层调用下降。
  - Instrumentation status: existing permanent traces sufficient
  - Instrumentation lifecycle:
    - retain
- Evidence gate: satisfied
- Related evidence:
  - E-022
- E-023
- Conclusion: 被证伪。本轮为 4/5 escaped runs、6 calls，历史 B0 为 3/5、6 calls；局部 description 没有降低逃逸总量或发生率。
- Repair design readiness: rejected; candidate reverted
- Next step: 不在该 description 候选上继续堆叠文字；转向可结构性限制首响应 Function 名的机制，或重评 outer/inner Function 表达模型。
- Blocker:
  - none
- Close reason:
  - live repeat=5 did not improve escaped runs or calls

## Evidence E-022: TaskSpace 内层作用域合同已离线落地
- Related hypotheses:
  - H-009
- Direction: supports
- Type: implementation-verification
- Source: commit `bf0b7cc8b`
- Prediction or plan link:
  - H-009 要求只改变 TaskSpace 内层 `exec_command` description，保持 Standard 和其他内部 Tool 不变。
- Matched signal:
  - `sequence_schema.rs` 仅对无 namespace 且名称为 `exec_command` 的 TaskSpace catalog capability 追加局部作用域描述。
  - catalog 测试确认内层描述包含 `only inside the taskspace_exec.tools array` 和 `never emit exec_command as a top-level Function Tool call`。
  - 同一测试确认其他内部 Tool description 保持原值；Standard/native ToolSpec 构建路径未修改。
  - `cargo test -p codex-core taskspace_exec --lib --locked` 为 78 passed、0 failed。
  - 缓存门禁以指纹 `138a9410a2a9e20444b529c590b43a555728fbbb2d6825721f6c9cb2937e0d9f` 免费 final-wire 验证通过；发布仍等待真实回归证据。
- Correlation keys:
  - source commit
  - cache-sensitive fingerprint
- Raw content:
  ```text
  In TaskSpace, use `exec_command` only inside the `taskspace_exec.tools` array;
  never emit `exec_command` as a top-level Function Tool call.
  taskspace_exec tests: 78 passed, 0 failed
  cache gate: free final-wire PASS; release remains blocked
  ```
- Interpretation: 候选的工程作用域和离线正确性已验证；该证据不包含目标模型逃逸频率，不能关闭 H-009 或 I03。
- Time: 2026-08-19

## Evidence E-023: 内层作用域描述没有降低首响应逃逸
- Related hypotheses:
  - H-009
- Direction: refutes
- Type: controlled-experiment
- Source: `WAR-20260819-220821-R8-EXEC-SCOPE-DESCRIPTION-R5`
- Prediction or plan link:
  - H-009 要求 escaped runs/calls 相对 B0 明显下降，同时业务、Map 和合法内层调用不回归。
- Matched signal:
  - 候选 5/5 业务、公开验证、hidden oracle 和 Map 闭合通过。
  - 候选逃逸为 4/5 runs、6 calls、逐轮 `[3,0,1,1,1]`；B0 为 3/5、6 calls、逐轮 `[1,0,2,0,3]`。
  - 六次逃逸全部位于首个 `initialize_and_work` 同响应，早于任何 Tool feedback。
  - 五轮 Provider wire 的 client catalog 均比 B0 增加 140 bytes 且 hash 一致，证明候选实际进入请求。
- Correlation keys:
  - run index
  - provider first response
  - tools hash
- Raw content:
  ```text
  H-009: 4/5 escaped runs, 6 calls, [3,0,1,1,1]
  B0:    3/5 escaped runs, 6 calls, [1,0,2,0,3]
  candidate client catalog: 16164 bytes
  B0 client catalog:        16024 bytes
  ```
- Interpretation: 结果排除了“改动没有进入 Provider 请求”，并直接否定局部 description 足以约束首响应 Function 选择；请求/token 波动不构成候选收益。
- Time: 2026-08-19 22:14

## Hypothesis H-010: Base 中显式点名内外层 Tool 可抑制顶层提升
- Status: confirmed
- Parent: P-001
- Claim: 将 TaskSpace Base 的宏观唯一入口规则替换为显式点名 `taskspace_exec` 与 `exec_command` 的层级合同，可降低模型在首响应把内部 client Tool 提升为顶层 Function Call 的概率。
- Layer: interaction
- Factor relation: part_of
- Depends on:
  - H-006
- Rationale:
  - Base `3.0.5` 已有未点名 Tool identity 的宏观唯一入口规则，但 B0 与 H-009 仍高频逃逸。
  - H-009 证明嵌套 Tool description 强度不足；同类严格状态机规则放在 Base 时曾表现出更高遵循度。
  - 替换原句而非叠加，保持单一合同来源并避免提示词膨胀。
- Falsifiable predictions:
  - If true: 同一复杂样本中，顶层 `exec_command` 逃逸相对 B0/H-009 明显下降，业务、Map 和总体非法动作不回归。
  - If false: 逃逸频率不降，或错误迁移使总体非法动作、业务失败或 Map 不闭合上升。
- Diagnostic evidence plan:
  - Prediction or clause under test: TaskSpace Base `3.0.6` 只替换 client Tool 层级规则，其他模型可见合同和 Runtime 不变。
  - Signal: Base wire identity、逃逸 run/call、所有拒绝、业务/oracle、Map、请求/token/cache/time/cost。
  - Capture method: `subscription-billing-repair × map-request × repeat=5`，Docker，DeepSeek V4 Flash。
  - Event name or marker:
    - `function_call(name=exec_command)`
    - base instructions identity
  - Correlation keys:
    - run index
    - provider first response
    - Base version and SHA-256
  - Supports if:
    - 逃逸清晰下降且无正确性或总体非法动作回归。
  - Refutes if:
    - 逃逸无下降或错误迁移抵消收益。
  - Instrumentation status: existing permanent traces sufficient
  - Instrumentation lifecycle:
    - retain
- Evidence gate: satisfied
- Related evidence:
  - E-024
- Conclusion: 当前样本 repeat=5 中逃逸从 B0 3/5 runs、6 calls 和 H-009 4/5、6 calls 降至 0/5、0 calls；5/5 业务/oracle/Map 通过，总非法动作降至 4 次。候选获得支持并保留。
- Repair design readiness: implemented and retained; broader validation pending
- Next step: 在后续自然复杂验收中观察跨样本复发，不继续叠加同类文字。
- Blocker:
  - none

## Evidence E-024: Base 3.0.6 五轮未发生顶层 client Tool 逃逸
- Related hypotheses:
  - H-010
- Direction: supports
- Type: controlled-experiment
- Source: `WAR-20260819-223533-R8-BASE-CLIENT-SCOPE-R5`
- Prediction or plan link:
  - H-010 要求明确 Base 层级合同降低逃逸且不产生正确性或总体非法动作回归。
- Matched signal:
  - 五轮 Provider wire 均为 TaskSpace Base `3.0.6`、预期 hash、`matches_current_contract=true`。
  - 顶层 `exec_command` 为 0/5 runs、0 calls；全部 client work 位于 `taskspace_exec`。
  - 5/5 业务、公开验证、hidden oracle 和 Map 闭合通过。
  - 剩余两次 JSON syntax 与两次 Waiting preflight 均零副作用恢复；总体非法动作 4 次，低于 B0 的 14 次和 H-009 的 8 次。
- Correlation keys:
  - run index
  - Base identity
  - provider first response
- Raw content:
  ```text
  Base 3.0.6: 0/5 escaped runs, 0 calls, 4 total invalid actions
  H-009:      4/5 escaped runs, 6 calls, 8 total invalid actions
  B0:         3/5 escaped runs, 6 calls, 14 total invalid actions
  business/oracle/Map: 5/5 passed
  ```
- Interpretation: 显式 Base 层级合同在当前样本上产生清晰正向方向，且没有观察到错误迁移抵消；证据规模不足以关闭跨样本 I03。
- Time: 2026-08-19 22:40

## Evidence E-025: 当前生产代码未重新暴露顶层 client Tool
- Related hypotheses:
  - H-010
- Direction: supports
- Type: static-chain-revalidation
- Source: commit `50067e581`
- Matched signal:
  - `ToolRouter::into_taskspace` 只把 `taskspace_exec` 以及原生 Hosted Tool 放入模型可见的顶层 Tool 集合；`exec_command`、`write_stdin` 等 client Tool 只存在于 Exec 内部 catalog。
  - TaskSpace Base `3.0.6` 明确要求 `exec_command` 只能出现在 `taskspace_exec` 的内部序列，不能作为顶层 Function Tool。
  - Runtime 对 Provider 返回的未声明顶层 client Tool 在执行前 fail closed，不执行 Tool，也不提交 Map 候选状态。
  - 三项当前代码测试通过：`taskspace_cp01_records_deepseek_effective_surface`、`taskspace_base_excludes_standard_plan_and_direct_client_tool_contracts`、`response_rejects_forbidden_top_level_client_call_before_exec_claim`。
- Interpretation:
  - 当前代码不存在把 `exec_command` 偷偷重新加入顶层 schema 的工程回归。
  - 历史逃逸仍应解释为 Provider 偶发返回未声明 Function 名称，而不是 Runtime 提升、Map/context 丢失或 Tool schema 重复暴露。
  - Base `3.0.6` 是当前唯一获得在线支持且已经保留的缓解措施；在获得跨样本证据前，不继续增加同类提示文字，也不改写原生 client Tool identity。
- Remaining evidence gap:
  - 仍需在不同自然复杂样本中观察 Base `3.0.6` 是否持续抑制顶层 client Tool 逃逸；该缺口只影响 I03 的跨样本关闭，不影响当前硬边界正确性。
- Time: 2026-08-20

## Evidence E-026: Base 3.0.6 在第二个复杂样本三轮无顶层逃逸
- Related hypotheses:
  - H-010
- Direction: supports
- Type: cross-sample-production-validation
- Source: `WAR-20260820-023538-R8-I03-CROSS-SAMPLE-R3`
- Matched signal:
  - `release-dispatch-repair × map-request × repeat=3` 的 Provider wire 均为 Base `3.0.6` 和预期 hash。
  - 3/3 业务、公开验证、hidden oracle 和 Map 闭合通过。
  - 23 次顶层 Function Call 全部为 `taskspace_exec`；顶层 client Tool 逃逸为 0/3 runs、0 calls。
  - 连同 E-024，两个复杂样本累计 8/8 无顶层 client Tool 逃逸。
  - Run 1 仍有两次可恢复的 Exec envelope 拒绝；它们不推翻 H-010，但阻止关闭整个 I03。
- Interpretation: H-010 已获得跨样本支持，Base `3.0.6` 保留且不再叠加同义提示。I03 的剩余工作收敛为 Exec envelope 参数稳定性，不再把顶层 schema 重暴露或 Runtime 提升作为开放根因。
- Time: 2026-08-20

## Hypothesis H-011: apply_patch 的复合坏形状可唯一机械归一化
- Status: confirmed
- Claim: E-026 Run 1 的第二次拒绝同时包含一个缺失 action 闭合符和一个 Function 风格的
  `apply_patch.input.cmd` 包装；若只在错误邻域插入一个 `}`，再仅对完整 Patch 标记下的唯一包装候选做机械展开，
  则可得到唯一通过当前 Catalog 完整解码的原 Agent 动作，不需要 Runtime 推断节点、序列或业务语义。
- Predictions:
  - 原始坏形状不能被现有单闭合符自愈器接受。
  - 符合完整 Patch 标记、唯一 `cmd/node_id/tool` 形状的候选可恢复为同一 `node_id`、同一 Patch 正文和
    freeform `apply_patch`。
  - 非 Patch `cmd`、多个合法候选或最终不能通过 Catalog 解码的输入保持拒绝。
- Diagnostic evidence plan:
  - 用原始 trace 的 `update_and_work + input.cmd + 缺 action }` 形状建立确定性回归。
  - 以 `TaskSpaceExecCatalog::decode_plan` 作为最终语义不变门禁，并增加非 Patch 反例。
- Fix boundary:
  - 不补缺失的 sequence `type`，因为选择序列属于 Agent 决策。
  - 不接受通用 Function-to-freeform 猜测，不改变 Tool schema、Base instructions 或 Provider context。

## Evidence E-027: apply_patch 复合坏形状已在唯一解码门禁下自愈
- Related hypotheses:
  - H-011
- Direction: supports
- Type: deterministic-fix-validation
- Source: commit `3eeaeac3c`
- Matched signal:
  - 新回归先证明历史形状在旧实现下返回 `None`，随后验证修复操作为
    `normalize/apply_patch_wrapper`。
  - 修复后 plan 保持 `update_and_work`、`node_id=fix`、`tool=apply_patch` 和逐字相同 Patch 正文。
  - 非 Patch `cmd` 包装继续拒绝；所有候选仍须全局唯一并通过当前 Catalog 完整解码。
  - TaskSpace Exec 82/82、两项正式历史替换测试和缓存敏感面门禁通过。
- Interpretation: E-026 的第二个错误已获得确定性离线修复；它不关闭 I03，因为缺失 `type` 仍开放，且新分支尚未自然在线命中。
- Time: 2026-08-20

## Hypothesis H-012: Base 明确 sequence type 责任可降低缺失字段

- Status: supported
- Claim: `type` 决定 `taskspace_exec` 选择哪一个 schema 已定义合法序列，属于 Agent 必须显式作出的动作选择；若
  TaskSpace Base 明确要求每次调用都携带该字段，并说明 Runtime 不代填，则缺失 `type` 应减少，而不改变序列、状态机或
  Runtime 决策边界。
- Predictions:
  - Provider-visible Base identity 必须为 `3.0.7` 和当前 hash。
  - 每次 `taskspace_exec` 都存在 schema 接受的顶层 `type`。
  - 业务、hidden oracle、Map 闭环和其他 Exec reject 不回归。
- Fix boundary:
  - 不由 Runtime 推断或补写 `type`。
  - 不新增合法序列，不改变状态转换，不重复 Tool schema 的字段定义。

## Evidence E-028: Base 3.0.7 三轮共 18 次 Exec 均显式携带 type

- Related hypotheses:
  - H-012
- Direction: supports
- Type: controlled-production-validation
- Source: `WAR-20260820-050331-R8-BASE307-TYPE-R5`
- Matched signal:
  - 三轮 TaskSpace Provider wire 均为 Base `3.0.7`、预期 hash 和 `matches_current_contract=true`。
  - 18/18 次 `taskspace_exec` 显式携带 `type`；缺失、语法、合同、状态和执行拒绝均为 0。
  - 3/3 业务、公开验证、hidden oracle 和 Map 闭环通过。
  - 同一样本上一批 Base `3.0.6` 为 1/3 运行缺失 `type`，本轮为 0/3。
- Limitation:
  - Pair runner 交替逻辑模式，实际矩阵为 Standard 3 轮与 TaskSpace 3 轮，而非计划的 1+5。
  - 三轮只支持保留候选，不关闭整个 I03，也不晋升缓存基线。
- Interpretation: H-012 在当前复杂样本获得在线支持。该结果证明提示层补足 Agent 责任有效，不构成 Runtime 语义接管。
- Time: 2026-08-20

## Hypothesis H-013: sole Tool identity 未表达每响应 outer call 基数

- Status: confirmed
- Claim: “`taskspace_exec` 是唯一顶层 Function Tool”只约束可调用的 Tool 名称，不约束同一响应中该 Tool 的调用次数；
  若 Agent-visible Base/Tool protocol 未明确“一响应恰好一次 outer Exec，多个 client action 放入同一 `tools[]`”，模型可能
  把可并行工作生成为多个 sibling outer Exec。
- Predictions:
  - 复发响应可包含多个同名 `taskspace_exec`，而不是不同 Tool 逃逸。
  - 每个 sibling call 自身可以拥有合法 `type`、`tools[]` 和 `node_id`。
  - response scope 在任何一个 sibling 执行前整批拒绝，证明问题位于 Agent 动作组织而非 Runtime dispatch。
- Fix boundary:
  - 不由 Runtime 合并、重排或选择 sibling call。
  - 不改变普通 client Tool schema，不重新暴露顶层 client Tool。
  - 候选只补足已有冻结不变量的 Agent-visible表达，并单变量验证。

## Evidence E-029: Repeat 10 首轮生成两个 sibling outer Exec

- Related hypotheses:
  - H-013
- Direction: supports
- Type: production-failure-trace
- Source: `WAR-20260820-060735-R8-BASE307-TYPE-R10`
- Matched signal:
  - Request 1 的单个 `initialize_and_work` 被接受并完成一个 `inspect` client action。
  - Request 2 同时返回两个名为 `taskspace_exec` 的 Function Call；二者均为 `type=work`、owner=`inspect`，分别读取实现与测试。
  - response scope 返回 `TaskSpace response contains more than one Exec call`，整批零执行，Agent fatal，未产生 Patch。
  - Base identity 为 `3.0.7` 且两个 sibling 均有 `type`，因此该失败不否定 H-012。
- Interpretation: H-013 坐实。扩大验收按业务硬失败条件在 1/10 后停止；继续原样采样不能替代协议修复。
- Time: 2026-08-20

## Evidence E-030: Base 3.0.8 七轮未再生成 sibling outer Exec

- Related hypotheses:
  - H-013
- Direction: supports fix
- Type: controlled-production-validation-with-runner-deviation
- Source: `WAR-20260820-062550-R8-BASE308-OUTER-R10`
- Matched signal:
  - Base 与 Tool protocol 明确一响应只生成一个 outer `taskspace_exec`，多个 client action 合并进同一 `tools[]`；Runtime
    仍只拒绝，不合并或重排。
  - 7 次 TaskSpace 共 59 个 Provider 响应、53 个 outer Exec call，`exec_call_count > 1` 为 0，多 outer Exec 拒绝为 0。
  - 七次代码、公开验证、hidden oracle 和 Map 均完成；其中六次形成完整 final lifecycle，第七次 Map 闭合后生成自然语言
    final 所需请求触及单轮 10-request 上限。
  - 仍观察到两次 JSON syntax、一次顶层 `exec_command` 逃逸和一次缺失 `type`，均为 I03 的独立开放表现。
- Limitation:
  - `RunSide=right` 实际选择物理侧而不是 TaskSpace 逻辑模式，额外执行 6 次 Standard；批次超过已登记的 sample、request 和
    input 上限，费用仍低于 CNY 0.45。发现后未补跑剩余三次 TaskSpace。
- Interpretation: H-013 的协议修复在当前七次 TaskSpace 观察中成立；该证据不能写成 repeat=10 完成，也不能关闭整个 I03。
- Time: 2026-08-20

## Hypothesis H-014: 两次 Base 3.0.8 syntax 是可唯一组合修复的同形 Patch 错误

- Status: confirmed offline
- Parent: P-001
- Claim: Base 3.0.8 批次中的两次 JSON syntax reject 都由完整 `apply_patch.input` 内一个裸 LF 和 action 尾部一个
  多余 `}` 共同造成；若按固定顺序转义 LF、删除唯一多余括号，并要求完整 Catalog 解码后只有一个结果，则可机械自愈，
  无需扩大 Runtime 语义职责。
- Falsifiable predictions:
  - 两条原始参数在只转义 LF 后仍失败；再删除一个多余 `}` 后都通过完整 Exec decoder。
  - 两个多余括号、非完整 Patch 或多个有效结果必须保持拒绝。
  - 修复沿现有 ResponseItem 自愈入口发生，正式历史不会记录错误参数。
- Fix boundary:
  - 不建设通用多错误 JSON 修复器。
  - 不推断 Tool、节点、序列或状态语义。
  - 不因离线修复关闭 I03 的 Agent 行为问题。

## Evidence E-031: 两条原始坏形状可由唯一窄候选恢复

- Related hypotheses:
  - H-014
- Direction: supports
- Type: exact-trace-replay-and-deterministic-regression
- Source: `WAR-20260820-062550-R8-BASE308-OUTER-R10` pair 003 / pair 005
- Matched signal:
  - `call_00_9UASnGQQN5pDkfZjyubT5948` 与 `call_00_x5nWuQlNxbrVPUxGyxnd0011` 均只有一个 Patch 内裸 LF 和一个
    action 尾部多余 `}`。
  - 两条参数在转义 LF 并删除一个 `}` 后均通过当前 Catalog 完整解码。
  - 新正向、两个多余括号负向和既有其他复合错误拒绝测试均通过；正式历史替换链已有独立回归。
- Limitation:
  - 未执行新的真实 Agent run，不能证明在线发生频率下降或其他非法参数被覆盖。
- Interpretation: H-014 的工程根因和窄修复坐实；I03 继续 `verifying`。
- Time: 2026-08-20

## Hypothesis H-015: 同响应多 client action 存在单项顶层提升

- Status: supported
- Parent: P-001
- Claim: 当 Agent 在同一响应中组织多个独立 client action 时，模型可能正确生成一个 outer
  `taskspace_exec`，却把其中一个 action 错误序列化为 sibling 顶层 Function Call；这不是 Map、节点归属或 Tool
  结果丢失。
- Falsifiable predictions:
  - 失败响应同时包含一个合法 outer Exec 和一个未声明的 sibling client Tool。
  - sibling 仍携带正确 `node_id` 和新的原生 Tool input，说明 Agent 已经形成动作与归属，只是放错结构层级。
  - Runtime 拒绝后，Agent 能明确指出层级错误，并把同样的两个动作放回一个 `tools[]` 后成功执行。
  - 相同协议下应存在把两个动作正确放入同一个 `tools[]` 的对照响应，排除 schema 本身无法表达。
- Diagnostic evidence plan:
  - 对 Base 3.0.8 的 pair 007 原始 rollout 做响应级结构检查。
  - 用 pair 009、pair 011 的同类双读取响应作为自然对照。
- Evidence gate: satisfied for mechanism; trigger probability remains open
- Related evidence:
  - E-032
- Conclusion: 失败机制已由原始响应坐实，但“为什么偶发提升”尚无足够证据归因到某段提示或 schema。不得再用增加同义提示词作为已证修复，也不得把 sibling 自动搬入 Exec，后者会改写 Agent 的事务表达。
- Repair design readiness: blocked on product decision for structural self-heal; existing fail-closed boundary remains correct

## Evidence E-032: Pair 007 保留了动作语义和节点归属，只错在响应层级

- Related hypotheses:
  - H-015
- Direction: supports
- Type: exact-production-trace-and-natural-control
- Source: `WAR-20260820-062550-R8-BASE308-OUTER-R10`, pair 007 / 009 / 011
- Matched signal:
  - Pair 007 Request 2 同时返回一个 `taskspace_exec(type=work)` 和一个 sibling 顶层 `exec_command`。
  - sibling 携带 `node_id=explore`，input 是读取测试文件的新命令；不存在 Map 内容、owner 或 Tool input 缺失。
  - Runtime 在两者执行前按响应级边界零副作用拒绝。
  - 下一请求的 reasoning 明确承认第二个命令放错顶层，并把源码读取与测试读取两个命令都放进一个 outer Exec 的
    `tools[]`，随后成功。
  - Pair 009、pair 011 在相同 Base/Tool schema 下均直接生成含两个 `exec_command` 的单一 `tools[]` 并成功。
- Interpretation: I03 的这次逃逸是模型的结构布局错误，不是上下文反馈丢失、Map 绑定失败、Runtime 提升或 schema 表达能力缺口。现有 fail-closed 拒绝语义正确。
- Time: 2026-08-20

## Evidence E-033: Pair 013 缺失 type 是单字段遗漏，现有反馈可立即纠正

- Related hypotheses:
  - H-012
- Direction: mixed
- Type: exact-production-trace
- Source: `WAR-20260820-062550-R8-BASE308-OUTER-R10`, pair 013
- Matched signal:
  - Agent 返回有效 JSON，包含一个非空 `tools[]`、原生 `exec_command`、`node_id=explore` 和完整 input，但遗漏顶层
    `type`。
  - Runtime 返回精确的 `required property type is missing`，没有执行 Map 或 Tool action。
  - 下一请求明确说明遗漏字段，并以 `type=work` 重试。
- Interpretation:
  - Base 3.0.7/3.0.8 的显式 type 合同降低过该问题，但没有消除偶发字段遗漏；H-012 仍是概率性缓解而非关闭证据。
  - 对该观测输入，离线枚举显示只有 `work` 能通过完整 catalog 解码，但自动补写仍会改变“序列由 Agent 显式选择”的既定产品边界，因此本轮不实施，只登记为待决候选。
- Time: 2026-08-20

## Hypothesis H-016: 多 outer 合同拒绝被错误升级为 session fatal

- Status: confirmed
- Claim: response scope 已能在零副作用边界识别同一响应包含多个 outer Exec，但 turn 协调器只把顶层 client Tool
  escape 归为 recoverable；多 outer 直接转成 `CodexErr::Fatal`，pending outer calls 因而没有失败 output，Agent 也没有
  下一请求纠正机会。
- Predictions:
  - 原始 response trace 存在两个完整 `taskspace_exec` call，且没有对应 output。
  - `taskspace.exec.response_finalized` 报告 `exec_call_count=2`、`accepted=false`，不存在 preflight 或 client dispatch。
  - CLI 直接发出 `turn.failed`，而不是携带合同拒绝进入下一 Provider request。
- Fix boundary:
  - 整批保持零副作用拒绝；不得执行、选择、合并或重排任一 sibling outer call。
  - 每个原始 outer `call_id` 获得同一个 response-level 合同错误，成为后续正式上下文。
  - 不修改 Map、client Tool schema、Tool result 或 Agent 声明的节点归属。

## Evidence E-034: Base 3.0.8 真实复发由 fatal 恢复链截断

- Related hypotheses:
  - H-013
  - H-016
- Direction: supports
- Type: production-failure-trace-and-static-code-path
- Source: `WAR-20260820-214337-R8-MAP-REQUEST-R3/release-dispatch-repair/live-r3c/pair-001`
- Matched signal:
  - Request 1 的唯一 `initialize_and_work` 建立 5 节点 Map，并在 `inspect` 成功执行目录发现。
  - Request 2 并列生成两个 `taskspace_exec(type=work)`；二者都绑定 `inspect`，分别读取实现和测试，参数完整。
  - response scope 日志记录 `exec_call_count=2`、`response_contract_rejected`；没有任何 client Tool output 或文件副作用。
  - `turn.rs` 在 `scope.finalize` 返回错误后调用 `taskspace_response_reconciliation_error`，直接构造
    `CodexErr::Fatal`；`drain_in_flight` 位于该返回点之后，没有执行。
  - CLI 输出 `TaskSpace response contains more than one Exec call` 后立即 `turn.failed`。Observer 对两个 call 报
    `exec_result_missing`，与恢复输出缺失一致。
- Interpretation: H-013 在 Base 3.0.8 上低频复发；H-016 坐实为独立 Runtime 恢复缺口。文字合同此前的 59 响应正向
  证据仍有效，但不能替代结构化可恢复拒绝。
- Time: 2026-08-20

## Evidence E-035: 多 outer 恢复链与 observer 已离线闭环

- Related hypotheses:
  - H-016
- Direction: supports repair
- Type: deterministic-runtime-and-observer-tests
- Source: commit `8ae4f2759`; `taskspace-exec/94-i03-multi-outer-recovery-result.md`
- Matched signal:
  - 多 outer 仍由 response scope 判定非法，两个 `claim_response` 均失败。
  - turn 将该 response-level 错误分类为 recoverable 后才 drain 原始 futures，不再提前返回 fatal。
  - 两个 handler 按各自原始 `call_id` 返回同一 `invalid top-level contract`，client Tool 调用为 0，Map 不存在。
  - observer 双 outer fixture 统计 2 calls、2 contract rejects、2 correlated outer IDs、1 request，且
    `exec_result_missing=0`、availability=`measured`。
  - TaskSpace Exec 85 项、Core check 和 observer fixture 通过。
- Interpretation: H-016 的 Runtime 恢复缺口已完成确定性修复；Agent 多 outer 的自然复发和收到反馈后的在线纠正仍属 H-013
  行为验证，不得据此关闭整个 I03。
- Time: 2026-08-20

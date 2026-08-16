# Problem P-001: initialize_map 偶发被二次序列化为 JSON string
- Status: open
- Created: 2026-08-15 23:15
- Updated: 2026-08-16 18:58
- Objective: 确认 `taskspace_exec.initialize_map` 类型错误的实际发生层、频率和可验证根因，避免把模型输出错误误归因给 Runtime。
- Symptoms:
  - 同一 Function Tool schema 下，Agent 有时把应为 object 的 `initialize_map` 写成包含 JSON 文本的 string。
- Expected behavior:
  - Agent 生成 schema 合法的 object；非法类型被零副作用拒绝并收到准确反馈。
- Actual behavior:
  - 最新 repeat=5 中 1/5 首发为 string，下一请求恢复；紧邻上一轮曾连续十次重复 string。
- Impact:
  - Map 初始化延迟或完全失败，请求、token 和时间被放大。
- Reproduction:
  - `single-file-fast-fix × map-request × deepseek-v4-flash`，观察首个原始 `taskspace_exec` Function Call。
- Environment:
  - Linux Docker benchmark；subject `5e5f01e3f`；Whale binary SHA-256 `38f000033c49...`。
- Known facts:
  - 错误已存在于 Provider 原始 Function Call；Runtime 没有执行类型改写。
  - repeat=5 首发 object 4 次、string 1 次；string 在下一请求恢复。
  - 五轮首请求 Tool schema、Base Instructions、system section、cache shape、tool_choice 和 payload bytes 相同。
  - 加上紧邻的同 capability 运行，共 6 个可比首请求：object 4 次、string 2 次；错误并不与初始 Map 节点数或字符数单调相关。
  - 两次错误首发中的 string 都能独立解析成语义完整的 object；同一调用的 `type`、`tools[]` 和原生 Tool `input` 仍为正确结构。
  - Chat Completions 适配器只逐段拼接 Provider 返回的 Function `arguments`，不解析或重写嵌套字段。
  - 当前 `taskspace_exec` 为 `strict: false`，参数 schema 约 25,995 bytes，顶层为 8 个合法序列的 `anyOf`；`initialize_map` 通过 `$ref` 指向 object 定义。协议同时提供了正确 object 示例。
- Ruled out:
  - Runtime 把正确 object 扭曲为 string。
  - 某轮切换 Tool schema、提示词版本或 tool_choice。
- Fix criteria:
  - 通过单变量证据确认可控诱因；修复后在简单和复杂 sample 上降低首发类型错误且不损害合法序列表达、反馈语义和缓存。
- Current conclusion: 这是非 strict Function Call 生成阶段发生的、只改变嵌套 Map 参数表示形式的二次序列化错误。复杂 schema 边界是当前最强候选诱因，但尚无单变量因果证据；现有反馈缺少 expected/actual 类型是已确认的持续放大缺口，不是首发原因。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: 错误在 Provider 原始 Function Call 中产生
- Status: confirmed
- Parent: P-001
- Claim: `initialize_map` 在进入 Runtime 解码和校验之前已经是 JSON string。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 若原始 rollout 已保存 escaped string，而同一调用的其他字段仍为对象，则 Runtime 不是类型扭曲来源。
- Falsifiable predictions:
  - If true: 原始 response item 的 `arguments` 解码后，`initialize_map|type == string`。
  - If false: 原始字段为 object，只在 Runtime 后续 trace 中变成 string。
- Diagnostic evidence plan:
  - Prediction or clause under test: 检查 Provider 原始 response item，不读取 Runtime 重建值。
  - Signal: `payload.arguments` 解码后的字段类型。
  - Capture method: 解析五轮 `rollout.jsonl` 的首个 `taskspace_exec`。
  - Event name or marker:
    - `response_item/function_call/taskspace_exec`
  - Correlation keys:
    - outer `call_id`
  - Differentiates from:
    - H-002
  - Supports if:
    - string 已在原始 Function Call 中出现。
  - Refutes if:
    - 原始值始终为 object。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留原始 rollout 和 outer call identity。
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: confirmed
- Repair design readiness: blocked until 可控诱因被确认
- Next step: 检验 schema 嵌套表达的单变量候选，不增加 Runtime 语义修正。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: Runtime 把 object 扭曲为 string
- Status: refuted
- Parent: P-001
- Claim: Provider 返回 object，但 Runtime 的上下文、解码或自愈路径把它改成 string。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 该假设符合“先怀疑语义传递”的全局倾向，必须用原始调用证据排除。
- Falsifiable predictions:
  - If true: 原始 response item 为 object，Runtime 拒绝时才变成 string。
  - If false: 原始 response item 已为 string。
- Diagnostic evidence plan:
  - Prediction or clause under test: 比较原始 Provider call 与 Runtime rejection。
  - Signal: 原始字段类型和错误路径是否发生副作用。
  - Capture method: rollout call/output 成对核对。
  - Event name or marker:
    - `response_item/function_call_output`
  - Correlation keys:
    - outer `call_id`
  - Differentiates from:
    - H-001
  - Supports if:
    - object 进入 Runtime 后被改写。
  - Refutes if:
    - 原始值已是 string 且 Runtime 零副作用拒绝。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: refuted
- Repair design readiness: blocked
- Next step: 不在 Runtime 增加基于错误归因的 object-to-string 修复。
- Blocker:
  - none
- Close reason:
  - 原始 Provider call 已排除 Runtime 类型扭曲。

## Hypothesis H-003: schema 体积或嵌套深度是可控主因
- Status: unverified
- Parent: P-001
- Claim: 当前 Tool schema 的体积或 `initialize_map` 嵌套表达显著提高了二次序列化概率。
- Layer: sub-cause
- Factor relation: unknown
- Depends on:
  - H-001
- Rationale:
  - Tool section 为 25,001 bytes，参数 schema 约 25,995 bytes，字段嵌套较深；但正确首发可以承载同样数量甚至更长的 Map，不能从共现直接推出因果。
- Falsifiable predictions:
  - If true: 只简化一个 schema 变量会稳定降低类型错误，且不损害其他合法序列。
  - If false: 单变量变化不改变错误分布，或引入等量其他结构错误。
- Diagnostic evidence plan:
  - Prediction or clause under test: 冻结模型、sample、协议和缓存形状，仅改变一个 schema 表达因素。
  - Signal: 首发字段类型、完整序列正确率、请求/token/cache 和外部正确性。
  - Capture method: 先做离线 schema 差异审计，再申请受控 A/B 真实预算。
  - Event name or marker:
    - `provider.chat_wire_shape_recorded`
  - Correlation keys:
    - tools_hash
    - cache_shape_hash
  - Differentiates from:
    - 随机生成波动
  - Supports if:
    - 多轮中类型错误显著下降且其他指标不退化。
  - Refutes if:
    - 错误不下降或转化为其他结构错误。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 沿用现有 Provider wire 和 Exec trace。
- Evidence gate: pending
- Related evidence:
  - E-002
  - E-003
  - E-004
  - E-009
  - E-010
- Conclusion: unverified
- Repair design readiness: blocked until Evidence gate is satisfied
- Next step: 暂不实施；先设计最小单变量候选。
- Blocker:
  - 需要新的产品/预算决策。
- Close reason:
  - not closed

## Hypothesis H-004: 类型反馈缺少 expected/actual 会放大一次首发错误
- Status: confirmed
- Parent: P-001
- Claim: 当前拒绝反馈准确指出路径，但没有说明期望类型和实际类型，无法让 Agent 稳定区分“字段内容错误”与“字段被二次序列化”；错误调用进入后续自然历史后，可能被连续复用并扩展成 wrapper 尝试。
- Layer: feedback-amplifier
- Factor relation: dependent
- Depends on:
  - H-001
- Rationale:
  - 该缺口不能解释第一轮为何出错，但可以解释为什么同一错误有时一次恢复、有时耗尽整个请求预算。
- Falsifiable predictions:
  - If true: Agent 在收到现有错误后会猜测与实际类型无关的字段、Map 内容或 wrapper，并可能继续产生 string。
  - If false: Agent 能从现有反馈稳定识别 actual=string、expected=object，并在下一请求可靠纠正。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对比两次首发 string 后的 reasoning、下一次原始 Function Call 和反馈原文。
  - Signal: 是否准确识别实际类型；是否复用 string；是否引入 `input`/`request` wrapper。
  - Capture method: 静态解析两个同 capability rollout，不启动新运行。
  - Event name or marker:
    - `response_item/function_call_output`
    - `response_item/reasoning`
  - Correlation keys:
    - outer `call_id`
  - Differentiates from:
    - H-003 解释首发概率；H-004 只解释错误后的恢复稳定性。
  - Supports if:
    - 反馈后出现类型猜测和重复错误，而不是稳定纠正。
  - Refutes if:
    - 两次均在下一请求准确纠正且 reasoning 明确认出 actual/expected。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - 沿用原始 rollout。
- Evidence gate: satisfied
- Related evidence:
  - E-005
  - E-007
  - E-008
- Conclusion: confirmed
- Repair design readiness: ready for narrow feedback design; implementation still requires user authorization
- Next step: 将机械 schema violation 忠实表达为 `expected object, actual string`，不得解析、改写或接受错误值；单独验证恢复率。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 五轮原始 Function Call 类型与恢复路径
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: experiment
- Source: `docs/v0.0.5/build-R8/taskspace-exec/58-initialize-map-type-repeat5-result.md`
- Prediction or plan link:
  - H-001 原始字段类型预测；H-002 反向预测。
- Matched signal:
  - Run 2 原始 `initialize_map` 为 string；Runtime 零副作用拒绝；下一请求为 object。
- Correlation keys:
  - `call_00_NUzJiYKgUOeifJAENnv69868`
- Raw content:
  ```text
  $.initialize_map: value has the wrong JSON type
  first-call types: object, string, object, object, object
  ```
- Interpretation: 错误发生于模型/Provider Function Call 生成，不是 Runtime 扭曲；本轮频率为 1/5。
- Time: 2026-08-15 23:15

## Evidence E-002: 五轮首请求合同身份一致
- Related hypotheses:
  - H-003
- Direction: neutral
- Type: diagnostic-log
- Source: 五轮 `provider-wire-trace.jsonl` 的 `provider.chat_wire_shape_recorded`
- Prediction or plan link:
  - H-003 当前静态合同是否足以解释轮间差异。
- Matched signal:
  - tools/base/system/cache-shape/tool_choice/payload-bytes 全部相同。
- Correlation keys:
  - `tools_hash=26188b5a...`
  - `cache_shape_hash=5c8d3ef6...`
- Raw content:
  ```text
  tools_bytes=25001; tools_count=2; tool_choice=auto; provider_payload_bytes=50770
  ```
- Interpretation: 排除轮间 schema 切换，但不能证明或排除 schema 本身提高随机错误率。
- Time: 2026-08-15 23:15

## Evidence E-003: 六个同 capability 首请求与 Map 规模对比
- Related hypotheses:
  - H-003
- Direction: refutes-partial
- Type: trace-comparison
- Source: 两组 `single-file-fast-fix` rollout；`taskspace_capability_identity=49fd49b9...`
- Prediction or plan link:
  - 检查 Map 规模是否与首发 string 单调相关。
- Matched signal:
  - 6 个可比首请求为 4 object / 2 string。正确 object 可包含 3 个 Work 节点、约 391/392 字符，另有 519 字符的正确对象；错误 string 为 3 个 Work 节点、约 428 字符。
- Correlation keys:
  - `taskspace_capability_identity=49fd49b9a28b...`
  - `tools_hash=26188b5a4b39...`
- Raw content:
  ```text
  correct: 1 or 3 work nodes; 255, 391, 392, 519 chars
  wrong:   3 work nodes; 428 chars (two independent first requests)
  ```
- Interpretation: 排除“节点多或 Map 文本长就必然字符串化”；schema 总体认知负荷仍是候选背景因素。
- Time: 2026-08-16 00:35

## Evidence E-004: 错误只改变 Map 参数的表示层
- Related hypotheses:
  - H-003
- Direction: supports-partial
- Type: static-and-trace
- Source: 当前 schema、两次错误首发及 Chat Completions SSE 适配器。
- Prediction or plan link:
  - 判断错误是 Map 语义失败、全调用序列化失败，还是一个嵌套边界的表示错误。
- Matched signal:
  - string 内部均可解析为包含 Root、Work、Finish 的 object；外层 `type`、`tools[]`、`node_id`、原生 Tool `input` 正常。此前连续失败轨迹还把 `$ref` 对象 `read_map:{}` 写成 `read_map:"{}"`。适配器只执行 `state.arguments.push_str(&arguments)`。
- Correlation keys:
  - `call_00_cjuV2QaQEOnSNL1lLudX5743`
  - `call_00_NUzJiYKgUOeifJAENnv69868`
- Raw content:
  ```text
  initialize_map: "{\"root\":...,\"work_nodes\":...,\"finish\":...}"
  tools: [{"tool":"exec_command","node_id":"inspect","input":{"cmd":"..."}}]
  ```
- Interpretation: 最符合“模型在 Map operation object 边界多做一次序列化”，不支持 Runtime 或整个 Function Call transport 普遍字符串化。
- Time: 2026-08-16 00:35

## Evidence E-005: 同一反馈后的恢复与连续放大轨迹
- Related hypotheses:
  - H-004
- Direction: supports
- Type: trace-comparison
- Source: 当前 Run 2 与紧邻上一轮失败 rollout。
- Prediction or plan link:
  - 检查现有反馈能否稳定传递实际类型并支持纠正。
- Matched signal:
  - 两次收到完全相同的 `$.initialize_map: value has the wrong JSON type`。当前 Run 2 下一请求复制规范示例后恢复；上一轮前 5 次 `initialize_map` 均为 string，之后尝试 `read_map:"{}"`、`input` wrapper 和 `request` wrapper，10 次均失败。reasoning 多次声称“我传的是 object”，并猜测 `parents`、`tools` 或 harness 序列化。
- Correlation keys:
  - `call_00_NUzJiYKgUOeifJAENnv69868`
  - `call_00_cjuV2QaQEOnSNL1lLudX5743`
- Raw content:
  ```text
  current feedback: $.initialize_map: value has the wrong JSON type
  prior reasoning: That's strange since I'm passing an object.
  prior attempts: initialize_map string x5; read_map string; input wrapper x2; initialize_map string; request wrapper
  ```
- Interpretation: 反馈语义没有扭曲，但缺失实际/期望类型，导致恢复依赖随机猜中规范示例；错误历史会显著放大请求成本。
- Time: 2026-08-16 00:35

## Evidence E-006: Provider 非 strict 合同不保证参数符合 schema
- Related hypotheses:
  - H-003
- Direction: supports-background
- Type: external-primary-source
- Source: `https://api-docs.deepseek.com/guides/tool_calls` 与 `https://api-docs.deepseek.com/api/create-chat-completion`
- Prediction or plan link:
  - 确认当前 Provider 是否在生成阶段强制 schema 合法。
- Matched signal:
  - 当前声明为 `strict:false`。DeepSeek 官方文档说明普通 Function Call 的 `arguments` 由模型生成，可能不是合法 JSON 或包含 schema 未定义参数；只有 Beta strict mode 声明保证遵循 schema。
- Correlation keys:
  - `strict=false`
- Raw content:
  ```text
  Current taskspace_exec declaration: strict=false
  ```
- Interpretation: 这是首发错误能够到达 Runtime 的必要背景条件，不足以单独解释为何具体在 `initialize_map` 边界发生。
- Time: 2026-08-16 00:35

## Evidence E-007: expected/actual 类型反馈通过边界测试
- Related hypotheses:
  - H-004
- Direction: supports
- Type: focused-test
- Source: `schema_validation::tests::type_violation_reports_expected_and_actual_types` 与 `taskspace_exec_handler_tests::initialize_map_type_feedback_reports_expected_and_actual_types`
- Prediction or plan link:
  - H-004 的窄修复必须把实际类型和期望类型忠实传给 Agent，同时保持零副作用拒绝。
- Matched signal:
  - object schema 收到 string 时，校验器与完整 handler 均返回 `expected JSON type object, got string`；Map 未初始化，原生 Tool 未执行。
- Correlation keys:
  - `$.initialize_map`
- Raw content:
  ```text
  $.initialize_map: expected JSON type object, got string
  No Map or Tool actions were executed
  ```
- Interpretation: 候选 1 的实现没有引入解析、自愈或语义替代；它只修复已确认的反馈缺失。真实 Agent 恢复率仍需单独运行验证。
- Time: 2026-08-16 01:20

## Evidence E-008: 候选 1 五轮真实运行未触发类型反馈
- Related hypotheses:
  - H-004
- Direction: neutral
- Type: experiment
- Source: `docs/v0.0.5/build-R8/taskspace-exec/59-initialize-map-candidate1-feedback-result.md`
- Prediction or plan link:
  - 检验 expected/actual 反馈是否提高 string 首发后的恢复稳定性，同时不引入其他回归。
- Matched signal:
  - 五个有效 TaskSpace 观测的首次 `initialize_map` 均为 object，因而类型反馈 0 次触发。4/5 完整通过；另 1 轮在成功初始化后逃逸为未暴露的顶层 `exec_command`，与类型反馈无关。Request 2+ 加权缓存命中率为 92.18%。
- Correlation keys:
  - `subject=326e1430c`
  - `taskspace_capability_identity=49fd49b9a28b...`
- Raw content:
  ```text
  first initialize_map types: object, object, object, object, object
  expected/actual feedback triggers: 0
  ```
- Interpretation: 聚焦测试和真实运行均未显示反馈修复造成回归，但本轮缺少 string 触发条件，不能评价恢复率收益，也不能把 0/5 解释为首发生成改进。
- Time: 2026-08-16 18:40

## Evidence E-009: 候选 2 内联 schema 五轮均为 object
- Related hypotheses:
  - H-003
- Direction: neutral
- Type: experiment
- Source: `docs/v0.0.5/build-R8/taskspace-exec/60-initialize-map-candidate2-inline-schema-result.md`
- Prediction or plan link:
  - 冻结其余协议与 Runtime，只把 `initialize_map` 从 `$ref` 改为同合同的就地 object schema。
- Matched signal:
  - 五轮首次 `initialize_map` 均为 object，5/5 Agent complete，5/5 外部验证通过。Tool section 减少 63 bytes，Request 2+ 加权缓存命中率 92.90%。
- Correlation keys:
  - `subject=847da1c37`
  - `taskspace_capability_identity=a95be2ff3edf...`
- Raw content:
  ```text
  first initialize_map types: object, object, object, object, object
  tool section: 24,938 bytes
  ```
- Interpretation: 结果与候选机制一致且未见回归，但未改 schema 的候选 1 同样得到 object 5/5；当前样本不能区分内联收益与随机波动，H-003 仍未通过因果证据门禁。

## Evidence E-010: 移除首次初始化完整示例引入序列完整性回归
- Related hypotheses:
  - H-003
- Direction: refutes-candidate
- Type: experiment
- Source: `docs/v0.0.5/build-R8/taskspace-exec/61-initialize-map-candidate3-no-first-turn-example-result.md`
- Prediction or plan link:
  - 检验完整 JSON 示例是否诱发 `initialize_map` 二次序列化，同时不损害合法序列表达。
- Matched signal:
  - 五轮 `initialize_map` 均为 object，但仅 2/5 首次请求包含完整合法的 `initialize_and_work`；2 次缺少 work，1 次同时缺少 `type` 和 work。
- Correlation keys:
  - `taskspace_capability_identity=88043c22...`
  - `tools_hash=8862dfcb...`
- Raw content:
  ```text
  first initialize_map types: object, object, object, object, object
  first legal initialize_and_work: false, true, false, true, false
  first request rejected: work missing, none, work missing, none, type and work missing
  ```
- Interpretation: 候选没有提供类型错误的独立因果证据，却明确降低了首次合法序列完整率。完整示例当前承担操作示范，不能作为单变量优化删除；候选 3 判定失败。
- Time: 2026-08-16 18:50

# Phase B5 CP-03：公共 Tool 结果转换覆盖

- Date: 2026-08-10
- Scope: Function、Freeform、MCP、Tool Search、失败与大输出结果
- Status: verified offline
- Production behavior: 未改变

## 1. 结论

Codex 的 `ToolOutput` 是正确的公共结果事实边界，但 `code_mode_result()` 不是可以原样套用到 TaskSpace 的完整合同。
它混合了两类职责：大多数实现用于保留嵌套 Tool 结果，少数实现则带有 Code Mode 专属裁剪策略。

TaskSpace 当前更早调用 `AnyToolResult::into_response()`，把结果转换成 `ResponseInputItem` 后才放入 outer feedback。
这会让 Provider 传输 envelope 进入业务结果，同时也失去在 ToolOutput 边界选择最忠实结果表示的机会。

## 2. 覆盖矩阵

| 类型 | 当前公共转换事实 | TaskSpace 可否直接复用 | 证据 |
|---|---|---|---|
| 普通 Function | 默认从原 `to_response_item()` 提取文本/内容 | qualified：结果值可用，成功状态由 outer outcome 单独保留 | `function_payloads_remain_function_outputs` |
| 普通 Freeform | 与 Function 共用内容转换，并保持 Custom output 类型 | qualified | `custom_tool_calls_should_roundtrip_as_custom_outputs` |
| MCP / Namespace MCP | `McpToolOutput::code_mode_result()` 返回完整原始 `CallToolResult`，保留 structured content、error 和 meta | yes | `mcp_tool_output_code_mode_result_stays_raw_call_tool_result` |
| Tool Search 成功 | 返回完整 `LoadableToolSpec[]` | yes | `tool_search_code_mode_result_preserves_loadable_specs` |
| Apply Patch | Code Mode 专门返回 `{}`，Standard output 仍保留真实 Patch 文本 | no | `apply_patch_code_mode_policy_drops_feedback_preserved_by_standard_output` |
| Unified Exec 大输出 | Standard response 使用既有 output reference；Code Mode 结果走 `truncated_output()`，不携带 artifact reference | no | `exec_command_tool_output_referenceizes_large_response` 与 `ExecCommandToolOutput::code_mode_result()` 静态调用链 |
| 非致命 Tool 错误 | Function/Custom 的合成 response 含错误文本；Tool Search 被合成为空 tools，错误文本未进入 `ToolCallResponse` | no | `ToolCallRuntime::failure_response()` 静态分支 |
| Fatal Tool 错误 | 保留为 `CodexErr::Fatal` | yes，但 outer result 只能记录明确 error，不得伪造 Tool value | `handle_tool_call_with_status()` |

## 3. 对 CP-09 的约束

CP-09 在真实消费点完成以下最小修复，不提前新增无人使用的 API：

1. client dispatch 在构造 outer result 前保留 `AnyToolResult`，不先压成 `ResponseInputItem`；
2. 在公共 `ToolOutput` 边界增加中性的 nested result 读取能力，并由 TaskSpace 与 Code Mode 共同复用已有实现；
3. Apply Patch 的中性 nested result 保留真实 Tool 文本，Code Mode 仍可保持其 `{}` 专属策略；
4. Unified Exec 的中性 nested result 复用 Standard 已生成的 output reference，不回放、复制或重新总结大输出；
5. `ToolCallResponse` 机械保留非致命失败的模型可见错误文本，TaskSpace outer error 直接透传；Tool Search 不得再以空数组
   代替错误原因；
6. TaskSpace 只包装 `outcome + result/error`，不解释结果、不根据结果推进 Node，也不建立私有语义 converter。

## 4. 非目标

- 不改变 Standard Provider wire；
- 不移除 Code Mode 的专属输出策略；
- 不把 `ResponseInputItem` envelope 定义成 TaskSpace 业务结果；
- 不复制完整 Tool 输出到 Map；Map 仍只保存 Action 的机械 outcome；
- 不在 CP-03 增加无生产消费者的 trait method 或适配层。

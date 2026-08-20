# Phase B5 CP-09：公共中性 nested result

- Date: 2026-08-10
- Scope: `ToolOutput`、`AnyToolResult`、`ToolCallRuntime`、TaskSpace client dispatch 与 outer feedback
- Status: verified offline

## 1. 变更

在公共 `ToolOutput` 边界增加 `NestedToolResult`。它是按原生结果种类标记的机械结果值，不包含 TaskSpace Map
语义，也不携带 Provider 配对所需的内层 `call_id`：

- Function：保留 `FunctionCallOutputPayload`；
- Custom / Freeform：保留原 output 与可选原生名称；
- MCP：保留完整 `CallToolResult`，包括 structured content、`isError` 与 meta；
- Tool Search：保留 status、execution 和完整 Tool specs；
- Message：仅作为公共结果边界的完整机械覆盖。

默认实现复用每个 Tool 的 Standard `to_response_item()`，再只移除传输 envelope。`McpToolOutput` 是唯一 override，
因为其 Standard 上下文形态包含 wall-time 与裁剪，而中性事实必须保留原始 `CallToolResult`。

## 2. TaskSpace 反馈

TaskSpace client dispatch 在原生 Router、handler 和 hook 完成后调用公共 nested result 出口。Outer feedback 从：

```json
{"response":{"type":"function_call_output","call_id":"...","output":"native-result"}}
```

收敛为：

```json
{"result":{"type":"function","output":"native-result"}}
```

非致命 Tool 错误进入同项的 `error`，不再伪造普通成功结果。尤其 Tool Search 失败不再被表达成
`tools: []`。Fatal 与非致命错误都只作为真实失败文本进入 outer result；Runtime 不解释、纠正或重试。

Action outcome 仍是机械执行事实：cancelled 优先，其次执行错误，再读取结果自身的 success / MCP `isError`。
结果内容不会自动改变 Node 状态，也不会写入 Map。

## 3. 保持不变

1. Standard 仍调用原 `handle_tool_call_with_status()` 并返回同一 `ResponseInputItem`；
2. Code Mode 仍调用各 Tool 既有 `code_mode_result()`，Apply Patch 的 `{}` 等专属策略未改变；
3. Router、handler、hooks、并行安全和 settlement 顺序不变；
4. 大输出仍由 Standard 已有 output-reference 机制生成，TaskSpace 不复制原始大输出；
5. 没有 TaskSpace 私有 converter、结果摘要器或语义重组层。

## 4. 验证

- context：23 tests PASS；
- TaskSpace Exec：62 tests PASS；
- Patch nested result 保留真实 patch 反馈，Code Mode 仍为 `{}`；
- Unified Exec nested result 保留 `OutputReferenceV1` 与 artifact ref；
- MCP nested result 与原 `CallToolResult` 完全相等；
- Tool Search nested result 保留完整 deferred Tool spec；
- 非致命 Function / Tool Search 错误进入 `error`，不生成伪结果；
- outer feedback 不再暴露内层传输 `call_id`。

## 5. 后续

CP-05 可据此从自然上下文中的 TaskSpace Tool Search result 机械恢复已选择的 deferred capability。该恢复必须由
历史事实重算，不增加隐藏 session ledger，也不在首轮展开全部 deferred schema。CP-08 将从本固定结果 envelope 生成
同源 outer output schema。

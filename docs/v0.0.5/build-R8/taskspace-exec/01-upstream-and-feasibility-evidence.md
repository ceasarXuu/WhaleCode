# TaskSpace Exec 上游与可行性证据

- Created: 2026-08-05
- Status: Verified discovery evidence
- Latest upstream inspected: `openai/codex` main `5c44f110649f8811546745bb1635ba0b44a1639e`
- Local evidence cutoff: `17526e59f`

## 1. 最新 Codex 主线事实

截至上述主线提交，Codex `exec/code-mode` 的关键结构是：

1. `core/src/tools/spec_plan.rs` 统一组装 registry、hosted specs、model-visible specs 和 code-mode exposure；
   `core/src/tools/code_mode/execute_spec.rs` 负责 `exec` ToolSpec。
2. `tools/src/code_mode.rs` 从现有 Function、Freeform 和 Namespace ToolSpec 机械生成内部 ToolDefinition；WebSearch 和
   ToolSearch 等 provider 特殊 Tool 不被伪装为普通嵌套 client Tool。
3. `code-mode-protocol/src/description.rs` 把内部工具名称、描述、输入 JSON Schema 和输出 Schema 转换成模型可读的
   TypeScript 声明，避免手写第二份业务合同。
4. `core/src/tools/code_mode/mod.rs` 将内部调用重新构造成原生 ToolPayload，并通过同一 Tool runtime dispatch；`exec`
   显式禁止调用自身。
5. `core/src/tools/code_mode/execute_handler.rs` 负责启动隔离执行单元、收集内部调用结果和返回外层 Tool output；最新主线
   又把原 `spec.rs` 拆为 `spec_plan.rs`、`hosted_spec.rs` 等职责文件，并以 `ToolExposure` 统一 direct/deferred/code-mode
   暴露，但没有建立第二套业务 Tool handler。

这些事实支持 Whale 复用“单入口、内部派生、原 Router、禁止递归”四个机制。它们不直接证明 TaskSpace 的 Map
合法序列、节点归属和 hosted 双写合同。

## 2. Whale 已有 Function Exec 证据

DeepSeek 当前不稳定接受 Codex Freeform exec Tool。Whale 在本地 vendor 上增加了实验性 Function 形态：

```json
{
  "name": "exec",
  "parameters": {
    "type": "object",
    "properties": {"source": {"type": "string"}},
    "required": ["source"]
  }
}
```

相关证据：

| 证据 | 结论 | 限制 |
|---|---|---|
| `WAR-20260805-054853-R8-DEEPSEEK-FUNCTION-EXEC-001` | Function exec 消除 Freeform schema 阻塞并进入嵌套 Tool dispatch | Docker runner 参数错误，不能评价完整任务 |
| `WAR-20260805-055746-R8-DEEPSEEK-FUNCTION-EXEC-CORRECTED-001` | DeepSeek V4 Flash 完成真实编码任务闭环 | 单样本只证明可行性 |
| `WAR-20260805-061947-R8-FUNCTION-EXEC-CONTRACT-FIX-001` | 修正 Function `{source}` 说明后，两个已知误用诱因消失 | 暴露了内部结果可见性缺口 |
| `WAR-20260805-063652-R8-NESTED-RESULT-VISIBILITY-002` | 内部结果可见性修复后，8 个真实请求内完成业务、验证和最终答复 | 首次 `{cmd}` 错用仍偶发；请求聚合另有 I07 缺陷 |

对应实现提交：

- `49213445b`：内部 Tool 结果显式进入 exec 输出；
- `428f851ac`：真实 nested result visibility 证据；
- `17526e59f`：记录 Harness 请求聚合缺陷，避免用 15 次误报否定 8 次真实请求。

结论是：DeepSeek 可以使用 Function Call 形态的超级 Tool 并驱动内部 client Tool。尚不能由此宣称 Agent 已稳定遵循
TaskSpace 序列或 hosted 双写协议。

## 3. 可直接复用的工程面

| 能力 | 复用位置 | TaskSpace 增量 |
|---|---|---|
| Tool catalog 派生 | `codex-tools` 的 ToolSpec -> ToolDefinition | 过滤顶层能力并加入外层 binding/sequence 说明 |
| 内部调用 payload | code-mode 的 Function/Freeform/Namespace 转换 | 附加 Runtime invocation metadata，不改 inner input |
| 原生执行 | ToolRouter / registry / handler / permissions / hooks | dispatch 前运行 TaskSpace 机械 preflight |
| 结果转换 | code-mode response adapter 与原 Tool output | 增加调用身份和绑定结算，不改结果正文 |
| 递归保护 | `is_exec_tool_name` / nested Tool filter | `taskspace_exec` 和原 `exec` 均不得成为自身内部成员 |
| deferred Tool | `ALL_TOOLS` / ToolSearch 现有能力 | 需证明加载后 catalog identity 与缓存指纹一致 |

## 4. TaskSpace 增量边界

- 生产入口使用结构化 Function Call。静态 schema 定义可变长 `calls[]`、`hosted_bindings[]` 和各 Tool 参数形状；Agent
  构造每次实际 Tool、数量、参数、顺序和节点归属。
- Codex 不要求每个嵌套 Tool 绑定 Map 节点；Whale 在外层 invocation metadata 增加 Agent 声明的 `node_id`，不修改
  原生 Tool args。
- Codex 不负责把 provider-hosted 输出登记到 Map；Whale 从真实 provider response 建立可复算 reconciliation，节点归属
  仍由 Agent 在 `hosted_bindings[]` 中声明。
- 最新上游已经把 Tool 规划从本地仍在使用的 `spec.rs` 拆到 `spec_plan.rs`、`hosted_spec.rs` 等模块，并新增统一
  `ToolExposure`；Phase B 的 TX-06A～C 同步或中性抽取该 seam 后接共享 catalog，不覆盖 Whale 改造，也不在旧 `spec.rs`
  新增长期平行 catalog。

## 5. 外部依据

1. [OpenAI Codex code-mode 主线目录](https://github.com/openai/codex/tree/5c44f110649f8811546745bb1635ba0b44a1639e/codex-rs/code-mode)和
   [execute ToolSpec](https://github.com/openai/codex/blob/5c44f110649f8811546745bb1635ba0b44a1639e/codex-rs/core/src/tools/code_mode/execute_spec.rs)：单一
   `exec` Tool 与主线执行入口。
2. [OpenAI Codex ToolSpec 到内部定义转换](https://github.com/openai/codex/blob/5c44f110649f8811546745bb1635ba0b44a1639e/codex-rs/tools/src/code_mode.rs)和
   [统一 Tool 规划](https://github.com/openai/codex/blob/5c44f110649f8811546745bb1635ba0b44a1639e/codex-rs/core/src/tools/spec_plan.rs)：
   Function、Freeform、Namespace 的机械派生及特殊 Tool 排除边界。
3. [OpenAI Function Calling](https://developers.openai.com/api/docs/guides/function-calling)：Function Tool 使用 JSON
   Schema 声明参数，Tool call 与 Tool output 通过调用身份配对。
4. [OpenAI Web Search Tool](https://developers.openai.com/api/docs/guides/tools-web-search)：Web Search 属于 provider
   托管能力，其响应事实与 client function dispatch 的执行归属不同。
5. [Model Context Protocol Tools](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)：MCP Tool input、
   result 和 error 是明确协议对象，嵌套适配不得丢失结构化结果。

外部资料只证明通用 Tool 形态和执行归属。TaskSpace 的合法序列、Map 状态和节点绑定仍以本产品合同和本地证据为准。

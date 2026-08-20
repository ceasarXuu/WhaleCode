# Phase A TS-01～TS-03 提前验证结果

> **已封存证据（2026-08-05）**：旧容器计划停止；Tool 类型矩阵和 provider wire 事实仅作为 TaskSpace Exec 的调查输入。

- Date: 2026-08-04
- Status: TS-01～TS-03 completed / Hosted 节点归属合同片段已在 TS-06 验证
- Scope: client Tool 能力边界、provider-native hosted 输出身份、TaskSpace 请求 wire
- Runtime change: 无生产行为变化；只新增两项 `codex-api` 协议回归测试
- Whale Agent/API run: 未执行，token 与费用均为 0

## 1. 结论

容器方向仍然可行，但当前不能冻结 `provider_result_ref.output_ref` schema。

已经证实：

1. Function/Freeform client Tool 可从外层调用还原并复用原 Router；MCP、ToolSearch、LocalShell 需要各自已有执行身份，
   不能只复制 Function schema。
2. Responses SSE 可以在同一响应中无损解析 Web Search、Image Generation 和普通 function call；Web/Image provider ID
   在解析、TaskSpace event-store 持久化和下一请求 replay 中均能保留。
3. Responses 请求可以表达“一个 function 容器 + provider-native hosted descriptors”；当前 DeepSeek 配置自然支持 Web
   Search descriptor，但不自然支持 Image Generation descriptor。

尚未证实：

1. 模型是否能在同一次生成中看到并复制 provider 分配的 `ws_...` / `ig_...` item ID。
2. 若模型不能复制 ID，应使用 provider output index、按类型计数的 ordinal，还是下一请求由 Runtime 暴露机械 handle。
3. `tool_choice` 没有一种现有取值可以同时“强制每个 TaskSpace 响应包含容器”并“允许模型先使用任意 hosted Tool”。

这些是 Agent 合同和 provider 能力边界，不应由 Runtime 通过内容猜配、静默绑定或隐藏 hosted Tool 解决。

## 2. TS-01：Client Tool 能力矩阵

| Tool 形态 | 结论 | 容器接入要求 |
|---|---|---|
| Function | supported | 从原 ToolSpec 派生 input；复用 direct Router alias 归一化 |
| Freeform / apply_patch | supported | 保持原始 string payload；继续执行单 Patch 硬规则 |
| eager dynamic Function | supported | 保留 plain/namespaced `ToolName` 和 DynamicToolHandler |
| Namespace dynamic member | supported | 容器项必须保存结构化 namespace/name，不能拼接后反推 |
| MCP Namespace member | deferred | 复用 Session 中 provider/canonical/raw MCP 三层身份和 `ToolPayload::Mcp` |
| ToolSearch | deferred | 使用 `ToolPayload::ToolSearch` 和 loadable-tools 特殊结果，不伪装普通 Function |
| deferred MCP/dynamic | deferred | 工具加载后再从 canonical registry 更新容器 schema |
| LocalShell | deferred | 需要既有 typed action/input seam；`ToolSpec::LocalShell {}` 本身没有参数 schema |
| WebSearch/ImageGeneration | hosted | 不进入 client nested builder；由 provider 原生执行并进入结果 reconciler |

关键源码事实：

- `ToolSpec` 统一声明 Function、Namespace、ToolSearch、LocalShell、WebSearch、ImageGeneration 和 Freeform；
- 当前 `build_native_nested_tool_call()` 只支持 Function 与 Freeform；
- MCP handler 需要 server/raw tool/raw arguments，不能从 Namespace Function schema 单独恢复；
- direct Router 存在 `exec_command` 等 alias 归一化，当前 nested builder 尚未复用。

TS-01 的“盘点”目标已经完成。正式实现不得把 `supported` 误解为“所有类型共用一个 JSON 转换函数”。

## 3. TS-02：Hosted 输出身份

### 3.1 已通过的本地证据

| 验证 | 结果 | 说明 |
|---|---:|---|
| 同响应 Web/Image/function call SSE 解析 | PASS | 5 个事件按原始类型解析，ID 与 function arguments 不变 |
| Hosted item replay identity | PASS | Web ID 经 `attach_item_ids()` 回填，Image ID 原生序列化 |
| TaskSpace event-store round trip | PASS | 既有 Web/Image 全字段持久化测试通过 |
| `cargo test -p codex-api` | 134 passed | 120 unit + 14 integration，无失败 |
| Core event-store 定向测试 | 1 passed | 1967 filtered，无失败 |

OpenAI Responses 公开协议把 response output 定义为有序 item 数组，并为 Web Search、Image Generation 和 Function Call
提供唯一 item identity。该事实支持 Runtime 核对和持久化，但官方文档没有保证模型能够把平台生成的 item ID复制进随后
的 function arguments。

参考：

- [OpenAI Responses streaming events](https://platform.openai.com/docs/api-reference/responses-streaming/response/refusal/delta?lang=curl)
- [OpenAI Web Search](https://developers.openai.com/api/docs/guides/tools-web-search)
- [OpenAI Image Generation](https://developers.openai.com/api/docs/guides/image-generation)

### 3.2 身份结论

Provider 返回的 `id` / SSE `item_id` 已经能够唯一识别实际 hosted output item。Runtime 应直接保存并使用该 ID 关联
流式阶段、最终结果和后续 replay，不再为同一次调用创造 ordinal、内容指纹或 TaskSpace 自有调用 ID。

候选文档曾示例：

```json
{"output_ref":{"response_item_id":"ws_123"}}
```

这个结构错误地要求 Agent 回显 provider 在协议层生成的 ID。Agent 能使用 Hosted Tool 结果，并不等价于官方合同保证
模型能读取并复制 response item 的传输层 ID。该字段不得进入 Agent 必填合同。

因此 TS-02 的“Provider 是否提供稳定调用身份”已经完成。剩余问题不是身份，而是 TS-06 必须回答的节点归属：Agent 如何
声明 `node_id`，Runtime 又如何把这项声明与拥有真实 `id` 的 provider 事实机械关联。不能按参数、结果正文或工具名称猜配。

## 4. TS-03：请求能力矩阵

| 主题 | 发现 |
|---|---|
| OpenAI / 内置 DeepSeek | 都使用 Responses wire，tools/tool_choice/parallel 字段可原样构造 |
| 当前 TaskSpace 投影 | 仍暴露普通 Tools + taskspace_control，只隐藏 update_plan；尚无生产容器 ToolSpec |
| 当前响应处理 | 请求保留 hosted descriptors，但 TaskSpace 收到 Web/Image 后又进入 RejectedNative，存在明确矛盾 |
| DeepSeek Web Search | 当前配置可自然生成原生 web_search descriptor |
| DeepSeek Image Generation | 模型元数据只有 Text input，当前不会自然生成 image_generation descriptor |
| Chat Completions profile | 会把 Web Search 改成 client function 并丢弃 Image Generation，不等价于本专题 Responses 路径 |
| `tool_choice=auto` | 允许 hosted 与容器，但不保证容器一定出现 |
| `tool_choice=required` | 只保证某个 Tool，不能保证选中容器 |
| named container | 可以强制容器，但可能阻止同响应 hosted Tool |

TS-03 的请求能力盘点已经完成。纯 serde fixture 可以证明本地 wire 可表达，但在正式 `taskspace_tools` ToolSpec 出现前，
不能把合成 fixture 当成生产投影通过。

## 5. 剩余合同缺口

### 5.1 Hosted Tool 与节点归属

确定事实：

- Provider `id` 是 Tool 调用输出身份的唯一事实源；
- `node_id` 必须来自 Agent 声明；
- Runtime 只能机械关联两者，不能推断节点、复制 Tool 语义或按内容猜配；
- Tool 执行状态与节点生命周期继续正交。

未冻结的是两者之间的关联合同。TS-06 必须用最小 schema 和正反 fixture 证明它只有一份声明，且对同类型多次 Hosted Tool、
并行完成、无绑定结果和重复绑定都无歧义。

### 5.2 容器必达性

OpenAI Responses 的 `tool_choice=required` 只保证调用一个或多个允许的 Tool，不保证特定的容器 Tool 必须与 Hosted Tool
同时出现；named function choice 可以强制容器，但不表达“容器必选且 Hosted Tool 仍可选”。仓库当前 `ToolChoice` 也只有
`Auto`、`None`、`Required` 和单一 named Function 四种形状。

本地 wire 只能证明“容器和 Hosted descriptor 可以同时暴露”。后续真实 Provider 探针已进一步取得 2/2 同响应共存，
详见 [`06-hosted-container-provider-probe-result.md`](06-hosted-container-provider-probe-result.md)。这证明当前模型具备该能力，
但 `tool_choice=auto` 仍不构成容器必达的协议硬保证。

## 6. 后续验证状态

1. 用户已接受“同一 Response 的 Hosted output 全部归属一个 Agent 声明节点”的最简合同。
2. TS-06 已用本地 fixture 验证 `hosted_node_id`、混合状态、无容器、重复、重放与冲突；详见
   [`07-ts06-hosted-response-scope-mvt-result.md`](07-ts06-hosted-response-scope-mvt-result.md)。
3. Agent 不得回显 Provider ID 或另造调用 ID；Runtime 从原始响应逐项登记，原始 Provider 事实始终保留。
4. `tool_choice` 保持 Provider 原生 `auto`；缺失容器时事实为 unbound，不伪称 Runtime 能强制容器必达。
5. 后续复杂样本继续观测容器缺失率；若稳定漏掉容器，再返回产品设计停点。

当前证据不推翻容器，但也没有证明完整产品路径。它关闭了“Provider 调用身份”问题，并把后续验证准确收敛到“节点归属”与
“容器必达性”。

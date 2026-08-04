# Phase A TS-01～TS-03 提前验证结果

- Date: 2026-08-04
- Status: TS-01 completed / TS-02 transport verified but binding contract blocked / TS-03 completed
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

### 3.2 当前停点

候选文档曾示例：

```json
{"output_ref":{"response_item_id":"ws_123"}}
```

这只能证明 Runtime 可以解析该字符串，不能证明 Agent 能生成真实 ID。直接冻结会形成自证式合同，因此 TS-02 当前状态
是 `blocked-on-agent-visible-reference`。

Responses SSE 还提供 `output_index`，但当前 `ResponseEvent::OutputItemDone(ResponseItem)` 会丢弃 event envelope 中的
index。使用 ordinal 需要先保留该机械事实，并验证并行 hosted output 的稳定排序；不能依赖 done 事件到达顺序猜配。

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

## 5. 需要用户决策的引用方案

| 方案 | 机制 | 收益 | 代价/风险 |
|---|---|---|---|
| A. Provider item ID | Agent 在容器中复制 `ws_.../ig_...` | Runtime 核对最直接 | 没有证据证明模型能看到平台 ID；需要真实 provider probe |
| B. Type + ordinal | Agent 声明 `web_search#1` 等响应内顺序 | 不依赖平台 ID，Runtime 最终仍保存权威 ID | 必须保留 `output_index`；并行同类 Tool 和 Agent 计数需要验证 |
| C. 下一请求 handle | Runtime 在反馈中暴露机械 handle，Agent 下一轮绑定 | 身份最可靠，不要求模型知道隐藏 ID | hosted 动作稳定增加一次请求，降低连续行动收益 |

当前建议优先验证 B，并保留 C 作为“当前响应未绑定”的自然恢复路径，不把 C 变成所有 hosted 调用的固定流程。A 只有在
真实 provider probe 证明模型稳定复制 ID 后才可采用。

## 6. 下一步与安全停止

1. 用户确认是否允许以 B 为主候选。
2. 获准后先做纯本地 MVT：保留 hosted `output_index`，验证 added/done identity、并行乱序完成、类型+ordinal 唯一核对。
3. 本地 MVT 通过后再冻结 TS-06 的 `provider_result_ref`，不提前实施 reconciler。
4. `tool_choice` 暂保持 provider 原生 `auto`；缺失容器时保留 hosted 输出为 unbound，不丢失、不猜配、不回滚。
5. 若要验证 Agent 是否稳定生成 ordinal 或复制 ID，需另行申请最小真实 provider 预算；当前未获授权。

本停点不推翻容器。它只阻止在 Agent 可见引用尚未证实时冻结错误字段。

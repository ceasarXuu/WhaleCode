# DeepSeek Responses 迁移决策

- Created: 2026-07-31
- Status: completed（CR-15A 至 CR-17）
- Scope: R8 缓存门禁 Phase C、WhaleCode 内置 DeepSeek provider 与模型目录

## 1. 触发事实

CR-16 使用生产 DeepSeek Chat Completions 构造链路验证 Apps 时发现：Codex 将 Apps/MCP 工具表示为
`namespace`，现有 Chat 转换器只保留顶层 `function`，导致说明进入上下文而实际工具在最终请求前被静默丢弃。

DeepSeek 于 2026-07-31 发布 V4-Flash 正式更新，官方同时声明：

1. `deepseek-v4-flash` 原生支持 Responses API，并专门适配 Codex；
2. Codex 官方接入配置使用 `wire_api = "responses"`；
3. 当前只有 Flash 支持 Codex，Pro 预计在 2026 年 8 月初支持。

当前 WhaleCode 内置 provider 仍使用 Chat Completions，模型目录仍把 Pro 作为默认模型，因此本地产品配置已落后于
官方能力边界。

## 2. 决策

采用唯一 DeepSeek provider 的 Responses 主路径：

- 内置 `deepseek` provider 切换到 Responses；
- 默认模型临时改为 `deepseek-v4-flash`；
- Pro 在官方确认 Codex 支持前不进入可选模型集合，也不得作为后台压缩默认模型；
- 不新增 `deepseek-chat` / `deepseek-responses` 双 provider；
- 不为 Chat Completions 自研 namespace 展平、改名和反向路由层；
- 既有 Chat final-wire 快照保留在 Git 历史中，当前权威基线按 Responses 重新生成。

该决策不改变 TaskSpace 语义、Map 状态机、普通 Tool schema 或 Agent 行为协议。

## 3. 执行顺序

1. CR-15A：切换 provider 协议并冻结本地请求路由合同；
2. CR-15B：收敛 Flash/Pro 的产品暴露和默认值；
3. CR-15C：重建 CR-12 至 CR-15 的 Responses final-wire 基线；
4. CR-16：在 Responses 主路径验证 Apps 与 Plugins；
5. CR-17：验证普通 MCP 工具集合。

任何一步发现需要新增协议分叉、模型级 provider 自动切换或私有 Tool 编码时，应停止并重新确认设计。

当前完成证据：

- `1e5b5c0ba`：内置 DeepSeek provider 切换到 Responses，并冻结路由合同；
- `3e0a36aba`：Flash 成为默认支持模型，Pro 暂不进入选择列表或后台压缩路径；
- `128b47d88`：Standard、三种 TaskSpace、权限和 Skill 的最终线基线迁移到 Responses；
- `d229ac0aa`：缓存契约使用真实 DeepSeek provider 身份，不再只修改 OpenAI 测试 provider 的 wire 字段；
- `60c8744ef`：Apps namespace 与 Plugin 上下文进入独立最终线合同。
- `e8a810a0d`：普通 MCP 资源工具和业务 namespace 进入独立最终线合同。

## 4. 验收

- DeepSeek Flash 以 `https://api.deepseek.com/` 为 provider 根地址，请求只发送到 `/responses`；
- Apps/MCP namespace 在生产 final-wire 中保持 Codex 原生结构；
- 默认模型和模型选择器不会提供当前官方未支持 Codex 的 Pro；
- Standard、TaskSpace、权限、Skill、Apps、Plugins 的本地两请求快照可重复；
- 不调用真实 Whale Agent，不产生 API 成本；真实 provider 兼容性验证另行申请预算。

## 5. 外部依据

- [DeepSeek Codex 接入文档](https://api-docs.deepseek.com/quick_start/agent_integrations/codex/)：Codex 使用 DeepSeek 原生 Responses API，当前仅 Flash 支持。
- [DeepSeek API 更新日志](https://api-docs.deepseek.com/updates/)：2026-07-31 V4-Flash 更新声明原生 Responses 与 Codex 适配。
- [DeepSeek Responses API 指南](https://api-docs.deepseek.com/guides/responses_api/)：Responses 请求协议与能力边界。
- [DeepSeek Tool Calls 指南](https://api-docs.deepseek.com/guides/tool_calls/)：Chat Completions 路径只暴露 function calling 结构。

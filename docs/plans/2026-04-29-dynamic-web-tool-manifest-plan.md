# 动态 Web 工具 Manifest 实施计划

日期：2026-04-29

## 结论

本计划取代同日早前的 `web_search` 内部 auto 路由方案。新的方向是：

- 复用现有 `ToolsConfig -> ToolRegistryPlan -> ToolRouter` 工具注册链路。
- 不引入独立 manifest 系统，不把内置 provider 塞进外部 `DynamicToolSpec` 执行通道。
- 在现有 web 工具注册点生成 provider-specific native function tools。
- 只暴露当前可用、已配置凭据的 search provider 工具。
- 不暴露 `auto` 选项，不在 agent 可见 schema/description 中掺入成本或预算语义。
- runtime 负责可用性、凭据、健康检查、错误保护和日志；agent 负责根据工具语义选择合适工具。

最终模型可见工具形态：

```text
brave_search
exa_search
tavily_search
github_search
stack_exchange_search
jina_search
web_fetch
```

其中 search 工具按 provider 能力动态出现；`web_fetch` 继续作为 URL 读取工具存在。

## 背景总结

前面的讨论形成了几个约束：

1. Provider 不是同级能力。GitHub/Stack Exchange 更偏结构化技术源，Exa/Tavily/Brave 更偏不同类型的 Web 发现，Jina 更适合 fetch/readability，也可以作为 credentialed search provider。
2. 多 provider 可用时，不应该由 runtime 用固定策略替 agent 决策；这违背 agent 通过工具语义选择动作的原则。
3. 也不应该暴露一个带 `auto` 的大工具，让 agent 把关键选择交回 runtime。
4. 没有配置 key/token 的 provider 不应出现在工具 manifest 中，也不应被执行阶段隐式触发。
5. 配置体验必须简单：`/search-provider` 打开 picker，选择 provider，输入 API key/token，完成。
6. 实现要尽量复用 Codex 现有工具注册、handler、secrets 和 TUI popup 机制，避免补丁式旁路。

## 当前可复用基建

现有项目已经具备这些基础：

- 每轮都会从 `ToolsConfig` 构建 `ToolRegistryPlan`，再生成 `ToolRouter`。
- `ToolRegistryPlan` 已经集中负责 tool spec 和 handler 注册。
- `web_search` / `web_fetch` 已经有 native handler。
- `codex-secrets` 已经支持本地用户级 secret 存取。
- `/search-provider` 已经有 provider picker 和 masked secret prompt 雏形。
- `core/src/web_tools/providers.rs` 已经有 provider adapter、credential lookup、routing、fetch 和日志基础。

缺口是：工具 manifest 还没有根据 provider credential availability 动态生成；agent 仍看到一个粗粒度 `web_search` 工具，并通过 `provider_policy/preferred_providers/auto` 把选择重新交给 runtime。

## 架构原则

### 复用现有链路

动态 manifest 插入点是现有 `ToolRegistryPlan` 的 web 工具注册段，而不是新建 parallel registry。

目标结构：

```text
TurnContext
  -> resolve configured/available web providers
  -> ToolsConfig.web_search_available_providers
  -> ToolRegistryPlan builds provider-specific tool specs
  -> ToolRouter registers all search tool names to WebSearchHandler
  -> WebSearchHandler maps tool name to provider and executes existing adapter
```

### Native tools，不走外部 DynamicToolSpec

`DynamicToolSpec` 适合 app/thread 注入的外部工具，执行时需要通过 dynamic tool event 回调外部客户端。Web provider 是 Whale core native capability，走 `DynamicToolSpec` 会绕远执行边界并污染架构。

### Agent 选择 provider，runtime 约束可用性

Agent 只看到已可用工具及其语义描述。例如：

- `github_search`：GitHub repo/code/issues/commits/users。
- `stack_exchange_search`：Stack Overflow/Stack Exchange Q&A。
- `exa_search`：docs、repo、changelog、技术博客、语义技术搜索。
- `tavily_search`：多页面研究、agent-native web research。
- `brave_search`：广泛 Web、新闻、独立索引。
- `jina_search`：Jina Search 的轻量网页发现。

Runtime 只负责：

- 未配置 provider 不进入 manifest。
- API key/token 不进入日志、session transcript 或 tool output。
- 执行阶段继续做真实凭据读取、HTTP 错误、限流、健康和安全保护。
- `web_fetch` 继续做 URL 安全校验、redirect 校验和截断。

## 工具设计

### Search tools

每个 search provider 是一个独立 function tool。所有 search tool 都至少支持：

```json
{
  "query": "string",
  "max_results": 8,
  "freshness": "any|day|week|month|year",
  "domains": ["github.com"],
  "exclude_domains": ["example.com"]
}
```

Provider-specific 扩展：

`github_search`：

```json
{
  "query": "string",
  "github": {
    "search_type": "repositories|code|issues|commits|users",
    "repo": "owner/name",
    "org": "org",
    "user": "user",
    "language": "rust",
    "path": "src",
    "filename": "Cargo.toml"
  }
}
```

`stack_exchange_search`：

```json
{
  "query": "string",
  "stack_exchange": {
    "site": "stackoverflow",
    "tags": ["rust"],
    "accepted": true,
    "sort": "activity|votes|creation|relevance"
  }
}
```

不在 schema 中暴露：

- `auto`
- `provider_policy`
- `preferred_providers`
- price/cost/budget 字段

### Fetch tool

`web_fetch` 保持 provider-agnostic，因为 agent 的动作是“读取这个 URL”，不是选择 readability provider。

输入保持：

```json
{
  "url": "https://example.com/page",
  "format": "markdown|text",
  "max_chars": 50000,
  "reason": "why this URL is needed"
}
```

Fetch provider 选择仍属于 runtime safety/readability 层。

## Provider 可用性规则

第一阶段 manifest 可用性只做轻量同步判断：

1. `tools.web_search.enabled = true`
2. `web_search` mode 是 `live`
3. provider 对应 secret name 可解析
4. 当前进程 env var 有非空值，或 `codex-secrets` global scope 中有非空值

满足以上条件的 search provider 才进入 manifest。

执行阶段仍要重新读取 secret 并处理失败，因为 manifest 构建和工具调用之间可能有状态变化。

## `/search-provider` UX

主路径：

```text
/search-provider
  -> 打开 provider picker
  -> 用户选择 provider
  -> 如果需要 key/token，打开 masked secret prompt
  -> 保存到 codex-secrets
  -> 写入必要 config
  -> 执行 health check
  -> 下一轮 manifest 自动包含该 provider 工具
```

高级子命令保留给调试和自动化：

```text
/search-provider status
/search-provider set <provider>
/search-provider key <provider> [ENV_VAR]
/search-provider test
/search-provider on
/search-provider off
```

但普通用户不需要理解 TOML、env var、provider role 或 routing profile。

## 实施阶段

### 阶段 1：计划和文档

- 新增本文档，明确取代 runtime auto 路由方向。
- 更新 runbook，说明动态 manifest 和 provider-specific tools。

验收：

- 文档能解释为什么复用现有工具注册链路。
- 文档不再把 `auto` 作为 agent 可见选项。
- 文档不出现价格/预算作为 agent 决策语义。

### 阶段 2：工具 manifest builder

- 在 `codex-tools` 的 web tool 定义层新增 provider-specific tool spec builder。
- 在 `ToolsConfig` 中增加已解析的 web search provider availability 字段。
- 在 `ToolRegistryPlan` web 工具注册段调用 builder。
- 保留旧 `web_search` hosted spec 作为没有 availability 输入时的兼容路径。

验收：

- 给定 `[exa, github]` availability 时，manifest 只包含 `exa_search` 和 `github_search`。
- 未给 availability 时，旧测试和上游兼容路径不被破坏。

### 阶段 3：core availability resolver

- 在 core web tools 层新增 manifest availability resolver。
- 复用 `codex-secrets` 和 env lookup。
- `TurnContext` 构建 `ToolsConfig` 时注入 resolver 结果。
- 记录结构化日志：可暴露 provider 列表、跳过数量、fetch 是否启用，不记录 secret。

验收：

- 未配置 key 的 provider 不进入 manifest。
- 配置到 env 或 `codex-secrets` 的 provider 会进入 manifest。
- 日志可诊断 manifest 构建结果。

### 阶段 4：handler provider binding

- `WebSearchHandler` 根据 tool name 识别 provider。
- Provider-specific tool 调用会强制 `Single` + 对应 `preferred_providers`。
- 保留旧 `web_search` handler 兼容已有 transcript 或旧客户端调用。

验收：

- `exa_search` 只调用 Exa adapter。
- `github_search` 只调用 GitHub adapter。
- tool schema 不再要求 agent 填 provider 或 auto。

### 阶段 5：测试与安装

- 增加 `codex-tools` manifest 单元测试。
- 增加 core availability resolver 单元测试。
- 运行相关 cargo test/check。
- 构建并安装本地 `whale.exe`。
- 更新 runbook 的验证命令。

验收：

- `cargo test -p codex-tools web_tool`
- `cargo test -p codex-tools tool_registry_plan`
- `cargo test -p codex-core web_tools`
- `cargo check -p codex-tui`
- `cargo build -p codex-cli --bin whale --locked`
- 本地安装脚本成功，`C:\Users\77585\.whale\bin\whale.exe` 更新。

## 风险与控制

- 风险：把 native provider 做成 `DynamicToolSpec` 会让执行边界混乱。控制：不走该路径。
- 风险：一次性删除旧 `web_search` 会破坏上游兼容和旧 transcript。控制：manifest 动态路径启用 provider tools，旧 handler 保留兼容。
- 风险：manifest 阶段做真实网络 health check 会变慢。控制：只做本地 credential availability，真实健康留给 `/search-provider test` 和执行阶段。
- 风险：工具数量过多污染提示词。控制：只暴露配置好的 provider；未配置 provider 不出现。
- 风险：agent 看不懂 provider 差异。控制：每个工具描述只写能力、来源和输出形态，不写成本，不写 runtime 策略。

## 后续工作

后续可以继续推进：

- `/search-provider` 管理视图显示 manifest-visible 状态。
- Provider 健康和 backoff 状态参与 manifest 但不做网络阻塞。
- `web_fetch` 进一步拆出可选 `jina_fetch` / `direct_fetch`，仅当实践证明 agent 需要显式选择 fetch provider。
- 更细粒度的 GitHub issues/code/repo 子工具，前提是当前 provider-specific 工具仍过宽。

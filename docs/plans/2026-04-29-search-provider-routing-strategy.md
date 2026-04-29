# Search Provider 路由策略修订

日期：2026-04-29

## 修订状态

本文件记录的是上一版“单 `web_search` + runtime provider 路由”方案。该方案已经被动态工具
manifest 方案取代，当前执行口径见：

- `docs/plans/2026-04-29-dynamic-web-tool-manifest-plan.md`

## 结论

计划变更：第一阶段不再只接 Brave + Jina。需要把以下 provider 都接入：

- GitHub Search API
- Exa
- Tavily
- Brave Search API
- Stack Exchange API

工具边界不变：模型可见工具仍然只保留 `web_search` 和 `web_fetch`。不把每个
provider 暴露成单独工具。

原因：

- DeepSeek 只需要理解“搜索”和“读取 URL”两个动作。
- provider 选择、fallback、扇出、去重、重排属于 Whale runtime 责任。
- 如果暴露 `github_search`、`exa_search`、`tavily_search` 等多个工具，模型会被迫理解
  provider 差异，工具选择不稳定，prompt 成本也更高。

新策略是：

```text
model-visible tools:
  web_search
  web_fetch

runtime providers:
  github
  exa
  tavily
  brave
  stack_exchange
  jina_fetch
  direct_fetch
```

## 当前进度检查

截至本次检查，仓库里已经有一批未提交的搜索能力实现改动，说明实现已经启动，但还没有达到
新计划要求。

已出现的实现面：

- `core/src/web_tools/`：已有 provider registry、Brave/Jina search、Jina/direct fetch、
  URL safety、handler 基础代码。
- `tools/src/web_tool.rs`：已有 `web_fetch` tool spec。
- `tools/src/tool_registry_plan.rs`：已有 `web_search`/`web_fetch` handler 注册雏形。
- `tui/src/chatwidget/slash_dispatch.rs` 和 `tui/src/slash_command.rs`：已有
  `/search-provider` 雏形。
- `protocol/src/config_types.rs`：已有 `WebSearchProvider`、`WebFetchProvider`、
  `WebSearchConfig` 配置结构。

当前缺口：

- provider enum 只覆盖 Brave/Jina，未覆盖 GitHub、Exa、Tavily、Stack Exchange。
- `/search-provider` 只支持 Brave/Jina，不能配置多 provider。
- `/search-provider` 仍是文本状态/子命令雏形，不符合用户要求的“选择服务商 -> 输入
  API key/token -> done”交互。
- 当前 registry 是 primary + fallback，不是按任务意图路由或多 provider blending。
- `web_search` 参数还不足以表达 `provider_policy`、`scope`、`repo`、`language`、
  `tags`、`search_kind` 等结构化意图。
- 还没有 provider 级 rate limit、budget、cost、health、cache 策略。
- 还没有 GitHub/Exa/Tavily/Stack Exchange mock provider 测试。
- 当前实现尚未按新计划跑完整 cargo 测试、构建、安装验证。

所以当前状态应标记为：

```text
设计文档：已完成第一版，需要按本文修订
代码实现：已启动，但仍是 Brave/Jina-only 雏形
验收状态：未完成
```

## 设计原则

### 只保留两个模型工具

`web_search` 负责发现信息源和轻量摘录。

`web_fetch` 负责读取已经选中的 URL。

provider 不是工具。provider 是 runtime 执行策略。

### Provider 全部接入，但不每次全部调用

默认不做无脑全 provider fanout。原因：

- 成本不可控。
- 延迟不可控。
- GitHub/Stack Exchange 有明确 rate limit 和 backoff 要求。
- 多 provider 同时返回大量重复结果，会污染上下文。

默认策略应是 router 选择 1-2 个 primary provider，必要时再补充 provider。

### 技术搜索优先结构化来源

面向 coding agent，搜索质量不只看 Web SERP 覆盖面。很多任务更需要结构化来源：

- 查某个 repo、代码、issue、PR：GitHub Search API 优先。
- 查 Stack Overflow 问答和 tag：Stack Exchange API 优先。
- 查 docs、repo、changelog、技术博客、代码参考：Exa 优先。
- 查通用 Web、新闻、非技术资料：Brave 优先。
- 查需要网页正文、extract、research、crawl/map 的任务：Tavily 优先。

### Jina 保留为 fetch/readability 层

Jina 不再作为本次“必须接入的 search provider”主角，但仍建议保留：

- `web_fetch` 默认 provider：Jina `r.jina.ai`
- 无 key / 低配置兜底：Jina `s.jina.ai` 可作为 emergency search fallback

这满足低本地复杂度要求，也降低 fetch 实现难度。

## Provider 职责矩阵

| Provider | Runtime 职责 | 默认适用 | 不适用 |
| --- | --- | --- | --- |
| GitHub Search API | repo/code/issue/PR/commit/user 搜索 | 精确 GitHub 范围、repo 内代码、issue/PR 证据 | 高频暴力搜索、通用 Web |
| Exa | 技术资料、docs、repo、changelog、Stack Overflow、语义搜索、highlights | coding agent 默认技术搜索、低 token 摘录 | 精确 repo 内符号搜索不如 GitHub API |
| Tavily | agent-native web search、extract、research、crawl/map | 多网页研究、网页正文抽取、需要 raw content 的任务 | 第一跳低成本 SERP |
| Brave | 通用 Web、新闻、独立索引、LLM context | 默认广泛搜索、非 Google fallback、实时 Web | 代码结构化搜索 |
| Stack Exchange API | Stack Overflow/Stack Exchange Q&A、tag/score/activity 筛选 | 技术问答、历史答案、按 tag 找 accepted/high-score answers | GitHub issues、Discord、项目官方 docs |
| Jina Fetch | URL 到 Markdown/text | `web_fetch` 默认 readability | provider-specific structured search |

## `web_search` 新输入

建议把 `web_search` 输入扩展为 provider-agnostic schema：

```json
{
  "query": "string",
  "max_results": 8,
  "freshness": "any|day|week|month|year",
  "source_hint": "general|technical|github|docs|community|research|news",
  "provider_policy": "auto|single|fanout",
  "preferred_providers": ["exa", "github"],
  "domains": ["github.com"],
  "exclude_domains": [],
  "github": {
    "search_kind": "auto|repositories|code|issues|commits|users",
    "repo": "owner/name",
    "org": "org",
    "user": "user",
    "language": "rust",
    "path": "src",
    "filename": "Cargo.toml"
  },
  "stack_exchange": {
    "site": "stackoverflow",
    "tags": ["rust", "tokio"],
    "accepted": true,
    "min_score": 3
  }
}
```

字段原则：

- `query` 必填。
- `provider_policy` 默认 `auto`。
- `preferred_providers` 是 hint，不是强制，除非 `provider_policy=single`。
- provider-specific 字段只传给对应 provider，不泄露到其他 provider。
- 最大结果数是最终输出上限，不是每个 provider 的上限。

## `web_search` 输出

所有 provider 统一归一化为同一种结果：

```json
{
  "query": "normalized query",
  "strategy": "auto",
  "providers_used": ["exa", "github"],
  "fallback_used": false,
  "results": [
    {
      "rank": 1,
      "provider": "github",
      "source_type": "github_code",
      "title": "owner/repo: src/foo.rs",
      "url": "https://github.com/owner/repo/blob/main/src/foo.rs",
      "domain": "github.com",
      "snippet": "matched text or summary",
      "published_at": null,
      "score": 0.91,
      "metadata": {
        "repo": "owner/repo",
        "path": "src/foo.rs",
        "language": "Rust"
      }
    }
  ],
  "diagnostics": {
    "deduped": 4,
    "rate_limited_providers": [],
    "latency_ms": 920
  }
}
```

输出要求：

- 每条结果必须带 provider。
- 每条结果必须带 source_type。
- provider 原始响应只保留必要 metadata，不把完整 raw body 塞给模型。
- diagnostics 用于调试和日志，不要求模型长期依赖。

## Provider 路由策略

默认 `provider_policy=auto`。

### GitHub 精确任务

触发条件：

- `source_hint=github`
- query 包含 `repo:`、`org:`、`user:`、`language:`、`path:`、`filename:`
- 用户明确问 repo、issue、PR、commit、README、某个文件路径

路由：

```text
primary: github
secondary: exa
fallback: brave
fetch: jina_fetch
```

说明：

- GitHub API 负责结构化精确搜索。
- Exa 补 GitHub repo/docs/changelog 的语义召回。
- Brave 只在 GitHub API 不可用或需要 broader web 证据时补充。

### 技术资料 / docs / changelog

触发条件：

- `source_hint=technical|docs`
- 用户问最新 API、库行为、框架迁移、版本差异、错误信息

路由：

```text
primary: exa
secondary: brave
optional: github
fetch: jina_fetch
```

说明：

- Exa 对 coding agent 的 docs/repo/Stack Overflow/changelog 搜索更贴近。
- Brave 用于广泛 Web 补充。
- 如果 query 提到 repo 或 GitHub，再补 GitHub provider。

### 技术社区 / Q&A

触发条件：

- `source_hint=community`
- 用户问报错、坑、Stack Overflow、tag、accepted answer、社区讨论

路由：

```text
primary: stack_exchange
secondary: exa
fallback: brave
fetch: jina_fetch
```

说明：

- Stack Exchange API 可按 tag、score、activity、creation、fromdate/todate 筛选。
- Exa 补 Stack Overflow 之外的博客、GitHub issues、论坛。
- Brave 用于广泛 fallback。

### 通用 Web / 新闻 / 非技术资料

触发条件：

- `source_hint=general|news`
- 用户问公司、产品、新闻、价格、政策、广泛事实

路由：

```text
primary: brave
secondary: tavily
fallback: exa
fetch: jina_fetch
```

说明：

- Brave 是广泛 Web 默认入口。
- Tavily 适合需要更完整网页上下文或 research 的场景。
- Exa 可作为语义补充。

### 多网页研究 / 深度调研

触发条件：

- `source_hint=research`
- 用户要求比较、综述、调研多个来源、做方案评估

路由：

```text
primary: tavily
secondary: exa
fallback: brave
fetch: jina_fetch
```

说明：

- Tavily 的 search/extract/research/crawl/map 能力适合更重研究任务。
- 第一阶段仍只通过 `web_search` 返回候选和摘要，不默认暴露 crawl/map/extract 工具。

## Provider 配置策略

配置不再是单一 primary + fallback，而是 profiles + provider registry。

建议配置：

```toml
[tools.web_search]
enabled = true
strategy = "auto"
max_results = 8
timeout_ms = 10000
max_providers_per_query = 2
cache_ttl_seconds = 900

[tools.web_search.providers.brave]
enabled = true
api_key_env = "BRAVE_SEARCH_API_KEY"
role = ["general", "news", "fallback"]

[tools.web_search.providers.exa]
enabled = true
api_key_env = "EXA_API_KEY"
role = ["technical", "docs", "research", "community"]

[tools.web_search.providers.tavily]
enabled = true
api_key_env = "TAVILY_API_KEY"
role = ["research", "general", "extract_candidate"]

[tools.web_search.providers.github]
enabled = true
api_key_env = "GITHUB_TOKEN"
role = ["github", "code", "repo", "issues"]

[tools.web_search.providers.stack_exchange]
enabled = true
api_key_env = "STACK_EXCHANGE_KEY"
access_token_env = "STACK_EXCHANGE_ACCESS_TOKEN"
site = "stackoverflow"
role = ["community", "qa"]

[tools.web_fetch]
enabled = true
provider = "jina"
fallback_provider = "direct"
max_chars = 50000
timeout_ms = 15000
```

API key 策略：

- 面向普通用户的主路径不是手动写 env var，而是通过 `/search-provider` 交互式输入
  API key/token。
- 交互式输入的 secret 写入 Whale 用户级 credential store，不写入仓库、计划文档、
  session transcript 或普通日志。
- Windows 本地优先用系统凭据/DPAPI 能力；若实现初期需要文件型 credential store，
  必须位于 `%USERPROFILE%\.whale`，并做本机用户权限限制和全链路脱敏。
- env var 仍作为高级/自动化路径保留，例如 `BRAVE_SEARCH_API_KEY`、`EXA_API_KEY`、
  `TAVILY_API_KEY`、`GITHUB_TOKEN`、`STACK_EXCHANGE_KEY`。
- credential store 优先级高于 env var 还是低于 env var，需要在实现设计里固定；建议
  默认优先 env var，避免覆盖 CI/脚本环境，交互式输入只写本机用户配置。
- 没有 key/token 的 provider 进入 `unconfigured` 状态，不影响其他 provider。

## `/search-provider` 命令修订

关键 UX 决策：`/search-provider` 裸命令必须打开选择器，而不是打印一段状态和要求
用户记子命令。这个交互要和现有选择型命令保持一致：`/model` 打开模型选择 popup，
`/permissions` 打开权限选择 popup，`/search-provider` 也应打开 provider 选择 popup。

主流程：

```text
/search-provider
  -> 打开 SearchProviderPicker
  -> 用户选择 provider
  -> 如果未配置 key/token，打开 secret input prompt
  -> 用户输入 API key/token
  -> Whale 保存到用户 credential store
  -> 立即执行 provider health check
  -> 成功后启用 provider，并返回 done/status
```

Provider picker 首屏至少包含：

- Auto router
- GitHub Search API
- Exa
- Tavily
- Brave Search API
- Stack Exchange API

每个 provider 行展示：

- 名称。
- 适用场景短标签，例如 `GitHub code/issues`、`Docs/changelog`、`Research`、
  `General web`、`Stack Overflow Q&A`。
- 状态：`configured`、`needs key`、`healthy`、`rate-limited`、`disabled`。
- 费用/限流提示的短文案，例如 `requires key`、`token recommended`。

Provider 选择后的行为：

- 选择 `Auto router`：打开 provider 管理视图，允许一次性看到所有 provider 状态；不要求
  立即输入 key。
- 选择 GitHub：提示输入 `GITHUB_TOKEN` 或选择跳过；如果跳过，只启用无需认证的公共能力，
  并在 code/private search 场景提示需要 token。
- 选择 Exa：提示输入 Exa API key。
- 选择 Tavily：提示输入 Tavily API key。
- 选择 Brave：提示输入 Brave Search API key。
- 选择 Stack Exchange：提示输入 Stack Exchange key；access token 作为高级选项，不放在
  第一层主流程。

Secret input 要求：

- 输入框必须 mask secret。
- 支持 Esc 取消，不落盘。
- 提交后只显示 `saved` / `test ok` / `test failed: <sanitized error>`。
- 任何错误输出不得包含 secret 原文或 Authorization header。
- 如果 health check 失败，保留 secret 但标记 provider 为 `configured/unhealthy`，并允许用户
  立即重试或删除。

完成态要求：

- 成功配置后显示一行短状态，例如：

```text
Exa configured and healthy. Auto router will use it for docs, changelog, and technical search.
```

- 不需要用户再运行别的命令才生效。
- 不要求用户理解 config TOML、env var、provider role 或 routing profile。

高级命令可以保留，但不能成为主路径：

```text
/search-provider status
/search-provider list
/search-provider test all
/search-provider test exa
/search-provider off
/search-provider on
```

这些高级命令只服务自动化、debug 和 power user。普通用户只需要记住
`/search-provider`。

## 成本与限流策略

必须内置 provider 级 budget 和 backoff。

建议字段：

```toml
[tools.web_search.budget]
max_provider_calls_per_turn = 4
max_provider_calls_per_query = 2
max_fetch_calls_per_turn = 6
prefer_free_or_configured = true
```

运行时要求：

- GitHub 读取 rate-limit headers，遇到 primary/secondary limit 后进入 provider backoff。
- Stack Exchange 遇到 `backoff` 字段时必须按秒等待或跳过该 provider。
- Tavily/Exa/Brave 遇到 429 时进入 provider backoff。
- provider backoff 状态必须体现在 `/search-provider status` 和日志里。
- 默认对同一个 query 做短 TTL 缓存，避免重复付费和触发限流。

## Fetch 策略

`web_fetch` 不因为多 provider 接入而膨胀成综合工具。

默认：

```text
primary: jina_fetch
fallback: direct_fetch
```

特殊处理：

- GitHub blob URL 转 raw URL。
- GitHub API 搜索结果如包含 API URL，可转换为 html_url/raw_url。
- Stack Exchange 结果如果已有 question/answer body，可不再 fetch；需要完整上下文时再
  fetch 页面 URL。
- Exa/Tavily 已返回 highlights/raw_content 时，先把它作为 search result snippet/content
  metadata，不默认塞满正文。

`web_fetch` 仍必须保留 URL 安全校验：

- 禁止 `file://`
- 禁止 localhost/私有网段/link-local/metadata service
- redirect 后重新校验
- 默认截断正文

## 结果融合策略

多 provider 输出需要重排，不是简单拼接。

重排信号：

- provider role 是否匹配 source_hint
- URL/domain 可信度
- GitHub repo/path/language 精确匹配
- Stack Exchange accepted answer / score / activity
- Exa highlight score
- Tavily search score / raw content availability
- Brave freshness / domain diversity
- 是否和用户 query 中的 repo/package/version 精确匹配

去重规则：

- canonical URL 去重。
- GitHub blob/raw/html URL 归一到同一资源。
- Stack Overflow question URL 带 answer anchor 时归一到 question + answer id。
- 同域同标题高度相似结果降权。

## 日志与事件修订

新增 provider 维度事件：

- `search.started`
- `search.route_selected`
- `search.provider_started`
- `search.provider_result`
- `search.provider_backoff`
- `search.provider_error`
- `search.results_merged`
- `search.finished`
- `fetch.started`
- `fetch.provider_started`
- `fetch.finished`
- `fetch.blocked`

字段：

- strategy
- source_hint
- providers_selected
- provider
- provider_status
- query_hash
- query_shape_hash
- result_count
- deduped_count
- top_domains
- cost_units
- rate_limit_remaining
- backoff_seconds
- latency_ms
- error_class

默认不记录 raw query、API key、完整 URL query string。

## 实施阶段重排

### 阶段 0：冻结当前雏形并重对齐计划

- 不把当前 Brave/Jina-only 实现视为完成。
- 先按本文更新配置模型、provider enum、router 设计。
- 保留现有 URL safety、Jina fetch、web_fetch tool spec 作为可复用基础。

验收：

- 文档指向本文。
- 当前 provider 列表和 `/search-provider` 设计不再只有 Brave/Jina。

### 阶段 1：Provider registry 与 mock 全覆盖

- 定义统一 `SearchProvider` trait。
- 定义 `ProviderId`：`github|exa|tavily|brave|stack_exchange|jina`。
- 定义 `SearchRoute` 和 `SearchProviderRouter`。
- 为五个 search provider 加 mock adapter。
- 加 result normalization、dedupe、rerank 单元测试。

验收：

- 无网络也能测试所有 provider 路由。
- `source_hint` 到 provider 选择有测试。

### 阶段 2：配置与 `/search-provider`

- 扩展 config schema。
- 扩展 `/search-provider` 命令，但主路径必须是 popup/picker，不是子命令教程。
- 新增 SearchProviderPicker：复用现有 TUI popup 交互风格，行为上对齐 `/model` 和
  `/permissions`。
- 新增 masked secret input prompt：选择 provider 后输入 API key/token。
- 新增用户级 credential store 写入、读取、删除和脱敏显示。
- 加 provider health state。
- 保留 env var 配置作为高级路径，但不能替代交互式 key 输入。

验收：

- 裸 `/search-provider` 会打开 provider picker。
- 选择 provider 后能输入 key/token 并完成 health check。
- 成功后 provider 立即启用，不需要用户再编辑配置文件或运行额外命令。
- 能启停每个 provider。
- 能逐 provider test。
- secret 不进输出、日志或 session transcript。

### 阶段 3：结构化技术 provider

- 实现 GitHub Search API。
- 实现 Stack Exchange API。
- 处理 GitHub rate-limit headers。
- 处理 Stack Exchange `backoff`。

验收：

- GitHub repo/code/issues 搜索 mock + env-gated smoke。
- Stack Exchange tag/question/answer 搜索 mock + env-gated smoke。

### 阶段 4：Agent 搜索 provider

- 实现 Exa。
- 实现 Tavily。
- 接入 highlights/raw_content 到 normalized result。
- 不默认暴露 Tavily extract/crawl/map 为模型工具。

验收：

- Exa technical/docs 路由 smoke。
- Tavily research 路由 smoke。
- 成本和超时可配置。

### 阶段 5：Brave/Jina 与 fetch 收口

- 保留 Brave 广泛搜索。
- 保留 Jina fetch。
- 可选保留 Jina search 为 emergency no-key fallback。
- 完成 URL safety 和 fetch event。

验收：

- 通用 Web 路由走 Brave。
- 无 search provider key 时能给出清晰配置提示，或按配置使用 Jina emergency fallback。

### 阶段 6：DeepSeek tool 注入和端到端验证

- DeepSeek Chat Completions 只看到 `web_search`/`web_fetch` function tools。
- OpenAI/Responses provider-hosted `web_search` 原路径不破坏。
- 端到端模拟：搜索 -> fetch -> 回答。

验收：

- request capture 证明 DeepSeek 收到 function-style tools。
- 关闭搜索时不注入工具。
- cargo fmt/check/test 通过。
- 本地安装 Whale 后 `/search-provider status/test` 可用。

## 参考来源

- [GitHub REST API Search](https://docs.github.com/rest/search/search)
- [GitHub REST API Rate Limits](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api)
- [Exa Search API for Coding Agents](https://exa.ai/docs/reference/search-api-guide-for-coding-agents)
- [Exa API Pricing](https://exa.ai/pricing)
- [Tavily Search API](https://docs.tavily.com/documentation/api-reference/endpoint/search)
- [Tavily Credits and Pricing](https://docs.tavily.com/documentation/api-credits)
- [Brave Search API](https://brave.com/search/api/)
- [Stack Exchange API Throttles](https://api.stackexchange.com/docs/throttle)

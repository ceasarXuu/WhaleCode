# Web 搜索工具实施计划

日期：2026-04-29

## 修订状态

本文件记录的是第一版 Brave + Jina 方案，已被同日的新 provider 路由策略取代。

当前执行口径见：

- `docs/plans/2026-04-29-dynamic-web-tool-manifest-plan.md`

## 结论

第一阶段明确拆成两个工具：

- `web_search`：负责发现信息源，返回候选网页列表。
- `web_fetch`：负责读取指定 URL，返回可供模型推理的正文内容。

不做一个综合型 `search` 工具，也不在第一阶段暴露 `extract`、`crawl`、`map`、
browser automation 或 GitHub 专用搜索工具。

这个决策基于参考项目的共同模式：Cline 使用 `web_search` + `web_fetch`，
OpenCode 使用 `websearch` + `webfetch`，Open WebUI 使用 `search_web` +
`fetch_url`。它们都把“找来源”和“读内容”分开。

## 为什么不做综合 search

综合型搜索工具看起来简单，但对 coding agent 不合适：

- 搜索结果和正文内容混在一起，模型很难判断哪些来源值得深入读。
- 一次调用可能塞入大量正文，容易浪费上下文和 token。
- Brave 与 Jina 的能力边界不同，如果统一成一个大工具，fallback 行为会变得不透明。
- 日志不好排查：无法区分是搜索质量差、结果排序差，还是页面读取失败。
- 权限不好控制：搜索是低风险发现动作，读取 URL 是更高风险的网络访问动作。

拆成两步后，agent 的行为更接近人类调研流程：

1. 先搜一组候选来源。
2. 根据标题、摘要、域名、时间选择可信来源。
3. 再读取少量具体 URL。
4. 对多个来源做交叉验证。

## 第一阶段工具

### `web_search`

用途：发现候选信息源。

输入建议：

```json
{
  "query": "string",
  "max_results": 8,
  "freshness": "any|day|week|month|year",
  "domains": ["github.com", "stackoverflow.com"],
  "exclude_domains": [],
  "source_hint": "general|technical|github|docs|community"
}
```

输出建议：

```json
{
  "provider": "brave|jina",
  "query": "redacted-or-normalized query",
  "results": [
    {
      "rank": 1,
      "title": "string",
      "url": "https://example.com/page",
      "domain": "example.com",
      "snippet": "string",
      "published_at": "optional",
      "source_type": "web|docs|github|community|news|unknown"
    }
  ],
  "fallback_used": false,
  "latency_ms": 123
}
```

行为要求：

- 默认 provider 是 Brave。
- Brave 未配置、不可用、超时或限流时，fallback 到 Jina `s.jina.ai`。
- 面向技术问题时做 query shaping，而不是硬编码一个 GitHub crawler。
- 支持 `site:github.com`、官方 docs 域名、Stack Overflow、GitHub Issues/
  Discussions 等技术来源倾向。
- 返回轻量结果，不主动抓取每个结果的完整正文。
- 结果必须带 provider、rank、domain、snippet，方便模型选择下一步。

### `web_fetch`

用途：读取已知 URL。

输入建议：

```json
{
  "url": "https://example.com/page",
  "format": "markdown|text",
  "max_chars": 50000,
  "reason": "why the agent wants this URL"
}
```

输出建议：

```json
{
  "provider": "jina|direct",
  "url": "https://example.com/page",
  "final_url": "https://example.com/page",
  "title": "string",
  "content": "markdown or text",
  "truncated": false,
  "content_chars": 12345,
  "latency_ms": 456
}
```

行为要求：

- 默认使用 Jina `r.jina.ai`，因为它对 HTML、GitHub 页面、动态页面和 Markdown
  转换更友好。
- 后续可以补一个轻量 direct HTTP/readability fallback，但第一阶段不依赖本地
  headless browser。
- URL 必须来自用户显式提供，或来自前序 `web_search` / `web_fetch` 结果。
- 默认只允许 `http` / `https`。
- 必须阻止内网地址、本机地址、文件 URL、私有 IP 段和可疑 redirect。
- GitHub blob URL 可以转换成 raw URL，以提高读取质量。
- 输出必须声明是否截断，避免模型误以为读到了完整页面。

## 第一阶段不做的工具

### 不做 `web_extract`

结构化抽取很有价值，但不应进入第一阶段默认工具集。

原因：

- 它依赖 schema、抽取 prompt、模型二次处理和质量评估。
- 它更接近 Firecrawl/Tavily 的高级能力，而不是通用 coding agent 的基本搜索能力。
- 可以先让模型通过 `web_fetch` 读取内容后自行整理，等搜索链路稳定后再引入
  `web_extract`。

### 不做 `web_crawl` / `web_map`

爬站和站点地图发现会明显增加本地和远端复杂度。

原因：

- 对 coding agent 的常见需求来说，先搜索再读取少数页面更可控。
- crawl/map 容易产生大量请求，需要更强的速率限制、robots 处理、缓存和审计。
- 它们适合后续作为高级 web module 或 MCP provider，不适合默认能力。

### 不做 browser automation

浏览器自动化适合登录态、JS-heavy 页面和交互式页面，但第一阶段太重。

原因：

- 需要浏览器 runtime、截图、会话管理、超时控制和更复杂的安全策略。
- 用户要求减少本地复杂度。
- 对技术文档、GitHub、社区帖子，大多数场景用 Brave + Jina 已经足够。

### 不做 GitHub 专用搜索工具

GitHub 很重要，但第一阶段不做独立 `github_search`。

原因：

- GitHub API 会带来单独鉴权、rate limit、代码搜索语义和私有仓库权限问题。
- Brave 对 GitHub 公共内容覆盖较广，Jina 可读取 GitHub 页面。
- 第一阶段先通过 query shaping 支持 GitHub 倾向搜索。

后续如果要做 GitHub 专用工具，应作为独立能力设计，和用户 GitHub 登录、私有仓库
权限、issue/PR/code search 语义一起处理。

## Provider 决策

默认 provider 链路：

```text
web_search:
  Brave Search API
    -> Jina s.jina.ai fallback

web_fetch:
  Jina r.jina.ai
    -> direct HTTP/readability fallback（可选后续）
```

Brave 作为主搜索：

- 覆盖面广，有独立 web index。
- 适合技术站、GitHub、新闻、文档、社区等通用搜索。
- 支持 freshness、domain/filter、Goggles、extra snippets、LLM context 等能力。

Jina 作为兜底：

- 接入轻，默认不需要用户配置 API key。
- `s.jina.ai` 可以搜索并返回已读取的可用内容。
- `r.jina.ai` 很适合 URL 到 Markdown/text 的转换。
- 适合在 Brave 没 key、限流或读取页面时兜底。

暂不默认接入：

- Exa：适合语义搜索和技术资料，但当前用户更倾向 Brave + Jina。
- Tavily：agent 友好，但会增加 provider 面和付费配置复杂度。
- SearXNG：无 key 但需要本地或远端实例，违背“不要重本地托管”的第一阶段要求。
- DDGS：轻但不稳定，适合实验 fallback，不适合作为可靠默认。
- Firecrawl：强在 scrape/extract/crawl，但第一阶段过重。

## `/search-provider` 命令

`/search-provider` 只做配置和诊断，不生成自然语言回答。
自然语言搜索请求必须进入 Agent/Model 路径，由模型决定是否调用工具。

第一阶段命令：

```text
/search-provider
/search-provider set brave
/search-provider fallback jina
/search-provider key brave
/search-provider test
/search-provider off
```

命令行为：

- `/search-provider` 显示当前 provider、fallback、是否启用、key 来源、最近一次
  health check 状态。所有 secret 必须脱敏。
- `/search-provider set brave` 设置 Brave 为主搜索 provider。
- `/search-provider fallback jina` 设置 Jina 为 fallback provider。
- `/search-provider key brave` 引导用户配置 `BRAVE_SEARCH_API_KEY` 或 Whale 用户级
  credential store。
- `/search-provider test` 执行小型健康检查，显示 provider、延迟、结果数和脱敏错误。
- `/search-provider off` 关闭 web search/fetch 工具注入。

不做：

- 不在仓库文件里写 API key。
- 不把 API key 打进日志、debug 输出或 session transcript。
- 不让 slash command 假装成 agent 回答。

## 配置草案

配置可以先落在 Whale/Codex 现有 config 体系中：

```toml
[tools.web_search]
enabled = true
provider = "brave"
fallback_provider = "jina"
brave_api_key_env = "BRAVE_SEARCH_API_KEY"
max_results = 8
timeout_ms = 10000

[tools.web_fetch]
enabled = true
provider = "jina"
max_chars = 50000
timeout_ms = 15000
```

如果后续支持用户级 credential store，优先级建议：

1. 当前进程环境变量。
2. 用户级 Whale credential store。
3. config 中的 key 引用名。
4. 未配置时降级到 Jina fallback 或提示用户配置。

## 架构落点

实现上不要把 Brave/Jina 写死在工具 handler 里。

建议拆成：

- `SearchProvider`：搜索 provider trait。
- `FetchProvider`：URL 读取 provider trait。
- `BraveSearchProvider`：Brave API 实现。
- `JinaSearchProvider`：Jina `s.jina.ai` 实现。
- `JinaFetchProvider`：Jina `r.jina.ai` 实现。
- `SearchProviderRegistry`：根据 config 和健康状态选择 provider。
- `web_search` tool handler：只处理模型工具调用、权限、日志、fallback。
- `web_fetch` tool handler：只处理 URL 校验、权限、读取、日志、截断。

DeepSeek 路径必须是 function-style tool：

```text
DeepSeek Chat Completions
  -> model emits function call web_search/web_fetch
  -> Whale executes provider
  -> Whale returns function result to model
```

不要把 provider-hosted `web_search` 直接发给 DeepSeek，因为 DeepSeek API 没有这个
托管工具契约。

## 安全与权限

基础要求：

- `web_search` 默认可以比 `web_fetch` 更宽松。
- `web_fetch` 必须校验 URL 来源、scheme、host、redirect 和内网地址。
- 默认禁止访问：
  - `file://`
  - `localhost`
  - `127.0.0.0/8`
  - `::1`
  - RFC1918 私有网段
  - link-local 地址
  - metadata service 地址
- redirect 后重新校验最终 URL。
- 默认限制正文长度，例如 50k 字符。
- 默认限制每轮 tool 调用次数，避免模型无限搜索。

权限建议：

```text
web_search: allow 或 ask，取决于现有网络权限模式
web_fetch: ask 或 allow-with-safety-checks
```

具体接入时要复用 Codex 现有 permission/sandbox/tool approval 体系，不单独发明一套。

## 日志与事件

必须新增结构化日志和 session event，方便之后排查搜索质量问题。

建议事件：

- `search.started`
- `search.provider_request`
- `search.provider_result`
- `search.fallback`
- `search.finished`
- `fetch.started`
- `fetch.provider_request`
- `fetch.finished`
- `fetch.blocked`

日志字段：

- provider
- fallback_provider
- fallback_used
- query_hash
- domains
- result_count
- top_domains
- url_hash
- final_domain
- latency_ms
- status
- error_class
- truncated
- content_chars

默认不记录 raw query 和完整 URL query string，避免泄露 token、内部路径或用户隐私。

## 测试计划

第一阶段必须有 mock 测试，不依赖真实网络。

单元测试：

- provider config 解析。
- provider 优先级和 fallback。
- API key 脱敏。
- Brave response 解析。
- Jina search response 解析。
- Jina fetch response 解析。
- URL 安全校验。
- redirect 后重新校验。
- GitHub blob URL 到 raw URL 转换。
- 内容截断标记。

TUI/命令测试：

- `/search-provider` 状态显示。
- `/search-provider set brave`。
- `/search-provider fallback jina`。
- `/search-provider key brave` 不泄露 secret。
- `/search-provider test` 成功和失败输出。
- `/search-provider off` 后工具不注入。

协议/模型请求测试：

- DeepSeek Chat Completions 请求里注入的是 function-style `web_search` /
  `web_fetch`。
- provider-hosted `web_search` 不发送给 DeepSeek。
- 关闭搜索后模型可见工具列表不包含 web search/fetch。

日志测试：

- search started/result/fallback/finished 事件完整。
- fetch started/blocked/finished 事件完整。
- 错误事件包含 error_class，但不含 API key。

真实网络 smoke：

- `BRAVE_SEARCH_API_KEY` 存在时跑 Brave 搜索 smoke。
- 无 Brave key 时跑 Jina fallback smoke。
- Jina fetch 读取一个公开文档 URL。
- 这些测试默认 env-gated，不阻塞普通 CI。

## 分阶段交付

### 阶段 1：配置与命令

- 加 config schema。
- 加 `/search-provider` 命令。
- 加 key 来源显示和脱敏。
- 加 provider health check 框架。
- 不向模型暴露工具。

验收：

- 命令可用。
- key 不泄露。
- 文档和测试覆盖配置行为。

### 阶段 2：Provider 抽象和 mock 实现

- 加 search/fetch provider trait。
- 加 mock provider。
- 加 provider registry。
- 加 fallback 决策。
- 加结构化事件。

验收：

- mock 测试覆盖 provider 选择、fallback、日志。
- 无真实网络也能测试完整链路。

### 阶段 3：Brave + Jina 实现

- 实现 Brave search。
- 实现 Jina search fallback。
- 实现 Jina fetch。
- 加 URL 安全校验和截断。

验收：

- mock 测试通过。
- env-gated smoke 可跑通。
- 错误、限流、超时会 fallback 或给出可诊断错误。

### 阶段 4：DeepSeek 工具注入

- 将 `web_search` / `web_fetch` 作为 function tools 注入 DeepSeek。
- 保留 OpenAI/Responses provider-hosted web_search 的原有路径。
- 接入 tool result 返回。
- 接入 session event/replay。

验收：

- DeepSeek 请求捕获测试证明工具是 function-style。
- 关闭搜索时不注入工具。
- 一次“搜索 -> 读取 -> 回答”的模拟会话通过。

### 阶段 5：本地安装与回归

- cargo fmt/check/test。
- 构建并本地安装 Whale。
- `whale debug models` 或新增 debug 命令确认 search provider 配置。
- 文档记录安装、配置、测试经验。

验收：

- 本地 Whale 生效。
- git 工作区干净。
- commit 并 push。

## 后续能力

等第一阶段稳定后再考虑：

- `web_extract`：结构化抽取。
- `github_search`：GitHub API 专用搜索。
- Exa provider：技术/语义搜索模式。
- Tavily provider：更强 agent research。
- SearXNG/DDGS：可选 no-key provider。
- Browser automation：动态页面、登录态、复杂交互。

这些能力都不进入第一阶段默认工具集。

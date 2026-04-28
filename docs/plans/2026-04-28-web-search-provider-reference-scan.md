# Web Search Provider Reference Scan

Date: 2026-04-28

## Status

Reference scan for Whale web search. This is not the implementation decision.
It records the product constraints, competitor patterns, and the shape that the
next design must preserve before adding `/search-provider` or DeepSeek function
tools.

## Product Constraints

User requirements for the first Whale search capability:

- Search sources must be broad and general, with strong coverage for technical
  sites, GitHub, documentation, and technical communities.
- Do not add heavy local hosting requirements. Self-hosted search such as
  SearXNG can stay as an optional future path, not the default product path.
- Preferred provider direction is Brave Search as the main provider and Jina as
  a lightweight fallback.
- The CLI/TUI must support `/search-provider` so users can configure and verify
  provider choice and API keys.
- Do not decide too early whether Whale exposes `search`, `fetch`, and
  `extract` as separate tools. First check how reference agents split those
  responsibilities.

## Reference Findings

### Hosted Search Tools

OpenAI exposes provider-hosted `web_search` in the Responses API. The API can
return structured web-search call actions such as search/open/find-in-page and
citations. This is a provider-side tool contract, not a client-implemented
function tool.

Reference: [OpenAI Web Search](https://developers.openai.com/api/docs/guides/tools-web-search)

Anthropic exposes hosted `web_search` and separate hosted `web_fetch`. Search is
executed by the API, while fetch is restricted to explicit or previously
discovered URLs to reduce exfiltration risk. This is an important safety pattern
for Whale if fetch becomes a separate tool.

References:

- [Anthropic Web Search Tool](https://platform.claude.com/docs/en/agents-and-tools/tool-use/web-search-tool)
- [Anthropic Web Fetch Tool](https://platform.claude.com/docs/en/agents-and-tools/tool-use/web-fetch-tool)

Gemini exposes Grounding with Google Search as a built-in tool that lets the
model decide when to search and returns grounding metadata for citations.

Reference: [Gemini Grounding with Google Search](https://ai.google.dev/gemini-api/docs/google-search)

Whale implication: DeepSeek API does not provide an equivalent hosted search
tool, so Whale should not model this as "DeepSeek native search." For DeepSeek,
search must be a Whale-side function/tool implementation.

### Coding Agent Products

Cline exposes two web tools: `web_search` for discovery and `web_fetch` for
reading specific URLs. Its docs state that these call Cline's backend API and
are only available with the Cline provider.

Reference: [Cline Web Tools](https://docs.cline.bot/features/web-tools)

OpenCode exposes `webfetch` and `websearch` as built-in tools. Its docs make the
boundary explicit: use `websearch` for discovery and `webfetch` for retrieving
content from a known URL. OpenCode's web search currently uses Exa-hosted MCP
without requiring an API key.

Reference: [OpenCode Tools](https://opencode.ai/docs/tools/)

Open WebUI's agentic mode exposes `search_web` and `fetch_url`. Search returns
SERP-like snippets and does not do RAG; if snippets are insufficient, the model
can call `fetch_url` for full page content. It also supports many provider
backends, including Brave, DDGS, Exa, Jina, SearXNG, SerpAPI, Tavily, and
Perplexity.

References:

- [Open WebUI Agentic Search](https://docs.openwebui.com/features/chat-conversations/web-search/agentic-search/)
- [Open WebUI Providers](https://docs.openwebui.com/features/chat-conversations/web-search/providers/bing/)

Continue treats web and URL access primarily as context providers. `@Web`
selects relevant pages from the web, `@Url` converts a given URL to markdown,
and `@Search` is local code search powered by ripgrep.

Reference: [Continue Context Providers](https://docs.continue.dev/customize/custom-providers)

OpenHands uses browser automation for heavier web interaction. It is useful for
clicking, forms, and dynamic sites, but it adds more runtime complexity than a
first-stage search provider.

Reference: [OpenHands Browser Use](https://docs.openhands.dev/sdk/guides/agent-browser-use)

Qwen Code exposes `web_fetch` as a one-URL fetch and summarization/extraction
tool. It converts HTML to markdown/text, upgrades HTTP to HTTPS, and converts
GitHub blob URLs to raw URLs.

Reference: [Qwen Code Web Fetch](https://qwenlm.github.io/qwen-code-docs/en/tools/web-fetch/)

### Search Provider And MCP Projects

Brave is a strong fit for the primary provider because it is a broad web index,
has AI/RAG-oriented endpoints, supports web/local/image/video/news and LLM
context variants, and exposes query controls such as freshness, result filters,
Goggles, and extra snippets.

References:

- [Brave Search API](https://brave.com/search/api/)
- [Brave Search MCP Server](https://github.com/brave/brave-search-mcp-server)

Jina is a strong fallback because it has a very low integration burden:
`r.jina.ai` turns URLs into LLM-friendly content, and `s.jina.ai` searches the
web and returns top results with fetched readable content. It also supports
in-site search through `site` query parameters.

Reference: [Jina Reader](https://github.com/jina-ai/reader)

Tavily's MCP exposes search and extract, with remote MCP and OAuth/API-key
options. It is agent-oriented, but it adds another paid provider and more
surface area than the requested Brave + Jina first path.

Reference: [Tavily MCP Server](https://docs.tavily.com/documentation/mcp)

Firecrawl splits web access into scrape, map, search, crawl, extract, agent, and
browser-session tools. This is useful evidence that extraction/crawling are
separate capabilities, but also evidence that exposing everything at once would
make Whale's first product path too heavy.

Reference: [Firecrawl MCP Server](https://docs.firecrawl.dev/mcp-server)

## Pattern Synthesis

Reference products converge on these patterns:

- Provider-hosted tools exist for OpenAI, Anthropic, and Gemini, but they depend
  on the model provider executing the tool. That does not map to DeepSeek.
- Coding agents tend to split discovery from retrieval:
  `web_search`/`search_web` for finding candidate URLs and `web_fetch`/`fetch_url`
  for reading known URLs.
- Extraction, crawl, map, and browser automation are usually separate advanced
  tools or MCP integrations, not the minimal default search surface.
- Provider configurability belongs outside the model prompt path. Products use
  settings, config files, env vars, OAuth, or provider dashboards rather than
  asking the model to handle keys.
- Good implementations carry citations/source metadata and enough event data to
  debug search quality, fallback behavior, and provider failures.

## Candidate Direction For Whale

This is the direction to explore next, not a final tool contract:

- Primary provider: Brave Search API.
- Fallback provider: Jina Reader/Search.
- Optional future providers: Exa for semantic/technical search, Tavily for
  richer agent research, SearXNG/DDGS only as optional no-key paths because they
  are more brittle or add local/product complexity.
- Preserve a provider abstraction that can support separate discovery,
  retrieval, and extraction phases even if the first user-visible tool is small.
- Prefer remote APIs over local hosting in the default install.

Initial provider selection policy:

1. If a Brave API key is configured, use Brave first.
2. If Brave is unavailable, rate-limited, or not configured, use Jina fallback.
3. If the user asks for GitHub or technical-community-heavy results, shape the
   query before it reaches the provider rather than hardcoding a separate local
   crawler. Examples: `site:github.com`, `site:github.com OR site:stackoverflow.com`,
   official docs domains, project issue trackers, and language/framework forums.
4. Keep direct GitHub API search as a later optional provider, not the MVP,
   because it introduces separate auth, rate-limit, and code-search semantics.

## `/search-provider` UX Requirements

The command should be configuration plumbing, not a model answer shortcut.
Natural-language search requests must still go through the agent/model path.

Required command surfaces:

- `/search-provider` shows current provider, fallback provider, mode, configured
  key source, and last health-check status with secrets redacted.
- `/search-provider set brave` selects Brave as primary provider.
- `/search-provider fallback jina` selects Jina as fallback provider.
- `/search-provider key brave` starts a redacted key-entry flow for
  `BRAVE_SEARCH_API_KEY` or the Whale user credential store.
- `/search-provider test` runs a small provider health check and reports status,
  latency, provider used, and sanitized error details.
- `/search-provider off` disables Whale web search tools for users who want no
  network search.

Security requirements:

- Never store API keys in repo files, docs, transcripts, or logs.
- Prefer env vars first for portability: `BRAVE_SEARCH_API_KEY`.
- If Whale stores a key, store it under the user Whale runtime home with local
  file permissions and redaction in all debug output.
- Log provider, query hash, top domains, result count, latency, fallback reason,
  and error class. Do not log raw query text by default if it may contain user
  secrets.

## Tool Boundary Questions To Decide Later

Do not lock these in during the provider configuration work:

- Whether the first DeepSeek function tool is only `web_search`, or whether
  Whale exposes both `web_search` and `web_fetch`.
- Whether Brave LLM Context should be presented as search output, fetch output,
  or an internal enrichment strategy.
- Whether Jina `s.jina.ai` should be treated as search, search-plus-fetch, or
  fallback enrichment.
- Whether extract belongs in the default tool list or only in a later
  Firecrawl/Tavily-style advanced web module.
- Whether browser automation belongs in Whale core or stays as MCP/plugin
  territory.

Evidence bias from references: search and fetch should likely remain separate
at the architecture boundary, but the first visible UI can be simpler if the
provider layer keeps the two phases separable.

## Test And Logging Gate For Implementation

Before shipping runtime search:

- Unit-test provider config parsing, provider priority, fallback behavior, and
  key redaction.
- Unit-test `/search-provider` command parsing and status rendering.
- Add mocked Brave and Jina clients so CI does not depend on live network.
- Add env-gated smoke tests for real Brave and Jina calls.
- Add a DeepSeek Chat Completions request capture test proving that the search
  capability is sent as a function-style tool, not a provider-hosted
  `web_search`.
- Add event/log tests for `search.started`, `search.provider_result`,
  `search.fallback`, and `search.finished`.
- Verify that disabling web search removes all search/fetch tools from the
  model-visible tool list.

## Next Work Item

Write the implementation design for:

- provider config schema;
- `/search-provider` command flow;
- provider abstraction;
- event/log schema;
- DeepSeek function-tool injection;
- minimal mock and live smoke-test matrix.

That design should choose the first visible tool set only after mapping it to
the tests above.

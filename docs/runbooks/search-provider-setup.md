# Search Provider Setup Runbook

## Scope

This runbook covers the local setup flow for Whale's agent web search providers.

Model-visible search tools are generated from configured provider credentials:

- `brave_search`: broad web and news discovery through Brave Search.
- `exa_search`: technical docs, repos, changelogs, and semantic technical search.
- `tavily_search`: agent-native web research and multi-page discovery.
- `github_search`: GitHub repositories, code, issues, commits, and users.
- `stack_exchange_search`: Stack Overflow and Stack Exchange Q&A.
- `jina_search`: Jina Search discovery when a Jina key is configured.
- `web_fetch`: URL reading through Jina readability or direct HTTP fetch.

Unavailable search providers are not exposed to the model. `web_fetch` remains a
single URL-reading tool because choosing a readability/fetch provider is runtime
safety behavior, not an agent task.

Provider credentials are stored by the TUI in the `codex-secrets` local backend, not in `config.toml`.

## Interactive Setup

1. Run `/search-provider`.
2. Pick one provider from the popup.
3. Paste the API key or token into the masked prompt.
4. Whale stores the secret under the default env-style name and persists:
   - `web_search = "live"`
   - `tools.web_search.enabled = true`
   - the provider-specific secret env name, for example `EXA_API_KEY`
5. The TUI runs a provider-specific health check and reports `health_check=ok` or the HTTP failure code.
6. On the next turn, the tool manifest includes the matching provider-specific search tool.

Default secret names:

| Provider | Secret name |
| --- | --- |
| Brave Search | `BRAVE_SEARCH_API_KEY` |
| GitHub Search | `GITHUB_TOKEN` |
| Exa | `EXA_API_KEY` |
| Tavily | `TAVILY_API_KEY` |
| Jina Search | `JINA_API_KEY` |
| Stack Exchange | `STACK_EXCHANGE_KEY` |

Jina has two different roles. `web_fetch` still uses Jina Reader as the default readability/fetch provider and can work without a key. Jina Search uses `s.jina.ai` and is treated as a credentialed search provider through `JINA_API_KEY`; without that key it is skipped instead of being called.

## Debug Commands

The popup is the primary UX. Subcommands exist for scripting and diagnosis:

- `/search-provider status`
- `/search-provider set brave|github|exa|tavily|stack_exchange|jina`
- `/search-provider fallback brave|github|exa|tavily|stack_exchange|jina|off`
- `/search-provider key <provider> [ENV_VAR]`
- `/search-provider test`
- `/search-provider on`
- `/search-provider off`

`/search-provider key <provider> ENV_VAR` changes the lookup name only. It does not store a secret value.

## Runtime Credential Lookup

Provider adapters resolve credentials in this order:

1. Process environment variable named by config.
2. `codex-secrets` global secret with the same name.

This keeps shell-based automation compatible while letting normal users configure keys entirely inside the TUI.

## Routing And Degradation

The runtime keeps search discovery and URL reading separate:

- The tool manifest exposes only search providers with configured credentials.
- `web_fetch` reads selected URLs through Jina Reader or direct HTTP.
- Provider-specific search tools bind directly to their provider; the runtime does not auto-select a different provider for that tool.
- Providers that need a key and do not have one are omitted before the model sees the tool list.
- Logs include provider routing, skipped providers, provider start, success/failure, result count, and latency. They must not include raw queries or secret values.

## Verification

Recommended local checks after changing this area:

```powershell
cd D:\whalecode-alpha\third_party\codex-cli\codex-rs
cargo test -p codex-core web_tools --quiet
cargo test -p codex-protocol web_search --quiet
cargo test -p codex-api chat_completions_maps_hosted_web_search_to_function_tool --quiet
cargo check -p codex-tui --quiet
```

Operational notes:

- Do not log raw key material. `PersistSearchProviderSecret` is intentionally excluded from app-event session logging.
- Keep provider health-check messages mechanical: provider name, status, HTTP code only.
- DeepSeek receives ordinary function tools; hosted OpenAI `web_search` is mapped to local `web_search` for Chat Completions providers.

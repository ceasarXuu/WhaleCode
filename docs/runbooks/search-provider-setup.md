# Search Provider Setup Runbook

## Scope

This runbook covers the local setup flow for Whale's agent web search providers.

Model-visible tools stay stable:

- `web_search`: provider-routed candidate discovery.
- `web_fetch`: URL reading through Jina readability or direct HTTP fetch.

Provider credentials are stored by the TUI in the `codex-secrets` local backend, not in `config.toml`.

## Interactive Setup

1. Run `/search-provider`.
2. Pick one provider from the popup.
3. Paste the API key or token into the masked prompt.
4. Whale stores the secret under the default env-style name and persists:
   - `web_search = "live"`
   - `tools.web_search.enabled = true`
   - `tools.web_search.provider = "<provider>"`
   - `tools.web_search.strategy = "auto"`
5. The TUI runs a provider-specific health check and reports `health_check=ok` or the HTTP failure code.

Default secret names:

| Provider | Secret name |
| --- | --- |
| Brave Search | `BRAVE_SEARCH_API_KEY` |
| GitHub Search | `GITHUB_TOKEN` |
| Exa | `EXA_API_KEY` |
| Tavily | `TAVILY_API_KEY` |
| Stack Exchange | `STACK_EXCHANGE_KEY` |

Jina does not require an API key and remains the fetch/readability default plus emergency search fallback.

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

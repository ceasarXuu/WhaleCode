# Codex CLI Upstream Snapshot

Imported for WhaleCode as a whole-repo upstream substrate.

| Field | Value |
| --- | --- |
| Upstream repository | https://github.com/openai/codex |
| Upstream ref | `refs/heads/main` |
| Upstream commit | `fed0a8f4faa58db3138488cca77628c1d54a2cd8` |
| Commit date | 2026-04-26T19:49:54Z |
| Import date | 2026-04-27 |
| Import method | GitHub codeload tarball |
| Tarball URL | https://codeload.github.com/openai/codex/tar.gz/refs/heads/main |
| Local vendor path | `third_party/codex-cli/` |
| Nested Git metadata | Not imported |
| Local patch count | 1 active Whale overlay |
| License | Apache-2.0, see `LICENSE` |

The vendor tree should stay as close to upstream as possible, but the first
Whale runtime cut intentionally applies a small direct overlay for user-visible
branding, DeepSeek defaults, and text-only provider compatibility. Keep crate
names and broad module shape close to upstream unless a Whale runtime boundary
requires otherwise.

Current local overlay:

- `whale` CLI binary and user-facing Whale naming.
- `WHALE_HOME` / `~/.whale` runtime home isolation.
- DeepSeek provider as the default, using `DEEPSEEK_API_KEY` and
  Chat Completions streaming.
- Unsupported OpenAI-specific entry points hidden or disabled for the Whale
  build: desktop app integration, cloud/app server commands, remote auth,
  native image input, native web search, and upstream self-update.

Every future upstream refresh must update this file and add a matching sync log
under `docs/migration/codex-sync/`.

# Codex CLI Upstream Snapshot

Imported for WhaleCode as a whole-repo upstream substrate.

| Field | Value |
| --- | --- |
| Upstream repository | https://github.com/openai/codex |
| Original import ref | `refs/heads/main` |
| Initial imported baseline commit | `fed0a8f4faa58db3138488cca77628c1d54a2cd8` |
| Commit date | 2026-04-26T19:49:54Z |
| Import date | 2026-04-27 |
| Import method | GitHub codeload tarball |
| Original tarball URL | https://codeload.github.com/openai/codex/tar.gz/refs/heads/main |
| Immutable baseline tarball | https://codeload.github.com/openai/codex/tar.gz/fed0a8f4faa58db3138488cca77628c1d54a2cd8 |
| Local vendor path | `third_party/codex-cli/` |
| Nested Git metadata | Not imported |
| Current vendor state | `rust-v0.147.0` substrate + U3 minimal Whale identity/home/auth/default overlay |
| Overlay inventory | [`overlay-inventory.json`](../../docs/v0.0.5/codex-upstream-sync/overlay-inventory.json) |
| Authoritative backport ledger | [`backport-ledger.json`](../../docs/v0.0.5/codex-upstream-sync/backport-ledger.json) |
| Provenance backlog | [`backport-provenance-backlog.json`](../../docs/v0.0.5/codex-upstream-sync/backport-provenance-backlog.json) |
| Current imported substrate | `rust-v0.147.0` / `be6e8eac029b183056b7e4402879f15d2c85f61b` |
| Candidate qualification | [`upstream-candidate.json`](../../docs/v0.0.5/codex-upstream-sync/upstream-candidate.json), direction-supported with known test risks |
| Upstream delta | [`upstream-delta-inventory.json`](../../docs/v0.0.5/codex-upstream-sync/upstream-delta-inventory.json) |
| Overlay replay ledger | [`overlay-replay-ledger.json`](../../docs/v0.0.5/codex-upstream-sync/overlay-replay-ledger.json) |
| License | Apache-2.0, see `LICENSE` |

This is a derived vendor tree, not an unchanged snapshot of the initial
baseline. The machine-generated inventory is the source of truth for path
counts and classification; this file intentionally does not carry a manually
maintained patch count. The authoritative ledger records confirmed selective
backports. Inferred upstream provenance remains separate until its source and
verification evidence are proved.

Current U4 substrate overlay:

- `whale` primary CLI binary and top-level command identity.
- `WHALE_HOME` / `~/.whale` runtime home isolation.
- Whale-scoped direct and encrypted auth keyring services.
- `remote_plugin` and `plugin_sharing` remain disabled by default while the
  existing local `plugins` capability remains enabled.

DeepSeek Responses API, cache, TaskSpace, remaining user-facing branding,
helper binary names, and release/install behavior are intentionally not part of
U4. They are replayed and verified by their later closed-loop plan units before
release.

Every future upstream refresh must update this file and add a matching sync log
under `docs/migration/codex-sync/`. Validate the current state from the
repository root with:

```text
python3 scripts/codex-upstream/validate_sync_metadata.py
```

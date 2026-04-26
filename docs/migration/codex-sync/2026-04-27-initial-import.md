# Codex Sync: Initial Import

Date: 2026-04-27

## Source

| Field | Value |
| --- | --- |
| Previous Codex commit | None |
| New Codex commit | `fed0a8f4faa58db3138488cca77628c1d54a2cd8` |
| Upstream ref | `refs/heads/main` |
| Upstream repository | https://github.com/openai/codex |
| Import method | GitHub codeload tarball |
| Vendor path | `third_party/codex-cli/` |

## Import Notes

- Imported the whole Codex CLI upstream working tree into
  `third_party/codex-cli/`.
- Did not import nested `.git` metadata.
- Added `third_party/codex-cli/UPSTREAM.md` for pinning and attribution.
- No local Codex vendor patches were applied in this import.

## Adopted Upstream Features

Initial import only. Feature adoption will be decided through the Codex
inventory and Whale bridge plan.

## Disabled Or Rejected Upstream Features

None yet. OpenAI-specific login, cloud task, image, and provider assumptions
will be evaluated during model/provider and branding isolation work.

## Patch Queue

No patch files.

## Tests

Completed after import and before Whale overlay edits:

- `cargo check -p codex-cli --locked` from
  `third_party/codex-cli/codex-rs`.
- `cargo run -p codex-cli --bin codex -- --version` with isolated `HOME` and
  `CODEX_HOME`.
- `cargo run -p codex-cli --bin codex -- --help` with isolated `HOME` and
  `CODEX_HOME`.
- verified no nested `.git` directory was imported under
  `third_party/codex-cli/`.

## Residual Risks

- The import came from a tarball because Git HTTPS to `github.com` was unstable
  while `codeload.github.com` and the GitHub API were reachable.
- Future syncs should prefer `git fetch` when Git HTTPS connectivity is healthy,
  but tarball imports remain acceptable when the upstream commit is recorded.

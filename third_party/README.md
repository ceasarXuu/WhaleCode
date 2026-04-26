# Third-Party Upstream Sources

This directory is reserved for whole-repo upstream substrates.

Planned first import:

- `third_party/codex-cli/`: pinned Codex CLI upstream snapshot.

Do not place Whale-specific product code directly in vendor snapshots. Prefer
bridge or overlay crates. If a vendor edit is unavoidable, record it under
`patches/codex-cli/` and in a sync log.


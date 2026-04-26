# Codex CLI Patch Queue

This directory records unavoidable Whale patches against the vendored Codex CLI
upstream snapshot.

Patch files should include:

- upstream Codex commit they apply to;
- reason bridge-only integration was insufficient;
- files touched;
- tests added or updated;
- whether the patch should be proposed upstream;
- sync-log entry that last replayed the patch.

Prefer an empty patch queue. The primary integration path is
`whalecode-codex-bridge`, not direct vendor modification.


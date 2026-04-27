# After-Change Smoke And Regression Strategy

Date: 2026-04-27

## Status

Updated after the Codex upstream substrate import and the first Whale brand /
DeepSeek overlay. The previous test script and Rust demo workspace are archived
under `archive/deprecated/2026-04-27-rust-demo/`.

## Goal

Every change must keep the repository aligned with the active direction:
Codex CLI whole-repo upstream substrate, Whale bridge, DeepSeek provider, and
multi-agent Primitive overlay. During the transition there is intentionally no
active root Cargo workspace. Rust checks run from
`third_party/codex-cli/codex-rs`.

For the day-to-day command matrix, use
`docs/runbooks/development-workflow.md`. This file remains the higher-level
after-change strategy.

## Current Gate

Run after documentation or repo-structure changes:

```bash
git status --short
git diff --check
rg -n "Codex import lands|Until the new substrate is imported" \
  README.md docs \
  --glob '!docs/testing/2026-04-27-after-change-smoke-regression-strategy.md' \
  || true
```

Expected:

- `git status --short` only shows intentional changes.
- `git diff --check` reports no whitespace errors.
- Search hits, if any, are explicitly historical or superseded references.

Run after Rust provider, CLI, config, or substrate changes:

```bash
cd third_party/codex-cli/codex-rs
cargo check -p codex-cli --locked
cargo test -p codex-api --locked chat_completions
cargo test -p codex-model-provider-info --locked
cargo test -p codex-core --locked defaults_to_deepseek_flash_provider
cargo test -p codex-core --locked responses_websocket_features_do_not_change_wire_api
cargo test -p codex-core --locked config_schema_matches_fixture
cargo run --quiet -p codex-cli --bin whale -- --version
```

## Future Gates

Keep rebuilding the repo-owned test script around these stages:

| Gate | Purpose |
| --- | --- |
| Codex import gate | Verify upstream URL, commit, license, and clean import metadata. |
| Patch queue gate | Reapply `patches/codex-cli/` deterministically after upstream updates. |
| Bridge unit gate | Keep Whale interfaces stable while Codex internals evolve. |
| Substrate behavior gate | Preserve Codex-grade permission, sandbox, apply patch, exec, session, context, MCP, and skills behavior. |
| DeepSeek provider gate | Verify auth failure, SSE parsing, reasoning deltas, tool-call deltas, and usage aggregation. |
| Overlay replay gate | Verify Primitive, Viewer, Create/Debug, WorkUnit, and PatchArtifact events replay correctly. |
| Upstream sync gate | Produce a sync log for every Codex version advance, including adopted features, rejected features, patch conflicts, and test results. |

## Done Criteria For Migration Steps

A migration step is not complete until it has:

- updated or added the relevant plan/ADR/sync log;
- emitted structured logs or events for new runtime behavior;
- included a smoke or regression test for the adopted Codex behavior;
- documented any upstream patch and whether it should be upstreamed;
- left no uncommitted generated artifacts outside intentional archive/vendor
  paths.

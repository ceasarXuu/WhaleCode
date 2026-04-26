# After-Change Smoke And Regression Strategy

Date: 2026-04-27

## Status

Updated for the Codex upstream substrate migration. The previous test script and
Rust demo workspace are archived under
`archive/deprecated/2026-04-27-rust-demo/`.

## Goal

Every change must keep the repository aligned with the active direction:
Codex CLI whole-repo upstream substrate, Whale bridge, DeepSeek provider, and
multi-agent Primitive overlay. During the transition there is intentionally no
active root Cargo workspace; tests focus on documentation consistency, archive
integrity, and migration readiness until Codex import lands.

## Current Gate

Run after documentation or repo-structure changes:

```bash
git status --short
rg -n "deprecated runtime marker patterns" README.md docs || true
find archive/deprecated/2026-04-27-rust-demo -maxdepth 2 -type f | sort | head
```

Expected:

- `git status --short` only shows intentional changes.
- Search hits, if any, are explicitly historical, superseded, or archived-demo
  references.
- The demo archive exists and contains its README plus the previous workspace
  metadata.

## Future Gates

After Codex import, rebuild the repo-owned test script around these stages:

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

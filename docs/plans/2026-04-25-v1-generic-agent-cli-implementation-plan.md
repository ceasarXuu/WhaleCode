# V1 Generic Agent CLI Implementation Plan

Date: 2026-04-25

## Status

Superseded on 2026-04-27 by
`docs/plans/2026-04-27-codex-cli-upstream-substrate-migration-plan.md`.

The original V1 plan described a from-scratch Rust workspace with
`whalecode-*` crates, a local bootstrap agent, a DeepSeek live-loop demo,
patch-safe edit experiments, and JSONL session replay. That work has been
archived under `archive/deprecated/2026-04-27-rust-demo/`.

## Why Superseded

The original plan was useful for learning the product surfaces, but it rebuilt
too many hard coding-agent substrate details manually:

- permission and approval policy;
- sandbox and command execution;
- shell command risk handling;
- patch application and edit safety;
- session/thread/replay;
- context compaction;
- MCP and skills;
- TUI/runtime interaction details.

Codex CLI already contains mature implementations of those areas and continues
to evolve quickly. Rebuilding them from scratch would lose behavior details and
make WhaleCode slower to reach a production-grade baseline.

## Replacement Direction

The active V1 direction is:

```text
Codex CLI whole-repo upstream substrate
  -> Whale Codex bridge
  -> DeepSeek V4 provider
  -> Whale Primitive / multi-agent / Viewer overlay
```

The new plan keeps the original product target but changes the implementation
base:

- Codex supplies mature single-agent coding substrate behavior.
- Whale bridge translates Codex runtime events and decisions into Whale
  protocol.
- Whale overlay adds DeepSeek, multi-agent cohorts, Create/Debug gates,
  Primitive Modules, Viewer, and future Web Viewer behavior.
- Upstream Codex updates remain absorbable through pinned imports, patch queue,
  bridge compatibility tests, and sync logs.

## Historical Artifacts

Archived demo contents:

- root Cargo workspace metadata;
- `crates/whalecode-*` demo crates;
- demo CLI tests;
- previous after-change smoke script.

The archive is recoverable evidence only. Do not continue product work there.
Useful ideas must be ported deliberately into the new Codex bridge or Whale
overlay layer.

## Current Acceptance Line

V1 is acceptable when:

- Whale runs on a Codex-derived substrate;
- DeepSeek is the default provider path;
- Codex permission, sandbox, patch, exec, session, context, MCP, and skills
  behavior are available through Whale bridge interfaces;
- Whale Primitive and multi-agent layers add gates and replayable events without
  requiring unmanaged edits inside the Codex vendor tree;
- future Codex upstream versions can be imported, diffed, patched, tested, and
  documented through the sync workflow.

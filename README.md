# WhaleCode

DeepSeek-first terminal AI coding agent built on a Codex CLI upstream substrate.

## Current Direction

WhaleCode is being repositioned away from a from-scratch Rust demo runtime. The
active plan is to import Codex CLI as a whole-repo upstream substrate, keep that
snapshot syncable with future Codex releases, and build Whale-specific behavior
through bridge and overlay layers.

Active architecture:

```text
Codex CLI upstream substrate
  -> Whale Codex bridge
  -> DeepSeek V4 provider
  -> Multi-agent / Primitive / Viewer / Create-Debug overlay
```

The previous `whalecode-*` Rust demo crates are archived at
`archive/deprecated/2026-04-27-rust-demo/`. They are retained for recoverability
and migration reference only; do not continue product work there.

## Product Goal

Build an open-source terminal AI coding agent centered on DeepSeek V4:

- Rust-first local execution core inherited from Codex-grade substrate work.
- TypeScript Web Viewer for real-time event and agent visualization later.
- DeepSeek V4 Flash/Pro routing with reasoning-content streaming.
- Multi-Agent First coordination through cohorts, WorkUnits, Patch League, and
  evidence-weighted decisions.
- Create and Debug as runtime primitives rather than prompt-only workflows.
- Primitive Modules for scaffolding-first Create, evidence-chain Debug,
  reference-driven design, independent Viewer, and skill evolution.

## Repository Strategy

Planned active layout:

```text
third_party/codex-cli/          # pinned Codex CLI upstream snapshot
patches/codex-cli/              # local patch queue for unavoidable vendor edits
crates/whalecode-codex-bridge/  # Codex-to-Whale adapter layer
crates/whalecode-*/             # Whale protocol, provider, swarm, primitive overlay
apps/viewer/                    # future read-only Web Viewer
docs/migration/codex-sync/      # upstream sync logs
archive/deprecated/             # inactive historical implementations
```

Codex is not just a reference. It is the upstream substrate WhaleCode will
import, pin, test, diff, and periodically upgrade. Whale-specific changes should
land in the bridge or overlay layer first. Direct edits inside
`third_party/codex-cli/` require a patch-queue entry and sync log.

## Key Documents

- Migration plan:
  `docs/plans/2026-04-27-codex-cli-upstream-substrate-migration-plan.md`
- ADR:
  `docs/adr/2026-04-27-codex-cli-upstream-substrate.md`
- Original system architecture, now aligned to substrate direction:
  `docs/plans/2026-04-24-system-architecture.md`
- Primitive architecture:
  `docs/plans/2026-04-25-differentiated-primitives-architecture.md`
- Multi-agent architecture:
  `docs/plans/2026-04-25-multi-agent-collaboration-architecture.md`

## Current Development State

There is intentionally no active root Cargo workspace after the demo archive.
The next implementation milestone is Codex upstream import plus inventory and
bridge skeleton, not continued extension of the archived demo.

Until the new substrate is imported, verification is documentation and repo
hygiene focused:

```bash
git status --short
rg -n "deprecated runtime marker patterns" README.md docs || true
```

After Codex import, repo-owned gates must be rebuilt around substrate import,
bridge compatibility, DeepSeek provider behavior, and Primitive overlay replay.

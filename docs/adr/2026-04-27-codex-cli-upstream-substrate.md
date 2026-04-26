# ADR: Codex CLI Upstream Substrate

Date: 2026-04-27

## Status

Accepted. Supersedes the "Fork Codex CLI" rejection in
`docs/adr/2026-04-25-rust-first-core-runtime.md`.

## Context

WhaleCode originally started building a Rust runtime from scratch while using
Codex CLI as a design reference. That approach loses important implementation
details in the exact areas where coding agents are hardest to get right:
permission policy, sandboxing, shell execution, patch application, session
history, context compaction, MCP, skills, and TUI interaction.

Codex CLI is still moving quickly. A one-time source copy would create a deep
fork that cannot absorb future upstream improvements. WhaleCode therefore needs
both of these properties:

- Preserve Codex implementation detail by importing the whole upstream tree.
- Keep future Codex upgrades practical by isolating Whale changes outside the
  upstream snapshot whenever possible.

## Decision

WhaleCode will adopt Codex CLI as a whole-repo upstream substrate. The active
architecture becomes:

```text
WhaleCode
  -> pinned Codex CLI upstream snapshot
  -> Whale Codex bridge layer
  -> Whale DeepSeek, multi-agent, Primitive, Viewer, Create/Debug overlay
```

Codex is not only a reference document source. It is the runtime substrate that
WhaleCode will import, build against, diff against, and periodically upgrade.

## Consequences

- The first Rust demo runtime is deprecated and archived under
  `archive/deprecated/2026-04-27-rust-demo/`.
- New runtime work must start from Codex import, inventory, bridge interfaces,
  and targeted replacement of Whale demo surfaces.
- `third_party/codex-cli/` should remain as close to upstream as possible.
- Whale-specific behavior belongs in `crates/whalecode-codex-bridge/` and Whale
  overlay crates, not scattered through the vendored Codex tree.
- Direct edits to Codex upstream files require a patch-queue entry, sync note,
  reason, and regression coverage.
- Whale's product identity remains DeepSeek-first, multi-agent-first, and
  Primitive-driven. Codex supplies the mature single-agent coding substrate.

## Alternatives Considered

### Keep building from scratch with Codex as reference

Rejected. It preserves architecture intent but loses too many production-grade
details and recreates mature infrastructure slowly.

### Hard fork Codex CLI directly into the repository root

Rejected. It would preserve detail but make Whale's DeepSeek and multi-agent
product boundaries harder to maintain, and future upstream merges would become
fragile.

### Submodule-only Codex dependency

Rejected as the default path because it worsens open-source checkout and release
ergonomics. A subtree or vendored snapshot with upstream metadata is preferred.

## Follow-ups

- Create `docs/plans/2026-04-27-codex-cli-upstream-substrate-migration-plan.md`.
- Add `third_party/codex-cli/UPSTREAM.md` during the import phase.
- Add `patches/codex-cli/` only when the first unavoidable upstream patch exists.
- Build a sync log for every future Codex upgrade.


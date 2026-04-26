# ADR: Rust-first Core Runtime

Date: 2026-04-25

## Status

Superseded by `docs/adr/2026-04-27-codex-cli-upstream-substrate.md`.

This ADR still records why WhaleCode needs a Rust-first local execution core,
but its "Fork Codex CLI" rejection and Phase 0 from-scratch workspace
consequence are no longer active.

## Context

WhaleCode was originally described as a TypeScript / Node / Bun terminal AI coding agent. That choice was made before the system architecture clarified the project’s real center of gravity: local execution safety, multi-agent orchestration, deterministic workflow phases, tool permissions, patch artifacts, long-running sessions, replay, and TUI/Web Viewer event consumption.

The project still benefits from TypeScript for Web Viewer and plugin-facing examples, but the core runtime has stronger requirements than a typical scripting CLI.

## Decision

WhaleCode will use:

- Rust for CLI, TUI, core runtime, Agent Supervisor, Message Bus, Tool Runtime, Permission Engine, Patch Engine, Session Store, Context Manager, DeepSeek adapter, and MCP host.
- TypeScript + React + Vite for the Web Viewer.
- Protocol boundaries for MCP, Skills, plugin processes, and Viewer event consumption.

## Alternatives Considered

### TypeScript / Node / Bun core

Rejected as the long-term core runtime because WhaleCode needs reliable subprocess control, shell safety, write locks, patch application, session replay, and state-machine-heavy orchestration. TypeScript remains appropriate for Web Viewer and plugin examples.

### Go core

Viable fallback. Go has strong single-binary distribution and concurrency ergonomics, and OpenCode proves the approach can work. Rust is preferred because its type system better fits long-lived permission, ownership, and patch-contract boundaries.

### Fork Codex CLI

Original 2026-04-25 decision: rejected as product strategy. Codex CLI should be
a reference implementation for Rust architecture patterns, not the WhaleCode
product base.

2026-04-27 update: this rejection is superseded. WhaleCode will import Codex CLI
as a whole-repo upstream substrate, while keeping Whale-specific DeepSeek,
multi-agent, Create/Debug, Primitive, and Viewer behavior in bridge and overlay
layers.

## Consequences

- Phase 0 from-scratch Rust scaffolding is deprecated. The archived demo is in
  `archive/deprecated/2026-04-27-rust-demo/`; active implementation restarts
  from Codex upstream import and bridge construction.
- Existing architecture docs must replace Node/Bun MVP assumptions with Rust-first assumptions.
- DeepSeek support should be implemented as a Rust HTTP/SSE adapter with capability probing.
- Web Viewer consumes versioned events from Rust core and must remain read-only by default.
- Skills and MCP tools must be mediated by the Rust Permission Engine.
- Mature coding-agent infrastructure should be adopted from the Codex upstream
  substrate through Whale bridge interfaces. Permission, sandbox, unified exec,
  patch, session, context, MCP/skills, and observability are no longer
  from-scratch rebuild targets.

## References

- Detailed plan: `docs/plans/2026-04-25-rust-first-technology-architecture.md`
- Superseding substrate ADR: `docs/adr/2026-04-27-codex-cli-upstream-substrate.md`
- Migration plan: `docs/plans/2026-04-27-codex-cli-upstream-substrate-migration-plan.md`
- System architecture: `docs/plans/2026-04-24-system-architecture.md`
- Reference audit: `docs/plans/2026-04-25-codex-first-reference-audit.md`

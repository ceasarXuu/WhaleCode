# ADR: Rust-first Core Runtime

Date: 2026-04-25

## Status

Accepted

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

Rejected as product strategy. Codex CLI should be a reference implementation for Rust architecture patterns, not the WhaleCode product base. WhaleCode’s core primitives are Multi-Agent First, Create/Debug workflows, DeepSeek-specific routing, and Viewer-driven critique.

## Consequences

- Phase 0 must scaffold a Rust workspace, not a pnpm-first TypeScript monorepo.
- Existing architecture docs must replace Node/Bun MVP assumptions with Rust-first assumptions.
- DeepSeek support should be implemented as a Rust HTTP/SSE adapter with capability probing.
- Web Viewer consumes versioned events from Rust core and must remain read-only by default.
- Skills and MCP tools must be mediated by the Rust Permission Engine.
- Mature coding-agent infrastructure must pass the Codex-first Reference Audit Gate before implementation. Permission, sandbox, unified exec, patch, session, context, MCP/skills, and observability should start from Codex CLI behavior, then adapt only where WhaleCode's DeepSeek, multi-agent, Create/Debug, or Viewer requirements require it.

## References

- Detailed plan: `docs/plans/2026-04-25-rust-first-technology-architecture.md`
- System architecture: `docs/plans/2026-04-24-system-architecture.md`
- Reference audit: `docs/plans/2026-04-25-codex-first-reference-audit.md`

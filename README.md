# WhaleCode

WhaleCode is an open-source terminal AI coding agent built for developers who
want a Claude Code / Codex CLI style workflow with DeepSeek V4 at the center.

It is designed to work inside real repositories: read code, plan changes, run
commands, apply patches, verify results, and keep a replayable record of what
happened. The product direction is multi-agent first, coding-native, and
optimized for DeepSeek V4 Flash and Pro.

## Why WhaleCode

Modern coding agents are becoming the main interface between developers and
large codebases. WhaleCode focuses on three product goals:

- Make terminal-based AI coding practical for daily engineering work.
- Use multiple specialized agents when one linear assistant is not enough.
- Turn create and debug workflows into explicit runtime primitives, not just
  prompt conventions.

WhaleCode starts from the mature Codex CLI substrate for local execution,
permissions, tools, patches, sessions, context handling, logs, MCP, and skills.
Whale-specific behavior is added through bridge and overlay layers so the
project can keep upstream compatibility while evolving its own product surface.

## Core Capabilities

| Capability | What it means for users |
| --- | --- |
| DeepSeek V4 first | Routes work across `deepseek-v4-flash` and `deepseek-v4-pro`, with support for long context, long output, and reasoning-content streaming. |
| Terminal-native coding | Operates directly in your repo, using shell commands, patches, tests, logs, and local files as first-class workflow objects. |
| Multi-agent collaboration | Coordinates role-based agents such as scouts, analysts, implementers, reviewers, judges, and verifiers to reduce single-agent blind spots. |
| Create primitive | Treats new feature construction as a structured workflow with scaffolding, constraints, logging, tests, and verification gates. |
| Debug primitive | Builds evidence chains from goals, hypotheses, logs, runtime behavior, and patches so diagnosis converges on root cause. |
| Primitive modules | Keeps differentiated workflows pluggable so capabilities can be measured, replayed, improved, or removed without rewriting the core. |
| Web Viewer direction | Plans a read-only TypeScript viewer for agent networks, DAG progress, tool activity, and session statistics. |

## Product Shape

WhaleCode is being built as a practical CLI first:

```text
Developer terminal
  -> WhaleCode CLI
  -> Codex-derived execution substrate
  -> Whale bridge and primitive modules
  -> DeepSeek V4 provider
  -> optional Web Viewer
```

The near-term V1 goal is to deliver a mainstream coding-agent CLI foundation:

- safe command execution and patch application;
- repository-aware context management;
- reliable session logs and replayable state;
- model/provider configuration for DeepSeek V4;
- create/debug workflows that can be tested as product behavior;
- extension points for skills, tools, MCP servers, and primitive modules.

## Current Status

WhaleCode is under active development. The repository currently vendors Codex
CLI under `third_party/codex-cli/` and layers Whale-specific work around that
upstream substrate.

The active Rust workspace lives here:

```powershell
cd third_party/codex-cli/codex-rs
cargo check -p codex-cli --locked
cargo run --quiet -p codex-cli --bin whale -- --version
```

On Windows, local Whale builds should be installed through:

```powershell
scripts/install-whale-local.ps1
```

The installer places `whale.exe` under `%USERPROFILE%\.whale\bin` and keeps it
separate from official Codex locations such as `%APPDATA%\npm` and WindowsApps.

## Repository Map

```text
third_party/codex-cli/          Codex CLI upstream substrate snapshot
patches/codex-cli/              local patch queue for unavoidable vendor edits
docs/                           product, architecture, ADR, and runbook docs
scripts/                        local development and installation scripts
archive/deprecated/             recoverable historical implementations
```

Whale-specific product work should prefer bridge, overlay, module, or script
layers before changing the vendored upstream directly. When upstream files must
change, the work should be documented so future Codex syncs remain manageable.

## Documentation

- [Development workflow](docs/runbooks/development-workflow.md)
- [Windows development restore](docs/runbooks/windows-development-restore.md)
- [Cross-system restore](docs/runbooks/cross-system-restore.md)
- [Codex upstream substrate ADR](docs/adr/2026-04-27-codex-cli-upstream-substrate.md)
- [System architecture](docs/plans/2026-04-24-system-architecture.md)
- [Differentiated primitives architecture](docs/plans/2026-04-25-differentiated-primitives-architecture.md)
- [Multi-agent collaboration architecture](docs/plans/2026-04-25-multi-agent-collaboration-architecture.md)

## Project Principles

- Open source by default.
- Coding behavior must be generated through the agent/model path, not hardcoded
  keyword replies or fake intelligence.
- Differentiated features must become artifact schemas, phase gates, session
  events, and replayable state.
- Logging, testing, and constraints are part of product quality, not cleanup
  tasks after implementation.

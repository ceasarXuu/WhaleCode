# V1 Generic Agent CLI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build WhaleCode V1 as a mainstream-capable coding agent CLI substrate before enabling differentiated Primitive Modules.

**Architecture:** Start with a Rust workspace that provides a generic agent runtime: model streaming, agent loop, tool runtime, permission/sandbox decisions, patch/workspace safety, JSONL session replay, context management, and a pluggable primitive host. Differentiated abilities such as evidence-chain Debug, scaffolding-first Create, reference-driven design, Viewer, and skill evolution register through `PrimitiveModule` rather than coupling to the agent loop.

**Tech Stack:** Rust stable, Tokio, clap, serde/serde_json, tracing, JSONL session logs, reqwest/SSE for DeepSeek adapter, Codex-first reference audit for mature coding-agent infrastructure.

---

## Implementation Rules

- V1 must work as a generic coding agent CLI even when all non-basic Primitive Modules are disabled.
- All mature infrastructure follows `docs/plans/2026-04-25-codex-first-reference-audit.md`.
- All differentiated abilities follow `docs/plans/2026-04-25-differentiated-primitives-architecture.md`.
- Every schema-bearing crate must support event-sourced replay.
- Every tool write path must go through permission and patch/workspace safety.
- Keep the first implementation vertical and testable; do not implement full swarm before the single-agent substrate works.

## V1 Acceptance Line

WhaleCode V1 is acceptable when a user can run the CLI in a real repository and complete this loop:

```text
natural-language task
  -> model response
  -> read/search relevant files
  -> edit/write through patch-safe path
  -> show diff metadata
  -> run a controlled verification command
  -> persist JSONL session
  -> replay the transcript and tool/patch events
```

## Current Bootstrap Slice

Implemented first: `whale` / `whale run` starts a replayable local bootstrap agent turn, persists JSONL session events, routes read-only tools through a permission decision, and reports a final assistant transcript. This is intentionally not yet a live DeepSeek agent and intentionally does not mutate files.

```yaml
reference_source:
  codex:
    - tmp/whalecode-refs/codex-cli/codex-rs/core/src/thread_manager.rs
    - tmp/whalecode-refs/codex-cli/codex-rs/core/src/tools/context.rs
    - tmp/whalecode-refs/codex-cli/codex-rs/core/src/exec_policy.rs
  opencode:
    - tmp/whalecode-refs/opencode/internal/llm/tools/edit.go
borrowed_behavior:
  - separate CLI command surface, agent loop, tool runtime, permission decision, and append-only session log
  - record replayable session events before building richer UI or DB indexes
  - keep tool outputs structured and truncated before model-facing reuse
  - use gitignore-aware file walking and skip local agent config directories
  - reject mutating tools until permission and patch safety are both in the execution path
whalecode_delta:
  - expose the product binary as whale
  - use a bootstrap-local model runtime until the DeepSeek SSE adapter is verified with fixtures
rejected_behavior:
  - no unsafe shell/write shortcut in the first CLI loop
  - no DB-first session store before JSONL replay is stable
license_boundary:
  - design-only reference; no copied reference-project source code
acceptance_tests:
  - whale run creates a session file
  - JSONL replay reconstructs user and assistant transcript entries
  - read-only tools are allowed in Analyze phase
  - file listing omits gitignored files and local agent config directories
  - write/shell operations are rejected or require approval before execution
```

## Milestone 0: Workspace Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: `crates/whalecode-protocol/Cargo.toml`
- Create: `crates/whalecode-protocol/src/lib.rs`
- Create: `crates/whalecode-core/Cargo.toml`
- Create: `crates/whalecode-core/src/lib.rs`
- Create: `crates/whalecode-model/Cargo.toml`
- Create: `crates/whalecode-model/src/lib.rs`
- Create: `crates/whalecode-tools/Cargo.toml`
- Create: `crates/whalecode-tools/src/lib.rs`
- Create: `crates/whalecode-permission/Cargo.toml`
- Create: `crates/whalecode-permission/src/lib.rs`
- Create: `crates/whalecode-patch/Cargo.toml`
- Create: `crates/whalecode-patch/src/lib.rs`
- Create: `crates/whalecode-session/Cargo.toml`
- Create: `crates/whalecode-session/src/lib.rs`
- Create: `crates/whalecode-context/Cargo.toml`
- Create: `crates/whalecode-context/src/lib.rs`
- Create: `crates/whalecode-primitives/Cargo.toml`
- Create: `crates/whalecode-primitives/src/lib.rs`
- Create: `crates/whalecode-cli/Cargo.toml`
- Create: `crates/whalecode-cli/src/main.rs`
- Modify: `.gitignore`
- Modify: `README.md`

**Step 1: Create workspace manifests**

Add a root workspace with all Phase 1 crates and shared dependencies.

**Step 2: Add empty-but-compilable crate entrypoints**

Each library crate exports a small public type or trait that reflects its boundary.

**Step 3: Add minimal CLI**

`whalecode-cli` should expose `whalecode --version` and `whalecode status`.

**Step 4: Verify**

Run:

```bash
cargo fmt --check
cargo test --workspace
```

Expected: all crates compile and tests pass.

**Step 5: Commit**

```bash
git add .
git commit -m "Scaffold Rust workspace"
git push
```

## Milestone 1: Protocol And Event Schema

**Files:**
- Modify: `crates/whalecode-protocol/src/lib.rs`
- Test: `crates/whalecode-protocol/tests/event_schema.rs`

**Step 1: Define identifiers**

Add typed IDs for `SessionId`, `TraceId`, `TurnId`, `AgentId`, `ToolCallId`, `ArtifactId`, `PrimitiveId`, and `WorkUnitId`.

**Step 2: Define event envelope**

Create:

```rust
pub struct EventEnvelope<T> {
    pub schema_version: u32,
    pub session_id: SessionId,
    pub trace_id: TraceId,
    pub turn_id: Option<TurnId>,
    pub sequence: u64,
    pub occurred_at: DateTime<Utc>,
    pub payload: T,
    pub redaction: RedactionSummary,
}
```

**Step 3: Define V1 event payloads**

Include session, model, tool, permission, patch, primitive, and replay-oriented events.

**Step 4: Add serialization tests**

Test JSON roundtrip for each event family.

**Step 5: Verify**

Run:

```bash
cargo test -p whalecode-protocol
```

Expected: event schema tests pass.

## Milestone 2: JSONL Session Store

**Files:**
- Modify: `crates/whalecode-session/src/lib.rs`
- Test: `crates/whalecode-session/tests/jsonl_store.rs`

**Step 1: Write failing append/replay test**

Create a temp session, append two events, replay them in sequence.

**Step 2: Implement append-only JSONL writer**

Guarantee monotonic `sequence` and write one JSON object per line.

**Step 3: Implement replay snapshot skeleton**

Replay should reconstruct at least transcript, tool events, patch events, and primitive enable/disable state.

**Step 4: Verify**

Run:

```bash
cargo test -p whalecode-session
```

Expected: append/replay tests pass and malformed JSONL fails clearly.

## Milestone 3: Permission Engine

**Files:**
- Modify: `crates/whalecode-permission/src/lib.rs`
- Test: `crates/whalecode-permission/tests/decision.rs`

**Step 1: Port Codex-first semantics**

Implement deny-before-allow, filesystem/network split, approval policy, and phase/role/workunit context.

**Step 2: Add tests**

Cover:

- deny outranks write allow.
- approval policy `Never` rejects ask.
- read-only phase rejects write shell.
- session grant can be scoped to a single agent or work unit.

**Step 3: Verify**

Run:

```bash
cargo test -p whalecode-permission
```

Expected: permission precedence tests pass.

## Milestone 4: Tool Runtime

**Files:**
- Modify: `crates/whalecode-tools/src/lib.rs`
- Test: `crates/whalecode-tools/tests/read_search.rs`
- Test: `crates/whalecode-tools/tests/output_truncation.rs`

**Step 1: Implement read/search tools**

Add deterministic local `read_file`, `list_files`, and `search_text` handlers.

**Step 2: Implement tool result envelope**

Every result includes stdout/content, metadata, truncation info, and redaction summary.

**Step 3: Add output truncation tests**

Cover head/tail preservation and max token/byte limits.

**Step 4: Verify**

Run:

```bash
cargo test -p whalecode-tools
```

Expected: read/search and truncation tests pass.

## Milestone 5: Patch And Workspace Safety

**Files:**
- Modify: `crates/whalecode-patch/src/lib.rs`
- Test: `crates/whalecode-patch/tests/patch_artifact.rs`

**Step 1: Define `PatchArtifact`**

Include base commit, touched files, ownership claims, diff, tests run, and risk summary.

**Step 2: Add read-before-write contract**

A write request must reference the file version it read.

**Step 3: Add dry-run apply result**

Return applied/conflict/rejected without mutating the shared workspace in tests.

**Step 4: Verify**

Run:

```bash
cargo test -p whalecode-patch
```

Expected: stale reads and ownership overlap are rejected.

## Milestone 6: DeepSeek Model Adapter Skeleton

**Files:**
- Modify: `crates/whalecode-model/src/lib.rs`
- Test: `crates/whalecode-model/tests/mock_sse.rs`

**Step 1: Define model capability probe output**

Include context window, max output, thinking support, tool-call support, pricing source, and observed timestamp.

**Step 2: Define streaming events**

Represent assistant text delta, reasoning delta, tool call delta, final usage, error, and finish.

**Step 3: Add mock SSE fixtures**

Test thinking + tool-call sub-turn without calling the live provider.

**Step 4: Verify**

Run:

```bash
cargo test -p whalecode-model
```

Expected: mock SSE parser preserves reasoning/tool-call ordering.

## Milestone 7: Agent Loop And CLI Vertical Slice

**Files:**
- Modify: `crates/whalecode-core/src/lib.rs`
- Modify: `crates/whalecode-cli/src/main.rs`
- Test: `crates/whalecode-core/tests/agent_loop.rs`

**Step 1: Implement agent loop skeleton**

The loop accepts a user task, model events, tool requests, tool results, and final output.

**Step 2: Wire CLI command**

Add:

```bash
whalecode run "summarize this repo"
whalecode status
```

**Step 3: Persist session**

Each CLI run writes JSONL events through `whalecode-session`.

**Step 4: Verify**

Run:

```bash
cargo test --workspace
cargo run -p whalecode-cli -- status
```

Expected: tests pass and status prints workspace/runtime information.

## Milestone 8: Primitive Host Skeleton

**Files:**
- Modify: `crates/whalecode-primitives/src/lib.rs`
- Test: `crates/whalecode-primitives/tests/registry.rs`

**Step 1: Define `PrimitiveModule`**

Use the contract from `docs/plans/2026-04-25-differentiated-primitives-architecture.md`.

**Step 2: Implement registry**

Support register, enable, disable, list, and resolve gates/hooks/reducers.

**Step 3: Add tests**

Cover:

- disabled module contributes no gates.
- historical events still replay after module disable.
- dependency/conflict validation.

**Step 4: Verify**

Run:

```bash
cargo test -p whalecode-primitives
```

Expected: registry tests pass.

## Milestone 9: V1 End-To-End Fixture

**Files:**
- Create: `fixtures/repos/basic-rust/`
- Create: `tests/e2e/v1_cli.rs`

**Step 1: Create fixture repo**

Add a tiny Rust fixture with one failing or missing behavior.

**Step 2: Add CLI e2e test**

The test runs the CLI in the fixture, reads/searches/edits a file through the runtime, and verifies JSONL replay.

**Step 3: Verify**

Run:

```bash
cargo test --workspace
```

Expected: V1 generic CLI e2e passes without enabling differentiated primitives.

## Done Criteria

V1 substrate is done when:

- `cargo fmt --check` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo test --workspace` passes.
- CLI can run `status` and one non-interactive task.
- JSONL replay reconstructs the visible transcript and tool/patch events.
- Non-basic Primitive Modules can be disabled without breaking the generic CLI.
- README explains local build, run, and test commands.

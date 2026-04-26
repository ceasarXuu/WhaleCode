# After-Change Smoke And Regression Strategy

Date: 2026-04-27

## Goal

Every code change must prove that WhaleCode still works as a real terminal AI
coding agent substrate before a human spends time on manual testing. The default
gate is not allowed to pass by only compiling or snapshotting current behavior.
Tests must encode the expected product contract first, then fail when runtime
behavior diverges.

## Core Functional Surface

WhaleCode V1 currently has these critical runtime surfaces:

| Surface | Product expectation | Primary crates |
| --- | --- | --- |
| CLI command surface | `whale status`, `whale run`, `whale logs`, `whale model-smoke`, and interactive mode expose mechanical status and route natural language through the agent path. | `whalecode-cli` |
| DeepSeek model adapter | Provider config, SSE streaming, reasoning deltas, text deltas, tool-call deltas, token usage, and clear missing-key failures are deterministic at the adapter boundary. | `whalecode-model` |
| Agent loop | A natural-language turn records session lifecycle, transcript, model events, permissions, tools, patch events, usage, and final status. | `whalecode-core` |
| Tool runtime | Read/search tools are workspace-rooted and gitignore-aware; `run_command` is argument-array based and permission gated. | `whalecode-tools`, `whalecode-core` |
| Patch safety | `edit_file` requires explicit write permission, exact replacement, fresh snapshot identity, path boundary checks, and replayable patch metadata. | `whalecode-patch`, `whalecode-core` |
| Permission policy | Mutating writes and commands are rejected unless explicitly enabled; rejected attempts are recorded instead of silently skipped. | `whalecode-permission` |
| Session replay | JSONL logs are append-only, monotonic, replayable, and inspectable with `whale logs`. | `whalecode-session`, `whalecode-cli` |
| Primitive contracts | Differentiated Create/Debug primitives must remain artifact/schema/event driven, not prompt-only behavior. | `whalecode-protocol`, `whalecode-primitives` |

## Default Command

Run this after every code change:

```bash
scripts/test-after-change.sh
```

The script runs formatting, all workspace tests, clippy, and runtime CLI smoke
checks. It writes temporary runtime artifacts under `${TMPDIR:-/tmp}/whalecode-test-runs/`
so the session logs can be inspected after a failure without deleting evidence.

For a faster local loop while editing one small change:

```bash
scripts/test-after-change.sh smoke
```

Before release or when provider behavior changes:

```bash
DEEPSEEK_API_KEY=... scripts/test-after-change.sh live-provider
```

## Tier 0: Static Gate

Purpose: reject broken code before runtime tests spend time starting binaries.

Command:

```bash
cargo fmt --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets -- -D warnings
```

Expected result:

- All crates compile with the locked dependency graph.
- Unit and integration tests pass.
- Clippy has zero warnings because warnings often hide error-path drift in CLI
  tools.

This tier is necessary but never sufficient. Passing it does not prove that the
agent can run, write session logs, reject unsafe operations, or replay a turn.

## Tier 1: Deterministic Runtime Smoke

Purpose: prove the installed command path works without live provider cost.

The default script verifies:

| Command | Expected result |
| --- | --- |
| `cargo run -p whalecode-cli --bin whale -- status` | Exits 0 and reports `runtime: live_deepseek_tool_loop`, JSONL sessions, and smoke command hints. |
| `whale run --bootstrap "inspect fixture"` | Exits 0, reads a fixture repo, prints assistant output and token lines, and writes a non-empty session JSONL file. |
| `whale logs --session <bootstrap-session>` | Exits 0 and prints the turn, tool output, and assistant transcript from the session file. |
| `whale run "hi"` without a key | Exits non-zero, prints startup status and exactly one session path, writes a failure session, and reports that a DeepSeek API key is required. |

This tier catches the failures static tests miss: binary wiring, process cwd,
session file creation, stdout/stderr contract drift, and runtime failure-path
logging.

## Tier 2: Mocked Live Agent Regression

Purpose: prove the real live loop can consume provider-like streams and execute
tools without calling the network.

Current coverage already includes a local TCP mock in
`crates/whalecode-cli/tests/live_status.rs`. Required expectations:

- streamed `edit_file` tool-call deltas are grouped into one tool call;
- `--allow-write` permits the write only through patch safety;
- stdout exposes permission, patch, changed-file, usage, and duration status;
- the fixture file actually changes from the expected old text to the expected
  new text;
- cached token accounting is printed from provider usage metadata.

Any change to model streaming, tool execution, permission events, patch status,
or CLI run display must add or update a mocked live-loop regression before the
change is considered done.

## Tier 3: Provider Smoke

Purpose: detect real DeepSeek auth, endpoint, SSE, and aggregation drift without
letting a live model edit the workspace.

Command:

```bash
DEEPSEEK_API_KEY=... scripts/test-after-change.sh live-provider
```

Expected result:

- `whale model-smoke --model deepseek-v4-flash "say hello"` exits 0.
- The command streams and aggregates text.
- No tool execution or file write is possible in this mode.

This tier is opt-in for normal development because it uses a live credential and
network, but it is required before changes to provider config, auth loading,
SSE parsing, request serialization, or release candidates.

## Tier 4: Focused Regression Matrix

When touching a subsystem, add the closest focused regression before editing the
implementation:

| Touched area | Required regression |
| --- | --- |
| CLI args or display | `crates/whalecode-cli/tests/*.rs` command-output test with exact user-facing markers. |
| Interactive input | PTY/stdin integration test covering `/apikey`, `/permissions`, Unicode input, and `/exit`. |
| Model adapter | SSE parser tests for reasoning, text, tool calls, malformed JSON, usage, and finish events. |
| Tool definitions | Contract test that serialized tool JSON still matches model-facing schema expectations. |
| `run_command` | Runtime test proving command args are not shell-expanded and timeout/failure output is structured. |
| `edit_file` | Patch test for stale reads, duplicate old text, hidden config, path escape, and applied diff metadata. |
| Session replay | JSONL test for monotonic sequence rejection and replay summary counters. |
| Primitive modules | Schema roundtrip plus phase-gate test proving artifacts/events exist before behavior is enabled. |

Regression tests must fail for the intended bug, not merely cover nearby code.
If the failure cannot be reproduced deterministically, first add logging or a
test harness that makes the failure observable.

## Anti Self-Deception Rules

- Write the expected behavior from product contracts before looking at current
  output.
- Do not weaken assertions to match current broken output.
- Do not count `cargo test` alone as a completed gate for CLI or agent-loop
  changes.
- Do not treat a mocked provider test as a provider compatibility guarantee;
  run Tier 3 for provider-boundary changes.
- Do not leave a failing runtime artifact unexplained. Inspect the session JSONL
  and update this document if the failure teaches a reusable operational lesson.
- Do not add local fixed natural-language replies to make a smoke test pass.
  Natural-language input must still route through the Agent/Model path.

## Logging Expectations

Every new feature or bug fix must leave enough runtime evidence to diagnose
future failures:

- CLI startup prints workspace, model, write gate, command gate, max turns, and
  session path.
- Tool execution records started/finished events with status and artifact id.
- Permission decisions record allowed or rejected outcomes.
- Patch application records both artifact creation and apply result.
- Provider calls preserve reasoning/text/tool-call deltas and token usage.
- Failure paths still write a session when the run has started.

When a test fails, prefer asserting on session events over fragile prose once the
event contract exists. Prose assertions are acceptable for top-level CLI smoke
markers that users rely on.

## Current Gaps To Close

1. Add a first-class `cargo xtask` or Rust test harness for runtime smoke so the
   shell script can become a thin wrapper.
2. Add a `run_command` runtime regression with a command that would behave
   differently if passed through a shell string.
3. Add session-log assertions that parse JSONL and verify event families instead
   of only grepping CLI replay output.
4. Add primitive-module phase-gate tests before implementing differentiated
   Create/Debug behavior.
5. Add a future web-viewer Storybook and visual regression tier when the
   TypeScript viewer enters the repo.

## References

- Cargo tests and workspace execution: https://doc.rust-lang.org/cargo/commands/cargo-test.html
- Rustfmt command gate: https://github.com/rust-lang/rustfmt
- Clippy lint gate: https://doc.rust-lang.org/clippy/
- CLI integration testing pattern: https://docs.rs/assert_cmd/latest/assert_cmd/
- Smoke testing in delivery pipelines: https://martinfowler.com/bliki/SmokeTest.html

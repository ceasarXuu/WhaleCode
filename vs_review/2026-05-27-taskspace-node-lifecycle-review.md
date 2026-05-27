# TaskSpace Node Lifecycle Review

## Round 1 - Implementation Review

### Review Input Packet

- Objective: verify the TaskSpace node lifecycle implementation that lets the main agent explicitly finish or block the current task node through `taskspace_control`.
- Review target: code implementation, runtime state-machine behavior, tool schema, session wiring, and tests.
- Target locations:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs`
  - `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`
- Change introduction: `taskspace_control` now supports `finish_node` and `block_node`; runtime records a main-node result, changes node status, unlocks downstream dependencies only on completion, and session/tool layers expose the path.
- Risk focus:
  - State-machine correctness for current-node binding, pending/completed/subagent-held nodes, and downstream readiness.
  - Whether blocker/completion records can corrupt node context, leases, maintenance barriers, or active map state.
  - Whether tool schema and handler semantics are sufficient for the model to use the new actions without hidden protocol gaps.
  - Whether tests cover the real behavioral risks introduced by the new lifecycle path.
- Verification status before review:
  - `rustup run stable rustfmt ...` completed with existing stable rustfmt warnings about unstable config.
  - `git diff --check` passed, with CRLF warnings only.
  - `cargo test -p codex-core action_map::runtime --lib --locked --jobs 2` passed: 35 tests.
  - `cargo test -p codex-tools --lib --locked --jobs 2` passed: 134 passed, 1 ignored.
  - `cargo test -p codex-core --test all action_map_scenario_evaluation --locked --jobs 2 -- --nocapture` passed: 2 tests.
- Reviewer instructions: fresh internal subagent, no forked main context, read-only review, inspect targets directly, cite file paths and line numbers where possible, and return summary, blocking findings, non-blocking risks, required fixes, missing tests, and missing logs/observability.

### Reviewer Selection

- Reviewer role: TaskSpace runtime state-machine reviewer.
- Rationale: the change is narrow but affects state transitions, node result storage, and model-facing tool control.

### Launch Records

- Round: 1
- Reviewer role: TaskSpace runtime state-machine reviewer
- Internal subagent mechanism: `multi_agent_v1.spawn_agent`
- Agent id: `019e6988-6610-7e01-ba00-1869b72f93c4`
- Nickname: `Fermat`
- Context inheritance: `fork_context=false`; main-agent chat history and reasoning were not forked.
- Explicitly excluded context: full conversation history, main-agent hidden reasoning, implementation rationale beyond the neutral input packet, and full diff dumps.
- Permissions: read-only instructions; reviewer was told not to modify files.
- Input packet source: the packet recorded above in this report.

### Reviewer Output

- Summary: implementation path is wired, but initial state-machine behavior was not acceptable.
- Blocking finding 1: `finish_node(next_node_id=...)` silently swallowed bind failures when the next node was missing, pending, completed, leased, or blocked by a maintenance barrier.
- Blocking finding 2: after `block_node`, the next ordinary tool call could auto-bind the blocked node because automatic main binding fell back from ready nodes to blocked nodes.
- Blocking finding 3: `bind_node` allowed binding a running or subagent-held node, so main-agent ordinary tools could be attributed to a node already held by a subagent.
- Non-blocking risk 1: result ids were allocated before full validation, so failed lifecycle attempts could consume result sequence ids.
- Non-blocking risk 2: `taskspace_control` remains `strict: false` and action-specific required fields are enforced by runtime parsing rather than by a discriminated schema.
- Missing tests: invalid next-node handling, block-then-ordinary-work behavior, subagent-held node binding rejection, and handler/session edge paths.
- Missing observability: no explicit warning/event for swallowed next-node bind failures in the initial implementation; handler output did not report resulting binding.

### Main-Agent Responses

- Blocking finding 1: accept. Fixed by validating `next_node_id` before lifecycle mutation and propagating bind errors instead of swallowing them. Added `finish_main_node_rejects_invalid_next_node_without_mutating_current_node`.
- Blocking finding 2: accept. Fixed by making automatic main binding choose only ready nodes; blocked nodes now require explicit `bind_node` or a newly created recovery node. Added `block_main_node_does_not_auto_rebind_blocked_node_for_ordinary_work`.
- Blocking finding 3: accept. Fixed by rejecting `Running` or `active_lease` nodes in `bind_main_node` and `validate_main_binding`. Added `bind_main_node_rejects_subagent_held_node`.
- Non-blocking risk 1: accept. Moved result-id allocation after immutable validation in `record_main_node_lifecycle_result`.
- Non-blocking risk 2: defer. The current `JsonSchema` helper does not model action-specific `oneOf` contracts cleanly; runtime parsing still enforces required fields. This should be revisited when the tool schema layer supports tagged unions without adding ad hoc schema construction.
- Missing tests: accept for runtime-critical cases; added three negative runtime tests. Handler/session serde edge tests remain deferred because this slice focuses on runtime correctness and existing handler path delegates to the tested runtime APIs.
- Missing observability: accept for swallowed bind failure because the failure is no longer swallowed. Defer richer handler output until the taskspace viewer/event model is revised, to avoid expanding the user-facing internal protocol in this slice.

### Closure Status

- Accepted blocking findings fixed locally.
- Validation after fixes:
  - `rustup run stable rustfmt third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs third_party/codex-cli/codex-rs/core/src/session/mod.rs third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`
  - `git diff --check`
  - `cargo test -p codex-core action_map::runtime --lib --locked --jobs 2`: 38 passed.
  - `cargo test -p codex-tools --lib --locked --jobs 2`: 134 passed, 1 ignored.
  - `cargo test -p codex-core --test all action_map_scenario_evaluation --locked --jobs 2 -- --nocapture`: 2 passed.
- Closure review required because blocking findings were accepted.

## Round 2 - Blocking Closure Review

### Review Input Packet

- Objective: verify closure of the three accepted blocking findings from Round 1.
- Review target: fixed TaskSpace runtime state-machine behavior and regression tests.
- Target locations:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`
- Closure claims to verify:
  - `finish_node(next_node_id=...)` no longer silently swallows bind failures and validates the next node before mutating the current node.
  - `block_node` no longer allows automatic fallback binding to the blocked node on the next ordinary tool call.
  - `bind_node` and main binding validation reject running or subagent-held nodes.
  - result id allocation no longer happens before main lifecycle validation succeeds.
- Validation status before closure review:
  - `git diff --check` passed with CRLF warnings only.
  - `cargo test -p codex-core action_map::runtime --lib --locked --jobs 2` passed: 38 tests.
  - `cargo test -p codex-tools --lib --locked --jobs 2` passed: 134 passed, 1 ignored.
  - `cargo test -p codex-core --test all action_map_scenario_evaluation --locked --jobs 2 -- --nocapture` passed: 2 tests.
- Reviewer instructions: fresh internal subagent, no forked main context, read-only review, focus only on closure of accepted blocking findings, cite file paths and line numbers where possible, and say whether closure passes or still has blocking gaps.

### Reviewer Selection

- Reviewer role: TaskSpace blocking-closure reviewer.
- Rationale: accepted blocking state-machine findings require a fresh independent closure check.

### Launch Records

- Round: 2
- Reviewer role: TaskSpace blocking-closure reviewer
- Internal subagent mechanism: `multi_agent_v1.spawn_agent`
- Agent id: `019e6995-0202-7a21-a701-b73314096f29`
- Nickname: `Galileo`
- Context inheritance: `fork_context=false`; main-agent chat history and reasoning were not forked.
- Explicitly excluded context: full conversation history, main-agent hidden reasoning, implementation rationale beyond the neutral closure packet, and full diff dumps.
- Permissions: read-only instructions; reviewer was told not to modify files.
- Input packet source: the packet recorded above in this report.

### Reviewer Output

- Closure Verdict: pass.
- Remaining blocking findings: none.
- Non-blocking risks: none found in the scoped files.
- Evidence:
  - `runtime.rs`: `finish_main_node` validates `next_node_id` before lifecycle mutation and propagates `bind_main_node(next_node_id)?`.
  - `runtime.rs`: `validate_next_main_binding_after_finish` rejects self-binding, missing, completed, running/subagent-held, unresolved pending, and barriered next nodes.
  - `runtime.rs`: lifecycle result recording clears `current_main_node_id`; automatic fallback only selects `Ready`, so a just-blocked node is not auto rebound.
  - `runtime.rs`: `bind_main_node` rejects `Running` or `active_lease`.
  - `runtime.rs`: main binding validation rejects `Running` or `active_lease`.
  - `runtime.rs`: main lifecycle validation completes before `next_result_id()`.
  - `taskspace_tool.rs`: tool surface exposes the four expected actions and `next_node_id`.

### Main-Agent Responses

- Closure verdict: accept. No remaining blocking findings.
- Non-blocking risks: accept as none in scoped closure review.

### Closure Status

- Passed. Accepted blocking findings from Round 1 are closed by a fresh internal subagent review.

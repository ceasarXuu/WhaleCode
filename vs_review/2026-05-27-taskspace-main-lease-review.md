# TaskSpace Main Lease Review

## Round 1 - Implementation Review

### Review Input Packet

- Objective: verify the TaskSpace main-agent lease implementation that makes main-agent ordinary work hold a real node lease instead of relying only on `current_main_node_id`.
- Review target: runtime state machine, lease lifecycle, multi-agent spawn interaction, and tests.
- Target locations:
  - `third_party/codex-cli/codex-rs/core/src/action_map/map.rs`
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_tests.rs`
  - `third_party/codex-cli/codex-rs/core/tests/suite/action_map_scenario_evaluation.rs`
- Change introduction:
  - `AssignmentLease` now records `previous_node_status`.
  - `ActionMapRuntimeState` tracks `current_main_lease_id`.
  - Main-agent bind/ordinary-tool paths claim a `LeaseHolder::Main` lease, mark the node running, record main tool results against that lease, and release it on finish/block/restart/rebind.
  - Subagent timeout lookup and thread result lookup ignore main leases.
  - Tests now create explicit subagent nodes before spawning and treat main implementation nodes as separate main leases.
- Risk focus:
  - Main and subagent leases must remain mutually exclusive on a node.
  - Main lease release must restore the previous node status on rebind/restart but must not reopen completed or blocked results.
  - Existing subagent spawn, close, wait-timeout, and child-result paths must not accidentally act on main leases.
  - Scenario tests should represent the new product semantics: subagent nodes use `bind_current=false`, implementation nodes use `bind_current=true`.
- Verification status before review:
  - `git diff --check` passed with CRLF warnings only.
  - `cargo test -p codex-core action_map::runtime --lib --locked --jobs 2` passed: 38 tests.
  - `cargo test -p codex-core action_map --lib --locked --jobs 2` passed: 46 tests.
  - `cargo test -p codex-core --test all action_map_scenario_evaluation --locked --jobs 2 -- --nocapture` passed: 2 tests.
- Reviewer instructions: fresh internal subagent, no forked main context, read-only review, inspect targets directly, cite file paths and line numbers where possible, and return summary, blocking findings, non-blocking risks, required fixes, missing tests, and missing logs/observability.

### Reviewer Selection

- Reviewer role: TaskSpace lease/state-machine reviewer.
- Rationale: this change touches mutual exclusion, lease restore semantics, and spawn/wait/close interaction.

### Launch Records

- Round: 1
- Reviewer role: TaskSpace lease/state-machine reviewer
- Internal subagent mechanism: `multi_agent_v1.spawn_agent`
- Agent id: `019e69b1-1c6c-7532-86cf-9f5833419d38`
- Nickname: `Russell`
- Context inheritance: `fork_context=false`; main-agent chat history and reasoning were not forked.
- Explicitly excluded context: full conversation history, main-agent hidden reasoning, implementation rationale beyond the neutral input packet, and full diff dumps.
- Permissions: read-only instructions; reviewer was told not to modify files.
- Input packet source: the packet recorded above in this report.

### Reviewer Output

- Summary: no blocking issue found. The direction is valid: main ordinary tool work now holds a real `LeaseHolder::Main` lease, and subagent paths filter by holder.
- Blocking findings: none.
- Non-blocking risk 1: `release_lease` is a generic API and can release a main lease by id. Current real subagent call paths use subagent assignment leases, but the API itself is not holder-limited.
- Non-blocking risk 2: `attach_agent_to_lease` did not limit itself to `LeaseHolder::SubAgent`; if a main lease id were passed by mistake, it could overwrite main lease ownership.
- Non-blocking risk 3: `LeaseReleased` events did not include `holder`, making rollout/viewer diagnosis less direct than `LeaseCreated`.
- Missing tests:
  - main-held node skipped or rejected by subagent spawn.
  - main rebind releases previous node to its prior status.
  - timeout and child-result paths ignore main leases.
  - realistic scenario should cover final `finish_node`/`block_node` release for a main implementation node.
- Missing logs / observability:
  - Add `holder` to lease release events.
  - Include call/tool context in `record_action_map_main_tool_result` failure warnings.

### Main-Agent Responses

- Blocking findings: none.
- Non-blocking risk 1: defer. `release_lease` remains the internal generic primitive because main rebind/restart and subagent cleanup both need the same state transition. Public/session-facing paths are constrained by holder-aware lookup or by assignment ids created for subagents.
- Non-blocking risk 2: accept. `attach_agent_to_lease` now returns `None` unless the target lease holder is `SubAgent`.
- Non-blocking risk 3: accept. `MapRuntimeLeaseReleasedEvent` now includes `holder`.
- Missing tests: accept. Added direct tests for main-held spawn exclusion, main rebind release/restore, and subagent timeout/child-result paths ignoring main leases. Updated realistic scenario so the main implementation node calls `finish_node` and releases the main lease.
- Missing logs / observability: accept. Added release `holder` and included `call_id`/`tool_name` in main tool result recording warnings.

### Closure Status

- Round 1 found no blocking findings.
- Accepted non-blocking follow-ups implemented locally.
- Validation after follow-ups:
  - `git diff --check`
  - `cargo test -p codex-core action_map --lib --locked --jobs 2`: 49 passed.
  - `cargo test -p codex-protocol --lib --locked --jobs 2`: 178 passed.
  - `cargo test -p codex-core --test all action_map_scenario_evaluation --locked --jobs 2 -- --nocapture`: 2 passed.
- A fresh closure review is being run for the accepted non-blocking follow-ups.

## Round 2 - Follow-Up Closure Review

### Review Input Packet

- Objective: verify the non-blocking follow-ups implemented after Round 1.
- Review target: attach/release holder handling, release-event observability, main-lease regression tests, and realistic scenario `finish_node` coverage.
- Target locations:
  - `third_party/codex-cli/codex-rs/protocol/src/protocol.rs`
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
  - `third_party/codex-cli/codex-rs/core/tests/suite/action_map_scenario_evaluation.rs`
- Closure claims to verify:
  - `attach_agent_to_lease` ignores main leases.
  - `LeaseReleased` events now expose `holder`.
  - direct runtime tests cover main-held spawn exclusion, main rebind restore, and subagent timeout/child-result paths ignoring main leases.
  - realistic scenario now has the main implementation node call `finish_node` and expects two lease releases.
  - warning logs include call/tool context when main tool result recording fails.
- Validation status before closure review:
  - `cargo test -p codex-core action_map --lib --locked --jobs 2`: 49 passed.
  - `cargo test -p codex-protocol --lib --locked --jobs 2`: 178 passed.
  - `cargo test -p codex-core --test all action_map_scenario_evaluation --locked --jobs 2 -- --nocapture`: 2 passed.
- Reviewer instructions: fresh internal subagent, no forked main context, read-only review, focus on whether the follow-ups create any new blocking issue, cite file paths and line numbers where possible.

### Reviewer Selection

- Reviewer role: TaskSpace main-lease follow-up closure reviewer.
- Rationale: a second pass checks the post-review edits without inheriting the main thread context.

### Launch Records

- Round: 2
- Reviewer role: TaskSpace main-lease follow-up closure reviewer
- Internal subagent mechanism: `multi_agent_v1.spawn_agent`
- Agent id: `019e69c2-d4af-7901-9899-14001fb1e6c0`
- Nickname: `Nietzsche`
- Context inheritance: `fork_context=false`; main-agent chat history and reasoning were not forked.
- Explicitly excluded context: full conversation history, main-agent hidden reasoning, implementation rationale beyond the neutral closure packet, and full diff dumps.
- Permissions: read-only instructions; reviewer was told not to modify files.
- Input packet source: the packet recorded above in this report.

### Reviewer Output

- Closure verdict: pass.
- Remaining blocking findings: none.
- Non-blocking risk: the realistic scenario asserted total `lease_released` count but did not assert holder split as exactly one `subagent` and one `main`. Runtime direct tests already covered holder behavior, so this was a test-hardening gap rather than a blocking issue.
- Evidence:
  - `protocol.rs`: `MapRuntimeLeaseReleasedEvent` includes `holder`.
  - `runtime.rs`: `attach_agent_to_lease` returns `None` for non-`SubAgent` leases.
  - `runtime.rs`: generic `release_lease` emits `holder`.
  - `runtime.rs`: timeout targets and child-result lookup only match `LeaseHolder::SubAgent`.
  - `runtime.rs`: main lifecycle result release emits `holder: main`.
  - `runtime.rs`: direct tests cover main-held spawn exclusion, main rebind release/restore, and subagent paths ignoring main leases.
  - `action_map_scenario_evaluation.rs`: realistic scenario calls `finish_node` for `node-2` and expects two lease releases.
  - `session/mod.rs`: warning log includes `error`, `call_id`, and `tool_name`.

### Main-Agent Responses

- Closure verdict: accept. No remaining blocking findings.
- Non-blocking risk: accept. Hardened the realistic scenario with `count_lease_released_by_holder`, asserting exactly one `subagent` release and one `main` release.

### Closure Status

- Passed. Follow-up closure review found no blocking findings.
- Additional validation after test hardening:
  - `cargo test -p codex-core --test all action_map_scenario_evaluation --locked --jobs 2 -- --nocapture`: 2 passed.

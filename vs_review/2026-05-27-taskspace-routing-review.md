# Subagent VS Review: TaskSpace Routing Controls

- Created: 2026-05-27T22:38:51+08:00
- Updated: 2026-05-27T23:27:00+08:00
- Task: Add explicit TaskSpace task routing controls while keeping semantic task selection in the main agent.
- Report path: `vs_review/2026-05-27-taskspace-routing-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: open

## Round 1: Implementation Review

### Review Input

#### Objective
TaskSpace should support multiple task maps inside one session. The main agent must semantically choose whether a user request belongs to an existing task or starts a new task. Runtime must only expose task ids/state and validate structural operations; it must not use keyword or semantic matching.

#### Review Target
Code implementation for `taskspace_control(action=start_task)` and `taskspace_control(action=route_task)`, including runtime state transitions, developer context, handler wiring, tool schema, and tests.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs`
- `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`
- Command: `rustup run stable cargo test -p codex-core action_map --lib --locked --jobs 2`

#### Change Introduction
The implementation adds a first-class `start_task` action that creates a new TaskSpace task, active map, and first node, and a `route_task` action that switches active runtime state to an existing task chosen by the agent. Developer context now prints a task inventory and instructs the agent to choose `route_task` or `start_task` before ordinary work. Existing node lifecycle actions remain available.

#### Risk Focus
- Runtime must not perform semantic matching or keyword routing.
- Switching tasks must not leave stale main leases, blocked maintenance barriers, or incorrect active task status behind.
- `start_task` must create a valid task/map/node path without relying on the old default BaseMap graph.
- Prompt/tool schema must give the agent enough concrete ids and action rules to supply the right arguments.
- Tests must cover state transitions, failure cases, and developer-context routing instructions rather than only happy-path field creation.

#### Verification Status
- Passed: `rustup run stable cargo test -p codex-core action_map --lib --locked --jobs 2` with 54 passing tests.
- Not yet run in this round: full tools crate tests, scenario E2E, diff whitespace check.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Report blocking findings, non-blocking risks, required fixes, missing tests, and missing logs or observability.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | This change touches runtime state transitions, tool handlers, leases, and tests. | correctness, state consistency, hidden edge cases |
| architecture-adversary | The change adds a new routing abstraction that must preserve the existing TaskSpace/runtime boundaries. | module boundaries, runtime-vs-agent responsibility split |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1.spawn_agent` fresh internal subagent | `019e69e0-89ef-7072-8c47-b8387de71191` / Boyle | spawn tool result in current Codex thread | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| architecture-adversary | `multi_agent_v1.spawn_agent` fresh internal subagent | `019e69e0-d1b1-7e42-b661-9d476ebb726c` / Beauvoir | spawn tool result in current Codex thread | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### implementation-adversary / Boyle

##### Summary
Not ready to merge. `route_task` validates only the supplied `task_id`; no runtime keyword or semantic matching was found. `start_task` creates a new task/map/first node without seeding the old BaseMap graph. The blocking issue is state cleanup: task switching can erase an unresolved maintenance barrier and make the old over-budget node ready again.

##### Blocking Findings
- Barrier can be incorrectly cleared by task switching. `start_task_for_main` and `route_task_for_main` release the current main lease and clear the global maintenance barrier. `release_lease` restores a running node to its previous status, commonly `Ready`. Result: an over-budget node can become bindable again after routing away/back, bypassing the recovery-node requirement.

##### Non-blocking Risks
- Tool schema is weak for model reliability: only `action` is required, `strict: false`, no enum/oneOf per action, and `bind_current` is described only for `create_node` even though `start_task` accepts it.
- Runtime is not doing semantic matching, but developer context exposes semantic `title/objective` inventory. If the contract literally means only id/state, this is broader than stated.
- There are no explicit task-created/task-routed/task-status events; consumers infer from map/lease events plus snapshots.

##### Required Fixes
- Make maintenance barriers task/map scoped, or otherwise preserve unresolved barriers when routing away from a task.
- When returning to a task with an unresolved barrier, ordinary work and spawn should still be blocked unless a different recovery node is created or bound.
- Tighten `taskspace_control` schema: action enum, conditional required fields, `bind_current` description covering `start_task`, and preferably strict validation.

##### Missing Tests
- Barrier state invariant: raise barrier, `start_task`, route back, then prove ordinary tools cannot auto-bind the old over-budget node.
- Same scenario for `route_task` between two existing tasks.
- Failure no-mutation tests for empty `task_id`, wrong owner, missing `active_map_id`, missing target map, inactive target map.
- Handler/schema tests for `start_task` and `route_task` argument parsing and model-facing spec.
- Full tools crate/scenario E2E is still missing in this round.

##### Missing Logs / Observability
- Add structured runtime events or logs for task creation, route attempts, route success/failure, previous/next active task, previous/next active map, and barrier carry/clear reason.
- Add observability for rejected route attempts without mutating state.

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` - `route_task` validates only id/existence/owner/map status.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` - `start_task` creates task/map and then a first concrete node.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` - existing tests covered happy path and missing task no-mutation, but not barrier preservation across task switches.

#### architecture-adversary / Beauvoir

##### Summary
The implementation mostly preserves the boundary that the agent performs semantic routing and runtime performs structural validation: `route_task` precisely looks up a supplied `task_id` and validates owner, active map, and map status. Two architectural blockers remain: routing constraints are not reliably refreshed each normal user turn, and task route/start lacks replayable task-level events.

##### Blocking Findings
- Developer context is not reliably refreshed before every ordinary action. `build_developer_context()` includes the routing instruction and task inventory, but the steady-state path uses settings diffs and can skip `build_initial_context()`, where action map context is injected.
- `start_task` / `route_task` task state changes have no task-level `MapRuntimeEvent`. Observability, resume/replay, and E2E cannot rely on a replayable task state machine if route/start only emit map/node/lease events or no events.

##### Non-blocking Risks
- `prepare_main_tool_call()` auto-binds the first ready node on the active map. That preserves node-driven execution but makes stale task inventory more dangerous.
- Tool schema is `strict: false` and only requires `action`; handler validation helps but model-facing guidance remains weak.
- The referenced same-day runtime redesign plan document was not found.

##### Required Fixes
- Inject latest TaskSpace developer context on steady-state regular user turns, or add a light runtime gate that requires explicit route/start per user turn.
- Add task-level events such as task_created/task_routed/task_status_changed and use them for viewer/E2E/replay.
- Include task/map runtime state in resume/replay restoration.
- Tighten `taskspace_control` schema by action.

##### Missing Tests
- Session-level test that start/route is followed by a later developer context containing the latest task inventory.
- Resume/reconstruction test for task/map state.
- Handler/tool schema tests for missing required fields and invalid action.
- More route_task boundary tests: wrong owner, target task without active map, abandoned map, route with barrier.

##### Missing Logs / Observability
- Route/start need task-level structured event/log output.
- A route with no lease/barrier can otherwise produce no event; snapshot polling alone is not enough for replay.

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` - no hidden semantic matching: `route_task_for_main` trims and looks up `tasks.get(task_id)`.
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs` - action map developer context was only injected by `build_initial_context()`.
- `third_party/codex-cli/codex-rs/core/src/session/rollout_reconstruction.rs` - reconstruction restored only map runtime mode before this fix.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Boyle | Barrier can be cleared by task switching and old over-budget node can become bindable | blocking | accept | The previous implementation used one global `maintenance_barrier` and cleared it in `start_task` / `route_task`. | Replaced the global barrier with map-scoped `maintenance_barriers`; removed route/start barrier clearing; added route/start regression tests that prove old task barriers still block when routed back. | Round 2 closure review |
| Boyle | Tool schema lacks action enum and `bind_current` does not mention start_task | non-blocking | accept | The schema only described action as free string and documented bind_current for create_node only. | Added `JsonSchema::string_enum` for action and updated `bind_current` description for start_task/create_node. | Validate with `codex-tools` tests |
| Boyle | No explicit task events | non-blocking | accept | Route/start state changes should be replayable and observable. | Added `task_created`, `task_status_changed`, `task_routed`, and `snapshot_updated` events. | Round 2 closure review |
| Beauvoir | Steady-state developer context can go stale | blocking | accept | Regular user turns with an existing reference context used only settings diffs. | `record_context_updates_and_set_reference_context_item` now appends latest TaskSpace developer context in steady state; added a session regression test. | Round 2 closure review |
| Beauvoir | Task route/start state lacks replayable events and resume restoration | blocking | accept | Reconstruction only restored runtime mode. | Added task-level events, snapshot_updated events, snapshot restoration, and a reconstruction regression test. | Round 2 closure review |
| Beauvoir | Stronger per-user-turn route/start gate | non-blocking | defer | A hard per-turn gate is a product behavior change: it may force redundant route_task calls even when continuing the current task. Latest context injection keeps the agent informed without adding a new state machine yet. | Deferred; current runtime still blocks ordinary work when no task path/node binding exists. | Revisit after real E2E traces show stale-task mistakes |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: pending
- Blocking re-review passed: pending
- Blocking re-review round links:
  - Round 2 pending
- Blocking re-review launch records:
  - pending
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Allowed to proceed: no

## Round 2: Blocking Closure Review

### Review Input

#### Objective
Verify that the accepted Round 1 blocking findings are actually closed: task switching must preserve unresolved maintenance barriers; steady-state developer context must refresh latest TaskSpace inventory; task route/start must emit replayable task-level state and restore from rollout reconstruction.

#### Review Target
Closure review for the TaskSpace routing implementation after Round 1 fixes.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/session/tests.rs`
- `third_party/codex-cli/codex-rs/core/src/session/rollout_reconstruction.rs`
- `third_party/codex-cli/codex-rs/core/src/session/rollout_reconstruction_tests.rs`
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs`
- `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`

#### Change Introduction
The fix makes maintenance barriers map-scoped, removes barrier clearing from `start_task` and `route_task`, adds task-level events and snapshot-updated events, restores runtime state from the latest snapshot during rollout reconstruction, and injects current TaskSpace developer context on steady-state regular user turns.

#### Risk Focus
- The old over-budget node must not become bindable after routing away and back.
- Other tasks must remain usable while a different task has a maintenance barrier.
- Snapshot restore must preserve task/map/node/lease/result/barrier state sufficiently for subsequent runtime validation.
- Steady-state context refresh must not duplicate full initial context or break standard mode.
- Task-level events must not break existing event consumers.

#### Verification Status
- Passed: `rustup run stable cargo test -p codex-core action_map --lib --locked --jobs 2` with 57 passing tests.
- Passed: `rustup run stable cargo test -p codex-core record_context_updates_refreshes_taskspace_inventory_in_steady_state --lib --locked --jobs 2`.
- Passed: `rustup run stable cargo test -p codex-core reconstruct_history_restores_latest_map_runtime_snapshot --lib --locked --jobs 2`.
- Passed: `rustup run stable cargo test -p codex-protocol --lib --locked --jobs 2` with 178 passing tests.
- Passed: `rustup run stable cargo test -p codex-tools --lib --locked --jobs 2` with 134 passing, 1 ignored.
- Passed: `rustup run stable cargo test -p codex-core --test all action_map_scenario_evaluation --locked --jobs 2 -- --nocapture` with 2 passing scenario tests.
- Passed: `git diff --check`.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Focus on whether Round 1 blocking findings are closed.
- Cite evidence paths and line numbers when possible.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | Closure depends on runtime state correctness and tests. | barrier preservation, snapshot restore, event emission |
| architecture-adversary | Closure depends on runtime/agent boundary and replay/context architecture. | routing contract, context refresh, replay model |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary + architecture-adversary | `multi_agent_v1.spawn_agent` fresh internal subagent | `019e6a04-127e-7be2-8518-44f22b473b44` / Singer | spawn tool result in current Codex thread | no | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### Singer

##### Summary
Round 1 blocking findings are closed based on read-only code inspection. The reviewer did not modify files or rerun tests; it used provided test results plus direct source review.

##### Blocking Findings
- None.

##### Non-blocking Risks
- Replay is snapshot-driven, not event-sourced. This is acceptable for closure, but future TaskSpace mutators must keep using the session emit path so `snapshot_updated` is always recorded after state changes.
- `route_task` no-op to the already-active task emits no event/snapshot. No state changes, so not blocking.
- `taskspace_control` still has `strict: false` and no per-action conditional required schema. Runtime validates required fields, so this remains a model-reliability risk, not a closure blocker.

##### Required Fixes
- None for the accepted Round 1 blockers.

##### Missing Tests
- No public resume-path E2E that then proves `Session::action_map_snapshot()` plus ordinary-work blocking behavior matches the restored snapshot.
- No rollback/compaction-specific test for TaskSpace snapshots. Current reconstruction restores the latest `snapshot_updated` event globally.

##### Missing Logs / Observability
- Success paths now emit task-level events and snapshots.
- Rejected `route_task` attempts still return errors without structured `MapRuntimeEvent` observability. This is useful follow-up logging but not required to close the accepted blockers.

##### Evidence
- Barrier preservation is now map-scoped and is serialized/restored in snapshots.
- Ordinary tools and spawn check only the active map's barrier before auto-binding or assignment.
- `start_task` and `route_task` release leases/statuses but no longer clear old barriers; `route_task` emits `TaskRouted`.
- Regression tests cover route-away/back and start-new/back barrier preservation.
- Steady-state context appends current TaskSpace developer context when a reference context already exists, with focused test coverage.
- Task-level protocol events and snapshot events exist.
- Session emission persists every runtime event and follows non-empty event batches with `snapshot_updated`.
- Rollout reconstruction captures the latest snapshot and apply restores it, with focused coverage.

##### Closure Verdict
passed

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Singer | Round 1 blocking findings are closed | blocking closure | accept | Closure reviewer found no blocking findings. | No further blocking fix required. | None |
| Singer | Snapshot-driven replay requires future mutators to emit snapshots | non-blocking | accept | Session emit path now appends `snapshot_updated`; future mutators should keep using it. | Documented in review report as an implementation constraint. | Watch in future reviews |
| Singer | No public resume-path E2E / rollback-specific TaskSpace snapshot test | non-blocking | defer | Focused reconstruction test exists; broader public resume/rollback E2E is larger than this slice. | Deferred as follow-up validation hardening. | Add when resume/compaction TaskSpace work expands |
| Singer | Rejected route attempts lack structured events | non-blocking | defer | Rejections currently return model-visible errors without mutation; not required to close accepted blockers. | Deferred as observability enhancement. | Add if route failure diagnosis becomes hard |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Allowed to proceed: yes

## Final Conclusion

Round 1 blocking findings are closed. The implementation may proceed with the noted non-blocking follow-ups.

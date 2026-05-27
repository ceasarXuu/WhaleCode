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
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2 passed
- Blocking re-review launch records:
  - Singer / `019e6a04-127e-7be2-8518-44f22b473b44`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Allowed to proceed: yes

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

## Round 3: Per-Turn Routing Gate Review

### Review Input

#### Objective
Validate the implementation slice that makes TaskSpace enforce explicit agent task routing on every real user turn, prevents runtime-created default maps from replacing agent-created task maps, and keeps ordinary tools/subagent spawn blocked until the agent has routed to an existing task or started a new task.

#### Review Target
Code implementation, test updates, and runtime/prompt contract for the TaskSpace per-turn routing gate.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/tests/suite/action_map_scenario_evaluation.rs`
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs`
- `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`
- `third_party/codex-cli/codex-rs/tui/src/app/action_map_viewer.rs`
- `docs/plans/2026-05-27-taskspace-runtime-rearchitecture-implementation-plan.md`

#### Change Introduction
The current implementation adds `routing_required` and `bootstrap_required` to runtime state and snapshots. Entering TaskSpace and every real user turn set the routing gate. `taskspace_control(action=start_task)` starts a semantic task, creates its task path and first node, and clears the gate. `taskspace_control(action=route_task)` routes to an existing task and clears the gate. `create_node` no longer bootstraps a task path. `prepare_main_tool_call` no longer auto-binds or auto-creates anything; it only validates routing, maintenance barriers, and an existing main node lease. Spawn assignment is also blocked while routing is required. The viewer exposes routing/bootstrap status.

#### Risk Focus
- `begin_user_turn` must run at the correct boundary: real user turns, not every internal tool turn or subagent continuation.
- `prepare_main_tool_call` must not mutate TaskSpace state, auto-create maps, or auto-bind nodes.
- `create_node` must not reintroduce implicit first-task creation.
- The agent must have clear developer-context/tool-schema instructions for `start_task`, `route_task`, explicit binding, and ordinary-work gating.
- Snapshot/restore and viewer protocol changes must preserve the gate state without breaking existing consumers.
- Tests must prove black-box behavior: no work/spawn before routing, route/start clears the gate, restored snapshots keep the gate.
- The implementation must stay inside existing ActionMap/Session/tool infrastructure and not create a duplicate TaskSpace runtime.

#### Verification Status
- Passed before this review: `rustup run stable cargo test -p codex-core action_map::runtime --lib --locked --jobs 2`.
- Passed before this review: `rustup run stable cargo test -p codex-core --test all action_map_scenario_evaluation --locked --jobs 2 -- --nocapture`.
- Passed before this review: `rustup run stable cargo test -p codex-core record_context_updates_refreshes_taskspace_inventory_in_steady_state --lib --locked --jobs 2`.
- Passed before this review: `rustup run stable cargo test -p codex-protocol --lib --locked --jobs 2`.
- Not yet rerun after review: full `codex-tools` lib tests, focused reconstruction test, focused TUI command/viewer tests, and `git diff --check`.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Challenge the implementation; do not assume the design is correct.
- Cite evidence paths and line numbers when possible.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | The change is a runtime state/gate mutation across tool dispatch, spawn, snapshot, and tests. | state correctness, edge cases, compatibility |
| architecture-adversary | The change is meant to enforce TaskSpace via existing Codex runtime seams without duplicating infrastructure. | boundaries, mutation ownership, long-term design |
| test-validity-adversary | The user explicitly requires real behavior coverage and has challenged weak/mock-like tests before. | self-deceptive tests, missing black-box paths |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1.spawn_agent` fresh internal subagent | `019e6a2e-f7bd-7571-b6fd-92249e3de372` / Kierkegaard | spawn tool result in current Codex thread | no | Round 3 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| architecture-adversary | `multi_agent_v1.spawn_agent` fresh internal subagent | `019e6a2f-3aa1-71a1-b46b-a1c46350a066` / Avicenna | spawn tool result in current Codex thread | no | Round 3 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| test-validity-adversary | `multi_agent_v1.spawn_agent` fresh internal subagent | `019e6a30-0926-7a63-b912-e7f32f7fc2da` / Rawls | spawn tool result in current Codex thread | no | Round 3 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### implementation-adversary / Kierkegaard

##### Summary
The implementation direction is right, but the first reviewed slice was not ready: the routing gate was placed too broadly and the legacy spawn path still bypassed the new TaskSpace contract.

##### Blocking Findings
- `begin_user_turn` was wired through a generic context refresh path, so internal prompt/debug/pending work could accidentally set the user-turn routing gate.
- Legacy/default `spawn_agent` did not go through the same TaskSpace assignment checks as the v2 spawn handler.
- New snapshot fields were not backward-compatible for old rollout snapshots.
- Some multi-agent tests still created nodes directly before `start_task`, which preserved obsolete test assumptions.

##### Non-blocking Risks
- Runtime/tool tests were stronger than black-box session tests; a real user turn needed direct coverage.
- The viewer exposed state, but the protocol needed to tolerate missing gate fields from older sessions.

##### Required Fixes
- Move gate activation to the real `Op::UserInput` / `Op::UserTurn` boundary.
- Gate both legacy and v2 subagent spawn paths.
- Add serde defaults for new snapshot fields.
- Update stale tests to use agent-created task paths.

##### Missing Tests
- A session test that proves real user input sets the routing gate and emits a snapshot.
- Legacy spawn rejection/assignment tests.

##### Missing Logs / Observability
- Snapshot visibility for route/bootstrap/reborn flags should be exposed to TaskSpace viewer consumers.

#### architecture-adversary / Avicenna

##### Summary
The runtime/agent boundary is mostly preserved: runtime validates structure and does not semantically choose tasks. Three blockers remained in the reviewed slice.

##### Blocking Findings
- The per-turn routing gate was not durably persisted by a `snapshot_updated` event at the point it was set.
- `ActionMapSnapshot` added fields without `serde(default)`, risking old rollout/session compatibility.
- `/task-reborn` still directly created a replacement map, violating the new rule that map generation must be done by the agent through TaskSpace controls.

##### Non-blocking Risks
- A route/start gate is only useful if the agent prompt makes the required ids and action sequence explicit.
- `/task-reborn` should be a pending intent, not a runtime-side map factory.

##### Required Fixes
- Emit a snapshot immediately after a real user turn sets the gate.
- Add defaults and a legacy deserialization test.
- Change reborn to request-only state and let the next agent turn generate the new map.

##### Missing Tests
- Reborn should prove no task/map path is created by the runtime.
- Snapshot restore should preserve gate flags.

##### Missing Logs / Observability
- Viewer/snapshot should make route required, bootstrap required, and reborn requested visible.

#### test-validity-adversary / Rawls

##### Summary
The tests were improving but still left room for false confidence. The main problem was that several checks exercised helpers instead of the same observable path a user/session would hit.

##### Blocking Findings
- Tests did not sufficiently prove the real session/user-input path sets the gate before model work.
- There was no handler-level proof that old legacy spawn cannot bypass routing.
- The E2E scenario did not prove that enabling TaskSpace leaves the graph empty until the agent performs `start_task`.
- Regression scripts could still report confusing output if the cargo invocation produced no meaningful matching tests or stale scenario reports.

##### Non-blocking Risks
- The automated E2E remains a scripted-provider scenario, so it validates the runtime/tool path but is not a live paid-model acceptance test.

##### Required Fixes
- Add real session/user input coverage.
- Add legacy spawn coverage.
- Assert initial empty snapshot and `task_created` before `map_created` in the scenario.
- Make scripts fail unless tests actually ran and the scenario report belongs to the current run.

##### Missing Tests
- A future live CLI acceptance run should still be kept separate from the deterministic regression suite.

##### Missing Logs / Observability
- Scenario reports should keep explicit `provider_requests`, `map_events`, rollout path, and validation output.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Kierkegaard | `begin_user_turn` was wired too broadly through context refresh/internal work | blocking | accept | A TaskSpace gate should represent a real user turn, not internal context maintenance. | Moved gate activation into `user_input_or_turn_inner` after `new_turn_with_sub_id` succeeds; removed gate mutation from generic context refresh. Added `real_user_input_sets_taskspace_routing_gate_and_snapshot`. | Round 4 closure review |
| Kierkegaard | Legacy/default `spawn_agent` bypassed TaskSpace assignment checks | blocking | accept | The standard legacy handler could spawn work before route/start. | Added TaskSpace assignment preparation/release to the legacy spawn handler; assignment is prepended to child input when allowed. Added legacy spawn reject/claim tests. | Round 4 closure review |
| Kierkegaard / Avicenna | New snapshot gate fields were not backward-compatible | blocking | accept | Old rollout snapshots lack the new booleans. | Added `serde(default)` for `routing_required`, `bootstrap_required`, and `reborn_requested`; added a legacy deserialization regression. | Round 4 closure review |
| Kierkegaard | Stale multi-agent tests created nodes before `start_task` | blocking | accept | Tests encoded the old implicit map/node path. | Reworked action-map multi-agent tests to use `start_task` before creating/binding work nodes. | Round 4 closure review |
| Avicenna | Routing gate was not persisted immediately when set | blocking | accept | A gate set only in memory could be missed by viewer/replay until another event occurred. | Added `begin_action_map_user_turn` to emit `snapshot_updated` immediately when the real user turn sets the gate. | Round 4 closure review |
| Avicenna | `/task-reborn` still created a new map directly | blocking | accept | Reborn map generation must be agent-authored through `start_task`, not runtime default construction. | Replaced restart behavior with `request_action_map_reborn`; it sets `reborn_requested` and routing/bootstrap requirements without creating tasks/maps. Added runtime regression. | Round 4 closure review |
| Rawls | E2E did not prove no default map after enable | blocking | accept | The scenario previously allowed runtime bootstrap assumptions. | Added initial snapshot assertions for empty tasks/maps plus `routing_required=true` and `bootstrap_required=true`; asserted `task_created` precedes `map_created`. | Round 4 closure review |
| Rawls | Regression scripts could hide 0-test or stale-report success | blocking | accept | Prior script output confused filtered binaries with meaningful tests and could inspect stale scenario artifacts. | Regression script now requires `passedCount > 0` and is scoped to relevant packages; E2E script requires a current scenario report and `passedCount > 0`. | Round 4 closure review |
| Rawls | Automated E2E is scripted-provider, not a live paid-model CLI acceptance test | non-blocking | defer | Deterministic CI should keep scripted provider; live CLI runs are slower, costful, and less stable. | Documented this limitation in the review and testing docs. | Add live CLI acceptance separately when release gating is designed |

### Validation After Fixes

- Passed: `rustup run stable cargo test -p codex-core action_map::runtime --lib --locked --jobs 2` with 52 passing tests.
- Passed: `rustup run stable cargo test -p codex-protocol --lib --locked --jobs 2` with 179 passing tests.
- Passed: `rustup run stable cargo test -p codex-tools --lib --locked --jobs 2` with 134 passing, 1 ignored.
- Passed: `rustup run stable cargo test -p codex-core legacy_spawn_agent_rejects_before_taskspace_routing --lib --locked --jobs 2`.
- Passed: `rustup run stable cargo test -p codex-core legacy_spawn_agent_claims_taskspace_node_after_start_task --lib --locked --jobs 2`.
- Passed: `rustup run stable cargo test -p codex-core real_user_input_sets_taskspace_routing_gate_and_snapshot --lib --locked --jobs 2`.
- Passed: `rustup run stable cargo test -p codex-core record_context_updates_refreshes_taskspace_inventory_in_steady_state --lib --locked --jobs 2`.
- Passed: `rustup run stable cargo test -p codex-core reconstruct_history_restores_latest_map_runtime_snapshot --lib --locked --jobs 2`.
- Passed: `rustup run stable cargo test -p codex-core tools::handlers::multi_agents::tests::action_map --lib --locked --jobs 2` with 5 passing tests.
- Passed: `rustup run stable cargo test -p codex-tui action_map_commands_are_routed_through_app_server_in_tui --lib --locked --jobs 2`.
- Passed: `rustup run stable cargo test -p codex-core --test all action_map_scenario_evaluation --locked --jobs 2 -- --nocapture` with 2 passing tests.
- Passed: `.\scripts\run-action-map-regression.ps1`; report `D:\whalecode-alpha\target\test-reports\action-map-20260528-011703-735\report.md`, 60 passed, 0 failed.
- Passed: `.\scripts\run-action-map-e2e-scenario.ps1`; report `D:\whalecode-alpha\target\test-reports\action-map-e2e-20260528-011722-235\report.md`, 1 passed, 0 failed.
- Passed: `git diff --check`.

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 4 implementation passed
  - Round 5 test-validity passed
  - Round 6 final hardening passed
- Blocking re-review launch records:
  - Averroes / `019e6a72-6f03-7f62-a50d-2324e2407ce3`
  - Peirce / `019e6a72-a766-7d50-9634-ab62aa23931b`
  - Raman / `019e6a7e-b957-72f0-b0b2-6c64f0b7c192`
  - Jason / `019e6a85-8307-7e03-a9e0-2db7026300cb`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Allowed to proceed: yes

## Round 4: Per-Turn Routing Gate Closure Review

### Review Input

#### Objective
Verify that the accepted Round 3 blocking findings are actually closed. The system must enforce explicit agent routing/start on every real user turn, avoid runtime-created default maps, persist gate state for viewer/replay, and gate both legacy and v2 subagent spawn paths.

#### Review Target
Closure review after fixes for TaskSpace per-turn routing gate, reborn request semantics, legacy spawn gating, snapshot compatibility, deterministic E2E assertions, and regression scripts.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/session/handlers.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/session/tests.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents/spawn.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_tests.rs`
- `third_party/codex-cli/codex-rs/core/tests/suite/action_map_scenario_evaluation.rs`
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs`
- `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`
- `third_party/codex-cli/codex-rs/tui/src/app/action_map_viewer.rs`
- `scripts/run-action-map-regression.ps1`
- `scripts/run-action-map-e2e-scenario.ps1`
- `docs/testing/2026-05-08-action-map-e2e-scenario.md`

#### Change Introduction
The latest changes move routing-gate activation to the real session user-turn boundary, immediately emit a snapshot when the gate is set, keep `/task-reborn` as a pending request rather than a runtime map factory, add serde defaults for new snapshot fields, gate legacy spawn, update stale multi-agent tests to use `start_task`, require E2E empty initial TaskSpace snapshots, and harden scripts against 0-test/stale-report false positives.

#### Risk Focus
- A real user turn must set the routing gate; internal context refresh should not.
- Ordinary tools and both spawn implementations must fail before `start_task` or `route_task`.
- Enabling TaskSpace and requesting reborn must not create a default map.
- Snapshot/restore must preserve gate/reborn flags and remain compatible with old snapshots.
- The E2E scenario must prove `task_created` precedes `map_created`.
- Script reports must not pass on 0 matching tests or stale scenario artifacts.
- The implementation must reuse ActionMap/Session/tool infrastructure instead of creating a parallel runtime.

#### Verification Status
- Passed all validation listed in Round 3 "Validation After Fixes".

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Focus on whether the accepted Round 3 blockers are closed.
- Cite evidence paths and line numbers when possible.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | Closure depends on runtime/session/spawn correctness. | gate state, tool blocking, legacy spawn, reborn |
| test-validity-adversary | The user explicitly challenged weak tests and demanded real-path validation. | false positives, E2E assertions, script reports |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1.spawn_agent` fresh internal subagent | `019e6a72-6f03-7f62-a50d-2324e2407ce3` / Averroes | spawn tool result in current Codex thread | no | Round 4 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| test-validity-adversary | `multi_agent_v1.spawn_agent` fresh internal subagent | `019e6a72-a766-7d50-9634-ab62aa23931b` / Peirce | spawn tool result in current Codex thread | no | Round 4 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### implementation-adversary / Averroes

##### Summary
Accepted Round 3 implementation blockers appear closed by read-only code inspection. No remaining implementation blocker was found.

##### Blocking Findings
- None.

##### Non-blocking Risks
- `/taskspace` mode enable emits `mode_changed` but not an immediate `snapshot_updated`; replay can infer gate state from mode fallback, but event-only observers wait for a later snapshot.
- `taskspace_control` schema remains `strict: false` and does not express per-action conditional required fields, though handler/runtime validation exists.
- Rejected route/work/spawn attempts return model-visible errors but do not emit structured rejection events.

##### Required Fixes
- None for Round 3 implementation closure.

##### Missing Tests
- No direct v2 handler negative test for routing-before-spawn at the time of review.
- No assertion around whether `/taskspace` enable emits `snapshot_updated`.

##### Missing Logs / Observability
- No explicit `routing_required_set` or rejection event; state is visible through snapshots.

##### Evidence
- Real user turns set the gate after real turn context creation and emit a snapshot when changed.
- Ordinary tools validate routing, barrier, and existing main binding without auto-create/auto-bind.
- Legacy and v2 spawn both use the same session assignment gate.
- `/task-reborn` requests reborn without creating a map.
- Snapshot fields have serde defaults and restore copies gate/reborn flags.
- Viewer exposes route/bootstrap/reborn status from read snapshot.

##### Closure Verdict
passed for implementation blockers.

#### test-validity-adversary / Peirce

##### Summary
Round 3 test-validity blockers were only partially closed. E2E empty graph, event order, script false-positive hardening, and old-path documentation cleanup were mostly closed. Two blockers remained.

##### Blocking Findings
- `real_user_input_sets_taskspace_routing_gate_and_snapshot` asserted live in-memory snapshot state but did not consume the event stream or prove `snapshot_updated` was emitted.
- v2 spawn lacked a direct handler-level negative test proving `SpawnAgentHandlerV2` rejects before routing/start_task.

##### Non-blocking Risks
- E2E remains scripted-provider, not live paid-model CLI acceptance.
- E2E stale-report detection based on report timestamp is adequate for ordinary stale reports but can still be affected by concurrent runs.

##### Required Fixes
- Consume the session event receiver in the real user input test and assert `MapRuntimeEvent::SnapshotUpdated` with `routing_required=true`.
- Add `v2_spawn_agent_rejects_before_taskspace_routing`.

##### Missing Tests
- `Op::UserTurn` boundary could also be covered in addition to `Op::UserInput`.
- E2E did not require `snapshot_updated` at the time of review.

##### Missing Logs / Observability
- Scenario report should include initial snapshot flags and `snapshot_updated` count.

##### Evidence
- Existing E2E asserted initial empty tasks/maps and `task_created` before `map_created`.
- Regression/E2E scripts required passed tests and current scenario reports.
- Testing document no longer described runtime-created default maps as the valid path.

##### Closure Verdict
not closed for test-validity blockers.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Averroes | Implementation blockers closed | closure | accept | Code inspection found no remaining implementation blocker. | No implementation change required from this finding. | None |
| Averroes | `/taskspace` enable lacks immediate `snapshot_updated` | non-blocking | defer | Current mode event plus read snapshot/viewer polling expose state; this is observability polish beyond the accepted blocker. | Deferred. | Revisit if event-only viewer/replay consumers need immediate mode-enable snapshot |
| Averroes | `taskspace_control` schema remains `strict: false` | non-blocking | defer | Runtime/handler validation is authoritative; schema tightening is useful but not required for this gate slice. | Deferred. | Consider when tool schema contract is revisited |
| Averroes | Rejection attempts lack structured events | non-blocking | defer | Model-visible errors are sufficient for correctness; structured rejection telemetry is useful later. | Deferred. | Add if diagnosing route failures becomes hard |
| Peirce | Real user input test did not prove `snapshot_updated` emission | blocking | accept | The test used `_rx` and only checked live runtime state. | Updated the test to consume the event receiver and assert `MapRuntimeEvent::SnapshotUpdated` with `routing_required=true` and `bootstrap_required=false`. | Round 5 closure review |
| Peirce | v2 spawn lacked before-routing handler negative test | blocking | accept | Legacy negative test existed, v2 only had post-start assignment coverage. | Added `v2_spawn_agent_rejects_before_taskspace_routing`, asserting model-visible TaskSpace gate error and no captured child ops. | Round 5 closure review |
| Peirce | E2E did not require `snapshot_updated` and report lacked snapshot flags | non-blocking | accept | This improves evidence quality for future investigation. | Added `snapshot_updated` event assertion and scenario report fields for initial routing/bootstrap flags, initial task/map counts, and snapshot event count. | Round 5 closure review |

### Validation After Round 4 Fixes

- Passed: `rustup run stable cargo test -p codex-core real_user_input_sets_taskspace_routing_gate_and_snapshot --lib --locked --jobs 2`.
- Passed: `rustup run stable cargo test -p codex-core v2_spawn_agent_rejects_before_taskspace_routing --lib --locked --jobs 2`.
- Passed: `rustup run stable cargo test -p codex-core --test all action_map_scenario_evaluation --locked --jobs 2 -- --nocapture` with 2 passing tests.

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Allowed to proceed: yes

## Round 5: Test-Validity Closure Review

### Review Input

#### Objective
Verify that the two accepted Round 4 test-validity blockers are closed: real user input now proves `snapshot_updated` emission, and v2 spawn now has a direct before-routing negative handler test.

#### Review Target
Closure review after adding event-stream snapshot assertions, v2 spawn negative coverage, and stronger E2E/report evidence.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/session/tests.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_tests.rs`
- `third_party/codex-cli/codex-rs/core/tests/suite/action_map_scenario_evaluation.rs`
- `vs_review/2026-05-27-taskspace-routing-review.md`

#### Change Introduction
The real user input test now consumes the session event receiver and asserts a `snapshot_updated` payload with routing gate flags. A new v2 spawn negative test calls `SpawnAgentHandlerV2` with TaskSpace enabled but no routing/start_task and asserts it fails before child ops are captured. The E2E scenario now requires `snapshot_updated` and writes initial snapshot flags plus snapshot event count to the scenario report.

#### Risk Focus
- The event assertion must prove the session emission path, not just live state.
- The v2 test must call the actual handler and fail before spawn side effects.
- The scenario assertion/report changes must not be cosmetic only.

#### Verification Status
- Passed all validation listed in Round 4 "Validation After Round 4 Fixes".

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Focus on whether Round 4 accepted blockers are closed.
- Cite evidence paths and line numbers when possible.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| test-validity-adversary | Round 4 blockers were test-validity specific. | false-positive tests, handler-level coverage, event persistence evidence |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| test-validity-adversary | `multi_agent_v1.spawn_agent` fresh internal subagent | `019e6a7e-b957-72f0-b0b2-6c64f0b7c192` / Raman | spawn tool result in current Codex thread | no | Round 5 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### test-validity-adversary / Raman

##### Summary
The two accepted Round 4 test-validity blockers are closed. The session test now consumes the event receiver and asserts `SnapshotUpdated`; the v2 handler negative test now directly verifies before-routing rejection. E2E now requires `snapshot_updated`, and the realistic scenario report writes initial gate flags and snapshot count.

##### Blocking Findings
- None.

##### Non-blocking Risks
- The basic E2E scenario also asserts `snapshot_updated`, but its report still lacked initial snapshot flags and snapshot count at review time.
- `Op::UserTurn` still lacked a separate real-user-turn gate/snapshot test at review time; `Op::UserInput` was covered.

##### Required Fixes
- None for the accepted Round 4 blockers.

##### Missing Tests
- Add `Op::UserTurn` version of the real user turn gate/snapshot test if we want both user input variants covered.

##### Missing Logs / Observability
- No blocker. `snapshot_updated` is asserted and the realistic report records gate evidence.

##### Evidence
- `session/tests.rs` consumes `rx.recv()` and matches `EventMsg::MapRuntime(MapRuntimeEvent::SnapshotUpdated(payload))`.
- `multi_agents_tests.rs` contains `v2_spawn_agent_rejects_before_taskspace_routing`, which calls `SpawnAgentHandlerV2`, asserts model-facing TaskSpace gate error, and checks `captured_ops()` is empty.
- Both E2E scenarios assert `snapshot_updated`; the realistic report includes initial routing/bootstrap flags, task/map counts, and snapshot event count.

##### Closure Verdict
passed for the two accepted Round 4 blockers.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Raman | Round 4 blockers are closed | closure | accept | The reviewer confirmed event-stream snapshot assertion and v2 before-routing handler coverage. | No further blocker fix required. | None |
| Raman | Basic E2E report lacked initial snapshot flags/count | non-blocking | accept | The basic scenario already asserted `snapshot_updated`; report evidence should be consistent with the realistic scenario. | Updated `write_basic_artifacts` to include initial routing/bootstrap flags, initial task/map counts, and snapshot event count. | Round 6 final hardening review |
| Raman | `Op::UserTurn` boundary lacked a separate test | non-blocking | accept | `Op::UserTurn` and `Op::UserInput` share handler plumbing but are different protocol variants. | Added `real_user_turn_sets_taskspace_routing_gate_and_snapshot`, covering `Op::UserTurn` event-stream snapshot emission. | Round 6 final hardening review |

### Validation After Round 5 Follow-ups

- Passed: `rustup run stable cargo test -p codex-core real_user_turn_sets_taskspace_routing_gate_and_snapshot --lib --locked --jobs 2`.
- Passed: `rustup run stable cargo test -p codex-core --test all action_map_scenario_evaluation --locked --jobs 2 -- --nocapture` with 2 passing tests.

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Allowed to proceed: yes

## Round 6: Final Test-Hardening Review

### Review Input

#### Objective
Verify the final non-blocking hardening applied after Round 5: `Op::UserTurn` now has explicit gate/snapshot coverage, and the basic E2E report now records the same initial TaskSpace snapshot evidence as the realistic report.

#### Review Target
Final read-only test-validity review for the latest follow-up changes.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/session/tests.rs`
- `third_party/codex-cli/codex-rs/core/tests/suite/action_map_scenario_evaluation.rs`
- `vs_review/2026-05-27-taskspace-routing-review.md`

#### Change Introduction
Added `real_user_turn_sets_taskspace_routing_gate_and_snapshot`, which sends `Op::UserTurn`, consumes the session event stream, and asserts `SnapshotUpdated` gate flags. Updated the basic scenario report to include initial routing/bootstrap flags, initial task/map counts, and `snapshot_updated` count.

#### Risk Focus
- The new `Op::UserTurn` test must use the real handler path and event stream.
- The basic report fields must be sourced from the real initial snapshot and timeline, not constants.
- The report must accurately reflect review closure and not hide remaining blockers.

#### Verification Status
- Passed all validation listed in Round 5 "Validation After Round 5 Follow-ups".

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Report whether any blocker remains.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| test-validity-adversary | Latest changes are test/report evidence hardening. | false-positive tests, report evidence quality |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| test-validity-adversary | `multi_agent_v1.spawn_agent` fresh internal subagent | `019e6a85-8307-7e03-a9e0-2db7026300cb` / Jason | spawn tool result in current Codex thread | no | Round 6 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### test-validity-adversary / Jason

##### Summary
The two Round 5 follow-up hardenings pass read-only review. `Op::UserTurn` now uses the real handler path, consumes the session event stream, and asserts `SnapshotUpdated` gate flags. The basic scenario report fields are sourced from `initial_snapshot` and `timeline`, not constants.

##### Blocking Findings
- None.

##### Non-blocking Risks
- The Round 6 report section was still pending at review time and needed bookkeeping after incorporating this review.

##### Required Fixes
- Update Round 6 reviewer output, main response, closure status, and final conclusion.

##### Missing Tests
- None for the requested scope.

##### Missing Logs / Observability
- No blocker. Basic report now writes initial routing/bootstrap flags, initial task/map counts, and `snapshot_updated_events`.

##### Evidence
- `session/tests.rs` contains `real_user_turn_sets_taskspace_routing_gate_and_snapshot`, calls `handlers::user_input_or_turn` with `Op::UserTurn`, consumes `rx.recv()`, matches `MapRuntimeEvent::SnapshotUpdated`, and asserts `routing_required=true` / `bootstrap_required=false`.
- `action_map_scenario_evaluation.rs` passes the real `initial_snapshot` into `write_basic_artifacts`; the basic report writes flags/counts from `initial_snapshot` and `snapshot_updated_events` from `count_event(timeline, "snapshot_updated")`.

##### Closure Verdict
passed.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Jason | Round 5 follow-up hardenings pass | closure | accept | Reviewer confirmed both `Op::UserTurn` event-stream coverage and basic scenario report evidence are correct. | No code change required. | None |
| Jason | Round 6 report still pending | non-blocking | accept | This was expected before incorporating the reviewer output. | Updated Round 6 launch record, reviewer output, main response, closure status, and final conclusion. | None |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Allowed to proceed: yes

## Final Conclusion

TaskSpace per-turn routing gate implementation and test hardening passed subagent-vs-review closure. No unresolved blocking findings remain.

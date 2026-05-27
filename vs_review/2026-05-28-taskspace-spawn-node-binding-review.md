# Subagent VS Review: TaskSpace Spawn Node Binding

- Created: 2026-05-28T02:03:33.7629009+08:00
- Updated: 2026-05-28T02:52:00+08:00
- Task: TaskSpace Phase 4 implementation for explicit subagent node binding.
- Report path: `vs_review/2026-05-28-taskspace-spawn-node-binding-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: closed

## Round 1: Implementation And Test Review

### Review Input

#### Objective
Ensure `spawn_agent` in TaskSpace mode can explicitly bind a subagent to a selected node, while rejecting ambiguous spawns when multiple ready nodes exist and no `node_id` is supplied.

#### Review Target
Code implementation, tool schema, prompt/developer-context guidance, and test coverage for TaskSpace subagent node binding.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents/spawn.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_tests.rs`
- `third_party/codex-cli/codex-rs/tools/src/agent_tool.rs`
- `third_party/codex-cli/codex-rs/tools/src/agent_tool_tests.rs`
- `third_party/codex-cli/codex-rs/tools/src/tool_registry_plan_tests.rs`
- `scripts/run-action-map-regression.ps1`
- `scripts/run-action-map-e2e-scenario.ps1`

#### Change Introduction
The runtime now accepts an optional `node_id` for TaskSpace spawn assignment. If TaskSpace is off, spawn behavior remains unchanged. If TaskSpace is on, the runtime validates routing and barriers, then claims a ready, unleased node. Without `node_id`, exactly one ready node may be auto-claimed for compatibility; zero ready nodes or multiple ready nodes reject before child thread creation. Both legacy and v2 `spawn_agent` handlers pass the optional field through, and the tool schema exposes it.

#### Risk Focus
- Standard mode and legacy `spawn_agent` compatibility.
- Explicit `node_id` must not claim pending, running, blocked, completed, missing, or leased nodes.
- Multiple ready nodes must fail before child thread creation and without leaving a lease.
- Claim and lease creation must remain inside the existing session state lock.
- Developer context and tool schema must be sufficient for the model to know when `node_id` is required.
- Tests must cover actual handler paths, not only private runtime helpers.

#### Verification Status
- `cargo test -p codex-core action_map --lib --locked`: 65 passed.
- `cargo test -p codex-tools spawn_agent --lib --locked`: 3 passed.
- `cargo test -p codex-tools multi_agent_v2_uses_task_names --lib --locked`: 1 passed.
- Full regression scripts and real CLI E2E have not yet been rerun for this slice.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Report Summary, Blocking Findings, Non-blocking Risks, Required Fixes, Missing Tests, Missing Logs / Observability, and Evidence.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | Multi-module runtime/tool-handler change affects state flow and lease semantics. | Correctness, compatibility, atomicity |
| test-validity-adversary | The user explicitly requires meaningful coverage and rejects mock-only confidence. | Test coverage, real-path validation gaps |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1.spawn_agent` explorer | `019e6a9a-b73d-7671-ad1f-eb8a3324d121` | spawn_agent tool result, nickname `Anscombe` | no | Round 1 Review Input plus implementation-specific risk list | main-agent history, reasoning, drafts, conclusions, full diff persuasion | yes |
| test-validity-adversary | `multi_agent_v1.spawn_agent` explorer | `019e6a9a-ff2d-7bc0-a5da-14609e62edd6` | spawn_agent tool result, nickname `Dirac` | no | Round 1 Review Input plus test-validity-specific risk list | main-agent history, reasoning, drafts, conclusions, full diff persuasion | yes |

### Reviewer Outputs

#### implementation-adversary (`019e6a9a-b73d-7671-ad1f-eb8a3324d121`)

Summary:
- Node selection semantics are mostly correct: standard mode remains unchanged, explicit `node_id` validation rejects non-ready nodes, and ambiguous multi-ready spawns reject before child spawn.

Blocking findings:
- `accept`: lease attach was not race-safe. `spawn_agent_with_metadata` sends the child its initial input before returning to the handler, while `record_child_result` only finds leases that already have `agent_thread_id`. A fast child could finish before `attach_action_map_assignment`, causing the result notification to miss the lease and leave the node running.

Required fixes:
- Bind or recover the lease/result ordering gap.
- Add deterministic coverage for the fast-child result path.

Missing tests:
- Handler-level multiple-ready rejection should prove no child op and no lease remain.
- Legacy `spawn_agent` needs explicit `node_id` handler coverage.
- Explicit `node_id` rejection should cover running, blocked, completed, and leased nodes.
- Prompt/schema guidance should prove the model can discover `node_id`.

Missing logs / observability:
- Add structured logging for spawn-assignment rejection and late final-status recovery checks.

Evidence cited by reviewer:
- `third_party/codex-cli/codex-rs/core/src/agent/control.rs`: `spawn_agent_with_metadata` sends input before returning.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`: `record_child_result` resolves by `find_lease_by_thread`.
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents/spawn.rs` and `multi_agents_v2/spawn.rs`: assignment attach happened after spawn return.

#### test-validity-adversary (`019e6a9a-ff2d-7bc0-a5da-14609e62edd6`)

Summary:
- Existing tests are real unit/handler tests, not fake assertions, but the first slice did not justify claiming full Phase 4 coverage.

Blocking findings:
- `accept`: legacy v1 handler parsed and passed `node_id`, but lacked multiple-ready rejection and explicit `node_id` success tests.
- `accept`: the regression script default path only ran `codex-core action_map`; it did not run `codex-tools` spawn/schema tests.

Required fixes:
- Add legacy v1 handler tests for multi-ready rejection and explicit `node_id`.
- Update regression script default matrix to include `codex-tools` spawn and registry/schema paths.
- Add handler rejection state assertions: no child op, no lease, statuses unchanged.

Missing tests:
- v2 explicit pending-node rejection.
- hidden-metadata v2 output mode with explicit `node_id`.
- E2E scenario that forces multiple ready nodes and then retries with an explicit node.

Missing logs / observability:
- Rejections currently surfaced to the model but did not emit a local diagnostic log line.

Evidence cited by reviewer:
- `scripts/run-action-map-regression.ps1`: default package/filter only covered `codex-core action_map`.
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_tests.rs`: legacy single-ready TaskSpace test existed, but no legacy multi-ready/explicit tests existed.

### Main Agent Response

- `accept`: lease attach race. Added `Session::record_final_action_map_child_result_if_needed`, called by both legacy and v2 spawn handlers immediately after `attach_action_map_assignment`. The helper re-reads the child status after the lease is attached and records the node result if the child has already reached a final status. It also logs the post-attach final-status check.
- `accept`: deterministic race coverage. Added `child_result_can_be_recorded_after_late_lease_attach`, which proves a pre-attach result miss can be recovered after the lease gets the child thread id.
- `accept`: legacy handler coverage. Added `legacy_spawn_agent_requires_node_id_for_multiple_ready_nodes` and `legacy_spawn_agent_claims_explicit_node_id`.
- `accept`: v2 handler state coverage. Expanded multi-ready rejection to assert no child op, no lease, and unchanged ready statuses. Added explicit pending-node rejection and hidden-metadata explicit-node tests.
- `accept`: runtime status coverage. Added explicit requested-node rejection coverage for blocked, completed, running, and already leased nodes.
- `accept`: prompt/schema guidance. Added tool-description and parameter-description tests for `node_id`, and updated v1/v2 `spawn_agent` descriptions to mention the multi-ready TaskSpace requirement.
- `accept`: regression script gap. Updated `scripts/run-action-map-regression.ps1` default matrix to run `codex-core action_map`, `codex-tools spawn_agent`, `codex-tools multi_agent_v2_uses_task_names`, and `codex-tools tool_registry_plan`, with per-run report rows and per-run pass enforcement.

Validation after fixes:
- `cargo fmt --all`: passed with existing stable rustfmt warnings for unstable `imports_granularity`.
- `cargo test -p codex-core action_map --lib --locked`: 69 passed, 0 failed.
- `cargo test -p codex-tools spawn_agent --lib --locked`: 3 passed, 0 failed.
- `cargo test -p codex-tools multi_agent_v2_uses_task_names --lib --locked`: 1 passed, 0 failed.
- `cargo test -p codex-tools tool_registry_plan --lib --locked`: 40 passed, 0 failed, 1 ignored.
- `.\scripts\run-action-map-regression.ps1`: PASS, 4 runs, 113 total passed, 0 failed, 0 relevant Windows crash events. Report: `D:\whalecode-alpha\target\test-reports\action-map-20260528-022201-833\report.md`.

## Closure Round 1: Accepted Blocking Fix Review

### Closure Review Input

#### Objective
Verify that accepted blocking findings from Round 1 were fixed without introducing a new correctness or test-coverage blocker.

#### Review Target
Implementation and test closure for TaskSpace `spawn_agent` explicit node binding.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents/spawn.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_tests.rs`
- `third_party/codex-cli/codex-rs/tools/src/agent_tool.rs`
- `third_party/codex-cli/codex-rs/tools/src/agent_tool_tests.rs`
- `third_party/codex-cli/codex-rs/tools/src/tool_registry_plan_tests.rs`
- `scripts/run-action-map-regression.ps1`

#### Risk Focus
- Fast-child result race after lease attachment.
- Legacy v1 handler coverage.
- v2 handler coverage for explicit `node_id`, multiple-ready rejection, pending-node rejection, and hidden-metadata output mode.
- Tool schema and registry visibility for `node_id`.
- Whether the default regression command proves the intended paths ran.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files and reports directly.
- Do not modify files.
- Cite evidence paths and line numbers where possible.

### Closure Review Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-closure | `multi_agent_v1.spawn_agent` explorer | `019e6ab0-cbdc-76d3-8ea0-f085d4816ce6` | spawn_agent tool result, nickname `Lovelace`; close_agent returned completed review output | no | Closure Round 1 input plus implementation race focus | main-agent history, reasoning, drafts, conclusions, full diff persuasion | yes |
| test-closure | `multi_agent_v1.spawn_agent` explorer | `019e6ab1-1fd7-7e83-8243-3ce447f537ba` | spawn_agent tool result, nickname `Turing`; close_agent returned completed review output | no | Closure Round 1 input plus test-regression focus | main-agent history, reasoning, drafts, conclusions, full diff persuasion | yes |

### Closure Reviewer Outputs

#### implementation-closure (`019e6ab0-cbdc-76d3-8ea0-f085d4816ce6`)

Summary:
- No blocking issue found.
- The handler call sites attach the TaskSpace lease after child creation and then invoke post-attach final-status recovery.
- Runtime result recording removes the lease on first result, so the late recovery path does not create an obvious duplicate-result hazard.

Blocking findings:
- None.

Non-blocking risks:
- Fast-child race coverage is runtime-level rather than full handler/AgentControl integration coverage.
- Hidden metadata mode was covered in the v2 handler, but the reviewer suggested also proving schema exposure when metadata fields are hidden.

Required fixes:
- None blocking.

Missing tests:
- Optional duplicate-suppression test for repeated `record_child_result` on the same attached child.
- Optional hidden-metadata schema test proving `node_id` remains exposed.

Missing logs / observability:
- No blocking gap. Rejection and post-attach final-status checks have debug logs.

Evidence cited by reviewer:
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`: lease assignment, node validation, and result recording.
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`: post-attach final-status helper.
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents/spawn.rs`: legacy handler attach then recovery.
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs`: v2 handler attach then recovery.
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_tests.rs`: legacy and v2 handler coverage.

#### test-closure (`019e6ab1-1fd7-7e83-8243-3ce447f537ba`)

Summary:
- Real handler/runtime path tests exist, but the default regression script still did not execute the new legacy v1 handler tests in the report reviewed by this subagent.

Blocking findings:
- `accept`: default regression did not run legacy v1 handler tests. The script used only the `action_map` filter for `codex-core`, so the reviewed report could not prove `legacy_spawn_agent_*` tests ran.

Non-blocking risks:
- No real CLI/E2E scenario yet exercises multiple ready nodes followed by explicit `node_id` retry.
- Runtime rejection tests could add stronger state/lease unchanged assertions.

Required fixes:
- Add a default regression matrix entry for `codex-core --lib legacy_spawn_agent`.
- Rerun `scripts/run-action-map-regression.ps1`.
- Confirm the generated report and stdout log contain the legacy handler test names.

Missing tests:
- Blocking: default regression lacked legacy v1 handler execution evidence.
- Non-blocking: no node-specific CLI/E2E scenario.

Missing logs / observability:
- No blocking logging gap.

Evidence cited by reviewer:
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_tests.rs`: legacy handler tests existed.
- `scripts/run-action-map-regression.ps1`: previous default matrix did not include `legacy_spawn_agent`.
- `target/test-reports/action-map-20260528-022201-833/report.md`: previous report did not prove legacy v1 execution.

### Main Agent Response To Closure Round 1

- `accept`: default regression missing legacy v1 handler execution evidence. Added a `core-legacy-spawn-agent` default run to `scripts/run-action-map-regression.ps1`, using `cargo test -p codex-core --lib legacy_spawn_agent --locked --jobs 2`.
- `accept`: runtime mutation assertions. Added explicit assertions that rejected requested-node cases do not create a lease and leave node status unchanged.
- `accept`: duplicate result suppression. Added coverage proving a duplicate child result after lease release is ignored.
- `accept`: hidden-metadata schema exposure. Added a codex-tools schema test proving `node_id` remains exposed when hidden metadata output mode is used.
- `defer`: node-specific real CLI/E2E scenario. This remains useful for a later end-to-end suite, but the closure blocker was default regression coverage of existing handler tests.

Validation after Closure Round 1 fixes:
- `.\scripts\run-action-map-regression.ps1`: PASS, 5 runs, 119 total passed, 0 failed, 0 relevant Windows crash events. Report: `D:\whalecode-alpha\target\test-reports\action-map-20260528-023529-704\report.md`.
- The latest report includes `core-legacy-spawn-agent` with filter `legacy_spawn_agent`, 4 passed tests, and nonzero matched binaries.
- The latest legacy stdout log includes:
  - `legacy_spawn_agent_rejects_before_taskspace_routing ... ok`
  - `legacy_spawn_agent_requires_node_id_for_multiple_ready_nodes ... ok`
  - `legacy_spawn_agent_claims_taskspace_node_after_start_task ... ok`
  - `legacy_spawn_agent_claims_explicit_node_id ... ok`

Because Closure Round 1 accepted a blocking finding, a second fresh closure review is required by the `subagent-vs-review` rules.

## Closure Round 2: Regression Matrix Closure Review

### Closure Review Input

#### Objective
Verify that the accepted Closure Round 1 blocker is now closed: default regression must execute legacy v1 handler tests and the generated report must prove nonzero matching passed tests.

#### Review Target
Regression script matrix, latest regression report, latest legacy stdout log, and legacy handler test coverage.

#### Target Locations
- `scripts/run-action-map-regression.ps1`
- `target/test-reports/action-map-20260528-023529-704/report.md`
- `target/test-reports/action-map-20260528-023529-704/cargo-test-core-legacy-spawn-agent.stdout.log`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_tests.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents/spawn.rs`

#### Risk Focus
- Default regression includes a legacy v1 handler run.
- Report proves nonzero matching legacy v1 tests.
- Legacy tests cover multiple-ready rejection and explicit `node_id` success.
- No closure-specific blocker remains.

### Closure Round 2 Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| regression-matrix-closure | `multi_agent_v1.spawn_agent` explorer | `019e6abd-9ca7-7762-979d-bee0fd915272` | spawn_agent tool result, nickname `Mendel` | no | Closure Round 2 input plus regression-matrix focus | main-agent history, reasoning, drafts, conclusions, full diff persuasion | yes |

### Closure Round 2 Reviewer Output

#### regression-matrix-closure (`019e6abd-9ca7-7762-979d-bee0fd915272`)

Summary:
- Blocker is closed.
- The default regression matrix includes `core-legacy-spawn-agent`.
- The latest report proves the legacy run executed with 4 passed tests.
- Source tests cover both legacy multiple-ready rejection and explicit `node_id` success.

Blocking findings:
- None.

Non-blocking risks:
- Individual legacy test names are visible in the stdout log rather than summarized directly in `report.md`.

Required fixes:
- None.

Missing tests:
- None for this closure target.

Missing logs / observability:
- None blocking. The report links the stdout log and enforces nonzero matched tests.

Evidence cited by reviewer:
- `scripts/run-action-map-regression.ps1`: default matrix includes legacy v1 run and rejects empty filtered runs.
- `target/test-reports/action-map-20260528-023529-704/report.md`: 5 runs, PASS, 119 total passed, and `core-legacy-spawn-agent` with 4 passed tests.
- `target/test-reports/action-map-20260528-023529-704/cargo-test-core-legacy-spawn-agent.stdout.log`: four named legacy tests ran and passed.
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_tests.rs`: legacy multiple-ready rejection and explicit `node_id` success tests.
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents/spawn.rs`: legacy handler accepts and passes `node_id`.

### Main Agent Response To Closure Round 2

- `accept`: non-blocking report readability risk. No code change required because the report links the per-run stdout log and the script enforces nonzero matched tests. This can be improved later by summarizing matched test names in the report.
- No blocking finding remains.

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Closure Round 1
  - Closure Round 2
- Blocking re-review launch records:
  - `019e6ab0-cbdc-76d3-8ea0-f085d4816ce6`
  - `019e6ab1-1fd7-7e83-8243-3ce447f537ba`
  - `019e6abd-9ca7-7762-979d-bee0fd915272`
- Rejected findings backed by evidence: none rejected
- Deferred findings documented: node-specific CLI/E2E scenario deferred as non-blocking future coverage
- Allowed to proceed: yes

## Final Conclusion

TaskSpace spawn node binding review is closed. All accepted blocking findings were fixed and received fresh internal subagent closure review.

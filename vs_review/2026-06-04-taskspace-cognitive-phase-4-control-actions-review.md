# Subagent VS Review: TaskSpace Cognitive Phase 4 Control Actions

- Created: 2026-06-04T21:56:00+08:00
- Updated: 2026-06-04T22:29:00+08:00
- Report schema: adversarial-v1
- Task: Implement TaskSpace cognitive-state MVP control actions while reusing the existing ActionMap/TaskSpace runtime.
- Report path: `vs_review/2026-06-04-taskspace-cognitive-phase-4-control-actions-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: passed

## Round 1: Phase 4A Implementation Review

### Review Input

#### Objective
Validate the Phase 4A implementation that lets the main agent record TaskSpace output contracts, fact sources, active facts, and result validity/evidence packages without creating a second authoritative state system.

#### Review Target
Code implementation, protocol/schema compatibility, runtime state boundaries, validation coverage, and documentation accuracy.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/action_map/cognitive.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs`
- `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs`
- `third_party/codex-cli/codex-rs/app-server-protocol/tests/schema_fixtures.rs`
- `third_party/codex-cli/codex-rs/app-server-protocol/schema/typescript/ActionMapSnapshotEvidenceRef.ts`
- `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md`
- Verification report: `target/test-reports/action-map-20260604-215229-719/report.md`

#### Change Introduction
Phase 4A extends the existing `taskspace_control` tool and `ActionMapRuntime` with four MVP cognitive actions:

- `record_output_contract`
- `record_fact_source`
- `record_fact`
- `mark_result_validity`

The intended design keeps authoritative state in `TaskState.cognitive_state` and `NodeResult.evidence_package`. Runtime events are minimal reference notifications only. `EvidenceRef` now supports `fact_source_id` / `factSourceId`.

#### Risk Focus
- A hidden second state source or event payload becoming authoritative.
- Unaccepted, questioned, invalid, inferred, unknown, or test-generated data being promoted to active facts.
- Tool schema allowing malformed payloads that silently degrade into natural-language summaries.
- Protocol/generated TypeScript/JSON schema drifting from Rust protocol types.
- Existing TaskSpace node/lease/routing behavior being regressed by cognitive actions.
- Tests proving only helper self-consistency rather than production runtime paths.

#### Assumptions To Attack
- The main agent cannot record cognitive state outside TaskSpace experiment mode or outside its owned active task.
- Every record that claims authority has stable IDs and mechanical evidence refs.
- `record_fact` cannot use untrusted provenance or unaccepted results.
- `mark_result_validity=accepted` cannot succeed without both top-level evidence refs and claim evidence refs.
- Minimal runtime events do not leak raw claims/descriptions/evidence packages and do not become a parallel source of truth.
- Generated app-server schema exposes `factSourceId` while preserving legacy compatibility.

#### Adversarial Lenses
- implementation
- architecture
- state
- input
- failure
- data
- maintenance
- testing
- observability

#### Verification Status
- `rustup run stable cargo fmt --all` completed with existing stable-rustfmt warnings for nightly-only import options.
- `git diff --check` passed with CRLF conversion warnings only.
- `rustup run stable cargo test -p codex-core action_map::runtime::tests --lib --locked` passed: 119 tests.
- `rustup run stable cargo test -p codex-protocol action_map_snapshot --lib --locked` passed: 7 tests.
- `rustup run stable cargo test -p codex-protocol map_runtime_cognitive_events --lib --locked` passed: 1 test.
- `rustup run stable cargo test -p codex-tools taskspace_control --lib --locked` passed: 2 tests.
- `rustup run stable cargo test -p codex-app-server-protocol --test schema_fixtures --locked` passed: 4 tests.
- `.\scripts\run-action-map-regression.ps1` passed: 10 cargo runs, 3 script runs, 197 passed, 0 failed, 0 relevant crash events.
- Known gaps: viewer cognitive side panel, final artifact audit hard gate, sentinel clear action, prompt/developer context injection, promotion/collapse are intentionally not complete in Phase 4A.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Focus on blocking or high-impact issues; do not inflate style preferences into blocking findings.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 20 minutes | one bounded extension if the reviewer is active | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | The change touches runtime state transitions, tool input validation, and trust-boundary checks. | state, input, data, failure |
| architecture-adversary | The change must not create a parallel runtime or second authoritative state source, and must stay maintainable in an existing large ActionMap runtime. | architecture, maintenance, protocol boundary |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1.spawn_agent` | `019e92ec-7749-7923-a931-fccb7697368b` (`Meitner`) | spawn tool result | `fork_context=false` | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| architecture-adversary | `multi_agent_v1.spawn_agent` | `019e92ec-8b85-7f53-9bf8-238d9f3cc005` (`Dewey`) | spawn tool result | `fork_context=false` | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| implementation-round-1 | implementation-adversary | 1 | `019e92ec-7749-7923-a931-fccb7697368b` | ~20 minutes | completed | reviewer returned findings | completed |
| architecture-round-1 | architecture-adversary | 1 | `019e92ec-8b85-7f53-9bf8-238d9f3cc005` | ~20 minutes | completed | reviewer returned findings | completed |

### Reviewer Outputs

#### implementation-round-1

##### Summary
Implementation mostly reuses the existing `taskspace_control -> Session -> ActionMapRuntimeState -> MapRuntimeEvent/snapshot` path, and guards are mostly in runtime.

##### Blocking Findings
- `record_output_contract`, `record_fact_source`, and `record_fact` can write with an active task but no active map.
  - Broken assumption: Phase 4A cognitive writes require active task and active map.
  - Failure scenario: restored state can contain `active_task_id` without `active_map_id`; three cognitive record actions would still update task state.
  - Impact: task facts/contracts can be detached from the task path that should make them auditable.
  - Proof needed: runtime must reject all cognitive record actions without active map.
- Existing active facts can become backed by an invalid/questioned result after a later validity change.
  - Broken assumption: invalid/questioned results must not support active facts.
  - Failure scenario: record a fact from an accepted result, then later mark that result invalid/questioned; the active fact remains.
  - Impact: final audit can trust a fact whose evidence was later rejected.
  - Proof needed: result downgrade must reject while active facts cite the result, or facts must be explicitly invalidated.

##### Non-blocking Risks
- `taskspace_control` schema is permissive (`strict: false`, only `action` globally required); handler/runtime still reject bad payloads.
- `artifact_ref`, `validator_ref`, and `claim_id` evidence refs are normalized but not joined to a known object.
- Snapshot restore trusts current-schema cognitive refs without revalidating joins.

##### Required Fixes
- Require active task and active map for all four Phase 4A actions.
- On result downgrade from accepted to questioned/invalid, reject while active facts depend on that result or explicitly invalidate facts.
- Add restore-time invariant repair or validation for active task/map linkage.

##### Missing Tests
- Handler/session production-path tests for all four actions.
- Negative tests for standard mode, missing active task, missing active map, wrong owner, bad refs.
- Regression test for accepted-result fact followed by result downgrade.

##### Missing Logs / Observability
- Success events exist, but rejected cognitive writes return errors without structured rejection events.

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` - cognitive update context previously returned optional map.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` - `record_fact` only checked result validity at write time.
- `target/test-reports/action-map-20260604-215229-719/report.md` - pre-review regression was PASS.

#### architecture-round-1

##### Summary
The implementation mostly conforms: authority lives in existing runtime state, and no shadow result index was introduced. The main architecture issue was one strict minimal-event contract violation.

##### Blocking Findings
- `result_validity_changed` duplicated `validity_reason`, which is already stored in authoritative `NodeResult.evidence_package`.
  - Broken assumption: runtime events are minimal ref notifications and do not carry semantic state.
  - Failure scenario: event consumers can treat `validityReason` as authoritative state separate from snapshot.
  - Impact: second-source-of-truth drift.
  - Proof needed: remove `validity_reason` from the event and keep it only in evidence package/snapshot.

##### Non-blocking Risks
- `taskspace_control` is now a broad flat property bag with 10 actions; still acceptable for Phase 4A, but close to needing a dedicated cognitive-state tool.
- `claim_id` evidence refs are not join-validated.
- Runtime Event section in the plan listed several events not implemented in Phase 4A.

##### Required Fixes
- Remove or ref-ify `validity_reason` from `MapRuntimeResultValidityChangedEvent`.
- Align the Runtime Event section in the plan with actual Phase 4A scope.
- Either validate `claim_id` refs or document them as unchecked external refs until Phase 7 audit work.

##### Missing Tests
- No session-level test proving each cognitive action emits minimal event plus `SnapshotUpdated`.
- No negative test for non-existent `claim_id` refs.
- No handler-level JSON parsing tests for new cognitive actions.

##### Missing Logs / Observability
- Failed cognitive control attempts are not emitted as structured audit events.
- `ResultValidityChanged` has no previous validity; transition debugging requires snapshot diffing.

##### Evidence
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs` - event shape carried `validity_reason` before the fix.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` - runtime writes to existing task/result stores.
- `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md` - docs correctly defer viewer/audit/promotion/collapse, but event section was too broad.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| implementation-adversary | Cognitive records could write with active task but no active map | Cognitive state must remain bound to an auditable task path | blocking | accept | Shared context returned `Option<ActionMapId>` | Changed `active_task_context_for_cognitive_update` to require active map, task `active_map_id`, task `map_ids`, and map `task_id` consistency. Added `cognitive_control_actions_require_active_task_path`. | Closure review required |
| implementation-adversary | Active facts could remain backed by downgraded result | Invalid/questioned results must not support active facts | blocking | accept | `record_fact` checked at write time only | Added `active_fact_citing_result` guard; `mark_result_validity` rejects non-accepted validity while an active fact cites that result. Added downgrade regression test. | Closure review required |
| implementation-adversary | Restore-time invariant repair | Restored snapshot can have inconsistent active task/map linkage | blocking | accept-partial | Active cognitive writes now hard-reject inconsistent restored active path before mutation | Implemented active task/map consistency validation in the cognitive update path. Full restore repair remains deferred because it affects broader routing semantics. | Track as Phase 5/6 restore hygiene |
| architecture-adversary | `result_validity_changed` carried `validity_reason` | Event could become a second authoritative semantic state source | blocking | accept | `validity_reason` belongs in `NodeResult.evidence_package` | Removed `validity_reason` from `MapRuntimeResultValidityChangedEvent`, runtime emission, and protocol event test; test now asserts no `validityReason`. | Closure review required |
| architecture-adversary | Runtime Event docs over-claimed Phase 4A events | Future implementation could chase inaccurate event contract | non-blocking | accept | Plan listed events not yet implemented | Updated Runtime Event section to separate Phase 4A implemented events from Phase 6/7 or v1.1 events. | None |
| architecture-adversary | `claim_id` refs are not join-validated | Evidence refs can cite non-joinable claims | non-blocking | defer | `claim_id` may refer to a claim being introduced in the same package; strict validation needs a claim index/audit pass | Document as Phase 7 audit work; do not expand Phase 4A. | Add in Phase 7 audit design |
| both | Handler/session/log tests are thinner than runtime tests | Production path coverage could miss wrapper/parser issues | non-blocking | defer | Current regression covers runtime/tool schema/protocol; handler/session tests require async harness work | Keep as Phase 5/6 testing debt, not a blocker for Phase 4A after runtime guards. | Add session-level cognitive emission tests |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2
- Blocking re-review launch records:
  - implementation-adversary-closure `019e9305-8569-7940-8dca-73b94433caa0`
  - architecture-adversary-closure `019e9305-99c3-76e1-88e8-dcc461a20812`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Blocked reason: n/a
- Allowed to proceed: yes

### Round 1 Interim Conclusion

Round 1 found blocking issues. They were accepted, fixed, and sent to Round 2 closure review.

## Round 2: Blocking Closure Review

### Review Input

#### Objective
Verify that Round 1 accepted blocking findings were actually fixed without introducing a new state-boundary regression.

#### Review Target
Blocking closure for TaskSpace cognitive Phase 4A control actions.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs`
- `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md`
- `vs_review/2026-06-04-taskspace-cognitive-phase-4-control-actions-review.md`

#### Change Introduction
After Round 1, the implementation was changed to:

- require active task plus active map for cognitive record/update actions, including task/map ownership consistency checks;
- reject marking a result non-accepted while an active fact cites that result;
- remove `validity_reason` from `MapRuntimeResultValidityChangedEvent`;
- align the Runtime Event section of the engineering plan with actual Phase 4A scope.

#### Risk Focus
- The active-map guard still has bypasses.
- The result downgrade guard misses active facts or blocks legitimate accepted updates.
- `validityReason` still leaks into runtime event serialization.
- The doc still overstates Phase 4A completion.

#### Assumptions To Attack
- All four cognitive actions now require an active task path.
- Existing active facts cannot remain backed by a result after that result is downgraded.
- Runtime events remain minimal refs; semantic reason stays only in snapshot state.

#### Adversarial Lenses
- implementation
- architecture
- state
- testing

#### Verification Status
- `rustup run stable cargo test -p codex-core action_map::runtime::tests::cognitive_control --lib --locked` passed: 2 tests.
- `rustup run stable cargo test -p codex-core action_map::runtime::tests::mark_result_validity --lib --locked` passed: 2 tests.
- `rustup run stable cargo test -p codex-core action_map::runtime::tests::record_fact_rejects_untrusted_provenance_and_unaccepted_result --lib --locked` passed: 1 test.
- `rustup run stable cargo test -p codex-protocol map_runtime_cognitive_events --lib --locked` passed: 1 test.
- `rustup run stable cargo test -p codex-protocol action_map_snapshot --lib --locked` passed: 7 tests.
- `rustup run stable cargo test -p codex-tools taskspace_control --lib --locked` passed: 2 tests.
- `rustup run stable cargo test -p codex-app-server-protocol --test schema_fixtures --locked` passed: 4 tests.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Focus only on whether accepted blocking findings are closed or still reproducible.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| normal | 12 minutes | one bounded extension if active | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary-closure | Verifies runtime guards and tests close the concrete state bugs. | state, input, data |
| architecture-adversary-closure | Verifies minimal-event state boundary and doc scope are now aligned. | architecture, protocol boundary |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary-closure | `multi_agent_v1.spawn_agent` | `019e9305-8569-7940-8dca-73b94433caa0` (`Mendel`) | spawn tool result | `fork_context=false` | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| architecture-adversary-closure | `multi_agent_v1.spawn_agent` | `019e9305-99c3-76e1-88e8-dcc461a20812` (`Nietzsche`) | spawn tool result | `fork_context=false` | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| implementation-closure-round-2 | implementation-adversary-closure | 1 | `019e9305-8569-7940-8dca-73b94433caa0` | ~7 minutes | completed | reviewer returned no blocking findings | completed |
| architecture-closure-round-2 | architecture-adversary-closure | 1 | `019e9305-99c3-76e1-88e8-dcc461a20812` | ~7 minutes | completed | reviewer returned no blocking findings | completed |

### Reviewer Outputs

#### implementation-closure-round-2

##### Summary
Blocking closure is satisfied in implementation. All four cognitive actions route through `active_task_context_for_cognitive_update`, and that helper now requires experiment mode, active task, correct owner, active map, task/map linkage, existing map, and `map.task_id` ownership. Result downgrade is blocked for any non-accepted validity while an active fact cites that result.

##### Blocking Findings
- none

##### Non-blocking Risks
- The report still showed Round 2 as pending during read-only review.
- Tests proved the shared downgrade branch with `invalid`, but did not separately exercise `questioned` and `unreviewed`.

##### Required Fixes
- none for closure scope

##### Missing Tests
- Add explicit negative for `mark_result_validity_for_main` with missing active map.
- Add table coverage for downgrade to `questioned` and `unreviewed`.

##### Missing Logs / Observability
- No new closure-blocking logging gap found.

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` - four cognitive entrypoints call the shared active task path helper.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` - helper enforces active map and task/map ownership.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` - non-accepted result validity is rejected when an active fact cites the result.

#### architecture-closure-round-2

##### Summary
Substantive closure is closed for the scoped architecture blocker. `MapRuntimeResultValidityChangedEvent` is minimal-ref only, runtime emission matches that struct, and protocol tests assert the old semantic fields are absent. The engineering plan separates Phase 4A implemented events from Phase 6/7 and v1.1 events.

##### Blocking Findings
- none

##### Non-blocking Risks
- The report still showed Round 2 as pending during read-only review.
- `validityReason` still exists in `ActionMapSnapshotResultEvidencePackage`, which is correct because snapshot/evidence package remains authoritative state.

##### Required Fixes
- none for closure scope

##### Missing Tests
- no closure-blocking missing tests found

##### Missing Logs / Observability
- no new closure-blocking logging gap found

##### Evidence
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs` - `MapRuntimeResultValidityChangedEvent` has only `task_id`, `map_id`, `node_id`, `result_id`, `validity`.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` - emitted event only passes minimal refs.
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs` - protocol test asserts no `validityReason`, `claims`, or `evidenceRefs`.
- `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md` - plan states runtime events do not carry description, claims, evidence refs, or validity reason.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| implementation-adversary-closure | Round 1 implementation blockers closed | Active task path and fact/result trust boundaries now hold | n/a | accept | Fresh closure review found no blocking findings | Kept fixes; added extra missing-active-map validity test and downgrade table for `invalid/questioned/unreviewed`. | None |
| architecture-adversary-closure | Round 1 architecture blocker closed | Runtime event no longer duplicates semantic validity reason | n/a | accept | Fresh closure review found no blocking findings | Kept protocol/doc fixes. | None |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2
- Blocking re-review launch records:
  - implementation-adversary-closure `019e9305-8569-7940-8dca-73b94433caa0`
  - architecture-adversary-closure `019e9305-99c3-76e1-88e8-dcc461a20812`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Blocked reason: n/a
- Allowed to proceed: yes

## Final Conclusion

Passed for Phase 4A. The accepted blocking findings were fixed and fresh closure review found no blocking findings. Remaining items are non-blocking Phase 5/6/7 debt: session-level event emission tests, handler parser tests, structured rejection observability, claim-id audit joins, viewer cognitive panel, final artifact audit hard gate, and sentinel clear action.

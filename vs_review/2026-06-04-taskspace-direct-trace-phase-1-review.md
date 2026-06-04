# Subagent VS Review: TaskSpace Direct Trace Phase 1

- Created: 2026-06-04T16:50:12+08:00
- Updated: 2026-06-04T18:45:00+08:00
- Report schema: adversarial-v1
- Task: Implement Phase 1 append-only TaskSpace direct trace without changing standard mode behavior.
- Report path: `vs_review/2026-06-04-taskspace-direct-trace-phase-1-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: passed

## Round 1: Implementation And Validation Review

### Review Input

#### Objective
Implement the Phase 1 TaskSpace trace path so experiment mode records append-only structured observation events for main tool results, while standard mode remains unchanged and trace does not become a second authoritative problem-state store.

#### Review Target
Code implementation, snapshot protocol, generated schema fixtures, local tests, and the engineering-plan note documenting the Phase 1 boundary.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/action_map/map.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs`
- `third_party/codex-cli/codex-rs/app-server-protocol/tests/schema_fixtures.rs`
- `third_party/codex-cli/codex-rs/app-server-protocol/schema/typescript/ActionMapSnapshot.ts`
- `third_party/codex-cli/codex-rs/app-server-protocol/schema/typescript/ActionMapSnapshotTraceEventRef.ts`
- `third_party/codex-cli/codex-rs/app-server-protocol/schema/typescript/ActionMapSnapshotTraceSummary.ts`
- `third_party/codex-cli/codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.schemas.json`
- `third_party/codex-cli/codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.v2.schemas.json`
- `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md`

#### Change Introduction
The runtime now stores `taskspace_trace_events` in `ActionMapRuntimeState`, appends a `main_tool_result` trace event after a main tool result is successfully recorded to a node, restores trace refs from snapshots, and exposes `trace_summary` plus `trace_events` in `ActionMapSnapshot`. Trace refs include IDs, task/map/node/result join keys, call ID, action class, tool success, mechanical tags, artifact refs, and timestamp; they deliberately omit raw preview/body. Generated app-server TypeScript and JSON fixtures were refreshed, and schema tests assert the new surface.

#### Risk Focus
- Standard mode must not record trace or change behavior.
- Trace must stay observational and must not duplicate objective/facts/open questions.
- Snapshot restore must preserve trace sequence and not corrupt active task/map/node state.
- Tool preview/body must not leak through trace refs or be parsed into false structured semantics.
- Validator failure tags must derive only from structured action class plus success, not text.
- Schema fixtures and protocol defaults must remain backward-compatible.
- Barrier and node result recording behavior must stay unchanged.

#### Assumptions To Attack
- Appending trace after result persistence is enough to avoid orphan trace events.
- Using `Option<ActionClass>` plus `success` is sufficient for Phase 1 tags.
- `trace_summary` derived from snapshot refs is stable and cheap enough.
- Legacy snapshots without trace fields deserialize with empty trace.
- Restored snapshots can safely resume trace IDs from the highest `trace-*` suffix.
- Adding trace fields to app-server schema does not require a separate event notification.
- Deferring explicit `taskspace_control` tags to Phase 2 is consistent with the current handler interface.

#### Adversarial Lenses
- implementation
- architecture
- state and restore
- compatibility
- testing validity
- observability
- maintenance

#### Verification Status
- `rustup run stable cargo test -p codex-core trace --lib --locked` passed.
- `rustup run stable cargo test -p codex-core action_map::runtime::tests --lib --locked` passed: 108 tests.
- `rustup run stable cargo test -p codex-protocol action_map_snapshot --lib --locked` passed: 2 tests.
- `rustup run stable cargo test -p codex-app-server-protocol --test schema_fixtures --locked` passed: 4 tests.
- `git diff --check` passed; only Windows CRLF warnings were reported.
- E2 smoke and full Whale binary rebuild were not run in this Phase 1 slice.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Do not trust this report as proof; use it only as navigation.
- Cite evidence paths and line numbers when possible.
- Focus on high-impact correctness, compatibility, state, and validation failures.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 12 minutes | one bounded 8 minute extension if alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | Runtime state mutation, restore, result recording, and summary derivation changed in core execution code. | correctness, state flow, compatibility |
| architecture-adversary | Phase 1 must avoid creating a second authoritative state store and must fit existing TaskSpace/runtime/snapshot boundaries. | boundaries, long-term maintainability |
| test-validity-adversary | The change is test-heavy and could still miss user-visible or restore/schema regressions. | self-deceptive tests, missing failure paths |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1.spawn_agent` | `019e91d4-5715-7f02-9d27-2ed9d42bf1e9` (`Lovelace`) | spawn_agent tool result | fork_context=false | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| architecture-adversary | `multi_agent_v1.spawn_agent` | `019e91d4-b1b6-7002-b07c-bc8289227cf4` (`Russell`) | spawn_agent tool result | fork_context=false | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| test-validity-adversary | `multi_agent_v1.spawn_agent` | `019e91d5-0667-7b01-9586-57d6408a9138` (`Anscombe`) | spawn_agent tool result | fork_context=false | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| implementation-adversary-r1 | implementation-adversary | 1 | `019e91d4-5715-7f02-9d27-2ed9d42bf1e9` | completed | completed | reviewer returned findings | recorded below |
| architecture-adversary-r1 | architecture-adversary | 1 | `019e91d4-b1b6-7002-b07c-bc8289227cf4` | completed | completed | reviewer returned findings | recorded below |
| test-validity-adversary-r1 | test-validity-adversary | 1 | `019e91d5-0667-7b01-9586-57d6408a9138` | completed | completed | reviewer returned blocking findings | accepted and fixed |

### Reviewer Outputs

#### implementation-adversary-r1

Summary: Phase 1 direction is sound and trace is append-only, but observability and coverage were too narrow in the initial slice.

Blocking findings: none.

Non-blocking risks:

- Missing live runtime event/log for trace records made the feature hard to observe outside snapshot polling.
- Standard-mode transition semantics were ambiguous: existing trace after disabling experiment mode needed an explicit test documenting that historical trace remains but new standard-mode results do not append trace.
- Runtime coverage should include read/edit/test action-class matrix rather than one generic tool-result path.

#### architecture-adversary-r1

Summary: Trace does not appear to become a second authoritative problem-state store, but the boundary needs guardrails.

Blocking findings: none.

Non-blocking risks:

- Restored trace tags were free-form and could become a future sentinel injection surface if accepted unchanged.
- Trace observability should be emitted as a runtime event, not only exposed through snapshot refs.
- Child/subagent trace is intentionally out of Phase 1, but the gap should remain explicit to avoid overclaiming.
- Snapshot trace refs may need a future bound or pagination strategy if long runs grow large.

#### test-validity-adversary-r1

Summary: The initial implementation had real test gaps: it proved runtime helpers, but not enough production/session behavior or real CLI regression.

Blocking findings:

- Missing production/session-path test: the initial tests did not prove the session API path used by dispatcher/tool handling emits trace.
- Missing `taskspace_trace_event_recorded` live event/log: snapshot-only trace could not be reliably observed in rollout streams.
- Regression/E2 smoke was not proven for the installed Whale path.

Non-blocking risks:

- Protocol tests should prove trace refs do not expose raw tool output.
- The report should distinguish Phase 1 mechanism evidence from later cognitive-state MVP claims.

### Main Agent Response

| Finding | Response | Action Taken |
|---|---|---|
| Missing production/session-path test | accept | Added `session_main_tool_result_emits_taskspace_trace_event_and_snapshot` and `session_standard_mode_main_tool_result_does_not_record_trace` in `core/src/session/tests.rs`. |
| Missing live trace event/log | accept | Added `MapRuntimeTraceEventRecordedEvent`, `MapRuntimeEvent::TaskspaceTraceEventRecorded`, and runtime emission after successful node-result persistence. |
| Regression/E2 smoke not proven | accept | Ran action-map regression matrix and installed-binary `single-file-fast-fix` paired smoke; evidence recorded below. |
| Standard-mode transition ambiguous | accept | Added runtime test proving historical trace remains visible but new standard-mode tool results do not append trace. |
| Read/edit/test matrix missing | accept | Added runtime test covering read, edit, and test action classes, including validator success tag for test action. |
| Restored trace tags free-form | accept | Added restore-time tag sanitization to the allowed Phase 1 tag set. |
| Protocol raw output leakage risk | accept | Protocol and schema tests assert trace refs expose IDs/tags only and omit preview/body fields. |
| Child/subagent trace gap | accept as documented boundary | Phase 1 records main tool results only; subagent result evidence remains stored in `NodeResult`. Child trace is deferred to later evidence-package work. |
| Snapshot trace refs may grow large | defer | Kept as a future pagination/bounding concern; Phase 1 adds summary plus refs and does not use trace as active prompt state. |

### Post-Fix Verification

Commands and evidence:

- `rustup run stable cargo test -p codex-core taskspace_trace --lib --locked` passed: 2 tests.
- `rustup run stable cargo test -p codex-core session_standard_mode_main_tool_result_does_not_record_trace --lib --locked` passed: 1 test.
- `rustup run stable cargo test -p codex-core action_map::runtime::tests --lib --locked` passed: 113 tests.
- `rustup run stable cargo test -p codex-protocol action_map_snapshot --lib --locked` passed: 2 tests.
- `rustup run stable cargo test -p codex-protocol map_runtime_trace_event_recorded_serializes_refs_without_raw_output --lib --locked` passed: 1 test.
- `rustup run stable cargo test -p codex-app-server-protocol --test schema_fixtures --locked` passed: 4 tests.
- `scripts/run-action-map-regression.ps1` passed after adding trace session-path runs to the default matrix: `target/test-reports/action-map-20260604-184129-465/report.md`, overall `PASS`, 179 matched tests passed, 0 failed, 3 script tests passed, 0 relevant Windows crash events.
- Installed-binary smoke passed: `C:\Users\77585\AppData\Local\Temp\whale-paired-bench-runs\single-file-fast-fix\20260604-183001-885\pair-001\pair-report.md`; TaskSpace side had 1 map, 4 nodes, 3 edges, 0 edge-order violations, business success true.
- Real rollout evidence: `C:\Users\77585\AppData\Local\Temp\whale-paired-bench-runs\single-file-fast-fix\20260604-183001-885\pair-001\right\artifacts\rollout.jsonl` contains `taskspace_trace_event_recorded` and snapshot `traceSummary.totalEventCount=8`.

### Round 2: Accepted Blocking Closure Review

#### Review Input

Fresh closure reviewer receives only the target paths, the accepted blocking findings, the claimed fixes, and the verification evidence above. The reviewer is asked to attack whether Phase 1 blocking findings are really closed, without treating Phase 1 as proof of the later cognitive-state MVP.

#### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| closure-test-validity-adversary | `multi_agent_v1.spawn_agent` | `019e9232-ad22-71b1-b466-a0190ff53fd2` (`Carver`) | spawn_agent tool result | fork_context=false | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

#### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| closure-test-validity-adversary-r2 | closure-test-validity-adversary | 1 | `019e9232-ad22-71b1-b466-a0190ff53fd2` | completed | completed | closure verdict pass | accepted non-blocking follow-up below |

#### Reviewer Outputs

Summary: TaskSpace Direct Trace Phase 1 has basically closed the first-round accepted blocking findings. The production session API path is tested, the runtime live event is wired, snapshot/restore/standard-mode paths are covered, and regression plus installed-binary smoke prove Phase 1 did not break real execution. This does not prove the later cognitive MVP.

Blocking findings:

- none.

Non-blocking risks:

- Shell action class still depends on command-string classification in the existing tool/session layer. This is acceptable for Phase 1 observation, but Phase 2 sentinel must explicitly distinguish allowed `ActionClass` consumption from forbidden preview/body/result semantic parsing.
- The initial action-map regression report did not include the new session trace tests in its default matrix; they were only recorded as separate targeted cargo commands.
- Trace refs do not expose raw preview/body, but the full snapshot still contains legacy `maps.results[].body` preview text. This is not a new trace leak, but viewer/audit code must continue treating full snapshot result bodies as sensitive display material.

Closure verdict: pass.

Rationale: Runtime persists `NodeResult` first, then appends trace and emits `node_result_recorded -> taskspace_trace_event_recorded -> maintenance_barrier`, reducing orphan/order risk. Standard mode does not append trace. Restore preserves trace refs, resumes `trace-*` sequencing, and filters unknown tags. Protocol/schema tests cover legacy defaults and trace no-preview/no-body shape. Installed-binary smoke is diagnostic E2-candidate only, but proves Phase 1 mechanism survives real `whale exec --taskspace` and rollout exposes `taskspace_trace_event_recorded`.

#### Main Agent Response To Closure Review

| Finding | Response | Action Taken |
|---|---|---|
| Command classifier boundary is unclear | accept | Updated the engineering plan to state that tool/session `ActionClass` classification is allowed as a mechanical structured signal, while runtime/sentinel must not parse preview/body/result text or infer output/provenance/final-artifact semantics from raw command text. |
| Default regression matrix omitted session trace tests | accept | Added `core-taskspace-trace` and `core-session-standard-trace` to `scripts/run-action-map-regression.ps1`; reran full default regression with 179 matched tests passed. |
| Full snapshot legacy result body still contains preview text | accept as existing sensitive surface | Kept out of trace refs; deferred viewer/audit display hardening to the viewer/audit phase where full snapshot rendering is handled. |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2: Accepted Blocking Closure Review
- Blocking re-review launch records:
  - `019e9232-ad22-71b1-b466-a0190ff53fd2`
- Rejected findings backed by evidence: none
- Deferred findings documented: snapshot trace ref growth bound/pagination is deferred beyond Phase 1
- Blocked reason: none
- Allowed to proceed: yes

## Final Conclusion

Phase 1 direct trace implementation is allowed to proceed. It closes the accepted blocking findings for append-only trace, live event observability, session production-path coverage, and default action-map regression coverage. It does not claim that the later cognitive-state MVP, sentinel, provenance, or result-validity phases are complete.

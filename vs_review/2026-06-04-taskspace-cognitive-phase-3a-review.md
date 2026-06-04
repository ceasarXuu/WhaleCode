# Subagent VS Review: TaskSpace Cognitive Phase 3A

- Created: 2026-06-04T20:18:00+08:00
- Updated: 2026-06-04T21:36:00+08:00
- Report schema: adversarial-v1
- Task: Implement TaskSpace Phase 3A cognitive data model and versioned snapshot without adding control actions or semantic runtime gates.
- Report path: `vs_review/2026-06-04-taskspace-cognitive-phase-3a-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: open

## Round 1: Implementation And Validation Review

### Review Input

#### Objective

Review the Phase 3A TaskSpace cognitive-state implementation. This slice should add the minimal production data model and versioned snapshot surface for output contracts, fact sources/provenance, result claims/evidence/validity, while keeping runtime semantics narrow. It must not add `taskspace_control` actions, hard gates, viewer behavior, or hidden semantic parsing.

#### Review Target

Code implementation, protocol/schema fixtures, runtime snapshot/restore behavior, tests, regression evidence, and engineering-plan update documenting Phase 3A boundary.

#### Target Locations

- `third_party/codex-cli/codex-rs/core/src/action_map/cognitive.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/map.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs`
- `third_party/codex-cli/codex-rs/app-server-protocol/tests/schema_fixtures.rs`
- `third_party/codex-cli/codex-rs/app-server-protocol/schema/typescript/ActionMapSnapshot*.ts`
- `third_party/codex-cli/codex-rs/app-server-protocol/schema/json/codex_app_server_protocol*.schemas.json`
- `docs/plans/2026-06-04-taskspace-cognitive-state-engineering-plan.md`

#### Change Introduction

The runtime now stores `TaskState.cognitive_state` and `NodeResult.evidence_package`. Snapshot now emits `cognitive_schema_version = taskspace-cognitive-v1`, task `cognitiveState`, and result `evidencePackage`. Restore accepts current-version cognitive payloads, defaults legacy snapshots to empty state, and ignores unknown cognitive schema versions rather than treating future payloads as current truth. Generated JSON/TypeScript schema exposes the new fields, and schema fixture tests assert defaults and field visibility.

#### Risk Focus

- Phase 3A must not create a second authoritative state outside `TaskState.cognitive_state` and `NodeResult.evidence_package`.
- Restore must be safe for legacy snapshots and unknown future cognitive schema versions.
- New snapshot fields must be defaulted and schema fixtures must prove generated JSON/TS freshness.
- Result evidence package must preserve existing result join keys instead of creating a shadow result index.
- Standard mode and existing TaskSpace action/map behavior must remain unchanged.
- This slice must not overclaim Phase 4/6/7 capabilities such as control actions, viewer cognitive panel, result validity guard, final artifact audit gates, or sentinel clear actions.

#### Verification Status

- `rustup run stable cargo fmt --all`: passed with existing nightly `imports_granularity` warnings.
- `rustup run stable cargo test -p codex-protocol action_map_snapshot --lib --locked`: 4 passed, 0 failed before review.
- `rustup run stable cargo test -p codex-core action_map::runtime::tests --lib --locked`: 116 passed, 0 failed.
- `rustup run stable cargo run -p codex-app-server-protocol --bin write_schema_fixtures --locked -- --schema-root app-server-protocol/schema`: passed.
- `rustup run stable cargo test -p codex-app-server-protocol --test schema_fixtures --locked`: 4 passed, 0 failed.
- `scripts/run-action-map-regression.ps1`: PASS, `target/test-reports/action-map-20260604-200625-146/report.md`, 191 matched tests passed, 0 failed, 3 script tests passed, 0 relevant Windows crash events.
- `git diff --check`: passed with line-ending warnings only.

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Do not trust this report as proof; use it only as navigation.
- Cite evidence paths and line numbers where possible.
- Focus on high-impact correctness, compatibility, state ownership, semantic-boundary, schema, and validation failures.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 12 minutes | one bounded 8 minute extension if alive | 2 | cannot pass if accepted blocking review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| cognitive-state-boundary-adversary | Phase 3A changes core Task/Result state ownership and restore semantics. | state ownership, restore safety, semantic boundary |
| protocol-schema-compat-adversary | Phase 3A changes protocol structs, generated TS/JSON schema, and fixture tests. | compatibility, schema defaults, viewer/API surface |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| cognitive-state-boundary-adversary | `multi_agent_v1.spawn_agent` | `019e9291-8d21-7d71-a658-159692a8f083` (`Copernicus`) | spawn_agent tool result | false | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| protocol-schema-compat-adversary | `multi_agent_v1.spawn_agent` | `019e9291-ce06-74d2-9bbf-f103b4b9941d` (`Raman`) | spawn_agent tool result | false | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### cognitive-state-boundary-adversary

Summary: Phase 3A mostly respects state ownership and semantic boundaries: cognitive state lives on `TaskState`, result evidence lives on `NodeResult`, join keys are preserved, and no new `taskspace_control` actions were added.

Blocking findings:

- Future cognitive payloads can fail before `restore_snapshot` can ignore them. `restore_snapshot` checks `cognitive_schema_version` only after typed `ActionMapSnapshot` exists, while nested cognitive/evidence structs required v1-shaped fields. Counterexample: a v999 snapshot with `cognitiveState: {"futureFacts": [...]}` or incompatible `evidencePackage` shape can be rejected at serde/schema boundary before runtime reaches the unknown-version branch.

Non-blocking risks:

- Unknown-version restore is lossy by design: after dropping v999 cognitive payload, snapshot re-emits `taskspace-cognitive-v1` with empty cognitive/evidence state.
- No production mutation path or observability exists for cognitive writes yet; tests populate state by direct internal mutation and docs defer control actions to Phase 4.
- Protocol exposes provenance/kind/validity as strings, so schema cannot reject invalid values even though core filters/downgrades them.

Required fixes:

- Make cognitive/evidence restore tolerant before typed v1 parsing, or at minimum add defaults and tests for `{}` / partial payloads.
- Add JSON deserialization tests for unknown future `cognitiveSchemaVersion` with future/partial cognitive/evidence shapes.
- Update schema fixtures so required/default behavior matches the intended restore contract.

#### protocol-schema-compat-adversary

Summary: Current v1 snapshots and legacy snapshots with fields absent are structurally close, and generated TS/JSON freshness tests pass. The unknown-version compatibility claim fails at the raw JSON boundary.

Blocking findings:

- Unknown-version compatibility is only proven after typed deserialization, not at the JSON boundary. The unknown-version runtime test mutates a valid typed v1 snapshot to `taskspace-cognitive-v999`, so it does not cover future-shaped or partial raw JSON.

Non-blocking risks:

- Generated TypeScript makes defaulted non-Option object fields required, which can mislead external/viewer clients consuming legacy cached JSON.
- `/task-show` viewer ignores cognitive/evidence fields; acceptable only if viewer cognitive display is explicitly deferred.
- Current v1 restore silently drops unknown fact-source/output-contract enum values and downgrades unknown result validity to `unreviewed`.

Required fixes:

- Add tolerant/version-aware deserialization or otherwise guarantee raw JSON with unknown `cognitiveSchemaVersion` cannot fail before restore.
- Add raw JSON tests for unknown-version snapshots with future/partial `cognitiveState` and `evidencePackage`.
- Add schema fixture assertions that `ActionMapSnapshotTask.required` excludes `cognitiveState` and `ActionMapSnapshotResult.required` excludes `evidencePackage`.
- Decide whether generated TS should expose defaulted fields as optional or whether clients must use a normalizer.

### Main Agent Response

- accept: Unknown/future cognitive payload must not fail before runtime restore. Implemented `deserialize_default_on_error` for the task/result cognitive containers and added field-level defaults to `ActionMapSnapshotCognitiveState`, `ActionMapSnapshotFactSource`, `ActionMapSnapshotOutputContract`, `ActionMapSnapshotCognitiveClaim`, and `ActionMapSnapshotResultEvidencePackage`.
- accept: Add raw JSON tests. Added protocol tests for partial cognitive payloads and unknown future cognitive schema with incompatible future shapes; updated runtime restore test to deserialize future-shaped raw JSON before restore.
- accept: Add schema fixture required/default assertions. Added checks that `ActionMapSnapshotTask.required` excludes `cognitiveState`, `ActionMapSnapshotResult.required` excludes `evidencePackage`, and evidence package required fields do not force `claims` / `evidenceRefs` / `validity`.
- accept: TypeScript nullable accuracy for `cognitiveSchemaVersion`. Changed it to `#[ts(optional = nullable)]`, yielding optional nullable TypeScript while preserving JSON default `null`.
- defer: Non-Option default object fields still appear required in generated TypeScript because `ts-rs` only allows `#[ts(optional)]` for `Option<T>`. Changing `cognitive_state` / `evidence_package` to `Option` would weaken the runtime model; JSON schema is the compatibility authority for legacy cached snapshots.
- defer: Viewer cognitive side panel and viewer empty-state smoke remain Phase 6 work; Phase 3A only exposes snapshot/schema state.
- defer: Production mutation actions and observability for cognitive writes remain Phase 4 work; this slice intentionally avoids new `taskspace_control` actions.
- defer: Logs/audit events for unknown cognitive payload drop remain Phase 4/6 observability work. Phase 3A documents the destructive downgrade and tests that unknown payload is not trusted.
- reject: Free-string schema for provenance/kind/validity is not a Phase 3A blocker. Runtime conversion already filters unknown values, and hard validation belongs to future control-action handlers, not snapshot replay.

Post-fix validation:

- `rustup run stable cargo test -p codex-protocol action_map_snapshot --lib --locked`: 6 passed, 0 failed after blocking fix.
- `rustup run stable cargo run -p codex-app-server-protocol --bin write_schema_fixtures --locked -- --schema-root app-server-protocol/schema`: passed.
- `rustup run stable cargo test -p codex-app-server-protocol --test schema_fixtures --locked`: 4 passed, 0 failed.
- `rustup run stable cargo test -p codex-core action_map::runtime::tests --lib --locked`: 116 passed, 0 failed.
- Closure reviewer suggested adding explicit `null` / non-object cognitive container coverage. Added `action_map_snapshot_deserializes_null_or_non_object_cognitive_containers`; rerun `rustup run stable cargo test -p codex-protocol action_map_snapshot --lib --locked`: 7 passed, 0 failed.
- Final full regression after closure/test supplement: `scripts/run-action-map-regression.ps1` PASS, `D:\whalecode-alpha\target\test-reports\action-map-20260604-210113-029\report.md`, 194 matched tests passed, 0 failed, 3 script tests passed, 0 relevant Windows crash events.

### Closure Review

#### Closure Launch Record

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| blocking-closure-compat-reviewer | `multi_agent_v1.spawn_agent` | `019e92b2-aaed-7e32-b09a-294ccde4fa9b` (`Huygens`) | spawn_agent tool result | false | Closure review input focused on accepted blocking fix | main-agent history, reasoning, drafts, conclusions, write access | yes |

#### Closure Reviewer Output

Summary: The Round 1 blocking compatibility issue appears fixed. Protocol deserialization now tolerates missing, `{}`, partial, and future-shaped incompatible `cognitiveState` / `evidencePackage` containers before runtime restore gets control.

Blocking findings:

- None.

Evidence cited by reviewer:

- `deserialize_default_on_error` defaults malformed cognitive/evidence container shapes.
- The tolerant deserializer is applied to `cognitiveState` and `evidencePackage`.
- Raw JSON protocol tests cover legacy missing fields, `{}` partial payloads, and v999 incompatible shapes.
- Runtime only trusts cognitive payload when version equals current v1.
- Unknown-version restore defaults task cognitive state and result evidence package.
- Runtime test now deserializes future-shaped raw JSON before restore and verifies it is dropped.

Non-blocking risks:

- Unknown-version cognitive data is destructively downgraded; this is intentional and documented.
- Generated TypeScript still shows non-Option default object fields as required even though JSON schema/serde allow legacy absence; JSON schema remains the compatibility authority for legacy cached snapshots.
- JSON schema remains v1-shaped for known-property type validation; external pre-validation of unknown future payloads can still reject incompatible known fields even though Rust serde accepts them.

Missing tests/logs:

- Suggested explicit `null` / non-object cognitive container tests; accepted and added after closure review.
- Log/audit event for unknown-version or malformed cognitive/evidence drop remains deferred to later observability work.

Verdict: pass for closure.

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Rejected findings backed by evidence: yes
- Deferred findings documented: yes
- Blocked reason: none
- Allowed to proceed: yes

## Final Conclusion

Phase 3A review closed. The accepted blocking compatibility finding was fixed and passed fresh closure review. Remaining risks are documented as non-blocking Phase 4/6 follow-up work.

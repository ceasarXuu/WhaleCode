# TaskSpace Phase 2 Ledger Review

## Review Target

- Objective: complete TaskSpace 0.0.4 Phase 2 by making ProblemStateLedgerV1 first-class runtime state.
- Target type: code implementation, runtime state, protocol schema, gate behavior, tool schema, viewer display, tests.
- Target locations:
  - `third_party/codex-cli/codex-rs/core/src/action_map/ledger.rs`
  - `third_party/codex-cli/codex-rs/core/src/action_map/map.rs`
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs`
  - `third_party/codex-cli/codex-rs/protocol/src/protocol.rs`
  - `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`
  - `third_party/codex-cli/codex-rs/tui/src/app/action_map_viewer.rs`

## Review Input Packet

Fresh reviewer instructions:

- Start from the repository state directly. Do not inherit the main agent's conversation context.
- Read the target files directly. Do not edit files.
- Challenge assumptions and happy paths.
- Focus on high-impact defects in:
  - serde backward compatibility for old snapshots
  - runtime restore and snapshot roundtrip
  - start_task initial success criteria handling
  - preflight gate boundary and bypasses
  - v1 cognitive state weak mapping into ledger
  - taskspace_control schema/handler mismatch
  - viewer ledger display assumptions
  - missing tests or weak tests
- Cite file paths and line numbers where possible.

Verification already run before review:

- `cargo check -p codex-protocol --locked`
- `cargo check -p codex-tools --locked`
- `cargo check -p codex-core --locked`
- `cargo check -p codex-tui --locked`
- `cargo test -p codex-protocol action_map_snapshot --locked`
- `cargo test -p codex-tools taskspace_control --locked`
- `cargo test -p codex-core problem_ledger --locked`
- `cargo test -p codex-core cognitive_preflight_requires_problem_success_criteria --locked`
- `cargo test -p codex-core restore_snapshot_without_ledger_version_marks_task_legacy --locked`
- `cargo test -p codex-tui viewer_html_contains_polling_snapshot_endpoint --locked`

Required reviewer output:

- Summary
- Blocking findings
- Non-blocking risks
- Required fixes
- Missing tests
- Missing logs or observability
- Evidence paths and line numbers when possible

## Launch Records

| Round | Reviewer Role | Mechanism | Agent ID | Fork Context | Input Scope | Status |
|---|---|---|---|---|---|---|
| 1 | Runtime state and schema adversary | internal `multi_agent_v1.spawn_agent` | `019eb782-b515-7183-b9fc-e0ff0ac6e01f` | false | packet above plus target paths | completed |
| 2 | Blocking-fix closure reviewer | internal `multi_agent_v1.spawn_agent` | `019eb791-8a35-7621-b5b4-741e74bd6b43` | false | closure packet for Round 1 findings | completed |

## Round 1 Reviewer Output

### Summary

Reviewer found blocking issues against the Phase 2 ledger claim. Main areas: stale app-server generated protocol schemas, legacy snapshot migration, partial dual-write between cognitive state and ledger, and mismatch between `initial_success_criteria` wording and runtime behavior.

### Blocking Findings

1. App-server protocol schemas do not expose `problemLedger` / `problemStateLedgerVersion`.
2. Old snapshots with old cognitive criteria can restore into a newly blocked state because legacy criteria are not migrated into ledger criteria.
3. Ledger is not fully first-class because preflight still mixes one ledger field with two legacy cognitive fields, and `record_fact_source` does not enter the ledger.
4. `start_task.initial_success_criteria` is advertised as required but accepted as optional.

### Non-Blocking Risks

- `record_output_contract` weak-mapping to success criteria is semantically lossy.
- Viewer has only static HTML string assertions, not malformed payload render tests.
- Ledger includes hypotheses/risks/blockers without mutation actions in Phase 2.

### Missing Tests / Logs

- Restore old snapshot with populated cognitive criteria and verify post-restore preflight behavior.
- Snapshot/restore roundtrip with ledger criteria/facts/questions/decisions/next action.
- Handler-level `start_task` criteria behavior test.
- App-server protocol fixture assertions for new ledger fields.
- Ledger mutation observability is currently generic `CognitiveStateUpdated`.

## Main-Agent Triage

- Finding 1: accept. Regenerate app-server protocol fixtures and add assertions.
- Finding 2: accept. Migrate legacy `cognitiveState.successCriteria` and `outputContracts` into ledger success criteria during restore, with `schema_incomplete=true`.
- Finding 3: partially accept. Phase 2 intentionally keeps cognitive v1 for compatibility, but ledger should receive fact-source records as known facts and ledger update event kinds should be explicit.
- Finding 4: accept as wording/schema issue. Runtime intentionally allows `start_task` without initial criteria so the agent can recover through `record_success_criteria`; update tool description to say optional but gate-blocking if absent, and add handler/runtime tests for this behavior.

## Fixes Applied After Round 1

- Regenerated app-server protocol schema fixtures with `cargo run -p codex-app-server-protocol --bin write_schema_fixtures --locked`.
- Added app-server fixture assertions for `problemStateLedgerVersion`, `problemLedger`, and `ActionMapSnapshotProblemStateLedger`.
- Added legacy restore migration from `cognitiveState.successCriteria`, `outputContracts`, and `factSources` into `ProblemStateLedger`, while keeping `schema_incomplete=true`.
- Added post-restore preflight test to prove migrated legacy cognitive state does not become newly blocked.
- Added ledger snapshot/restore roundtrip assertions for open questions, decisions, and next best action.
- Added `record_fact_source` weak mapping into ledger `known_facts`.
- Renamed ledger mutation event kinds to `problem_ledger.*` under the existing `CognitiveStateUpdated` event.
- Updated `taskspace_control` schema wording so `initial_success_criteria` is optional at start but ordinary work remains blocked until success criteria exist.
- Added handler parsing tests for missing and present `initial_success_criteria`.

## Fix Validation

- `cargo test -p codex-app-server-protocol action_map_snapshot_schema_exposes_trace_summary_and_refs --locked` passed.
- `cargo test -p codex-core restore_snapshot_without_ledger_version_migrates_legacy_cognitive_state --locked` passed.
- `cargo test -p codex-core problem_ledger_records_questions_decisions_and_next_action --locked` passed.
- `cargo test -p codex-core start_task_parses_initial_success_criteria_when_present --locked` passed.
- `cargo test -p codex-core start_task_accepts_missing_initial_success_criteria_for_gate_recovery --locked` passed.
- `cargo test -p codex-core cognitive_control_actions_update_task_state_and_result_package --locked` passed after tightening the fact-source event assertion to `problem_ledger.fact_source`.
- `cargo test -p codex-tools taskspace_control --locked` passed.
- `cargo test -p codex-protocol action_map_snapshot --locked` passed.
- Real installed CLI smoke initially exposed two TaskSpace control usability errors:
  - `record_output_contract.kind` was schema-ambiguous because the shared `kind` field was documented as a node kind while runtime expected output contract kinds.
  - `mark_result_validity accepted` was attempted without claims in an evidence-free smoke path.
- Follow-up fix:
  - Added explicit `output_contract_kind` to the tool schema and kept `kind` as a handler alias for compatibility.
  - Tightened `mark_result_validity` tool wording so `accepted` is only used with non-empty claims and evidence refs.
- Follow-up validation:
  - `cargo test -p codex-tools taskspace_control --locked` passed.
  - `cargo test -p codex-core record_output_contract_prefers_specific_kind_field_and_keeps_alias --locked` passed.
  - Rebuilt and installed local `whale.exe`.
  - Real smoke `whale -a never exec --taskspace --ephemeral --json -s read-only "<smoke prompt>"` exited 0 with no `TaskSpace`, `taskspace_control`, or `ERROR codex_core` stderr matches.

## Final Main-Agent Verification

- `cargo fmt` completed; rustfmt emitted the repository's existing stable-channel warning for `imports_granularity = Item`.
- `cargo check -p codex-core -p codex-protocol -p codex-tools -p codex-tui -p codex-app-server-protocol --locked` passed.
- `cargo test -p codex-protocol action_map_snapshot --locked` passed.
- `cargo test -p codex-tools taskspace_control --locked` passed.
- `cargo test -p codex-core problem_ledger --locked` passed.
- `cargo test -p codex-core cognitive_preflight_requires_problem_success_criteria --locked` passed.
- `cargo test -p codex-core restore_snapshot_without_ledger_version_migrates_legacy_cognitive_state --locked` passed.
- `cargo test -p codex-core start_task_accepts_missing_initial_success_criteria_for_gate_recovery --locked` passed.
- `cargo test -p codex-core start_task_parses_initial_success_criteria_when_present --locked` passed.
- `cargo test -p codex-core cognitive_control_actions_update_task_state_and_result_package --locked` passed.
- `cargo test -p codex-core record_output_contract_prefers_specific_kind_field_and_keeps_alias --locked` passed.
- `cargo test -p codex-tui viewer_html_contains_polling_snapshot_endpoint --locked` passed.
- `cargo test -p codex-app-server-protocol action_map_snapshot_schema_exposes_trace_summary_and_refs --locked` passed.
- `cargo test -p codex-app-server-protocol schema_fixtures_match_generated --locked` passed.
- `cargo build -p codex-cli --bin whale --locked` passed.
- `scripts/install-whale-local.ps1 -BackupLegacyCopies` installed `C:\Users\77585\.whale\bin\whale.exe` from `D:\BuildCache\whalecode\cargo-target\debug\whale.exe`.
- `where.exe whale`, `whale --version`, and `whale debug models` passed; installed CLI resolves to `C:\Users\77585\.whale\bin\whale.exe` and exposes only the DeepSeek V4 Pro/Flash model list.
- Real installed `whale exec --taskspace` smoke passed with thread `019eb7ad-b0e5-7192-9f64-9d3283ba7429` and no TaskSpace/runtime stderr errors.

## Closure Status

Closure review completed. No remaining blocking findings.

## Round 2 Closure Reviewer Output

### Closure Summary

The closure reviewer found no remaining blocking findings against the four Round 1 accepted blockers.

### Original Blocking Finding Status

- App-server protocol schemas now expose `problemStateLedgerVersion` and `problemLedger`: closed.
- Old snapshots without ledger version now migrate legacy cognitive state into `ProblemStateLedger`: closed.
- `record_fact_source` now writes a ledger known fact and ledger mutation events use specific `problem_ledger.*` kinds: closed for the accepted Round 1 scope.
- `start_task.initial_success_criteria` is no longer advertised as required, and missing criteria are intentionally accepted for gate recovery: closed.

### Non-Blocking Risks

- Ledger is still not the sole preflight fact source. Preflight checks ledger success criteria but keeps legacy cognitive output contract and fact-source gates for compatibility. This is accepted for Phase 2 and should be revisited when Phase 3+ moves more cognitive protocol into ledger.
- Worktree still needed commit/push at review time.
- After closure review, a real installed CLI smoke found a schema usability bug in the shared `kind` field. Main-agent fixed it and validated with focused tests plus a clean real smoke. No fresh internal subagent spawn tool was exposed in the current runtime turn, so this post-closure follow-up is recorded as main-agent validated rather than independently re-reviewed.

### Missing Tests

- Suggested hardening: assert `record_fact_source_for_main` returns update kind `problem_ledger.fact_source`.
- Main-agent follow-up: implemented in `cognitive_control_actions_update_task_state_and_result_package` and verified with `cargo test -p codex-core cognitive_control_actions_update_task_state_and_result_package --locked`.

### Closure Verification

Reviewer re-ran and passed:

- `codex-app-server-protocol action_map_snapshot_schema_exposes_trace_summary_and_refs`
- `codex-core restore_snapshot_without_ledger_version_migrates_legacy_cognitive_state`
- `codex-core problem_ledger_records_questions_decisions_and_next_action`
- `codex-core start_task_parses_initial_success_criteria_when_present`
- `codex-core start_task_accepts_missing_initial_success_criteria_for_gate_recovery`
- `codex-tools taskspace_control`
- `codex-protocol action_map_snapshot`

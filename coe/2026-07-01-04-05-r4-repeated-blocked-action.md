# Problem P-001: R4 TaskSpace repeats the same blocked action until timeout
- Status: closed
- Created: 2026-07-01 04:05
- Updated: 2026-07-01 13:20
- Objective: Prevent TaskSpace action-contract sessions from repeatedly submitting an identical already-blocked tool action while preserving open-ended agent attempts through valid alternative actions.
- Symptoms:
  - `large-output-ref-smoke` TaskSpace run remains on one inspect node and times out after repeatedly requesting the same duplicate diagnostic `run_test`.
- Expected behavior:
  - After a tool action is blocked with structured recovery, the next turn should either choose a valid alternative action or receive a stronger stateful recovery that excludes the repeated invalid action until new evidence changes the node state.
- Actual behavior:
  - TaskSpace returns visible duplicate-diagnostic recovery, but the provider can repeat the same blocked action and burn the session budget until `agent_exec_timeout`.
- Impact:
  - R4-D benefit validation cannot pass on a tool-stress sample; TaskSpace wastes time and context on an invalid action loop.
- Reproduction:
  - Run `large-output-ref-smoke` with TaskSpace after the path-list evidence classifier fix.
- Environment:
  - Windows, PowerShell, branch workspace `D:\whalecode-alpha`, Rust codex-core action map/session turn path, 2026-07-01.
- Known facts:
  - E-001
  - E-002
  - E-003
  - E-004
  - E-005
- Ruled out:
  - Missing duplicate diagnostic gate alone; the gate emits `inspect_duplicate_successful_diagnostic_test`.
  - Premature forced transition after `rg --files` alone; classifier fix keeps the graph in inspect.
- Fix criteria:
  - A focused unit test proves repeated same blocked action is recognized as stateful recovery rather than generic repeated prompt text.
  - `large-output-ref-smoke` no longer times out from repeated duplicate diagnostic action and either solves or fails for a non-loop reason.
- Current conclusion: H-001 and H-002 are resolved for the `large-output-ref-smoke` correctness blocker. The repeated duplicate diagnostic no longer causes an unbounded timeout, failed `apply_patch` feedback is preserved, false missing-source blockers are rejected, and the same sample now solves. Performance remains a separate R4-G issue because TaskSpace wall time is still 6.45x standard.
- Related hypotheses:
  - H-001
- Resolution basis:
  - E-004
  - E-005
- Close reason:
  - Real rerun `target/r4-d-missing-source-blocker-20260701/large-output-ref-smoke/20260701-130225-851` changed `src/large_output_demo.py`, passed public validation and hidden oracle, and no longer timed out from repeated blocked actions.

## Hypothesis H-001: blocked-action recovery is stateless and repeatable
- Status: confirmed
- Parent: P-001
- Claim: TaskSpace currently treats a repeated identical blocked action as another ordinary gate recovery, so the model-visible signal does not become a stateful exclusion and the same invalid action can recur until timeout.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - The runtime emits `ToolActionBlocked` and `TaskSpaceGateRecoveryV1`, while the session turn path adds duplicate diagnostic recovery text. There is no confirmed per-node fingerprint memory that escalates identical blocked actions into a different recovery state.
- Falsifiable predictions:
  - If true: code inspection will show blocked gate errors are returned as visible recovery but not persisted as per-node same-action repeat state; real rollout will stay in the inspect node and repeatedly request the same action.
  - If false: an existing per-node blocked-action repeat tracker should prevent the identical action from being reissued or should force a different recovery after the first repeat.
- Diagnostic evidence plan:
  - Prediction or clause under test: repeated blocked actions are not currently statefully excluded.
  - Signal: runtime/session code path plus real benchmark summary.
  - Capture method: inspect `prepare_main_tool_call`, session gate-recovery handling, and latest benchmark pair report.
  - Event name or marker:
    - `inspect_duplicate_successful_diagnostic_test`
    - `TaskSpaceGateRecoveryV1`
  - Correlation keys:
    - `target/r4-d-path-list-not-working-evidence-20260701/large-output-ref-smoke/20260701-030240-333`
  - Differentiates from:
    - H-002 model never sees tool feedback.
    - H-003 premature forced transition.
  - Supports if:
    - The code has recovery text but no repeat fingerprint state, and the real run times out with one inspect node after duplicate diagnostic blocking.
  - Refutes if:
    - The code already records repeated blocked action fingerprints and changes allowed actions after repeat.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: implement repeated blocked action state and focused tests.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: latest real run stayed in inspect and timed out
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: `target/r4-d-path-list-not-working-evidence-20260701/large-output-ref-smoke/20260701-030240-333`
- Prediction or plan link:
  - H-001 predicts the real rollout will stay in inspect and repeatedly request the same blocked action.
- Matched signal:
  - `outcome_taskspace=agent_exec_timeout`, `nodes=1`, `edges=0`, `open_leaf_nodes=1`, `changed_paths=.large_output_probe_ran`, rollout near 1GB.
- Correlation keys:
  - `20260701-030240-333`
- Raw content:
  ```text
  outcome_taskspace=agent_exec_timeout
  exec_exit_code=124
  nodes=1
  edges=0
  open_leaf_nodes=1
  changed_paths=.large_output_probe_ran
  rollout=992,140,475 bytes
  ```
- Interpretation: The classifier no longer forces a premature transition; the remaining failure is an inspect-node action loop.
- Time: 2026-07-01 04:05

## Evidence E-002: duplicate diagnostic gate exists and emits structured recovery
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`, `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Prediction or plan link:
  - H-001 predicts ordinary gate recovery exists but is not enough to make the repeated action stateful.
- Matched signal:
  - Runtime returns `inspect_duplicate_successful_diagnostic_test`; session builds `TaskSpaceGateRecoveryV1` duplicate diagnostic inspect recovery.
- Correlation keys:
  - none
- Raw content:
  ```text
  reason: inspect_duplicate_successful_diagnostic_test
  next_valid_actions: read_file or search for implementation/test evidence
  ```
- Interpretation: The model is not missing the initial recovery message; the gap is repeated invalid action convergence.
- Time: 2026-07-01 04:05

## Evidence E-003: no per-node blocked-action repeat tracker was found in the active code path
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `rg` over `runtime.rs` and `turn.rs` for `ToolActionBlocked`, `TaskSpaceGateRecoveryV1`, and duplicate diagnostic handlers.
- Prediction or plan link:
  - H-001 predicts blocked action repeat state is absent.
- Matched signal:
  - Search located gate events and recovery builders, but no state key that fingerprints repeated blocked tool/action/command per node.
- Correlation keys:
  - none
- Raw content:
  ```text
  MapRuntimeEvent::ToolActionBlocked(...)
  TaskSpaceGateRecoveryV1
  build_taskspace_duplicate_diagnostic_inspect_recovery_item(...)
  ```
- Interpretation: The existing mechanism records that an action was blocked, but does not turn a repeated identical blocked action into a stateful exclusion.
- Time: 2026-07-01 04:05

## Hypothesis H-002: failed patch feedback is visible, but a false missing-source blocker can still be accepted
- Status: confirmed-resolved
- Parent: P-001
- Claim: After repeated diagnostic recovery is fixed, the model can reach implement_solution and receive failed `apply_patch` feedback, but `block_main_node` may still accept a blocker claiming source is not visible even when dependency inspect evidence already contains the source file.
- Layer: root-cause
- Factor relation: sequential
- Depends on:
  - H-001
- Rationale:
  - The 2026-07-01 12:15 rerun showed `TaskSpaceEditFailureRecoveryV1` and the raw `apply_patch verification failed` output were model-visible. The subsequent failure came from accepting `block_node` with "Current implementation ... is unknown because no excerpt was provided", creating an unreviewed blocker loop.
- Falsifiable predictions:
  - If true: rollout will show a failed internal `apply_patch`, a model-visible failed edit recovery message, then an accepted missing-source blocker.
  - If false: either failed patch feedback is absent, or runtime rejects the missing-source blocker before it can become an unreviewed result.
- Diagnostic evidence plan:
  - Prediction or clause under test: failed patch feedback is visible and the remaining gap is blocker validation.
  - Signal: rollout response items around `taskspace-action-contract-5-apply_patch` and `taskspace-action-contract-6-blocked`.
  - Capture method: parse `right/artifacts/rollout.jsonl`.
  - Event name or marker:
    - `TaskSpaceEditFailureRecoveryV1`
    - `apply_patch verification failed`
    - `TaskSpace node blocked: node-2 result result-5`
  - Correlation keys:
    - `target/r4-d-internal-policy-blocker-20260701/large-output-ref-smoke/20260701-121553-941`
  - Supports if:
    - Runtime accepts missing-source blocker after failed patch feedback and inspected source evidence.
  - Refutes if:
    - Runtime rejects the missing-source blocker with an actionable next step.
  - Instrumentation status: existing rollout evidence
  - Instrumentation lifecycle:
    - retained under target run artifacts
- Evidence gate: satisfied
- Related evidence:
  - E-004
  - E-005
- Conclusion: confirmed and repaired by missing-source blocker rejection plus structured action-contract feedback.
- Repair design readiness: implemented
- Next step: continue R4-G performance and 10-sample validation.
- Blocker:
  - none
- Close reason:
  - E-005 demonstrates the same sample now solves.

## Evidence E-004: failed patch feedback was visible but false missing-source blocker was accepted
- Related hypotheses:
  - H-002
- Direction: supports
- Type: reproduction
- Source: `target/r4-d-internal-policy-blocker-20260701/large-output-ref-smoke/20260701-121553-941/pair-001/right/artifacts/rollout.jsonl`
- Prediction or plan link:
  - H-002 predicts failed patch feedback is not missing; blocker validation is the remaining gap.
- Matched signal:
  - `taskspace-action-contract-5-apply_patch` failed with `apply_patch verification failed`.
  - Next provider-visible developer message contained `TaskSpaceEditFailureRecoveryV1`.
  - Runtime accepted `taskspace-action-contract-6-blocked` with "Current implementation ... is unknown because no excerpt was provided".
- Correlation keys:
  - `20260701-121553-941`
- Raw content:
  ```text
  taskspace-action-contract-5-apply_patch:
  apply_patch verification failed: Failed to find expected lines ... src\large_output_demo.py

  TaskSpaceEditFailureRecoveryV1:
  The previous edit tool call failed. Treat the tool result exactly like standard mode feedback...

  taskspace-action-contract-6-blocked:
  TaskSpace node blocked: node-2 result result-5
  ```
- Interpretation: The tool feedback path worked; the defect was accepting a blocker that contradicted available source evidence.
- Time: 2026-07-01 12:15

## Evidence E-005: repaired run solves large-output-ref-smoke
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: validation
- Source: `target/r4-d-missing-source-blocker-20260701/large-output-ref-smoke/20260701-130225-851/pair-001/pair-report.md`
- Prediction or plan link:
  - Fix criteria require no repeated-action timeout and either solved or a non-loop failure.
- Matched signal:
  - `outcome_standard=solved`, `outcome_taskspace=solved`, `failure_taxonomy=none`.
  - TaskSpace changed `src/large_output_demo.py`.
  - Public validation and hidden oracle exit code 0.
  - Rollout shows failed `apply_patch` followed by successful corrected `apply_patch` and `pytest` pass.
- Correlation keys:
  - `20260701-130225-851`
- Raw content:
  ```text
  outcome_taskspace: solved
  public_validation_exit_code: 0
  hidden_oracle_exit_code: 0
  taskspace_changed_paths: .large_output_probe_ran, src/large_output_demo.py
  taskspace_wall_time_ratio: 6.45
  taskspace_tool_call_ratio: 0.57
  ```
- Interpretation: Correctness blocker is resolved. The remaining 6.45x wall time is a separate R4-G performance issue, not evidence that the original repeated-blocked-action bug remains open.
- Time: 2026-07-01 13:07

## Hypothesis H-003: successful internal edit artifacts were not backfilled into the TaskSpace map
- Status: confirmed-resolved
- Parent: P-001
- Claim: In action-contract internal tool mode, a successful `apply_patch` can modify files while the active map's implement result still has no `changed_artifacts`, causing validation actions to be repeatedly blocked by `validation_test_missing_changed_artifact_coverage`.
- Layer: root-cause
- Factor relation: sequential
- Depends on:
  - H-001
  - H-002
- Rationale:
  - The `processing-pipeline` v2 run reached implementation and changed `generate_report.sh`, but TaskSpace then recorded 94 validation coverage blocks. The map had a successful edit result without artifact/evidence refs.
- Falsifiable predictions:
  - If true: backfilling changed artifacts from the turn diff into the successful implement result should drive the repeated validation coverage block count to zero on the same sample.
  - If false: the same validation block should continue after backfill, or the map should already contain artifacts before the fix.
- Diagnostic evidence plan:
  - Prediction or clause under test: successful edit artifact backfill removes validation coverage block loop.
  - Signal: count of `validation_test_missing_changed_artifact_coverage` in graph events.
  - Capture method: compare v2 and v3 run artifacts.
  - Event name or marker:
    - `observed_implementation_edit_artifacts`
    - `validation_test_missing_changed_artifact_coverage`
  - Correlation keys:
    - `C:\WhaleRunCache\r4-public10-20260701\actual\processing-pipeline-v2\runs\terminal_bench__processing-pipeline\20260701-132747-117`
    - `C:\WhaleRunCache\r4-public10-20260701\actual\processing-pipeline-v3\runs\terminal_bench__processing-pipeline\20260701-141309-114`
  - Supports if:
    - v2 has repeated coverage blocks and v3 has zero after the code fix.
  - Refutes if:
    - v3 still repeats the same coverage block.
  - Instrumentation status: existing graph/rollout evidence
  - Instrumentation lifecycle:
    - retained under `C:\WhaleRunCache\r4-public10-20260701`
- Evidence gate: satisfied
- Related evidence:
  - E-006
  - E-007
- Conclusion: confirmed and repaired for the validation coverage loop. It does not close `processing-pipeline`, which now exposes a separate inspect no-progress loop.
- Repair design readiness: implemented
- Next step: fix inspect-node repeated read convergence and metrics extraction.
- Blocker:
  - R4-G remains open.
- Close reason:
  - E-007 demonstrates the same coverage block dropped from 94 to 0.

## Evidence E-006: processing-pipeline v2 loops on validation coverage after a successful edit
- Related hypotheses:
  - H-003
- Direction: supports
- Type: reproduction
- Source: `C:\WhaleRunCache\r4-public10-20260701\actual\processing-pipeline-v2\runs\terminal_bench__processing-pipeline\20260701-132747-117`
- Prediction or plan link:
  - H-003 predicts successful internal edit artifact loss will produce repeated `validation_test_missing_changed_artifact_coverage` blocks.
- Matched signal:
  - TaskSpace timed out.
  - `tool_action_blocked` count was 94 and all were `validation_test_missing_changed_artifact_coverage`.
  - Graph had nodes=3, edges=388, result_count=13.
  - Rollout size was 408,573,635 bytes.
- Correlation keys:
  - `20260701-132747-117`
- Raw content:
  ```text
  outcome_taskspace: agent_exec_timeout
  changed path observed in pair metrics: generate_report.sh
  validation_test_missing_changed_artifact_coverage: 94
  ```
- Interpretation: Validation was not blocked because the model never edited; it was blocked because the edit was not represented as changed artifact evidence in the active map.
- Time: 2026-07-01 13:27

## Evidence E-007: artifact backfill removes the validation coverage block on the same sample
- Related hypotheses:
  - H-003
- Direction: supports
- Type: validation
- Source: `C:\WhaleRunCache\r4-public10-20260701\actual\processing-pipeline-v3\runs\terminal_bench__processing-pipeline\20260701-141309-114`
- Prediction or plan link:
  - H-003 predicts coverage blocks disappear after backfilling changed artifacts into the successful implement result.
- Matched signal:
  - `validation_test_missing_changed_artifact_coverage` dropped from 94 to 0.
  - `tool_action_blocked` total dropped to 0.
  - Targeted Rust tests and build passed after the fix.
- Correlation keys:
  - `20260701-141309-114`
- Raw content:
  ```text
  validation_test_missing_changed_artifact_coverage: 0
  tool_action_blocked: 0

  cargo test -j1 -p codex-core observed_edit_backfill_records_changed_artifacts_on_implementation_result --lib
  PASS

  cargo build -j1 --profile dev-small -p codex-cli --bin whale
  PASS
  ```
- Interpretation: The artifact backfill fix has a concrete engineering benefit. The sample still timed out for a different reason: read-only inspect loop before any implement/test transition.
- Time: 2026-07-01 14:13

## Problem P-002: TaskSpace can spend a full public sample budget in read-only inspect without node progress
- Status: confirmed-and-repaired-for-inspect-transition; validation closeout remains open
- First observed: 2026-07-01 14:13
- Symptom:
  - After H-003 was repaired, `processing-pipeline` no longer repeated validation coverage blocks, but TaskSpace timed out with only one `inspect_code_context` node, 107 results, no edges, and no implement/test node.
- Impact:
  - R4-G public 10 comprehensive validation cannot close.
  - Runtime cost and rollout size remain high even without hard request caps.
  - Metrics are currently misleading because pair report can show `tool_call_count=0` while graph/rollout show many action-contract tool results.
- Current best root-cause candidates:
  - Inspect node lacks no-progress/read-duplication convergence that promotes to implement when enough local evidence exists.
  - Action-contract metrics extractor misses internal tool calls when normal `whale-exec.jsonl` evidence is absent or rollout is too large for full scan.
  - Terminal-Bench fixture exposure needs review because rollout shows reads of `task.yaml`.
- Evidence:
  - `C:\WhaleRunCache\r4-public10-20260701\actual\processing-pipeline-v3\runs\terminal_bench__processing-pipeline\20260701-141309-114`
- Next diagnostic:
  - Close the new validation-node infra failure path after `Bash/Service/CreateInstance/E_ACCESSDENIED`.
  - Prove metrics extractor writes rollout-derived tool counts in a fresh paired run.
  - Continue R4-G public 10 validation after the local validator infra path is controlled.

## Hypothesis H-004: inspect progress convergence was tied to provider budget snapshots
- Status: confirmed-and-repaired
- Parent: P-002
- Claim: The existing `inspect_progress_convergence` rule was only checked through provider-response/budget-snapshot paths. Action-contract tool results could be recorded without an active budget snapshot, so a single inspect node kept accumulating successful read/search results without transitioning to `implement_solution`.
- Layer: root-cause
- Factor relation: enabling
- Depends on:
  - P-002
- Rationale:
  - v4 real run still produced one inspect node and 94 results after the first code change. Its runtime artifacts had empty active-budget events, and no `forced_inspect_transition` event appeared.
- Falsifiable predictions:
  - If true: an action-contract run with no active budget snapshot can exceed the inspect contract hint without forced transition.
  - If fixed: record-main-tool-result should use a progress snapshot fallback that does not install or depend on active budget.
- Evidence gate: satisfied for inspect transition
- Related evidence:
  - E-008
  - E-009
- Conclusion: implemented a fallback progress snapshot and proved it in a real `processing-pipeline-v7` run. The sample now leaves inspect, edits `generate_report.sh`, and enters validation. The remaining timeout is a different validator infrastructure closeout problem.
- Repair design readiness: implemented and real-sample-proven for inspect transition.
- Next step: address validation infra recovery and public-10 closeout.
- Blocker:
  - `Bash/Service/CreateInstance/E_ACCESSDENIED` during `run_test` prevents clean validation-node closure in the bounded run.

## Evidence E-008: v4 proves the first inspect-convergence patch did not cover real action-contract runtime
- Related hypotheses:
  - H-004
- Direction: supports
- Type: reproduction
- Source: `C:\WhaleRunCache\r4-public10-20260701\actual\processing-pipeline-v4\runs\terminal_bench__processing-pipeline\20260701-151740-194`
- Matched signal:
  - `graph-health.json`: node_count=1, edge_count=0, result_count=94.
  - `active-budget-events.jsonl`: empty.
  - `rollout.jsonl`: repeated `read_file`/`list_files` in `node-1`; no `forced_inspect_transition`.
- Raw content:
  ```text
  node_count: 1
  result_count: 94
  rollout_bytes: 351,884,108
  active-budget-events.jsonl length: 0
  ```
- Interpretation: checking `provider_request_budget_snapshot()` inside the tool-result path was not sufficient because real action-contract runs may have no active budget snapshot.
- Time: 2026-07-01 15:38

## Evidence E-009: fallback progress snapshot is unit-tested without active budget
- Related hypotheses:
  - H-004
- Direction: supports
- Type: validation
- Source: `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Matched signal:
  - `record_main_tool_result_with_class` now falls back to `provider_request_progress_snapshot_for_node`.
  - The test clears `state.active_budget = None` and still expects `inspect_progress_convergence` to auto-finish into implementation after enough inspected code evidence.
- Raw content:
  ```text
  cargo test -j1 -p codex-core inspect_progress_convergence_force_finishes_after_contract_hint --lib
  PASS

  cargo fmt --all --check
  PASS

  cargo build -j1 --profile dev-small -p codex-cli --bin whale
  PASS
  ```
- Interpretation: The mechanism no longer depends on installing an active budget, matching the design constraint that profile/budget hints must not gate session progress.
- Time: 2026-07-01 16:20

## Evidence E-010: processing-pipeline v7 proves inspect transition and real edit with current binary
- Related hypotheses:
  - H-004
- Direction: supports
- Type: real-sample validation
- Source: `C:\WhaleRunCache\r4-public10-20260701\actual\processing-pipeline-v7\runs\terminal_bench__processing-pipeline\20260701-163507-220`
- Matched signal:
  - `forced_inspect_transition` appears in rollout with `trigger:inspect_progress_convergence`.
  - The transition binds `node-1` to `node-2` at `request_count:13`, `max_requests:20`.
  - `graph-health.json`: `node_count=3`, `edge_count=2`, `result_count=17`.
  - `metrics.json`: `changed_paths=[generate_report.sh]`.
  - `rollout.jsonl`: `apply_patch` succeeds and `taskspace_control finish_node` creates `node-3`.
- Raw content:
  ```text
  trace kind: forced_inspect_transition
  tags: trigger:inspect_progress_convergence, request_count:13, max_requests:20,
        source_node_kind:inspect_code_context, next_node_kind:implement_solution,
        bound_next_node_id:node-2
  apply_patch: Success. Updated generate_report.sh
  run_test: local_validator_infra_failure: Bash/Service/CreateInstance/E_ACCESSDENIED
  ```
- Interpretation: The read-only inspect loop is no longer the current blocker. The next blocker is validator infrastructure failure after implementation.
- Time: 2026-07-01 16:45

## Evidence E-011: install script previously installed stale debug binary instead of dev-small
- Related hypotheses:
  - H-004
- Direction: explains false-negative run
- Type: harness integrity repair
- Source: `scripts/install-whale-local.ps1`
- Matched signal:
  - Before repair, installed binary hash was `5DCBE599...` from `D:\BuildCache\whalecode\cargo-target\debug\whale.exe`.
  - Current build artifact hash is `29A68B8C...` from `D:\BuildCache\whalecode\cargo-target\dev-small\whale.exe`.
  - `processing-pipeline-v6` used the old binary, so it could not validate H-004.
- Raw content:
  ```text
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-whale-local.ps1 -InstallDir D:\whalecode-alpha\target\install-whale-local-selftest-2
  Source: D:\BuildCache\whalecode\cargo-target\dev-small\whale.exe
  Hash: 29A68B8C57B425DFBFA326B23C9877D0E45B0BE51119A9D5CD82740181A9CB06
  WhaleBinaryAttestation: D:\whalecode-alpha\target\install-whale-local-selftest-2\whale.exe.build-attestation.json
  ```
- Interpretation: R4 benchmark integrity requires install/attestation to point at the actual current build artifact. The script now selects the latest candidate when `-BinaryPath` is omitted and refreshes attestation after install.
- Time: 2026-07-01 16:50

## Evidence E-012: metrics extractor now counts action-contract tools from rollout
- Related hypotheses:
  - P-002
- Direction: supports
- Type: metrics repair validation
- Source: `scripts/taskspace-benchmark/lib/metrics-extractor.ps1`
- Matched signal:
  - `Get-TaskspaceRolloutToolStats` counts non-control rollout tool calls, failed calls, and `taskspace_control` separately.
  - The focused gate covers shell success, apply_patch success, test infra failure, and taskspace_control separation.
  - v7 rollout recomputation returns `Completed=15`, `Failed=1`, `Control=2`.
- Raw content:
  ```text
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-r4-metrics-extractor-large-rollout.ps1
  PASS: R4 metrics extractor large rollout gate passed

  Get-TaskspaceRolloutToolStats(v7 rollout)
  Completed: 15
  Failed: 1
  Control: 2
  Availability: measured
  ```
- Interpretation: Future paired reports can avoid the misleading `tool_call_count=0` when `whale-exec.jsonl` lacks action-contract internals but rollout is available for scan.
- Time: 2026-07-01 16:55

## Hypothesis H-005: local validator infra failure closeout depends on an unnecessary model turn

- Related problems:
  - P-002
- Status: confirmed-and-repaired-by-unit-tests; real-sample rerun pending
- Claim: after TaskSpace records a definitive local validator infrastructure failure from a validation tool result, runtime still relies on a later model `state_commit` or `blocked` action to close the validation node. In bounded runs this can consume the remaining wall-clock window even though the tool result already contains sufficient evidence for map-level invalidation and blocking.
- Predictions:
  - A `run_test` result containing `local_validator_infra_failure` or `Bash/Service/CreateInstance/E_ACCESSDENIED` will be visible in the map as a validation tool failure before timeout.
  - Existing runtime code can already block validation nodes and create rework nodes for local infra failures, but the automatic path is only attached to `state_commit`.
  - Moving the deterministic invalidation/blocking step into `record_main_tool_result_with_class` will let the same tool result produce `ResultValidity::Invalid` plus a blocked validation node without another model request.
- Diagnostic evidence plan:
  - Inspect runtime code around tool result recording and existing local-infra auto-block after state_commit.
  - Add focused unit tests for direct local-infra tool-result closeout, action-contract `run_test` closeout, and changed-artifact rework routing.
  - Re-run a bounded `processing-pipeline` sample after building/installing the updated binary to confirm real-sample closeout.
- Conclusion: confirmed by E-013 for the code path and unit behavior. Real-sample evidence is still required before marking the R4 public-sample closeout done.

## Evidence E-013: local infra validation failures now close at tool-result recording time

- Related hypotheses:
  - H-005
- Direction: supports
- Type: repair validation
- Source: `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Matched signal:
  - `record_main_tool_result_with_class` now checks failed `Build`/`Test` results immediately after recording the standard tool feedback.
  - When the current validation node contains a local validator infra failure, runtime marks that tool result `invalid` via `ResultValidityChanged` and reuses `block_main_node` for the existing blocked-validation/rework semantics.
  - The fix does not introduce a separate tool-feedback path; it operates after the same `MainToolCall` result has been persisted to the map.
- Raw content:
  ```text
  cargo test -j1 -p codex-core local_infra_tool_result_auto_blocks_validation_node --lib
  PASS

  cargo test -j1 -p codex-core action_contract_run_test_local_infra_result_auto_blocks_validation --lib
  PASS

  cargo test -j1 -p codex-core local_infra_validation_block_routes_unvalidated_changed_artifact_to_rework --lib
  PASS

  cargo test -j1 -p codex-core local_infra --lib
  PASS: 6 passed

  cargo fmt --all --check
  PASS
  ```
- Interpretation: the current blocker is no longer "TaskSpace needs the model to restate a deterministic validator infra failure before the map can close the validation node." The remaining required proof is a fresh real-sample run using the updated binary.
- Time: 2026-07-01 21:05

## Hypothesis H-006: inspect convergence must force unread referenced scripts before implementation

- Related problems:
  - P-002
- Status: confirmed-and-repaired-by-unit-tests-and-real-sample
- Claim: after reading an orchestrator script, TaskSpace can know that referenced scripts remain unread. If the active inspect node does not force those reads, the model may keep rereading already inspected files or listing the directory instead of completing the evidence set.
- Evidence:
  - `processing-pipeline-v8` timed out with `exec_timed_out=true`, `changed_paths=[]`, `node_count=1`, and no `forced_inspect_transition`.
  - The rollout had repeated reads of `run_pipeline.sh`, `collect_data.sh`, and `process_data.sh`, but no successful read of `generate_report.sh`.
  - `run_pipeline.sh` referenced `./generate_report.sh`; runtime correctly recognized this as missing referenced evidence, but the prompt/gate did not force the missing read.
- Repair:
  - `prepare_main_tool_call` now blocks non-read actions on an inspect node while referenced scripts remain unread.
  - The action-contract prompt injects `TaskSpaceActionContractInspectMissingScriptsV1` with the exact required `read_file` target list.
- Validation:
  ```text
  cargo test -j1 -p codex-core inspect_unread_referenced_script_gate_requires_missing_read --lib
  PASS

  cargo test -j1 -p codex-core taskspace_action_contract_inspect_missing_scripts_narrows_to_read_file --lib
  PASS
  ```
- Real-sample result:
  - `processing-pipeline-v9` and later runs read `generate_report.sh`, produced `forced_inspect_transition`, and reached implementation.
- Time: 2026-07-01 21:30

## Hypothesis H-007: local validator infra failures need recoverability classification

- Related problems:
  - P-002
- Status: confirmed-and-repaired-by-unit-tests-and-real-sample
- Claim: treating all local validator infra failures the same causes incorrect rework. `InvalidEndOfLine` is recoverable by changing command syntax, but `Bash/Service/CreateInstance/E_ACCESSDENIED` is an unavailable executor/service failure and should close validation as infrastructure-blocked instead of creating implementation rework.
- Evidence:
  - `processing-pipeline-v9` reached `generate_report.sh` and hit `local_validator_infra_failure: Bash/Service/CreateInstance/E_ACCESSDENIED`.
  - Runtime marked the validation result invalid and blocked node-3, but then created node-4 `implement_solution` because the previous policy routed any local infra failure with changed artifacts to rework.
  - The next prompt told the model to patch implementation or run platform-compatible syntax, which is valid for `InvalidEndOfLine` but wrong for `E_ACCESSDENIED`.
- Repair:
  - Added `local_validator_infra_failure_can_rework_command`.
  - Validation rework routing now only applies to recoverable host-shell command syntax failures such as `InvalidEndOfLine`.
  - Unrecoverable executor/service failures such as `E_ACCESSDENIED` mark the validation result invalid and close the validation node without creating rework.
  - Prompt recovery now distinguishes recoverable command syntax from unrecoverable executor/service failures.
- Validation:
  ```text
  cargo test -j1 -p codex-core access_denied_local_infra_blocks_validation_without_rework_after_changed_artifact --lib
  PASS

  cargo test -j1 -p codex-core local_infra_validation_block_routes_unvalidated_changed_artifact_to_rework --lib
  PASS

  cargo test -j1 -p codex-core local_infra --lib
  PASS: 8 passed
  ```
- Real-sample result:
  - `processing-pipeline-v10`: `exec_timed_out=false`, `open_leaf=0`, `node_count=3`, `blocked_node_ratio=0.3333`, `invalid=1`.
  - `processing-pipeline-v11`: same closed graph shape, plus `TaskSpaceActionContractClosedValidationV1=1` and final task completion message summarizing the local infra blocker.
- Time: 2026-07-01 22:00

## Evidence E-014: processing-pipeline v11 closes the TaskSpace graph after local infra failure

- Related hypotheses:
  - H-006
  - H-007
- Direction: supports
- Type: real-sample validation
- Source: `C:\WhaleRunCache\r4-public10-20260701\actual\processing-pipeline-v11\runs\terminal_bench__processing-pipeline\20260701-214838-298`
- Matched signal:
  - `exec_timed_out=false`
  - `open_leaf=0`
  - `node_count=3`
  - `edge_count=2`
  - `result_count=15`
  - `invalid=1`
  - `TaskSpaceActionContractClosedValidationV1=1`
  - final task message: local validator infrastructure failure prevents shell-script execution; implementation evidence and `generate_report.sh` patch are recorded.
- Raw content:
  ```text
  local_validator_infra_failure: Bash/Service/CreateInstance/E_ACCESSDENIED
  result_validity_changed node-3 result-14 validity=invalid
  node_status_changed node-3 running -> blocked
  task_complete last_agent_message=blocked_by_taskspace_action_contract: Local validator infrastructure failure...
  ```
- Interpretation: the current TaskSpace tool-feedback/control path now reaches a stable closed graph for this sample instead of timing out or routing an unrecoverable executor failure back into implementation rework.
- Time: 2026-07-01 22:05

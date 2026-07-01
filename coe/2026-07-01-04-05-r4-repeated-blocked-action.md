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

## Problem P-003: failed validation can still branch back into inspect rediscovery

- Status: investigating
- Symptom:
  - In the R4 public 10 actual run, `csv-to-parquet` had `outcome_standard=solved` but `outcome_taskspace=engineering_unclean`.
  - TaskSpace timed out after a failed validation result instead of routing the known failure into implementation rework.
- Expected behavior:
  - Once a smoke/regression validation node records a non-infra failed test/build result, the next structural step must be implementation rework that uses the failed validator evidence.
  - TaskSpace must not create a fresh inspect node that reopens discovery after the failure is already known.
- Actual behavior:
  - The map recorded the failed `run_test` result, including `IndentationError: unexpected indent` from `convert_csv.py`.
  - The runtime then allowed a new `inspect_code_context` node, which reread `convert_csv.py` and other files until the session timed out.
- Fix criteria:
  - A focused unit test proves `create_node(inspect_code_context)` is rejected after a failed validation result.
  - The same `csv-to-parquet` public sample no longer times out on this branch.

## Hypothesis H-008: create_node lacks the failed-validation rework guard

- Related problems:
  - P-003
- Status: confirmed
- Claim:
  - Existing validation-failure routing only blocks read/search tool calls inside the validation node and rejects state commits that cite failed validation without rework. It does not guard the structural `create_node` path, so the model can create a new inspect node after the failed validation result is already present.
- Predictions:
  - Code should contain a guard in `prepare_main_tool_call` for read/search after failed validation.
  - Code should not contain an equivalent guard in `create_node_for_main_with_kind`.
  - Real sample map should show a failed validation result followed by a new inspect node.
- Diagnostic evidence plan:
  - Inspect `runtime.rs` around tool-call preparation, node creation, and validation-failure helpers.
  - Inspect the `csv-to-parquet` observability map and graph health for node/result order.
- Evidence:
  - `prepare_main_tool_call` contains `validation_failed_requires_rework_routing` for read/search on a validation node.
  - `create_node_for_main_with_kind` proceeds from active map lookup into budget/default-dependency logic without checking pending failed validation results.
  - `csv-to-parquet-v2` graph had node-3 `smoke_test` with failed `run_test` result and node-4 `inspect_code_context` running afterward; `open_leaf_nodes=2`, `unreviewed_result_count=11`, `exec_timed_out=true`.
- Conclusion:
  - The root cause is a missing structural guard at node creation, not missing tool-result visibility.
- Time: 2026-07-01 22:40

## Evidence E-015: create_node failed-validation guard blocks rediscovery and real sample routes to rework

- Related hypotheses:
  - H-008
- Direction: supports
- Type: repair validation
- Source:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `C:\WhaleRunCache\r4-public10-20260701\actual\csv-to-parquet-v3\runs\terminal_bench__csv-to-parquet\20260701-225045-476`
- Matched signal:
  - `create_node_for_main_with_kind` now rejects any non-`implement_solution` node while an unresolved smoke/regression validation node has a failed non-infra test/build result or recoverable local-validator infra failure.
  - The focused unit test proves `create_node(inspect_code_context)` is rejected after a failed validation result with `validation_failed_requires_rework_routing`.
  - In the `csv-to-parquet-v3` rerun, node-3 `smoke_test` failed with `IndentationError`, then node-3 was blocked and node-4 `implement_solution` was created. The old failure mode, a fresh inspect node after the failed validation, did not recur.
- Raw content:
  ```text
  cargo test -j1 -p codex-core validation_node_failed_test_blocks_inspect_node_rediscovery --lib
  PASS

  cargo test -j1 -p codex-core validation_node_failed_test_blocks_read_rediscovery --lib
  PASS

  cargo test -j1 -p codex-core state_commit_rejects_failed_validation_result_without_rework_transition --lib
  PASS

  cargo test -j1 -p codex-core local_infra_validation_block_routes_unvalidated_changed_artifact_to_rework --lib
  PASS

  cargo fmt --all --check
  PASS

  cargo build -j1 --profile dev-small -p codex-cli --bin whale
  PASS
  ```
- Interpretation:
  - P-003's specific create-node rediscovery defect is repaired.
  - The same public sample then exposed a separate no-action convergence defect on node-6 `implement_solution`: the runtime projected `next_valid_actions` requiring edit or finish, but the model produced no tool/control result until agent timeout.
- Time: 2026-07-01 23:15

## Problem P-004: implement rework can loop with tool-free no-action after clear next action

- Status: investigating
- Symptom:
  - In `csv-to-parquet-v3`, TaskSpace reached node-6 `implement_solution` after node-5 validation failed with `FileNotFoundError: data.csv`.
  - The compact projection told the model the current node was `implement_solution` and `next_valid_actions` were `edit implementation artifacts` or `taskspace_control(action=finish_node) into smoke_test/regression_test`.
  - The provider kept receiving tool-free action-contract requests and no node-6 result was recorded before timeout.
- Expected behavior:
  - After a blocked validation node creates an implement rework node with a concrete failure summary, no-action recovery should force either an edit-capable tool call or an explicit blocked-with-evidence result.
- Actual behavior:
  - node-6 has zero results and remained running until the agent execution timeout.
  - Trace events show repeated `provider_request_budget` entries on node-6 with `tools_present:false`, `request_shape_classifier:tool_free_action_contract`, and no actionable TaskSpace result.
- Fix criteria:
  - Runtime should not spend many model requests in an implement node with known failed-validation evidence and no result.
  - A focused test should prove implement no-action recovery after blocked validation becomes concrete edit/block guidance, not generic follow-up.

## Hypothesis H-009: blocked validation rework was not classified as implementation evidence

- Related problems:
  - P-004
- Status: confirmed-and-repaired-by-unit-and-real-sample-tests
- Claim:
  - The runtime only treated successful inspect/read evidence or repeated implement-node reads as `implementation_needs_edit`. A rework node created from a blocked validation node had clear failed-validation evidence in its context, but that evidence did not set `current_node_has_dependency_working_evidence` and did not make `current_main_implement_progress_needs_edit()` true.
- Predictions:
  - The real sample should show the rework node created correctly but with zero results until timeout.
  - Code should gate implement read/search on inspect dependency evidence and progress count, but not on blocked validation dependency evidence.
  - A unit reproduction should fail before the fix by allowing read/search after blocked validation rework, and pass after the fix by requiring edit/block.
- Evidence:
  - `csv-to-parquet-v3` node-5 `smoke_test` blocked with `FileNotFoundError: data.csv`; node-6 `implement_solution` was created, but recorded zero results and remained running until timeout.
  - `provider_request_budget` traces on node-6 repeatedly had `tools_present:false` and `request_shape_classifier:tool_free_action_contract`, while the projection still allowed read/search/edit/control instead of narrowed edit/block.
  - `prepare_main_tool_call` and `current_main_implement_progress_needs_edit` checked `implement_node_has_dependency_working_evidence` and progress count, but blocked validation evidence was a separate shape and did not satisfy either condition.
- Repair:
  - Added `implement_node_dependency_validation_rework_summary`.
  - Treat blocked smoke/regression validation evidence as implementation rework evidence when deriving provider snapshots, no-action recovery, and implement read/search gating.
  - Preserve the failed validation result and blocker summary as the working evidence summary for the active rework node.
- Validation:
  ```text
  cargo test -j1 -p codex-core blocked_validation_rework_requires_edit_before_rediscovery --lib
  PASS

  cargo test -j1 -p codex-core blocked_validation_rework_can_edit_without_reviewing_blocker_result --lib
  PASS

  cargo test -j1 -p codex-core validation_after_rework_can_test_without_reviewing_origin_blocker_result --lib
  PASS

  cargo fmt --all --check
  PASS
  ```
- Interpretation:
  - The runtime now classifies failed-validation rework as concrete implementation evidence and blocks rediscovery reads before an edit. Real-sample validation is still required before P-004 can be marked fixed end-to-end.
- Time: 2026-07-01 23:40

## Hypothesis H-010: action-contract apply_patch hunk mismatch feedback was under-structured

- Related problems:
  - P-004
- Status: confirmed-and-repaired-by-unit-tests
- Claim:
  - After P-004's first repair, TaskSpace correctly forced edit attempts, but `apply_patch` failures with `Failed to find expected lines` were summarized as generic tool failures. The model repeated mismatching hunks instead of using a different patch strategy such as exact-context update or full small-file replacement.
- Evidence:
  - `csv-to-parquet-v4` improved over v3: TaskSpace tool calls dropped from 17 to 8, nodes from 6 to 4, and node-4 produced edit attempts instead of no-action.
  - node-4 result-8 and result-9 were both failed `apply_patch` calls with `Failed to find expected lines in V:\app\convert.py`.
  - No successful edit or validation rerun followed; node-4 remained running until timeout.
- Repair:
  - Added `apply_patch_expected_lines_mismatch` classification in action-contract tool feedback.
  - Extracts and normalizes the target path from Windows `/app` style paths, including `V:\app\convert.py`.
  - Provides an explicit next action: do not repeat the same hunk; use exact existing context or, for small/generated files with known intended contents, `*** Delete File` plus `*** Add File`.
- Validation:
  ```text
  cargo test -j1 -p codex-core action_contract_prompt_structures_apply_patch_expected_lines_feedback --lib
  PASS

  cargo test -j1 -p codex-core taskspace_apply_patch_expected_lines_target_is_detected --lib
  PASS

  cargo test -j1 -p codex-core edit_failure_recovery_preserves_failed_tool_feedback --lib
  PASS

  cargo test -j1 -p codex-core blocked_validation_rework_requires_edit_before_rediscovery --lib
  PASS

  cargo fmt --all --check
  PASS
  ```
- Interpretation:
  - The tool feedback path now carries a semantically distinct patch-hunk mismatch signal and a concrete recovery strategy. Real-sample validation is still required.
- Time: 2026-07-01 23:58

## Hypothesis H-011: failed validation nodes could repeat validation instead of yielding to rework

- Related problems:
  - P-004
- Status: confirmed-and-repaired-by-unit-tests
- Claim:
  - The runtime blocked read/search rediscovery after a smoke/regression validation node had a failed non-infra test result, but it still allowed another non-control `run_test` call in the same failed validation node. This let TaskSpace spend the remaining session budget repeating known-failed validation rather than routing the concrete failure back to implementation rework.
- Evidence:
  - In `csv-to-parquet-v5`, node-2 successfully created `convert.py`.
  - node-3 `smoke_test` then recorded repeated failed validation results:
    - result-8: `pytest` collected zero tests and emitted cache permission warnings.
    - result-9: `python convert.py` failed with `FileNotFoundError: data.csv`.
    - result-10: another validation call hit the same failure class.
  - The session ended with TaskSpace timeout instead of forcing rework from the first actionable validation failure.
- Root cause:
  - `prepare_main_tool_call` had a failed-validation guard, but the guard only rejected `Read` and `Search`. A repeated `RunTest` was neither read nor search, so it remained allowed even after the node had already produced a non-infra failed test result.
- Repair:
  - Changed the guard to reject any non-`Control` action after a validation node has a failed non-infra result.
  - Kept control actions available so the runtime/model can finish or block the validation node and route the failure into implementation rework.
  - Updated the rejection message to cover validation retry and rediscovery, not only read/search.
- Validation:
  ```text
  cargo test -j1 -p codex-core validation_node_failed_test_blocks_repeated_validation --lib
  PASS

  cargo test -j1 -p codex-core validation_node_failed_test_blocks_read_rediscovery --lib
  PASS

  cargo fmt --all --check
  PASS
  ```
- Interpretation:
  - The runtime now treats the first actionable validation failure as a phase transition signal instead of allowing repeated validation calls inside the same failed node. Real-sample validation is still required.
- Time: 2026-07-02 00:20

## Hypothesis H-012: failed validation routing still depended on model-driven control after tool blocking

- Related problems:
  - P-004
- Status: confirmed-and-repaired-by-unit-tests
- Claim:
  - Blocking repeated validation tools was insufficient because the failed validation node remained running. The model-visible recovery message told the agent to block/finish the node and create rework, but runtime did not perform the state transition. When the model kept attempting the blocked test, TaskSpace consumed requests until agent timeout.
- Evidence:
  - In `csv-to-parquet-v6`, TaskSpace produced only `convert.py` as changed path and the artifact inventory showed the intended implementation file.
  - The run still timed out after 360 seconds with `open_leaf_nodes=1`; node-3 `smoke_test` remained `running`.
  - node-3 recorded one failed validator result (`result-11`) and then 16 `tool_action_blocked` events for repeated test attempts.
  - Request summary showed 29 model requests, about 3.49M input tokens, and about 3.47M cached input tokens, meaning the cache path was stable but the session still wasted repeated recovery requests.
  - The last actionability event preview showed the runtime feedback: "TaskSpace blocked this test because validation node `node-3` already has a failed test/build result `result-11`..."
- Root cause:
  - `prepare_main_tool_call` rejected repeated non-control actions on a failed validation node, but only returned a gate error plus blocked-tool event. It did not invoke the existing `block_main_node` path that records a blocker, changes the validation node to `blocked`, and creates the dependent `implement_solution` rework node.
- Repair:
  - Reused the existing `block_main_node` transition from the failed-validation tool gate.
  - The blocked tool feedback remains model-visible, but runtime now also records the blocker and routes to the runtime-created implementation rework node.
  - Recovery text now tells the model to continue on the runtime-created rework node instead of asking it to perform the routing manually.
- Validation:
  ```text
  cargo test -j1 -p codex-core validation_node_failed_test_blocks_repeated_validation --lib
  PASS

  cargo test -j1 -p codex-core validation_node_failed_test_blocks_read_rediscovery --lib
  PASS

  cargo test -j1 -p codex-core blocked_validation_rework_requires_edit_before_rediscovery --lib
  PASS

  cargo fmt --all --check
  PASS
  ```
- Interpretation:
  - Failed validation recovery is now a runtime state transition instead of a model-only instruction. Real-sample validation is still required.
- Time: 2026-07-02 00:45

## Hypothesis H-013: successful validation finish rejected unconfirmed but valid test output

- Related problems:
  - P-004
- Status: confirmed-and-repaired-by-unit-tests
- Claim:
  - After implementation and validation succeeded, TaskSpace still rejected `finish_node` on the validation node because validation completion required a satisfied test/validator success criterion before the auto-accept path had created one. The auto-accept path also treated successful `Test/Build` tool results without explicit "passed" wording as unconfirmed, even when the tool succeeded and had no failure signal.
- Evidence:
  - In `csv-to-parquet-v7`, TaskSpace edited `convert.py`, produced `data.parquet`, and recorded a successful validation result:
    - `Conversion completed: data.parquet created`
    - `5 rows written`
  - The rollout then emitted repeated `taskspace_control finish_node` calls for `node-3`.
  - Each was rejected with: "cannot be completed without a satisfied success criterion tied to this validation node's successful test/build result."
  - The run timed out with only 5 tool calls and no repeated blocked-action loop, showing this was a new closeout gate problem rather than the previous validation-retry bug.
- Root cause:
  - `finish_main_node_with_next` called `validate_existing_validation_criteria_for_finish` before `auto_accept_validation_result_for_finish`, so the auto-accept path could not create the missing satisfied criterion before the gate checked it.
  - `node_result_is_successful_validation` also required explicit success wording such as "passed"; benchmark validators often express success through exit status plus task-specific output, so valid successful `run_test` results were classified as unconfirmed.
- Repair:
  - Reordered validation finish so `auto_accept_validation_result_for_finish` runs before the completion evidence gate.
  - Introduced a shared `validation_output_supports_successful_tool_result` predicate used by both trace tagging and validation evidence recognition.
  - The predicate trusts successful `Test/Build` tool execution unless the output contains strong failure signals or matches the existing pure-diagnostic-output guard.
- Validation:
  ```text
  cargo test -j1 -p codex-core validation --lib
  PASS, 55 passed
  ```
- Interpretation:
  - Successful validation closeout no longer depends on the model manually recording state_commit criteria after a successful validator. The remaining required proof is a real rerun of `csv-to-parquet`.
- Time: 2026-07-02 01:05

## Hypothesis H-014: failed validation results must route to rework at record time, not on the next model action

- Related problems:
  - P-004
- Status: confirmed-and-repaired-by-unit-tests
- Claim:
  - The H-012 fix still left a timeout window: runtime routed a failed validation node to rework only when the model attempted another non-control action after the failed result. If the session timed out immediately after recording the failed validation result, the validation node stayed running and graph health reported an open leaf.
- Evidence:
  - In `csv-to-parquet-v8`, TaskSpace reached a later and narrower failure mode:
    - `tool_call_count=13`, `node_count=5`, `edge_count=4`, `blockedToolActions=1`;
    - rollout request count was 19 with about 2.28M input tokens and about 2.25M cached input tokens, so cache behavior remained stable;
    - `node-5` remained `smoke_test/running` with `result-16` failed validation;
    - graph health reported `open_leaf_nodes=1`;
    - the agent timed out at 360s before another action could trigger H-012's delayed failed-validation gate.
  - The final `convert.py` still read `data.csv`, and `result-16` showed `FileNotFoundError: 'data.csv'`.
- Root cause:
  - `record_main_tool_result_with_class` already auto-blocked local-infra validation failures immediately, but ordinary non-infra failed validation results were only observed and left for a later `prepare_main_tool_call` gate.
  - This split made failed-validation routing depend on one extra model turn.
- Repair:
  - Added `current_main_validation_node_failed_noninfra_summary`.
  - Extended `record_main_tool_result_with_class` so a failed non-infra `Test/Build` result on the current validation node immediately calls the existing `block_main_node` transition.
  - Updated tests that previously expected the next read/test/create-node action to trigger routing; they now assert immediate `blocked` validation plus bound `implement_solution` rework.
- Validation:
  ```text
  cargo test -j1 -p codex-core validation_node_failed_test --lib
  PASS, 3 passed

  cargo test -j1 -p codex-core validation --lib
  PASS, 55 passed

  csv-to-parquet-v9
  RunDir: C:\WhaleRunCache\r4-public10-20260701\actual\csv-to-parquet-v9\runs\terminal_bench__csv-to-parquet\20260702-021506-444
  TaskSpace exec_timed_out=false
  TaskSpace open_leaf_nodes=0
  TaskSpace model_request_count=8 from rollout
  TaskSpace input_tokens=959,735
  TaskSpace cached_input_tokens=947,456
  ```
- Interpretation:
  - Failed validation recovery is now an atomic runtime transition at result-record time. Compared with v8, the same sample no longer timed out, open leaf dropped from 1 to 0, rollout model requests dropped from 19 to 8, and input tokens dropped from about 2.28M to about 0.96M.
- Time: 2026-07-02 01:55

## Hypothesis H-015: remaining csv-to-parquet wrong is a path-semantics issue, not a TaskSpace state-machine stall

- Related problems:
  - P-004
- Status: confirmed-and-repaired-by-harness-tests
- Claim:
  - After H-014, `csv-to-parquet-v9` no longer shows timeout, open leaf, or tool feedback loss. The remaining wrong result comes from the agent choosing a local-harness path default (`task-deps/data.csv`) while the public validator compares against `/app/data.csv`.
- Evidence:
  - `csv-to-parquet-v9` TaskSpace side:
    - `exec_exit_code=0`;
    - `exec_timed_out=false`;
    - `open_leaf_nodes=0`;
    - `tool_call_count=5`;
    - `nodes=3`, `edges=2`;
    - `public_validation_exit_code=1`;
    - `outcome_taskspace=wrong`.
  - Final `convert.py` defaulted to `task-deps/data.csv`:
    - `input_csv = sys.argv[1] if len(sys.argv) > 1 else 'task-deps/data.csv'`
    - `output_parquet = sys.argv[2] if len(sys.argv) > 2 else 'data.parquet'`
  - Public validation failed `test_data_matches` because it tried `pd.read_csv('/app/data.csv')` and hit `FileNotFoundError`.
- Root cause candidate:
  - The agent-visible local workspace exposes `task-deps/data.csv`, while the task instruction and public validator semantics refer to `/app/data.csv`.
  - TaskSpace's failed local validation feedback pushed the model toward local path repair instead of preserving the container-visible `/app/data.csv` contract.
- Root cause:
  - The Terminal-Bench adapter correctly adapted the prompt to say the current working directory is `/app`, but fixture materialization only copied public files as source-tree paths.
  - The validator builds the Docker image, where `Dockerfile` executes `COPY task-deps/data.csv ./`, but then runs the validator with `-v ${repoDockerPath}:/app`. That bind mount replaces the image's `/app` contents with the agent repo, so Dockerfile-created `/app/data.csv` disappears unless the fixture also projects it into the repo root.
  - Standard solved v9 by creating root `data.csv`; TaskSpace did not, because the local workspace exposed `task-deps/data.csv` and the failed validation feedback made that path appear to be the available repair target.
- Repair:
  - Added a Terminal-Bench fixture projection helper that parses simple public `COPY`/`ADD` Dockerfile file copies and materializes their `/app` destinations into the agent-visible fixture.
  - The projection is adapter-scoped, not TaskSpace runtime-scoped, so runtime tool feedback remains benchmark-agnostic.
  - Projection metadata is recorded under `external_benchmark.adapter_metadata.agent_app_fixture_projection`.
- Validation:
  ```text
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1
  PASS

  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-terminal-bench-adapter-harness.ps1
  PASS

  csv-to-parquet adapter materialization check
  root_data_csv=true
  task_deps_data_csv=true
  projected=true
  projection_destination=data.csv
  allowlist_contains_data_csv=true
  ```
- Remaining validation:
  - Run `csv-to-parquet` through the paired agent benchmark again and confirm TaskSpace no longer fails public validation due missing `/app/data.csv`.
- Time: 2026-07-02 02:25

## Hypothesis H-016: successful validation result must be accepted at record time

- Related problems:
  - P-004
- Status: confirmed-and-repaired-by-unit-tests
- Claim:
  - After H-015, `csv-to-parquet-v10` produced the correct files but still timed out because TaskSpace left the smoke-test node running. The successful test result was present, but success criterion/result validity were not recorded until `finish_node`; the action-contract recovery loop could hit completion gates before the auto-accept finish path had a chance to mutate state.
- Evidence:
  - `csv-to-parquet-v10` TaskSpace side:
    - `exec_timed_out=true`;
    - `tool_call_count=0` in pair metrics because `whale-exec.jsonl` was empty, while `rollout.jsonl` was 77MB;
    - graph health showed `node-3` as `smoke_test/running`, `open_leaf_nodes=1`, `edge_count=313`;
    - final app had `data.csv`, `convert.py`, and `data.parquet`.
  - `node-3` result `result-6` was a successful test:
    - command: `python convert.py; ... assert os.path.exists('data.parquet')`;
    - output: `Test passed`;
    - action class: `test`;
    - tool success: `true`.
  - The provider repeatedly called `finish_node`, but runtime returned:
    - `cannot be completed without a satisfied success criterion tied to this validation node's successful test/build result`.
- Root cause:
  - Successful validation auto-accept ran inside `finish_main_node`, but not when the successful test/build result was first recorded.
  - Other recovery/action-contract paths can evaluate validation completion state before the finish path mutates result validity and success criteria, so a valid test result can be visible while the completion gate still rejects closeout.
- Repair:
  - Extended `record_main_tool_result_with_class` so a successful `Test` or `Build` result on a validation node immediately reuses the existing validation auto-accept logic.
  - The runtime now records accepted result validity and a satisfied success criterion at the same time it records successful validation evidence.
- Validation:
  ```text
  cargo test -j1 -p codex-core successful_validation_tool_result_auto_accepts_at_record_time --lib
  PASS

  cargo test -j1 -p codex-core validation_node_failed_test --lib
  PASS, 3 passed

  cargo test -j1 -p codex-core validation --lib
  PASS, 56 passed

  cargo fmt --all --check
  PASS

  cargo build -j1 --profile dev-small -p codex-cli --bin whale
  PASS
  ```
- Remaining validation:
  - Rerun `csv-to-parquet` as v11 with the new binary and confirm TaskSpace no longer times out after a successful smoke test.
- Time: 2026-07-02 03:00

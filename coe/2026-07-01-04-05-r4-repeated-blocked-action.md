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
- Status: confirmed-and-repaired-by-real-sample
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

  csv-to-parquet-v11
  RunDir: C:\WhaleRunCache\r4-public10-20260701\actual\csv-to-parquet-v11\runs\terminal_bench__csv-to-parquet\20260702-030114-692
  outcome_standard=solved
  outcome_taskspace=solved
  taskspace_exec_timed_out=false
  taskspace_public_validation_exit_code=0
  taskspace_hidden_oracle_exit_code=0
  taskspace_open_leaf_nodes=0
  taskspace_edges=2
  taskspace_tool_call_count=4
  taskspace_wall_time_ms=93,724
  standard_wall_time_ms=42,415
  taskspace_wall_time_ratio=2.21
  ```
- Interpretation:
  - The same sample no longer times out after successful smoke validation. Compared with v10, open leaf dropped from 1 to 0 and edge count dropped from 313 to 2.
  - The harness process still returned exit code 1 because E3/audit scoring gates were intentionally unmet (`repeats_lt_3`, `audit_review_missing`, external validator fidelity not E3 eligible). Pair-level business outcomes were solved on both sides.
- Time: 2026-07-02 03:00

## Hypothesis H-017: closed-validation action-contract must not hide ready recovery nodes

- Related problems:
  - P-004
- Status: confirmed-and-repaired-by-unit-tests; real-sample benefit confirmed, sample still not solved
- Claim:
  - After a failed validation creates a follow-up `implement_solution` recovery node, the action-contract prompt can still see "no active node + blocked validation" and inject `TaskSpaceActionContractClosedValidationV1`. That closes the path to `final_answer` or `blocked` even when a ready recovery node exists.
- Evidence:
  - `sqlite-db-truncate-output-contract-validation-fix`:
    - RunDir: `C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-output-contract-validation-fix\runs\terminal_bench__sqlite-db-truncate\20260702-094101-549\pair-001`
    - `outcome_taskspace=engineering_unclean`;
    - `business_success=false`;
    - `tool_call_count=9`;
    - `changed_paths` included `recover.py` but not `recover.json`;
    - graph health: `nodes=6`, `open_leaf_nodes=1`;
    - node table showed `node-6` as `implement_solution/ready`, while `last-message.md` was a terminal `blocked` action claiming validation was closed.
  - The live `recover.py` was already corrected after the second edit, but the model was told that remaining actions were final/blocked instead of binding the ready recovery node.
- Root cause:
  - Closed-validation detection used only `active_map_has_blocked_validation_result`.
  - It did not check for ready recovery nodes (`implement_solution`, `smoke_test`, or `regression_test`) that the projection layer would otherwise expose as the next valid action.
- Repair:
  - Added `active_map_has_ready_recovery_node`.
  - The action-contract prompt now injects `TaskSpaceActionContractClosedValidationV1` only when blocked validation exists and no ready recovery node exists.
  - The terminal `blocked` rewrite path uses the same guard, so a ready recovery node cannot be masked by the closed-validation shortcut.
- Validation:
  ```text
  cargo test -j1 -p codex-core blocked_validation_with_ready_recovery_node_is_not_closed --lib
  PASS

  cargo test -j1 -p codex-core direct_final_response_rejects_open_contract_without_validation_after_thin_work --lib
  cargo test -j1 -p codex-core access_denied_bash_validation_command_routes_unvalidated_artifact_to_rework --lib
  cargo test -j1 -p codex-core access_denied_local_infra_blocks_validation_without_rework_after_changed_artifact --lib
  cargo test -j1 -p codex-core validation_node_blocks_vacuous_test_after_changed_artifact --lib
  cargo test -j1 -p codex-core validation_rework_rejects_validator_procedure_blocker_before_edit --lib
  cargo test -j1 -p codex-core validation_rework_rejects_missing_current_artifact_visibility_blocker --lib
  cargo test -j1 -p codex-core manual_local_infra_validation_block_routes_unvalidated_changed_artifact_to_rework --lib
  cargo test -j1 -p codex-core action_contract_prompt_structures_validator_procedure_blocker_rejection --lib
  PASS

  cargo fmt --all -- --check
  PASS

  cargo build -j1 --profile dev-small -p codex-cli --bin whale
  PASS
  ```
- Real-sample rerun:
  ```text
  RunDir: C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-ready-recovery-fix\runs\terminal_bench__sqlite-db-truncate\20260702-101238-428\pair-001
  outcome_standard=solved
  outcome_taskspace=engineering_unclean
  business_success=false
  public_validation_skipped=true
  public_validation_skip_reason=agent_exec_timeout
  taskspace_wall_time_ms=900037
  tool_call_count=24
  rollout_trace_model_request_count=28
  changed_paths=recover.py, trunc.db.recovered
  nodes=6
  open_leaf_nodes=1
  ```
- Interpretation:
  - The original closed-validation masking bug is repaired: the rerun continued through the recovery path instead of immediately returning `blocked`.
  - The sample still fails. The new failure is long-flow convergence: `node-6` remains running until the 900s external timeout after `python recover.py` exposed `PermissionError: [WinError 5]` on `trunc.db.recovered`.
  - This is a separate R4/R5 utility issue, not proof that the ready-recovery guard failed.
- Time: 2026-07-02 10:36

## Hypothesis H-018: nested validation/rework chains must exempt ancestor lifecycle results from active tool gates

- Related problems:
  - P-004
- Status: confirmed-and-repaired-by-unit-tests; real-sample timeout benefit confirmed, sample now fails as agent_patch_wrong
- Claim:
  - After more than one validation/rework cycle, TaskSpace's unreviewed-result gate treated older validation blockers and rework lifecycle results as unrelated stale results. Those results were actually ancestors of the current active rework/validation chain, so forcing `state_commit` before the next edit/test created repeated tool-free recovery turns instead of letting the agent continue the standard edit/test loop.
- Evidence:
  - `sqlite-db-truncate-ready-recovery-fix` reached a second rework after `python recover.py` raised `PermissionError: [WinError 5]`, but timed out at 900s with `node-6` still open.
  - Rollout inspection showed repeated model attempts to emit `apply_patch` for `recover.py`, while the runtime injected recovery text requiring review of an older TaskSpace result before ordinary work.
  - The first focused regression failed before the repair:
    ```text
    nested_validation_rework_can_edit_without_reviewing_prior_blocker_result
    FAIL: TaskSpace result `result-4` on node `node-2` is still unreviewed.
    ```
  - A subsequent real rerun exposed the same category one step later: node-7 validation recovery repeatedly complained about `result-15` on node-3, producing `tool_free_action_contract` requests until timeout. Pair report:
    ```text
    RunDir: C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-nested-rework-barrier-fix\runs\terminal_bench__sqlite-db-truncate\20260702-112316-691\pair-001
    outcome_taskspace=agent_exec_timeout
    taskspace_wall_time_ms=900042
    taskspace_tool_call_count=17
    nodes=7
    edges=699
    open_leaf_nodes=1
    provider_request_count reached 50 with request_phase=validation_recovery
    ```
- Root cause:
  - Runtime dependency checks only handled direct validation/rework adjacency:
    - active rework edit exemption handled only direct `origin_node_id == failed_validation_node_id`;
    - validation-after-rework exemption handled only direct dependency edges;
    - completed rework lifecycle `Result` was exempt only for direct validation dependencies.
  - Runtime-created failed-validation rework nodes encode provenance through both `origin_node_id` and graph edges. The gate walked only part of that combined structure, so nested chains were misclassified as stale unreviewed history.
- Repair:
  - Added dependency-chain traversal that follows both incoming edges and `origin_node_id`.
  - Extended active rework input checks so ancestor blocked validation nodes and ancestor completed rework results do not block the current active rework edit or validation test.
  - Kept final-answer readiness strict; the exemption applies only to active edit/test progression inside the same validation/rework chain.
- Validation:
  ```text
  cargo test -j1 -p codex-core blocked_validation_rework_can_edit_without_reviewing_blocker_result --lib
  cargo test -j1 -p codex-core validation_after_rework_can_test_without_reviewing_origin_blocker_result --lib
  cargo test -j1 -p codex-core nested_validation_rework_can_edit_without_reviewing_prior_blocker_result --lib
  cargo test -j1 -p codex-core nested_validation_after_rework_can_test_without_reviewing_prior_chain_results --lib
  cargo test -j1 -p codex-core blocked_validation_with_ready_recovery_node_is_not_closed --lib
  PASS

  cargo fmt --all -- --check
  PASS

  cargo build -j1 --profile dev-small -p codex-cli --bin whale
  PASS
  ```
- Real-sample rerun:
  ```text
  RunDir: C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-nested-validation-chain-fix\runs\terminal_bench__sqlite-db-truncate\20260702-115413-930\pair-001
  outcome_standard=solved
  outcome_taskspace=engineering_unclean
  failure_taxonomy=engineering_unclean, agent_patch_wrong, audit_unclean
  taskspace_exec_timed_out=false
  taskspace_public_validation_exit_code=1
  taskspace_hidden_oracle_exit_code=0
  taskspace_wall_time_ms=566817
  standard_wall_time_ms=209836
  taskspace_wall_time_ratio=2.70
  taskspace_tool_call_count=18
  standard_tool_call_count=18
  taskspace_tool_call_ratio=1.00
  nodes=10
  edges=9
  open_leaf_nodes=0
  ```
- Interpretation:
  - The nested result-review gate no longer creates an agent-exec timeout. The sample now reaches completed public validation on the TaskSpace side.
  - Remaining failure has moved from engineering timeout/tool-loop to answer quality: TaskSpace changed `recover.py` and project scaffolding but did not produce the required `recover.json`, so public validation failed as `agent_patch_wrong`.
  - This becomes the next R4 unresolved utility issue; it is not the same gate-loop defect.
- Time: 2026-07-02 12:12

## Hypothesis H-019: validation rework must allow narrow reads of the failing artifact before editing

- Related problems:
  - P-004
- Status: repaired-by-unit-tests; real-sample rerun pending
- Claim:
  - After H-018 removed the nested result-review loop, `sqlite-db-truncate` exposed a second TaskSpace utility issue: validation rework could see that `recover.py` was failing, but the read/search gate still treated reading the failing file as rediscovery instead of a focused validation-repair action. The agent then attempted blind one-line patches, accumulated indentation errors, and finally emitted a blocker saying it could not read the file.
- Evidence:
  - Real rerun:
    ```text
    RunDir: C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-nested-validation-chain-fix\runs\terminal_bench__sqlite-db-truncate\20260702-115413-930\pair-001
    outcome_taskspace=engineering_unclean
    failure_taxonomy=engineering_unclean, agent_patch_wrong, audit_unclean
    taskspace_exec_timed_out=false
    taskspace_public_validation_exit_code=1
    ```
  - Public validation failed because the expected artifact was not created:
    ```text
    FileNotFoundError: [Errno 2] No such file or directory: '/app/recover.json'
    ```
  - Rollout showed the validation rework chain repeatedly running `python recover.py`, observing successive `IndentationError` failures, and patching individual import lines without first reading the current file.
  - The final TaskSpace app contained `recover.py` but no `recover.json`; `recover.py` was a concatenation of two scripts with malformed top-level indentation.
  - The agent eventually used `taskspace_control(block_node)` with:
    ```text
    Cannot read recover.py to fix indentation errors after failed smoke_test.
    ```
- Root cause:
  - The active validation-rework read/search gate correctly tried to prevent broad rediscovery after validation failure, but it did not distinguish a narrow read of the exact failing traceback artifact from unrelated exploration.
  - The blocker classifier also missed phrases such as `Cannot read ...`, so the runtime accepted a missing-source-visibility blocker even though the correct next action was a focused read plus edit.
- Repair:
  - Extract artifact references from validation failure text, including traceback-style paths such as `S:\app\recover.py` and `/app/recover.py`.
  - Allow active validation-rework read/search actions only when their descriptor targets one of those failed artifacts.
  - Keep unrelated reads/searches blocked until an edit is attempted, preserving the original anti-rediscovery intent.
  - Extend missing-source-visibility blocker detection for `cannot read`, `can't read`, `lack visibility`, and `lacks visibility`.
- Validation:
  ```text
  cargo fmt --all -- --check
  PASS

  cargo test -j1 -p codex-core validation_rework_allows_target_file_read_from_traceback_before_edit --lib
  PASS

  cargo test -j1 -p codex-core validation_rework_rejects_missing_current_artifact_visibility_blocker --lib
  PASS

  cargo test -j1 -p codex-core blocked_validation_rework_requires_edit_before_rediscovery --lib
  PASS

  cargo build -j1 --profile dev-small -p codex-cli --bin whale
  PASS
  ```
- Time: 2026-07-02 12:44

## Hypothesis H-020: validation success-exit semantic failures must route to rework instead of recovery-looping

- Related problems:
  - P-004
- Status: repaired-by-unit-tests; real-sample rerun pending
- Claim:
  - A validation command can exit 0 while still proving that the implementation is semantically wrong. TaskSpace previously rejected such output for validation closeout, but did not route it to a rework node. The active validation node stayed running, then repeated `validation_recovery` requests with no tools until the external 900s timeout.
- Evidence:
  - Real rerun:
    ```text
    RunDir: C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-target-read-rework-fix\runs\terminal_bench__sqlite-db-truncate\20260702-130118-459\pair-001
    outcome_taskspace=agent_exec_timeout
    taskspace_exec_timed_out=true
    right_validation_lifecycle_stage=unknown
    right_tests_started_seen=False
    taskspace_tool_call_count=10
    nodes=7
    edges=686
    open_leaf_nodes=1
    ```
  - `node-7` recorded a successful local command result, but the output showed semantic failure:
    ```text
    command: python recover.py
    Exit code: 0
    Backup failed: file is not a database
    Recovered 0 rows
    OK: 0 rows
    ```
  - After that result, provider requests continued in `request_phase:validation_recovery` with `tools_present:false` and repeated last-message preview:
    ```text
    A successful implementation edit is already recorded.
    ```
  - The final app had `recover.py` and `recover.json`, but `recover.json` was `[]`, and external validation was skipped due to `agent_exec_timeout`.
- Root cause:
  - `force_finish_validation_after_successful_tool` correctly refused to close the validation node because the output was not a confirmed validator success.
  - The failed-validation rework path only covered `tool_success=false` results and local infrastructure failures. It did not cover `tool_success=true` results whose output contained failure semantics.
  - As a result, the validation node was neither completed nor blocked, so the turn loop kept asking the model for validation recovery instead of moving to implementation rework.
- Repair:
  - Added `validation_node_semantic_failure_result(...)` for successful test/build tool calls whose output includes failure markers and is not a successful validation.
  - Added `recovered 0 rows` to semantic failure markers for recovery/data-processing tasks.
  - Routed semantic-failure validation results through the same block-and-rework path as normal failed validation.
  - Kept genuine successful validation closeout unchanged.
- Validation:
  ```text
  cargo test -j1 -p codex-core semantic_failure_success_exit_auto_blocks_validation_and_routes_rework --lib
  PASS

  cargo test -j1 -p codex-core semantic_failure_output_blocks_validation_instead_of_closeout --lib
  PASS

  cargo test -j1 -p codex-core force_finish_validation_after_successful_tool_closes_smoke_node --lib
  PASS

  cargo test -j1 -p codex-core local_infra_tool_result_auto_blocks_validation_node --lib
  PASS

  cargo test -j1 -p codex-core validation_rework_allows_target_file_read_from_traceback_before_edit --lib
  PASS

  cargo fmt --all -- --check
  PASS

  cargo build -j1 --profile dev-small -p codex-cli --bin whale
  PASS
  ```
- Time: 2026-07-02 14:06

## Hypothesis H-021: action-contract apply_patch feedback must normalize native/unified grammar failures

- Related problems:
  - P-004
- Status: repaired-by-unit-tests; real-sample exposed next root cause H-022
- Claim:
  - After H-020 routed semantic validation failures into implementation rework, `sqlite-db-truncate` still wasted requests on repeated `apply_patch` failures. The raw tool feedback was visible, but the action-contract path did not structure two failure shapes from the real run and did not normalize native patch payloads that included unified-diff hunk ranges.
- Evidence:
  - Real rerun before this repair:
    ```text
    RunDir: C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-semantic-rework-fix\runs\terminal_bench__sqlite-db-truncate\20260702-134303-636\pair-001
    outcome_taskspace=engineering_unclean
    taskspace_exec_timed_out=true
    nodes=6
    edges=5
    open_leaf_nodes=1
    ```
  - `node-4` consumed multiple attempts on patch grammar/path mistakes before one edit succeeded:
    ```text
    apply_patch verification failed: Failed to find context '-1,1 +1,1 @@' in S:\app\recover.py
    apply_patch verification failed: Failed to read file to update S:\app\src\new ... (os error 3)
    apply_patch verification failed: invalid hunk at line 3, '@@ -0,0 +1,44 @@' is not a valid hunk header
    ```
  - Real rerun after this repair:
    ```text
    RunDir: C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-patch-feedback-fix\runs\terminal_bench__sqlite-db-truncate\20260702-143858-261\pair-001
    taskspace_exec_timed_out=true
    nodes=8
    edges=466
    provider requests=31
    main_tool_result events=21
    exact_payload_scan_passed=true
    replacement_confirmed=true
    legacy_taskspace_history_present=false
    ```
  - The post-repair app shows the immediate patch-feedback issue moved forward: `right\app\recover.py` exists and includes `import sqlite3`, addressing the prior `Python sqlite3: name 'sqlite3' is not defined` validation failure. No post-repair evidence showed the same context-mismatch or invalid unified hunk apply_patch failure as the active blocker.
  - The post-repair failure shifted to a separate state-machine problem: repeated implement/validation nodes, `node-7` with 13 results, `node-9` still running, and 466 duplicate edges before the 900s sample timeout.
- Root cause:
  - `taskspace_action_contract_tool_feedback_summary(...)` classified missing update targets and expected-line mismatches, but did not classify `Failed to find context ... in <path>` or invalid unified hunk headers inside native apply_patch payloads.
  - `normalize_taskspace_apply_patch(...)` normalized full unified diffs, but native `*** Update File` / `*** Add File` payloads containing `@@ -x +y @@` headers passed through unchanged. For `Add File`, converting that line to `@@` would still be invalid because add-file sections accept only added lines.
- Repair:
  - Added context-mismatch parsing that extracts the target path from real `Failed to find context ... in S:\app\recover.py` messages.
  - Added structured model-visible feedback for:
    - `apply_patch_context_mismatch`
    - `apply_patch_unified_hunk_header_in_native_patch`
  - Normalized native update hunks by stripping unified range metadata while preserving trailing context, for example `@@ -5,7 +5,7 @@ def f` becomes `@@ def f`.
  - Dropped unified hunk headers from `*** Add File` sections instead of converting them to another invalid hunk.
  - Kept existing full unified-diff conversion and missing-target behavior intact.
- Validation:
  ```text
  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core taskspace_apply_patch_context_mismatch_target_is_detected --lib
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core taskspace_action_contract_apply_patch_normalizes_native_unified_update_hunk_headers --lib
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core taskspace_action_contract_apply_patch_drops_unified_hunk_header_from_add_file --lib
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core action_contract_prompt_structures_apply_patch_context_mismatch_feedback --lib
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core action_contract_prompt_structures_apply_patch_unified_hunk_header_feedback --lib
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core taskspace_action_contract_apply_patch_normalizes --lib
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core taskspace_apply_patch_ --lib
  PASS, 13 tests

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core edit_failure_recovery_preserves_failed_tool_feedback --lib
  PASS

  cargo fmt --all -- --check
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo build -j1 --profile dev-small -p codex-cli --bin whale
  PASS
  ```
- Time: 2026-07-02 15:13

## Hypothesis H-022: large-rollout summary edge count duplicated every snapshot edge

- Related problems:
  - P-004
- Status: repaired-by-test-and-rerender; runtime convergence follow-up remains open
- Claim:
  - The post H-021 `edges=466` signal was an observability export bug, not proof that the runtime graph had created hundreds of logical edges. The large-rollout summary path appended every edge from every `snapshot_updated` event, while the full export path already deduplicated edges by `(mapId, from, to)`.
- Evidence:
  - Same real rollout after H-021:
    ```text
    Source: C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-patch-feedback-fix\runs\terminal_bench__sqlite-db-truncate\20260702-143858-261\pair-001\right\artifacts\rollout.jsonl
    Original summary edges=466
    Rerendered summary edges=7
    nodes=8
    runtime_events=1266
    ```
  - The rerender used the same rollout bytes and only changed the summary exporter. Therefore the original edge count was a reporting artifact.
- Root cause:
  - `New-ActionMapLargeRolloutSummary` kept a flat `$edges` list and appended snapshot edges on every `snapshot_updated` event.
  - Unlike the non-summary export path, it had no `$edgeKeys` set to preserve one logical edge per `(mapId, from, to)`.
- Repair:
  - Added summary-export edge de-duplication with `$edgeKeys`.
  - Extended the summary-export self-test so 135 repeated snapshots of the same `node-1 -> node-2` edge must produce exactly one summary edge and one exported edge row.
- Validation:
  ```text
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\test-action-map-observability-summary-export.ps1
  PASS

  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\export-action-map-observability.ps1 -RolloutPath C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-patch-feedback-fix\runs\terminal_bench__sqlite-db-truncate\20260702-143858-261\pair-001\right\artifacts\rollout.jsonl -JsonlPath C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-patch-feedback-fix\runs\terminal_bench__sqlite-db-truncate\20260702-143858-261\pair-001\right\artifacts\whale-exec.jsonl -OutputDir C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-patch-feedback-fix\rerender-observability-h022 -ArtifactRoot C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-patch-feedback-fix\runs\terminal_bench__sqlite-db-truncate\20260702-143858-261\pair-001\right\app
  PASS, Edges=7
  ```
- Remaining runtime finding:
  - The edge-count fix only removes a false signal. The same real run still timed out after repeated implement/validation rework. `node-7` accumulated 13 results and `node-9` was still running at the 900s timeout, so the next root cause is TaskSpace state-machine convergence, not observability accounting.
- Time: 2026-07-02 15:42

## Hypothesis H-023: validation rework must deduplicate same-artifact reads and route local-infra changed-artifact checks back to validation

- Related problems:
  - P-004
- Status: repaired-by-unit-tests; real-sample exposed next root cause H-024
- Claim:
  - After H-021, `sqlite-db-truncate` still had two state-machine gaps:
    - validation rework could read the same failed artifact repeatedly before any edit;
    - local validator infrastructure failures involving an already changed artifact were routed to an `implement_solution` node, even when the next useful action was to run the changed artifact with platform-compatible syntax.
- Evidence:
  - Pre-repair real run:
    ```text
    RunDir: C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-patch-feedback-fix\runs\terminal_bench__sqlite-db-truncate\20260702-143858-261\pair-001
    taskspace_exec_timed_out=true
    node_count=8
    logical_edges_after_rerender=7
    node-7 result_count=13
    node-9 running at timeout
    ```
  - First post duplicate-read repair run:
    ```text
    RunDir: C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-duplicate-rework-read-fix\runs\terminal_bench__sqlite-db-truncate\20260702-152414-237\pair-001
    taskspace_exec_timed_out=false
    taskspace_wall_time_ms=124948
    standard_wall_time_ms=159798
    taskspace_tool_call_count=5
    standard_tool_call_count=10
    node_count=4
    edge_count=3
    public_validation_exit_code=1
    ```
  - This proves the repeated-rework timeout was removed, but the run still failed because `recover.py` was created and never executed, leaving `/app/recover.json` missing.
  - Second post local-infra routing run:
    ```text
    RunDir: C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-local-infra-validation-retry\runs\terminal_bench__sqlite-db-truncate\20260702-160146-549\pair-001
    taskspace_exec_timed_out=false
    outcome_taskspace=wrong
    node_count=3
    edge_count=2
    taskspace_tool_call_count=4
    public_validation_exit_code=1
    ```
- Root cause:
  - The `implement_node_validation_rework_read_targets_failure_artifact(...)` exception allowed the first focused read of a traceback artifact, but there was no second-order guard for rereading that same artifact before an edit.
  - `block_main_node(...)` treated every validation rework as implementation rework. For local infrastructure failures on changed artifacts, that created a policy contradiction: the context asked for platform-compatible execution, while the active node kind still pressured the agent toward another edit.
  - The lifecycle review exemption for origin validation blockers only covered implement-rework chains, not validation retry nodes whose `origin_node_id` points to the blocked validation node.
- Repair:
  - Added `implement_node_duplicate_validation_rework_artifact_read(...)`.
  - Added `validation_rework_duplicate_artifact_read` gate with prior result id and next valid action `apply_patch`.
  - Split validation rework routing:
    - failed/semantic validation still routes to `implement_solution`;
    - local-infra unvalidated changed artifact routes to a fresh validation node of the same kind, preserving dependency edges to the implementation node.
  - Extended active rework input-blocker exemption so validation retry nodes can proceed without reviewing the origin infra blocker first.
- Validation:
  ```text
  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core validation_rework_allows_target_file_read_from_traceback_before_edit --lib
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core validation_rework_ --lib
  PASS, 6 tests

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core access_denied_ --lib
  PASS, 2 tests

  cargo fmt --all -- --check
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo build -j1 --profile dev-small -p codex-cli --bin whale
  PASS
  ```
- Remaining runtime finding:
  - H-023 improved convergence but did not solve `sqlite-db-truncate`. The next real blocker is H-024: inspect-node `run_test` command normalization allowed Bash-style `||` to reach PowerShell unchanged.
- Time: 2026-07-02 16:23

## Hypothesis H-024: action-contract run_test must normalize Bash logical operators before PowerShell execution

- Related problems:
  - P-004
- Status: repaired-by-unit-tests; real pair rerun invalid due provider/model first-event timeout
- Claim:
  - TaskSpace action-contract `run_test` can pass Bash-style `||` through to PowerShell, causing `InvalidEndOfLine` before useful diagnostic output is produced. The model then over-attributes the failure to validator infrastructure and gets stuck in state_commit/block recovery instead of running the changed artifact or fixing the command.
- Evidence:
  - Real rerun:
    ```text
    RunDir: C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-local-infra-validation-retry\runs\terminal_bench__sqlite-db-truncate\20260702-160146-549\pair-001
    outcome_taskspace=wrong
    exec_timed_out=false
    taskspace_tool_call_count=4
    public_validation_exit_code=1
    ```
  - The inspect diagnostic command was:
    ```text
    sqlite3 trunc.db ".tables" 2>&1 || echo 'sqlite3 not available, trying python'; python -c ...
    ```
  - PowerShell rejected it before the Python fallback:
    ```text
    The token '||' is not a valid statement separator in this version.
    FullyQualifiedErrorId : InvalidEndOfLine
    ```
  - After that, the run created `recover.py` but never created `recover.json`, and the final external validator failed with:
    ```text
    FileNotFoundError: [Errno 2] No such file or directory: '/app/recover.json'
    ```
- Root cause hypothesis:
  - Existing action-contract command normalization handles some `run_test` shell cases, but does not convert or reject Bash logical OR (`||`) for the PowerShell execution environment.
  - This is a tool invocation normalization problem, not a budget problem and not the same as H-023 local-infra validation retry.
- Next repair target:
  - Normalize `cmd1 || cmd2` into a PowerShell-compatible equivalent or reject it with structured feedback that requires a platform-compatible command.
  - Add tests for `run_test` command translation with `||`, and rerun `sqlite-db-truncate`.
- Time: 2026-07-02 16:23

- Repair:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs` now normalizes top-level Bash OR chains before Windows PowerShell execution:
    ```text
    cmd1 || cmd2; tail
    =>
    cmd1; if ($LASTEXITCODE -ne 0) { cmd2 }; tail
    ```
  - The splitter ignores `||` and `;` inside single or double quotes, so command strings such as Python snippets are not corrupted.
- Validation:
  ```text
  cargo test -j1 -p codex-core run_test_normalizes --lib
  PASS: 2 tests

  cargo test -j1 -p codex-core taskspace_powershell_ --lib
  PASS: 2 tests

  cargo fmt --all -- --check
  PASS

  cargo build -j1 --profile dev-small -p codex-cli --bin whale
  PASS
  ```
- Invalid real rerun:
  ```text
  RunDir: C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-powershell-or-chain-fix\runs\terminal_bench__sqlite-db-truncate\20260702-163512-422\pair-001
  left whale-exec.jsonl len=0
  right whale-exec.jsonl len=0
  left timeout=900s before first JSON event
  right timeout=900s before first JSON event
  ```
  - This rerun did not reach the tool execution layer, so it cannot prove or disprove H-024's runtime benefit.
  - It is evidence of an external/provider/model first-event timeout in the harness run, not evidence that the `||` normalization failed.

## Hypothesis H-025: public-10 cost reporting must expose effective model request amplification

- Related problems:
  - P-004
- Status: repaired-by-report-gate
- Claim:
  - Public-10 reporting previously exposed token ratios and cache hit rates, but did not expose the effective TaskSpace model request count from rollout/provider traces. This hid the causal link between high cache hit and still-high cost: TaskSpace repeatedly asks the model more times than standard.
- Evidence:
  - `heterogeneous-dates` solved in both modes, but TaskSpace remained much more expensive:
    ```text
    standard request-summary model_request_count=1
    taskspace request-summary top-level model_request_count=1
    taskspace rollout_trace.model_request_count=12
    taskspace provider_cache_trace.provider_request_count=11
    taskspace_token_ratio=11.082
    request_2_plus_cache_hit_rate=0.98556
    ```
  - After report repair, public-10 rows expose effective request ratios:
    ```text
    vim-terminal-task: 6x
    heterogeneous-dates: 12x
    sqlite-db-truncate: 9x
    git-workflow-hack: 21x
    sqlite-with-gcov: 18x
    csv-to-parquet: 8x
    tmux-advanced-workflow: 28x
    ```
- Root cause:
  - `write-r4-public-10-tool-stress-report.ps1` used token/cost fields but had no first-class model request count fields. The available trace hierarchy had more precise data in `request-summary.rollout_trace.model_request_count` and `provider-cache-trace-summary.provider_request_count`, but the report schema and gate did not require those fields.
- Repair:
  - Added effective model request count extraction with source priority:
    ```text
    rollout_trace.model_request_count
    provider-cache-trace-summary.provider_request_count
    request-summary.model_request_count
    metrics.model_request_count
    ```
  - Added report fields:
    ```text
    standard_model_request_count
    taskspace_model_request_count
    taskspace_model_request_ratio
    standard_model_request_count_source
    taskspace_model_request_count_source
    model_request_count_availability
    ```
  - Extended public-10 gate and usage-accounting negative test so `model_request_count_availability=measured` cannot pass without `taskspace_model_request_ratio`.
- Validation:
  ```text
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\write-r4-public-10-tool-stress-report.ps1 -RequireComplete
  PASS: complete_run_count=10 missing_run_count=0

  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-r4-public-10-tool-stress-plan.ps1 -ReportPath target\r4-public-10-tool-stress\r4-public-10-tool-stress-report.json
  PASS: R4 public-10 tool-stress gate passed: 10 planned samples

  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-r4-public-10-usage-accounting-gate.ps1
  PASS: R4 public-10 usage accounting gate rejects ambiguous token usage
  ```
- Conclusion:
  - The reporting gap is fixed. The underlying TaskSpace utility issue remains open: high cache hit does not compensate for long-flow convergence and request-count amplification.
- Time: 2026-07-02 18:10

## Hypothesis H-026: inspect nodes keep accepting low-value reads after successful diagnostic evidence

- Related problems:
  - P-004
- Status: repaired-by-focused-tests; real rerun pending build availability
- Claim:
  - In action-contract TaskSpace sessions, `inspect_code_context` only auto-converged after duplicate diagnostics, no-action recovery, progress pressure, or request budget pressure. It did not reuse the existing successful-required-action auto-finish path for the common case where inspect already had both a successful diagnostic and concrete working evidence, so the model could keep reading already-known or wrong-path files before implementation.
- Evidence:
  - Real `heterogeneous-dates` public-10 row:
    ```text
    outcome_standard=solved
    outcome_taskspace=solved
    standard_model_request_count=1
    taskspace_model_request_count=12
    taskspace_token_ratio=11.082
    request_2_plus_cache_hit_rate=0.98556
    ```
  - The TaskSpace inspect node had six main tool results before convergence:
    ```text
    result-3 read task-deps/daily_temp_sf_high.csv success
    result-4 run_test computed 11.428571428571429 success
    result-5 re-read task-deps/daily_temp_sf_high.csv success
    result-6 read daily_temp_sf_low.csv at wrong root failed
    result-7 forced inspect transition only after inspect_no_action_with_evidence
    ```
  - Code inspection confirmed `should_finish_node_after_successful_required_action(...)` only handled `implement_solution` and validation nodes, not `inspect_code_context`.
- Root cause:
  - Inspect convergence had a forced-transition path, but not a semantic auto-finish path for "successful diagnostic + working evidence + next action is more discovery." This is a state-machine convergence gap, not a cache-hit failure and not a fixed request-budget issue.
- Repair:
  - Added `current_main_inspect_has_successful_diagnostic_and_working_evidence()` in the action map runtime.
  - Added a session wrapper for that query.
  - Extended action-contract auto-finish so an inspect node with successful diagnostic plus working evidence converts subsequent `list_files`/`search`/`read_file` into `finish_node`.
  - Added a dedicated inspect finish action that creates an `implement_solution` next node with the inspect node as dependency.
- Validation:
  ```text
  cargo fmt --all -- --check
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core inspect_successful_diagnostic_and_working_evidence_marks_convergence_ready --lib
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core taskspace_finish_inspect_to_implementation_action_builds_next_node --lib
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core taskspace_action_contract_finish_node --lib
  PASS
  ```
- Build/rerun gap:
  ```text
  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo build -j1 --profile dev-small -p codex-core
  TIMEOUT after 604s
  ```
  - The timed-out build process was stopped to avoid RAM pressure.
  - A real `heterogeneous-dates` rerun still requires a fresh Whale binary; do not claim runtime benefit until that rerun is complete.
- Time: 2026-07-02 19:05

## Hypothesis H-027: forced validation closeout did not accept implementation evidence for final readiness

- Related problems:
  - P-004
- Status: repaired-by-focused-tests-and-real-rerun
- Claim:
  - After inspect convergence repair, `heterogeneous-dates` reached a clean validation state but TaskSpace still rejected `final_answer`. The forced validation closeout path finished the validation node without accepting the directly dependent implementation edit/lifecycle evidence and without accepting the forced validation lifecycle result, so the final readiness gate saw open/unreviewed evidence and kept the session alive until timeout.
- Evidence:
  - Real rerun before this repair:
    ```text
    RunDir:
    C:\WhaleRunCache\r4-inspect-convergence-heterogeneous-20260702-minfree15\runs\terminal_bench__heterogeneous-dates\20260702-180700-127\pair-001

    outcome_standard=solved
    outcome_taskspace=agent_exec_timeout
    failure_taxonomy=engineering_unclean, taskspace_overhead_timeout, audit_unclean
    standard_wall_ms=86114
    taskspace_wall_ms=900039
    public_validation_exit_code_standard=0
    public_validation_exit_code_taskspace=0
    taskspace_changed_paths=avg_temp.txt, solve.py
    ```
  - Tail logs repeatedly reported:
    ```text
    TaskSpace final_answer rejected by final readiness gate. Continue the same task and clear the gate before final_answer.
    ```
  - Active context showed `current_node: none` while `output-contract-1 status=open`, which means the graph had no useful next work node but the readiness gate still blocked final synthesis.
  - A focused regression initially failed with:
    ```text
    TaskSpace result result-2 on node node-1 is still unreviewed
    ```
- Root cause:
  - `force_finish_validation_after_successful_tool(...)` closed the validation node semantically, but did not propagate acceptance to successful edit/lifecycle results on directly dependent `implement_solution` nodes. It also created a validation closeout lifecycle result that remained unaccepted. The final gate correctly rejected unreviewed evidence, but the runtime gave the agent no meaningful graph state to repair.
- Repair:
  - Added dependency evidence acceptance before forced validation closeout:
    ```text
    successful_dependency_edit_result_ids(...)
    dependency_implementation_lifecycle_result_ids(...)
    dependency_implementation_result_ids(...)
    accept_implementation_evidence_for_validation_closeout(...)
    ```
  - Accepted the forced validation closeout lifecycle result after node finish through `accept_forced_transition_result(...)`.
- Validation:
  ```text
  cargo fmt --all -- --check
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core forced_validation_closeout_accepts_dependency_edit_for_final_readiness --lib
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core force_finish_validation_after_successful_tool_closes_smoke_node --lib
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core finish_final_synthesis_accepts_open_behavior_after_accepted_fix_and_validation --lib
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo build -j1 --profile dev-small -p codex-cli --bin whale
  PASS
  ```
  - Real rerun after repair:
    ```text
    RunDir:
    C:\WhaleRunCache\r4-final-readiness-heterogeneous-20260702\runs\terminal_bench__heterogeneous-dates\20260702-185101-849\pair-001

    outcome_standard=solved
    outcome_taskspace=engineering_unclean
    failure_taxonomy=engineering_unclean, audit_unclean
    standard_wall_ms=42043
    taskspace_wall_ms=229459
    standard_exec_timed_out=false
    taskspace_exec_timed_out=false
    public_validation_exit_code_standard=0
    public_validation_exit_code_taskspace=0
    taskspace_model_request_count=16
    request_2_plus_cache_hit_rate=0.988319
    active_context_replacement_confirmed=true
    legacy_taskspace_history_present=false
    taskspace_control_count=7
    ```
- Conclusion:
  - The final-readiness timeout failure is fixed for this sample: TaskSpace moved from 900s timeout to non-timeout completion with public validation passing. The run still ended `engineering_unclean` because a separate validation path-error classification issue polluted blocked-node accounting.
- Time: 2026-07-02 19:35

## Hypothesis H-028: known input path errors in validation were misclassified as implementation failure

- Related problems:
  - P-004
- Status: repaired-by-focused-tests; real rerun inconclusive due model patch error
- Claim:
  - Validation failures caused by a validator command referencing a known input artifact basename from the wrong working directory should remain validation/invocation errors, not be treated as implementation failure. Otherwise TaskSpace creates unnecessary rework/blockage after the artifact already exists at its known path.
- Evidence:
  - In the H-027 post-repair rerun, both standard and TaskSpace public validation passed, but TaskSpace still reported `engineering_unclean`.
  - The blocked graph state was tied to validation code using `daily_temp_sf_high.csv` from the task root while known evidence had the artifact at:
    ```text
    task-deps/daily_temp_sf_high.csv
    ```
  - The validation failure was therefore about the validation command's path assumption, not about a missing implementation artifact.
- Root cause:
  - `validation_node_failed_noninfra_result(...)` treated file-not-found validation failures as non-infra validation blockers without checking whether the missing basename was already represented by a known artifact path in the map.
- Repair:
  - Added `validation_failure_is_known_input_path_error(map, result)`.
  - Excluded validation failures from non-infra blocking when the stderr references a missing basename that is already known under a task artifact path.
  - Preserved the existing rework behavior for truly unknown missing files such as `data.csv`.
- Validation:
  ```text
  cargo fmt --all -- --check
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core validation_known_input_path_error_stays_on_validation_node --lib
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core validation_node_failed_test_blocks_repeated_validation --lib
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo build -j1 --profile dev-small -p codex-cli --bin whale
  PASS
  ```
  - Real rerun after repair:
    ```text
    RunDir:
    C:\WhaleRunCache\r4-validation-path-heterogeneous-20260702\runs\terminal_bench__heterogeneous-dates\20260702-190925-951\pair-001

    outcome_standard=wrong
    outcome_taskspace=engineering_unclean
    failure_taxonomy=engineering_unclean, agent_patch_wrong, audit_unclean
    standard_wall_ms=56673
    taskspace_wall_ms=404609
    public_validation_exit_code_standard=1
    public_validation_exit_code_taskspace=1
    taskspace_changed_paths=.python-version, calculate.py, main.py, pyproject.toml, README.md, uv.lock
    ```
  - This rerun cannot prove utility benefit because standard also failed and TaskSpace generated invalid Python (`SyntaxError` / `IndentationError`). It is evidence that the next blocker is model/tool-use quality on that stochastic run, not proof that the path classification repair failed.
- Conclusion:
  - The classification path is fixed by focused regression and the unknown-file negative regression remains protected. A clean real-sample benefit proof still requires another non-polluted rerun or a smaller deterministic tool-stress sample.
- Time: 2026-07-02 19:50

## Hypothesis H-029: validation closeout did not adopt open user criteria and output contract

- Related problems:
  - P-004
- Status: repaired-by-focused-tests-and-real-rerun
- Claim:
  - After H-027/H-028, `heterogeneous-dates` could write the correct artifact and pass validation, but final answer still timed out when the initial problem ledger contained open user criteria without evidence refs. Forced validation closeout accepted implementation and validation results, but did not update the original open criteria/output contract to `satisfied`, so final readiness still rejected a valid `final_answer`.
- Evidence:
  - Real rerun before repair:
    ```text
    RunDir:
    C:\WhaleRunCache\r4-h028-rerun-heterogeneous-20260702\runs\terminal_bench__heterogeneous-dates\20260702-192443-140\pair-001

    outcome_standard=solved
    outcome_taskspace=agent_exec_timeout
    failure_taxonomy=engineering_unclean, taskspace_overhead_timeout, audit_unclean
    taskspace_wall_ms=900034
    public_validation_exit_code_taskspace=0
    hidden_oracle_exit_code_taskspace=0
    taskspace_changed_paths=avg_temp.txt
    ```
  - The model produced a valid `final_answer`, but the snapshot still had:
    ```text
    criterion-1 status=open evidenceRefs=[]
    criterion-2 status=open evidenceRefs=[]
    criterion-3 status=open evidenceRefs=[]
    output-contract-1 status=open
    sc-node-3-validation-pass status=satisfied result-7
    provider_request_count reached 57
    ```
  - This proved the remaining issue was not tool execution, not validation correctness, and not JSON parsing. It was ledger adoption: the validated edit/test evidence was accepted but not joined back to the user-facing acceptance records.
- Root cause:
  - `force_finish_validation_after_successful_tool(...)` accepted implementation edit/lifecycle evidence and the validation closeout result, but only auto-created the node-local validation criterion. It did not satisfy pre-existing open criteria whose kinds were directly provable by accepted implementation plus accepted validation.
- Repair:
  - Added validation-closeout ledger adoption:
    ```text
    satisfy_closeout_success_criteria(...)
    latest_accepted_successful_validation_result_id(...)
    criterion_kind_can_be_satisfied_by_validated_artifact(...)
    closeout_success_criterion_evidence_refs(...)
    ```
  - The adoption path updates open criteria of these kinds only:
    ```text
    test, validator, artifact, behavior, user_visible_output
    ```
  - It cites both the accepted implementation result and the accepted validation result. It intentionally does not auto-satisfy unrelated `performance` / `compatibility` criteria.
- Validation:
  ```text
  cargo fmt --all -- --check
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core forced_validation_closeout_satisfies_open_user_criteria_for_final_answer --lib
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core forced_validation_closeout_accepts_dependency_edit_for_final_readiness --lib
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core force_finish_validation_after_successful_tool_closes_smoke_node --lib
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core validation_known_input_path_error_stays_on_validation_node --lib
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core validation_node_failed_test_blocks_repeated_validation --lib
  PASS

  CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo build -j1 --profile dev-small -p codex-cli --bin whale
  PASS
  ```
  - Real rerun after repair:
    ```text
    RunDir:
    C:\WhaleRunCache\r4-ledger-adoption-heterogeneous-20260702\runs\terminal_bench__heterogeneous-dates\20260702-195745-535\pair-001

    outcome_standard=solved
    outcome_taskspace=solved
    failure_taxonomy=engineering_unclean, audit_unclean
    standard_wall_ms=57668
    taskspace_wall_ms=105841
    standard_exec_timed_out=false
    taskspace_exec_timed_out=false
    public_validation_exit_code_standard=0
    public_validation_exit_code_taskspace=0
    hidden_oracle_exit_code_standard=0
    hidden_oracle_exit_code_taskspace=0
    standard_tool_call_count=13
    taskspace_tool_call_count=6
    taskspace_tool_call_ratio=0.46
    taskspace_wall_time_ratio=1.84
    ```
  - Action-map observability after repair:
    ```text
    accepted results=5
    final artifacts=1
    cognitive hard gate=True
    finalArtifactMissingWhyChainCount=0
    nonAcceptedFinalArtifactDependencyCount=0
    criterion-1 updated after validation closeout
    criterion-2 updated after validation closeout
    output-contract-1 updated after validation closeout
    ```
  - Request/cache evidence:
    ```text
    rollout_trace.model_request_count=9
    provider_request_count=8
    request_2_plus_hit_rate=0.981959
    ```
- Conclusion:
  - The H-029 repair has real engineering benefit: the same public sample moved from 900s timeout with public validation already passing to non-timeout solved with both public and hidden oracle passing. Remaining `engineering_unclean` in the pair report is caused by E3/audit eligibility requirements, not by TaskSpace execution failure.
- Time: 2026-07-02 20:10

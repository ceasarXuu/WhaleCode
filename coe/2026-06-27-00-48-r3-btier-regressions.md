# Problem P-001: R3 B-tier smoke exposes missing timing attribution and open TaskSpace leaf
- Status: open
- Created: 2026-06-27 00:48
- Updated: 2026-06-27 01:06
- Objective: Make the R3 B-tier smoke produce trustworthy benefit evidence by fixing proven harness attribution gaps and then closing the remaining TaskSpace graph lifecycle blocker.
- Symptoms:
  - B-tier smoke finished with business success on both standard and TaskSpace, but speed evidence was blocked because `model_request_duration_ms` was missing.
  - The same run left `open_leaf_nodes=1` on the TaskSpace side.
- Expected behavior:
  - Benchmark metrics should read provider lifecycle timing from the artifact that actually contains provider events.
  - A successful TaskSpace run should close all graph leaves or provide a precise lifecycle rejection reason.
- Actual behavior:
  - Pair timing reported `wait_attribution_status=missing` with missing field `model_request_duration_ms`.
  - Graph health reported one open leaf after the TaskSpace side completed the user task.
- Impact:
  - R3 cannot claim real speed/cost benefit from B-tier evidence while timing attribution is incomplete.
  - R3 graph lifecycle benefits remain unproven while successful runs can still leave a running leaf.
- Reproduction:
  - Run B-tier smoke at `target\phase-r3-btier-smoke-20260627-003813\single-file-fast-fix\20260627-003814-503`.
- Environment:
  - Windows PowerShell, branch `whalecode-alpha`, local debug `whale.exe` from `D:\BuildCache\whalecode\cargo-target\debug\whale.exe`.
- Known facts:
  - See E-001 through E-005.
- Ruled out:
  - none
- Fix criteria:
  - Timing attribution self-test passes and B-tier metrics can read provider lifecycle timing from `rollout.jsonl`.
  - The graph closeout blocker has a confirmed root cause, focused regression coverage, and a rerun that no longer leaves a successful open leaf.
- Current conclusion: Timing attribution has a confirmed runner source-selection root cause. Graph closeout has a confirmed action-contract rewrite root cause: explicit validation `finish_node` was replaced by `final_answer` before the lifecycle tool could run.
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: Benchmark runner reads the wrong JSONL for provider timing
- Status: confirmed
- Parent: P-001
- Claim: `run-taskspace-benchmark.ps1` passed `whale-exec.jsonl` to `Get-TaskspaceModelTimingAttribution`, but provider lifecycle timing is emitted in artifact `rollout.jsonl`, so metrics incorrectly reported missing model request duration.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - Provider lifecycle timing was added to rollout event messages, while `whale-exec.jsonl` is the process wrapper transcript and does not contain those events.
- Falsifiable predictions:
  - If true: the same B-tier run parses non-null provider lifecycle timing from `rollout.jsonl` and null timing from `whale-exec.jsonl`.
  - If false: either both files contain timing, or neither can provide provider lifecycle duration.
- Diagnostic evidence plan:
  - Prediction or clause under test: compare timing parser output for the same side's `rollout.jsonl` and `whale-exec.jsonl`.
  - Signal: `Get-TaskspaceModelTimingAttribution` result.
  - Capture method: run the timing parser on both files from the same B-tier artifact directory.
  - Event name or marker:
    - `provider_request_budget`
  - Correlation keys:
    - `target\phase-r3-btier-smoke-20260627-003813\single-file-fast-fix\20260627-003814-503\pair-001\right`
  - Differentiates from:
    - provider runtime did not emit lifecycle duration
  - Supports if:
    - `rollout.jsonl` returns `provider_lifecycle_timing` with non-null duration while `whale-exec.jsonl` returns `jsonl_without_timing`.
  - Refutes if:
    - `whale-exec.jsonl` also contains provider lifecycle timing, or `rollout.jsonl` cannot parse provider timing.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: use a tested helper that prefers artifact `rollout.jsonl` and falls back to exec JSONL only when rollout is unavailable.
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: Explicit `finish_node` for the final smoke-test node is not committed
- Status: confirmed
- Parent: P-001
- Claim: The final TaskSpace node remains running because the explicit `finish_node` action in the model transcript is not converted into a durable graph transition.
- Layer: root-cause
- Factor relation: unknown
- Depends on:
  - none
- Rationale:
  - The B-tier transcript contains a `finish_node` action for `node-3`, but graph health still reports one open leaf.
- Falsifiable predictions:
  - If true: the transcript contains a final `finish_node` request for `node-3`, while rollout graph snapshots or tool results show no corresponding committed node completion.
  - If false: no valid final `finish_node` request exists, or it was rejected with an explicit action-contract error.
- Diagnostic evidence plan:
  - Prediction or clause under test: compare model action transcript, tool/action-contract result, and final graph snapshot for `node-3`.
  - Signal: `whale-exec.jsonl`, `rollout.jsonl`, and final graph snapshot entries.
  - Capture method: targeted extraction around `finish_node`, action-contract errors, and final graph state.
  - Event name or marker:
    - `taskspace_control`
    - `finish_node`
    - `taskspace_trace_event_recorded`
  - Correlation keys:
    - `node-3`
    - `result-16`
  - Differentiates from:
    - model failed to issue finish action
    - finish action was valid but ignored
    - finish action was invalid and rejected
  - Supports if:
    - a valid `finish_node` action exists without a durable completion event.
  - Refutes if:
    - the finish action is absent or explicitly rejected for a documented contract reason.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-003
  - E-004
  - E-005
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: preserve explicit lifecycle `finish_node` as a `taskspace_control` tool call; do not rewrite it to `final_answer`.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: Rollout JSONL contains provider lifecycle timing for the failed B-tier proof
- Related hypotheses:
  - H-001
- Direction: supports
- Type: probe
- Source: `. scripts\taskspace-benchmark\lib\timing.ps1; Get-TaskspaceModelTimingAttribution target\phase-r3-btier-smoke-20260627-003813\single-file-fast-fix\20260627-003814-503\pair-001\right\artifacts\rollout.jsonl`
- Prediction or plan link:
  - H-001 If true prediction.
- Matched signal:
  - `model_timing_source_status=provider_lifecycle_timing`
- Correlation keys:
  - `pair-001\right\artifacts\rollout.jsonl`
- Raw content:
  ```text
  model_request_duration_ms: 117196
  model_timing_event_count: 16
  model_timing_source_status: provider_lifecycle_timing
  model_timing_parse_errors: 0
  ```
- Interpretation: Provider runtime timing exists in the run artifacts and the parser can read it from rollout JSONL.
- Time: 2026-06-27 00:48

## Evidence E-002: Exec JSONL lacks provider lifecycle timing for the same run
- Related hypotheses:
  - H-001
- Direction: supports
- Type: probe
- Source: `. scripts\taskspace-benchmark\lib\timing.ps1; Get-TaskspaceModelTimingAttribution target\phase-r3-btier-smoke-20260627-003813\single-file-fast-fix\20260627-003814-503\pair-001\right\artifacts\whale-exec.jsonl`
- Prediction or plan link:
  - H-001 If true prediction.
- Matched signal:
  - `model_timing_source_status=jsonl_without_timing`
- Correlation keys:
  - `pair-001\right\artifacts\whale-exec.jsonl`
- Raw content:
  ```text
  model_request_duration_ms: null
  model_timing_event_count: 0
  model_timing_source_status: jsonl_without_timing
  model_timing_parse_errors: 0
  ```
- Interpretation: The missing timing in B-tier metrics is caused by runner source selection, not missing provider lifecycle emission.
- Time: 2026-06-27 00:48

## Evidence E-003: B-tier run left one TaskSpace leaf open after business success
- Related hypotheses:
  - H-002
- Direction: neutral
- Type: observation
- Source: `target\phase-r3-btier-smoke-20260627-003813\single-file-fast-fix\20260627-003814-503\pair-001\right\artifacts\graph-health.json` and `metrics.json`
- Prediction or plan link:
  - H-002 problem symptom.
- Matched signal:
  - `open_leaf_nodes=1`
- Correlation keys:
  - `pair-001\right`
- Raw content:
  ```text
  business_success: True
  open_leaf_nodes: 1
  ```
- Interpretation: This proves the graph closeout symptom exists, but does not yet prove whether the cause is model action emission, parser/action contract, or runtime commit.
- Time: 2026-06-27 00:48

## Evidence E-004: The final finish_node existed only as assistant text, not as a tool call
- Related hypotheses:
  - H-002
- Direction: supports
- Type: diagnostic-log
- Source: `target\phase-r3-btier-smoke-20260627-003813\single-file-fast-fix\20260627-003814-503\pair-001\right\artifacts\whale-exec.jsonl`
- Prediction or plan link:
  - H-002 If true prediction.
- Matched signal:
  - `item_98` is an `agent_message` containing `taskspace_control finish_node`; no subsequent `function_call` or `function_call_output` appears for that action.
- Correlation keys:
  - `node-3`
  - `item_98`
- Raw content:
  ```text
  item_98 agent_message:
  {"schema_version":"taskspace-action-v1","action":"taskspace_control","node_id":"node-3","args":{"action":"finish_node","result_validities":[...],"result":"All 3 tax calculation tests pass."},"rationale":"Tests pass; closing smoke_test node and marking all criteria as satisfied."}

  item_100 error:
  TaskSpaceProviderResponseActionabilityV1 actionability=final_candidate recovery_action=none request_count=16/8 phase=validation_recovery node_kind=smoke_test assistant_message_present=true saw_actionable_output=false end_turn=unknown preview=Validation passed; final result is ready.
  ```
- Interpretation: The model emitted a valid lifecycle action, but the runtime did not synthesize or execute a corresponding `taskspace_control` call.
- Time: 2026-06-27 01:06

## Evidence E-005: Code rewrote validation finish_node into final_answer before tool conversion
- Related hypotheses:
  - H-002
- Direction: supports
- Type: code-location
- Source: `third_party\codex-cli\codex-rs\core\src\session\turn.rs`
- Prediction or plan link:
  - H-002 Diagnostic evidence plan: distinguish valid-but-ignored from rejected action.
- Matched signal:
  - `should_answer_after_successful_validation_finish_node` returned true for explicit `taskspace_control(action=finish_node)` on smoke/regression nodes with successful test/build evidence; the caller replaced the action with `taskspace_final_answer_action` before `taskspace_action_to_tool_call`.
- Correlation keys:
  - `should_answer_after_successful_validation_finish_node`
  - `taskspace_final_answer_action`
- Raw content:
  ```text
  else if ... should_answer_after_successful_validation_finish_node(&action, snapshot, sess.as_ref()).await {
      taskspace_final_answer_action("Validation passed; final result is ready.")
  }
  ```
- Interpretation: The runtime intentionally bypassed the explicit lifecycle action, which explains both `saw_actionable_output=false` and the remaining open leaf.
- Time: 2026-06-27 01:06

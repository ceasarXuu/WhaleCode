# Problem P-001: R3 B-tier smoke exposes missing timing attribution and open TaskSpace leaf
- Status: open
- Created: 2026-06-27 00:48
- Updated: 2026-06-27 01:54
- Objective: Make the R3 B-tier smoke produce trustworthy benefit evidence by fixing proven harness attribution gaps and then closing the remaining TaskSpace graph lifecycle blocker.
- Symptoms:
  - B-tier smoke finished with business success on both standard and TaskSpace, but speed evidence was blocked because `model_request_duration_ms` was missing.
  - The same run left `open_leaf_nodes=1` on the TaskSpace side.
  - After graph closeout was fixed, a later B-tier run reached `open_leaf_nodes=0` but the TaskSpace process exited 1 after reopening work with no active node.
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
  - See E-001 through E-007.
- Ruled out:
  - none
- Fix criteria:
  - Timing attribution self-test passes and B-tier metrics can read provider lifecycle timing from `rollout.jsonl`.
  - The graph closeout blocker has a confirmed root cause, focused regression coverage, and a rerun that no longer leaves a successful open leaf.
- Current conclusion: Timing attribution has a confirmed runner source-selection root cause. Graph closeout has a confirmed action-contract rewrite root cause. A follow-on closeout blocker is now confirmed: after all leaves close, action-contract state did not guide or guard final answer, so the model reopened work from a no-active-node state.
- Related hypotheses:
  - H-001
  - H-002
  - H-003
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

## Hypothesis H-003: Closed graph with no active node is not treated as final-answer state
- Status: confirmed
- Parent: P-001
- Claim: After all TaskSpace leaves close and no active node remains, the action-contract prompt and runtime still allow the model to start new work instead of forcing or guiding `final_answer`, causing process exit 1 despite public and hidden validation passing.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - H-002
- Rationale:
  - The post-fix B-tier run had `open_leaf_nodes=0` and validation success, but later emitted `create_node` / `list_files` actions with `node_id=null`.
- Falsifiable predictions:
  - If true: artifacts show all graph nodes completed, then later provider requests have `node_kind=unknown`, `node_id=null`, and rejected work actions rather than `final_answer`.
  - If false: process exit 1 is caused by validation failure, open graph leaf, or missing provider timing.
- Diagnostic evidence plan:
  - Prediction or clause under test: compare graph-health, metrics, and final whale-exec tail after the post-fix B-tier run.
  - Signal: `open_leaf_nodes`, validation exit codes, `TaskSpaceProviderResponseActionabilityV1`, and terminal `turn.failed`.
  - Capture method: inspect B-tier artifacts under `target\phase-r3-btier-smoke-20260627-012652`.
  - Event name or marker:
    - `TaskSpaceProviderResponseActionabilityV1`
    - `turn.failed`
  - Correlation keys:
    - `node_id=null`
    - `node_kind=unknown`
  - Differentiates from:
    - graph closeout failure
    - validation failure
    - timing attribution failure
  - Supports if:
    - graph is closed, validators pass, and the final failure is a no-active-node action-policy loop.
  - Refutes if:
    - an open leaf or failed validator explains the exit.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-006
  - E-007
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: add no-active-node completed-task final-answer guidance and runtime final-answer guard.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-006: Post-fix B-tier proves graph and timing benefits but exits 1
- Related hypotheses:
  - H-003
- Direction: supports
- Type: reproduction
- Source: `target\phase-r3-btier-smoke-20260627-012652\single-file-fast-fix\20260627-012654-869`
- Prediction or plan link:
  - H-003 If true prediction.
- Matched signal:
  - `open_leaf_nodes=0`, `model_request_duration_ms=503738`, public and hidden validation exit 0, but TaskSpace `exec_exit_code=1`.
- Correlation keys:
  - `pair-001\right`
- Raw content:
  ```text
  right metrics:
  exec_exit_code: 1
  public_validation_exit_code: 0
  hidden_oracle_exit_code: 0
  business_success: false
  open_leaf_nodes: 0
  model_request_duration_ms: 503738
  model_timing_source_status: provider_lifecycle_timing
  ```
- Interpretation: The prior graph and timing fixes produced their direct benefits, but did not fully close the session success path.
- Time: 2026-06-27 01:54

## Evidence E-007: Final failure reopens work from no active node
- Related hypotheses:
  - H-003
- Direction: supports
- Type: diagnostic-log
- Source: `target\phase-r3-btier-smoke-20260627-012652\single-file-fast-fix\20260627-012654-869\pair-001\right\artifacts\whale-exec.jsonl`
- Prediction or plan link:
  - H-003 If true prediction.
- Matched signal:
  - After node closeout, provider requests report `phase=unknown node_kind=unknown`; assistant emits repeated `create_node` actions and finally `list_files node_id=null`, which is rejected with `node_policy_violation:unknown:list_files`.
- Correlation keys:
  - `item_147`
  - `item_153`
  - `item_159`
  - `item_165`
  - `item_171`
  - `item_173`
- Raw content:
  ```text
  item_147: taskspace_control create_node kind=inspect_code_context
  item_153: taskspace_control create_node kind=inspect_code_context
  item_159: taskspace_control create_node kind=inspect_code_context
  item_165: taskspace_control create_node kind=inspect_code_context
  item_171: list_files node_id=null
  item_173: TaskSpaceActionV1 rejected: node_policy_violation:unknown:list_files
  turn.failed: too many non-action assistant messages while requesting follow-up
  ```
- Interpretation: The final-answer state after graph closeout is under-specified; the model is still being routed as if more TaskSpace work should start.
- Time: 2026-06-27 01:54

## Hypothesis H-004: Current B-tier rerun is blocked by local Windows commit/pagefile pressure
- Status: confirmed
- Parent: P-001
- Claim: After the no-active-node fix, the next B-tier proof cannot currently start because this Windows host cannot build a fresh `whale.exe`; the failures are resource exhaustion during Rust metadata/codegen/link, not source-level compile errors.
- Layer: environment
- Factor relation: blocking
- Depends on:
  - H-003
- Rationale:
  - B-tier needs a `whale.exe` built after commit `76e0b96e`.
  - The only available `phase-r3-current-cargo-target\debug\whale.exe` is older than the commit.
  - Fresh and incremental builds fail with out-of-memory / pagefile errors before producing an updated executable.
- Falsifiable predictions:
  - If true: reducing parallelism or using `dev-small` still fails with Windows commit/pagefile or rustc allocation errors, while prior targeted unit/regression tests remain compiled and passing.
  - If false: cargo reports deterministic Rust source errors tied to this patch, or a fresh post-commit `whale.exe` is produced.
- Diagnostic evidence plan:
  - Prediction or clause under test: compare executable timestamp, cargo build output, and OS memory/pagefile state.
  - Signal: `LastWriteTimeUtc`, `rustc-LLVM ERROR: out of memory`, `memory allocation ... failed`, Windows `os error 1455`, free physical memory, pagefile usage.
  - Capture method: run candidate builds with fresh target, existing target incremental, and `dev-small`; inspect `Win32_OperatingSystem` and `Win32_PageFileUsage`.
  - Event name or marker:
    - `rustc-LLVM ERROR: out of memory`
    - `页面文件太小，无法完成操作。 (os error 1455)`
  - Differentiates from:
    - source compile error
    - stale binary preflight failure
    - B-tier business/graph failure
  - Supports if:
    - no post-commit binary is produced and cargo fails in metadata/codegen/link with memory or pagefile signals.
  - Refutes if:
    - cargo produces a fresh post-commit `whale.exe`, or reports deterministic source errors.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-008
- Conclusion: confirmed
- Repair design readiness: blocked
- Next step: free local memory / increase Windows commit limit, then rebuild with `--profile dev-small -j1` or existing-target incremental and rerun B-tier.
- Blocker:
  - user/operator action required to free memory or increase pagefile / close memory-heavy background agents.
- Close reason:
  - not closed

## Evidence E-008: Post-fix B-tier proof cannot start because fresh whale.exe build exhausts Windows commit/pagefile
- Related hypotheses:
  - H-004
- Direction: supports
- Type: environment
- Source: local build attempts on 2026-06-27
- Prediction or plan link:
  - H-004 diagnostic evidence plan.
- Matched signal:
  - Existing binary timestamp is older than HEAD:
    `phase-r3-current-cargo-target\debug\whale.exe LastWriteTimeUtc=2026-06-26T17:26:35.1780363Z`;
    HEAD commit time is `2026-06-27T02:05:21+08:00`.
  - Fresh target build failed with `rustc-LLVM ERROR: out of memory` and invalid rmeta follow-on errors.
  - Existing target incremental build reached `codex-cli` final binary compile but failed with `memory allocation of 2097152 bytes failed`.
  - `dev-small` build failed with `页面文件太小，无法完成操作。 (os error 1455)` while reading Rust toolchain metadata.
  - OS snapshots showed low free physical memory, including about 2.3GB before retry and about 1.6GB after later attempts; pagefile usage was high (`AllocatedBaseSize=49152`, `CurrentUsage=22207`, `PeakUsage=23248` MB).
- Correlation keys:
  - `76e0b96e`
  - `phase-r3-current-cargo-target-2`
  - `phase-r3-current-cargo-target-3`
  - `dev-small`
- Raw content:
  ```text
  rustc-LLVM ERROR: out of memory
  memory allocation of 2097152 bytes failed
  页面文件太小，无法完成操作。 (os error 1455)
  FreePhysicalMemory: 2359512 KB, then 1648988 KB
  PageFile: AllocatedBaseSize=49152 MB, CurrentUsage=22207 MB, PeakUsage=23248 MB
  ```
- Interpretation: The current blocker is host resource exhaustion before B-tier can run a post-fix executable. This does not disprove the no-active-node fix; it prevents measuring its real B-tier benefit until the host can build.
- Time: 2026-06-27 03:22

## Hypothesis H-005: Timing gate remains blocked because provider queue/retry attribution is marked unavailable despite lifecycle evidence
- Status: confirmed
- Parent: P-001
- Claim: The benchmark timing layer hard-coded `model_queue_wait_ms` and `model_retry_backoff_ms` as unavailable even though provider lifecycle events already expose enough state to compute stream-open wait and no-retry backoff for real B-tier runs.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - H-001
- Rationale:
  - Real rollout events include `status:started`, `status:stream_opened`, `status:response_completed`, `started_at_ms`, `completed_at_ms`, `createdAtMs`, `logical_request_id`, and `attempt_seq`.
  - `scripts\taskspace-benchmark\lib\timing.ps1` nevertheless emitted `wait_attribution_unavailable_fields` for queue/retry and forced `runtime_optimization_status=blocked`.
- Falsifiable predictions:
  - If true: parsing the same B-tier `rollout.jsonl` with lifecycle-derived queue/retry logic yields non-null `model_queue_wait_ms`, zero retry backoff when no retry attempts exist, and complete wait attribution.
  - If false: rollout lacks `stream_opened` or terminal lifecycle events, or retry absence cannot be distinguished from missing retry telemetry.
- Diagnostic evidence plan:
  - Prediction or clause under test: compute timing from provider lifecycle events in a real B-tier artifact and rerun B-tier after parser/aggregation changes.
  - Signal: direct parser output, `sample-timing.json`, pair metrics, cache/context artifacts, and pair report outcome.
  - Capture method: run `Get-TaskspaceModelTimingAttribution` on real `rollout.jsonl`, then rerun B-tier `single-file-fast-fix`.
  - Event name or marker:
    - `status:stream_opened`
    - `status:response_completed`
    - `wait_attribution_status`
  - Correlation keys:
    - `target\phase-r3-btier-smoke-20260627-035703`
    - `target\phase-r3-btier-smoke-20260627-041043`
  - Differentiates from:
    - provider missing lifecycle timing
    - graph closeout failure
    - cache/context replacement failure
  - Supports if:
    - timing fields become non-null and wait attribution becomes complete while other gates remain green.
  - Refutes if:
    - timing remains unavailable/missing after lifecycle-derived parsing.
  - Instrumentation status: retained
  - Instrumentation lifecycle:
    - retained as benchmark timing attribution logic.
- Evidence gate: satisfied
- Related evidence:
  - E-009
  - E-010
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: keep speedup claim blocked by actual pair outcome if TaskSpace remains slower, not by missing timing.
- Blocker:
  - none for timing attribution gate.
- Close reason:
  - not closed

## Evidence E-009: Existing B-tier rollout contains enough lifecycle evidence for queue/retry attribution
- Related hypotheses:
  - H-005
- Direction: supports
- Type: diagnostic-run
- Source: `target\phase-r3-btier-smoke-20260627-035703\single-file-fast-fix\20260627-035705-541\pair-001\right\artifacts\rollout.jsonl`
- Prediction or plan link:
  - H-005 If true prediction.
- Matched signal:
  - Direct parser output after the timing fix:
    `model_request_duration_ms=112764`, `model_queue_wait_ms=6825`, `model_retry_backoff_ms=0`, `model_timing_event_count=12`, `model_timing_source_status=provider_lifecycle_timing`.
- Correlation keys:
  - `status:started`
  - `status:stream_opened`
  - `status:response_completed`
- Raw content:
  ```text
  Get-TaskspaceModelTimingAttribution rollout.jsonl:
    model_request_duration_ms: 112764
    model_queue_wait_ms: 6825
    model_retry_backoff_ms: 0
    model_timing_source_status: provider_lifecycle_timing
  ```
- Interpretation: Queue/retry attribution was available from existing lifecycle events; the blocker was benchmark parser/aggregation logic.
- Time: 2026-06-27 04:10

## Evidence E-010: Fresh B-tier passes business, graph, context, cache, and wait attribution gates but remains cost-higher
- Related hypotheses:
  - H-003
  - H-005
- Direction: supports
- Type: fix-validation
- Source: `target\phase-r3-btier-smoke-20260627-041043\single-file-fast-fix\20260627-041044-436`
- Prediction or plan link:
  - H-003 fix validation and H-005 fix validation.
- Matched signal:
  - TaskSpace metrics: `exec_exit_code=0`, `business_success=true`, public/hidden validation exit 0, `open_leaf_nodes=0`.
  - Timing: `wait_attribution_status=complete`, `model_queue_wait_ms=9952`, `model_retry_backoff_ms=0`, `model_request_duration_ms=166112`, `runtime_optimization_status=ready`.
  - Context/cache: `exact_context_bundle_verified=true`, `replacement_confirmed=true`, `raw_taskspace_control_history_tokens=0`, `request_2_plus_hit_rate=0.986813`, `cache_usage_missing_count=0`.
  - Outcome remains `both_success_taskspace_cost_higher`; TaskSpace wall-time ratio 4.87, tool-call ratio 1.38.
- Correlation keys:
  - `pair-001`
  - `provider-cache-trace-summary.json`
  - `active-context-replacement-report.json`
  - `sample-timing.json`
- Raw content:
  ```text
  wait_attribution_status = complete
  runtime_optimization_status = ready
  business_success = true
  open_leaf_nodes = 0
  request_2_plus_hit_rate = 0.986813
  outcome = both_success_taskspace_cost_higher
  taskspace_wall_time_ratio = 4.87
  taskspace_tool_call_ratio = 1.38
  ```
- Interpretation: R3 correctness, graph closeout, context replacement, cache-hit, and timing observability gates are now evidenced on B-tier. The real remaining blocker is not missing telemetry; it is that this B-tier sample still shows TaskSpace slower/costlier than standard.
- Time: 2026-06-27 04:16

## Hypothesis H-006: P0 deep TaskSpace run times out because action-contract execution stays in read/model-sampling loops instead of converging to implementation
- Status: confirmed
- Parent: P-001
- Claim: On the P0 `processing-pipeline` targeted diagnostic, TaskSpace context/cache replacement works, but the action-contract loop does not force a transition from repeated read/model-sampling actions into implementation and validation, so the run consumes 95 provider requests and hits the 900s process timeout.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - H-005
- Rationale:
  - B-tier thin task validates context/cache/timing, but P0 deep task exercises a longer inspect -> implement transition.
  - The P0 run starts TaskSpace and performs action-contract reads, but remains on the inspect node and never produces changed files.
  - `provider-cache-trace-summary.json` and `active-context-replacement-report.json` pass, so the timeout is not caused by the old raw-history/context pollution blocker.
- Falsifiable predictions:
  - If true: the P0 diagnostic shows high provider request count, high cache hit, exact context replacement pass, zero changed files on TaskSpace side, no business success, and phase diversity failing because requests remain mostly `model_sampling`.
  - If false: the run fails because cache/context replacement regressed, because the validator/harness failed before agent execution, or because implementation edits were made but were semantically wrong.
- Diagnostic evidence plan:
  - Prediction or clause under test: compare P0 pair report, request summary, cache summary, active replacement report, phase summary, and rollout action traces.
  - Signal: `exec_exit_code`, `exec_timed_out`, `request_2_plus_hit_rate`, `exact_context_bundle_verified`, `replacement_confirmed`, `phase_diversity_gate_pass`, provider request count, changed paths, and action trace lines.
  - Capture method: run targeted `processing-pipeline` diagnostic from pinned terminal-bench source and inspect structured artifacts only; avoid loading full 344MB rollout except targeted `rg` probes.
  - Event name or marker:
    - `tool_free_action_contract`
    - `provider_response_actionability`
    - `main_tool_result`
    - `phase_diversity_gate_pass`
  - Correlation keys:
    - `target\phase-r3-targeted-diagnostic-20260627-1616`
    - `terminal_bench__processing-pipeline`
    - `pair-001`
  - Differentiates from:
    - active context replacement failure
    - DeepSeek cache miss instability
    - timing attribution failure
    - validator/docker harness failure
  - Supports if:
    - context/cache/timing pass while TaskSpace has many provider requests, no edits, no business success, and timeout.
  - Refutes if:
    - context/cache/timing fail first, or TaskSpace produces implementation edits that fail validation.
  - Instrumentation status: retained
  - Instrumentation lifecycle:
    - retain phase/cache/context/actionability summaries as permanent diagnostic artifacts.
- Evidence gate: satisfied
- Related evidence:
  - E-011
  - E-012
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: inspect the action-contract transition/recovery code path and design a structural convergence fix for deep TaskSpace runs.
- Blocker:
  - none for diagnosis; implementation still needs code-path inspection.
- Close reason:
  - not closed

## Evidence E-011: Current-HEAD non-agent gates pass after Rust gate timeout fix
- Related hypotheses:
  - H-006
- Direction: neutral
- Type: regression-test
- Source: `target\phase-r3-non-agent-gates-20260627-160303\v005-non-agent-gates.json`
- Prediction or plan link:
  - H-006 differentiates P0 runtime convergence from non-agent gate failure.
- Matched signal:
  - Artifact status `pass`, git commit `435999bb0fbcff87b40b97c8770a6fb2b7e63804`.
  - All subgates pass: `provider_request_hook`, `runtime_budget_response`, `budget_quality_impact`, `active_context_replacement`, `state_commit_displacement`, `spawn_node_budget`, `request_phase_attribution`, `release_decision_fixture`, `start_gate_fixture`.
  - The earlier failure was timeout-only: single `cargo test -p codex-core provider_request_budget --lib` passed in about 254s, which exceeded the old 240s builder timeout.
- Correlation keys:
  - `435999bb0`
  - `build-v005-non-agent-gates.ps1`
  - `profile_hash=7059327f0b8b501ad3399f07ca95d8d994e6ff9ed8199dd96079881ec0b51e3a`
- Raw content:
  ```text
  v005-non-agent-gates status = pass
  provider_request_hook = pass
  active_context_replacement = pass
  state_commit_displacement = pass
  spawn_node_budget = pass
  start_gate_fixture = pass
  ```
- Interpretation: The current blocker is not non-agent gate readiness. The gate infrastructure can produce a current-HEAD pass artifact after the Rust test timeout is made realistic for this Windows host.
- Time: 2026-06-27 16:08

## Evidence E-012: P0 targeted diagnostic times out despite context/cache/timing gates passing
- Related hypotheses:
  - H-006
- Direction: supports
- Type: diagnostic-run
- Source: `target\phase-r3-targeted-diagnostic-20260627-1616\runs\terminal_bench__processing-pipeline\20260627-161031-920`
- Prediction or plan link:
  - H-006 If true prediction.
- Matched signal:
  - Pair report: TaskSpace `exec_exit_code=124`, `exec_timed_out=true`, `business_success=false`, `wall_time_ms=900054`, changed paths empty.
  - Request summary from rollout: `model_request_count=96`, `input_tokens=11538792`, `cached_input_tokens=11398016`, `output_tokens=49096`, `last_input_tokens_per_request=120236`.
  - Cache: `request_2_plus_hit_rate=0.987655`, `cache_usage_missing_count=0`, `native_tools_schema_hot_path_count=0`.
  - Context replacement: `exact_context_bundle_verified=true`, `replacement_confirmed=true`, `legacy_taskspace_history_present=false`, `raw_taskspace_control_history_tokens=0`, `protected_items_present=true`.
  - Timing: `wait_attribution_status=complete`, `model_queue_wait_ms=64457`, `model_retry_backoff_ms=0`, `model_request_duration_ms=580872`.
  - Phase summary: `phase_diversity_gate_pass=false`, `model_sampling` dominates, only 4 `budget_recovery` events.
  - Targeted rollout probe: initial `taskspace_control(start_task)` succeeds; later traces show `provider_response_actionability` and `main_tool_result` read actions, but no implementation edits before timeout.
- Correlation keys:
  - `processing-pipeline`
  - `tool_free_action_contract`
  - `phase_diversity_gate_pass=false`
  - `exec_timeout`
- Raw content:
  ```text
  TaskSpace exec_exit_code = 124
  TaskSpace exec_timed_out = true
  TaskSpace changed_paths = empty
  rollout_trace.model_request_count = 96
  request_2_plus_hit_rate = 0.987655
  exact_context_bundle_verified = true
  replacement_confirmed = true
  wait_attribution_status = complete
  phase_diversity_gate_pass = false
  ```
- Interpretation: The P0 failure is a real convergence/runtime behavior issue after R3 context/cache/timing fixes. It is not explained by cache misses, legacy context leakage, missing wait attribution, or validator preflight failure.
- Time: 2026-06-27 16:44

## Hypothesis H-007: P0 repeated-read convergence improved after dynamic implement-needs-edit state, but validation actions were misclassified
- Status: confirmed
- Parent: P-001
- Claim: The new implement-needs-edit state moves the P0 sample out of the unbounded read loop and into an edit, but the action-contract `run_test` shell call loses its semantic `test` class during tool attribution and is blocked on the `smoke_test` node as `unknown`.
- Layer: root-cause
- Factor relation: sequential
- Depends on:
  - H-006
- Rationale:
  - The `205657` diagnostic no longer times out and has a real `apply_patch`, so the original no-edit convergence failure is materially improved.
  - The same run emits two `run_test` action-contract calls on `node-3` (`smoke_test`), but both are blocked before execution because the shell text classifier treats `bash /app/run_pipeline.sh` and `./run_pipeline.sh` as `unknown`.
  - This is an attribution-layer bug: the provider already supplied `action=run_test`, so runtime should not discard that semantic action and re-infer only from shell text.
- Falsifiable predictions:
  - If true: rollout contains `taskspace-action-contract-*-run_test` function calls followed by `tool_action_blocked` with `actionClass=unknown` and reason `smoke_test does not allow unknown`.
  - If true after repair: a unit test can prove `taskspace-action-contract-*-run_test` maps to `ActionClass::Test` while ordinary `bash /app/run_pipeline.sh` remains `Unknown`.
  - If false: the block is caused by node policy disallowing tests, by the action-contract parser rejecting `run_test`, or by a validator/runtime environment failure.
- Diagnostic evidence plan:
  - Prediction or clause under test: inspect the P0 rollout for run_test call ids and blocked runtime events; inspect `tools/parallel.rs` attribution code; add a targeted unit test for action-contract class preservation.
  - Signal: `call_id=taskspace-action-contract-*-run_test`, `actionClass=unknown`, `reason=smoke_test does not allow unknown`, and a passing unit test for `run_test -> ActionClass::Test`.
  - Capture method: use targeted `rg` over the rollout, code inspection, and `cargo test -j1 -p codex-core taskspace_action_contract_call_id_preserves_run_test_class --lib`.
  - Event name or marker:
    - `tool_action_blocked`
    - `taskspace-action-contract-23-run_test`
    - `taskspace-action-contract-25-run_test`
  - Correlation keys:
    - `target\phase-r3-targeted-diagnostic-20260627-205657`
    - `pair-001\right\artifacts\rollout.jsonl`
    - `node-3`
  - Differentiates from:
    - semantic agent patch failure
    - validator/docker failure
    - smoke_test policy rejecting all tests
  - Supports if:
    - the rollout block reason is `unknown`, while action-contract call ids explicitly name `run_test`.
  - Refutes if:
    - `run_test` is mapped to `test` but still blocked, or no `run_test` action exists.
  - Instrumentation status: retained
  - Instrumentation lifecycle:
    - keep call-id based action-class tests as permanent regression coverage.
- Evidence gate: satisfied
- Related evidence:
  - E-013
  - E-014
  - E-015
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: rebuild `whale` and rerun the targeted P0 sample to verify `run_test` is no longer blocked as `unknown`.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-013: Dynamic implement-needs-edit state reduces the no-edit loop and produces an edit
- Related hypotheses:
  - H-006
  - H-007
- Direction: supports
- Type: diagnostic-run
- Source: `target\phase-r3-targeted-diagnostic-20260627-205657\runs\terminal_bench__processing-pipeline\20260627-205724-454\pair-001`
- Prediction or plan link:
  - H-006 repair validation and H-007 rationale.
- Matched signal:
  - Pair report: TaskSpace right side `exec_exit_code=1`, `exec_timed_out=false`, `wall_time_ms=247708`, `tool_call_count=21`, changed paths `collect_data.sh` and `data/output/raw_data.txt`.
  - Request phase summary: `provider_request_distinct_count=25`, `phase_diversity_gate_pass=true`, `model_sampling=84`, `budget_recovery=4`, `validation_recovery=12`.
  - Metrics: `rollout_trace_model_request_count=26`, `rollout_trace_input_tokens=3124918`, `taskspace_control_count=2`, `nodes=3`, `accepted_results=2`.
  - Whale exec trace: one `TaskSpaceImplementNeedsEditRecoveryV1` then `item_111` is an `apply_patch` against `collect_data.sh`.
- Correlation keys:
  - `TaskSpaceImplementNeedsEditRecoveryV1`
  - `item_111`
  - `phase_diversity_gate_pass=true`
- Raw content:
  ```text
  right exec_timed_out = false
  right changed_paths = collect_data.sh, data/output/raw_data.txt
  provider_request_distinct_count = 25
  phase_diversity_gate_pass = true
  item_111 action = apply_patch
  ```
- Interpretation: The previous no-edit timeout mechanism is no longer the primary blocker in this run. The model made an edit and progressed to validation, exposing later correctness and action-class issues.
- Time: 2026-06-27 21:15

## Evidence E-014: Action-contract run_test is blocked as unknown on smoke_test
- Related hypotheses:
  - H-007
- Direction: supports
- Type: diagnostic-run
- Source: `target\phase-r3-targeted-diagnostic-20260627-205657\runs\terminal_bench__processing-pipeline\20260627-205724-454\pair-001\right\artifacts\rollout.jsonl`
- Prediction or plan link:
  - H-007 prediction: run_test call ids followed by `actionClass=unknown` block.
- Matched signal:
  - Rollout line 620: function call `shell_command` with call id `taskspace-action-contract-23-run_test`, command `bash /app/run_pipeline.sh`.
  - Rollout line 625: `tool_action_blocked`, `nodeKind=smoke_test`, `actionClass=unknown`, reason `smoke_test does not allow unknown`.
  - Rollout line 630: function output says the same call was blocked as `unknown`.
  - Rollout lines 670 and 680 repeat the pattern for `taskspace-action-contract-25-run_test`, command `./run_pipeline.sh`.
- Correlation keys:
  - `taskspace-action-contract-23-run_test`
  - `taskspace-action-contract-25-run_test`
  - `tool_action_blocked`
- Raw content:
  ```text
  call_id=taskspace-action-contract-23-run_test command=bash /app/run_pipeline.sh
  tool_action_blocked nodeKind=smoke_test actionClass=unknown reason=smoke_test does not allow unknown
  call_id=taskspace-action-contract-25-run_test command=./run_pipeline.sh
  TaskSpace blocked this tool call ... action class: unknown
  ```
- Interpretation: The smoke_test policy itself is not the issue; the action semantic is lost before policy evaluation. The action-contract call id retains the source action and is the correct low-risk attribution key.
- Time: 2026-06-27 21:28

## Evidence E-015: Action-contract class preservation fix passes targeted and TaskSpace regression tests
- Related hypotheses:
  - H-007
- Direction: supports
- Type: fix-validation
- Source: `third_party\codex-cli\codex-rs\core\src\tools\parallel.rs`
- Prediction or plan link:
  - H-007 after-repair prediction.
- Matched signal:
  - Code change: `ToolCallRuntime::classify_taskspace_tool_action` now first checks `taskspace_action_contract_class(&call.call_id)` before shell payload inference.
  - Mapping: `run_test -> ActionClass::Test`, `apply_patch -> Edit`, `taskspace_control -> Control`, `list_files/read_file -> Read`, `search -> Search`.
  - Targeted unit test: `taskspace_action_contract_call_id_preserves_run_test_class` passed.
  - TaskSpace aggregate regression: `cargo test -j1 -p codex-core taskspace --lib` passed, `99 passed`.
  - Guardrail: the test also asserts ordinary `bash /app/run_pipeline.sh` remains `ActionClass::Unknown`, so only action-contract calls get this semantic override.
- Correlation keys:
  - `taskspace_action_contract_class`
  - `taskspace_action_contract_call_id_preserves_run_test_class`
  - `cargo test -j1 -p codex-core taskspace --lib`
- Raw content:
  ```text
  test tools::parallel::tests::taskspace_action_contract_call_id_preserves_run_test_class ... ok
  test result: ok. 1 passed; 0 failed
  test result: ok. 99 passed; 0 failed; 1906 filtered out
  ```
- Interpretation: The engineering regression that turned action-contract validation into an unknown shell action is fixed at the attribution boundary without broadening ordinary shell command classification.
- Time: 2026-06-27 21:34

## Evidence E-016: 205657 remaining TaskSpace failure is semantic, not cache/context/timing
- Related hypotheses:
  - H-007
- Direction: neutral
- Type: diagnostic-run
- Source: `target\phase-r3-targeted-diagnostic-20260627-205657\runs\terminal_bench__processing-pipeline\20260627-205724-454\pair-001\right\artifacts\validation.stdout.log`
- Prediction or plan link:
  - H-007 differentiates action attribution from agent semantic correctness.
- Matched signal:
  - Public validation reached tests and completed.
  - Failures show `generate_report.sh` still has `#!/bin/nonexistent`, pipeline exits 1 because `process_data.sh` expects `/data/output/raw_data.txt`, and expected `/data/output/*` files are missing.
  - The model edited `collect_data.sh` to use `./data/output`, which contradicts validator expectations.
- Correlation keys:
  - `test_correct_shebang`
  - `test_pipeline_execution`
  - `test_all_output_files_created`
  - `test_data_was_processed`
- Raw content:
  ```text
  FAILED test_correct_shebang: '#!/bin/nonexistent' != '#!/bin/bash'
  FAILED test_pipeline_execution: Error: Input data not found!
  FAILED test_all_output_files_created: /data/output/raw_data.txt should be created
  FAILED test_data_was_processed: FileNotFoundError /data/output/processed_data.txt
  ```
- Interpretation: After the convergence fix, this run still fails because the agent chose the wrong patch and missed `generate_report.sh`. That is a remaining semantic solving failure to evaluate after the `run_test` attribution repair is validated live.
- Time: 2026-06-27 21:31

## Hypothesis H-008: Interrupted benchmark source protection can leave stale Windows deny ACLs that break later source hashing
- Status: confirmed
- Parent: P-001
- Claim: The external source guard can fail before applying fresh protection when a previous interrupted run left a deny ACE on the pinned terminal-bench source file; the guard must remove stale deny ACEs before computing source hashes.
- Layer: root-cause
- Factor relation: independent
- Depends on:
  - H-006
- Rationale:
  - The `203312` diagnostic failed before measuring TaskSpace because `Get-FileHash` on the pinned `run-tests.sh` source returned access denied.
  - Inspecting ACLs showed a deny `Read, Synchronize` ACE for the current user on that source file.
  - Removing the stale deny ACE made `Get-FileHash` succeed.
- Falsifiable predictions:
  - If true: stale ACL inspection shows a deny ACE; after removing it, hash succeeds; source guard smoke with the real manifest shape protects and releases the source with remove/deny/release exit codes all zero.
  - If false: access denied persists after stale deny removal, or the source guard cannot protect/release the same file cleanly.
- Diagnostic evidence plan:
  - Prediction or clause under test: check ACL on the pinned sensitive file, remove stale deny, retry hash, then run source guard smoke using `ExternalBenchmark.adapter_metadata.sensitive_source_files`.
  - Signal: ACL deny ACE, successful SHA256 hash, and source guard smoke JSON with `protected=true`, `released=true`, `stale_remove_exit=0`, `deny_exit=0`, `remove_exit=0`.
  - Capture method: PowerShell `Get-Acl`, `icacls /remove:d`, `Get-FileHash`, and targeted source guard smoke.
  - Event name or marker:
    - `stale_deny_remove_exit_code`
    - `sensitive_source_files`
  - Correlation keys:
    - `target\terminal-bench-pinned-1a6ffa9\original-tasks\processing-pipeline\run-tests.sh`
    - `phase-r3-targeted-diagnostic-20260627-203312`
- Evidence gate: satisfied
- Related evidence:
  - E-017
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: keep stale-deny removal fields in source guard proof for future benchmark reruns.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-017: Source guard stale deny removal fixes pinned benchmark hash access
- Related hypotheses:
  - H-008
- Direction: supports
- Type: fix-validation
- Source: `scripts\taskspace-benchmark\lib\source-guard.ps1`
- Prediction or plan link:
  - H-008 prediction and repair validation.
- Matched signal:
  - Pre-fix pinned source ACL included `ROG306\77585 Deny Read, Synchronize`, and `Get-FileHash` failed with access denied.
  - After `icacls ... /remove:d ROG306\77585`, `Get-FileHash` succeeded with SHA256 `FD7911468D6830532C48CA5049E0F67D1DD4AECD768D1470F21EFCE55C81BE97`.
  - Source guard now removes stale deny ACEs before hashing and records `stale_deny_remove_exit_code` / `stale_deny_remove_output`.
  - Correct manifest-shape smoke result: `protected=true`, `released=true`, `stale_remove_exit=0`, `deny_exit=0`, `remove_exit=0`.
- Correlation keys:
  - `Protect-TaskspaceExternalSensitiveSource`
  - `stale_deny_remove_exit_code`
  - `ExternalBenchmark.adapter_metadata.sensitive_source_files`
- Raw content:
  ```text
  stale ACL: ROG306\77585 Deny Read, Synchronize
  Get-FileHash SHA256 FD7911468D6830532C48CA5049E0F67D1DD4AECD768D1470F21EFCE55C81BE97
  {"protected":true,"released":true,"stale_remove_exit":0,"deny_exit":0,"remove_exit":0}
  ```
- Interpretation: The ACL failure was benchmark infrastructure contamination from a previous interrupted protection cycle, not a TaskSpace agent behavior failure. The guard now self-heals stale deny ACEs before re-protecting sensitive sources.
- Time: 2026-06-27 20:45

## Evidence E-018: Live rerun confirms run_test is preserved as test, exposing the next convergence issue
- Related hypotheses:
  - H-007
- Direction: supports
- Type: fix-validation
- Source: `target\phase-r3-targeted-diagnostic-20260627-2142\runs\terminal_bench__terminal_bench__processing-pipeline\20260627-213629-806`
- Prediction or plan link:
  - H-007 next step: live targeted P0 validation after rebuilding `whale`.
- Matched signal:
  - `run-status.json`: `phase=completed`, `run_validity=valid`, `exit_code=0`, `attempted_pairs=1`, `completed_pairs=1`.
  - Pair report: standard side solved; TaskSpace side still false, but no longer because `run_test` is blocked as `unknown`.
  - Rollout line 636: `taskspace-action-contract-24-run_test` invokes `shell_command` with `bash run_pipeline.sh`.
  - Rollout line 645 / trace-339: `main_tool_result`, `callId=taskspace-action-contract-24-run_test`, `actionClass=test`, `toolSuccess=false`, tags `tool_failure`, `validator_failure`.
  - No matching `smoke_test does not allow unknown` for the run_test call.
- Correlation keys:
  - `20260627-213629-806`
  - `taskspace-action-contract-24-run_test`
  - `trace-339`
- Raw content:
  ```text
  run_validity = valid
  outcome_standard = solved
  outcome_taskspace = engineering_unclean
  call_id = taskspace-action-contract-24-run_test
  actionClass = test
  toolSuccess = false
  unclassifiedShellWarningCount = 0
  ```
- Interpretation: The action-class preservation repair is validated live. The next failure is not action-class policy; it is that implementation still re-reads too long, chooses a partial/wrong patch, and the local smoke command fails with a Windows/WSL `Bash/Service/CreateInstance/E_ACCESSDENIED` tool failure before the external Docker validator reports the semantic misses.
- Time: 2026-06-27 21:58

## Hypothesis H-009: Implement nodes re-read because inherited inspect evidence is not treated as implementation convergence evidence
- Status: confirmed
- Parent: P-001
- Claim: After a forced inspect -> implement transition, the implementation node contains enough dependency evidence to patch, but the runtime only counts reads performed on the implementation node itself. That allows a second read loop, loses focus on high-signal dependency evidence, and delays edit pressure until the implement node itself reaches the read threshold.
- Layer: root-cause
- Factor relation: sequential
- Depends on:
  - H-007
- Rationale:
  - The `2142` live rerun shows inspect result `result-14` already captured `generate_report.sh` with `#!/bin/nonexistent`, but the implement node continued to read/list through request 21 before `TaskSpaceImplementNeedsEditRecoveryV1`.
  - The actual patch only changed `process_data.sh`, leaving `generate_report.sh` and output path expectations unresolved.
  - Code inspection showed `current_main_implement_progress_needs_edit` and the runtime read/search gate used only `current_node_progress_signature`, ignoring dependency nodes.
- Falsifiable predictions:
  - If true: after inspect transition, an implement node with dependency code/test evidence returns `current_main_implement_progress_needs_edit() == true` before any implement-node read.
  - If true after repair: a read/search tool call on that implement node is blocked with a reason including `dependency_working_evidence`, while `apply_patch` remains allowed.
  - If false: dependency evidence is already considered, or the repeated reads are caused by parser/action-contract rejection rather than runtime state.
- Diagnostic evidence plan:
  - Prediction or clause under test: create an inspect node, record a successful high-signal shell script read, force transition to implement, then attempt an implementation read.
  - Signal: `current_main_implement_progress_needs_edit()`, `dependency_working_evidence` in the gate error, and action-contract state containing `implementation_needs_edit`.
  - Capture method: focused Rust unit tests plus TaskSpace aggregate regression.
  - Event name or marker:
    - `current_node_has_dependency_working_evidence`
    - `dependency_working_evidence`
    - `TaskSpaceImplementNeedsEditRecoveryV1`
  - Correlation keys:
    - `result-14`
    - `generate_report.sh`
    - `#!/bin/nonexistent`
- Evidence gate: satisfied
- Related evidence:
  - E-019
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: rebuild `whale` and rerun the targeted P0 sample to check whether implement re-read loops are reduced and whether the model patches all high-signal files.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-019: Dependency evidence now narrows implement nodes immediately
- Related hypotheses:
  - H-009
- Direction: supports
- Type: fix-validation
- Source: `third_party\codex-cli\codex-rs\core\src\action_map\runtime.rs`, `third_party\codex-cli\codex-rs\core\src\session\turn.rs`
- Prediction or plan link:
  - H-009 after-repair prediction.
- Matched signal:
  - Snapshot now exposes `current_node_has_dependency_working_evidence`.
  - Runtime gate blocks implement read/search if the current implement node has no successful edit and either dependency working evidence exists or current-node progress has reached the hint.
  - Provider-visible action-contract state uses the same condition via `taskspace_snapshot_requires_implementation_edit`.
  - Focused tests passed:
    - `taskspace_action_contract_state_narrows_implementation_from_dependency_evidence`
    - `implement_dependency_evidence_needs_edit_immediately_after_inspect_transition`
  - Aggregate TaskSpace regression passed: `cargo test -j1 -p codex-core taskspace --lib`, `100 passed`.
- Correlation keys:
  - `implement_node_has_dependency_working_evidence`
  - `current_node_has_dependency_working_evidence`
  - `dependency_working_evidence`
- Raw content:
  ```text
  test taskspace_action_contract_state_narrows_implementation_from_dependency_evidence ... ok
  test implement_dependency_evidence_needs_edit_immediately_after_inspect_transition ... ok
  test result: ok. 100 passed; 0 failed; 1907 filtered out
  ```
- Interpretation: The second read loop is addressed structurally at the graph boundary. Implementation nodes created from inspected evidence now start in an edit/block convergence state instead of rebuilding their own read history.
- Time: 2026-06-27 22:13

## Hypothesis H-010: Existing-file add-patch rejection consumes recovery before the model can update mandatory evidence
- Status: confirmed
- Parent: P-001
- Claim: When action-contract apply_patch tries to add a file that already exists, the runtime correctly rejects the patch, but the rejection is surfaced as generic no-action recovery. Repeated Add File attempts can exhaust no-action recovery before the model receives a stable, specific correction to update the existing file, even though mandatory evidence correctly requires that file to be patched.
- Layer: root-cause
- Factor relation: sequential
- Depends on:
  - H-009
- Rationale:
  - The `20260628-031556-911` targeted run reached `implement_solution`, identified `generate_report.sh`, and attempted several patches.
  - The first multi-file patch and two follow-up patches were rejected as `apply_patch_existing_file_as_add:generate_report.sh`.
  - The final `finish_node` attempt was correctly rejected because `generate_report.sh (invalid_shebang, result-5)` remained uncovered by successful edits.
  - The turn then failed with generic no-action recovery exhausted (`3/3 recoveries spent`), so the model did not get a stable format-specific recovery loop.
- Falsifiable predictions:
  - If true before repair: action-contract rejection text contains `apply_patch_existing_file_as_add:generate_report.sh`, followed by `TaskSpaceNoActionRecoveryV1` attempts and eventual `turn.failed`.
  - If true after repair: the same rejection produces `TaskSpaceApplyPatchFormatRecoveryV1`, does not count as `TaskSpaceNoActionRecoveryV1`, and instructs `*** Update File: <path>` or `--- a/<path>` / `+++ b/<path>` for existing files.
  - If false: the failure would be caused by runtime mandatory-evidence gate overreach or by a missing parser path, not by rejection recovery semantics.
- Diagnostic evidence plan:
  - Prediction or clause under test: inspect latest targeted run and focused unit tests for recovery semantics.
  - Signal: `apply_patch_existing_file_as_add`, `TaskSpaceApplyPatchFormatRecoveryV1`, no-action marker absence, and state_commit compact record-fact conversion.
  - Capture method: targeted run artifact inspection plus `codex-core` focused tests.
  - Event name or marker:
    - `TaskSpaceApplyPatchFormatRecoveryV1`
    - `apply_patch_existing_file_as_add`
    - `TaskSpaceNoActionRecoveryV1`
  - Correlation keys:
    - `20260628-031556-911`
    - `generate_report.sh`
    - `result-5`
- Evidence gate: satisfied
- Related evidence:
  - E-020
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: rebuild `whale` and rerun the targeted P0 sample to verify the model no longer dies on repeated existing-file add-patch recovery.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-020: Existing-file add-patch and compact record_fact recovery are fixed in focused regression
- Related hypotheses:
  - H-010
- Direction: supports
- Type: fix-validation
- Source: `third_party\codex-cli\codex-rs\core\src\session\turn.rs`
- Prediction or plan link:
  - H-010 after-repair predictions.
- Matched signal:
  - Added `TaskSpaceApplyPatchFormatRecoveryV1` for `apply_patch_existing_file_as_add:<path>`.
  - The new recovery item explicitly instructs `*** Update File: <path>` and unified diff `--- a/<path>` / `+++ b/<path>` for existing files.
  - The recovery item does not contain `TaskSpaceNoActionRecoveryV1` and does not count against the generic no-action cap.
  - Action-contract `taskspace_control(action=record_fact, fact=...)` now compacts into `state_commit` with an observed fact source instead of reaching the legacy `record_fact` schema and failing on missing `claim_id`.
  - Focused tests passed:
    - `taskspace_action_contract_record_fact_compacts_to_state_commit`
    - `apply_patch_format_recovery_does_not_count_as_no_action_retry`
  - Aggregate TaskSpace regression passed: `cargo test -j1 -p codex-core taskspace --lib`, `106 passed`.
- Correlation keys:
  - `TaskSpaceApplyPatchFormatRecoveryV1`
  - `fact-source-action-contract-1`
  - `taskspace-state-commit-v1`
- Raw content:
  ```text
  test taskspace_action_contract_record_fact_compacts_to_state_commit ... ok
  test apply_patch_format_recovery_does_not_count_as_no_action_retry ... ok
  test result: ok. 106 passed; 0 failed; 1913 filtered out
  ```
- Interpretation: The runtime keeps strict mandatory-evidence and existing-file patch validation, but now gives the model a format-specific recovery path and prevents a compact fact note from dying at schema parse time.
- Time: 2026-06-28 04:00

## Hypothesis H-011: taskspace_control tool errors are not recorded as model-visible action-contract feedback
- Status: confirmed
- Parent: P-001
- Claim: In the action-contract synchronous tool path, a `taskspace_control` runtime error from mandatory-evidence validation is logged to stderr but not recorded as a `FunctionCallOutput` conversation item. The next request therefore lacks `TaskSpaceActionContractRecentToolOutputsV1` feedback and the model repeats `finish_node` until generic no-action recovery is exhausted.
- Layer: root-cause
- Factor relation: sequential
- Depends on:
  - H-010
- Rationale:
  - The `20260628-040040-607` targeted run no longer hit existing-file add-patch rejection; it applied one edit to `collect_data.sh`.
  - Four `finish_node` attempts were rejected by runtime mandatory evidence because `generate_report.sh (invalid_shebang, result-5)` remained uncovered.
  - The rejection text appears in `whale-exec.stderr.log`, but not as a model-visible function-call output in `whale-exec.jsonl`.
  - The model consequently saw generic `TaskSpaceNoActionRecoveryV1` and repeated `finish_node` instead of patching `generate_report.sh`.
- Falsifiable predictions:
  - If true before repair: stderr contains the high-signal mandatory-evidence rejection, but recent tool outputs do not inject the high-signal `Next action must be apply_patch` hint.
  - If true after repair: action-contract tool errors are recorded as `FunctionCallOutput` with `success=false`; recent-output summarization sees `high-signal inspected evidence` and injects patch-specific guidance.
  - If false: the model would have received the exact rejection and still chosen to repeat finish_node due to model instruction failure alone.
- Diagnostic evidence plan:
  - Prediction or clause under test: convert a synthetic mandatory-evidence `CodexErr` into action-contract tool output and feed it through recent-output summarization.
  - Signal: `FunctionCallOutput success=false`, `generate_report.sh`, `high-signal inspected evidence is still uncovered`, `Next action must be apply_patch`.
  - Capture method: focused unit test plus aggregate TaskSpace regression.
  - Event name or marker:
    - `TaskSpaceActionContractRecentToolOutputsV1`
    - `FunctionCallOutputPayload.success=false`
    - `high-signal inspected evidence`
  - Correlation keys:
    - `20260628-040040-607`
    - `taskspace-action-contract-12-taskspace_control`
    - `generate_report.sh`
- Evidence gate: satisfied
- Related evidence:
  - E-021
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: rebuild `whale` and rerun the targeted P0 sample to verify finish_node gate errors now steer the next action to patch the uncovered artifact.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-021: action-contract tool errors now become recent-output feedback
- Related hypotheses:
  - H-011
- Direction: supports
- Type: fix-validation
- Source: `third_party\codex-cli\codex-rs\core\src\session\turn.rs`
- Prediction or plan link:
  - H-011 after-repair predictions.
- Matched signal:
  - Added `response_input_for_taskspace_action_tool_error(...)`, which records action-contract tool errors as `FunctionCallOutput` or `CustomToolCallOutput` with `success=false`.
  - The synchronous action-contract tool execution path now records that failure output and preserves the tool error as `last_agent_message` for recovery.
  - Focused test passed: `action_contract_tool_error_is_recordable_recent_output_feedback`.
  - Aggregate TaskSpace regression passed: `cargo test -j1 -p codex-core taskspace --lib`, `106 passed`.
- Correlation keys:
  - `response_input_for_taskspace_action_tool_error`
  - `TaskSpaceActionContractRecentToolOutputsV1`
  - `FunctionCallOutputPayload.success=false`
- Raw content:
  ```text
  test action_contract_tool_error_is_recordable_recent_output_feedback ... ok
  test result: ok. 106 passed; 0 failed; 1914 filtered out
  ```
- Interpretation: mandatory-evidence `finish_node` failures should now remain visible to the next model request as compact dynamic feedback, preserving strict validation while enabling recovery.
- Time: 2026-06-28 04:26

## Hypothesis H-012: mandatory evidence does not become a hard edit target contract
- Status: confirmed
- Parent: P-001
- Claim: The runtime correctly detects uncovered mandatory evidence and prevents premature `finish_node`, but the action-contract state only says that high-signal evidence is uncovered. It does not make the uncovered artifact path a hard `apply_patch` target, so the model can patch a plausible but wrong filename such as `report_generation.sh` while the required `generate_report.sh` remains uncovered.
- Layer: root-cause
- Factor relation: sequential
- Depends on:
  - H-011
- Rationale:
  - The `20260628-042821-008` targeted run showed H-011 recovery working far enough for the model to attempt another edit after mandatory-evidence feedback.
  - The next edit targeted `report_generation.sh` even though the uncovered mandatory evidence was `generate_report.sh (invalid_shebang, result-13)`.
  - The wrong-target patch was rejected only by the generic "successful implementation edit already recorded" path, not by a precise mandatory-evidence target check.
  - This kept the node in an edit-required state but did not force the next patch to cover the actual evidence artifact.
- Falsifiable predictions:
  - If true before repair: action-contract state mentions uncovered mandatory evidence, but a wrong file target is not rejected with a mandatory-target-specific error.
  - If true after repair: the snapshot carries uncovered mandatory evidence artifact names, action-contract state prints required edit targets, and `apply_patch` for any wrong target is rejected with `apply_patch_missing_mandatory_evidence_targets:<artifact>`.
  - If false: the model's wrong-file edit would already have produced a target-specific mandatory-evidence rejection, and the remaining failure would be pure model noncompliance.
- Diagnostic evidence plan:
  - Prediction or clause under test: synthesize an implementation snapshot with uncovered `generate_report.sh` mandatory evidence and run the action-contract state plus `apply_patch` gate.
  - Signal: required target text in the state item; wrong-target rejection; right-target acceptance.
  - Capture method: focused unit test plus aggregate TaskSpace regression.
  - Event name or marker:
    - `Required edit targets from uncovered mandatory evidence`
    - `apply_patch_missing_mandatory_evidence_targets`
  - Correlation keys:
    - `20260628-042821-008`
    - `generate_report.sh`
    - `report_generation.sh`
- Evidence gate: satisfied
- Related evidence:
  - E-022
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: rebuild `whale` and rerun the targeted P0 sample to verify real action-contract traffic either patches `generate_report.sh` or receives the mandatory-target-specific rejection.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-022: uncovered mandatory evidence is now enforced as apply_patch target set
- Related hypotheses:
  - H-012
- Direction: supports
- Type: fix-validation
- Source: `third_party\codex-cli\codex-rs\core\src\session\turn.rs`; `third_party\codex-cli\codex-rs\core\src\action_map\runtime.rs`
- Prediction or plan link:
  - H-012 after-repair predictions.
- Matched signal:
  - Added `ActionMapProviderRequestBudgetSnapshot.current_node_uncovered_mandatory_evidence`.
  - Action-contract state now prints `Required edit targets from uncovered mandatory evidence` and names `generate_report.sh`.
  - Added `taskspace_apply_patch_missing_mandatory_targets(...)`, which rejects wrong-target patches with `apply_patch_missing_mandatory_evidence_targets:generate_report.sh`.
  - Focused test passed: `taskspace_apply_patch_must_cover_uncovered_mandatory_evidence_target`.
  - Aggregate TaskSpace regression passed: `cargo test -j1 -p codex-core taskspace --lib`, `107 passed`.
- Correlation keys:
  - `current_node_uncovered_mandatory_evidence`
  - `apply_patch_missing_mandatory_evidence_targets`
  - `generate_report.sh`
- Raw content:
  ```text
  test taskspace_apply_patch_must_cover_uncovered_mandatory_evidence_target ... ok
  test result: ok. 107 passed; 0 failed; 1914 filtered out
  ```
- Interpretation: The runtime no longer relies only on soft prompt pressure after high-signal inspection evidence is discovered. Once an implement node has uncovered mandatory evidence, the next `apply_patch` must cover the actual artifact path named by that evidence.
- Time: 2026-06-28 05:18

## Hypothesis H-013: local validator infrastructure failures are not lifted into action-contract next-step policy
- Status: confirmed
- Parent: P-001
- Claim: The runtime can tag local shell failures such as WSL `Bash/Service/CreateInstance/E_ACCESSDENIED` and PowerShell `InvalidEndOfLine` as validator infrastructure failures, but the action-contract recent-output layer treated them as ordinary failed test output. The model therefore kept issuing more Bash/PowerShell diagnostics inside a Windows validation node until the no-action recovery cap ended the turn.
- Layer: root-cause
- Factor relation: sequential
- Depends on:
  - H-012
- Rationale:
  - The `20260628-050004-157` targeted run patched only `generate_report.sh`, and the external public and hidden validators both exited 0.
  - The in-agent validation node still failed because `run_test` executed `bash run_pipeline.sh` through the Windows host shell, producing `Bash/Service/CreateInstance/E_ACCESSDENIED`.
  - The follow-up diagnostic command `bash -x run_pipeline.sh 2>&1 || echo EXIT_CODE=$?` was then parsed by PowerShell and failed with `The token '||' is not a valid statement separator` / `FullyQualifiedErrorId : InvalidEndOfLine`.
  - The turn ended with `TaskSpace stopped this turn because the model produced too many non-action assistant messages`, even though the code patch was externally valid.
- Falsifiable predictions:
  - If true before repair: recent action-contract feedback summarizes the raw shell failure but does not say that local validator infrastructure failed or require `state_commit`/blocked.
  - If true after repair: recent action-contract feedback detects local validator infra signatures and says not to rerun Bash/PowerShell diagnostics; accepted `state_commit` outputs that record blocker/result-validity sections remain visible despite legacy taskspace-control filtering.
  - If false: the model would already have received a specific local-validator-infra next-step policy and still ignored it.
- Diagnostic evidence plan:
  - Prediction or clause under test: synthesize recent tool outputs for `E_ACCESSDENIED`, PowerShell `InvalidEndOfLine`, and accepted `state_commit` blocker/result-validity output.
  - Signal: `TaskSpaceActionContractRecentToolOutputsV1` contains local-validator-infra guidance, `action=state_commit`, and no-rerun guidance; accepted state_commit output is not filtered out as legacy taskspace history.
  - Capture method: focused unit tests plus aggregate TaskSpace regression.
  - Event name or marker:
    - `TaskSpaceActionContractRecentToolOutputsV1`
    - `Bash/Service/CreateInstance/E_ACCESSDENIED`
    - `FullyQualifiedErrorId : InvalidEndOfLine`
    - `TaskSpace state_commit ... status=accepted`
  - Correlation keys:
    - `20260628-050004-157`
    - `taskspace-action-contract-13-run_test`
    - `taskspace-action-contract-14-run_test`
- Evidence gate: satisfied
- Related evidence:
  - E-023
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: rebuild `whale` and rerun the targeted P0 sample to verify the agent no longer ends as `turn.failed` after host-shell validator infra failure.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-023: local validator infra failures now become action-contract recovery guidance
- Related hypotheses:
  - H-013
- Direction: supports
- Type: fix-validation
- Source: `third_party\codex-cli\codex-rs\core\src\session\turn.rs`
- Prediction or plan link:
  - H-013 after-repair predictions.
- Matched signal:
  - Added detection for local validator infrastructure signatures in recent tool outputs: `Bash/Service/CreateInstance/E_ACCESSDENIED`, `E_ACCESSDENIED`, `InvalidEndOfLine`, and PowerShell statement-separator parse errors.
  - Recent feedback now instructs: do not rerun Bash/PowerShell diagnostics for the same local validator infra failure; next action must be `taskspace_control(action=state_commit)` marking the failed run invalid because local validator infrastructure failed, or blocked with exact infrastructure evidence.
  - Accepted `state_commit` outputs with `result_validities` or `blockers` are now allowed through the action-contract recent-output filter even though generic legacy taskspace-control history remains filtered.
  - Focused tests passed:
    - `action_contract_prompt_guides_state_commit_after_local_validator_infra_failure`
    - `action_contract_prompt_guides_block_after_recorded_local_validator_infra_failure`
    - existing runtime local-validator infra tests.
  - Aggregate TaskSpace regression passed: `cargo test -j1 -p codex-core taskspace --lib`, `107 passed`.
- Correlation keys:
  - `taskspace_output_mentions_local_validator_infra_failure`
  - `taskspace_output_mentions_local_validator_infra_state_commit`
  - `is_actionable_taskspace_gate_feedback_output`
- Raw content:
  ```text
  test action_contract_prompt_guides_state_commit_after_local_validator_infra_failure ... ok
  test action_contract_prompt_guides_block_after_recorded_local_validator_infra_failure ... ok
  test result: ok. 107 passed; 0 failed; 1916 filtered out
  ```
- Interpretation: TaskSpace now separates code-validation evidence from host-local validator availability in the context path that is actually sent back to the model, without weakening the typed smoke/regression finish contract.
- Time: 2026-06-28 05:47

## Hypothesis H-014: compact top-level state_commit action is rejected instead of normalized
- Status: confirmed
- Parent: P-001
- Claim: After local validator infra guidance tells the model to record a `state_commit`, the model may emit `{"action":"state_commit", ...}` as a compact top-level TaskSpace action instead of wrapping it as `{"action":"taskspace_control","args":{"action":"state_commit", ...}}`. The action-contract policy rejects that equivalent form in validation nodes, causing recovery to fail even though the intended operation is valid.
- Layer: root-cause
- Factor relation: sequential
- Depends on:
  - H-013
- Rationale:
  - The `20260628-054840-759` targeted run showed H-013 guidance working: after PowerShell `InvalidEndOfLine`, the model stopped Bash diagnostics and attempted to record local validation as invalid.
  - The emitted action was top-level `state_commit` with `args.action=state_commit`.
  - The policy rejected it as `node_policy_violation:smoke_test:state_commit`, then the no-action recovery cap ended the turn.
- Falsifiable predictions:
  - If true before repair: top-level `state_commit` in a smoke_test node is rejected by action policy.
  - If true after repair: top-level `state_commit` is allowed anywhere `taskspace_control` is allowed and normalizes to a `taskspace_control` tool call with `schema_version=taskspace-state-commit-v1`.
  - If false: the same emitted action would already have reached the taskspace_control handler.
- Diagnostic evidence plan:
  - Prediction or clause under test: parse a top-level `state_commit` TaskSpaceActionV1 and convert it with a smoke_test provider snapshot.
  - Signal: produced tool name is `taskspace_control`; arguments contain `action=state_commit`, default schema, active node id, and normalized blocker object.
  - Capture method: focused unit test plus aggregate TaskSpace regression.
  - Event name or marker:
    - `state_commit`
    - `taskspace_control`
    - `taskspace-state-commit-v1`
  - Correlation keys:
    - `20260628-054840-759`
    - `node_policy_violation:smoke_test:state_commit`
    - `taskspace-action-contract-15-state_commit`
- Evidence gate: satisfied
- Related evidence:
  - E-024
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: rebuild `whale` and rerun the targeted P0 sample to verify the compact `state_commit` no longer fails the validation recovery path.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-024: top-level state_commit now normalizes to taskspace_control
- Related hypotheses:
  - H-014
- Direction: supports
- Type: fix-validation
- Source: `third_party\codex-cli\codex-rs\core\src\session\turn.rs`
- Prediction or plan link:
  - H-014 after-repair predictions.
- Matched signal:
  - `taskspace_action_allowed_for_node(...)` now permits top-level `state_commit` in nodes where `taskspace_control` is valid.
  - `taskspace_action_to_tool_call(...)` maps top-level `state_commit` to the `taskspace_control` tool and inserts `args.action=state_commit` when needed.
  - Focused test passed: `taskspace_action_contract_top_level_state_commit_normalizes_to_control_tool`.
  - Aggregate TaskSpace regression passed: `cargo test -j1 -p codex-core taskspace --lib`, `108 passed`.
- Correlation keys:
  - `taskspace_action_contract_top_level_state_commit_normalizes_to_control_tool`
  - `taskspace_control`
  - `taskspace-state-commit-v1`
- Raw content:
  ```text
  test taskspace_action_contract_top_level_state_commit_normalizes_to_control_tool ... ok
  test result: ok. 108 passed; 0 failed; 1916 filtered out
  ```
- Interpretation: The action contract now accepts the compact cognitive operation form that DeepSeek naturally emits after local-validator-infra recovery guidance, while keeping the underlying state mutation routed through the existing taskspace_control handler.
- Time: 2026-06-28 06:17

## Hypothesis H-015: UTF-16/garbled host-shell errors evade local validator infra detection
- Status: confirmed
- Parent: P-001
- Claim: WSL/Bash host failures on Windows can appear in tool output as UTF-16-like text with embedded NUL/control bytes, for example `B\0a\0s\0h\0/.../E\0_\0A...`. Plain substring matching for `Bash/Service/CreateInstance/E_ACCESSDENIED` misses this form, so the action-contract recent-output layer fails to classify the result as local validator infrastructure failure.
- Layer: root-cause
- Factor relation: sequential
- Depends on:
  - H-013
- Rationale:
  - The `20260628-061941-127` targeted run still repeated validation commands after a garbled WSL `E_ACCESSDENIED` output.
  - The output contained the same semantic marker, but interleaved with NUL/control characters and therefore did not match the plain text detector.
  - The next request lacked the local-validator-infra progress hint and the model tried `chmod +x *.sh && ./run_pipeline.sh`, which then failed under PowerShell with `InvalidEndOfLine`.
- Falsifiable predictions:
  - If true before repair: a recent tool output containing `B\0a\0s\0h\0/.../E\0_\0A...` does not produce local-validator-infra guidance.
  - If true after repair: compact signal matching removes control characters/separators and detects both `bashservicecreateinstancee_accessdenied` and `invalidendofline`.
  - If false: the garbled output would already trigger the same `state_commit`/blocked guidance as plain `E_ACCESSDENIED`.
- Diagnostic evidence plan:
  - Prediction or clause under test: synthesize a function-call output with embedded NULs in the Bash E_ACCESSDENIED marker and feed it through action-contract recent-output construction.
  - Signal: `TaskSpaceActionContractRecentToolOutputsV1` contains local-validator-infra guidance, no-rerun guidance, and `action=state_commit`.
  - Capture method: focused unit test plus aggregate TaskSpace regression.
  - Event name or marker:
    - `taskspace_compact_ascii_signal`
    - `bashservicecreateinstancee_accessdenied`
    - `TaskSpaceActionContractRecentToolOutputsV1`
  - Correlation keys:
    - `20260628-061941-127`
    - `Bash/Service/CreateInstance/E_ACCESSDENIED`
    - `InvalidEndOfLine`
- Evidence gate: satisfied
- Related evidence:
  - E-025
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: rebuild `whale` after the next context-retention repair and rerun the targeted P0 sample.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-025: local validator infra detection now handles garbled Windows Bash output
- Related hypotheses:
  - H-015
- Direction: supports
- Type: fix-validation
- Source: `third_party\codex-cli\codex-rs\core\src\session\turn.rs`
- Prediction or plan link:
  - H-015 after-repair predictions.
- Matched signal:
  - Added `taskspace_compact_ascii_signal(...)`, which strips non-ASCII signal characters and lowercases the remaining marker.
  - Local-validator-infra detection now matches `bashservicecreateinstanceeaccessdenied`, `bashservicecreateinstancee_accessdenied`, `eaccessdenied`, `e_accessdenied`, and `invalidendofline`.
  - Focused test passed: `action_contract_prompt_detects_utf16_garbled_local_validator_infra_failure`.
  - Focused local-validator group passed: `5 passed`.
  - Aggregate TaskSpace regression passed: `cargo test -j1 -p codex-core taskspace --lib`, `108 passed`.
- Correlation keys:
  - `taskspace_compact_ascii_signal`
  - `action_contract_prompt_detects_utf16_garbled_local_validator_infra_failure`
  - `local_validator_infra_failure`
- Raw content:
  ```text
  test action_contract_prompt_detects_utf16_garbled_local_validator_infra_failure ... ok
  test result: ok. 5 passed; 0 failed; 2020 filtered out
  test result: ok. 108 passed; 0 failed; 1917 filtered out
  ```
- Interpretation: The action-contract feedback path now recognizes the exact Windows-host failure shape observed in targeted terminal-bench runs, instead of relying on clean UTF-8 substrings.
- Time: 2026-06-28 06:36

## Hypothesis H-016: terminal TaskSpace actions are still rejected by the normal final-response gate
- Status: confirmed
- Parent: P-001
- Claim: After the runtime rewrites closed blocked validation evidence into a terminal blocked response, `session/turn.rs` still runs the ordinary final-response gate for the same request. That gate treats the terminal blocked candidate as an unactionable final candidate, inserts no-action recovery, and can still end the turn as failed even though the TaskSpace graph is already closed.
- Layer: root-cause
- Factor relation: sequential
- Depends on:
  - H-013
  - H-015
- Rationale:
  - The `20260628-103125` targeted diagnostic showed `open_leaf_nodes=0` and a blocked validation node, but the run still failed.
  - The log contained `blocked_by_taskspace_action_contract: TaskSpace validation is blocked by local validator infrastructure evidence already recorded on the closed validation node.`
  - The same log then recorded `TaskSpaceProviderResponseActionabilityV1 actionability=final_candidate recovery_action=none`, meaning the normal final gate handled the terminal blocked candidate as if no typed terminal action had occurred.
- Falsifiable predictions:
  - If true before repair: a request that already applied a terminal TaskSpace action can still be rejected by the normal final-response gate and consume no-action recovery.
  - If true after repair: once a terminal TaskSpace action is observed in a request, the normal final-response recording/rejection gate is skipped for that request, and the targeted diagnostic no longer emits `turn.failed`.
  - If false: the terminal blocked candidate would not pass through the ordinary final gate, or `turn.failed` would be caused by another later action rejection.
- Diagnostic evidence plan:
  - Prediction or clause under test: rebuild a current binary with request-level terminal action tracking and rerun the same targeted diagnostic sample.
  - Signal: no `turn.failed`, `exec_exit_code=0`, `business_success=true`, `public_validation_exit_code=0`, `hidden_oracle_exit_code=0`, and `open_leaf_nodes=0`.
  - Capture method: focused unit regression plus real targeted diagnostic run.
  - Event name or marker:
    - `taskspace_terminal_action_observed_in_request`
    - `blocked_by_taskspace_action_contract`
    - `turn.failed`
  - Correlation keys:
    - `20260628-103125`
    - `20260628-110353`
    - `TaskSpaceProviderResponseActionabilityV1`
- Evidence gate: satisfied
- Related evidence:
  - E-026
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: handle residual benchmark engineering-clean taxonomy for local validator infra blockers, then rerun formal non-agent gates when host resources allow.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-026: terminal action gate fix removes targeted diagnostic turn.failed
- Related hypotheses:
  - H-016
- Direction: supports
- Type: fix-validation
- Source: `target\phase-r3-targeted-diagnostic-20260628-110353\runs\terminal_bench__processing-pipeline\20260628-110410-426`
- Prediction or plan link:
  - H-016 after-repair predictions.
- Matched signal:
  - Added request-level terminal action tracking in `session/turn.rs`.
  - Aggregate TaskSpace regression passed: `cargo test -j1 -p codex-core taskspace --lib`, `111 passed`.
  - Low-memory dev-small whale build passed and produced `target\phase-r3-current-cargo-target\dev-small\whale.exe` with SHA256 `2B4293F1E60F2FA5484724F48F8FE3EB357234C735F981DF71045E048739C9DA`.
  - Targeted diagnostic run `20260628-110410-426` produced `exec_exit_code=0`, `business_success=true`, `public_validation_exit_code=0`, `hidden_oracle_exit_code=0`, and `open_leaf_nodes=0`.
  - Targeted log search found no `turn.failed`.
  - Provider cache proof stayed healthy: `request_2_plus_hit_rate=0.984414`, `cache_usage_missing_count=0`, `trace_coverage=1`.
- Correlation keys:
  - `taskspace_terminal_action_observed_in_request`
  - `target\phase-r3-targeted-diagnostic-20260628-110353`
  - `request_2_plus_hit_rate=0.984414`
- Raw content:
  ```text
  exec_exit_code                    : 0
  business_success                  : True
  public_validation_exit_code       : 0
  hidden_oracle_exit_code           : 0
  open_leaf_nodes                   : 0
  Select-String turn.failed         : False
  request_2_plus_hit_rate           : 0.984414
  ```
- Interpretation: The targeted diagnostic failure has been reduced from a runtime turn failure to a successful TaskSpace run with an explicit blocked local-validator-infra lifecycle edge. The remaining `active_sentinel_warning:validator_failure` is a benchmark taxonomy/engineering-clean residual, not the original graph or terminalization failure.
- Time: 2026-06-28 11:22

## Hypothesis H-017: failed tool-result preview drops stable local-infra error signals
- Status: confirmed
- Parent: P-001
- Claim: When a direct tool call returns `FunctionCallError::RespondToModel`, `tools/parallel.rs` records only `Tool call failed before producing a result.` into ActionMap. This protects raw error text, but also drops stable local validator infrastructure signatures such as `Bash/Service/CreateInstance/E_ACCESSDENIED`, causing runtime sentinel classification to raise `validator_failure` instead of `validator_infra_failure`.
- Layer: root-cause
- Factor relation: sequential
- Depends on:
  - H-016
- Rationale:
  - The `20260628-110353` targeted diagnostic had `active_sentinel_warning_count=1`.
  - The active sentinel referenced `node-3 / result-33 / trace-448`.
  - `result-33` had `actionClass=test`, `success=false`, and body `Tool call failed before producing a result.`
  - The corresponding `whale-exec.jsonl` command output for `bash run_pipeline.sh` contained UTF-16/NUL-shaped `Bash/Service/CreateInstance/E_ACCESSDENIED`.
  - Runtime already had tests proving a result body with the canonical infra signature is classified as `validator_infra_failure`, not `validator_failure`.
- Falsifiable predictions:
  - If true before repair: a `RespondToModel` error containing NUL-separated `Bash/Service/CreateInstance/E_ACCESSDENIED` produces the generic preview and loses the infra signature.
  - If true after repair: the same error produces a canonical, non-raw preview containing `local_validator_infra_failure: Bash/Service/CreateInstance/E_ACCESSDENIED`, and the targeted diagnostic has no active validator_failure sentinel.
  - If false: preserving the canonical infra signal would not change `active_sentinel_warning_count`.
- Diagnostic evidence plan:
  - Prediction or clause under test: add a focused unit test for `action_map_tool_error_preview(...)`, run the existing runtime sentinel local-infra test, and rerun the same targeted diagnostic.
  - Signal: focused tests pass; targeted metrics show `active_sentinel_warning_count=0`, `outcome_taskspace=solved`, and no `turn.failed`.
  - Capture method: unit tests plus real targeted diagnostic artifacts.
  - Event name or marker:
    - `local_validator_infra_failure`
    - `Bash/Service/CreateInstance/E_ACCESSDENIED`
    - `active_sentinel_warning_count`
  - Correlation keys:
    - `result-33`
    - `trace-448`
    - `20260628-110353`
    - `20260628-114800`
- Evidence gate: satisfied
- Related evidence:
  - E-027
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: run formal current-HEAD non-agent gates, then decide whether formal E3 start gate can proceed after explicit user approval marker.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-027: safe local-infra preview removes active validator_failure sentinel
- Related hypotheses:
  - H-017
- Direction: supports
- Type: fix-validation
- Source: `target\phase-r3-targeted-diagnostic-20260628-114800\runs\terminal_bench__processing-pipeline\20260628-114818-716`
- Prediction or plan link:
  - H-017 after-repair predictions.
- Matched signal:
  - `tools/parallel.rs` now keeps `FunctionCallError::RespondToModel` generic by default, but emits canonical summaries for known local validator infra signatures.
  - Focused test passed: `action_map_error_preview_keeps_safe_local_validator_infra_signal`.
  - Existing runtime sentinel test passed: `local_validator_infra_failure_does_not_raise_validator_failure`.
  - Aggregate TaskSpace regression passed: `cargo test -j1 -p codex-core taskspace --lib`, `111 passed`.
  - New dev-small whale binary built with SHA256 `9E5B08528D6B11C5BAA742374CFBA193FFDE5F4EAB632384EE447AB04A777CEA`.
  - Targeted diagnostic rerun produced `outcome_taskspace=solved`, `active_sentinel_warning_count=0`, `exec_exit_code=0`, `business_success=true`, `public_validation_exit_code=0`, `hidden_oracle_exit_code=0`, and `open_leaf_nodes=0`.
  - Provider cache remained healthy: `request_2_plus_hit_rate=0.982693`, `cache_usage_missing_count=0`, `trace_coverage=1`.
  - Request/tool count improved on this sample versus the previous targeted run: provider requests `34 -> 16`, tool calls `30 -> 10`.
- Correlation keys:
  - `action_map_error_preview_keeps_safe_local_validator_infra_signal`
  - `target\phase-r3-targeted-diagnostic-20260628-114800`
  - `active_sentinel_warning_count=0`
- Raw content:
  ```text
  outcome_taskspace                 : solved
  exec_exit_code                    : 0
  business_success                  : True
  public_validation_exit_code       : 0
  hidden_oracle_exit_code           : 0
  tool_call_count                   : 10
  rollout_trace_model_request_count : 17
  active_sentinel_warning_count     : 0
  open_leaf_nodes                   : 0
  request_2_plus_hit_rate           : 0.982693
  ```
- Interpretation: The remaining R3 targeted diagnostic runtime/observability blocker is closed. The run is still not formal E3 evidence because its only remaining engineering reasons are external validator fidelity / E3 eligibility flags.
- Time: 2026-06-28 12:05

## Hypothesis H-018: suite child runner repeats string-array parameter names
- Status: confirmed
- Parent: P-001
- Claim: `run-taskspace-e3-suite.ps1` and `run-taskspace-external-benchmark.ps1` emitted one `-SampleNames <value>` pair per sample. PowerShell treats repeated binding of the same `string[]` named parameter as an error, so formal multi-sample E3 and formal plan-only calibration fail before scheduling useful work.
- Layer: harness-contract
- Factor relation: sequential
- Depends on:
  - H-017
- Rationale:
  - Formal `terminal-bench_E3-P0_3_5` task list contains three sample names.
  - Formal plan-only calibration with v0.0.5 markers failed before model execution with `Cannot bind parameter because parameter 'SampleNames' is specified more than once`.
  - The failing command path is generated by suite/external wrappers, not by user shell syntax.
- Falsifiable predictions:
  - If true before repair: any suite child with more than one sample name receives repeated `-SampleNames` arguments and fails during parameter binding.
  - If true after repair: wrappers emit one `-SampleNames` token followed by all non-empty names, and start-gate self-tests still pass.
- Diagnostic evidence plan:
  - Prediction or clause under test: run `test-e3-start-gate.ps1`, then rerun formal plan-only calibration with marker paths.
  - Signal: fixture test passes; plan-only no longer fails with duplicate `SampleNames`.
  - Capture method: PowerShell self-test output and formal plan-only artifacts.
  - Event name or marker:
    - `SampleNames`
    - `ParameterAlreadyBound`
- Evidence gate: partially satisfied
- Related evidence:
  - E-028
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: commit/push the runner contract fix, recompute formal profile hash, and rerun formal non-agent gates because runner script SHA is part of profile identity.
- Blocker:
  - formal E3 identity must be regenerated after the script-hash-changing fix
- Close reason:
  - not closed

## Evidence E-028: SampleNames array binding fix passes start-gate self-test
- Related hypotheses:
  - H-018
- Direction: supports
- Type: fix-validation
- Source: `scripts\taskspace-benchmark\run-taskspace-e3-suite.ps1`, `scripts\taskspace-benchmark\run-taskspace-external-benchmark.ps1`
- Prediction or plan link:
  - H-018 after-repair predictions.
- Matched signal:
  - Both wrappers now emit `-SampleNames` once, followed by all non-empty sample names.
  - Because `powershell.exe -File` binds `string[]` parameters as one string across process boundaries, wrappers now pass a CSV value and entry scripts normalize comma-separated names back to arrays.
  - `run-taskspace-external-benchmark.ps1` now declares and forwards `SuiteReceiptPath` / `SuiteReceiptSha256`, matching the suite runner arguments and the downstream benchmark runner provenance parameters.
  - External materialization now uses a short `target\external-materialized\<hash>` root, plus a run-local `materialized-scenarios-pointer.json`, avoiding repeated Windows `MAX_PATH` failures in formal suite roots.
  - External benchmark common copy/hash helpers are long-path aware for fixture and validator-source file trees.
  - `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1` passed.
  - `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-external-wrapper-harness.ps1` passed.
  - `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-terminal-bench-adapter-harness.ps1` passed.
  - Formal `terminal-bench_E3-P0_3_5` plan-only suite passed materialization/preflight for all three samples.
  - The fix changes `run-taskspace-e3-suite.ps1` SHA, so formal `profile_hash` and v0.0.5 non-agent marker evidence must be regenerated on the new HEAD.
- Correlation keys:
  - `SampleNames`
  - `test-e3-start-gate.ps1`
- Raw content:
  ```text
  E3 start gate self-test: PASS
  RunRoot: D:\whalecode-alpha\target\e3-start-gate-selftest\20260628-171620-334

  TaskSpace external wrapper self-test: PASS
  RunRoot: D:\whalecode-alpha\target\external-wrapper-selftest\20260628-172201-777

  Terminal-Bench adapter self-test: PASS
  RunRoot: D:\whalecode-alpha\target\terminal-bench-adapter-selftest\20260628-175207-395

  formal plan-only:
  SuiteRoot: D:\whalecode-alpha\target\phase-r3-formal-e3-20260628-170557\plan-after-short-materialization-root\suite-20260628-175509
  sample_set_id = terminal-bench_E3-P0_3_5
  task_list_hash = de1c223db57ea05e0c87839bb9d13677eb4faa84d3a3830df2b36d7e0ecac5a2
  profile_hash = c04582a682c487647ffea44b9f6a2010a23619c0724a1d8a1a09c538b01f0bd4
  status = completed
  suite_score_valid = true
  score_valid_child_runs = 3
  score_invalid_child_runs = 0
  ```
- Interpretation: The formal E3 harness no longer has a known multi-sample argument-binding blocker, but the formal start gate remains blocked until regenerated identity-bound markers and calibration evidence pass.
- Time: 2026-06-28 17:22

## Hypothesis H-019: formal start gate creates a circular dependency on calibration evidence
- Status: confirmed
- Parent: P-001
- Claim: The formal E3 start gate required calibration evidence before allowing the first identity-bound formal E3 run, while that calibration evidence can only be produced by the formal suite or an equivalent identity-bound suite run. This blocks evidence generation without proving a TaskSpace runtime defect.
- Layer: release-gate
- Factor relation: sequential
- Depends on:
  - H-018
- Rationale:
  - Current-head non-agent gates and user approval markers can be generated without running model work.
  - Serial calibration timing for `terminal-bench_E3-P0_3_5` requires the suite to execute enough real pairs to produce timing evidence.
  - The previous start gate treated missing calibration as a blocker for `full_e3_allowed`, so the run that would generate the missing evidence could not start.
- Falsifiable predictions:
  - If true before repair: with valid v0.0.5 markers and no calibration evidence, start gate reports `full_e3_allowed=false` and routes to `serial_calibration`.
  - If true after repair: with valid v0.0.5 markers and `-AllowSkippedCalibrationGate`, start gate reports `full_e3_allowed=true`, `calibration_gate_skipped_allowed=true`, and `speed_claim_allowed=false`.
  - If false: release decision would allow speed/cost claims without calibration, or start gate would still block formal E3 after markers pass.
- Diagnostic evidence plan:
  - Prediction or clause under test: add a start-gate fixture that passes current v0.0.5 marker inputs, skips calibration explicitly, and asserts full E3 is allowed while speed claims remain blocked.
  - Signal: start-gate self-test and release decision self-test.
  - Capture method: PowerShell fixture tests.
  - Event name or marker:
    - `calibration_gate_skipped_allowed`
    - `speed_claim_allowed`
    - `full_e3_allowed`
- Evidence gate: satisfied
- Related evidence:
  - E-029
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: commit/push the start-gate semantic fix, recompute formal identity, rerun current-HEAD non-agent gates and markers, then start formal E3 with calibration skipped only for evidence generation.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-029: start gate can generate formal calibration evidence without authorizing speed claims
- Related hypotheses:
  - H-019
- Direction: supports
- Type: fix-validation
- Source: `scripts\taskspace-benchmark\lib\e3-start-gate.ps1`, `scripts\taskspace-benchmark\run-taskspace-e3-suite.ps1`, `scripts\taskspace-benchmark\test-e3-start-gate.ps1`
- Prediction or plan link:
  - H-019 after-repair predictions.
- Matched signal:
  - `run-taskspace-e3-suite.ps1` now forwards `-AllowSkippedCalibrationGate` to the start gate.
  - `New-TaskspaceE3GateDecision` treats `calibration_gate=skipped_allowed` plus passing v0.0.5 markers as enough to set `full_e3_allowed=true`.
  - The same decision keeps `speed_claim_allowed=false` and `calibration_gate_passed=false`.
  - `test-e3-start-gate.ps1` added `gate-skipped-calibration-with-markers`.
  - `test-release-decision.ps1` still passes, so final release-like claims remain calibration-gated.
- Correlation keys:
  - `AllowSkippedCalibrationGate`
  - `calibration_gate_skipped_allowed`
  - `speed_claim_allowed=false`
- Raw content:
  ```text
  git diff --check = PASS
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1
  E3 start gate self-test: PASS
  RunRoot: D:\whalecode-alpha\target\e3-start-gate-selftest\20260628-180849-953

  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1
  Release decision self-test: PASS
  RunRoot: D:\whalecode-alpha\target\release-decision-selftest\run-20260628-180637-101
  ```
- Interpretation: The gate no longer blocks the first formal evidence-generating run on evidence that the run itself must produce, while preserving the stricter rule for speed/cost claims and release decisions.
- Time: 2026-06-28 18:12

## Hypothesis H-020: suite runner emits empty optional marker args across process boundary
- Status: confirmed
- Parent: P-001
- Claim: `run-taskspace-e3-suite.ps1` unconditionally emitted optional marker hash/path parameters into child PowerShell invocations. When those values were empty, Windows PowerShell dropped or failed to bind the empty argument, so the downstream wrapper saw a named parameter without a value.
- Layer: harness-contract
- Factor relation: sequential
- Depends on:
  - H-019
- Rationale:
  - Formal plan-only does not require v0.0.5 marker hash/path values.
  - The child runner args included `-ApprovalMarkerSha256 ""`.
  - The downstream error was `Missing an argument for parameter 'ApprovalMarkerSha256'.`
  - Required identity fields were non-empty and present in the same command, so the failure is limited to optional empty strings.
- Falsifiable predictions:
  - If true before repair: formal plan-only without marker paths fails before sample preflight with a missing optional marker argument.
  - If true after repair: optional marker args are omitted when empty, required identity/provenance args remain present, and formal plan-only completes all three sample preflights.
- Diagnostic evidence plan:
  - Prediction or clause under test: guard optional child args by non-empty value, then rerun formal plan-only for `terminal-bench_E3-P0_3_5`.
  - Signal: suite health status and child resume commands.
  - Capture method: formal plan-only suite artifacts.
  - Event name or marker:
    - `ApprovalMarkerSha256`
    - `V005UserApprovalMarkerPath`
    - `suite_score_valid`
- Evidence gate: satisfied
- Related evidence:
  - E-030
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: commit/push the optional-args repair, recompute final current-HEAD formal identity, then rerun non-agent gates and markers.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-030: formal plan-only completes after optional empty args are omitted
- Related hypotheses:
  - H-020
- Direction: supports
- Type: fix-validation
- Source: `target\phase-r3-formal-e3-20260628-170557\plan-current-head-after-optional-arg-fix\suite-20260628-181726`
- Prediction or plan link:
  - H-020 after-repair predictions.
- Matched signal:
  - `run-taskspace-e3-suite.ps1` now emits optional marker hash/path parameters only when non-empty.
  - Formal plan-only completed all three samples.
  - Child resume commands preserve required identity fields: `TaskListHash`, `ProfileHash`, `SourceVersion`, `SampleSetId`, `SuiteReceiptPath`, `SuiteReceiptSha256`.
  - Empty marker hash/path values are not present in the child resume commands.
- Correlation keys:
  - `terminal-bench_E3-P0_3_5`
  - `profile_hash=2aebff6baaf60a71367f9c999e93a1fd01a140257d48e4cee8378fccb0cbc013`
- Raw content:
  ```text
  SuiteRoot: D:\whalecode-alpha\target\phase-r3-formal-e3-20260628-170557\plan-current-head-after-optional-arg-fix\suite-20260628-181726
  status = completed
  suite_score_valid = true
  score_valid_child_runs = 3
  score_invalid_child_runs = 0
  task_list_hash = de1c223db57ea05e0c87839bb9d13677eb4faa84d3a3830df2b36d7e0ecac5a2
  profile_hash = 2aebff6baaf60a71367f9c999e93a1fd01a140257d48e4cee8378fccb0cbc013
  ```
- Interpretation: The formal suite runner no longer depends on fragile empty-string argument binding for optional marker values.
- Time: 2026-06-28 18:18

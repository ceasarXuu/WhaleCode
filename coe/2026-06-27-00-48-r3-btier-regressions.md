# Problem P-001: R3 B-tier smoke exposes missing timing attribution and open TaskSpace leaf
- Status: open
- Created: 2026-06-27 00:48
- Updated: 2026-06-30 04:55
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

## Hypothesis H-021: deep formal suite roots exceed Windows Git loose-object path budget
- Status: confirmed
- Parent: P-001
- Claim: Full formal E3 runs under the long `target\phase-r3-formal-e3-20260628-170557\formal-run-*` roots failed during workspace materialization because Git loose-object temporary paths exceeded the effective Windows path budget. Git left suffixed temporary object files, while the index referenced the unsuffixed object id.
- Layer: harness-environment
- Factor relation: sequential
- Depends on:
  - H-020
- Rationale:
  - Two consecutive full formal attempts failed before model execution with `invalid object 100644 83544132e76f2c3e3f5cee636e8e0ca0cabb5faf for 'Dockerfile'`.
  - The failing loose-object temporary path length was 281 characters.
  - A short isolated repro path of 160 characters passed `git init/add/commit/fsck`.
  - Re-running the same formal identity under short root `target\e3f-final` passed workspace materialization and entered real agent execution.
- Falsifiable predictions:
  - If true: deep roots fail before child agent execution with Git object materialization errors, while the same task list and source under a short run root passes materialization.
  - If false: short root would fail with the same Git object error, or deep root failure would show task content corruption independent of path length.
- Diagnostic evidence plan:
  - Prediction or clause under test: compare deep-root failures with short-root formal run progress.
  - Signal: `pair-abort.json`, failed object path length, and short-root suite status.
  - Capture method: formal E3 artifacts and path-length probe.
  - Event name or marker:
    - `harness_materialization_failure/workspace_materialization_failed`
    - `invalid object 100644 ... Dockerfile`
- Evidence gate: satisfied
- Related evidence:
  - E-031
- Conclusion: confirmed
- Repair design readiness: implemented by operational run-root selection; a future hardening item is to make the suite choose a short formal run root automatically on Windows.
- Next step: keep formal E3 run roots short (`target\e3f-*`) until the runner owns this path-budget policy.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-031: short formal run root clears Git materialization and reaches real agent execution
- Related hypotheses:
  - H-021
- Direction: supports
- Type: diagnostic
- Source: `target\e3f-final\suite-20260628-184253`
- Prediction or plan link:
  - H-021 short-root prediction.
- Matched signal:
  - Deep formal runs failed with `harness_materialization_failure/workspace_materialization_failed`.
  - Failed Git loose object temp path length was 281 characters.
  - Short-root formal run under `target\e3f-final` completed `processing-pipeline` with 5 attempted and 5 completed pairs, `run_validity=valid`, and `exit_code=0`.
- Correlation keys:
  - `terminal-bench_E3-P0_3_5`
  - `task_list_hash=de1c223db57ea05e0c87839bb9d13677eb4faa84d3a3830df2b36d7e0ecac5a2`
  - `profile_hash=2aebff6baaf60a71367f9c999e93a1fd01a140257d48e4cee8378fccb0cbc013`
- Raw content:
  ```text
  deep failure: invalid object 100644 83544132e76f2c3e3f5cee636e8e0ca0cabb5faf for 'Dockerfile'
  failed loose object temp path length: 281
  short root: target\e3f-final\suite-20260628-184253
  processing-pipeline sample-status:
    phase = audit_required
    run_validity = valid
    attempted_pairs = 5
    completed_pairs = 5
    exit_code = 0
  ```
- Interpretation: The Git materialization blocker is path-budget related, not scenario content corruption or agent execution failure.
- Time: 2026-06-28 20:11

## Hypothesis H-022: suite score enforcement conflates pending audit with invalid harness
- Status: confirmed
- Parent: P-001
- Claim: `run-taskspace-e3-suite.ps1` treated any aggregate with `score_valid=false` as `invalid_harness`. This incorrectly converted `score_block_reason=audit_required` into `harness_materialization_failure/score_invalid`, skipped the remaining formal samples, and hid a valid evidence-generation state behind a false harness failure.
- Layer: harness-state-machine
- Factor relation: sequential
- Depends on:
  - H-021
- Rationale:
  - `aggregate-report.ps1` defines missing E3 human review as `run_validity=valid`, `score_ready=false`, `score_block_reason=audit_required`, and blank `score_invalid_reason`.
  - `run-taskspace-e3-suite.ps1` ignored `score_block_reason` and upgraded every `score_valid=false` aggregate to `invalid_harness`.
  - The real short-root formal run had 5 completed engineering-clean pairs, but suite health recorded `harness_materialization_failure/score_invalid` and skipped the remaining 2 samples.
- Falsifiable predictions:
  - If true before repair: a child run with `run_validity=valid` and aggregate `score_block_reason=audit_required` becomes suite `invalid_harness`.
  - If true after repair: the same child remains valid, suite status becomes `audit_required`, no remaining samples are skipped for this reason, and `suite_score_pending_audit` events identify the handoff.
  - If false: pending audit would already be preserved by the suite, or aggregate would label it as engineering-unclean.
- Diagnostic evidence plan:
  - Prediction or clause under test: add a suite fixture whose child writes a valid `audit_required` sample status and aggregate, then run suite scoring mode.
  - Signal: suite health fields and events.
  - Capture method: `test-e3-harness-guardrails.ps1` and `test-e3-score-validity.ps1`.
  - Event name or marker:
    - `suite_score_pending_audit`
    - `score_pending_audit_child_runs`
    - `suite_score_ready=false`
- Evidence gate: satisfied
- Related evidence:
  - E-032
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: commit/push the state-machine fix, recompute formal identity, regenerate current-HEAD markers/gates, and rerun formal E3 under a short root.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-032: pending audit is now preserved as audit_required instead of invalid_harness
- Related hypotheses:
  - H-022
- Direction: supports
- Type: fix-validation
- Source: `scripts\taskspace-benchmark\lib\suite-status.ps1`, `scripts\taskspace-benchmark\run-taskspace-e3-suite.ps1`, `scripts\taskspace-benchmark\test-e3-score-validity.ps1`, `scripts\taskspace-benchmark\test-e3-harness-guardrails.ps1`
- Prediction or plan link:
  - H-022 after-repair predictions.
- Matched signal:
  - Suite score summary now records `score_pending_audit_child_runs` and keeps `suite_score_ready=false` for pending audit.
  - Suite runner emits `suite_score_pending_audit` and does not call `New-TaskspaceSuiteChildFailureStatus` when `score_block_reason=audit_required` and `score_invalid_reason` is blank.
  - Pending-audit fixture exits 0 with suite `status=audit_required`, `score_invalid_child_runs=0`, `remaining_samples_skipped=0`, and `score_pending_audit_child_runs=2`.
  - Existing invalid-harness fixtures still fail and skip as before.
- Correlation keys:
  - `score_block_reason=audit_required`
  - `suite_score_pending_audit`
- Raw content:
  ```text
  git diff --check = PASS
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-score-validity.ps1
  E3 score-validity self-test: PASS
  RunRoot: D:\whalecode-alpha\target\e3-score-validity-selftest\20260628-200746-621

  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-harness-guardrails.ps1
  E3 harness guardrails self-test: PASS
  RunRoot: D:\whalecode-alpha\target\e3-guardrails-selftest\20260628-200746-801

  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1
  E3 start gate self-test: PASS
  RunRoot: D:\whalecode-alpha\target\e3-start-gate-selftest\20260628-200846-069

  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1
  Release decision self-test: PASS
  RunRoot: D:\whalecode-alpha\target\release-decision-selftest\run-20260628-200846-008
  ```
- Interpretation: Pending human audit is now an explicit formal handoff state, while actual score/harness invalid states remain blocking.
- Time: 2026-06-28 20:11

## Hypothesis H-023: Terminal-Bench Docker build does not forward reachable proxy settings
- Status: confirmed
- Parent: P-001
- Claim: The formal E3 suite is blocked on `multi-source-data-merger` because the generated Terminal-Bench validator performs Docker build steps that require Debian apt network access, but the adapter skips Windows loopback proxy variables for WSL and does not pass proxy build args into `docker build`.
- Layer: harness-environment
- Factor relation: sequential
- Depends on:
  - H-022
- Rationale:
  - The failing Dockerfile installs `tmux` and `asciinema` during build.
  - The host proxy is configured as `127.0.0.1:7890`.
  - The generated validator logs `proxy_env_skipped_loopback` for WSL and `proxy_env_count=0`.
  - The build command does not include explicit proxy build args.
- Falsifiable predictions:
  - If true: failed build logs show apt cannot connect to `deb.debian.org`, the runtime manifest records proxy bypass for WSL loopback, and a direct WSL Docker container with `--network host` can connect to `127.0.0.1:7890`.
  - If false: the same build would fail after successful apt/proxy access, or WSL Docker host networking could not reach the loopback proxy even when explicitly tested.
- Diagnostic evidence plan:
  - Prediction or clause under test: compare formal suite validator logs, generated adapter code, host proxy state, and a minimal WSL Docker connectivity probe.
  - Signal: `validation.stderr.log`, `validation.stdout.log`, `docker-build-result.json`, adapter generated command path, `Get-NetTCPConnection`, and `docker run --network host` socket probe.
  - Capture method: inspect the failed pair artifacts and execute a no-code-change connectivity probe against `python:3.11-slim`.
  - Event name or marker:
    - `proxy_env_skipped_loopback`
    - `docker_build_environment_failure`
    - `Unable to connect to deb.debian.org:http`
- Evidence gate: satisfied
- Related evidence:
  - E-033
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: make the generated Terminal-Bench validator forward proxy settings to both Docker build and run phases, preserving loopback proxy values when WSL uses host networking, and add harness coverage for that command-generation contract.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-033: failed formal pair proves build-network proxy bypass rather than agent solve failure
- Related hypotheses:
  - H-023
- Direction: supports
- Type: diagnostic
- Source: `target\e3f-after-pending-audit-fix\suite-20260628-202449\samples\multi-source-data-merger\runs\terminal_bench__multi-source-data-merger\20260628-215345-335\pair-001`
- Prediction or plan link:
  - H-023 diagnostic evidence plan.
- Matched signal:
  - Both left and right validator builds failed in Dockerfile step `RUN apt-get update && apt-get install -y tmux asciinema`.
  - Stderr contained `Unable to connect to deb.debian.org:http`, `Could not connect to debian.map.fastlydns.net:80`, `Package 'tmux' has no installation candidate`, and `Unable to locate package asciinema`.
  - Stdout recorded `docker_backend=wsl`, `proxy_env_skipped_loopback=HTTP_PROXY`, `proxy_env_skipped_loopback=HTTPS_PROXY`, `proxy_env_count=0`, `docker_cache_enabled=False`, and `docker_cache_bypass_reason=dockerfile_base_image_not_digest_pinned`.
  - `docker-build-result.json` classified the build phase as `docker_build_environment_failure`.
  - The host had `HTTP_PROXY=http://127.0.0.1:7890` and `HTTPS_PROXY=http://127.0.0.1:7890`, with port 7890 listening on `127.0.0.1`.
  - A minimal WSL Docker probe using `docker run --rm --network host python:3.11-slim` connected successfully to `127.0.0.1:7890`.
- Correlation keys:
  - `terminal_bench__multi-source-data-merger`
  - `pair-001`
  - `proxy_env_skipped_loopback`
- Raw content:
  ```text
  docker_build_result:
    phase = build
    exit_code = 1
    classification = docker_build_environment_failure

  validator stdout:
    docker_backend=wsl
    proxy_env_skipped_loopback=HTTP_PROXY
    proxy_env_skipped_loopback=HTTPS_PROXY
    proxy_env_count=0
    docker_cache_enabled=False
    docker_cache_bypass_reason=dockerfile_base_image_not_digest_pinned

  connectivity probe:
    proxy_connect=ok
  ```
- Interpretation: The formal run reached a harness infrastructure dependency: build-time apt traffic was direct even though a reachable proxy existed under WSL host networking. This does not indicate an agent engineering or solution-quality failure.
- Time: 2026-06-28 22:40

## Evidence E-034: WSL Docker build proxy forwarding clears the apt build blocker
- Related hypotheses:
  - H-023
- Direction: supports
- Type: fix-validation
- Source: `scripts\taskspace-benchmark\adapters\terminal-bench-adapter.ps1`, `scripts\taskspace-benchmark\test-terminal-bench-adapter-harness.ps1`, `target\r3-proxy-build-probe\20260628-2222`
- Prediction or plan link:
  - H-023 repair design direction.
- Matched signal:
  - Generated validators now preserve WSL loopback proxy variables under host networking and add matching `--build-arg` entries for Docker build.
  - `test-terminal-bench-adapter-harness.ps1` asserts proxy build args, loopback preservation, and absence of the old skip marker.
  - `test-terminal-bench-docker-cache-smoke.ps1` and `test-external-wrapper-harness.ps1` passed.
  - A no-agent `multi-source-data-merger` validator probe recorded `proxy_env_count=4`, `proxy_build_arg_count=4`, and `docker build` phase `exit_code=0`, `classification=ok`.
  - The same probe then failed in Docker run because the raw fixture had not solved the task, with missing `/app/merged_users.parquet` and `/app/conflicts.json`; this is expected for a no-agent build-network probe.
- Correlation keys:
  - `proxy_env_preserved_loopback`
  - `proxy_build_arg_count`
  - `docker_build_environment_failure`
- Raw content:
  ```text
  Terminal-Bench adapter self-test: PASS
  Terminal-Bench Docker cache smoke: PASS
  TaskSpace external wrapper self-test: PASS

  target\r3-proxy-build-probe\20260628-2222:
    proxy_env_count = 4
    proxy_build_arg_count = 4
    build exit_code = 0
    build classification = ok
    run exit_code = 1
    run classification = docker_run_failure
  ```
- Interpretation: The original formal blocker was removed at the harness build-network layer. The remaining no-agent probe failure is business-test failure on an unsolved fixture, not a Docker/apt materialization failure.
- Time: 2026-06-28 22:24

## Hypothesis H-024: start gate self-test fixture still expected old WSL proxy skip behavior
- Status: confirmed
- Parent: P-001
- Claim: After the build-network repair, full formal E3 was blocked by `test-harness.ps1` because the fixture still asserted the old `proxy_env_skipped_loopback` behavior, while the repaired adapter intentionally preserves WSL loopback proxy values under host networking.
- Layer: harness-test-contract
- Factor relation: sequential
- Depends on:
  - H-023
- Rationale:
  - Suite runner invokes start gate with `-RunSelfTests`.
  - The start gate failed only on `test-harness.ps1`.
  - The failure message named the stale WSL loopback proxy guard expectation.
- Falsifiable predictions:
  - If true before repair: start gate reports `self_test_failed` and `test-harness.ps1` fails on the WSL loopback proxy assertion.
  - If true after repair: updating the fixture to assert `proxy_env_preserved_loopback`, build proxy args, and absence of skip behavior makes both `test-harness.ps1` and `test-e3-start-gate.ps1` pass.
- Diagnostic evidence plan:
  - Prediction or clause under test: inspect the suite start-gate artifact and rerun the two affected self-tests after fixture update.
  - Signal: `e3-start-gate.json` self-test list and PowerShell self-test output.
  - Capture method: direct artifact read plus targeted self-test runs.
  - Event name or marker:
    - `self_test_failed`
    - `proxy_env_skipped_loopback`
    - `proxy_env_preserved_loopback`
- Evidence gate: satisfied
- Related evidence:
  - E-035
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: commit/push the fixture-contract update, regenerate current-HEAD gates/markers, then rerun formal E3.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-035: start gate self-test passes after proxy-contract fixture update
- Related hypotheses:
  - H-024
- Direction: supports
- Type: fix-validation
- Source: `scripts\taskspace-benchmark\test-harness.ps1`, `target\e3f-after-build-proxy-fix\suite-20260628-223434\start-gate\e3-start-gate.json`
- Prediction or plan link:
  - H-024 after-repair prediction.
- Matched signal:
  - Failed start gate recorded `first_failure_stable_code=self_test_failed`, `first_failure_command=.\scripts\taskspace-benchmark\test-harness.ps1`, and output `terminal-bench validator did not guard WSL loopback proxy injection`.
  - `test-harness.ps1` now asserts `proxy_env_preserved_loopback`, Docker build proxy args, and absence of `proxy_env_skipped_loopback`.
  - `test-harness.ps1` passed.
  - `test-e3-start-gate.ps1` passed.
- Correlation keys:
  - `test-harness.ps1`
  - `test-e3-start-gate.ps1`
- Raw content:
  ```text
  failed suite start gate:
    self_test = .\scripts\taskspace-benchmark\test-harness.ps1
    output = terminal-bench validator did not guard WSL loopback proxy injection

  after fixture update:
    TaskSpace benchmark harness self-test: PASS
    E3 start gate self-test: PASS
    git diff --check: PASS
  ```
- Interpretation: The formal E3 start-gate blocker was a stale test contract, not a runtime or model failure.
- Time: 2026-06-28 22:38

## Hypothesis H-025: large rollout cost instrumentation caused the formal E3 sample runner memory runaway
- Status: confirmed
- Parent: P-001
- Claim: `multi-source-data-merger` pair-003 did not fail in agent execution or Docker validation; it stalled while extracting metrics after validation because cost instrumentation repeatedly scanned a 103MB `rollout.jsonl`, causing PowerShell memory to grow into multi-GB range.
- Layer: benchmark-runner-observability
- Factor relation: sequential
- Depends on:
  - H-023
  - H-024
- Rationale:
  - pair-003 had both side validation logs and Docker results, but only left `metrics.json`.
  - right artifacts had `git-diff.patch` and `graph-health.json`, placing the stall after changed inventory and graph-health writing.
  - The stuck `run-taskspace-benchmark.ps1` process used about 3GB private memory and consumed about one CPU core continuously.
  - right `rollout.jsonl` was 103,255,682 bytes.
- Falsifiable predictions:
  - If true before repair: directly invoking metrics extraction on pair-003 right artifacts either hangs or grows memory while parsing rollout-derived cost diagnostics.
  - If true after repair: large rollout files are guarded by a cost scan policy, metrics extraction completes quickly, writes `metrics.json`, and records `rollout_scan_mode=skipped_large_rollout`.
- Diagnostic evidence plan:
  - Prediction or clause under test: reproduce metrics extraction on the exact pair-003 right artifacts without rerunning agent or Docker.
  - Signal: elapsed time, managed memory before/after, `metrics.json`, `cost-scan-policy.json`.
  - Capture method: targeted PowerShell invocation of `Get-TaskspaceBenchmarkMetrics`.
  - Event name or marker:
    - `rollout_scan_mode`
    - `skipped_large_rollout`
    - `cost-scan-policy.json`
- Evidence gate: satisfied
- Related evidence:
  - E-036
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: commit/push, refresh current-HEAD gates/markers, rerun formal E3 from a fresh run root.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-036: bounded changed inventory and large-rollout scan policy clear the pair-003 runner memory failure
- Related hypotheses:
  - H-025
- Direction: supports
- Type: fix-validation
- Source: `scripts\taskspace-benchmark\lib\metrics-extractor.ps1`, `scripts\taskspace-benchmark\lib\cost-instrumentation.ps1`, `scripts\taskspace-benchmark\lib\e3-proof.ps1`, `target\e3f-after-proxy-selftest-fix\suite-20260628-224933\samples\multi-source-data-merger\runs\terminal_bench__multi-source-data-merger\20260628-234737-098\pair-003`
- Prediction or plan link:
  - H-025 after-repair prediction.
- Matched signal:
  - The runaway process was stopped after CPU increased 59.5 seconds over a 60 second window and working set grew by about 684MB.
  - Suite health recorded `invalid_harness` because the child process was interrupted to prevent host memory pressure.
  - A focused metrics extractor harness passed, proving `.tbench-testing` files are excluded from changed inventory and large rollout scans are guarded.
  - E3 proof harness passed, proving validator-source isolation scanning skips runtime dependency trees while still detecting real repo leaks.
  - Direct metrics extraction on the real pair-003 right artifacts completed in 1,951ms, wrote `metrics.json`, and recorded `rollout_scan_mode=skipped_large_rollout`, `rollout_bytes=103255682`.
- Correlation keys:
  - `rollout.jsonl`
  - `cost-scan-policy.json`
  - `skipped_large_rollout`
  - `.tbench-testing`
- Raw content:
  ```text
  runaway process:
    PID = 7872
    CPU delta over 60s = 59.515625
    working set delta over 60s = 683671552
    private memory ~= 3.4GB

  focused tests:
    TaskSpace metrics extractor harness self-test: PASS
    TaskSpace E3 proof harness self-test: PASS

  real pair-003 right metrics extraction:
    elapsed_ms = 1951
    rollout_scan_mode = skipped_large_rollout
    rollout_bytes = 103255682
    changed_count = 0
  ```
- Interpretation: The formal suite failure after the proxy fix was a runner observability memory bug. The fix bounds diagnostic scanning without changing agent execution budgets or Docker validation semantics.
- Time: 2026-06-29 00:45

## Observation O-006: multi-source-data-merger still shows TaskSpace task-strategy regression
- Related hypotheses:
  - H-025
- Direction: neutral
- Type: behavioral-observation
- Source: pair reports and stderr logs under `target\e3f-after-proxy-selftest-fix\suite-20260628-224933\samples\multi-source-data-merger`
- Matched signal:
  - pair-001 and pair-002 both reported `outcome_standard=solved`, `outcome_taskspace=wrong`, `engineering_unclean=False`.
  - TaskSpace logs repeatedly attempted to read `/data/source_a/users.json` and `/data/source_b/users.csv` from the Windows agent workspace, where those paths do not exist.
  - TaskSpace also attempted to patch `W:\app\src\merge_users.py`, which does not exist in the materialized task.
- Interpretation: This is a real task-strategy/context issue exposed after the runner blockers were cleared. It is not fixed by H-025 and should remain a follow-up R3 product/agent-context investigation.
- Time: 2026-06-29 00:45

## Hypothesis H-026: observability exporter fully materializes large rollout payloads and duplicates them into HTML
- Status: confirmed
- Parent: P-001
- Claim: The current formal E3 rerun was stopped because `export-action-map-observability.ps1` reads the entire `rollout.jsonl` into memory, keeps raw snapshot/result payloads in the reduced model, writes a near-1GB JSON report, and embeds the same JSON into the HTML report. This is a benchmark observability artifact blow-up, not an agent solve failure.
- Layer: benchmark-runner-observability
- Factor relation: sequential
- Depends on:
  - H-025
- Rationale:
  - A later formal run reached `multi-source-data-merger` pair-001 after the cost instrumentation guard.
  - The TaskSpace side produced a 243,219,874 byte rollout, then the exporter process grew to multi-GB working set and generated near-1GB JSON and HTML artifacts.
  - The exporter code path uses `Read-JsonLines` to materialize all rows, stores raw `$payload` in `timeline.details`, copies snapshot result bodies into nodes, and embeds the full reduced JSON in the HTML `trace-data` script.
- Falsifiable predictions:
  - If true before repair: running the exporter on the same large rollout creates very large JSON/HTML and high memory pressure before scoring can proceed.
  - If true after repair: the same rollout is exported in `summary_only_large_rollout` mode, output artifacts stay small, oversized event payloads are counted but not materialized, and downstream cost metrics can read exact runtime event counts from the summary.
- Diagnostic evidence plan:
  - Prediction or clause under test: rerun only the observability exporter and cost reader on the exact pair-001 TaskSpace artifacts without rerunning agent or Docker.
  - Signal: rollout bytes, export mode, output JSON/HTML byte sizes, skipped large-line count, timeline boundedness, runtime event count, and cost instrumentation source status.
  - Capture method: focused exporter self-test plus direct export of `target\e3f-current\suite-20260629-010004\...\pair-001\right\artifacts\rollout.jsonl`.
  - Event name or marker:
    - `summary_only_large_rollout`
    - `action-map-observability-policy.json`
    - `largeLineSkippedCount`
    - `timelineEventsDropped`
- Evidence gate: satisfied
- Related evidence:
  - E-037
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: commit/push the bounded observability exporter, refresh current-HEAD gates/markers because script SHA changed, then rerun formal E3 from a fresh run root.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-037: bounded summary exporter clears the 243MB rollout observability blow-up
- Related hypotheses:
  - H-026
- Direction: supports
- Type: fix-validation
- Source: `scripts\export-action-map-observability.ps1`, `scripts\action-map-observability-summary-lib.ps1`, `scripts\test-action-map-observability-summary-export.ps1`, `target\r3-real-large-observability-export-test`
- Prediction or plan link:
  - H-026 after-repair prediction.
- Matched signal:
  - Focused summary exporter harness passed and proved raw large result-body markers are absent from JSON/HTML.
  - Existing small-rollout observability report harness still passed.
  - Cost instrumentation self-test still passed.
  - Direct export of the real 243,219,874 byte rollout completed in 20.7 seconds and wrote small reports: JSON 394,428 bytes, HTML 402,737 bytes, Markdown 28,642 bytes.
  - The real export recorded `rollout_export_mode=summary_only_large_rollout`, `timeline_count=240`, `timeline_dropped=1805`, `parsed_lines=1952`, `largeLineSkippedCount=95`, `parse_errors=0`, and `mapRuntimeEvents=3067`.
  - Downstream cost instrumentation read the summary JSON with `observability_source_status=summary_only_large_rollout` and preserved `taskspace_runtime_event_count=3067`.
- Correlation keys:
  - `target\e3f-current\suite-20260629-010004`
  - `pair-001\right\artifacts\rollout.jsonl`
  - `action-map-observability-policy.json`
- Raw content:
  ```text
  focused tests:
    test-action-map-observability-summary-export.ps1 = PASS
    test-action-map-observability-lib.ps1 = PASS
    test-cost-instrumentation.ps1 = PASS

  real rollout export:
    rollout_bytes = 243219874
    mode = summary_only_large_rollout
    json_bytes = 394428
    html_bytes = 402737
    timeline_count = 240
    timeline_dropped = 1805
    parsed_lines = 1952
    large_skipped = 95
    parse_errors = 0
    map_runtime_events = 3067

  downstream cost read:
    observability_source_status = summary_only_large_rollout
    taskspace_runtime_event_count = 3067
    runtime_state_commit_count = 6
  ```
- Interpretation: The new guard bounds observability artifact size and memory risk while retaining exact runtime event counts for downstream cost metrics. It does not change agent execution, TaskSpace graph semantics, provider budgets, or Docker validation.
- Time: 2026-06-29 01:35

## Hypothesis H-027: pre-agent validator probe artifacts were written into agent-visible artifacts
- Status: confirmed
- Parent: P-001
- Claim: The post-observability formal E3 run failed validator-source isolation proof because the runner wrote pre-agent validator probe manifests under `side.ArtifactDir\vprobe`, which is mounted as `W:\artifacts` and can be read by the agent. This exposed validator-source path metadata to the standard-side transcript even though actual validator source files were protected by the source guard.
- Layer: benchmark-runner-isolation
- Factor relation: sequential
- Depends on:
  - H-026
- Rationale:
  - The failed pair solved the task on both standard and TaskSpace sides.
  - Source guard proof showed validator files were denied and restored.
  - The isolation failure was driven by `agent_artifact_validator_tokens` on the standard side.
  - The standard agent read `W:\artifacts\vprobe\terminal-bench-runtime-manifest.json`, which contained the `external-validator-source` path token.
- Falsifiable predictions:
  - If true before repair: `left\artifacts\whale-exec.jsonl` contains an agent command reading `W:\artifacts\vprobe\terminal-bench-runtime-manifest.json`, and the manifest contains `external-validator-source`.
  - If true after repair: probe proof files move under `_runner-private\<side>\vprobe`, `left/right/artifacts\vprobe` are absent, `agent_artifact_validator_tokens` are empty for both sides, and `proof_agent_cannot_read_validator_source=True`.
  - If false: the validator-source token still appears in agent transcript or repo files after moving pre-agent probe artifacts out of `ArtifactDir`.
- Diagnostic evidence plan:
  - Prediction or clause under test: compare the failed formal pair's agent-visible artifacts with a focused processing-pipeline rerun after moving probe output to runner-private storage.
  - Signal: `external-isolation-proof.json`, `pair-report.md`, existence of `left/right/artifacts/vprobe`, and `_runner-private/*/vprobe/validator-probe-result.json`.
  - Capture method: inspect failed formal artifacts, update runner workspace/probe paths, run harness self-tests and a one-pair processing-pipeline scoring rerun.
  - Event name or marker:
    - `agent_artifact_validator_tokens`
    - `proof_agent_cannot_read_validator_source`
    - `external-validator-source`
- Evidence gate: satisfied
- Related evidence:
  - E-038
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: commit/push the runner-private probe isolation fix, refresh current-HEAD gates/markers because runner scripts changed, then rerun formal E3 from a fresh run root.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-038: runner-private probe output clears validator-source isolation proof
- Related hypotheses:
  - H-027
- Direction: supports
- Type: fix-validation
- Source: `scripts\taskspace-benchmark\lib\workspace.ps1`, `scripts\taskspace-benchmark\run-taskspace-benchmark.ps1`, `scripts\taskspace-benchmark\test-harness.ps1`, `target\r3-processing-pipeline-runner-private-proof`
- Prediction or plan link:
  - H-027 after-repair prediction.
- Matched signal:
  - Pre-agent validator probe outputs are now written under `pair-001\_runner-private\<side>\vprobe`.
  - `left\artifacts\vprobe` and `right\artifacts\vprobe` are absent in the focused rerun.
  - `external-isolation-proof.json` reports `agent_artifact_validator_tokens=[]` for both sides and `agent_cannot_read_validator_source_proven=true`.
  - `pair-report.md` reports `engineering_unclean=False`, `outcome_standard=solved`, `outcome_taskspace=solved`, `proof_agent_cannot_read_validator_source=True`, and `proof_validator_e3_eligible=True`.
  - Harness self-tests passed before the focused rerun.
- Correlation keys:
  - `target\e3f-after-observability-guard\suite-20260629-023835`
  - `target\r3-processing-pipeline-runner-private-proof\terminal_bench__processing-pipeline\20260629-025723-879`
  - `_runner-private`
  - `artifacts\vprobe`
- Raw content:
  ```text
  focused tests:
    scripts\taskspace-benchmark\test-harness.ps1 = PASS
    scripts\taskspace-benchmark\test-e3-proof-harness.ps1 -RunRoot target\r3-e3-proof-runner-private-test = PASS

  focused processing-pipeline rerun:
    command exit_code = 0
    proof_agent_cannot_read_validator_source = True
    proof_validator_e3_eligible = True
    engineering_unclean = False
    outcome_standard = solved
    outcome_taskspace = solved
    left_artifacts_vprobe_exists = False
    right_artifacts_vprobe_exists = False
    runner_private_validator_probe_results = 2
  ```
- Interpretation: The E3 isolation failure was a runner artifact placement bug. Moving pre-agent probe outputs to runner-private storage preserves proof evidence for the runner while removing validator-source metadata from the agent-readable artifact mount.
- Time: 2026-06-29 03:12

## Hypothesis H-028: Terminal-Bench validator Docker backend probe timeout is too brittle for WSL under pressure
- Status: confirmed
- Parent: P-001
- Claim: The latest formal E3 rerun stopped on `multi-source-data-merger` because the generated Terminal-Bench validator uses a fixed 20 second PowerShell `Start-Job` probe for `wsl -d whale-docker -- docker version`; under current Windows/WSL resource pressure this can time out even though Docker is available and a direct or later job probe succeeds.
- Layer: benchmark-runner-environment-probe
- Factor relation: sequential
- Depends on:
  - H-027
- Rationale:
  - The formal suite completed all five `processing-pipeline` pairs and preserved them as `audit_required`.
  - `multi-source-data-merger` pair-001 failed in scoring mode with `docker_backend_unavailable` before standard-side tests started.
  - The failure message is specifically `probe timed out after 20 seconds`, not a Docker build or test assertion failure.
  - A direct Docker check after the failure returned Docker server version `29.1.3`, and a generated validator with a 60 second configurable probe passed `-ProbeOnly`.
- Falsifiable predictions:
  - If true before repair: rerunning the old generated validator `-ProbeOnly` on the same left workspace still fails with `docker_backend_unavailable` and `probe timed out after 20 seconds`.
  - If true after repair: newly materialized validators expose `TASKSPACE_DOCKER_BACKEND_PROBE_TIMEOUT_SECONDS`, default to 60 seconds with a bounded 20-300 second range, and the same real sample `-ProbeOnly` passes with `docker_backend=wsl`.
  - If false: the new validator would still fail backend discovery, or Docker would be unavailable to direct probes as well.
- Diagnostic evidence plan:
  - Prediction or clause under test: compare old formal pair probe result, direct Docker probes, generated validator text, harness tests, and a newly materialized real-sample `-ProbeOnly`.
  - Signal: `validator-probe-result.json`, Docker version command elapsed time, adapter self-tests, and real validator `-ProbeOnly` result.
  - Capture method: targeted PowerShell probes without rerunning agent, then adapter code update and no-agent real-sample validator probe.
  - Event name or marker:
    - `docker_backend_unavailable`
    - `probe timed out after 20 seconds`
    - `TASKSPACE_DOCKER_BACKEND_PROBE_TIMEOUT_SECONDS`
- Evidence gate: satisfied
- Related evidence:
  - E-039
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: commit/push the configurable backend probe timeout, refresh current-HEAD gates/markers because adapter/test scripts changed, then rerun formal E3.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-039: configurable 60 second Docker backend probe clears the real sample probe
- Related hypotheses:
  - H-028
- Direction: supports
- Type: fix-validation
- Source: `scripts\taskspace-benchmark\adapters\terminal-bench-adapter.ps1`, `scripts\taskspace-benchmark\test-harness.ps1`, `scripts\taskspace-benchmark\test-terminal-bench-adapter-harness.ps1`, `target\r3-docker-probe-timeout-materialized-probe`
- Prediction or plan link:
  - H-028 after-repair prediction.
- Matched signal:
  - The failed formal pair recorded `docker_backend_unavailable`, `left_tests_started_seen=False`, and `WSL[whale-docker] exit=124; probe timed out after 20 seconds`.
  - Direct Docker checks after the failure returned server version `29.1.3`; one WSL direct probe took 11,601ms and a `Start-Job` probe took 10,379ms, showing backend availability with non-trivial WSL latency.
  - The old generated validator `-ProbeOnly` reproduced the 20 second timeout on the left workspace.
  - The adapter now generates `TASKSPACE_DOCKER_BACKEND_PROBE_TIMEOUT_SECONDS`, default `return 60`, lower bound 20, upper bound 300.
  - `test-harness.ps1`, `test-terminal-bench-adapter-harness.ps1`, and `git diff --check` passed.
  - A newly materialized real `multi-source-data-merger` validator passed `-ProbeOnly` with `docker_backend=wsl` and wrote `validator_probe_completed=true`.
- Correlation keys:
  - `target\e3f-after-runner-private\suite-20260629-032450`
  - `target\r3-multi-source-left-probe-recheck`
  - `target\r3-docker-probe-timeout-materialized`
  - `TASKSPACE_DOCKER_BACKEND_PROBE_TIMEOUT_SECONDS`
- Raw content:
  ```text
  failed formal pair:
    reason = docker_backend_unavailable
    normalized_message = WSL[whale-docker] exit=124; probe timed out after 20 seconds
    left_tests_started_seen = False

  focused checks:
    direct wsl docker version = 29.1.3, elapsed_ms = 11601
    Start-Job wsl docker version = 29.1.3, elapsed_ms = 10379
    old validator -ProbeOnly exit_code = 1

  focused tests:
    test-harness.ps1 = PASS
    test-terminal-bench-adapter-harness.ps1 = PASS
    git diff --check = PASS

  new real validator probe:
    exit_code = 0
    status = pass
    stage = probe
    docker_backend = wsl
  ```
- Interpretation: Docker itself was available; the harness misclassified availability because the generated backend probe used a brittle fixed timeout. The repair keeps the probe time-bounded but makes the bound appropriate and configurable for WSL-backed validation under load.
- Time: 2026-06-29 04:52

## Observation O-007: latest formal E3 still exposes a real TaskSpace regression on multi-source-data-merger
- Related hypotheses:
  - H-028
- Direction: neutral
- Type: behavioral-observation
- Source: `target\e3f-after-runner-private\suite-20260629-032450\samples\multi-source-data-merger\runs\terminal_bench__multi-source-data-merger\20260629-042351-268\pair-001\pair-report.md`
- Matched signal:
  - Standard side changed `conflicts.json` and `merged_users.parquet`.
  - TaskSpace side exited 1, changed no files, and left `open_leaf_nodes=1`.
  - TaskSpace wall time ratio was 12.86 and tool call ratio was 7.18.
  - Right-side Docker build succeeded and tests ran, so the TaskSpace side had a real wrong outcome independent of the left-side pretest failure.
- Interpretation: H-028 must be fixed before this sample can be scored cleanly, but the TaskSpace behavior remains a product/agent-context issue for R3. It should not be hidden as a pure harness failure after Docker probe stability is restored.
- Time: 2026-06-29 04:52

## Hypothesis H-029: TaskSpace implement phase loses executable edit intent after path evidence and strict-JSON rejection
- Status: confirmed
- Parent: P-001
- Claim: After harness blockers are cleared, the remaining formal E3 failures on `multi-source-data-merger` and `recover-accuracy-log` are caused by TaskSpace implement-phase context/action convergence, not by Docker, validator, DeepSeek outage, or request hard limits. The model sees useful path evidence, but the next provider payload does not constrain implementation strongly enough; when it emits `apply_patch` with extra prose or a second action, strict JSON rejection is followed by more read/list actions instead of a forced corrected patch.
- Layer: taskspace-context-action-contract
- Factor relation: recurring
- Depends on:
  - H-028
- Rationale:
  - The formal E3 run after the Docker probe timeout fix completed all three samples with `run_validity=valid` and no invalid harness samples.
  - `multi-source-data-merger` and `recover-accuracy-log` both show standard solving the task while TaskSpace repeatedly reports `agent_no_patch`.
  - The transcripts show TaskSpace discovered correct relative paths such as `task_deps/generator.log`, then later returned to wrong absolute paths such as `/app/raw_logs/generator.log`.
  - At least one `apply_patch` response was rejected because the assistant output was not exactly one strict JSON action object; recovery then continued with file reads/lists.
- Falsifiable predictions:
  - If true before repair: formal artifacts show repeated `agent_no_patch`, `action_contract_output_not_strict_json`, wrong-path reads after correct-path evidence, and no successful edit artifact.
  - If true after repair: a focused rerun should either execute a corrected patch after strict-JSON rejection or terminate as `blocked` with evidence, and should not allow read/list rediscovery after an emitted-but-unexecuted patch.
  - If false: failures should correlate with validator infrastructure, Docker backend/build failures, model API errors, or hard request/session budget stops.
- Diagnostic evidence plan:
  - Prediction or clause under test: inspect pair transcripts and pair reports from the completed formal E3 run.
  - Signal: `failure_taxonomy`, `outcome_standard`, `outcome_taskspace`, `TaskSpaceProviderRequestBudgetEventV1`, `TaskSpaceProviderResponseActionabilityV1`, `action_contract_output_not_strict_json`, wrong-path read errors, and `apply_patch` assistant messages.
  - Capture method: parse `pair-report.md` and `whale-exec.jsonl` for `multi-source-data-merger` and `recover-accuracy-log`.
  - Event name or marker:
    - `agent_no_patch`
    - `action_contract_output_not_strict_json`
    - `TaskSpaceNoActionRecoveryV1`
    - `over_profile_hint`
  - Correlation keys:
    - `target\e3f-after-docker-probe-timeout\suite-20260629-061122`
    - `terminal_bench__multi-source-data-merger`
    - `terminal_bench__recover-accuracy-log`
  - Differentiates from:
    - Docker backend unavailability
    - validator build/run environment failure
    - API outage or billing failure
    - request budget hard stop
  - Supports if:
    - run_validity stays valid, engineering_unclean_count stays zero, standard solves, and TaskSpace fails via no-patch / wrong-path / strict-JSON recovery loops.
  - Refutes if:
    - TaskSpace failures are invalid harness, validator infra, or hard budget stops.
  - Instrumentation status: available
  - Instrumentation lifecycle:
    - formal E3 artifacts retained under target
- Evidence gate: satisfied
- Related evidence:
  - E-040
  - E-041
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: implement edit-intent latch plus context-compiler implementation facts for verified paths/output targets; update action-contract recovery so strict JSON patch rejection forces a single corrected action rather than rediscovery.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-040: Docker-probe-fixed formal E3 completes all samples as audit_required, not invalid_harness
- Related hypotheses:
  - H-029
- Direction: supports
- Type: formal-run
- Source: `target\e3f-after-docker-probe-timeout\suite-20260629-061122\suite-health.json`
- Prediction or plan link:
  - H-029 differentiates from harness failure.
- Matched signal:
  - `status=audit_required`
  - `invalid_harness_sample_count=0`
  - `completed_child_processes=3`
  - `score_pending_audit_child_runs=3`
  - `suite_score_ready=false`
  - `suite_score_valid=false`
- Correlation keys:
  - `terminal-bench_E3-P0_3_5`
  - `de1c223db57ea05e0c87839bb9d13677eb4faa84d3a3830df2b36d7e0ecac5a2`
  - `e9278edb8951ccc392cda407be0a4213fa70e3ce2c1b9ee647b1d4720e9a6789`
- Raw content:
  ```text
  status = audit_required
  invalid_harness_sample_count = 0
  completed_child_processes = 3
  score_pending_audit_child_runs = 3
  expected_time_saved_basis = no_skipped_work
  generated_at = 2026-06-29T09:15:29.5652387+08:00
  ```
- Interpretation: The current blocker is no longer a suite materialization or Docker probe failure. The suite produced valid, auditable artifacts for all selected samples.
- Time: 2026-06-29 09:20

## Evidence E-041: formal E3 shows stable agent_no_patch regression on two samples
- Related hypotheses:
  - H-029
- Direction: supports
- Type: formal-run-artifact
- Source: `aggregate.json` and `pair-report.md` files under `target\e3f-after-docker-probe-timeout\suite-20260629-061122`
- Prediction or plan link:
  - H-029 If true prediction.
- Matched signal:
  - `recover-accuracy-log`: all five pairs `standard=solved`, `taskspace=wrong`, `failure_taxonomy=agent_no_patch`.
  - `multi-source-data-merger`: all five pairs `standard=solved`; TaskSpace has four `wrong` and one `agent_exec_timeout`; taxonomy includes `agent_no_patch=4` and `taskspace_overhead_timeout=1`.
  - `engineering_unclean_count=0` for both samples.
- Correlation keys:
  - `terminal_bench__recover-accuracy-log`
  - `terminal_bench__multi-source-data-merger`
  - `agent_no_patch`
- Raw content:
  ```text
  recover-accuracy-log aggregate:
    run_validity = valid
    engineering_unclean_count = 0
    audit_required_count = 5
    failure_taxonomy_summary = agent_no_patch=5, audit_unclean=5

  multi-source-data-merger aggregate:
    run_validity = valid
    engineering_unclean_count = 0
    audit_required_count = 5
    failure_taxonomy_summary = agent_no_patch=4, taskspace_overhead_timeout=1, audit_unclean=5

  recover-accuracy-log pair-002 transcript:
    action_contract_output_not_strict_json
    later list/read actions continue
    /app/raw_logs/generator.log not found after task_deps/generator.log was listed/read
  ```
- Interpretation: The failure is a real TaskSpace action/context strategy regression. It is not an infrastructure failure, and profile overrun remains advisory rather than a hard stop.
- Time: 2026-06-29 09:20

## Evidence E-042: strict JSON apply_patch intent now enters implementation edit recovery
- Related hypotheses:
  - H-029
- Direction: supports-repair
- Type: code-change-and-test
- Source: `third_party\codex-cli\codex-rs\core\src\session\turn.rs`
- Prediction or plan link:
  - H-029 after-repair prediction: strict JSON patch rejection should force corrected patch or blocked, not rediscovery.
- Matched signal:
  - Added `TaskSpacePatchIntentFormatRecoveryV1`.
  - `action_contract_output_not_strict_json` responses that contain a taskspace `apply_patch` intent now carry an `apply_patch_intent` marker plus a short rejected-output preview.
  - Recovery dispatch handles this marker before generic `TaskSpaceNoActionRecoveryV1`.
  - The recovery prompt forbids `read_file`, `list_files`, `search`, broad discovery, and validation from the implementation node.
  - The recovery counts against the implementation edit recovery cap, not the generic no-action cap.
  - Warning logs now distinguish `TaskSpacePatchIntentFormatRecoveryV1`.
- Correlation keys:
  - `TASKSPACE_PATCH_INTENT_FORMAT_MARKER`
  - `taskspace_raw_text_mentions_apply_patch_intent`
  - `taskspace_message_hit_apply_patch_intent_format_rejection`
  - `build_taskspace_patch_intent_format_recovery_item`
- Raw content:
  ```text
  cargo fmt -p codex-core = PASS
  cargo test -p codex-core taskspace_patch_intent --lib -- --nocapture = PASS
  cargo test -p codex-core taskspace_strict_json_apply_patch_intent --lib -- --nocapture = PASS
  cargo test -p codex-core patch_intent_format_recovery_has_own_cap_marker --lib -- --nocapture = PASS
  cargo test -p codex-core taskspace_action_contract --lib -- --nocapture = PASS, 40 tests
  cargo test -p codex-core implement_needs_edit --lib -- --nocapture = PASS
  ```
- Interpretation: The specific failure chain seen in `recover-accuracy-log` pair-002 is now covered at the action-contract layer. This proves the recovery mechanics, but not yet external-task solve-rate improvement; the next evidence must come from a real rerun.
- Time: 2026-06-29 10:20

## Hypothesis H-030: input-data evidence is not promoted into implement working evidence
- Claim: Some Terminal-Bench tasks require editing from verified input data rather than code/test source. TaskSpace treated successful reads of files such as `task_deps/generator.log` and `task_deps/judge.log` as historical tool output, not as dependency working evidence for the implementation node. That allowed implement nodes to rediscover wrong absolute paths instead of editing from already verified data.
- Parent:
  - H-029
- If true:
  - A real `recover-accuracy-log` rerun after strict-JSON recovery should still fail without triggering `TaskSpacePatchIntentFormatRecoveryV1`.
  - The transcript should show inspect reading `task_deps/*.log`, then implement reading or listing wrong paths such as `/app/raw_logs/*`.
  - Promoting input data artifact refs into working evidence should reduce rediscovery and force apply_patch/blocked.
- If false:
  - The rerun should either solve or fail from semantic patch/test issues after a successful edit.
- Diagnostic evidence plan:
  - Prediction or clause under test: compare the post strict-JSON rerun with the post input-evidence rerun.
  - Signal: `tool_call_count`, `open_leaf_nodes`, wrong-path reads, `TaskSpaceImplementNeedsEditRecoveryV1`, `changed_paths`.
  - Capture method: inspect `whale-exec.jsonl`, `whale-exec.stderr.log`, active-context report, and pair artifacts.
  - Event name or marker:
    - `verified_input_evidence`
    - `dependency_working_evidence`
    - `TaskSpaceImplementNeedsEditRecoveryV1`
  - Correlation keys:
    - `target\r3-patch-intent-recover-accuracy-log`
    - `target\r3-input-evidence-recover-accuracy-log`
  - Differentiates from:
    - strict JSON patch intent rejection
    - Docker or validator infrastructure
    - hard request budget stop
    - semantic solve error after an edit
  - Supports if:
    - before fix: correct `task_deps` reads are followed by wrong-path rediscovery and no patch.
    - after fix: wrong-path rediscovery stops and the model attempts apply_patch or blocks from edit execution evidence.
  - Refutes if:
    - no verified input data was read, or implement already received the data and still made only semantic mistakes after successful edit.
- Evidence gate: satisfied
- Related evidence:
  - E-043
  - E-044
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: validate whether the next blocker is patch grammar rather than context evidence.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-043: post strict-JSON rerun still fails by losing verified input paths
- Related hypotheses:
  - H-030
- Direction: supports
- Type: real-task-rerun
- Source: `target\r3-patch-intent-recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260629-095321-016`
- Prediction or plan link:
  - H-030 before-repair prediction.
- Matched signal:
  - `standard=solved`
  - `taskspace=wrong`
  - `failure_taxonomy=engineering_unclean, agent_no_patch, audit_unclean`
  - `changed_paths` empty
  - `tool_call_count=11`
  - `open_leaf_nodes=1`
  - Active context replacement passed: `exact_payload_scan_passed=true`, `replacement_confirmed=true`, `legacy_taskspace_history_present=false`.
  - No `TaskSpacePatchIntentFormatRecoveryV1` marker appeared in the run.
  - Transcript showed successful reads of `task_deps/generator.log` and `task_deps/judge.log`, then implement-stage rediscovery of `/app/raw_logs/*`.
- Correlation keys:
  - `20260629-095321-016`
  - `recover-accuracy-log`
  - `agent_no_patch`
- Raw content:
  ```text
  RunRoot = target\r3-patch-intent-recover-accuracy-log
  pair-001 standard = solved
  pair-001 taskspace = wrong
  taskspace changed_paths = empty
  taskspace tool_call_count = 11
  taskspace open_leaf_nodes = 1
  active context replacement = passed
  ```
- Interpretation: The strict JSON patch-intent recovery is not sufficient for this sample because the run did not reach that failure mode. The next limiting factor is context/evidence promotion from inspected input data into implementation pressure.
- Time: 2026-06-29 10:35

## Evidence E-044: input-data working evidence repair reduces rediscovery and reaches apply_patch
- Related hypotheses:
  - H-030
- Direction: supports-repair
- Type: code-change-test-and-real-task-rerun
- Source:
  - `third_party\codex-cli\codex-rs\core\src\action_map\runtime.rs`
  - `target\r3-input-evidence-recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260629-101533-488`
- Prediction or plan link:
  - H-030 after-repair prediction.
- Matched signal:
  - Added `projection_verified_input_evidence`.
  - Successful reads of input artifacts with extensions such as `.log`, `.jsonl`, `.json`, `.csv`, `.tsv`, `.yaml`, `.yml`, and `.txt` now count as working evidence unless they are docs/tests/readme.
  - `current_main_implement_progress_needs_edit()` becomes true when inspect dependency input data exists.
  - Real rerun: `tool_call_count` dropped from 11 to 7, `open_leaf_nodes` dropped from 1 to 0.
  - Real rerun: implement no longer continued path rediscovery after evidence pressure; it attempted `apply_patch` twice, then blocked.
- Correlation keys:
  - `projection_verified_input_evidence`
  - `successful_read_result_has_working_evidence`
  - `implement_dependency_input_data_evidence_blocks_rediscovery_reads`
  - `20260629-101533-488`
- Raw content:
  ```text
  cargo fmt -p codex-core = PASS
  cargo test -p codex-core implement_dependency_input_data_evidence_blocks_rediscovery_reads --lib -- --nocapture = PASS
  cargo test -p codex-core projection_ --lib -- --nocapture = PASS, 8 tests
  cargo test -p codex-core implement_dependency --lib -- --nocapture = PASS, 2 tests
  cargo test -p codex-core forced_inspect_transition_skips_with_readme_only_evidence --lib -- --nocapture = PASS
  cargo test -p codex-core active_projection_keeps_high_signal_artifact_excerpt_for_implement_node --lib -- --nocapture = PASS
  cargo test -p codex-core inspect_progress_convergence_force_finishes_after_contract_hint --lib -- --nocapture = PASS

  real rerun pair-001:
    standard = solved
    taskspace = wrong
    failure_taxonomy = engineering_unclean, agent_no_patch, audit_unclean
    taskspace changed_paths = empty
    taskspace tool_call_count = 7
    taskspace open_leaf_nodes = 0
  ```
- Interpretation: This proves a real convergence benefit, but not solve-rate benefit. The blocker moved from context/path rediscovery to apply_patch new-file grammar.
- Time: 2026-06-29 10:40

## Hypothesis H-031: new-file apply_patch attempts use unified-diff add syntax that native apply_patch treats as update
- Claim: After input-data evidence repair, the model reaches edit intent but emits a new-file patch using unified diff headers (`--- /dev/null`, `+++ b/<path>`, `@@ -0,0 +...`). The native apply_patch grammar expects `*** Add File: <path>` for new files, so the tool tries to update a missing file, fails, and the model incorrectly blocks because it thinks a missing file cannot be patched.
- Parent:
  - H-030
- If true:
  - Real rerun artifacts should show apply_patch payloads with `/dev/null` new-file headers.
  - Tool stderr should contain `apply_patch verification failed: Failed to read file to update ...`.
  - The model should then block with a reason similar to "target file does not exist".
  - A recovery marker should instruct `*** Add File: <relative/path>` and count against implement-needs-edit cap.
- If false:
  - apply_patch failures should be due to semantic content, permissions, or validator/test failures rather than native patch grammar.
- Diagnostic evidence plan:
  - Prediction or clause under test: parse right-side artifacts from `20260629-101533-488`.
  - Signal: apply_patch payload headers, stderr tool error, blocked reason, new recovery marker tests.
  - Capture method: targeted `Select-String` on `whale-exec.jsonl` and `whale-exec.stderr.log`.
  - Event name or marker:
    - `TaskSpaceApplyPatchMissingTargetRecoveryV1`
    - `apply_patch verification failed`
    - `Failed to read file to update`
  - Correlation keys:
    - `item_24`
    - `item_27`
    - `item_30`
  - Differentiates from:
    - no-action recovery
    - strict JSON format rejection
    - existing-file add recovery
    - semantic wrong answer
  - Supports if:
    - patch payload is `/dev/null` new-file style and tool error says missing update target.
  - Refutes if:
    - patch syntax is native `*** Add File` or `*** Update File` and the failure is downstream validation.
- Evidence gate: satisfied
- Related evidence:
  - E-045
  - E-046
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: rebuild whale and rerun `recover-accuracy-log` to prove whether the marker converts the next patch to `*** Add File` and records changed paths.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-045: real rerun shows unified-diff new-file payloads and missing update target failure
- Related hypotheses:
  - H-031
- Direction: supports
- Type: real-task-rerun-artifact
- Source:
  - `target\r3-input-evidence-recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260629-101533-488\pair-001\right\artifacts\whale-exec.jsonl`
  - `target\r3-input-evidence-recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260629-101533-488\pair-001\right\artifacts\whale-exec.stderr.log`
- Prediction or plan link:
  - H-031 If true prediction.
- Matched signal:
  - `item_24` apply_patch payload starts with `--- /dev/null` and `+++ recover_accuracy.py`.
  - `item_27` apply_patch payload starts with `--- /dev/null` and `+++ b/recover_accuracy.py`.
  - stderr has two errors: `apply_patch verification failed: Failed to read file to update W:\app\src\recover_accuracy.py: 系统找不到指定的路径。 (os error 3)`.
  - `item_30` blocks with `Cannot apply_patch because target file recover_accuracy.py does not exist... Need to create new Python script but current narrowed state prevents file creation.`
- Correlation keys:
  - `item_24`
  - `item_27`
  - `item_30`
  - `recover_accuracy.py`
- Raw content:
  ```text
  item_24 patch:
    *** Begin Patch
    --- /dev/null
    +++ recover_accuracy.py
    @@ -0,0 +1,112 @@

  item_27 patch:
    *** Begin Patch
    --- /dev/null
    +++ b/recover_accuracy.py
    @@ -0,0 +1,80 @@

  stderr:
    apply_patch verification failed: Failed to read file to update W:\app\src\recover_accuracy.py:
    系统找不到指定的路径。 (os error 3)
  ```
- Interpretation: The model had executable edit intent and enough task evidence, but the action-contract layer did not recover the native new-file patch grammar. This is a system contract gap rather than a task-solving failure.
- Time: 2026-06-29 10:45

## Evidence E-046: missing-target apply_patch recovery is implemented and covered by focused tests
- Related hypotheses:
  - H-031
- Direction: supports-repair
- Type: code-change-and-test
- Source: `third_party\codex-cli\codex-rs\core\src\session\turn.rs`
- Prediction or plan link:
  - H-031 after-repair prediction.
- Matched signal:
  - Added `TaskSpaceApplyPatchMissingTargetRecoveryV1`.
  - Tool errors matching `apply_patch verification failed: Failed to read file to update ...` are parsed into relative targets such as `recover_accuracy.py`.
  - Recovery instructs native `*** Add File: <relative/path>` for new files and `*** Update File: <relative/path>` for existing targets.
  - Recovery forbids `--- /dev/null`, `+++ b/<path>`, and `@@ -0,0 +...` native patch payloads.
  - Recovery counts against implement-needs-edit cap, not generic no-action cap.
  - Static TaskSpace action-contract instructions now mention native Add File / Update File grammar.
- Correlation keys:
  - `TASKSPACE_APPLY_PATCH_MISSING_TARGET_MARKER`
  - `taskspace_missing_update_targets_from_apply_patch_error`
  - `build_taskspace_apply_patch_missing_target_recovery_item`
- Raw content:
  ```text
  cargo test -p codex-core taskspace_apply_patch_missing_target --lib -- --nocapture = PASS, 2 tests
  cargo test -p codex-core implement_dependency_input_data_evidence_blocks_rediscovery_reads --lib -- --nocapture = PASS
  cargo test -p codex-core taskspace_patch_intent --lib -- --nocapture = PASS
  cargo test -p codex-core taskspace_action_contract --lib -- --nocapture = PASS, 40 tests
  cargo test -p codex-core implement_needs_edit --lib -- --nocapture = PASS
  ```
- Interpretation: The system now has a structured recovery path for the exact next blocker found by the real rerun. The remaining required proof is another real `recover-accuracy-log` rerun after rebuilding the CLI.
- Time: 2026-06-29 10:50

## Evidence E-047: post recovery rerun proves no-patch is eliminated and exposes validation strategy blocker
- Related hypotheses:
  - H-030
  - H-031
- Direction: supports-repair
- Type: real-task-rerun
- Source:
  - `target\r3-missing-target-recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260629-104713-839\pair-001\pair-report.md`
  - `target\r3-missing-target-recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260629-104713-839\pair-001\right\artifacts\whale-exec.jsonl`
  - `target\r3-missing-target-recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260629-104713-839\pair-001\right\artifacts\validation.stdout.log`
- Prediction or plan link:
  - H-031 next step: prove whether the marker/guidance converts the next patch to `*** Add File` and records changed paths.
- Matched signal:
  - `failure_taxonomy` changed from `agent_no_patch` to `agent_patch_wrong`.
  - TaskSpace produced `changed_paths=recover_logs.py`.
  - `tool_call_count` improved to 6.
  - `open_leaf_nodes=0`.
  - `item_23` uses native `*** Add File: /app/recover_logs.py`.
  - `item_24` records completed file add for `W:\app\recover_logs.py`.
  - Public validation fails because `/app/recovered_logs/results.json` does not exist.
- Correlation keys:
  - `20260629-104713-839`
  - `recover_logs.py`
  - `agent_patch_wrong`
- Raw content:
  ```text
  pair-001:
    standard = solved
    taskspace = wrong
    failure_taxonomy = engineering_unclean, agent_patch_wrong, audit_unclean
    taskspace changed_paths = recover_logs.py
    taskspace tool_call_count = 6
    taskspace open_leaf_nodes = 0

  whale-exec:
    item_23: *** Begin Patch / *** Add File: /app/recover_logs.py / *** End Patch
    item_24: file_change add W:\app\recover_logs.py completed

  validation:
    AssertionError: Expected output file /app/recovered_logs/results.json does not exist
  ```
- Interpretation: The context/action convergence repairs have real benefit: TaskSpace now edits. The remaining failure is not no-patch; it is that the smoke_test node accepted a vacuous pre-check and did not execute the changed artifact or generate the declared output contracts.
- Time: 2026-06-29 11:00

## Hypothesis H-032: validation nodes accept vacuous tests that do not exercise changed artifacts or output contracts
- Claim: After a successful implementation edit, smoke_test/regression_test nodes can run a command that exits 0 without invoking changed artifacts or producing required output contracts. This lets the graph advance despite no task outputs being generated, leading to external public validation failure.
- Parent:
  - H-031
- If true:
  - The transcript should show a successful edit followed by a validation command that only checks or creates environment state.
  - The validation command should not execute the changed file.
  - External public validation should fail because declared output contracts are missing.
- If false:
  - The validation command should have executed the changed artifact and failure should be semantic output mismatch rather than missing files.
- Diagnostic evidence plan:
  - Prediction or clause under test: inspect the post-edit smoke_test command and external validator output.
  - Signal: changed_paths, run_test command text, output_contract paths, validator missing-file assertions.
  - Capture method: parse `whale-exec.jsonl`, `pair-report.md`, and `validation.stdout.log`.
  - Event name or marker:
    - `run_test`
    - `changed_paths`
    - `output_contract`
  - Correlation keys:
    - `item_28`
    - `recover_logs.py`
    - `/app/recovered_logs/results.json`
  - Differentiates from:
    - no-patch
    - patch grammar failure
    - validator infrastructure failure
    - semantic accuracy mismatch after outputs exist
  - Supports if:
    - smoke_test command exits 0 but does not execute the changed file or generate any declared outputs.
  - Refutes if:
    - smoke_test executes the changed artifact and outputs exist before external validation.
- Evidence gate: pending
- Related evidence:
  - E-047
- Conclusion: suspected
- Repair design readiness: needs-design
- Next step: enforce validation commands against changed artifacts and output contracts; validation should run the changed artifact or explicitly block rather than accept environment pre-checks.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-048: validation coverage gate blocks vacuous tests and preserves real validators
- Related hypotheses:
  - H-032
- Direction: supports-repair
- Type: code-change-and-test
- Source: `third_party\codex-cli\codex-rs\core\src\action_map\runtime.rs`
- Prediction or plan link:
  - H-032 repair design: validation commands must exercise changed artifacts or real validators.
- Matched signal:
  - Added `validation_test_coverage_block`.
  - Validation nodes now collect changed artifacts from dependency implementation nodes.
  - Output contract path-like targets are included in recovery evidence.
  - Commands that do not mention changed artifacts are blocked unless they are recognized real validators such as `pytest`, `cargo test`, `npm test`, or `run-tests`.
  - `*** Add File:` is now included in changed artifact extraction.
- Correlation keys:
  - `validation_test_missing_changed_artifact_coverage`
  - `validation_node_blocks_vacuous_test_after_changed_artifact`
  - `task_output_contract_artifact_targets`
- Raw content:
  ```text
  cargo test -p codex-core validation_node_blocks_vacuous_test_after_changed_artifact --lib -- --nocapture = PASS
  cargo test -p codex-core validation_node --lib -- --nocapture = PASS, 15 tests
  cargo test -p codex-core changed_artifact --lib -- --nocapture = PASS, 5 tests
  cargo test -p codex-core taskspace_action_contract --lib -- --nocapture = PASS, 40 tests
  ```
- Interpretation: The vacuous validation class is covered in unit tests without blocking ordinary project validators.
- Time: 2026-06-29 11:20

## Evidence E-049: real rerun now executes changed artifact during validation
- Related hypotheses:
  - H-032
- Direction: supports-repair
- Type: real-task-rerun
- Source:
  - `target\r3-validation-coverage-recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260629-112704-778\pair-001\pair-report.md`
  - `target\r3-validation-coverage-recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260629-112704-778\pair-001\right\artifacts\whale-exec.jsonl`
  - `target\r3-validation-coverage-recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260629-112704-778\pair-001\right\artifacts\whale-exec.stderr.log`
- Prediction or plan link:
  - H-032 If true after repair: smoke_test should execute changed artifact or a real validator instead of a vacuous pre-check.
- Matched signal:
  - TaskSpace changed path is `recover.py`.
  - `item_23` run_test command is `python recover.py`.
  - The run_test fails with a real traceback from `recover.py`, not an empty pre-check.
  - Failure taxonomy remains `agent_patch_wrong`, not `agent_no_patch`.
- Correlation keys:
  - `20260629-112704-778`
  - `item_23`
  - `python recover.py`
  - `FileNotFoundError`
- Raw content:
  ```text
  right / taskspace:
    business_success = False
    exec_exit_code = 1
    public_validation_exit_code = 1
    changed_paths = recover.py
    tool_call_count = 6
    open_leaf_nodes = 1

  whale-exec:
    item_23 run_test command = python recover.py

  run_test output:
    FileNotFoundError: [Errno 2] No such file or directory: './raw_logs/generator.log'
  ```
- Interpretation: H-032 is repaired enough to produce real validation of the changed artifact. The next blocker is failed-validation rework routing: the validation node reads the same missing path again instead of routing back to implementation with the traceback and known `task_deps` evidence.
- Time: 2026-06-29 11:40

## Hypothesis H-033: failed validation does not force rework routing when traceback points to an implementation defect
- Claim: After a changed artifact is executed and fails, TaskSpace validation nodes may continue read/search rediscovery instead of recording the failed validation result and creating/binding an implementation rework node. This loses the opportunity to patch the changed artifact using the traceback plus prior verified input evidence.
- Parent:
  - H-032
- If true:
  - A rerun should show a failed `run_test` result from the changed artifact.
  - The next validation action should be read/search of already known paths rather than state_commit/block/finish into implement_solution.
  - The turn may fail by no-action recovery cap or open validation leaf.
- If false:
  - The failed validation should be recorded and routed into a follow-up implementation node, or the system should block with exact failed validation evidence.
- Diagnostic evidence plan:
  - Prediction or clause under test: inspect actions after `python recover.py` failure in `20260629-112704-778`.
  - Signal: failed test result, next action type, traceback target path, prior verified input evidence path.
  - Capture method: parse `whale-exec.jsonl`, `graph-health.json`, and context projection artifacts.
  - Event name or marker:
    - `validator_failure`
    - `validation_recovery`
    - `TaskSpaceNoActionRecoveryV1`
  - Correlation keys:
    - `recover.py`
    - `./raw_logs/generator.log`
    - `task_deps/generator.log`
  - Differentiates from:
    - no-patch
    - vacuous validation
    - external validator infrastructure failure
    - pure semantic accuracy mismatch after files exist
  - Supports if:
    - the failed validation is followed by repeated missing-path reads and no rework implementation node.
  - Refutes if:
    - a failed validation state commit or implement rework node is created immediately.
- Evidence gate: pending
- Related evidence:
  - E-049
- Conclusion: suspected
- Repair design readiness: needs-design
- Next step: add failed-validation rework policy so validation nodes with failed test/build results cannot continue broad rediscovery; they must state_commit failed result and route to implement_solution or block.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-050: validation recovery gate now preserves executable changed-artifact guidance
- Related hypotheses:
  - H-033
- Direction: supports-repair
- Type: code-change-and-test
- Source:
  - `third_party\codex-cli\codex-rs\core\src\action_map\runtime.rs`
  - `third_party\codex-cli\codex-rs\core\src\session\turn.rs`
- Prediction or plan link:
  - H-033 repair path: blocked validation commands must carry specific recovery guidance into the next request, otherwise the model can fall back to rediscovery.
- Matched signal:
  - `validation_test_coverage_block` now emits executable changed-artifact actions such as `run_test with command `python <artifact>``.
  - The gate message explicitly forbids discovery commands such as `find`, `ls`, `rg`, and `Get-ChildItem` for rediscovering already-known changed artifacts.
  - `turn.rs` extracts `TaskSpaceGateRecoveryV1` from failed `FunctionCallOutput` / `CustomToolCallOutput`.
  - `TaskSpaceNoActionRecoveryV1` now replays the most recent `TaskSpaceGateRecoveryV1` and tells the model to obey `next_valid_actions`.
- Correlation keys:
  - `TaskSpaceGateRecoveryV1`
  - `validation_test_missing_changed_artifact_coverage`
  - `no_action_recovery_preserves_recent_gate_recovery_context`
  - `extracts_gate_recovery_from_blocked_tool_output`
- Raw content:
  ```text
  cargo test -p codex-core validation_node_blocks_vacuous_test_after_changed_artifact --lib -- --nocapture = PASS
  cargo test -p codex-core no_action_recovery --lib -- --nocapture = PASS, 3 tests
  cargo test -p codex-core extracts_gate_recovery_from_blocked_tool_output --lib -- --nocapture = PASS
  cargo test -p codex-core validation_node --lib -- --nocapture = PASS, 16 tests
  cargo test -p codex-core active_context_replacement --lib -- --nocapture = PASS, 113 tests
  cargo build -p codex-cli --bin whale --profile dev-small = PASS
  ```
- Interpretation: The recovery instructions are now carried through the same active-context replacement path that had previously hidden legacy tool outputs, without reintroducing raw legacy TaskSpace history.
- Time: 2026-06-29 12:05

## Evidence E-051: rerun proves gate recovery bridge works but no-action hard stop remains a blocker
- Related hypotheses:
  - H-033
- Direction: supports-and-refines
- Type: real-task-rerun
- Source:
  - `target\r3-gate-recovery-recover-accuracy-log-rerun\runs\terminal_bench__recover-accuracy-log\20260629-120857-814\pair-001\pair-report.md`
  - `target\r3-gate-recovery-recover-accuracy-log-rerun\runs\terminal_bench__recover-accuracy-log\20260629-120857-814\pair-001\right\artifacts\rollout.jsonl`
  - `target\r3-gate-recovery-recover-accuracy-log-rerun\runs\terminal_bench__recover-accuracy-log\20260629-120857-814\pair-001\right\artifacts\taskspace-control-usage.json`
- Prediction or plan link:
  - E-050 should cause no-action recovery to include the exact changed artifact command after a validation coverage block.
- Matched signal:
  - Gate output line 276 says the changed artifact is `/app/recover.py` and gives `next_valid_actions=["run_test with command `python /app/recover.py` ...]`.
  - No-action recovery line 284 replays that `TaskSpaceGateRecoveryV1`.
  - The model still emits `list_files`; with smoke_test no-action cap at 1, the turn fails before another recovery can converge.
- Correlation keys:
  - `20260629-120857-814`
  - `python /app/recover.py`
  - `TaskSpaceNoActionRecoveryV1`
  - `node_policy_violation:smoke_test:list_files`
- Raw content:
  ```text
  right / taskspace:
    outcome_taskspace = wrong
    changed_paths = recover.py
    tool_call_count = 7
    open_leaf_nodes = 1

  line 276:
    next_valid_actions = run_test with command `python /app/recover.py`
  line 284:
    no-action recovery replayed TaskSpaceGateRecoveryV1
  ```
- Interpretation: H-033's gate-context loss is repaired. The remaining blocker is a hard no-action recovery stop that turns one validation-node policy violation into task failure.
- Time: 2026-06-29 12:20

## Hypothesis H-034: validation no-action recovery cap is a hard stop that prevents convergence after recoverable policy violations
- Claim: On validation nodes, `TaskSpaceNoActionRecoveryV1` uses a cap of 1 and returns `None` after the first spent recovery. This makes a recoverable action-contract policy violation terminal, even when the runtime has exact next-valid-action guidance and remaining provider budget.
- Parent:
  - H-033
- If true:
  - A rerun should show correct `TaskSpaceGateRecoveryV1` guidance followed by one policy violation and then turn failure.
  - Making the no-action threshold advisory should allow later provider requests to continue and eventually run the changed artifact.
- If false:
  - The failure would persist even after advisory no-action recovery, or the model would fail for semantic output correctness rather than premature turn stop.
- Diagnostic evidence plan:
  - Prediction or clause under test: compare `20260629-120857-814` before advisory recovery and the next focused rerun after changing no-action cap behavior.
  - Signal: `TaskSpaceNoActionRecoveryV1`, request count progression, final taskspace outcome, public/hidden validation exit codes, and open leaf count.
  - Capture method: parse `pair-report.md`, `whale-exec.jsonl`, `rollout.jsonl`, and `taskspace-control-usage.json`.
  - Event name or marker:
    - `beyond the advisory recovery threshold`
    - `TaskSpaceNoActionRecoveryV1`
    - `provider_request_profile_hint_exceeded`
  - Supports if:
    - the post-change rerun continues past the previous cap and reaches successful validation.
  - Refutes if:
    - the rerun still stops at the same no-action recovery point.
- Evidence gate: satisfied by E-052 and E-053.
- Related evidence:
  - E-051
  - E-052
  - E-053
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: investigate why validation success still needs many redundant recovery turns before finalization.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-052: no-action recovery threshold is advisory, not terminal
- Related hypotheses:
  - H-034
- Direction: supports-repair
- Type: code-change-and-test
- Source: `third_party\codex-cli\codex-rs\core\src\session\turn.rs`
- Prediction or plan link:
  - H-034 repair: recoverable no-action guidance should continue after the advisory threshold instead of ending the turn.
- Matched signal:
  - The previous `return None` branch for generic no-action cap exhaustion was removed.
  - The warning now says recovery continues beyond the advisory threshold.
  - Existing no-action and active-context replacement tests still pass.
- Correlation keys:
  - `TaskSpaceNoActionRecoveryV1 beyond the advisory recovery threshold`
  - `no_action_recovery`
  - `active_context_replacement`
- Raw content:
  ```text
  cargo test -p codex-core no_action_recovery --lib -- --nocapture = PASS, 3 tests
  cargo test -p codex-core active_context_replacement --lib -- --nocapture = PASS, 113 tests
  cargo build -p codex-cli --bin whale --profile dev-small = PASS
  ```
- Interpretation: The generic no-action threshold no longer acts as a hard budget stop. This matches the R3 direction that profiles and recovery thresholds should guide behavior rather than terminate open-ended agent work.
- Time: 2026-06-29 12:25

## Evidence E-053: focused rerun solves recover-accuracy-log after advisory no-action recovery
- Related hypotheses:
  - H-031
  - H-032
  - H-033
  - H-034
- Direction: supports-repair
- Type: real-task-rerun
- Source:
  - `target\r3-noaction-advisory-recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260629-122726-092\pair-001\pair-report.md`
  - `target\r3-noaction-advisory-recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260629-122726-092\pair-001\right\artifacts\whale-exec.jsonl`
  - `target\r3-noaction-advisory-recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260629-122726-092\pair-001\right\artifacts\taskspace-control-usage.json`
- Prediction or plan link:
  - The chain of action-contract, validation coverage, gate-recovery, and advisory recovery repairs should convert `recover-accuracy-log` from no-patch/wrong into a solved focused sample.
- Matched signal:
  - TaskSpace outcome is `solved`.
  - `business_success=True`, `exec_exit_code=0`, `public_validation_exit_code=0`, `hidden_oracle_exit_code=0`.
  - `changed_paths` includes `recover_accuracy.py` plus all required `recovered_logs/*` outputs.
  - `open_leaf_nodes=0`.
  - `taskspace_control_count=4`.
- Correlation keys:
  - `20260629-122726-092`
  - `recover_accuracy.py`
  - `recovered_logs/results.json`
  - `business_success: True`
- Raw content:
  ```text
  right / taskspace:
    business_success = True
    exec_exit_code = 0
    public_validation_exit_code = 0
    hidden_oracle_exit_code = 0
    tool_call_count = 6
    open_leaf_nodes = 0
    changed_paths = recover_accuracy.py, recovered_logs/results.json, recovered_logs/run_1_generator.jsonl, recovered_logs/run_1_judge.jsonl, recovered_logs/run_2_generator.jsonl, recovered_logs/run_2_judge.jsonl, recovered_logs/run_3_generator.jsonl, recovered_logs/run_3_judge.jsonl
  ```
- Interpretation: This is a real focused-task benefit proof, not just a unit-level gate proof. It is still E2-candidate rather than formal E3 because repeats and human review are missing and the external Terminal-Bench validator remains downgraded for official-runner fidelity.
- Time: 2026-06-29 12:40

## Hypothesis H-035: successful validation lacks a runtime-owned closeout path
- Claim: After a smoke_test/regression_test records a successful Test/Build result, TaskSpace still depends on the model to explicitly finish the validation node and emit final_answer. When the model keeps probing or asking for follow-up, this creates long no-action recovery tails even though validation evidence is already sufficient.
- Parent:
  - H-034
- If true:
  - A runtime check after tool drain can finish the validation node once successful validation evidence exists.
  - The next provider request should receive a closeout recovery item and converge to final_candidate without more read/search/validation loops.
- If false:
  - Forced closeout would either not trigger or would leave open_leaf_nodes/non-final state unchanged.
- Diagnostic evidence plan:
  - Prediction or clause under test: compare the failed `20260629-131743-056` focused run against a rerun after adding forced validation closeout.
  - Signal: `TaskSpaceForcedValidationCloseoutV1`, final_candidate request count, business_success, validation exits, open_leaf_nodes, wall_time_ms, and runtime event count.
  - Capture method: parse `pair-report.md`, `whale-exec.jsonl`, `metrics.json`, and `taskspace-control-usage.json`.
  - Supports if:
    - forced closeout fires after successful run_test, final_candidate appears, business_success=true, and open_leaf_nodes=0.
  - Refutes if:
    - validation succeeds but the run still loops without closeout or leaves an open validation node.
- Evidence gate: satisfied by E-054 and E-055.
- Related evidence:
  - E-054
  - E-055
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: keep formal E3 blocked until repeats/human review/official-runner fidelity are addressed.
- Blocker:
  - none for focused E2-candidate proof
- Close reason:
  - not closed

## Evidence E-054: forced validation closeout implementation passes targeted gates
- Related hypotheses:
  - H-035
- Direction: supports-repair
- Type: code-change-and-test
- Source:
  - `third_party\codex-cli\codex-rs\core\src\action_map\runtime.rs`
  - `third_party\codex-cli\codex-rs\core\src\session\mod.rs`
  - `third_party\codex-cli\codex-rs\core\src\session\turn.rs`
- Prediction or plan link:
  - H-035 repair: successful validation evidence should allow runtime-owned closeout after tool drain.
- Matched signal:
  - `force_finish_validation_after_successful_tool` finishes a smoke/regression node when it already has successful Test/Build evidence.
  - `TaskSpaceForcedValidationCloseoutV1` warning records trigger, request count, source node, and result id.
  - `TaskSpaceForcedValidationCloseoutRecoveryV1` instructs the next model response to emit final_answer instead of reading, searching, validating, or creating nodes.
- Correlation keys:
  - `taskspace-forced-validation-closeout-v1`
  - `TaskSpaceForcedValidationCloseoutV1`
  - `validation_success_after_tool_drain`
- Raw content:
  ```text
  cargo test -p codex-core force_finish_validation_after_successful_tool_closes_smoke_node --lib -- --nocapture = PASS
  cargo test -p codex-core validation_node --lib -- --nocapture = PASS, 16 tests
  cargo test -p codex-core active_context_replacement --lib -- --nocapture = PASS, 113 tests
  cargo build -p codex-cli --bin whale --profile dev-small = PASS
  ```
- Interpretation: The runtime can now close a validation node from recorded validation evidence without requiring the model to discover the lifecycle transition.
- Time: 2026-06-29 13:48

## Evidence E-055: focused rerun proves forced validation closeout real benefit
- Related hypotheses:
  - H-035
  - H-034
- Direction: supports-repair
- Type: real-task-rerun
- Source:
  - `target\r3-validation-rework-recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260629-135014-854\pair-001\pair-report.md`
  - `target\r3-validation-rework-recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260629-135014-854\pair-001\right\artifacts\whale-exec.jsonl`
  - `target\r3-validation-rework-recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260629-135014-854\pair-001\right\artifacts\metrics.json`
- Prediction or plan link:
  - H-035 should convert successful validation evidence into closed graph and final_candidate without a long recovery tail.
- Matched signal:
  - TaskSpace outcome is solved: `business_success=True`, `exec_exit_code=0`, `public_validation_exit_code=0`, `hidden_oracle_exit_code=0`.
  - `open_leaf_nodes=0`.
  - `TaskSpaceForcedValidationCloseoutV1` fired at request_count 14/20 with `source_node_id=node-3` and `result_id=result-14`.
  - final_candidate appeared at request_count 15/20.
  - Compared with `20260629-131743-056`, wall time dropped from 808530 ms to 197838 ms and runtime events dropped from 1840 to 308.
- Correlation keys:
  - `20260629-135014-854`
  - `TaskSpaceForcedValidationCloseoutV1`
  - `result-14`
  - `open_leaf_nodes=0`
- Raw content:
  ```text
  previous focused run:
    business_success = False
    public_validation_exit_code = 1
    wall_time_ms = 808530
    taskspace_runtime_event_count = 1840
    request_count = 49/20

  current focused run:
    business_success = True
    public_validation_exit_code = 0
    hidden_oracle_exit_code = 0
    wall_time_ms = 197838
    taskspace_runtime_event_count = 308
    final_candidate at request_count = 15/20
  ```
- Interpretation: The benefit is real for this focused sample, but remains E2-candidate because the harness score is disabled by repeats, manual review, and external validator fidelity requirements.
- Time: 2026-06-29 14:00

## Hypothesis H-036: failed validation needs runtime-owned rework routing
- Claim: When a smoke_test/regression_test records a non-infrastructure failed Test/Build result, the validation node cannot legally perform further read/search/edit work. If block_node only blocks the validation node and clears the binding, the model can keep using the old validation node id and repeatedly hit policy walls instead of entering a repair node.
- Parent:
  - H-035
- If true:
  - Blocking a failed validation node should create and bind an implement_solution rework node with the failed result preview.
  - The map should retain blocked validation evidence while giving the model a legal edit node.
- If false:
  - block_node alone should already produce a valid rework path, or creating a rework node would not change the active binding.
- Diagnostic evidence plan:
  - Prediction or clause under test: unit-level state transition after failed Test/Build result and block_node.
  - Signal: validation node status blocked, current main node kind implement_solution, rework context contains failed result preview.
  - Capture method: targeted runtime unit tests.
  - Supports if:
    - block_node returns with current_main_node_id bound to a new implement_solution node.
  - Refutes if:
    - current_main_node_id remains none or the new node cannot bind due blocked dependency semantics.
- Evidence gate: satisfied by E-056 for code behavior; real-task benefit is not yet proven because the follow-up rerun did not trigger failed-validation rework.
- Related evidence:
  - E-056
- Conclusion: confirmed-at-unit-level
- Repair design readiness: implemented
- Next step: require the next real failed-validation sample to prove the rework node appears in logs.
- Blocker:
  - no real-task trigger observed after implementation
- Close reason:
  - not closed

## Evidence E-056: failed validation block creates a bound rework implementation node
- Related hypotheses:
  - H-036
- Direction: supports-repair
- Type: code-change-and-test
- Source: `third_party\codex-cli\codex-rs\core\src\action_map\runtime.rs`
- Prediction or plan link:
  - H-036 repair: block_main_node should transform failed validation evidence into a legal implementation rework node.
- Matched signal:
  - `block_main_node` detects smoke/regression nodes with non-infra failed Test/Build result.
  - It blocks the validation node, then creates and binds an `implement_solution` node.
  - The rework node context includes the validation node id, failed result id, failure preview, and instruction to rerun validation after fixing.
  - It intentionally does not depend on the blocked validation node because DAG readiness treats only completed dependencies as ready.
- Correlation keys:
  - `block_validation_node_allows_failed_validator_result`
  - `active_map_detects_blocked_validation_result`
  - `failed test/build result`
- Raw content:
  ```text
  cargo test -p codex-core block_validation_node_allows_failed_validator_result --lib -- --nocapture = PASS
  cargo test -p codex-core active_map_detects_blocked_validation_result --lib -- --nocapture = PASS
  cargo test -p codex-core validation_node --lib -- --nocapture = PASS, 16 tests
  cargo test -p codex-core active_context_replacement --lib -- --nocapture = PASS, 113 tests
  cargo build -p codex-cli --bin whale --profile dev-small = PASS
  ```
- Interpretation: The structural routing bug is repaired at runtime state-machine level. This evidence is not yet a real-task benefit proof because the current successful rerun used forced closeout instead of the failed-validation rework path.
- Time: 2026-06-29 14:03

## Hypothesis H-037: local infra validation failure must still route changed artifacts to rework
- Claim: When a validation node fails because the local host shell or validator infrastructure is incompatible, but the failed validation is attached to changed artifacts that were never proven by a platform-compatible run, the runtime should not close the path as pure infra-blocked. It should create an implement_solution rework node so the changed artifact can be patched or executed with compatible syntax.
- Parent:
  - H-036
- Evidence gate: satisfied by unit proof and focused real-run structural proof.
- Related evidence:
  - E-057
  - E-058
- Conclusion: confirmed
- Repair design readiness: implemented
- Close reason:
  - real focused run showed rework node creation after local-infra validation failure.

## Evidence E-057: local infra validation failure now creates a rework path for unproven changed artifact
- Related hypotheses:
  - H-037
- Direction: supports-repair
- Type: code-change-and-test
- Source:
  - `third_party\codex-cli\codex-rs\core\src\action_map\runtime.rs`
- Matched signal:
  - `validation_node_local_infra_unvalidated_artifact_result` detects local validator infrastructure failure plus dependency changed artifacts.
  - `block_main_node` routes that state into an implement_solution rework node instead of treating it as a terminal infra-only blocker.
- Raw content:
  ```text
  cargo test -p codex-core local_infra_validation_block_routes_unvalidated_changed_artifact_to_rework --lib -- --nocapture = PASS
  cargo test -p codex-core validation_node --lib -- --nocapture = PASS, 16 tests
  cargo build -p codex-cli --bin whale --profile dev-small = PASS
  ```
- Interpretation: Unit-level behavior confirms the local-infra failure path no longer loses the changed artifact proof obligation.
- Time: 2026-06-29 18:40

## Evidence E-058: focused multi-source rerun proves local-infra rework routing but exposes prompt/review blockers
- Related hypotheses:
  - H-037
- Direction: supports-repair-and-finds-next-blocker
- Type: real-task-rerun
- Source:
  - `target\r3-multisource-after-local-infra-rework-gib16\runs\terminal_bench__multi-source-data-merger\20260629-184450-157\pair-001\pair-report.md`
- Matched signal:
  - Previous current-HEAD run stopped after `state_commit invalid_results=["result-11"]` with only `merge_users.py` changed and no rework.
  - After repair, the same focused sample created more graph structure (`nodes=5` instead of `nodes=3`), proving the runtime entered a rework path.
  - The sample still failed because the prompt kept steering the implement rework node back toward state_commit/block behavior.
- Interpretation: H-037 was structurally proven in a real run, but follow-up prompt specialization was required.
- Time: 2026-06-29 18:55

## Hypothesis H-038: rework prompt must be specialized by current node kind
- Claim: Recent-tool-output guidance for local validator infrastructure failures was correct on validation nodes, but wrong after runtime already moved into implement_solution rework. In rework, the next valid action should be patching or platform-compatible execution, not repeating state_commit/block for the old validation failure.
- Parent:
  - H-037
- Evidence gate: satisfied by unit proof and real-run differential evidence.
- Related evidence:
  - E-059
- Conclusion: confirmed
- Repair design readiness: implemented
- Close reason:
  - focused rerun after prompt repair moved past the repeated local-infra state_commit pattern and exposed the next runtime blocker.

## Evidence E-059: implement rework prompt no longer repeats local-infra block instruction
- Related hypotheses:
  - H-038
- Direction: supports-repair
- Type: code-change-and-test
- Source:
  - `third_party\codex-cli\codex-rs\core\src\session\turn.rs`
  - `target\r3-multisource-after-rework-prompt-gib16\runs\terminal_bench__multi-source-data-merger\20260629-191712-138\pair-001\right\artifacts\rollout.jsonl`
- Matched signal:
  - `prepare_taskspace_action_contract_prompt_items_for_node` now receives `current_node_kind`.
  - On `implement_solution`, local-infra recovery text says the current node is implementation rework and the next action is patching or platform-compatible execution.
  - Unit proof:
    `cargo test -p codex-core action_contract_prompt_guides_platform_compatible_rework_after_recorded_local_infra --lib -- --nocapture = PASS`
- Interpretation: Prompt-level recovery now matches the active node responsibility instead of leaking validation-node closeout guidance into implementation rework.
- Time: 2026-06-29 19:20

## Hypothesis H-039: observed old diffs must not be reattributed to a new rework node
- Claim: After a rework node is created, the session-level observed edit recorder can see an old working-tree diff and incorrectly record it as a successful edit on the new node. That prematurely closes implementation rework without a real patch.
- Parent:
  - H-038
- Evidence gate: satisfied by unit/build proof and real-run negative proof.
- Related evidence:
  - E-060
- Conclusion: confirmed
- Repair design readiness: implemented
- Close reason:
  - focused rerun after the fix no longer auto-completed the new rework node from the old diff.

## Evidence E-060: active-map edit guard prevents stale diff attribution
- Related hypotheses:
  - H-039
- Direction: supports-repair
- Type: code-change-and-real-run
- Source:
  - `third_party\codex-cli\codex-rs\core\src\session\turn.rs`
  - `third_party\codex-cli\codex-rs\core\src\session\mod.rs`
  - `third_party\codex-cli\codex-rs\core\src\action_map\runtime.rs`
  - `target\r3-multisource-after-diff-attribution-gib15\runs\terminal_bench__multi-source-data-merger\20260629-194145-129\pair-001\pair-report.md`
- Matched signal:
  - `record_taskspace_observed_implement_edit` now refuses to attribute an observed diff when the active map already has a successful edit action.
  - Focused rerun after the fix left the rework node running instead of falsely completing it, proving stale-diff attribution was removed.
  - That run timed out later because a separate unreviewed blocker result prevented ordinary rework actions.
- Raw content:
  ```text
  cargo test -p codex-core action_contract_prompt_guides --lib -- --nocapture = PASS, 5 tests
  cargo test -p codex-core local_infra_validation_block_routes_unvalidated_changed_artifact_to_rework --lib -- --nocapture = PASS
  cargo build -p codex-cli --bin whale --profile dev-small = PASS
  ```
- Interpretation: The stale attribution bug is repaired; the next blocker moved to lifecycle-result review semantics.
- Time: 2026-06-29 20:05

## Hypothesis H-040: validation blocker result must not block its own active rework input
- Claim: When a failed validation node is blocked and runtime creates an active implement_solution rework node, the blocker result is an input to the rework path. If the ordinary-work preflight requires that blocker result to be reviewed before allowing rework tools, the implementation node deadlocks.
- Parent:
  - H-039
- Evidence gate: satisfied by unit proof and focused real-run benefit proof.
- Related evidence:
  - E-061
  - E-062
- Conclusion: confirmed
- Repair design readiness: implemented
- Close reason:
  - focused real run moved from agent_exec_timeout to completed validation lifecycle after this repair.

## Evidence E-061: active rework can edit while origin validation blocker remains unreviewed
- Related hypotheses:
  - H-040
- Direction: supports-repair
- Type: code-change-and-test
- Source:
  - `third_party\codex-cli\codex-rs\core\src\action_map\runtime.rs`
- Matched signal:
  - `block_main_node` now stamps the auto-created rework node with `origin_node_id` pointing at the blocked validation node.
  - `validate_lifecycle_result_reviewed` allows the origin validation blocker as active rework input for ordinary work, while final-response readiness remains strict.
  - Unit proof:
    `cargo test -p codex-core blocked_validation_rework_can_edit_without_reviewing_blocker_result --lib -- --nocapture = PASS`
- Interpretation: The deadlock is fixed at runtime state-machine level.
- Time: 2026-06-29 20:18

## Evidence E-062: focused multi-source rerun proves blocker deadlock removal
- Related hypotheses:
  - H-040
- Direction: supports-repair-and-finds-next-blocker
- Type: real-task-rerun
- Source:
  - `target\r3-multisource-after-rework-blocker-input-gib15\runs\terminal_bench__multi-source-data-merger\20260629-201802-132\pair-001\pair-report.md`
  - `target\r3-multisource-after-rework-blocker-input-gib15\runs\terminal_bench__multi-source-data-merger\20260629-201802-132\pair-001\right\artifacts\metrics.json`
- Matched signal:
  - Previous run: `outcome_taskspace=agent_exec_timeout`, `exec_timed_out=True`, `right_validation_lifecycle_stage=unknown`, `open_leaf_nodes=1`.
  - Current run: `exec_timed_out=False`, `right_validation_lifecycle_stage=tests_completed`, `tests_started_seen=True`, `tests_completed_seen=True`, `open_leaf_nodes=0`.
  - The remaining failure changed to implementation correctness / action schema issues, not the unreviewed blocker deadlock.
- Interpretation: This is a real benefit proof for H-040. It does not prove task solved; it proves the deadlock class is removed.
- Time: 2026-06-29 20:40

## Hypothesis H-041: action-contract lifecycle aliases must normalize before taskspace_control serde
- Claim: The action-contract layer allowed semantically valid TaskSpace lifecycle actions but passed common fields such as `label`, `child_name`, `description`, and top-level/current node ids without converting them to runtime-required fields. This caused `missing field node_id/title/context_summary` loops.
- Parent:
  - H-040
- Evidence gate: satisfied by unit proof; real rerun after the first normalization showed partial repair and exposed additional aliases; disk space prevented a final rerun after the expanded alias set.
- Related evidence:
  - E-063
  - E-064
- Conclusion: partially-confirmed
- Repair design readiness: implemented
- Blocker:
  - D:\ free space dropped below the safe focused-run threshold after build and previous reruns.
- Close reason:
  - not closed; needs final real rerun when disk space is available.

## Evidence E-063: action-contract control args now normalize block/create/bind lifecycle aliases
- Related hypotheses:
  - H-041
- Direction: supports-repair
- Type: code-change-and-test
- Source:
  - `third_party\codex-cli\codex-rs\core\src\session\turn.rs`
- Matched signal:
  - `block_node` fills missing `node_id` from provider snapshot and maps `reason|summary|result` to `blocker_summary`.
  - `create_node` maps `node_kind|child_kind`, `node_title|label|name|child_name`, and `description|summary|objective` to runtime fields.
  - If there is an existing task but no active node, `create_node` defaults `bind_current=true`.
  - `bind_node` without `node_id` but with node-kind/title-like fields is rewritten to `create_node`.
- Raw content:
  ```text
  cargo test -p codex-core action_contract_control_ --lib -- --nocapture = PASS, 4 tests
  cargo test -p codex-core action_contract_prompt_guides --lib -- --nocapture = PASS, 5 tests
  cargo build -p codex-cli --bin whale --profile dev-small = PASS
  ```
- Interpretation: The schema-normalization blocker is repaired at targeted unit level.
- Time: 2026-06-29 21:15

## Evidence E-064: partial real-run after initial control normalization removed engineering unclean but still missed aliases
- Related hypotheses:
  - H-041
- Direction: mixed
- Type: real-task-rerun
- Source:
  - `target\r3-multisource-after-control-normalization-gib14\runs\terminal_bench__multi-source-data-merger\20260629-204947-673\pair-001\pair-report.md`
  - `target\r3-multisource-after-control-normalization-gib14\runs\terminal_bench__multi-source-data-merger\20260629-204947-673\pair-001\right\artifacts\rollout.jsonl`
- Matched signal:
  - `engineering_unclean=False`, `active_sentinel_warning_count=0`, and validation lifecycle reached `tests_completed`.
  - However TaskSpace still ended `agent_no_patch`, with repeated `missing field title/context_summary/node_id` on create/bind actions using aliases not covered by the first normalization pass.
  - Follow-up code added alias coverage for `label`, `child_name`, `child_kind`, `objective`, and default bind-current behavior.
- Interpretation: The real run proves the repair direction, but the expanded alias set still needs a final rerun. Disk space is the current external blocker.
- Time: 2026-06-29 21:25
## Hypothesis H-042: validation closeout must require semantic validation success, not only tool success
- Claim: A validation tool call can exit with `tool_success=true` while its output proves the task artifact was not generated. If TaskSpace force-closes validation on tool success alone, it records a false pass and lets an invalid final answer reach external scoring.
- Parent:
  - H-041
- Evidence gate: satisfied by unit tests and focused real-run differential evidence.
- Related evidence:
  - E-065
  - E-066
- Conclusion: confirmed
- Repair design readiness: implemented
- Close reason:
  - focused rerun after repair no longer treated `No source files found. Exiting.` as a validation pass; the failure moved to a rework/open-leaf state instead of a false final pass.

## Evidence E-065: focused rerun exposed false validation closeout on semantic failure
- Related hypotheses:
  - H-042
- Direction: supports-root-cause
- Type: real-task-rerun
- Source:
  - `target\r3-multisource-after-rework-chain-review-gate\runs\terminal_bench__multi-source-data-merger\20260629-235542-254\pair-001\right\artifacts\whale-exec.jsonl`
  - `target\r3-multisource-after-rework-chain-review-gate\runs\terminal_bench__multi-source-data-merger\20260629-235542-254\pair-001\right\artifacts\validation.stdout.log`
- Matched signal:
  - `python merge_users.py` exited 0 but printed `Warning: /data/source_a/users.json not found` and `No source files found. Exiting.`
  - Runtime emitted `TaskSpaceForcedValidationCloseoutV1`.
  - External validation later failed because `/app/merged_users.parquet` and `/app/conflicts.json` did not exist.
- Interpretation: The old closeout rule conflated shell command success with validation success.
- Time: 2026-06-30 00:20

## Evidence E-066: semantic validation gate blocks the false-positive closeout
- Related hypotheses:
  - H-042
- Direction: supports-repair
- Type: code-change-and-real-run
- Source:
  - `third_party\codex-cli\codex-rs\core\src\action_map\runtime.rs`
  - `third_party\codex-cli\codex-rs\core\src\session\turn.rs`
  - `target\r3-multisource-after-semantic-validation-gate\runs\terminal_bench__multi-source-data-merger\20260630-002557-891\pair-001\pair-report.md`
  - `target\r3-multisource-after-semantic-validation-gate\runs\terminal_bench__multi-source-data-merger\20260630-002557-891\pair-001\right\artifacts\whale-exec.jsonl`
- Matched signal:
  - `node_result_is_successful_validation` now requires test/build `tool_success=true` and rejects strong failure markers such as `no source files found`, `FileNotFoundError`, `failed`, and `no such file`.
  - `trace_tags_for` marks such test/build outputs as `validator_failure` rather than `validator_success`.
  - Focused rerun after the fix did not close the false-positive validation; it ended with `exec_exit_code=1`, `open_leaf_nodes=1`, and no false final pass.
- Raw content:
  ```text
  cargo test -p codex-core force_finish_validation_ --lib -- --nocapture = PASS, 2 tests
  cargo test -p codex-core action_contract_control_ --lib -- --nocapture = PASS, 5 tests
  cargo build -p codex-cli --bin whale --profile dev-small = PASS
  ```
- Interpretation: Real benefit is increased validation truthfulness, not sample solve-rate.
- Time: 2026-06-30 00:45

## Hypothesis H-043: TaskSpace action-contract run_test commands need host-shell normalization
- Claim: In Windows PowerShell 5.1, bash-style `&&` command chains fail before testing the changed artifact. Relying on the model to manually learn host-shell syntax causes repeated validation-infra recovery and read loops.
- Parent:
  - H-042
- Evidence gate: unit proof satisfied; real-run proof still pending because the follow-up rerun exceeded the outer tool timeout.
- Related evidence:
  - E-067
- Conclusion: partially-confirmed
- Repair design readiness: implemented
- Blocker:
  - Needs another focused rerun with a longer outer timeout or smaller harness timeout.

## Evidence E-067: run_test now normalizes top-level `&&` for Windows PowerShell
- Related hypotheses:
  - H-043
- Direction: supports-repair-at-unit-level
- Type: code-change-and-test
- Source:
  - `third_party\codex-cli\codex-rs\core\src\session\turn.rs`
- Matched signal:
  - `normalize_taskspace_action_contract_test_command` now calls host-shell normalization.
  - On Windows, top-level `a && b` becomes `a; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }; b`.
  - The splitter ignores `&&` inside single or double quoted strings.
- Raw content:
  ```text
  cargo test -p codex-core taskspace_action_contract_run_test_ --lib -- --nocapture = PASS, 3 tests
  cargo test -p codex-core force_finish_validation_ --lib -- --nocapture = PASS, 2 tests
  cargo test -p codex-core action_contract_control_ --lib -- --nocapture = PASS, 5 tests
  cargo fmt -p codex-core = PASS
  cargo build -p codex-cli --bin whale --profile dev-small = PASS
  ```
- Real-run note:
  - `target\r3-multisource-after-shell-chain-normalization` was started but the outer command timed out after 20 minutes. Residual benchmark PowerShell/validator processes were stopped manually, so this run is not counted as benefit evidence.
- Time: 2026-06-30 01:15

## Evidence E-068: focused diagnostic rerun solves multi-source-data-merger after the validation/rework repairs
- Related hypotheses:
  - H-041
  - H-042
  - H-043
- Direction: supports-combined-real-benefit
- Type: real-task-rerun
- Source:
  - `target\r3-multisource-after-shell-chain-normalization-rerun4\runs\terminal_bench__multi-source-data-merger\20260630-011704-254\pair-001\pair-report.md`
  - `target\r3-multisource-after-shell-chain-normalization-rerun4\runs\terminal_bench__multi-source-data-merger\20260630-011704-254\pair-001\right\artifacts\taskspace-control-usage.json`
  - `target\r3-multisource-after-shell-chain-normalization-rerun4\runs\terminal_bench__multi-source-data-merger\20260630-011704-254\pair-001\right\artifacts\graph-health.json`
- Matched signal:
  - Pair is valid and engineering-clean: `valid_pair=True`, `engineering_unclean=False`, `engineering_unclean_reasons=none`.
  - Both modes solved, including TaskSpace: `outcome_taskspace=solved`, `business_success=True`, `exec_exit_code=0`, `exec_timed_out=False`.
  - External validation passed: `public_validation_exit_code=0`, `hidden_oracle_exit_code=0`.
  - TaskSpace lifecycle converged: `right_validation_lifecycle_stage=tests_completed`, `right_tests_started_seen=True`, `open_leaf_nodes=0`, `edge_order_violations=0`.
  - TaskSpace produced final artifacts: `changed_paths=conflicts.json, merge_users.py, merged_users.parquet`.
  - Action-contract controls were accepted: `taskspace_control_count=4`, `action_contract_taskspace_control_count=4`, `parse_errors=0`.
- Limitation:
  - This rerun used `-AllowStaleWhaleBin` because Cargo did not rewrite an unchanged binary timestamp after commit; it is diagnostic evidence, not E3 evidence.
  - `reported_evidence_level=E2-candidate`, `failure_taxonomy=audit_unclean`, and utility scoring remained disabled because repeats and human audit were not complete.
  - The model did not issue a `&&` chained run_test in this rerun, so H-043 direct real-run proof remains pending; H-043 is still unit-confirmed only.
- Interpretation: The combined R3 validation/rework repairs have real task-level benefit on `multi-source-data-merger`; the sample moved from timeout/false validation/open-leaf states to solved. This does not yet establish E3 aggregate utility.
- Time: 2026-06-30 01:45

## Hypothesis H-044: R3 engineering closeout needs first-class binary attestation, marker generation, and current-HEAD gate coverage
- Claim: R3 cannot be called engineering-complete while Whale binary health depends on mtime-only checks, code-complete/user-approval markers are hand-written fixtures, or current-HEAD non-agent gates omit the new wrapper and marker scripts.
- Parent:
  - H-041
  - H-042
  - H-043
- Evidence gate: satisfied by code changes, self-tests, current-HEAD non-agent gates, and start-gate output.
- Related evidence:
  - E-069
  - E-070
  - E-071
- Conclusion: confirmed
- Repair design readiness: implemented
- Close reason:
  - `WhaleBinaryHealth` now accepts an attested Cargo no-op binary only when the attestation binds binary sha, repo root, and Codex source commit.
  - `write-v005-markers.ps1` creates first-class code-complete markers and refuses to create user-approval markers without explicit approval flags.
  - Current-HEAD non-agent gates pass with `external_wrapper_fixture` and `marker_writer_fixture` included.
  - E3 start gate is correctly blocked only by missing `v005_user_approval`, which is a user/process gate rather than a code gap.

## Evidence E-069: Whale binary health no longer requires unsafe stale override for attested no-op builds
- Related hypotheses:
  - H-044
- Direction: supports-repair
- Type: code-change-and-test
- Source:
  - `scripts\taskspace-benchmark\lib\harness-health.ps1`
  - `scripts\taskspace-benchmark\write-whale-binary-attestation.ps1`
  - `scripts\taskspace-benchmark\test-external-wrapper-harness.ps1`
  - `target\r3-engineering-closeout-attested-planonly\whale-binary-preflight-health.json`
- Matched signal:
  - Stale fake whale binaries still fail direct health and wrapper checks when no attestation is present.
  - Matching attestation converts the same stale-mtime case into a pass with `build_attestation_status=pass`.
  - Real runner preflight passed without `-AllowStaleWhaleBin` for `D:\BuildCache\whalecode\cargo-target\dev-small\whale.exe`.
- Raw content:
  ```text
  powershell -File scripts\taskspace-benchmark\test-external-wrapper-harness.ps1 = PASS
  cargo build -p codex-cli --bin whale --profile dev-small = PASS
  target\r3-engineering-closeout-attested-planonly\whale-binary-preflight-health.json:
    status = pass
    stale_for_codex_source = false
    build_attestation_status = pass
  ```
- Interpretation: The previous `AllowStaleWhaleBin` diagnostic dependency has a systematic identity-based replacement.
- Time: 2026-06-30 02:35

## Evidence E-070: v0.0.5 markers are generated by a reusable script and covered by gate fixtures
- Related hypotheses:
  - H-044
- Direction: supports-repair
- Type: code-change-and-test
- Source:
  - `scripts\taskspace-benchmark\write-v005-markers.ps1`
  - `scripts\taskspace-benchmark\test-v005-marker-writer.ps1`
  - `scripts\taskspace-benchmark\build-v005-non-agent-gates.ps1`
  - `scripts\taskspace-benchmark\lib\e3-start-gate.ps1`
  - `scripts\taskspace-benchmark\write-release-decision.ps1`
- Matched signal:
  - Code-complete marker generation binds `head`, `task_list_hash`, `source_version`, `profile_hash`, `sample_set_id`, and test-output sha.
  - User-approval marker generation requires explicit `-ApproveFullE3` and `-ApprovalSource`.
  - Non-agent gates now require `external_wrapper_fixture` and `marker_writer_fixture`.
- Raw content:
  ```text
  powershell -File scripts\taskspace-benchmark\test-v005-marker-writer.ps1 = PASS
  powershell -File scripts\taskspace-benchmark\test-v005-non-agent-gates-builder.ps1 = PASS
  powershell -File scripts\taskspace-benchmark\test-e3-start-gate.ps1 = PASS
  powershell -File scripts\taskspace-benchmark\test-release-decision.ps1 = PASS
  ```
- Interpretation: R3-F no longer depends on hand-written marker fixtures for engineering closeout.
- Time: 2026-06-30 02:40

## Evidence E-071: current-HEAD R3 engineering gates pass and full E3 is blocked only by user approval
- Related hypotheses:
  - H-044
- Direction: supports-closeout
- Type: gate-run
- Source:
  - `target\r3-engineering-closeout-non-agent-gates-head-4adadcc\v005-non-agent-gates.json`
  - `target\r3-engineering-closeout-markers\v005-code-complete.json`
  - `target\r3-engineering-closeout-start-gate\e3-start-gate.json`
  - `docs\v0.0.5\build-R3\09-r3-engineering-closeout.md`
- Matched signal:
  - Current HEAD: `4adadcc94`.
  - Non-agent gates pass for provider hook, runtime budget response, budget quality impact, active context replacement, state commit displacement, spawn budget, request phase attribution, release decision fixture, start gate fixture, external wrapper fixture, and marker writer fixture.
  - Code-complete marker status is pass.
  - Start gate status is `blocked_for_full_e3` with `v005_non_agent_gates=pass`, `v005_code_complete=pass`, and `v005_user_approval=blocked / v005_user_approval_missing`.
- Raw content:
  ```text
  task_list_hash = f603bd25c787f7142a756994e2b773f73ac36ad99141f2d18018462e6a4950fa
  sample_set_id = terminal-bench_E3-P0_3_5
  profile_hash = 53dc5d28741f87ad36b5a714d7971a471da6ff83f98e8ede6e0b82efad376861
  source_version = 1a6ffa9674b571da0ed040c470cb40c4d85f9b9b
  ```
- Interpretation: R3 engineering code complete is true. Full E3 remains intentionally blocked by the explicit user-approval marker, not by an unresolved engineering bug.
- Time: 2026-06-30 02:45

## Evidence E-072: post-closeout light experiment shows TaskSpace patch recovery still loses a trivial one-line fix
- Related hypotheses:
  - H-044
- Direction: limits-benefit-claim
- Type: light-effect-experiment
- Source:
  - `target\r3-light-effect-single-file-20260630-024154\single-file-fast-fix\20260630-024155-720\pair-001\pair-report.md`
  - `target\r3-light-effect-single-file-20260630-024154\single-file-fast-fix\20260630-024155-720\pair-001\right\artifacts\whale-exec.jsonl`
  - `target\r3-light-effect-single-file-20260630-024154\single-file-fast-fix\20260630-024155-720\pair-001\right\artifacts\active-context-replacement-report.json`
  - `docs\v0.0.5\build-R3\10-light-effect-experiment.md`
- Matched signal:
  - Standard solved the `single-file-fast-fix` pair; TaskSpace ended `wrong` with `agent_no_patch`.
  - TaskSpace correctly identified the fix (`round(..., 1)` should become `round(..., 2)`) but failed to land the edit.
  - First `apply_patch` omitted `*** Begin Patch`; second targeted `tax_calc.py` instead of `src/tax_calc.py`, causing context failure.
  - Context replacement and cache remained healthy: `replacement_confirmed=true`, `legacy_taskspace_history_present=false`, `raw_taskspace_control_history_tokens=0`, `request_2_plus_hit_rate=0.985235`.
- Raw content:
  ```text
  reported_evidence_level = E1
  included_in_utility_aggregate = false
  valid_pair = true
  utility_direction = standard_better
  outcome_standard = solved
  outcome_taskspace = wrong
  failure_taxonomy = agent_no_patch
  taskspace_wall_time_ratio = 1.63
  taskspace_tool_call_ratio = 0.71
  ```
- Interpretation: R3 engineering closeout is not disproved, but the lightweight effect signal is negative. The next benefit-oriented repair should target apply_patch failure recovery and path normalization, not more context or tighter budget controls.
- Time: 2026-06-30 02:50

## Hypothesis H-045: tool result correctness is split across several non-identical TaskSpace paths
- Status: unverified
- Parent: P-001
- Claim: After the standard direct-tool result path was aligned to `ToolOutput::to_response_item`, TaskSpace still has multiple tool-call or tool-return paths that can bypass, summarize, filter, or reclassify the model-visible result before the agent sees it again.
- Layer: diagnostic
- Factor relation: all_of
- Depends on:
  - H-044
- Rationale:
  - Recent focused samples show different symptoms around `apply_patch` failure feedback, `run_test` policy rejection, large-output log growth, and map control loops. These symptoms can share a broader cause family: TaskSpace has more than one result propagation path, and not all of them are proven equivalent to standard mode.
- Falsifiable predictions:
  - If true: code inspection will identify at least two tool result paths that do not trivially share the same `ToolOutput -> ResponseInputItem -> provider payload -> TaskSpace map` sequence.
  - If false: all TaskSpace tool execution, action-contract execution, nested tool execution, tool errors, and projected-history summaries are generated from one standard response item and verified in provider-visible payload tests.
- Diagnostic evidence plan:
  - Prediction or clause under test: audit current source paths for direct tool results, action-contract internal tool calls, action-contract compile rejections, active projection filtering, code-mode nested tools, MCP/tool-search outputs, multi-agent outputs, and large-output references.
  - Signal: source code locations and focused run artifacts.
  - Capture method: inspect `turn.rs`, `parallel.rs`, `registry.rs`, `context.rs`, code-mode and multi-agent handlers, plus the three-sample sweep artifacts.
  - Event name or marker:
    - `ToolOutput::to_response_item`
    - `TaskSpaceActionContractRecentToolOutputsV1`
    - `ProviderVisibleHistoryAction::Omit`
  - Correlation keys:
    - `target\r3-tools-result-sweep-20260630-3samples`
  - Differentiates from:
    - single apply_patch grammar failure
    - model-only task-solving error
  - Supports if:
    - multiple code paths can transform or drop tool feedback differently before model resampling.
  - Refutes if:
    - every path is shown to record and project the same bounded response item with focused tests.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: pending
- Related evidence:
  - E-073
- Conclusion: unverified
- Repair design readiness: blocked until the risky paths are converted into a concrete coverage matrix.
- Next step: build a tool-return path matrix and classify each path as covered, risky, or out-of-scope before changing code.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-073: source audit finds additional tool-return paths not proven equivalent to standard mode
- Related hypotheses:
  - H-045
- Direction: supports
- Type: code-location
- Source:
  - `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/context.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/code_mode/mod.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_common.rs`
- Prediction or plan link:
  - H-045 predicts multiple non-identical result propagation paths.
- Matched signal:
  - Direct successful tools record TaskSpace map previews through `tool_output_model_visible_preview(...)`, but direct tool errors use `action_map_tool_error_preview(&err)`.
  - Action-contract internal tools synthesize `ResponseItem` calls and outputs inside `turn.rs`, and action-contract parse/policy rejections are stored as recovery text rather than tool outputs.
  - Action-contract recent-output summarization only considers `FunctionCallOutput` and `CustomToolCallOutput`, while active projection separately omits legacy TaskSpace outputs and large raw outputs.
  - Code-mode nested tools use `ToolCallSource::CodeMode`, which `should_attribute_taskspace_tool` excludes from TaskSpace map attribution.
  - Multi-agent outputs use JSON text wrappers and are also excluded from the direct TaskSpace tool attribution list.
- Correlation keys:
  - none
- Raw content:
  ```text
  parallel.rs:
  - Ok(result) -> tool_output_model_visible_preview(result.result.as_ref(), ...)
  - Err(err) -> action_map_tool_error_preview(&err)
  - should_attribute_taskspace_tool returns false for ToolCallSource != Direct and excludes spawn/wait/taskspace_control-style tools.

  turn.rs:
  - response_input_for_taskspace_action_tool_error(...) manually builds FunctionCallOutput/CustomToolCallOutput.
  - prepare_taskspace_action_contract_prompt_items_for_node(...) builds TaskSpaceActionContractRecentToolOutputsV1 from selected recent tool outputs.
  - provider_visible_history_action(...) omits LegacyTaskspaceToolOutput and LargeRawToolOutput when active projection is present.

  code_mode/mod.rs:
  - nested tools call handle_tool_call_with_source(... ToolCallSource::CodeMode ...) and return code_mode_result() rather than a normal provider-visible tool output.
  ```
- Interpretation: The already-fixed standard direct path is not enough to prove global tool feedback correctness. The remaining risk is architectural coverage: each path needs a test proving model-visible feedback, provider payload inclusion or intentional projection, and TaskSpace map recording.
- Time: 2026-06-30 04:55

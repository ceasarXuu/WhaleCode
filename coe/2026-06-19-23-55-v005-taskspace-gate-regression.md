# Problem P-001: v0.0.5 TaskSpace P0 diagnostic regression
- Status: root cause identified
- Created: 2026-06-19 23:55
- Updated: 2026-06-20 00:02
- Objective: Determine why TaskSpace raw success dropped to 0/3 in `terminal-bench_E3-P0_3_1` after the v0.0.5 completion work.
- Symptoms:
  - `terminal-bench_E3-P0_3_1` reports TaskSpace raw success 0/3 while Standard is 2/3.
  - `processing-pipeline` previously succeeded under TaskSpace in `_3_2` but failed in `_3_1`.
  - TaskSpace stderr contains repeated runtime gate errors such as no ready node, detached inspect denial, broad inspect delegation debt, unreviewed results, and missing edit/test evidence.
- Expected behavior:
  - TaskSpace should preserve or improve raw success on P0 diagnostic samples while lowering cost.
  - Runtime gates should provide a reachable recovery path instead of trapping the agent in contradictory state requirements.
- Actual behavior:
  - TaskSpace fails or appears to fail all three `_3_1` P0 samples.
  - At least one clean regression sample, `processing-pipeline`, produces no file edits and ends with `/task-reborn`.
- Impact:
  - v0.0.5 cannot proceed to formal `terminal-bench_E3-P0_3_5` closure.
  - Cost-control evidence is confounded by runtime gate loops.
- Reproduction:
  - Compare `target\terminal-bench_E3-P0_3_2-v005-20260619-variant` with `target\terminal-bench_E3-P0_3_1-v005-20260619-diagnostic-4`.
  - Inspect TaskSpace side `whale-exec.stderr.log`, `pair-report.md`, `metrics.json`, `git-diff.patch`, and validator logs for the P0 samples.
- Environment:
  - Repository: `D:\whalecode-alpha`
  - Branch: `whalecode-alpha`
  - `_3_1` diagnostic repo commit recorded in report: `25fe8a9eeafacd286cdf791f35e412681f65621f`
  - `_3_1` run root: `target\terminal-bench_E3-P0_3_1-v005-20260619-diagnostic-4`
  - `_3_2` run root: `target\terminal-bench_E3-P0_3_2-v005-20260619-variant`
- Known facts:
  - E-001
  - E-002
  - E-003
  - E-004
  - E-005
  - E-006
  - E-007
  - E-008
  - E-009
  - E-010
- Ruled out:
  - H-003: provider attribution fallback removal is not the direct cause of the observed no-ready-node loop.
- Fix criteria:
  - Root cause is confirmed against a regression sample where prior TaskSpace success becomes current TaskSpace failure.
  - A future fix must make `processing-pipeline` TaskSpace produce the expected edits and pass visible validation without broad gate loops.
  - A future fix must preserve gate evidence quality without introducing contradictory recovery requirements.
- Current conclusion: The clean `processing-pipeline` regression is caused by an unreachable TaskSpace recovery path. A completed/broad inspect path creates unresolved delegation debt that blocks ordinary work, while the narrow-inspect fanout guard rejects the detached inspect nodes needed to satisfy that debt; generic gate wrapping then exposes an empty `TaskSpaceGateRecoveryV1`, so the model receives no executable recovery path and eventually falls into `/task-reborn`. Validator environment noise is a separate confound for `recover-accuracy-log`, but it does not explain the clean no-edit regression.
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: v0.0.5 gate hardening introduced an unreachable recovery path
- Status: confirmed
- Parent: P-001
- Claim: Recent v0.0.5 gate hardening made TaskSpace runtime constraints mutually incompatible in common P0 flows: one gate demands broad inspect/subagent debt closure while another forbids detached inspect or node transitions, causing the agent to loop instead of editing.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - The current failing run shows repeated contradictory TaskSpace router errors, and `processing-pipeline` is a clean regression from old TaskSpace success to new no-edit failure.
- Falsifiable predictions:
  - If true: The clean regression sample should show runtime gate errors before any successful edit and no changed artifacts on the TaskSpace side.
  - If false: The clean regression sample should show normal edit execution and fail for task-domain reasons rather than TaskSpace routing/gate reasons.
- Diagnostic evidence plan:
  - Prediction or clause under test: The clean regression sample fails because TaskSpace gates prevent a successful edit path.
  - Signal: Pair report, changed file inventory, stderr router errors, last message, and old/new pair comparison.
  - Capture method: Inspect existing diagnostic artifacts without running new agents.
  - Event name or marker:
    - `ERROR codex_core::tools::router`
    - `TaskSpace mode is active, but no ready node is available`
    - `cannot create a detached inspect_code_context node`
    - `implement_solution node ... cannot be completed without a recorded successful edit action`
  - Correlation keys:
    - sample `processing-pipeline`
    - run root `target\terminal-bench_E3-P0_3_1-v005-20260619-diagnostic-4`
  - Differentiates from:
    - H-002
    - H-003
  - Supports if:
    - Current TaskSpace side has no edits and repeated gate errors while old TaskSpace side succeeded on the same sample.
  - Refutes if:
    - Current TaskSpace side edited correctly and failed only due to validator or domain logic.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Related evidence:
  - E-001
  - E-002
  - E-003
  - E-004
  - E-006
  - E-007
  - E-008
  - E-010
- Evidence gate: satisfied
- Conclusion: confirmed. The failure mechanism is a state-machine contradiction, not a task-domain failure: runtime asks for subagent inspect tracks, but the same state forbids creating detached inspect nodes after a completed narrow inspect pass and offers an empty structured recovery object.
- Repair design readiness: ready for design; do not patch until the recovery invariant is specified.
- Next step: Design a deterministic runtime unit/regression test that constructs this state without running real agents, then make the state either reachable to recovery or non-enterable.
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: The regression is primarily validator or harness noise
- Status: partially confirmed
- Parent: P-001
- Claim: TaskSpace did not truly fail all three P0 tasks; the apparent 0/3 result is mostly caused by validator environment failures and score validity rules.
- Layer: interaction
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - `recover-accuracy-log` produced the same `results.json` as Standard, and validator logs show dependency download failures.
- Falsifiable predictions:
  - If true: At least one TaskSpace failure should have correct business artifacts and fail only because validation infrastructure fails.
  - If false: All TaskSpace failures should lack correct artifacts or fail visible tests for task-domain reasons.
- Diagnostic evidence plan:
  - Prediction or clause under test: Some TaskSpace failures are false negatives caused by validator environment failure.
  - Signal: Generated artifacts, Standard-vs-TaskSpace artifact comparison, public validation stderr.
  - Capture method: Inspect `recover-accuracy-log` artifacts and validator logs.
  - Event name or marker:
    - `Failed to download`
    - `Network is unreachable`
    - `error: Failed to spawn: pytest`
  - Correlation keys:
    - sample `recover-accuracy-log`
  - Differentiates from:
    - H-001
  - Supports if:
    - TaskSpace artifacts match Standard but public validator fails for network/dependency reasons.
  - Refutes if:
    - TaskSpace artifacts differ materially or validator fails with task assertion failures.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied for `recover-accuracy-log`; not applicable to `processing-pipeline`
- Related evidence:
  - E-005
- Conclusion: partially confirmed. Validator/network failure makes the aggregate 0/3 result misleading, because `recover-accuracy-log` appears business-correct. It does not explain the clean no-edit `processing-pipeline` regression.
- Repair design readiness: ready to separate benchmark reporting into business-success, harness-validity, and raw-validator-success columns.
- Next step: Keep validator false negatives out of root-cause attribution for TaskSpace execution failures.
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-003: The new provider attribution fallback removal directly caused node-selection failures
- Status: refuted
- Parent: P-001
- Claim: Removing provider attribution fallback caused provider request context to become missing, which directly caused the runtime to lose the current ready node and block tools.
- Layer: sub-cause
- Factor relation: unknown
- Depends on:
  - none
- Rationale:
  - A recent v0.0.5 change explicitly removed ready-node fallback for provider attribution; the failing logs contain `no ready node is available`.
- Falsifiable predictions:
  - If true: Current failing artifacts should contain missing provider context markers or `provider-context-missing` near the failure loop.
  - If false: The gate errors should be about node lifecycle / completion contracts rather than provider attribution context.
- Diagnostic evidence plan:
  - Prediction or clause under test: Provider attribution missing context appears in the failing TaskSpace traces.
  - Signal: stderr and rollout trace markers for `provider-context-missing` or `provider_request_context_missing_reason`.
  - Capture method: Search failing artifacts.
  - Event name or marker:
    - `provider-context-missing`
    - `provider_request_context_missing_reason`
  - Correlation keys:
    - current `_3_1` run root
  - Differentiates from:
    - H-001
  - Supports if:
    - Missing provider context markers appear around no-ready-node failures.
  - Refutes if:
    - No such markers appear and errors point to gate contract contradictions instead.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-009
- Conclusion: refuted for the observed `_3_1` failure loop. Current artifacts contain no `provider-context-missing`, `provider_request_context_missing_reason`, or `current_main_node_missing` markers; the observed errors come from action-map gate contracts.
- Repair design readiness: no provider fallback repair is indicated for this root cause.
- Next step: Do not prioritize provider attribution fallback unless a separate trace contains provider-context markers.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: old `_3_2` TaskSpace was not all-failing
- Related hypotheses:
  - H-001
- Direction: supports
- Type: observation
- Source: `target\terminal-bench_E3-P0_3_2-v005-20260619-variant\pair-summary.json`
- Prediction or plan link:
  - H-001 prediction: clean regression sample should show old success and current failure.
- Matched signal:
  - Old run shows TaskSpace 3/5 raw success, including `processing-pipeline` pair 001 and 002.
- Correlation keys:
  - sample set `terminal-bench_E3-P0_3_2`
- Raw content:
  ```text
  processing-pipeline pair-001 taskspace_success=true
  processing-pipeline pair-002 taskspace_success=true
  recover-accuracy-log pair-002 taskspace_success=true
  overall TaskSpace raw success: 3/5
  ```
- Interpretation: The all-failing TaskSpace result is not present in the earlier P0 diagnostic run.
- Time: 2026-06-19 23:55

## Evidence E-002: new `_3_1` TaskSpace raw success is 0/3
- Related hypotheses:
  - H-001
- Direction: supports
- Type: observation
- Source: `target\terminal-bench_E3-P0_3_1-v005-20260619-diagnostic-4\pair-summary.json`
- Prediction or plan link:
  - H-001 prediction: clean regression sample should show current failure.
- Matched signal:
  - New run shows TaskSpace failure for all three P0 samples.
- Correlation keys:
  - sample set `terminal-bench_E3-P0_3_1`
- Raw content:
  ```text
  processing-pipeline taskspace_success=false
  multi-source-data-merger taskspace_success=false
  recover-accuracy-log taskspace_success=false
  overall TaskSpace raw success: 0/3
  ```
- Interpretation: Current observed symptom is confirmed.
- Time: 2026-06-19 23:55

## Evidence E-003: `processing-pipeline` current TaskSpace side produced no edits
- Related hypotheses:
  - H-001
- Direction: supports
- Type: observation
- Source: `target\terminal-bench_E3-P0_3_1-v005-20260619-diagnostic-4\processing-pipeline\runs\terminal_bench__processing-pipeline\20260619-185744-707\pair-001\pair-report.md`, `right\app\generate_report.sh`
- Prediction or plan link:
  - H-001 prediction: current TaskSpace side has no edits and fails because the edit path was not reached.
- Matched signal:
  - `changed_paths` is empty; `generate_report.sh` still starts with `#!/bin/nonexistent`; last message is `/task-reborn`.
- Correlation keys:
  - sample `processing-pipeline`
  - pair `001`
- Raw content:
  ```text
  right / taskspace:
  business_success: False
  public_validation_exit_code: 1
  changed_paths:
  generate_report.sh first line: #!/bin/nonexistent
  last-message.md: /task-reborn
  ```
- Interpretation: This is a true TaskSpace runtime/productivity failure, not merely a validator false negative.
- Time: 2026-06-19 23:55

## Evidence E-004: current TaskSpace stderr shows repeated gate contradictions on the clean regression sample
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: `target\terminal-bench_E3-P0_3_1-v005-20260619-diagnostic-4\processing-pipeline\runs\terminal_bench__processing-pipeline\20260619-185744-707\pair-001\right\artifacts\whale-exec.stderr.log`
- Prediction or plan link:
  - H-001 prediction: gate errors appear before successful edit and explain no-edit failure.
- Matched signal:
  - 33 TaskSpace router errors, including no ready node, detached inspect denial, broad delegation debt, missing edit evidence, and active running node blocks.
- Correlation keys:
  - sample `processing-pipeline`
  - pair `001`
- Raw content:
  ```text
  error_count=33
  no_ready_node=9
  detached_inspect=6
  broad_debt=2
  unreviewed_result=3
  active_lease=1
  running_node=2
  successful_edit_required=2
  ```
- Interpretation: The clean regression sample got trapped in TaskSpace runtime gate handling rather than completing the intended script edits.
- Time: 2026-06-19 23:55

## Evidence E-005: `recover-accuracy-log` appears to be a validator false negative
- Related hypotheses:
  - H-002
- Direction: supports
- Type: observation
- Source: `target\terminal-bench_E3-P0_3_1-v005-20260619-diagnostic-4\recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260619-193200-574\pair-001`
- Prediction or plan link:
  - H-002 prediction: some TaskSpace failures have correct artifacts but fail due to validator environment.
- Matched signal:
  - TaskSpace `results.json` matches Standard exactly; all six JSONL files have 10 lines; validator stderr shows dependency/network failure.
- Correlation keys:
  - sample `recover-accuracy-log`
  - pair `001`
- Raw content:
  ```text
  right results.json equals left results.json:
  run 1: correct=7 total=9 accuracy=0.78
  run 2: correct=3 total=9 accuracy=0.33
  run 3: correct=8 total=8 accuracy=1.0
  right generated file line counts:
  run_1_generator.jsonl lines=10
  run_1_judge.jsonl lines=10
  run_2_generator.jsonl lines=10
  run_2_judge.jsonl lines=10
  run_3_generator.jsonl lines=10
  run_3_judge.jsonl lines=10
  validation.stderr.log:
  Failed to download distribution due to network timeout
  error: Failed to spawn: `pytest`
  ```
- Interpretation: The 0/3 raw success figure overstates agent business failure; one sample is confounded by validator infrastructure.
- Time: 2026-06-19 23:55

## Evidence E-006: Runtime gates encode mutually incompatible recovery requirements
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-inspection
- Source: `third_party\codex-cli\codex-rs\core\src\action_map\runtime.rs`
- Prediction or plan link:
  - H-001 prediction: the observed gate errors should map to real runtime constraints that can make recovery unreachable.
- Matched signal:
  - `validate_broad_inspect_delegation` blocks ordinary main-agent work until two accepted subagent inspect results exist.
  - `validate_broad_inspect_finish_transition` tells the agent to create at least two ready `inspect_code_context` nodes and spawn agents.
  - `create_node` rejects detached `inspect_code_context` nodes after a completed narrow inspect pass.
  - `validate_spawn_parallel_inspect_track` also rejects spawn when a completed narrow inspect exists and only one follow-up inspect track is available.
- Correlation keys:
  - `runtime.rs:2287-2295`
  - `runtime.rs:6186-6310`
  - `runtime.rs:6659-6676`
- Raw content:
  ```text
  runtime.rs:6278-6280 blocks ordinary work until two accepted subagent inspect results exist.
  runtime.rs:6242-6244 tells the model to create at least two ready inspect_code_context nodes.
  runtime.rs:2287-2295 rejects detached inspect_code_context after a completed narrow inspect pass.
  runtime.rs:6670-6676 rejects spawn after a completed narrow inspect unless at least two ready tracks already exist.
  ```
- Interpretation: The runtime can demand subagent inspect evidence while also rejecting the node creation/spawn path needed to produce it.
- Time: 2026-06-20 00:02

## Evidence E-007: The clean regression log reaches the contradictory state and then `/task-reborn`
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: `target\terminal-bench_E3-P0_3_1-v005-20260619-diagnostic-4\processing-pipeline\runs\terminal_bench__processing-pipeline\20260619-185744-707\pair-001\right\artifacts\rollout.jsonl`
- Prediction or plan link:
  - H-001 prediction: the model should be blocked by contradictory gate messages before any edit.
- Matched signal:
  - Lines 140-142 block ordinary work for broad inspect delegation debt and emit empty `TaskSpaceGateRecoveryV1`.
  - Lines 151-153 reject creating detached `inspect_code_context` nodes.
  - Lines 179-180 report no ready node.
  - Line 213 records the model's own reasoning that the runtime is contradictory.
  - `last-message.md` is `/task-reborn`.
- Correlation keys:
  - sample `processing-pipeline`
  - pair `001`
- Raw content:
  ```text
  TaskSpace blocked ordinary main-agent work because a broad inspect_code_context node exhausted its main-tool budget without two accepted subagent inspect results...
  TaskSpaceGateRecoveryV1: ... "next_valid_actions":[]
  TaskSpace cannot create a detached inspect_code_context node after a completed narrow inspect pass.
  TaskSpace mode is active, but no ready node is available.
  last-message.md: /task-reborn
  ```
- Interpretation: The failure loop is visible in the agent trace and ends before business edits are made.
- Time: 2026-06-20 00:02

## Evidence E-008: Old success and new failure differ in TaskSpace execution depth, not task difficulty
- Related hypotheses:
  - H-001
- Direction: supports
- Type: metric-comparison
- Source: old and new `processing-pipeline` TaskSpace `metrics.json`, `git-diff.patch`, and `last-message.md`
- Prediction or plan link:
  - H-001 prediction: old run should reach edits and richer TaskSpace execution while new run should stop early in gates.
- Matched signal:
  - Old `_3_2` TaskSpace pair succeeded with non-empty diff and substantial node/subagent execution.
  - New `_3_1` TaskSpace pair failed with no edits, zero subagent results, and `/task-reborn`.
- Correlation keys:
  - old run `target\terminal-bench_E3-P0_3_2-v005-20260619-variant\processing-pipeline`
  - new run `target\terminal-bench_E3-P0_3_1-v005-20260619-diagnostic-4\processing-pipeline`
- Raw content:
  ```text
  old right: business_success=True, tool_call_count=46, model_request_count=143, taskspace_runtime_event_count=511, nodes=11, edges=24, subagent_results=24, spawn_agent_calls=3, git-diff.patch length=1704
  new right: business_success=False, tool_call_count=9, model_request_count=66, taskspace_runtime_event_count=118, nodes=3, edges=2, subagent_results=0, spawn_agent_calls=0, git-diff.patch length=3, last-message.md=/task-reborn
  ```
- Interpretation: The new failure is an early orchestration/gate collapse, not a hard domain task that the old run also could not solve.
- Time: 2026-06-20 00:02

## Evidence E-009: Provider-context fallback removal is not observed in failing artifacts
- Related hypotheses:
  - H-003
- Direction: refutes
- Type: artifact-search
- Source: `target\terminal-bench_E3-P0_3_1-v005-20260619-diagnostic-4`
- Prediction or plan link:
  - H-003 prediction: provider context markers should appear near the failure loop if provider attribution fallback removal caused the no-ready-node behavior.
- Matched signal:
  - Repository artifact search found no `provider-context-missing`, `provider_request_context_missing_reason`, or `current_main_node_missing` markers in current `_3_1` artifacts.
- Correlation keys:
  - current `_3_1` run root
- Raw content:
  ```text
  rg provider-context-missing/provider_request_context_missing_reason/current_main_node_missing over current _3_1 artifacts: no matches
  ```
- Interpretation: The observed no-ready-node loop is not explained by missing provider request context in this run.
- Time: 2026-06-20 00:02

## Evidence E-010: Generic gate recovery wrapping erases actionable recovery for structural gates
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-inspection
- Source: `third_party\codex-cli\codex-rs\core\src\action_map\runtime.rs`
- Prediction or plan link:
  - H-001 prediction: the runtime should expose no machine-actionable recovery path when a generic gate error is thrown.
- Matched signal:
  - `ActionMapGateError::new` wraps any message without an existing `TaskSpaceGateRecoveryV1` using `gate_recovery_message(..., Vec::new(), Vec::new(), Vec::new())`.
  - The broad delegation debt error returns only a string, so the structured recovery object has empty `blocking_items`, `next_valid_actions`, and `missing_evidence`.
- Correlation keys:
  - `runtime.rs:166-179`
  - `runtime.rs:7421-7436`
  - `runtime.rs:6278-6280`
- Raw content:
  ```text
  ActionMapGateError::new -> gate_recovery_message(message, "taskspace_gate_blocked", Vec::new(), Vec::new(), Vec::new())
  failing rollout -> TaskSpaceGateRecoveryV1 ... "blocking_items":[],"next_valid_actions":[],"missing_evidence":[]
  ```
- Interpretation: The natural-language blocker may mention a recovery path, but the machine-readable recovery contract gives the model no executable next action and does not encode the conflicting gate state.
- Time: 2026-06-20 00:02

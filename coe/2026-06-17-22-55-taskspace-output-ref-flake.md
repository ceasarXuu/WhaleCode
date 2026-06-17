# Problem P-001: TaskSpace output reference creation is intermittent in large-output smoke

Status: fixed

Observed symptom:

- In `large-output-ref-smoke` 3-repeat run `target\v005-active-projection-e1\large-output-ref-smoke\20260617-224503-285`, pair-003 TaskSpace side was marked `engineering_unclean`.
- The pair-003 TaskSpace metrics reported `runtime_output_ref_created_count=0`, while the scenario expects at least 1 output reference.
- The same run had pair-001 and pair-002 TaskSpace sides solved with `runtime_output_ref_created_count=1`.

Expected behavior:

- TaskSpace mode should reliably create runtime output references for large-output smoke runs when the scenario expects output-ref materialization.
- Hidden oracle and engineering-clean gates should not depend on nondeterministic model behavior around large diagnostic output handling.

Impact:

- v0.0.5 E3 evidence cannot be release-grade while large-output output-ref creation is flaky.
- Active projection metrics are now measurable, but the output-ref gate still prevents all repeats from being aggregate-eligible.

Known facts:

- Pair-001 TaskSpace: solved, `runtime_output_ref_created_count=1`, active projection measured.
- Pair-002 TaskSpace: solved, `runtime_output_ref_created_count=1`, active projection measured.
- Pair-003 TaskSpace: hidden oracle failed, `runtime_output_ref_created_count=0`, active projection measured.
- Pair-003 TaskSpace had `runtime_state_commit_count=7`, `taskspace_control_count=0`, `large_output_replay_count=0`, and `raw_output_in_prompt_violation=false`.

Fix criteria:

- A controlled repeat of `large-output-ref-smoke` must show each TaskSpace side meeting `runtime_output_ref_created_count>=1` when that scenario expectation is enabled.
- The fix must preserve `large_output_replay_count=0` and active projection measurement.
- The fix must be backed by a targeted unit/self-test or benchmark harness check that catches the missing output-ref condition.

# Hypothesis H-001: Output-ref creation is tied to large tool-result capture, but pair-003 avoided the large diagnostic path

Status: open

Claim:

- The runtime output reference is only created when a large tool result passes through the output-reference materialization path. In pair-003, the agent solved the task without executing or surfacing the scenario's large-output diagnostic in the way the output-ref detector expects.

Predictions:

- Pair-003 rollout or tool-call log will show no large diagnostic command output that crosses the output-ref threshold.
- Pair-001 and pair-002 will contain a tool result or runtime event that created an output reference before the implementation fix.
- Pair-003 can still have active projection and state commits, because those are independent of output-ref materialization.

Diagnostic evidence plan:

- Compare pair-001, pair-002, and pair-003 TaskSpace side `rollout.jsonl`, `whale-exec.jsonl`, `taskspace-control-usage.json`, and observability artifacts for output-ref events, large diagnostic command calls, and tool-result sizes.
- Inspect runtime output-reference threshold logic and scenario prompt/validator expectations to determine whether the expectation is unconditional or should be tied to actual large-output command use.

# Hypothesis H-002: Benchmark instrumentation missed output-ref creation for pair-003

Status: open

Claim:

- Pair-003 may have created an output reference, but the instrumentation failed to count it because it only scans a subset of artifacts or event formats.

Predictions:

- Raw artifacts for pair-003 will contain `OutputReferenceV1`, `output-ref://`, or `output_ref.created` even though `metrics.json` reports zero.
- Re-running cost instrumentation against all pair-003 TaskSpace artifact sources would produce `runtime_output_ref_created_count>=1`.

Diagnostic evidence plan:

- Search pair-003 TaskSpace artifacts for output-reference markers.
- Compare artifact source paths used by the instrumentation with where output-reference markers are actually recorded.

# Evidence E-001: 3-repeat smoke produced two clean TaskSpace runs and one output-ref-missing TaskSpace run

Observation:

- Command: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 -Scenario large-output-ref-smoke -Repeats 3 -RunRoot target\v005-active-projection-e1 -TimeoutSeconds 600 -ValidationTimeoutSeconds 180 -ValidationPretestTimeoutSeconds 60 -ValidationTestTimeoutSeconds 180 -SandboxMode workspace-write -EnableAggregate -AllowNonE2Result`
- Run directory: `target\v005-active-projection-e1\large-output-ref-smoke\20260617-224503-285`
- Pair-001: `reported_evidence_level=E2`, included in aggregate, TaskSpace solved.
- Pair-002: `reported_evidence_level=E2`, included in aggregate, TaskSpace solved.
- Pair-003: `reported_evidence_level=E1`, not included in aggregate, TaskSpace `engineering_unclean`, reason `scenario_expected_runtime_output_ref_created_count_not_met:0<1`.

Supports:

- Problem P-001
- Hypothesis H-001 remains plausible.
- Hypothesis H-002 remains plausible until raw artifact search is complete.

# Evidence E-002: Pair-003 raw artifacts show the diagnostic command failed before emitting large output

Observation:

- Pair-003 TaskSpace `whale-exec.jsonl` contains `python scripts/emit_large_log.py` as a command execution.
- The command failed with `PermissionError: [Errno 13] Permission denied: '.large_output_probe_ran'`.
- Pair-003 `hidden-oracle.stderr.log` reports `AssertionError: diagnostic command was not executed`.
- Pair-003 artifacts contain no `OutputReferenceV1`, `output-ref://`, or `output_ref.created` marker for the diagnostic command path.
- Pair-001 and pair-002 successful TaskSpace sides contain `OutputReferenceV1` in observability artifacts and report `runtime_output_ref_created_count=1`.

Supports:

- H-001: the missing output-ref was not an instrumentation miss; the large-output command failed before producing the large stdout that creates the output reference.
- H-002 is downgraded because raw artifact search found no output-ref marker in pair-003 TaskSpace artifacts.

# Evidence E-003: Pair-003 started the diagnostic command alongside a file-enumeration command

Observation:

- Pair-003 TaskSpace `whale-exec.jsonl` first started `Get-ChildItem -Recurse -File ...`, then started `python scripts/emit_large_log.py` before the file-enumeration command completed.
- Pair-001 TaskSpace started and completed `python scripts/emit_large_log.py` as the first command before other commands.
- Pair-002 TaskSpace completed the diagnostic command before subsequent file inspection commands.
- A manual rerun of `python scripts\emit_large_log.py` in the failed pair-003 repo after the benchmark completed exited `0` and created `.large_output_probe_ran`.

Supports:

- H-001 with a narrower mechanism: the scenario prompt allowed the model to start diagnostic execution together with other reads; under the Windows sandbox this made the marker write path flaky. The scenario contract should require the diagnostic command as the first completed command before any other command starts.

# Evidence E-004: Scenario prompt was tightened to make the diagnostic command first and serial

Observation:

- Updated `benchmarks/taskspace/scenarios/large-output-ref-smoke/prompt.txt` from "Before editing code, run this diagnostic command exactly once" to "Before editing code, run this diagnostic command exactly once as the first command. Wait for it to finish before starting any other command".
- The wording avoids TaskSpace-internal terms and avoids prompt-guard context terms such as "parallel" or "concurrent".

Supports:

- H-001 repair direction: make the benchmark contract explicit enough that the model should not issue the large-output diagnostic command in the same batch as file-inspection commands.

# Evidence E-005: Prompt guard accepted the tightened scenario prompt

Observation:

- Command: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 -Scenario large-output-ref-smoke -RunRoot target\v005-outputref-firstcmd-planonly -PlanOnly`
- Result: `PromptInvalid: False`, `PromptManualReview: False`.
- Run directory: `target\v005-outputref-firstcmd-planonly\large-output-ref-smoke\20260617-225808-156`.

Supports:

- The prompt-contract fix did not introduce TaskSpace-internal leakage or context-sensitive prompt-guard hits.

# Evidence E-006: 3-repeat validation passed after the first-command prompt contract

Observation:

- Command: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 -Scenario large-output-ref-smoke -Repeats 3 -RunRoot target\v005-outputref-firstcmd-e1 -TimeoutSeconds 600 -ValidationTimeoutSeconds 180 -ValidationPretestTimeoutSeconds 60 -ValidationTestTimeoutSeconds 180 -SandboxMode workspace-write -EnableAggregate -AllowNonE2Result`
- Run directory: `target\v005-outputref-firstcmd-e1\large-output-ref-smoke\20260617-225818-837`.
- Pair-001, pair-002, and pair-003 all reported `reported_evidence_level=E2`, `included_in_utility_aggregate=True`, `Evidence Gate Failures: none`, `failure_taxonomy=none`, `engineering_unclean=False`, `outcome_standard=solved`, and `outcome_taskspace=solved`.
- TaskSpace output-ref metrics:
  - pair-001: `runtime_output_ref_created_count=1`, `large_output_replay_count=0`, active projection `availability=measured`, `projection_count=16`.
  - pair-002: `runtime_output_ref_created_count=1`, `large_output_replay_count=0`, active projection `availability=measured`, `projection_count=11`.
  - pair-003: `runtime_output_ref_created_count=1`, `large_output_replay_count=0`, active projection `availability=measured`, `projection_count=18`.

Supports:

- H-001 confirmed.
- Problem P-001 fixed under the declared reproduction conditions.

# Hypothesis H-001 conclusion

Status: confirmed

Conclusion:

- The missing output reference was caused by the benchmark prompt allowing the required diagnostic command to be issued in the same tool-call batch as file inspection. In the failing run, the diagnostic command failed before emitting the large stdout and therefore could not create an output reference. Tightening the scenario prompt to require the diagnostic command as the first completed command removed the observed flake across a 3-repeat validation.

# Hypothesis H-002 conclusion

Status: refuted

Conclusion:

- Pair-003 raw artifacts contained no output-reference marker. The reported zero count was accurate for the failed run.

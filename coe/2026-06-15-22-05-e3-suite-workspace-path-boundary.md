# Problem P-001: E3 suite workspace materialization fails at Git baseline

Status: investigating

## Problem

Symptoms:
- Serial calibration suite `D:\whalecode-alpha\target\e3-v004-proof-20260615\serial-calibration-v2\suite-20260615-213255` aborts before agent execution.
- `analyze-access-logs` aborts in `workspace_materialization` with `invalid object ... for 'Dockerfile'`.
- `log-summary` aborts in `workspace_materialization` with `unable to write file .git/objects/...: Invalid argument`.

Expected behavior:
- E3 suite materializes each pair workspace cleanly and reaches real pair execution.

Actual behavior:
- The suite marks the run `invalid_harness`, so score evidence is invalid.

Fix criteria:
- A focused test prevents Terminal-Bench pair repo paths from crossing the Windows Git object path boundary.
- The v2 serial calibration can pass workspace materialization and proceed beyond the previous abort point.

## Hypothesis H-001: Terminal-Bench suite repo paths exceed Windows Git object path budget

Status: confirmed

Rationale:
- The one-pair run used a shorter root and materialized `log-summary` successfully.
- The suite adds `suite-...\samples\<sample>\runs\...` under each sample, increasing `.git/objects` paths.

Predictions:
- Failed suite object paths should be at or above the 260-character boundary.
- The equivalent one-pair object path should be substantially shorter.
- Failure should occur during Git baseline initialization, not during fixture copy or validator execution.

Diagnostic evidence plan:
- Measure failed suite `.git/objects` path lengths and compare them with the successful one-pair path.
- Inspect `pair-abort.json` and pair directories for `.git` creation state.
- Confirm events show abort before agent and validator phases.

## Evidence E-001

Type: abort artifact

Observation:
- `log-summary` `pair-abort.json` reports `workspace_materialization_failed` and normalized message `error: unable to write file .git/objects/f8/93c5f827d194c3f3d7edda5ad397b83912bf4a: Invalid argument`.
- `analyze-access-logs` `pair-abort.json` reports `workspace_materialization_failed` and normalized message `error: invalid object 100644 3e40a9fe4e99fc548b9421b013c75c8cc706cb9d for 'Dockerfile'`.

Supports:
- H-001 prediction that the failure is Git baseline initialization, before agent or validator execution.

## Evidence E-002

Type: runtime state

Observation:
- Failed `log-summary` left side has `.git` and the target object path length is 259 characters; right side target object path length is 260 and the object is missing.
- Failed `analyze-access-logs` target object path length is 275 on the first side before the right side is created.
- Successful one-pair `log-summary` equivalent object path length is 219 characters.

Supports:
- H-001 prediction that suite nesting pushes Git object paths to the Windows path boundary while one-pair does not.

## Evidence E-003

Type: event log

Observation:
- `events.jsonl` for failed `log-summary` shows `pair_disk_health_completed`, `pair_started`, then `sample_aborted_by_guardrail` with `abort_phase=workspace_materialization`.
- No agent execution or validator events appear before the abort.

Supports:
- H-001 prediction that the failure is a harness materialization problem, not an agent solving or public validation outcome.

## Hypothesis H-002: Validator proof artifact paths exceed Windows writer path budget

Status: confirmed

Rationale:
- After shortening the Terminal-Bench repo path, the suite passed workspace materialization but `analyze-access-logs` failed during validator probe.

Predictions:
- Probe stdout should show normal validator startup before result writing.
- Probe stderr should fail while writing a proof artifact under `external-validator-runtime-probe`.
- The failing result path should exceed the Windows path boundary.

Diagnostic evidence plan:
- Read validator probe stdout/stderr and measure `validator-probe-result.json` path length.

## Evidence E-004

Type: probe artifact

Observation:
- `validator-probe.stdout.log` shows `validator_probe_started=true`, Docker backend detection, repo mount, and validator mount.
- `validator-probe.stderr.log` fails at generated `external-validator.ps1` while writing `external-validator-runtime-probe\validator-probe-result.json`.
- The measured full path length for `validator-probe-result.json` is 266 characters.

Supports:
- H-002 prediction that proof artifact path length, not Docker execution or task logic, caused the probe failure.

## Hypothesis H-003: Serial calibration v4 is not valid clean evidence because validator timeouts were recorded as engineering unclean at pair level

Status: confirmed

Rationale:
- The user-defined hard constraint allows only agent execution timeout as a non-infrastructure unexpected outcome.
- Public validation timeout is a validator/runtime failure and must invalidate scoring evidence.

Predictions:
- Any pair with `public_validation_exit_code=124` should have `audit.json.engineering_unclean=true`.
- Suite/sample timing should surface `engineering_unclean_slow` or equivalent blockers for those pairs.
- A clean rerun must have zero public validation timeout reasons before scores are trusted.

Diagnostic evidence plan:
- Inspect serial calibration v4 pair metrics, pair reports, audit manifests, and suite timing.

## Evidence E-005

Type: audit artifact

Observation:
- Serial calibration v4 `log-summary` pair 001 has `left/artifacts/metrics.json` with `public_validation_exit_code=124`, `validator_environment_failures=["public_validation_timeout"]`, and `validation_timeout_phase="tests"`.
- The same pair's `audit.json` has `engineering_unclean=true`, `engineering_unclean_reasons=["public_validation_timeout"]`, `run_score_valid=false`, and `score_exclusion_reason="engineering_unclean"`.

Supports:
- H-003 prediction that validator timeout is a hard engineering-unclean condition at pair level.

## Evidence E-006

Type: timing artifact

Observation:
- Recomputed serial calibration v4 `suite-timing.json` no longer reports missing sample timing after the nested timing-path fix (`missing_sample_timing_count=0`, `timing_sample_count=3`).
- It still reports `bottleneck_classification="engineering_unclean_slow"` because child pair timing contains `public_validation_timeout` reasons.

Supports:
- H-003 prediction that old serial calibration v4 is invalid clean-score evidence even after timing path reconstruction is fixed.

## Hypothesis H-004: E3 suite calibration bypassed early score-invalid abort because scoring enforcement was opt-in and incompatible with SkipStartGate

Status: confirmed

Rationale:
- Calibration runs need `-SkipStartGate` because they are the evidence used by the start gate.
- The suite previously only passed `-ScoringMode` to child runners when explicitly requested, and rejected `-ScoringMode` with `-SkipStartGate` for real runs.
- This allowed calibration runs to finish all pairs even when pair audit artifacts already marked validator timeout as engineering unclean.

Predictions:
- A suite run launched without explicit `-ScoringMode` should have child `resume_command` values that omit `-ScoringMode`.
- The same run can have pair-level `audit.json.engineering_unclean=true` while suite status remains completed if no other invalid-harness path fires.
- Making non-PlanOnly E3 suite runs enforce score validity by default should pass `-ScoringMode` to child runners even with `-SkipStartGate`.

Diagnostic evidence plan:
- Inspect serial clean/serial v4 child `resume_command` fields and suite code path for `New-SuiteChildArgs`.
- Add a stub-runner regression test proving default non-PlanOnly suite runs pass `-ScoringMode` and abort remaining samples on child score invalidity.

## Evidence E-007

Type: runtime state and code path

Observation:
- Serial clean v1 sample `resume_command` fields omit `-ScoringMode` when the suite command omits it.
- `run-taskspace-e3-suite.ps1` previously appended child `-ScoringMode` only when `$ScoringMode` was explicitly set.
- The script also rejected explicit score-bearing runs combined with `-SkipStartGate`, blocking calibration from using scoring enforcement directly.

Supports:
- H-004 prediction that calibration could bypass early score-invalid abort despite pair-level engineering-unclean artifacts.

# COE: E3 Rerun Suite Abort

## Problem P-001: E3 rerun exited 1 after partial suite execution

- Status: investigating
- Observed symptom: `C:\w\e3v004-rerun-20260613-164709\e3-suite.exit.json` recorded `exit_code=1` at `2026-06-13T20:04:54+08:00`.
- Expected behavior: after guardrail closure, the 0.0.4 E3 suite should either complete all configured samples or stop with a precise, actionable invalid-harness reason that preserves completed sample state.
- Actual behavior:
  - `recover-accuracy-log` completed 5/5 and reached `audit_required`.
  - `processing-pipeline` completed only 3/5 but suite-health kept its status as `phase=execute`, `run_validity=valid`.
  - `multi-source-data-merger` failed before materialization with `harness_materialization_failure/path_unresolvable`.
- Impact: the E3 rerun is not a successful full 0.0.4 validation. It also revealed that suite-level status can hide an incomplete child run when a child exits 1.
- Environment:
  - repo: `D:\whalecode-alpha`
  - commit: `d22709572`
  - run root: `C:\w\e3v004-rerun-20260613-164709`
  - task list: `C:\w\e3v004-rerun-20260613-164709\e3.jsonl`
- Fix criteria:
  - The missing task path and the child exit 1 cause are explained by evidence.
  - Suite status must not report a child run as valid/execute when the child process exits nonzero before completing requested pairs.
  - A rerun or targeted resume must preserve completed evidence and prove the original failure mode no longer occurs.

## Hypothesis H-001: The third sample failed because the task cache path in `e3.jsonl` pointed to a deleted temp directory

- Status: confirmed
- Rationale: `multi-source-data-merger` was configured under `%TEMP%\whale-real-external-benchmarks`, which is not a durable benchmark source location.
- Prediction:
  - The task list references `C:\Users\77585\AppData\Local\Temp\...\multi-source-data-merger`.
  - That path does not exist at rerun time.
  - The external materialization health artifact records `task_dir.exists=false` and `stable_code=path_unresolvable`.
- Diagnostic evidence plan:
  - Read `e3.jsonl`.
  - Test the referenced path.
  - Read `external-materialization-health.json` and `abort-summary.md`.

### Evidence E-001

- Type: runtime artifact
- Supports: H-001 prediction that the task list references a temp path.
- Observation: `C:\w\e3v004-rerun-20260613-164709\e3.jsonl` contains `task_dir` for `multi-source-data-merger` as `C:\Users\77585\AppData\Local\Temp\whale-real-external-benchmarks\terminal-bench\original-tasks\multi-source-data-merger`.

### Evidence E-002

- Type: runtime probe
- Supports: H-001 prediction that the referenced path does not exist.
- Observation: `Test-Path 'C:\Users\77585\AppData\Local\Temp\whale-real-external-benchmarks\terminal-bench\original-tasks\multi-source-data-merger'` returned `False`.

### Evidence E-003

- Type: runtime artifact
- Supports: H-001 prediction that materialization records a structured path failure.
- Observation: `C:\w\e3v004-rerun-20260613-164709\runs\suite-20260613-164709\samples\multi-source-data-merger\external-materialization-health.json` recorded `stable_code=path_unresolvable`, `checked_paths[0].exists=false`, and the same missing task path.

## Hypothesis H-002: `processing-pipeline` stopped at 3/5 because proof hashing treats a missing external source file as a fatal PowerShell error

- Status: confirmed
- Rationale: suite stderr shows `git hash-object` failing in `lib\e3-proof.ps1:213` for `terminal_bench\harness\harness.py`.
- Prediction:
  - The failing path is referenced by an external-source proof path.
  - The file is absent from the source root at the time of hashing.
  - The current code invokes `git hash-object` in a way that turns this missing optional source file into an unhandled terminating error.
- Diagnostic evidence plan:
  - Inspect `scripts/taskspace-benchmark/lib/e3-proof.ps1` around line 213.
  - Find the source guard proof artifact for `processing-pipeline` pair-004 or latest pair.
  - Verify whether `terminal_bench\harness\harness.py` is absent under the recorded source root.

### Evidence E-004

- Type: stderr
- Supports: H-002 initial symptom.
- Observation: `C:\w\e3v004-rerun-20260613-164709\e3-suite.stderr.log` contains `fatal: could not open 'C:\Users\77585\AppData\Local\Temp\whale-real-external-benchmarks\terminal-bench\terminal_bench\harness\harness.py' for reading: No such file or directory` and points to `D:\whalecode-alpha\scripts\taskspace-benchmark\lib\e3-proof.ps1:213`.

### Evidence E-006

- Type: code path
- Supports: H-002 mechanism.
- Observation: `scripts/taskspace-benchmark/lib/e3-proof.ps1` invoked `git -C $sourceRoot hash-object $path` for every `official.source_files` row without first checking whether `$path` existed and without catching native command failure.

### Evidence E-007

- Type: fix validation
- Supports: H-002 repair.
- Observation: after adding `Invoke-TaskspaceGitScalar` and skipping current blob hashing when `actualSha` is empty, `scripts/taskspace-benchmark/test-e3-proof-harness.ps1` passed and includes a missing official source file fixture that downgrades proof instead of throwing.

## Hypothesis H-003: Suite runner fails to capture child exit code 1 as invalid harness status

- Status: confirmed
- Rationale: `processing-pipeline` child status remains `phase=execute`, `run_validity=valid`, `attempted_pairs=3`, while suite process continued to the next sample and finally exited 1.
- Prediction:
  - The suite runner reads stale `sample-status.json` after a child exits 1.
  - It does not synthesize a sample-level invalid harness row when a child process exits nonzero without writing a final invalid status.
  - As a result, `suite-health.json` reports a misleading valid/incomplete child status.
- Diagnostic evidence plan:
  - Inspect `run-taskspace-e3-suite.ps1` child exit handling.
  - Compare `processing-pipeline` child process exit evidence against `sample-status.json` and suite-health row.

### Evidence E-005

- Type: runtime artifact
- Supports: H-003 symptom.
- Observation: `C:\w\e3v004-rerun-20260613-164709\runs\suite-20260613-164709\suite-health.json` records `processing-pipeline` with `phase=execute`, `run_validity=valid`, `attempted_pairs=3`, `completed_pairs=3`, despite the suite-level exit code being `1`.

### Evidence E-008

- Type: code path
- Supports: H-003 mechanism.
- Observation: `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1` read the newest child `sample-status.json` after `& powershell @args`, added that status directly to suite results, and only set suite `$exitCode` for child exit 1/2. It did not synthesize a child failure row when the child exited nonzero with a stale valid status.

### Evidence E-009

- Type: fix validation
- Supports: H-003 repair.
- Observation: `scripts/taskspace-benchmark/test-harness.ps1` passed after adding a regression assertion that the suite runner contains the stable `child_process_failed` classification for nonzero child exits.

### Evidence E-010

- Type: source recovery
- Supports: H-001 repair prerequisite.
- Observation: cloned `https://github.com/laude-institute/terminal-bench.git` to `C:\w\terminal-bench-1a6ffa9674b571da0ed040c470cb40c4d85f9b9b`, checked out `1a6ffa9674b571da0ed040c470cb40c4d85f9b9b`, and verified `original-tasks\multi-source-data-merger` exists.

## Hypothesis H-004: Suite runner misclassified completed diagnostic exit 1 as child process failure

- Status: confirmed
- Rationale: the second durable-path E3 run completed the first two samples at 5/5 pairs with child `run_validity=valid` and `phase=audit_required`, but the suite rewrote both sample rows to `invalid_harness` because the child process exited 1.
- Prediction:
  - Child `sample-status.json` for `recover-accuracy-log` and `processing-pipeline` records `attempted_pairs=5`, `completed_pairs=5`, `run_validity=valid`, and `phase=audit_required`.
  - The suite-level row records `harness_materialization_failure/child_process_failed` for those same samples.
  - `run-taskspace-benchmark.ps1` can intentionally exit 1 after a completed run when failed pairs remain and `AllowNonE2Result` is not set.

### Evidence E-011

- Type: runtime artifact
- Supports: H-004 symptom.
- Observation: `C:\w\e3v004-rerun2-20260613-202032\runs\suite-20260613-202033\suite-health.json` records repeated `harness_materialization_failure/child_process_failed` and skips `multi-source-data-merger`, while the child `sample-status.json` files for the first two samples record valid completed 5/5 diagnostic runs awaiting audit.

### Evidence E-012

- Type: code path
- Supports: H-004 mechanism.
- Observation: `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1` exits 1 after finalizing a completed run when failed pairs exist and `AllowNonE2Result` is not set, so exit 1 is a valid diagnostic result and not always an infrastructure crash.

### Evidence E-013

- Type: fix validation
- Supports: H-004 repair.
- Observation: added `lib\suite-status.ps1` so the suite preserves valid completed child statuses for `completed`, `audit_required`, or `finalize` phases with all required pairs attempted and completed; nonzero incomplete child statuses still synthesize `harness_materialization_failure/child_process_failed`. `test-e3-harness-guardrails.ps1` and `test-harness.ps1` both passed after the fix.

### Evidence E-014

- Type: rerun result
- Supports: H-001/H-002/H-003/H-004 repairs.
- Observation: E3 rerun root `C:\w\e3v004-rerun3-20260614-010208` completed at `2026-06-14T06:35:51+08:00` on commit `7a7e0aa28`. `suite-health.json` reports `status=completed`, empty `signature_counts`, empty `suite_abort_reason`, and three valid sample statuses.

### Evidence E-015

- Type: rerun result
- Supports: full-suite coverage.
- Observation: all three samples reached `phase=audit_required`, `run_validity=valid`, `attempted_pairs=5`, and `completed_pairs=5`: `recover-accuracy-log`, `processing-pipeline`, and `multi-source-data-merger`. The third sample was not skipped.

### Evidence E-016

- Type: rerun result
- Supports: outcome interpretation.
- Observation: suite exit code was `1`, but sample statuses had `abort_scope=none` and no `abort_signature`. Run summaries show all 15 pair reports were produced, but no pair was included in utility or E3 aggregate because results remained diagnostic/audit-pending or lower evidence level. This proves the suite process completed; it does not prove the E3 scoring execution was clean.

### Evidence E-017

- Type: execution contract update
- Supports: outcome reinterpretation.
- Observation: the hard E3 contract now allows only `solved`, `wrong`, and `agent_exec_timeout` as score-bearing agent outcomes. Docker/container/validator/materialization/path/disk/proof/report/audit failures are `engineering_unclean`; any such contamination makes the E3 run `score_valid=false`.

## Problem P-001 Current Conclusion

- Status: repair-validated-by-rerun
- Confirmed root causes:
  - H-001: the E3 task list used a non-durable temp task path that disappeared before rerun.
  - H-002: missing official source files in E3 proof hashing could abort the child process instead of producing a proof mismatch.
  - H-003: suite runner could preserve stale child status after child exit 1, making an incomplete sample look valid.
  - H-004: suite runner could also overcorrect by treating completed diagnostic exit 1 as child process failure.
- Repair implemented:
  - `e3-proof.ps1` now wraps git scalar calls and does not call current blob hashing when the source file is absent.
  - `run-taskspace-e3-suite.ps1` now synthesizes `harness_materialization_failure/child_process_failed` for nonzero child exits that did not already write invalid-harness status.
  - `run-taskspace-e3-suite.ps1` now preserves completed valid diagnostic child statuses even when the child exits 1.
  - A durable Terminal-Bench source checkout is available for the next E3 task list.
- Rerun validation:
  - Completed full 3-sample E3 suite from durable task paths under `C:\w\terminal-bench-1a6ffa9674b571da0ed040c470cb40c4d85f9b9b\original-tasks`.
  - The suite ran all samples and all 15 repeats without suite-level abort or invalid harness signature.
  - This validates the child-exit/materialization fixes only.
- Scoring conclusion:
  - The rerun is not a clean score-bearing E3 execution under the hard contract.
  - Any Docker/container/validator/materialization/path/disk/proof/report/audit contamination makes the correct scoring state `score_valid=false / engineering_unclean`.
  - The run can be used for engineering diagnosis, but not for Standard vs TaskSpace score, better/worse, pass-rate delta, cost delta, or agent ability claims.

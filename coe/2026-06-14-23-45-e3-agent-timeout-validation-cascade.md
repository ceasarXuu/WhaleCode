## Problem P-001

Symptom: an E3 pair can turn an allowed agent execution timeout into an invalid engineering failure by continuing to run public validation on the timed-out side.

Expected behavior: under the hard E3 execution contract, agent execution timeout is the only allowed unexpected timeout outcome. If a side times out during agent execution, the harness should classify that side as `agent_exec_timeout` and avoid spending additional validator time on an incomplete workspace unless an explicit diagnostic mode requests it.

Actual behavior: `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1` executes public validation for every side after agent execution, without checking whether `execBySide[$side.Name].timed_out` is true.

Impact: a TaskSpace side that already timed out can then produce `public_validation_timeout`, causing the whole E3 run to become `engineering_unclean`. This wastes minutes per affected side and invalidates score-bearing comparison for a field that does not reflect agent capability.

Known facts:
- The latest official one-pair smoke recorded TaskSpace agent timeout, then public validation timeout.
- `public_validation_timeout` is correctly classified as engineering-unclean when validation actually runs.
- The current runner has a pre-agent validator probe before agent execution.

Fix criteria:
- A timed-out agent side skips public validation and hidden oracle in scoring mode.
- The skip is explicit in metrics and timing artifacts.
- The side outcome is `agent_exec_timeout` when no other engineering-unclean reason exists.
- Validator timeout remains engineering-unclean when validation is actually executed.

## Hypothesis H-001

Claim: the root cause is an unconditional post-agent validation loop in `run-taskspace-benchmark.ps1`; it does not branch on `exec.timed_out`.

Predictions:
- The runner code will show public validation invocation inside a loop over both sides with no guard for `execBySide[$side.Name].timed_out`.
- Adding a guard that writes explicit validation-skip metrics should remove the cascade while preserving existing validator-timeout classification for non-skipped validation.

Diagnostic evidence plan:
- Inspect runner code around the validation loop.
- Add a focused test that constructs an exec-timeout side with validation skipped and confirms the side outcome is `agent_exec_timeout`.
- Add a focused timing assertion that skipped validation does not emit a real `public_validation` duration span.

## Evidence E-001

Supports H-001 prediction 1. Inspection of `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1` shows the runner builds `$execBySide` during agent execution, then immediately loops over `@($pair.Left, $pair.Right)` and invokes `Invoke-TaskspaceValidationCommand` for each side. The loop does not check `$execBySide[$side.Name].timed_out` before launching validation.

## Evidence E-002

Supports H-001 symptom mechanism. Existing taxonomy code treats metrics with `exec_timed_out=true` as `agent_exec_timeout` only after engineering-unclean reasons are absent. If validation runs after the timeout and exits `124`, `public_validation_timeout` becomes an engineering-unclean reason and overrides the allowed agent-timeout outcome.

## Evidence E-003

Supports H-001 repair validation. `scripts/taskspace-benchmark/test-e3-score-validity.ps1` now passes with fixtures proving that `exec_timed_out=true` plus `public_validation_skipped=true`, `public_validation_skip_reason=agent_exec_timeout`, and passed pre-agent probe hash yields `agent_exec_timeout`, while missing or failed pre-agent probe yields `engineering_unclean`.

## Evidence E-004

Supports H-001 repair validation. `scripts/taskspace-benchmark/test-e3-proof-harness.ps1` now passes with a fixture proving that E3 runtime and mount proof accept validation skip only when it is backed by a passed pre-agent validator probe hash.

## Evidence E-005

Supports H-001 regression validation. `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1` and `scripts/taskspace-benchmark/test-harness.ps1` pass after the runner, timing, taxonomy, and proof changes, showing the existing invalid-harness and reporting paths still work.

## Evidence E-006

Supports H-001 runtime validation. Official materialized Terminal-Bench one-pair smoke at `target\e3-official-smoke-agent-timeout-skip-proof\terminal_bench__analyze-access-logs\20260614-234737-022` completed with runner exit `0` under `TimeoutSeconds=5`. Both Standard and TaskSpace sides recorded `exec_timed_out=true`, `public_validation_skipped=true`, `public_validation_skip_reason=agent_exec_timeout`, `pre_agent_validator_probe_status=passed`, `public_validation_duration_ms=0`, and no `validator_environment_failures`. The pair audit recorded `engineering_unclean=false`, `outcome_standard=agent_exec_timeout`, and `outcome_taskspace=agent_exec_timeout`.

## Evidence E-007

Supports H-001 proof validation. In the same official smoke, `external-e3-proof.json.validator_fidelity` recorded `official_runner_or_equivalent=true`, `agent_cannot_read_validator_source=true`, `e3_eligible=true`, `runtime_proven=true`, and `validator_mount_proven=true`. This confirms that the validation skip no longer cascades into `e3_external_validator_fidelity_unproven`.

## Evidence E-008

Supports P0 timing guardrail validation. Regenerating `sample-timing.json` for the same official smoke after the wait-attribution timing change produced `runtime_optimization_status=blocked`, `timing_quality=incomplete`, `wait_attribution_status=missing`, and blockers for `model_queue_wait_ms`, `model_retry_backoff_ms`, `model_request_duration_ms`, `process_launch_wait_ms`, `docker_token_wait_ms`, `validation_token_wait_ms`, `disk_reservation_wait_ms`, `cache_lock_wait_ms`, and `resource_wait_ms_total`. This prevents using the smoke as a speed claim while wait attribution is not instrumented.

## Evidence E-009

Supports P0 wait-attribution validation. Official materialized one-pair smoke at `target\e3-official-smoke-process-wait\terminal_bench__analyze-access-logs\20260615-000128-881` recorded agent process timing sidecars for both sides. The pair and sample timing artifacts recorded `process_launch_wait_ms=20` and no longer listed `missing_wait_attribution:process_launch_wait_ms`, while still blocking runtime optimization for uninstrumented model/API/resource waits.

## Hypothesis H-002

Claim: the runtime speed gate can allow premature parallelism because `runtime-bottleneck-report.ps1` maps unknown or unhandled bottleneck classifications to `speedup_candidate_parallelism`.

Predictions:
- The decision function will default to `speedup_candidate_parallelism` for unrecognized classes, including `unknown`.
- The plan requires unknown or missing attribution to block speed claims, not proceed to parallelism.
- A synthetic decision fixture can prove the mapping without running E3.

Diagnostic evidence plan:
- Inspect `Get-TaskspaceRuntimeSpeedDecision`.
- Add synthetic decision fixtures for `unknown`, `cleanup_bound`, `model_queue_bound`, `mixed_or_unclassified`, invalid score, blocked timing, and approved evidence.
- Fix only the decision mapping and keep timing collection behavior unchanged.

## Evidence E-010

Supports H-002 prediction 1 and 2. Inspection of `scripts/taskspace-benchmark/lib/runtime-bottleneck-report.ps1` shows the decision function handles `agent_bound`, validator/Docker classes, `queue_bound`, and `engineering_unclean_slow`, then defaults every other class to `speedup_candidate_parallelism`. The plan's section 16.11.7 requires `unknown` or missing timing/wait evidence to publish `speedup_blocked_instrumentation`.

## Evidence E-011

Supports H-002 repair validation. `scripts/taskspace-benchmark/test-e3-score-validity.ps1` now passes with synthetic decision fixtures for missing timing, invalid score, blocked instrumentation, agent-bound, validator-bound, cleanup-bound, model-queue-bound, unknown, unrecognized future class, mixed clean timing, explicit approved evidence, and governed parallel smoke evidence. Unknown and unrecognized classifications now block instrumentation instead of proceeding to parallelism.

## Evidence E-012

Supports H-002 regression validation. `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1` still passes after the speed decision mapping change, proving the existing runtime bottleneck report and aggregate timing summary fixtures continue to render `speedup_blocked_instrumentation` for missing/unavailable wait attribution.

## Hypothesis H-003

Claim: resource-governed parallelism needs a serial-default contract before any parallel worker implementation; otherwise operators can pass planned parallel flags and assume they are active or safe.

Predictions:
- The suite runner has no independent resource-governor artifact proving configured limits, observed concurrency, disk reservation, or wait accounting.
- Adding a serial-default `parallelism.json` should not change default suite behavior.
- Passing unsupported parallel flags should fail before sample scheduling with a stable exit code and artifact path.

Diagnostic evidence plan:
- Inspect suite runner parameters and artifact outputs.
- Add a pure fixture for governor config, unsupported parallel flags, disk reservation failure, and wait totals.
- Run a cheap suite-level smoke with `-MaxParallelSamples 2` and verify it exits before child scheduling with `parallelism.json`.

## Evidence E-013

Supports H-003 implementation validation. `scripts/taskspace-benchmark/lib/resource-governor.ps1` now defines serial-default resource config, serial-only guard, disk reservation probe, resource wait snapshot, and `parallelism.json` writer. `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1` passes with fixtures proving default serial config, unsupported parallel field detection, low-disk reservation fail-closed behavior, and wait total persistence.

## Evidence E-014

Supports H-003 runner validation. A cheap suite smoke using `run-taskspace-e3-suite.ps1 -MaxParallelSamples 2` exited with code `4` before child scheduling and wrote `target\e3-resource-governor-suite-smoke\suite-20260615-010713\parallelism.json` with `serial_only_status=unsupported_parallelism`. The first implementation initially used `Write-Error` under `$ErrorActionPreference=Stop`, causing exit `1`; replacing it with explicit stderr output restored the intended stable exit code.

## Hypothesis H-004

Claim: Docker image cache proof was incomplete because the cache key and manifest did not include all important immutable inputs, especially normalized validator source and adapter/uv inputs.

Predictions:
- Existing cache metadata includes fixture and Dockerfile hashes, but not normalized validator source hash or adapter hash.
- Mutating `run-tests.sh` can leave Dockerfile/fixture inputs unchanged while changing validator semantics.
- Adding validator, adapter, uv, platform/network/env fields to the key and manifest should make such mutations visible and cache-invalidating.

Diagnostic evidence plan:
- Inspect adapter cache key construction.
- Add adapter fixtures proving cache key changes after validator source and source-version mutation.
- Run generated-validator parse test and real Docker cache smoke to verify manifest fields are emitted.

## Evidence E-015

Supports H-004 repair validation. `scripts/taskspace-benchmark/test-terminal-bench-adapter-harness.ps1` now passes with offline uv-cache seed and fixtures proving Docker cache metadata schema v2 records `validator_source_sha256`, `adapter_sha256`, uv hashes, platform/network/env fields, and that cache key changes after validator source or `SourceVersion` mutation.

## Evidence E-016

Supports H-004 runtime validation. `scripts/taskspace-benchmark/test-terminal-bench-docker-cache-smoke.ps1` passed after the cache-key v2 change. The generated `docker-cache-manifest.json` in `target\terminal-bench-docker-cache-smoke\20260615-011635-814` recorded `cache_schema_version=terminal-bench-image-cache-v2`, validator source hash, adapter hash, uv hashes, platform/network/env fields, and `cache_hit=true` on the second run.

## Hypothesis H-005

Claim: runtime speed decisions need a formal calibration artifact, not only `runtime-bottleneck.md`, because operators need one stable gate for score validity, timing quality, bottleneck class, resource governor status, and speedup decision.

Predictions:
- The current module writes `runtime-bottleneck.md/json`, but not `runtime-calibration-report.md/json`.
- The suite runner can produce calibration output from existing `suite-timing.json` and `parallelism.json` without rerunning validators.
- A synthetic fixture can verify that the report blocks speed claims when wait attribution is incomplete.

Diagnostic evidence plan:
- Add `Write-TaskspaceRuntimeCalibrationReport`.
- Wire suite runner to emit it after suite timing and bottleneck report.
- Extend guardrail fixture to assert markdown and JSON decision fields.

## Evidence E-017

Supports H-005 validation. `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1` now passes with a synthetic suite timing fixture that writes `runtime-calibration-report.md/json`, renders `speedup_decision=speedup_blocked_instrumentation`, includes profile/parallelism metadata, and preserves `parallelism.resource_governor_status=pass` in JSON.

## Hypothesis H-006

Claim: validation timeout phase splitting is only complete if the runner-level markers also drive the hard score taxonomy; otherwise a pretest hang can still be misread as an agent-visible validation timeout instead of engineering-unclean infrastructure failure.

Predictions:
- A timeout before `validator_tests_started=true` must preserve `validation_timeout_phase=pretest`, classify as `public_validation_timeout`, add `no_tests_started_marker`, and return `engineering_unclean`.
- A timeout after `validator_tests_started=true` must preserve `validation_timeout_phase=tests`, classify as `public_validation_timeout`, return `engineering_unclean`, and must not add `no_tests_started_marker`.
- The fixture should run inside `test-oracle-runner-harness.ps1` without invoking a full E3 suite.

Diagnostic evidence plan:
- Source `failure-taxonomy.ps1` in the oracle runner harness.
- Convert the existing pretest/tests timeout lifecycle fixtures into synthetic metrics.
- Assert the resulting unclean reasons and agent outcome for both phases.

## Evidence E-018

Supports H-006 validation. `scripts/taskspace-benchmark/test-oracle-runner-harness.ps1` now passes with taxonomy assertions proving pretest timeout maps to `public_validation_timeout` plus `no_tests_started_marker` and `engineering_unclean`, while tests-started timeout keeps `public_validation_timeout` and `engineering_unclean` without incorrectly adding `no_tests_started_marker`. `scripts/taskspace-benchmark/test-e3-score-validity.ps1` also still passes after this fixture extension.

## Hypothesis H-007

Claim: the E3 start gate is incomplete if it only exists as a standalone command; the canonical scoring suite must invoke it before scheduling any sample, otherwise operators can still start a full scoring run without guardrail self-tests or one-pair smoke evidence.

Predictions:
- `run-taskspace-e3-suite.ps1` should run the start gate for non-PlanOnly scoring runs unless explicitly bypassed for forensics.
- Missing one-pair smoke evidence should fail the suite before sample directories are created.
- When an earlier gate fails, expensive self-tests should be skipped and recorded as skipped due to the previous failure.

Diagnostic evidence plan:
- Wire `Invoke-TaskspaceE3StartGate` into the suite entrypoint.
- Add a suite-level start-gate fixture in `test-e3-start-gate.ps1`.
- Keep existing PlanOnly suite guardrail fixtures passing.

## Evidence E-019

Supports H-007 validation. `scripts/taskspace-benchmark/test-e3-start-gate.ps1` now passes with a canonical suite fixture proving a scoring suite without one-pair smoke exits `3`, writes `suite-health.json` and `start-gate/e3-start-gate.json`, records self-tests as skipped after the previous gate failure, and creates no sample run directories. `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1` still passes, showing existing PlanOnly guardrail fixtures remain usable.

## Hypothesis H-008

Claim: runtime speed work cannot progress from planning to evidence without an opt-in sample-level parallel scheduler. The resource governor should allow only `MaxParallelSamples` at this stage, continue rejecting pair/validation/model/Docker concurrency, and preserve deterministic suite artifacts.

Predictions:
- `MaxParallelSamples=2` should no longer fail as unsupported when other concurrency knobs remain `1`.
- Pair-level, validation-level, Docker, and model concurrency should still fail closed.
- Parallel sample children must write isolated sample roots, and parent merge order must follow the task list rather than completion order.
- Parallelism metadata must identify sample parallel mode and avoid claiming serial timing comparability.

Diagnostic evidence plan:
- Extend `run-taskspace-e3-suite.ps1` with a batch scheduler for sample-level parallel child processes.
- Keep the default serial path unchanged when `MaxParallelSamples=1`.
- Add a stub-runner suite smoke using three samples and `MaxParallelSamples=2`.

## Evidence E-020

Supports H-008 validation. `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1` now passes with a `MaxParallelSamples=2` stub-runner smoke proving the suite exits cleanly, writes isolated `samples/sample-*/sample-status.json` files, merges `sample-a,sample-b,sample-c` in task-list order, and records `parallelism.json` with `serial_only_status=sample_parallel_supported`, `sample_parallel_enabled=true`, `configured.max_parallel_samples=2`, and `timing_comparison_valid=false`. `scripts/taskspace-benchmark/test-e3-start-gate.ps1` and `scripts/taskspace-benchmark/test-e3-score-validity.ps1` also still pass after the scheduler change.

## Hypothesis H-009

Claim: a parallel smoke is not acceptable evidence until it is compared against a serial baseline for score drift. Relying on a hand-set `parallel_smoke_score_drift=false` flag would allow parallel scheduling to be accepted without proving score-validity, hard outcome, audit/proof, or profile equivalence.

Predictions:
- The harness needs a deterministic serial-vs-parallel suite-health comparator.
- The comparator must fail when score-bearing fields differ for any sample.
- The sample-level parallel smoke should emit an equivalence artifact with `parallel_smoke_score_drift=false` only when serial and parallel outputs match.

Diagnostic evidence plan:
- Add a small `parallel-diff.ps1` helper for suite-health equivalence.
- Extend the sample parallel smoke to run both serial and `MaxParallelSamples=2` suites with the same stub runner.
- Add a negative drift fixture that mutates one sample outcome and proves the comparator catches it.

## Evidence E-021

Supports H-009 validation. `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1` now passes with serial and sample-parallel stub suites compared by `scripts/taskspace-benchmark/lib/parallel-diff.ps1`. The generated `serial-vs-parallel-equivalence.json` records `comparable=true`, `parallel_smoke_score_drift=false`, `drift_count=0`, and compared samples `sample-a,sample-b,sample-c`; the negative fixture changes `sample-b.run_validity` and is detected as drift. `scripts/taskspace-benchmark/test-e3-score-validity.ps1` still passes after adding the comparator.

## Hypothesis H-010

Claim: a full 15-task E3 or runtime speedup conclusion is still unsafe if it only has parallel smoke evidence. The harness also needs a calibration gate that proves one-pair timing, representative serial calibration, and serial-vs-parallel equivalence all exist before allowing expensive full E3 execution or speed claims.

Predictions:
- Missing one-pair timing evidence must block `full_e3_allowed`.
- Missing or too-small serial calibration evidence must block `speed_claim_allowed`.
- Parallel score drift must block both full E3 and speed claims.
- Complete timing plus equivalence evidence should produce a machine-readable `calibration-gate.json` with `status=pass`.

Diagnostic evidence plan:
- Add `scripts/taskspace-benchmark/lib/calibration-gate.ps1`.
- Extend the guardrails self-test with complete, missing one-pair, and parallel-drift fixtures.
- Add the runtime calibration/speed plan to the 0.0.4 plan and acceptance checklist so future E3 execution is gated by the same artifact contract.

## Evidence E-022

Supports H-010 validation. `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1` now passes with a calibration gate fixture proving complete timing/equivalence evidence sets `status=pass`, `full_e3_allowed=true`, and `speed_claim_allowed=true`; a missing one-pair root fails with `one_pair_root_missing`; and a drifted equivalence artifact fails with `parallel_score_drift`. `scripts/taskspace-benchmark/test-e3-score-validity.ps1` and `git diff --check` also pass after the calibration gate and documentation updates.

## Hypothesis H-011

Claim: calibration evidence is not a real guardrail until the canonical scoring suite invokes it before sample scheduling. A library-only `Invoke-TaskspaceCalibrationGate` plus synthetic unit fixture can still let operators launch hours-long full E3 runs without serial calibration or parallel equivalence.

Predictions:
- `run-taskspace-e3-suite.ps1 -ScoringMode` must pass `SerialCalibrationRoot` and `ParallelEquivalencePath` into the start gate.
- `Invoke-TaskspaceE3StartGate` must call `Invoke-TaskspaceCalibrationGate` before self-tests and before any sample scheduling.
- An aggregate-only one-pair smoke root must fail calibration because it lacks `pair-timing.json`, `sample-timing.json`, and `runtime-bottleneck.md`.
- A scoring suite with missing calibration artifacts must exit `3`, write start-gate/suite-health artifacts, and create zero sample directories.
- Parallel equivalence must require `comparable=true` and `drift_count=0`, not only `parallel_smoke_score_drift=false`.

Diagnostic evidence plan:
- Wire calibration gate into `e3-start-gate.ps1`.
- Add suite/start-gate CLI parameters for serial calibration and parallel equivalence evidence.
- Extend `test-e3-start-gate.ps1` with aggregate-only and suite-level missing-calibration fail-closed fixtures.
- Extend `test-e3-harness-guardrails.ps1` with a non-comparable equivalence negative fixture.
- Run focused start-gate, guardrails, score-validity, and diff-check validation.

## Evidence E-023

Supports H-011 validation. `scripts/taskspace-benchmark/test-e3-start-gate.ps1` now passes with fixtures proving complete calibration evidence passes, an aggregate-only one-pair root fails `calibration_one_pair_smoke`, and `run-taskspace-e3-suite.ps1 -ScoringMode -OnePairSmokeRoot <aggregate-only>` exits `3` before creating sample directories. `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1` now also proves `comparable=false` fails with `parallel_not_comparable`. `scripts/taskspace-benchmark/test-e3-score-validity.ps1` and `git diff --check` pass after the suite/start-gate wiring.

## Hypothesis H-012

Claim: the scoring suite still has a score-bearing bypass if `-SkipStartGate` can be combined with `-ScoringMode` or `-RequireScoreValidity`. Calibration/start-gate enforcement is only closed when bypassing the gate is illegal for score-bearing non-PlanOnly runs.

Predictions:
- `run-taskspace-e3-suite.ps1 -ScoringMode -SkipStartGate` exits `4` before suite-root creation or sample scheduling.
- `run-taskspace-e3-suite.ps1 -RequireScoreValidity -SkipStartGate` exits `4` before suite-root creation or sample scheduling.
- `-PlanOnly -SkipStartGate` can remain available as a dry-run escape because it is not score-bearing.

Diagnostic evidence plan:
- Add an early suite guard rejecting `($ScoringMode -or $RequireScoreValidity) -and $SkipStartGate -and -not $PlanOnly`.
- Add start-gate self-test fixtures that capture child process exit codes for both score-bearing bypass attempts.
- Run a fresh closure review focused only on the remaining bypass.

## Evidence E-024

Supports H-012 validation. `scripts/taskspace-benchmark/test-e3-start-gate.ps1` now passes with negative fixtures proving both `-ScoringMode -SkipStartGate` and `-RequireScoreValidity -SkipStartGate` exit `4` with the explicit `SkipStartGate is not allowed` message. The Round 3 adversarial closure review in `vs_review/2026-06-15-e3-calibration-gate-review.md` found no remaining blocking findings and confirmed the guard runs before suite-root/sample scheduling.

## Hypothesis H-013

Claim: calibration artifacts can be stale or from the wrong run unless the gate can compare expected identity fields. A passing gate based only on file presence and timing field presence is not enough to prevent unrelated one-pair, serial calibration, or parallel equivalence artifacts from authorizing a full E3 run.

Predictions:
- When expected identity is provided, one-pair timing must include matching `task_list_hash`, `source_version`, and `profile_hash`.
- Serial calibration timing must enforce the same optional identity fields.
- Parallel equivalence must enforce the same optional identity fields.
- A mismatched expected task-list hash should fail the gate before full E3 is allowed.

Diagnostic evidence plan:
- Add optional expected identity parameters to `Invoke-TaskspaceCalibrationGate`.
- Add identity checks to all three calibration sub-gates.
- Extend the guardrails fixture with matching identity pass and mismatched identity fail.

## Evidence E-025

Supports H-013 validation at the gate layer. `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1` now passes with complete identity fields on one-pair timing, serial timing, and parallel equivalence artifacts, and a negative fixture proves `ExpectedTaskListHash=task-list-b` fails with `one_pair_smoke_identity_mismatch:task_list_hash`. This does not yet complete end-to-end identity enforcement; suite/start-gate still need to compute real task-list/profile identity and real timing/equivalence producers must emit those fields.

## Hypothesis H-014

Claim: calibration identity becomes an effective guardrail only when the canonical suite computes task-list/profile identity, passes it into the start gate, and real timing/equivalence producers persist the same fields. A gate-only fixture can still be bypassed by stale or manually assembled artifacts.

Predictions:
- `run-taskspace-e3-suite.ps1` should compute a SHA256 task-list hash and a stable profile hash before invoking the start gate.
- `Invoke-TaskspaceE3StartGate` should pass expected task-list/source/profile identity into `Invoke-TaskspaceCalibrationGate`.
- `pair-timing.json`, `sample-timing.json`, `suite-timing.json`, and `serial-vs-parallel-equivalence.json` should carry identity fields from real producers.
- A mismatched expected task-list hash should fail at the start gate before sample scheduling.
- Start gate should emit `gate-decision.json` so operators have a machine-readable next-command decision instead of relying on chat notes.

Diagnostic evidence plan:
- Add a small shared identity helper for SHA256 file hash and stable profile JSON hash.
- Thread identity through suite, external runner, benchmark runner, timing writers, parallel diff writer, and start gate.
- Extend start-gate and guardrails self-tests for identity mismatch and `gate-decision.json`.
- Run parser checks plus focused start-gate, guardrails, and score-validity tests.

## Evidence E-026

Supports H-014 validation for the canonical suite/start-gate path. `scripts/taskspace-benchmark/lib/e3-identity.ps1` now provides stable hash helpers. `run-taskspace-e3-suite.ps1` computes task-list hash and profile hash, passes them to `Invoke-TaskspaceE3StartGate`, and forwards them through child runs. `run-taskspace-external-benchmark.ps1` and `run-taskspace-benchmark.ps1` pass those fields into timing writers. `scripts/taskspace-benchmark/lib/timing.ps1` writes `task_list_hash`, `source_version`, and `profile_hash` into pair/sample/suite timing artifacts. `scripts/taskspace-benchmark/lib/parallel-diff.ps1` writes the same identity fields into equivalence artifacts. `scripts/taskspace-benchmark/lib/e3-start-gate.ps1` passes expected identity into `Invoke-TaskspaceCalibrationGate` and writes `gate-decision.json`.

Validation passed:
- PowerShell parser check for touched scripts.
- `.\scripts\taskspace-benchmark\test-e3-start-gate.ps1`
- `.\scripts\taskspace-benchmark\test-e3-harness-guardrails.ps1`
- `.\scripts\taskspace-benchmark\test-e3-score-validity.ps1`

Remaining scope: this does not complete the full Section 16.13 runtime plan. `calibration-selection.json`, previous-run reconstruction, final full E3 release gate evidence, and official one-pair/3-task calibration artifacts remain open.

## Hypothesis H-015

Claim: the representative three-task serial calibration cannot be trusted until the suite writes a deterministic `calibration-selection.json` that records the source task list hash, selected task IDs/families, subset hash, and excluded-task rationale. Without this artifact, a calibration subset can be cherry-picked or accidentally incomparable.

Predictions:
- A task list with task-family metadata should select at most one task from each family in deterministic family order, then fill remaining slots by task order.
- The artifact must include the source task-list hash and a stable subset hash.
- Excluded tasks must retain a machine-readable rationale.
- `run-taskspace-e3-suite.ps1` should write `calibration-selection.json` for canonical suite runs before scheduling samples.

Diagnostic evidence plan:
- Add `scripts/taskspace-benchmark/lib/calibration-selection.ps1`.
- Add a guardrails fixture with multiple families and a duplicate-family task.
- Assert the suite writes the artifact during the existing serial/parallel smoke fixture.

## Evidence E-027

Supports H-015 validation for artifact production. `scripts/taskspace-benchmark/lib/calibration-selection.ps1` now writes deterministic selection artifacts with `source_task_list_hash`, `subset_task_list_hash`, `selected_task_ids`, `selected_task_families`, `selected_tasks`, and `excluded_tasks`. `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1` writes `calibration-selection.json` under the suite root and records the path in `suite-health.json`. `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1` now validates the selection rule and confirms a suite smoke writes a complete selection artifact.

Validation passed:
- PowerShell parser check for `calibration-selection.ps1`, `run-taskspace-e3-suite.ps1`, and `test-e3-harness-guardrails.ps1`.
- `.\scripts\taskspace-benchmark\test-e3-harness-guardrails.ps1`
- `.\scripts\taskspace-benchmark\test-e3-start-gate.ps1`
- `.\scripts\taskspace-benchmark\test-e3-score-validity.ps1`

Remaining scope: this still does not produce official one-pair or representative three-task calibration evidence. Previous-run reconstruction, validator/Docker overhead proof, governed parallel smoke release evidence, and full E3 release-gate approval remain open.

## Hypothesis H-016

Claim: R0 runtime reconstruction must be a read-only artifact generator over an existing suite root. It should not rewrite historical pair/sample/suite artifacts, and it must compute first-invalid waste plus a bottleneck classification even when later samples are explicitly skipped.

Predictions:
- The reconstruction output should live under a separate `runtime-reconstruction/<timestamp-or-label>/` root.
- A suite with first sample `invalid_harness`, a later completed sample, and a later skipped sample should compute `time_after_first_invalid_ms` from the completed later sample only.
- Explicitly skipped samples should not be treated as missing timing.
- The reconstruction JSON/Markdown should name a bottleneck class such as `invalid_waste_bound` or `unknown` when timing fields are missing.

Diagnostic evidence plan:
- Add `scripts/taskspace-benchmark/lib/runtime-reconstruction.ps1`.
- Add `scripts/taskspace-benchmark/reconstruct-e3-runtime.ps1` CLI.
- Add a guardrails fixture for first-invalid waste and isolated output root.

## Evidence E-028

Supports H-016 validation for reconstruction tooling. `scripts/taskspace-benchmark/lib/runtime-reconstruction.ps1` scans `suite-health.json`, `suite-timing.json`, and sample timing files, then writes `runtime-reconstruction.json` and `runtime-reconstruction.md` under a separate output root. `scripts/taskspace-benchmark/reconstruct-e3-runtime.ps1` exposes the helper as an operator command. `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1` now verifies the first-invalid waste fixture: invalid first sample, completed second sample with 2000ms timing, skipped third sample with no timing, and classification `invalid_waste_bound`.

Validation passed:
- PowerShell parser check for `runtime-reconstruction.ps1`, `reconstruct-e3-runtime.ps1`, and `test-e3-harness-guardrails.ps1`.
- `.\scripts\taskspace-benchmark\test-e3-harness-guardrails.ps1`
- `.\scripts\taskspace-benchmark\test-e3-start-gate.ps1`
- `.\scripts\taskspace-benchmark\test-e3-score-validity.ps1`

Remaining scope: reconstruction tooling exists, but it has not yet been run against the previous invalid official E3 suite root. Validator/Docker overhead proof, governed parallel smoke release evidence, official calibration artifacts, and final full E3 release gate remain open.

## Evidence E-029

Extends H-016 with a real non-selftest reconstruction run. `scripts/taskspace-benchmark/reconstruct-e3-runtime.ps1` was executed against `target/suite-disk-preflight-smoke-3/suite-20260613-164323`. It wrote:

- `target/suite-disk-preflight-smoke-3/suite-20260613-164323/runtime-reconstruction/20260615-030246/runtime-reconstruction.json`
- `target/suite-disk-preflight-smoke-3/suite-20260613-164323/runtime-reconstruction/20260615-030246/runtime-reconstruction.md`

Observed output:
- `bottleneck_classification=unknown`
- `first_invalid_sample_index=0`
- `time_after_first_invalid_ms=0`
- `missing_fields=["suite-timing.json"]`
- `suite_health.status=aborted`
- `suite_abort_reason=harness_materialization_failure/disk_space_low`

Interpretation: the disk-preflight smoke aborted before sample scheduling/timing finalization. R0 reconstruction correctly blocks speed conclusions because suite timing is missing, while preserving the disk-space engineering failure as the suite-health cause. This is useful evidence for instrumentation closure: pre-scheduling aborts need either a minimal suite timing artifact or explicit reconstruction support for pre-timing aborts.

Legacy gap: `target/e3-full-20260606-014919` appears to be an older run-root layout with per-sample/pair artifacts but no canonical `suite-health.json` or `suite-timing.json`; the current R0 reconstructor cannot directly reconstruct it. A legacy importer or the actual canonical full E3 suite root is still needed before claiming the previous full E3 run has been reconstructed.

## Hypothesis H-017

Claim: pre-scheduling early aborts must write minimal timing and reconstruction artifacts. Otherwise, the runtime plan can discover an engineering problem but still fail to produce the evidence needed to explain or gate the next command.

Predictions:
- Start-gate failures should write `suite-health.json`, `suite-timing.json`, runtime bottleneck report, and runtime calibration report before exiting `3`.
- Disk-reservation failures before sample scheduling should write the same minimal suite artifacts before exiting `3`.
- Runtime reconstruction over these early-abort suite roots should produce one invalid suite-level sample row, `first_invalid_sample_index=0`, no false missing `suite-timing.json`, and `invalid_waste_bound`.

Diagnostic evidence plan:
- Add a suite early-abort artifact writer in `run-taskspace-e3-suite.ps1`.
- Fix `runtime-reconstruction.ps1` so single-element `sample_statuses` is treated as an array and suite-level abort rows do not require nonexistent per-sample timing.
- Extend `test-e3-start-gate.ps1` to assert suite timing and reconstruction sample-row behavior.
- Run a disk-reservation early-abort smoke through the canonical suite driver.

## Evidence E-030

Supports H-017 for start-gate and disk-reservation early-abort paths. `run-taskspace-e3-suite.ps1` now writes early abort artifacts before exiting `3`. `runtime-reconstruction.ps1` now preserves single-row `suite-health.sample_statuses`, reports `first_invalid_sample_index=0`, and does not create a false missing `sample-timing` entry for a suite-level abort row. `test-e3-start-gate.ps1` asserts `suite-timing.json`, reconstruction sample rows, and first-invalid index for start-gate failures.

Validation passed:
- PowerShell parser check for `runtime-reconstruction.ps1`, `run-taskspace-e3-suite.ps1`, and `test-e3-start-gate.ps1`.
- `.\scripts\taskspace-benchmark\test-e3-start-gate.ps1`
- `.\scripts\taskspace-benchmark\test-e3-harness-guardrails.ps1`
- `.\scripts\taskspace-benchmark\test-e3-score-validity.ps1`

Disk-reservation smoke:
- Command category: canonical suite driver with `-PlanOnly -SkipStartGate -DiskReserveGb 999999999`
- Suite root: `target/e3-disk-early-abort-timing-smoke-2/suite-20260615-032809`
- Exit code: `3`
- `suite-timing.json`: present
- Reconstruction output root: `target/e3-disk-early-abort-timing-smoke-2/suite-20260615-032809/runtime-reconstruction/smoke`
- Observed reconstruction: `first_invalid_sample_index=0`, `sample_rows=1`, `missing_fields=[]`, `bottleneck_classification=invalid_waste_bound`

Remaining scope: this does not close first-pair invalid fast-fail, validator/Docker overhead proof, governed parallel smoke release evidence, official one-pair/three-task calibration artifacts, legacy full-run import, or final full E3 release-gate approval.

## Hypothesis H-018

Claim: once a child sample returns `invalid_harness` in score-bearing suite flow, the suite driver must invalidate the suite, emit one suite invalidation event, and skip later samples. Otherwise a known engineering-unclean result can still consume the rest of the task list.

Predictions:
- A child runner that writes `sample-status.json` with `run_validity=invalid_harness` and exits `3` should make the suite exit `3`.
- The next sample should be materialized only as a skipped sample status, not executed by the child runner.
- `suite-health.json` should record `status=invalid_harness`, `suite_score_valid=false`, `remaining_samples_skipped=1`, and count the skipped invalid sample.
- `events.jsonl` should contain exactly one `suite_score_invalidated` event with the remaining skipped count.

Diagnostic evidence plan:
- Add a fixture to `test-e3-harness-guardrails.ps1` with a two-sample task list and a stub runner that always writes child `invalid_harness` status then exits `3`.
- Run the fixture through the canonical suite driver in `-PlanOnly -ScoringMode -SkipStartGate` mode so the test exercises suite scheduling and circuit-breaker behavior without requiring real E3 calibration evidence.

## Evidence E-031

Supports H-018 for fixture-level suite circuit-breaker behavior. `test-e3-harness-guardrails.ps1` now includes a `suite-child-invalid` fixture. The first sample's stub runner writes `run_validity=invalid_harness`, `abort_signature=harness_materialization_failure/stub_score_invalid`, and exits `3`; the suite driver exits `3`, writes invalid suite health, skips `sample-b`, and emits one `suite_score_invalidated` event.

Validation passed:
- PowerShell parser check for `test-e3-harness-guardrails.ps1`.
- `.\scripts\taskspace-benchmark\test-e3-harness-guardrails.ps1`
- `.\scripts\taskspace-benchmark\test-e3-start-gate.ps1`
- `.\scripts\taskspace-benchmark\test-e3-score-validity.ps1`

Remaining scope: this is a fixture-level proof, not an official Terminal-Bench invalid case. Validator lifecycle split, Docker cache proof edge cases, governed parallel smoke release evidence, official one-pair/three-task calibration artifacts, legacy full-run import, and final full E3 release-gate approval remain open.

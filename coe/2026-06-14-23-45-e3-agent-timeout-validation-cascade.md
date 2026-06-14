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

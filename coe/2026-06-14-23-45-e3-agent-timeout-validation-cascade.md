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

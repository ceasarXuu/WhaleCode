# Problem P-001

Status: confirmed

Symptoms:
- E3 runtime-reduction guardrail claims include validator pretest/test timeout splitting, but score-bearing public validation still uses one timeout.
- `validation_timeout_phase` can classify a timeout after `tests_started` as `pretest` when process output is lost on kill.
- Generated Terminal-Bench validator cleanup is timed as a phase but is not itself bounded.

Expected behavior:
- Public validation must fail fast when it does not reach `tests_started` within the pretest budget.
- Once `tests_started` is durably observed, the test budget applies and timeout phase is classified as `tests`.
- Cleanup commands must have bounded execution and observable duration/classification.

Actual behavior:
- `ValidationPretestTimeoutSeconds` is only applied to `-ProbeOnly`.
- Public validation calls use only `ValidationTimeoutSeconds`.
- Timeout handling kills the process before stdout/stderr are persisted, then writes timeout stderr, so lifecycle markers can be lost.
- Generated validator cleanup calls Docker directly in `finally`.

Impact:
- A malformed validator can still waste the full validation timeout per side.
- Timeout phase evidence can be wrong, weakening engineering-clean decisions and runtime bottleneck analysis.
- Cleanup hangs can consume the outer validation timeout.

Fix criteria:
- Runner-level tests prove pretest timeout applies to score-bearing validation.
- Runner-level tests prove a post-`tests_started` hang is classified as `tests`.
- Cleanup commands in the generated validator are bounded and report phase duration/classification.
- Timing metrics use reliable phase evidence.

# Hypothesis H-001

Status: confirmed

Claim:
- The root cause is that `Invoke-TaskspaceValidationCommand` delegates public validation to `Invoke-RealProcess`, which only supports one wall-clock timeout and persists stdout/stderr only after normal process exit.

Predictions:
- The runner will pass `ValidationPretestTimeoutSeconds` only to probe execution.
- Public validation will pass only one timeout to `Invoke-TaskspaceValidationCommand`.
- On timeout, `Invoke-RealProcess` will throw before writing captured stdout/stderr.

Diagnostic evidence plan:
- Inspect the runner call sites and process execution helper.
- Use reviewer evidence and local code inspection to confirm whether the mechanism explains both slow failure and wrong timeout classification.

# Evidence E-001

Type: code-inspection

Supports:
- H-001 predictions 1 and 2.

Observation:
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1` uses `ValidationPretestTimeoutSeconds` for the `-ProbeOnly` call, while score-bearing public validation uses `$effectiveValidationTimeout` derived only from `ValidationTimeoutSeconds`.

# Evidence E-002

Type: code-inspection

Supports:
- H-001 prediction 3.

Observation:
- `scripts/action-map-real-user-e2e-lib.ps1` starts async stdout/stderr reads, kills and throws on timeout, and writes stdout/stderr only after normal process exit. `scripts/taskspace-benchmark/lib/oracle-runner.ps1` then creates blank stdout if needed and writes timeout stderr.

# Evidence E-003

Type: adversarial-review

Supports:
- H-001 root-cause mechanism.

Observation:
- Three fresh read-only reviewers independently reported the same blocking mechanism: real public validation has no pretest/test split, timeout marker evidence is not durable, and generated validator cleanup is not bounded. See `vs_review/2026-06-14-e3-validator-timing-review.md`.

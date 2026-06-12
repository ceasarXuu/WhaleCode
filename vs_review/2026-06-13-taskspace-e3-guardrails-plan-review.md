# TaskSpace E3 Guardrails Plan Adversarial Review

- Created: 2026-06-13
- Updated: 2026-06-13
- Status: In progress
- Review type: Independent adversarial documentation/engineering-plan review
- Review round policy: one reviewer per round, fresh session, no inherited context

## Review Input

Primary artifact under review:

- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md`

Code and evidence anchors supplied to the reviewer:

- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1`
- `scripts/taskspace-benchmark/adapters/terminal-bench-uv-cache.ps1`
- `scripts/taskspace-benchmark/lib/metrics-extractor.ps1`
- `scripts/taskspace-benchmark/lib/failure-taxonomy.ps1`
- `scripts/taskspace-benchmark/lib/pair-report.ps1`
- `scripts/taskspace-benchmark/lib/aggregate-report.ps1`
- `scripts/taskspace-benchmark/lib/e3-proof.ps1`
- `coe/2026-06-07-05-18-taskspace-p0-benchmark-harness.md`

Review focus:

- Whether the plan is concrete enough to guide implementation.
- Whether guardrail phases map to real runner/adapter/report control flow.
- Whether abort policy prevents wasted E3 time without misclassifying real benchmark failures.
- Whether suite-level circuit breaker has a viable implementation entrypoint.
- Whether acceptance criteria catch the historical uv-cache relative path failure and similar infra failures with minimal-cost tests.

## Reviewer Selection

- Reviewer role: `documentation-skill-adversary`
- Rationale: The artifact is an engineering plan meant to guide future development. The highest risk is an apparently detailed plan that still misses concrete implementation seams, false-positive/false-negative guardrails, or testable acceptance criteria.
- Reviewer count this round: 1

## Launch Records

### Round 1

- Launched: 2026-06-13 02:08 +08:00
- Agent id: `019ebd07-e93e-7000-aa36-c957cf11483c`
- Agent nickname: `Bacon`
- Agent type: `explorer`
- Context mode: fresh session, `fork_context=false`
- Model override: none
- Instructions: read the plan and listed code anchors directly, perform read-only adversarial review, output findings by severity with evidence and verdict.

### Round 2

- Launched: 2026-06-13 02:08 +08:00
- Agent id: `019ebd10-27cb-78b1-864f-920dff58bce7`
- Agent nickname: `Locke`
- Agent type: `explorer`
- Context mode: fresh session, `fork_context=false`
- Model override: none
- Instructions: closure review only; verify whether Round 1 blocking and major findings are sufficiently closed by the revised plan.

## Reviewer Outputs

### Round 1

Verdict: `FAIL_BLOCKING`

Findings:

1. `blocking`: The abort policy depends on `tests_started`, but current Terminal-Bench generated validator does not emit `tests_started` / `tests_completed`; it only runs `bash /tests/run-tests.sh`. Metrics also has no lifecycle fields. Evidence cited by reviewer: plan lines 124-132 before revision, `terminal-bench-adapter.ps1:273-287`, `metrics-extractor.ps1:263-281`.
2. `blocking`: Suite-level circuit breaker was promised while the plan left canonical multi-sample driver unresolved. Existing wrappers run one external task or finalize one run, and none owns suite state or signature counts. Evidence cited by reviewer: plan lines 271-283 and 432 before revision, `run-taskspace-e3-external.ps1:25-48`, `run-taskspace-external-benchmark.ps1:33-53`, `finalize-taskspace-e3-run.ps1:37-65`.
3. `major`: Static preflight was placed only inside `run-taskspace-benchmark.ps1`, but external materialization happens before that runner exists. Adapter failures would have no runner `runDir` for `harness-health.json`.
4. `major`: Early Docker/backend failures can occur before Docker result JSON is emitted; metrics currently classifies many Docker failures only from that JSON or timeout.
5. `major`: Exit and resume semantics were still unresolved. Current code has exit `2` for ineligible and exit `1` for failed pairs, with no invalid-harness exit code.
6. `minor`: The current uv-cache regression test already captures the historical relative-path bug cheaply; the plan should preserve and extend it as the named regression anchor.

### Round 2

Verdict: `PASS`

Findings:

- No closure-affecting findings.

Closure notes:

- Round 1 blocking lifecycle-marker finding is closed. The revised plan now specifies exact validator lifecycle marker injection and metrics fields for `tests_started` / `tests_completed`.
- Round 1 blocking suite-driver finding is closed. The revised plan now names `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1` as the canonical multi-sample suite driver and defines parameters, child invocation, suite artifacts, skipped sample records, circuit breaker triggers, and exit behavior.
- Round 1 major findings are closed. The revised plan covers wrapper-level materialization health, early Docker/backend failures before Docker result JSON exists, invalid-harness exit code `3`, resume/finalize guard behavior, and preservation of the uv-cache regression fixture.
- Reviewer stated this was read-only review only: no files changed, no tests run, no external reviewer launched.

## Main-Agent Response

Accepted all findings.

Plan revisions applied:

- Added required Terminal-Bench lifecycle marker contract, including exact entry-script behavior for `validator_lifecycle_stage=tests_started`, `validator_tests_started=true`, `validator_lifecycle_stage=tests_completed`, and `validator_tests_completed=true`.
- Added required metrics fields: `tests_started_seen`, `tests_completed_seen`, `validation_lifecycle_stage`, `public_validation_reached_tests`, `pretest_failure`, and `infra_signature`.
- Required sentinel abort logic to consume lifecycle fields and never infer pre-test failure from exit code alone.
- Added wrapper-level materialization health in `run-taskspace-external-benchmark.ps1` for adapter failures before the inner runner exists.
- Required top-level validator try/catch and `validator-probe-result.json` creation before Docker/path operations, plus stderr fallback parsing for Docker/path/cache/source failures.
- Made `run-taskspace-e3-suite.ps1` the concrete canonical multi-sample suite driver and defined parameters, child invocation, suite artifacts, skipped sample record format, and exit behavior.
- Fixed `invalid_harness` process exit code to `3`.
- Required `run-status.json` and `sample-status.json`, plus `ResumeLatest` / `RunId` / finalize guard behavior.
- Preserved `test-terminal-bench-uv-cache-harness.ps1` as the named historical regression anchor.

## Closure Status

Round 1 closed with accepted blocking findings. Round 2 closure review required after plan revision.
Round 2 passed. No further review round required.

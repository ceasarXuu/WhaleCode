# TaskSpace E3 Guardrails Completion Audit

- Created: 2026-06-15
- Source plan: `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md`
- Scope: current-state audit of Section 16 canonical runtime sequence plus the hard clean-execution repair addendum.
- Status: partial, not complete

## Audit Method

This audit treats completion as unproven unless there is current code, test, report, or review evidence. Synthetic tests count only for the exact behavior they cover. A full E3 score or speed claim remains invalid without the required calibration artifacts.

## Requirement Matrix

| Plan Area | Requirement | Current Evidence | Status | Gap / Next Action |
|---|---|---|---|---|
| Hard clean execution | Docker, validator, proof, path, disk, report-generation failures are engineering-unclean and not score-bearing model outcomes. | `test-e3-score-validity.ps1`, `failure-taxonomy.ps1`, `aggregate-report.ps1`, COE E-018/E-019/E-023. | implemented for covered fixtures | Keep extending taxonomy fixtures when new infra signatures appear. |
| Start gate | Score-bearing suite runs must fail before scheduling when start/calibration evidence is missing. | `run-taskspace-e3-suite.ps1`, `e3-start-gate.ps1`, `test-e3-start-gate.ps1`, `vs_review/2026-06-15-e3-calibration-gate-review.md`. | implemented for canonical suite path | Direct helper entrypoints are non-scoring/dev-only until they enforce the same calibration identity gate; they must not authorize final E3 scoring evidence. |
| Calibration identity | Calibration artifacts should match expected task-list/source/profile identity when those expectations are supplied. | `calibration-gate.ps1`, `test-e3-harness-guardrails.ps1`. | partial | Gate supports optional identity checks and mismatch fixture; suite/start-gate still need to compute and pass real task-list/profile identity, and timing/equivalence producers need to emit it from real runs. |
| P0 timing evidence | Pair, sample, and suite timing artifacts exist; missing timing blocks speed conclusions. | `timing.ps1`, `runtime-bottleneck-report.ps1`, `test-e3-harness-guardrails.ps1`, `test-e3-score-validity.ps1`. | implemented for synthetic fixtures | Need official one-pair calibration artifact from a real Terminal-Bench sample before full E3/speed claims. |
| P0 wait attribution | Model/API/resource wait fields are present or explicitly unavailable; unknown attribution blocks speed claims. | `test-e3-score-validity.ps1` asserts blocked status when wait attribution is incomplete. | implemented for fixtures | Need real-run evidence showing unavailable fields are rendered and not treated as pass. |
| P1 agent-timeout skip | Agent execution timeout can skip public validation only with passed pre-agent proof; otherwise engineering-unclean. | `run-taskspace-benchmark.ps1`, `e3-proof.ps1`, `failure-taxonomy.ps1`, `test-e3-score-validity.ps1`, `test-e3-proof-harness.ps1`. | implemented for fixtures | Need one real short-timeout smoke to prove no downstream validator timeout is created. |
| P2 fast-fail invalid full runs | Hard engineering-unclean stops later work and suppresses score language. | `run-taskspace-e3-suite.ps1`, `suite-status.ps1`, `test-e3-harness-guardrails.ps1`, `test-e3-score-validity.ps1`. | implemented for fixtures | Need representative official invalid case artifact for release evidence. |
| P3 Docker/validator overhead | Cache requires immutable proof; pretest/test timeout split; finalize does not rerun validators. | `terminal-bench-adapter.ps1`, `test-terminal-bench-adapter-harness.ps1`, `test-terminal-bench-docker-cache-smoke.ps1`, `test-oracle-runner-harness.ps1`. | mostly implemented | Cache edge cases remain thin: ARG-based FROM, lowercase/multistage, package metadata drift, and no-FROM cases need fixtures. |
| P4 governed parallelism | Sample-level parallelism is opt-in, isolated, deterministic, resource-governed, and serial-vs-parallel compared. | `resource-governor.ps1`, `run-taskspace-e3-suite.ps1`, `parallel-diff.ps1`, `test-e3-harness-guardrails.ps1`, `calibration-gate.ps1`. | partially implemented | Only sample-level parallelism is implemented. Comparator does not yet prove every field in the full Section 16 list because some profile/config/artifact hashes are not always present in `suite-health.json`. |
| P4 observed concurrency | `parallelism.json` records configured and observed concurrency. | `resource-governor.ps1`, `test-e3-harness-guardrails.ps1`. | implemented for sample-level parallel fixture | Future Docker/model token managers must replace placeholder observed values when those modes are implemented. |
| P5 speed decision | Runtime calibration report gives defensible answer to why 15-task E3 takes hours and whether speedup is possible. | `runtime-bottleneck-report.ps1`, `runtime-calibration-report.md/json` fixture checks, calibration gate. | partial | No current official 3-task serial calibration plus parallel smoke artifact has been run under this final gate. Speed claims remain blocked. |
| Runtime bottleneck repair plan | The plan must explain why a 15-task E3 can take hours, how to identify the dominant bottleneck, and which speedups are allowed without changing the v0.0.4 scoring profile. | Section 16.13 of the implementation plan. | documented, not implemented | Execute R0-R5: reconstruct previous timing, close instrumentation, eliminate invalid-run waste, reduce validator/Docker overhead, produce `calibration-selection.json` and `gate-decision.json`, run governed parallel smoke, then pass the full E3 release gate. |

## Current No-Go Conditions

- Do not mark the full implementation plan complete.
- Do not run or report full E3 score unless `gate-decision.json` says `next_allowed_command_category=full_e3`, `full_e3_allowed=true`, `calibration-gate.json status=pass`, and task-list/source/profile identity checks match.
- Do not claim large speedup without serial baseline, comparable parallel smoke, and timing reconciliation.
- Do not enable pair-level, validation-level, Docker, or model concurrency; only sample-level parallelism is implemented.

## Next Engineering Actions

1. Add missing Docker cache proof edge-case fixtures.
2. Wire calibration identity end-to-end: suite/start-gate compute task-list hash and profile hash; timing/equivalence producers emit those fields; gate receives them for score-bearing runs.
3. Extend `parallel-diff.ps1` or suite health production so profile/config/prompt/proof artifact hashes required by Section 16 are actually compared.
4. Execute Section 16.13 R0-R5 in order: previous-run runtime reconstruction, instrumentation closure, invalid-run fast-fail proof, validator/Docker overhead reduction proof, deterministic 3-task calibration selection, governed sample-parallel smoke, and final release gate.
5. Run one official one-pair timing smoke and one representative 3-task serial calibration only when `gate-decision.json` says those command categories are allowed.
6. Only after the above, run full 15-task E3 when `next_allowed_command_category=full_e3`, `full_e3_allowed=true`, and identity checks match.

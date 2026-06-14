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
| Start gate | Score-bearing suite runs must fail before scheduling when start/calibration evidence is missing. | `run-taskspace-e3-suite.ps1`, `e3-start-gate.ps1`, `test-e3-start-gate.ps1`, `vs_review/2026-06-15-e3-calibration-gate-review.md`. | implemented | Direct helper still has explicit skipped-calibration dev policy; document before broader operator use. |
| P0 timing evidence | Pair, sample, and suite timing artifacts exist; missing timing blocks speed conclusions. | `timing.ps1`, `runtime-bottleneck-report.ps1`, `test-e3-harness-guardrails.ps1`, `test-e3-score-validity.ps1`. | implemented for synthetic fixtures | Need official one-pair calibration artifact from a real Terminal-Bench sample before full E3/speed claims. |
| P0 wait attribution | Model/API/resource wait fields are present or explicitly unavailable; unknown attribution blocks speed claims. | `test-e3-score-validity.ps1` asserts blocked status when wait attribution is incomplete. | implemented for fixtures | Need real-run evidence showing unavailable fields are rendered and not treated as pass. |
| P1 agent-timeout skip | Agent execution timeout can skip public validation only with passed pre-agent proof; otherwise engineering-unclean. | `run-taskspace-benchmark.ps1`, `e3-proof.ps1`, `failure-taxonomy.ps1`, `test-e3-score-validity.ps1`, `test-e3-proof-harness.ps1`. | implemented for fixtures | Need one real short-timeout smoke to prove no downstream validator timeout is created. |
| P2 fast-fail invalid full runs | Hard engineering-unclean stops later work and suppresses score language. | `run-taskspace-e3-suite.ps1`, `suite-status.ps1`, `test-e3-harness-guardrails.ps1`, `test-e3-score-validity.ps1`. | implemented for fixtures | Need representative official invalid case artifact for release evidence. |
| P3 Docker/validator overhead | Cache requires immutable proof; pretest/test timeout split; finalize does not rerun validators. | `terminal-bench-adapter.ps1`, `test-terminal-bench-adapter-harness.ps1`, `test-terminal-bench-docker-cache-smoke.ps1`, `test-oracle-runner-harness.ps1`. | mostly implemented | Cache edge cases remain thin: ARG-based FROM, lowercase/multistage, package metadata drift, and no-FROM cases need fixtures. |
| P4 governed parallelism | Sample-level parallelism is opt-in, isolated, deterministic, resource-governed, and serial-vs-parallel compared. | `resource-governor.ps1`, `run-taskspace-e3-suite.ps1`, `parallel-diff.ps1`, `test-e3-harness-guardrails.ps1`, `calibration-gate.ps1`. | partially implemented | Only sample-level parallelism is implemented. Comparator does not yet prove every field in the full Section 16 list because some profile/config/artifact hashes are not always present in `suite-health.json`. |
| P4 observed concurrency | `parallelism.json` records configured and observed concurrency. | `resource-governor.ps1`, `test-e3-harness-guardrails.ps1`. | implemented for sample-level parallel fixture | Future Docker/model token managers must replace placeholder observed values when those modes are implemented. |
| P5 speed decision | Runtime calibration report gives defensible answer to why 15-task E3 takes hours and whether speedup is possible. | `runtime-bottleneck-report.ps1`, `runtime-calibration-report.md/json` fixture checks, calibration gate. | partial | No current official 3-task serial calibration plus parallel smoke artifact has been run under this final gate. Speed claims remain blocked. |

## Current No-Go Conditions

- Do not mark the full implementation plan complete.
- Do not run or report full E3 score unless `calibration-gate.json status=pass`.
- Do not claim large speedup without serial baseline, comparable parallel smoke, and timing reconciliation.
- Do not enable pair-level, validation-level, Docker, or model concurrency; only sample-level parallelism is implemented.

## Next Engineering Actions

1. Add missing Docker cache proof edge-case fixtures.
2. Add artifact identity/provenance checks to `calibration-gate.ps1`: task-list hash, source version, command/profile hash, serial mode proof, score validity.
3. Extend `parallel-diff.ps1` or suite health production so profile/config/prompt/proof artifact hashes required by Section 16 are actually compared.
4. Run one official one-pair timing smoke and one representative 3-task serial calibration, then pass them through the calibration gate.
5. Only after the above, run a governed sample-parallel smoke and compare against serial.

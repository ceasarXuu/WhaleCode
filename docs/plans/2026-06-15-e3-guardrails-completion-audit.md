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
| Calibration identity | Calibration artifacts should match expected task-list/source/profile identity when those expectations are supplied. | `e3-identity.ps1`, `run-taskspace-e3-suite.ps1`, `run-taskspace-benchmark.ps1`, `timing.ps1`, `parallel-diff.ps1`, `e3-start-gate.ps1`, `test-e3-start-gate.ps1`, `test-e3-harness-guardrails.ps1`. | implemented for canonical suite/start-gate path | Direct helper/manual evidence paths must still pass expected identity explicitly; do not treat manually assembled calibration artifacts as score-bearing unless `gate-decision.json` and calibration identity checks pass. |
| P0 timing evidence | Pair, sample, and suite timing artifacts exist; missing timing blocks speed conclusions. | `timing.ps1`, `runtime-bottleneck-report.ps1`, `test-e3-harness-guardrails.ps1`, `test-e3-score-validity.ps1`. | implemented for synthetic fixtures | Need official one-pair calibration artifact from a real Terminal-Bench sample before full E3/speed claims. |
| P0 wait attribution | Model/API/resource wait fields are present or explicitly unavailable; unknown attribution blocks speed claims. | `test-e3-score-validity.ps1` asserts blocked status when wait attribution is incomplete. | implemented for fixtures | Need real-run evidence showing unavailable fields are rendered and not treated as pass. |
| P1 agent-timeout skip | Agent execution timeout can skip public validation only with passed pre-agent proof; otherwise engineering-unclean. | `run-taskspace-benchmark.ps1`, `e3-proof.ps1`, `failure-taxonomy.ps1`, `test-e3-score-validity.ps1`, `test-e3-proof-harness.ps1`. | implemented for fixtures | Need one real short-timeout smoke to prove no downstream validator timeout is created. |
| P2 fast-fail invalid full runs | Hard engineering-unclean stops later work and suppresses score language. | `run-taskspace-e3-suite.ps1`, `suite-status.ps1`, `test-e3-harness-guardrails.ps1`, `test-e3-score-validity.ps1`; first-child invalid fixture in `test-e3-harness-guardrails.ps1` proves a child `invalid_harness` exit `3` skips the second sample and emits one `suite_score_invalidated` event. | implemented for fixtures | Need representative official invalid case artifact for release evidence. |
| P3 Docker/validator overhead | Cache requires immutable proof; pretest/test timeout split; finalize does not rerun validators. | `terminal-bench-adapter.ps1`, `test-terminal-bench-adapter-harness.ps1`, `test-terminal-bench-docker-cache-smoke.ps1`, `test-oracle-runner-harness.ps1`; adapter harness asserts generated validator lifecycle markers, probe-before-tests structure, validator/source/Dockerfile/source-version cache-key drift, lowercase multistage digest-pinned cache eligibility, ARG-based FROM cache disablement, and no-FROM cache disablement; oracle runner harness proves pretest/test timeout phase preservation. | implemented for fixtures | Need representative real Docker cache smoke evidence for the final release gate; fixture coverage is now sufficient for the planned edge cases. |
| P4 governed parallelism | Sample-level parallelism is opt-in, isolated, deterministic, resource-governed, and serial-vs-parallel compared. | `resource-governor.ps1`, `run-taskspace-e3-suite.ps1`, `parallel-diff.ps1`, `test-e3-harness-guardrails.ps1`, `calibration-gate.ps1`. | partially implemented | Only sample-level parallelism is implemented. Comparator does not yet prove every field in the full Section 16 list because some profile/config/artifact hashes are not always present in `suite-health.json`. |
| P4 observed concurrency | `parallelism.json` records configured and observed concurrency. | `resource-governor.ps1`, `test-e3-harness-guardrails.ps1`. | implemented for sample-level parallel fixture | Future Docker/model token managers must replace placeholder observed values when those modes are implemented. |
| P5 speed decision | Runtime calibration report gives defensible answer to why 15-task E3 takes hours and whether speedup is possible. | `runtime-bottleneck-report.ps1`, `runtime-calibration-report.md/json` fixture checks, calibration gate. | partial | No current official 3-task serial calibration plus parallel smoke artifact has been run under this final gate. Speed claims remain blocked. |
| Runtime bottleneck repair plan | The plan must explain why a 15-task E3 can take hours, how to identify the dominant bottleneck, and which speedups are allowed without changing the v0.0.4 scoring profile. | Section 16.13 of the implementation plan; `e3-start-gate.ps1` writes `gate-decision.json`; `calibration-selection.ps1` writes deterministic `calibration-selection.json`; `runtime-reconstruction.ps1` writes `runtime-reconstruction.json/md`; R0 reconstruction was run on `target/suite-disk-preflight-smoke-3/suite-20260613-164323`; early-abort closure smoke was run on `target/e3-disk-early-abort-timing-smoke-2/suite-20260615-032809`. | partially implemented | Early start-gate/disk-reservation aborts now write minimal suite timing and reconstruct with `first_invalid_sample_index=0`, `sample_rows=1`, no missing fields, and `invalid_waste_bound`. Remaining R0-R5 work: legacy importer or located canonical root for the old full E3 artifact, first-pair invalid fast-fail proof, validator/Docker overhead proof, governed parallel smoke release evidence, official one-pair/3-task calibration artifacts, and final full E3 release gate. |
| Detailed speedup execution plan | The plan must turn the multi-hour runtime concern into concrete engineering tasks, artifact contracts, and phase gates before any full E3 rerun. | Section 16.14 of the implementation plan defines timing budget buckets, early-abort timing closure, invalid-run waste elimination, validator/Docker cost reduction, governed sample-level parallelism, speed-claim decision rules, concrete implementation order, and adversarial review checklist. Actions 1-5 are now fixture-covered: early-abort timing/reconstruction, first-child invalid fast-fail, validator lifecycle split, and Docker cache proof edge cases. | partially implemented | Continue Section 16.14 from action 6: complete parallel comparator fields. No large speedup claim is allowed until accepted serial and parallel calibration artifacts exist. |

## Current No-Go Conditions

- Do not mark the full implementation plan complete.
- Do not run or report full E3 score unless `gate-decision.json` says `next_allowed_command_category=full_e3`, `full_e3_allowed=true`, `calibration-gate.json status=pass`, and task-list/source/profile identity checks match.
- Do not claim large speedup without serial baseline, comparable parallel smoke, and timing reconciliation.
- Do not enable pair-level, validation-level, Docker, or model concurrency; only sample-level parallelism is implemented.

## Next Engineering Actions

Execute these in order. Actions 1-5 are now covered by fixtures for start-gate/disk early aborts, first-child invalid suite fast-fail, validator lifecycle timeout split, and Docker cache proof edge cases; continue from action 6 unless a new early-abort, invalid fast-fail, validator lifecycle, or Docker cache proof path is added.

1. Maintain early-abort timing artifact closure for start-gate and disk-reservation exits: `suite-health.json`, `suite-timing.json`, runtime bottleneck report, runtime calibration report, and `gate-decision.json` must exist on exit `3`.
2. Maintain runtime reconstruction sample-row extraction so early-abort suites produce `sample_rows`, `first_invalid_sample_index=0`, and no false missing `suite-timing.json`.
3. Maintain score-invalid fast-fail coverage for first-child engineering-unclean failures; current fixture proves child `invalid_harness` exit `3` skips later samples.
4. Maintain validator lifecycle split and timeout classification; current fixtures prove generated lifecycle markers, probe-before-tests structure, pretest timeout phase, tests timeout phase, and engineering-unclean taxonomy.
5. Maintain Docker cache proof edge-case fixtures: validator/source/Dockerfile/source-version cache-key drift, lowercase multistage digest-pinned eligibility, ARG-based FROM disablement, and no-FROM disablement.
6. Extend `parallel-diff.ps1` or suite health production so profile/config/prompt/proof artifact hashes required by Section 16 are always present and compared.
7. Add a legacy reconstruction/import path for older run roots such as `target/e3-full-20260606-014919`, or locate the canonical suite root from the invalid full E3 run.
8. Run one official one-pair timing smoke and one representative 3-task serial calibration only when the provenance-bearing `gate-decision.json` says those command categories are allowed and all referenced artifact hashes verify.
9. Use `calibration-selection.json` as the required representative 3-task selection artifact for that calibration.
10. Only after the above, run full 15-task E3 when `next_allowed_command_category=full_e3`, `full_e3_allowed=true`, `calibration_gate.status=pass`, serial/parallel artifact hashes verify, `review_gate.status=pass`, and identity checks match.

Backlog after actions 1-2 are closed:

- Docker cache edge fixtures beyond the current regression cases.
- Comparator coverage for optional prompt/config/proof hashes; absence of required hashes must block parallel acceptance rather than silently pass.
- Legacy importer for old non-canonical run roots.

# TaskSpace E3 Gate Review

Date: 2026-06-02

## Review Target

TaskSpace benchmark E3 real-world utility planning and non-breaking harness gate extension.

Target files:

- `docs/testing/2026-06-02-taskspace-e3-real-world-utility-plan.md`
- `docs/testing/templates/taskspace-e3-human-review.md`
- `benchmarks/taskspace/corpora/historical-failures/README.md`
- `scripts/taskspace-benchmark/lib/scenario-manifest.ps1`
- `scripts/taskspace-benchmark/lib/pair-report.ps1`
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- `scripts/taskspace-benchmark/test-harness.ps1`

## Review Input Packet

Objective: Move TaskSpace evidence work from E2 paired constructed scenarios toward E3 real-world utility evidence without overclaiming.

Change summary:

- Added an E3 planning document defining real-world/historical sample requirements, Terminal-Bench/SWE-bench style adapters, manual review, and E3 downgrade rules.
- Added a human review template.
- Added a historical failure corpus README.
- Extended scenario manifest parsing with optional E3 fields.
- Extended evidence gate with E3-specific failures and `E3` / `E3-candidate` levels.
- Extended pair/run/aggregate reports with E3 aggregate fields.
- Added harness self-tests for E3 downgrade/promotion behavior.

Risk focus:

- E2 reports must not be downgraded or polluted by missing E3 fields.
- E3 must not be reported unless sample origin is real/external, repeats meet the configured minimum, and human review is completed.
- `E3-candidate` must not enter utility or E3 aggregate.
- Existing positional PowerShell calls to `Get-TaskspaceEvidenceGate` must remain compatible.
- Documentation must not claim current E2 results are E3.
- No hidden prompt rewrites or TaskSpace-friendly prompt assumptions should be introduced.

Validation status:

- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1` passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 -Scenario single-file-fast-fix -Repeats 1 -PlanOnly` passed.

Reviewer instructions:

- Fresh read-only review.
- Inspect target files directly.
- Do not modify files.
- Report blocking findings, non-blocking risks, missing tests, and evidence paths.

## Reviewer Launch Records

### Round 1 / Reviewer A

- role: Benchmark evidence gate reviewer
- mechanism: internal subagent via `multi_agent_v1.spawn_agent`
- session/agent id: `019e84b7-cbe2-76a0-9c3b-0840ba8d497a`
- context inheritance: false
- read-only: true
- status: completed

### Round 2 / Reviewer B

- role: E3 closure reviewer
- mechanism: internal subagent via `multi_agent_v1.spawn_agent`
- session/agent id: `019e84bd-1c58-74b2-85d2-ade022135509`
- context inheritance: false
- read-only: true
- status: completed

### Round 3 / Reviewer C

- role: final E3 closure reviewer
- mechanism: internal subagent via `multi_agent_v1.spawn_agent`
- session/agent id: `019e84c2-64a3-7a03-993f-63065bc764e1`
- context inheritance: false
- read-only: true
- status: completed

## Reviewer Output

### Reviewer A Result

Summary:

The current E3 changes are directionally conservative for the real runner because E3 completion is hardcoded false, but the gate API and tests still allow a weaker "E3 ready" contract than the planning doc requires.

Blocking findings:

1. E3 can be promoted without the documented evidence chain.
   - The plan requires original prompt/instruction checksums, sanitization/privacy handling, human review records, and claim scope controls, but the first implementation only checked `sample_origin.type`, human review booleans, and repeats.
   - Evidence: `docs/testing/2026-06-02-taskspace-e3-real-world-utility-plan.md`, `scripts/taskspace-benchmark/lib/pair-report.ps1`, `scripts/taskspace-benchmark/test-harness.ps1`.

2. The actual runner cannot currently produce E3, despite docs/review text implying E3 promotion behavior exists.
   - `run-taskspace-benchmark.ps1` always passed `$false` for `HumanReviewCompleted`, and pair report printed `human_review_completed: False`.
   - Evidence: `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`, `scripts/taskspace-benchmark/lib/pair-report.ps1`.

3. Docs say aggregate E3 human-review statistics are required, but implementation only counts E3 pairs.
   - The plan requires manual review pass rate and decision distribution; aggregate output only reported `valid_e3_pairs`.
   - Evidence: `docs/testing/2026-06-02-taskspace-e3-real-world-utility-plan.md`, `scripts/taskspace-benchmark/lib/pair-report.ps1`.

Non-blocking risks:

- E2 reports can be noisy if blank E3 fields are emitted.
- `aggregate-report.ps1` is stale relative to the active writer in `pair-report.ps1`.
- Positional compatibility appears preserved.

Missing tests:

- E3 promotion must fail when origin type is valid but prompt checksum, privacy review, sanitized flag, claim scope, or external validator metadata is missing.
- Runner-path E3 fixture should later prove how `HumanReviewCompleted` becomes true from an actual review artifact.
- Aggregate tests should assert E2 reports contain no E3 noise.
- Aggregate tests should verify manual review decision distribution once review ingestion exists.

## Main Agent Response

1. accept
   - Tightened `Get-TaskspaceEvidenceGate` so E3 now also requires source, `original_prompt_sha256`, claim scope, historical sanitization, historical privacy review completion, or external sample/validator/adapter metadata depending on source type.
   - Added self-tests for missing prompt checksum, missing sanitization, missing privacy review, missing claim scope, and complete external benchmark metadata.

2. accept
   - Kept real runner behavior conservative: `run-taskspace-benchmark.ps1` still passes `HumanReviewCompleted = false`, so real runs can only become `E3-candidate` until review artifact ingestion is implemented.
   - Updated the E3 plan to explicitly state this limitation.
   - Pair report now prints `human_review_completed` from the evidence object instead of hardcoding report text.

3. accept with scope clarification
   - Current implementation now prevents E3 overclaim and records E3 aggregate counts only when E3 rows exist.
   - Full manual-review decision distribution is intentionally not implemented yet; the plan now treats it as the next implementation step before true E3 can be reported.

Non-blocking responses:

- accept: E2 summary/aggregate no longer emits E3 fields unless the row is E3/E3-candidate; self-tests assert this.
- accept: updated the plan to call out the stale aggregate writer and state that current active writer is in `pair-report.ps1`; cleanup remains a future refactor.
- accept: existing positional calls remain in tests, and new params were appended.

Validation:

- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1` passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 -Scenario single-file-fast-fix -Repeats 1 -PlanOnly` passed.

### Round 2 Closure Review Result

Closure status: partially closed.

Remaining blocking findings:

1. Round 1 finding 3 was not fully closed because docs required E3 human-review pass rate and disagreement, but implementation only emitted `valid_e3_pairs`.
2. E3 repeat protection was too configurable; a manifest could set `minimum_repeats` below 5.
3. Historical sanitization handling was thinner than the plan; gate checked booleans but not sanitization explanation/risk metadata.

Additional non-blocking risks:

- E3 runner exit semantics remain candidate-oriented until review ingestion exists.
- Pair-report-level E2 no-noise was less directly tested than summary/aggregate.
- Stale `aggregate-report.ps1` cleanup remains future work.

### Main Agent Response To Round 2

1. accept
   - Added E3 review decision and disagreement fields to the evidence object.
   - Aggregate now reports `e3_human_review_completed_pairs`, `e3_human_review_disagreement_pairs`, and `e3_human_review_decisions`.
   - E3 promotion requires a valid include decision when human review is completed.

2. accept
   - Clamped effective E3 minimum repeats to `Math.Max(5, configuredMinimum)`.
   - Added a negative self-test proving a manifest-level minimum below 5 still records `e3_repeats_lt_5`.

3. accept
   - Historical samples now require `sanitization_summary` and `privacy_risk_summary` in addition to `sanitized`, `privacy_review_completed`, and `original_prompt_sha256`.
   - Added negative self-tests for missing sanitization and privacy risk summaries.
   - Updated the E3 plan and historical corpus README.

Validation:

- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1` passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 -Scenario single-file-fast-fix -Repeats 1 -PlanOnly` passed.

## Closure Status

Round 3 closure review completed.

Final closure result:

- No remaining blocking findings.
- E3 promotion now requires the full evidence chain and cannot be reached by the current runner until human review artifact ingestion exists.
- Effective E3 repeats are clamped to at least 5.
- Historical sample sanitization and privacy metadata are enforced.
- E3 aggregate reports human review completion count, disagreement count, and decision distribution.
- E2 summary and aggregate paths avoid E3 noise.

Final validation:

- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1` passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 -Scenario single-file-fast-fix -Repeats 1 -PlanOnly` passed.

Remaining non-blocking follow-ups:

- Add runner-path E3 fixture after human review artifact ingestion exists.
- Add direct pair-report-level E2 no-noise assertion.
- Add per-field negative tests for external benchmark metadata.
- Clean up stale `lib/aggregate-report.ps1` once the active aggregate writer is consolidated.

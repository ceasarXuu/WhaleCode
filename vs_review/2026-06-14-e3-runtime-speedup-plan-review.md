# E3 Runtime Speedup Plan Adversarial Review

## Round 1 Review Input

Objective: review the new runtime bottleneck and speedup planning section added to the E3 guardrails implementation plan.

Review target: architecture and execution plan documentation.

Target file:

- `D:\whalecode-alpha\docs\plans\2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md`

Changed area:

- Metadata version/status updated to `0.5`.
- New section `## 16. Integrated Runtime Bottleneck And Speedup Plan`.
- The new section adds phases P0-P5 covering timing evidence, validation skip after agent timeout, fast-fail invalid runs, Docker/validator overhead reduction, resource-governed parallelism, and speed target decisions.

Risk focus:

- The plan might accidentally allow speed optimizations to contaminate E3 scoring validity.
- Skipping validation after agent timeout might hide validator infrastructure failures.
- Parallelism might make Standard vs TaskSpace comparisons non-equivalent.
- Timing artifacts might be insufficient to prove bottlenecks or speed claims.
- The plan might be too abstract to guide implementation.

Reviewer instructions:

- Use a fresh read-only review.
- Inspect the target file directly.
- Do not modify files.
- Focus on blocking or major risks.
- Cite file paths and line numbers where possible.
- Return blocking findings, non-blocking risks, missing tests/logs, and user-comprehension issues.

## Round 1 Reviewer Launch Record

| Field | Value |
|---|---|
| reviewer role | architecture/runtime plan adversarial reviewer |
| mechanism | internal subagent via `multi_agent_v1.spawn_agent` |
| forked main context | no |
| read-only | yes |
| input packet | Round 1 Review Input above |
| context excluded | main-agent reasoning, prior conversation history, persuasion brief, full diff dump |
| session id | `019ec6bc-179e-71c0-8cb3-3429569b02fe` |
| status | completed |

## Round 1 Reviewer Output

### Summary

Read-only adversarial review completed. The section is directionally good, but it should not be treated as implementation-ready until several validity risks are closed.

### Blocking Findings

- Section 16 is not clearly marked as superseding the multiple runtime plans in section 15. The document has overlapping R, S, and P phase sequences, so different implementers could follow different mandatory paths.
- P1 can contaminate validity by skipping public validation after agent timeout while only treating pre-agent probe/proof as a mitigation. A timed-out side should be clean only if a recorded pre-agent validator/proof probe passed.
- P4 compares score validity, inclusion, and proof metadata, but not actual per-task outcomes, pass/fail counts, pair outputs, model/profile hashes, prompts/config, or score values. A parallel run could change actual benchmark results while still passing `score_valid=true`.
- Docker cache key is under-specified for scoring safety. It does not require resolved base image digests, full build context, remote package inputs, platform/arch, or Docker-engine-sensitive inputs.
- P0 asks whether model/API queueing or resource waits are bottlenecks, but the timing tasks do not require model queue, retry/backoff, process launch wait, resource wait, or API throttling spans.

### Non-Blocking Risks

- Metadata said `Repair-ready` while the new section checklist was entirely unchecked, which read more mature than the actual state.

### Missing Tests / Logs

- Add a scoring test where `agent_exec_timeout=true` plus failed/missing pre-agent validator probe becomes `engineering_unclean`.
- Add a test proving non-timeout agent failures still run validation.
- Add serial-vs-parallel comparison tests for per-task outcome, score values, profile hash, model settings, prompt/config hash, artifact hashes, and pair ordering.
- Add Docker cache mutation fixtures for mutable base tags, changed build context, remote package drift, platform/arch drift, and uv cache drift.
- Add timing fixtures for model/API queue wait, retry backoff, resource wait, child-process launch latency, and unreconciled overlap.

### User-Comprehension Issues

- The document did not say whether section 16 replaces section 15 or merely adds another plan.
- `Repair-ready` was ambiguous: ready to implement, not ready to run or claim speedup.

## Round 1 Main-Agent Responses

- accept: Section 16 canonicality finding. Updated section 16 introduction to state it supersedes conflicting section 15 runtime notes and is the canonical post-0.5 runtime implementation sequence.
- accept: P1 pre-agent proof finding. Promoted passed pre-agent validator/proof probe to an entry criterion, entry check, design rule, metrics contract, validation fixture, and exit criterion. Missing/failed/stale proof now forces `engineering_unclean`.
- accept: P4 drift finding. Expanded serial-vs-parallel comparison to include score values, per-task and per-pair outcomes, pass/fail/timeout counts, better/worse/tie counts, profile/model/prompt/config/scenario hashes, artifact hashes, and ordering.
- accept: Docker cache immutability finding. Expanded cache key to include resolved base image digest, full build context hash, platform/arch, Docker engine/backend mode, package metadata, network mode, environment variables, and drift fixtures.
- accept: P0 wait timing finding. Added model queue wait, retry backoff, request duration, process launch wait, Docker/validation/disk/cache waits, total resource wait, unknown bottleneck guard, and blocked speed-claim acceptance.
- accept: Metadata maturity finding. Changed status to `Implementation-ready plan draft ...; not approved for speed claims`.

## Closure Status

Round 1 blocking findings accepted and patched. Closure review completed with no remaining blocking findings.

## Round 2 Closure Review Input

Objective: verify that the accepted Round 1 blocking findings are closed.

Target file:

- `D:\whalecode-alpha\docs\plans\2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md`

Closure claims:

1. Section 16 is canonical and supersedes conflicting section 15 runtime notes.
2. Validation skip after agent timeout is score-clean only with passed pre-agent validator/proof probe.
3. Serial-vs-parallel comparison includes actual outcomes, scores, profile/config identity, and artifact hashes.
4. Docker cache contract includes full immutable build context and resolved inputs.
5. Timing includes model/API/resource waits and unknown bottleneck blocks speed claims.
6. Status does not imply speed claims are approved.

Reviewer instructions:

- Use a fresh read-only review.
- Inspect the target file directly.
- Do not modify files.
- Return closed/not closed for each item with evidence paths and line numbers.

## Round 2 Reviewer Launch Record

| Field | Value |
|---|---|
| reviewer role | closure reviewer |
| mechanism | internal subagent via `multi_agent_v1.spawn_agent` |
| session id | `019ec6c0-92e2-7620-9edb-b45249319ba6` |
| forked main context | no |
| read-only | yes |
| input packet | Round 2 Closure Review Input above |
| context excluded | main-agent reasoning, prior conversation history, persuasion brief, full diff dump |
| status | completed |

## Round 2 Reviewer Output

Closure review result: all six claimed blockers are closed. No remaining blocking findings were reported for section 16 or metadata.

Closed items:

- closed: Section 16 is canonical and supersedes conflicting section 15 runtime notes.
- closed: Validation skip after agent timeout is score-clean only with passed pre-agent validator/proof probe.
- closed: Serial-vs-parallel comparison includes actual outcomes, scores, profile/config identity, and artifact hashes.
- closed: Docker cache contract includes full immutable build context and resolved inputs.
- closed: Timing includes model/API/resource waits and unknown bottleneck blocks speed claims.
- closed: Status does not imply speed claims are approved.

## Round 2 Main-Agent Responses

- accept: closure reviewer found all six accepted blocker fixes closed.
- accept: no additional blocking findings were reported.

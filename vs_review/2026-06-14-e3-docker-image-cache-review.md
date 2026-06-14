# Subagent VS Review: E3 Docker Image Cache

- Created: 2026-06-14T19:49:58+08:00
- Updated: 2026-06-14T20:05:00+08:00
- Task: Execute the TaskSpace E3 Harness Guardrails Implementation Plan.
- Report path: `vs_review/2026-06-14-e3-docker-image-cache-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: passed

## Round 1: Docker cache implementation review

### Review Input

#### Objective
Review the new flagged Terminal-Bench Docker image cache implementation for correctness, score validity, cache invalidation, and test adequacy.

#### Review Target
Code implementation, tests, and documentation for `-EnableDockerImageCache` and generated validator cache behavior.

#### Target Locations
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1`
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- `scripts/taskspace-benchmark/run-taskspace-external-benchmark.ps1`
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1`
- `scripts/taskspace-benchmark/lib/metrics-extractor.ps1`
- `scripts/taskspace-benchmark/test-terminal-bench-adapter-harness.ps1`
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md`

#### Change Introduction
The implementation adds a flagged Docker image cache path for generated Terminal-Bench validators. The runner exposes `-EnableDockerImageCache`, sets `TASKSPACE_DOCKER_IMAGE_CACHE=1` only during score-bearing public validation, and records the flag in `manifest.resolved`. The generated validator computes a content-derived cache key from source version, fixture tree hash, and Dockerfile hash; when the env flag is enabled it inspects `whale-taskspace-terminal-bench-cache:<key>` before build, records `cache_hit`, and builds the stable cache image only on miss. Metrics extraction treats `cache_hit` as a non-failure classification.

#### Risk Focus
- Cache key stale-hit risk: source mutation, Dockerfile mutation, fixture tree mutation, adapter/runtime behavior changes, floating base images.
- Score validity: cache hits must not hide validator/source mutations or convert Docker failures into clean agent outcomes.
- Cleanup and artifact consistency: cached images should not be deleted by per-run cleanup; containers still need bounded cleanup.
- Test adequacy: current tests assert generated script cache logic and key invalidation, but do not run a full real-Docker two-run cache hit smoke.
- Windows/PowerShell risks: generated validator syntax, env scoping, and command discovery.

#### Verification Status
- `.\scripts\taskspace-benchmark\test-terminal-bench-adapter-harness.ps1` passed.
- `.\scripts\taskspace-benchmark\test-terminal-bench-uv-cache-harness.ps1` passed.
- `.\scripts\taskspace-benchmark\test-e3-score-validity.ps1` passed.
- `git diff --check` passed.
- Known unverified area: real-Docker two-run cache hit smoke is intentionally not completed in this change.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Return findings in this structure: summary, blocking findings, non-blocking risks, required fixes, missing tests, missing logs or observability, evidence.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation/test/ops adversary | Cache touches generated runtime code, score validity, tests, and operational rollout. | Cache correctness, false clean scoring, test gaps |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation/test/ops adversary | multi_agent_v1.spawn_agent | 019ec5f8-5a77-7d81-84dc-c026afb4ac00 / Raman | spawn_agent result in current Codex thread | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### implementation/test/ops adversary / Raman

##### Summary
Read-only review completed. The cache is flag-gated, disabled by default, scoped around public validation, records `cache_hit`, and does not delete stable cache images during bounded cleanup. One blocking score-validity issue remains before production E3 scoring can trust the cache.

##### Blocking Findings
- Floating Docker base images can produce stale cache hits and change score-bearing semantics. The cache key used source version, fixture tree hash, and Dockerfile hash, but not resolved base image digest. On miss the validator uses `docker build --pull`; on hit it only inspects the existing cached image, so `FROM ubuntu:latest` or similar could silently reuse an old base where no-cache would pull a newer base.

##### Non-blocking Risks
- Cache enablement is not tied to `-ScoringMode` or `-RequireScoreValidity`; any caller passing the flag gets the cache behavior during public validation.
- No cache-key lock; serial suite execution makes this non-live now, but planned parallel Docker execution will need a lock.
- Cache metadata is split across runtime manifest and adapter metadata.

##### Required Fixes
- Reject caching unless all Dockerfile `FROM` references are digest-pinned, or record and compare resolved base image digests before cache hit.
- Add base-image proof to `docker-build-result.json` and runtime manifest.

##### Missing Tests
- Real-Docker two-run cache smoke.
- Floating-base invalidation/bypass test.
- Cache-hit score-validity fixture where docker run failure remains engineering-unclean.

##### Missing Logs / Observability
- Cache hit logs need proof of why the hit is valid, not just key/image presence.
- No base image digest, parent image id, or RepoDigest proof.
- No cache-build provenance timestamp or builder/runtime Docker version.

##### Evidence
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1`
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- `scripts/taskspace-benchmark/lib/metrics-extractor.ps1`
- `scripts/taskspace-benchmark/lib/oracle-runner.ps1`
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md`

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Raman | Floating Docker base images can produce stale cache hits | blocking | accept | Cache hit would not preserve `docker build --pull` semantics for floating tags | Added Dockerfile `FROM` parser; cache is eligible only when every base image is digest-pinned with `@sha256:<64 hex>`. Otherwise `cache_enabled=false`, `cache_eligible=false`, and `cache_bypass_reason=dockerfile_base_image_not_digest_pinned` are recorded. | Closure review required |
| Raman | Cache enablement not tied to scoring mode | non-blocking | accept | Flag is explicit and scoped to public validation; docs must not imply default production use | Plan states real-Docker two-run smoke is still required before enabling cache in full production E3. | n/a |
| Raman | No cache lock for future parallelism | non-blocking | defer | Current runner is serial and Phase R3 has not landed | Track with resource-governed parallelism. | Phase R3 |
| Raman | Metadata split across artifacts | non-blocking | defer | Current artifacts include enough key/proof fields; richer provenance can be added with real-Docker smoke | Track as observability hardening. | Future cache provenance |

Validation evidence after fixes:
- `.\scripts\taskspace-benchmark\test-terminal-bench-adapter-harness.ps1` passed at 2026-06-14T19:58+08:00.

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: pending closure review
- Blocking re-review completed: no
- Blocking re-review passed: pending
- Blocking re-review round links:
  - Round 2 pending
- Blocking re-review launch records:
  - pending
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Allowed to proceed: no, closure review required

## Round 1 Interim Conclusion

Pending closure review.

## Round 2: Floating base cache closure review

### Review Input

#### Objective
Verify whether the accepted Round 1 blocking finding is closed.

#### Review Target
Closure of floating Docker base image stale-cache risk.

#### Target Locations
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1`
- `scripts/taskspace-benchmark/test-terminal-bench-adapter-harness.ps1`
- `scripts/taskspace-benchmark/lib/metrics-extractor.ps1`
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md`

#### Change Introduction
The main agent added Dockerfile `FROM` parsing and made Docker image cache eligible only when every base image reference is digest-pinned as `@sha256:<64 hex>`. Generated validators now combine env opt-in with `cacheEligible`, and runtime artifacts record `cache_eligible` plus `cache_bypass_reason`.

#### Risk Focus
- `FROM` parsing misses multi-stage, lowercase, comments, ARG-based sources, or no-FROM cases.
- Cache still enabled for floating tags.
- Tests only check generated text weakly and do not prove metadata.
- New syntax or line-count issue.

#### Verification Status
- `.\scripts\taskspace-benchmark\test-terminal-bench-adapter-harness.ps1` passed after the fix.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Return findings in this structure: summary, blocking findings, non-blocking risks, required fixes, missing tests, missing logs or observability, evidence, closure verdict.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation/test closure adversary | Closure is a focused score-validity fix. | False closure, parsing edge cases, test strength |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation/test closure adversary | multi_agent_v1.spawn_agent | 019ec600-c86f-7711-8fc3-4d83a215ca4f / Harvey | spawn_agent result in current Codex thread | no | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### implementation/test closure adversary / Harvey

##### Summary
The accepted Round 1 blocking finding is closed. Floating or uncertain Docker bases do not reach the cache-hit path: the adapter fails closed unless every parsed `FROM` image ends in `@sha256:<64 hex>`, and the generated validator enables cache only when both `TASKSPACE_DOCKER_IMAGE_CACHE=1` and `cacheEligible` are true.

##### Blocking Findings
- None for the accepted stale Docker base cache finding.

##### Non-blocking Risks
- Parser is conservative, not complete; safe forms such as `FROM --platform=... alpine@sha256:...` can bypass cache.
- Tests cover `FROM scratch` bypass and digest-pinned eligibility, but not every parser edge case such as lowercase `from`, multi-stage, `ARG BASE`, or no `FROM`.
- Docs needed to explicitly mention digest-pinned-only eligibility.
- `terminal-bench-adapter.ps1` remains a pre-existing over-500-line generated-script file and grew further; future work should split cache helper logic out instead of continuing to expand it.

##### Required Fixes
- No required fix to close the blocking finding.

##### Missing Tests
- `FROM ubuntu:latest` bypass.
- lowercase `from`.
- multi-stage with all external bases digest-pinned.
- `ARG BASE=ubuntu:latest` / `FROM $BASE` bypass.
- no `FROM` bypass.

##### Missing Logs / Observability
- Runtime artifacts record `cache_eligible` and `cache_bypass_reason`; metrics should also lift these for report-level audit.

##### Evidence
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1:105`
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1:371`
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1:447`
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1:468`
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1:599`
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1:367`
- `scripts/taskspace-benchmark/test-terminal-bench-adapter-harness.ps1:195`

##### Closure Verdict
Pass. The Round 1 accepted blocking finding can be marked closed. Remaining items are non-blocking hardening and project-constraint cleanup, not stale-cache correctness blockers.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Harvey | No remaining stale floating-base cache blocker | blocking | accept | Closure reviewer found fail-closed eligibility and generated validator gate close the accepted blocker | Proceed after final validation | n/a |
| Harvey | Docs needed digest-pinned-only wording | non-blocking | accept | Plan previously emphasized fixture/Dockerfile hash more than eligibility | Updated Section 15.5 current implementation notes to state digest-pinned-only cache eligibility and bypass behavior | n/a |
| Harvey | Metrics did not lift cache eligibility/reason/from-images | non-blocking | accept | Useful for report-level audit | Added `docker_cache_eligible`, `docker_cache_bypass_reason`, and `dockerfile_from_images` to metrics extraction | n/a |
| Harvey | Parser edge-case tests missing | non-blocking | defer | Conservative bypass is safe; extra fixtures are hardening, not closure blockers | Track for cache hardening before production enablement | Future cache test expansion |
| Harvey | Adapter file remains over 500 lines | non-blocking | defer | File was already over the normal limit as a generated-script adapter; current line count is recorded and future cache work should split helpers | Track as refactor debt; do not expand further without splitting | Future adapter split |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2
- Blocking re-review launch records:
  - `019ec600-c86f-7711-8fc3-4d83a215ca4f / Harvey`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Allowed to proceed: yes

## Final Conclusion

Passed. The Docker image cache implementation remains behind an explicit flag, fails closed for floating or parser-uncertain Docker base images, and has no unresolved blocking findings after closure review.

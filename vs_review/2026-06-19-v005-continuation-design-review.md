# Subagent VS Review: v0.0.5 continuation engineering design

- Created: 2026-06-19T11:41:22.8381117+08:00
- Updated: 2026-06-19T12:00:00+08:00
- Report schema: adversarial-v1
- Task: 对 v0.0.5 未完成项继续开发方案执行对抗性审查，防止再次把观测、报告或模糊门禁误判为工程闭环。
- Report path: `vs_review/2026-06-19-v005-continuation-design-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: passed

## Round 1: Architecture and validation attack

### Review Input

#### Objective

审查 v0.0.5 continuation engineering design 是否足够可执行、可验证、可防止错误收口，并能真正推动 TaskSpace active profile 的成本控制目标。

#### Review Target

工程设计和验证方案。

#### Target Locations

- `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- `docs/v0.0.5/17-unfinished-work-inventory.md`
- `docs/v0.0.5/16-terminal-bench_E3-P0_3_2-variant-run.md`
- `docs/v0.0.5/13-design-corrections-and-engineering-contract.md`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs`
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1`
- `scripts/taskspace-benchmark/write-release-decision.ps1`

#### Change Introduction

The v0.0.5 plan was revised after a Terminal-Bench P0 diagnostic showed TaskSpace still had worse success and much higher time/token cost than Standard. A new continuation design proposes runtime active budgets, active context replacement proof, state_commit displacement gates, spawn/node budgets, request phase attribution, and stronger release-decision gates before any further formal E3 run.

#### Risk Focus

- The design may still be too high-level to implement without inventing architecture during coding.
- Proposed budgets may not correspond to actual request accounting paths.
- Active context replacement proof may be easier to fake with artifacts than to enforce in provider-visible prompts.
- Runtime hard stops may break correctness or validation recovery.
- Release gates may again validate report presence instead of actual execution effect.
- The plan may not specify enough non-agent tests to prove the gates before another real E3.

#### User-Perspective Review Focus

- Whether a future engineer can read the plan and know exactly what to build first.
- Whether the plan makes it obvious when v0.0.5 is still blocked.
- Whether the wording prevents future confusion between internal fixture success, diagnostic variants, and formal E3.
- Whether failure recovery is understandable and auditable.

#### Assumptions To Attack

- Runtime can count model requests accurately enough from `action_map` state.
- Benchmark scripts can reliably prove active context replacement.
- `state_commit` adoption can be increased through gates without increasing retries.
- Spawn/node budgets can reduce cost without invalidating TaskSpace's multi-agent purpose.
- The proposed phase gates are enough to prevent another premature E3.

#### Adversarial Lenses

- architecture
- validation
- observability
- maintenance
- failure
- release
- comprehension

#### Verification Status

- Documentation changes only so far.
- No new real E3 / Agent run was performed for this review target.
- `git diff --check` passed before the design was committed.
- Current design has not yet been implemented in Rust or benchmark scripts.

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Focus on blocking and major findings only; do not spend time on style.
- For each finding, include the broken assumption, failure scenario, trigger condition, impact, and proof needed.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 20 minutes | bounded extension up to 10 minutes if alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| architecture-adversary | The target is a high-risk engineering design that changes runtime boundaries, budget enforcement, release gates, and validation sequencing. | architecture, maintainability, executable phase gates |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| architecture-adversary | `multi_agent_v1.spawn_agent` | `019eddf8-e547-7033-926a-ca71b5b70d2c` | spawn_agent tool result nickname `Arendt` | fork_context=false | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| architecture-adversary-r1 | architecture-adversary | 1 | `019eddf8-e547-7033-926a-ca71b5b70d2c` | 20 minutes | completed | reviewer returned blocking findings | completed |

### Reviewer Outputs

#### architecture-adversary-r1

##### Summary

Blocking findings present. The continuation design points in the right direction, but it is not yet executable enough to prevent another mistaken closeout. The biggest issue is that the plan asks `action_map` runtime to enforce budgets for model requests, request phases, and provider-visible context, while the current runtime mostly sees TaskSpace tool/node/spawn state, not the model request lifecycle itself. That leaves too much room for report-only success.

##### Blocking Findings

- Runtime request budgets are not anchored to the actual model request path.
  - Broken assumption: runtime can accurately count model requests from `action_map` state.
  - Failure scenario: TaskSpace stays under node/spawn/tool budgets but still emits 30-100 internal model requests through projection, result processing, recovery, or synthesis.
  - Trigger condition: budget counters are updated only at `taskspace_control`, ordinary tool, spawn, or node lifecycle boundaries.
  - Impact: v0.0.5 can still burn tokens before the runtime knows a budget threshold was crossed.
  - Proof needed: a model-request lifecycle hook that increments `rollout_model_request_count`, phase, node/task ids, and budget state before or immediately after every provider call.
- Active context replacement proof can still be faked by artifacts.
  - Broken assumption: benchmark scripts can reliably prove provider-visible replacement.
  - Failure scenario: a run emits a `ContextProjectionV1 active replacement` block while full TaskSpace history, stale node bodies, or raw output remain elsewhere in the actual provider prompt.
  - Trigger condition: release gates trust projection counts and protected misses, not a captured provider payload or negative search over the exact request body.
  - Impact: release can pass "projection active" while input/request remains high.
  - Proof needed: per-request prompt reconstruction from the exact request payload, with hash/path, token counts, and negative assertions for legacy history/raw output.
- Phase gates are too soft to prevent premature E3.
  - Broken assumption: phase gates are enough to prevent another formal run.
  - Failure scenario: Phase 0 exits with "trace field design defined" but not implemented; later phases proceed using lossy timestamp joins or unknown attribution.
  - Trigger condition: gate allows "can do attribution OR required trace fields listed," not "trace instrumentation implemented and tested."
  - Impact: another E3 can be run with plausible artifacts but no enforceable request attribution.
  - Proof needed: fail-fast harness gate requiring request id / phase / task id / node id coverage above a defined threshold before Phase 1 or E3.
- Release gate design still blurs engineering partial with releasable success.
  - Broken assumption: stronger release gates alone prevent mistaken closeout.
  - Failure scenario: `PARTIAL` exits as code `2`, reports many gates passed, and is later described as v0.0.5 closeable despite missing the 2x release target.
  - Trigger condition: formal P0 "engineering partial" allows 3x ratios while the release target remains 2x.
  - Impact: repeats the prior closeout error in a subtler form.
  - Proof needed: decision taxonomy where only 2x is `release_pass`; 3x is explicitly `blocked_partial`, never closeable.

##### Non-blocking Risks

- Spawn budgets may cut the defining multi-agent value unless route modes define minimum useful parallelism, not only maximum fanout.
- `state_commit` displacement may increase retries if templates are too large or rejected. The plan notes this risk, but the acceptance threshold is fixture-only, not measured under realistic benchmark pressure.
- The design is still marked `Draft`, but it is being used like an implementation contract.

##### User-Perspective Checks

- Usability: risk - A future engineer knows the rough phase order, but not the first concrete code seam for model-request budget enforcement.
- Ease of use: pass - The docs make it obvious v0.0.5 is blocked through `NOT READY TO CLOSE`.
- Ease of understanding: pass - The diagnostic report explicitly says `terminal-bench_E3-P0_3_2` is not formal E3.
- Recovery path: risk - "hard stop," "final/abort," "validation recovery," and "explicit escalation" are not specified as a runtime action matrix.

##### Required Fixes

- Define the exact provider/model-request instrumentation hook before implementing runtime budgets.
- Make Phase 0 exit require implemented trace fields and fixture coverage, not just a field audit.
- Replace projection regex proof with exact request payload reconstruction and negative checks.
- Split release decisions into `release_pass`, `blocked_partial`, and `fail`; never let 3x partial imply closeout.
- Add active-budget state to runtime with explicit counters, events, and allowed recovery transitions.

##### Missing Tests

- Non-agent model-request budget fixture proving provider-call count stops within grace.
- Prompt reconstruction fixture with a deliberately fake projection artifact and legacy history still present.
- Release-decision fixture where all artifacts exist but active replacement is false.
- Hard-stop recovery tests for validation/final synthesis so budget stops do not hide correctness failures.
- State-commit retry pressure test measuring rejection and follow-up request count.

##### Missing Logs / Observability

- Per model request: request id, task id, node id, phase, route mode, budget state before/after, input/output/cached tokens.
- Budget events: threshold crossed, response action, grace count, blocked action, explicit escalation reason.
- Active replacement report tied to exact provider request payload hash.
- State-commit displacement: model-visible commit count vs runtime-synthesized commit count vs legacy fallback.
- Spawn/node budget: post-budget spawn count and unresolved subagent-result count.

##### Evidence

- `docs/v0.0.5/18-unfinished-work-engineering-design.md` - design requires request budgets and attribution but did not yet name the provider-call hook.
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1` - current request counting is post-run artifact parsing.
- `scripts/taskspace-benchmark/write-release-decision.ps1` - current release decision has `PARTIAL`.
- `docs/v0.0.5/17-unfinished-work-inventory.md` - current state is `NOT READY TO CLOSE`.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| architecture-adversary | Runtime request budgets are not anchored to actual model request path | Runtime-only node/tool counters cannot see every provider call before cost is incurred. | blocking | accept | The design named runtime counters but did not identify provider-call instrumentation as a prerequisite. | Update design to add a provider request instrumentation phase before runtime budgets; require request id, task id, node id, phase, route mode, and budget state at provider-call boundary. | Round 2 closure review |
| architecture-adversary | Active context replacement proof can be faked by artifacts | Projection artifacts do not prove exact provider-visible prompt replacement. | blocking | accept | Existing design allowed `active-context-replacement-report.json` but did not require exact payload hash/source. | Update design to require exact provider request payload capture/reconstruction, negative checks, and release blocker on missing payload proof. | Round 2 closure review |
| architecture-adversary | Phase gates are too soft to prevent premature E3 | Phase 0 could pass with field design rather than implemented trace coverage. | blocking | accept | Phase 0 gate allowed uncertainty to carry forward. | Tighten phase gates so Phase 0 cannot exit until trace fields are implemented and fixture coverage meets threshold. | Round 2 closure review |
| architecture-adversary | Release gate blurs engineering partial with releasable success | `PARTIAL` can be mistaken for closeable success. | blocking | accept | Prior closeout error came from treating partial/internal evidence too strongly. | Update decision taxonomy to `release_pass`, `blocked_partial`, `fail`; 3x target remains blocked and not closeable. | Round 2 closure review |
| architecture-adversary | Spawn budgets may cut multi-agent value | Maximum-only budgets can overcorrect and remove useful parallelism. | major | accept | The plan needs minimum useful parallelism/escalation semantics. | Add route action matrix including allowed escalation and minimum useful parallelism for broad tasks. | No blocking re-review required |
| architecture-adversary | `state_commit` displacement may increase retries | Fixture-only acceptance may not reveal rejection pressure. | major | accept | Retry pressure is a direct cost risk. | Add state_commit retry-pressure fixture and acceptance metric. | No blocking re-review required |
| architecture-adversary | Design is Draft but used like contract | Engineers may implement against unstable wording. | major | accept | Status currently `Draft`. | Keep draft status but add explicit "not approved for implementation until closure review passes" status note. | No blocking re-review required |
| architecture-adversary | Hard-stop recovery matrix missing | Future implementer may block validation/final recovery incorrectly. | major | accept | Reviewer identified unclear final/abort/recovery semantics. | Add runtime action matrix for budget states and recovery actions. | No blocking re-review required |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: in progress
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - Round 2 required after design updates
- Blocking re-review launch records:
  - pending
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Blocked reason: accepted blocking findings require design update and fresh closure review
- Allowed to proceed: no

## Round 2: Closure review for accepted blocking findings

### Review Input

#### Objective

Verify whether the accepted blocking findings from Round 1 were closed in the design document, without relying on the main agent's claims.

#### Review Target

Closure review of documentation/design changes after Round 1.

#### Target Locations

- `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- `vs_review/2026-06-19-v005-continuation-design-review.md`

#### Change Introduction

After Round 1 found four blocking issues, the design was updated to add a provider request instrumentation contract, exact provider payload replacement proof, stricter Phase 0A/0B gates, a budget-state action matrix, state_commit retry pressure, route-mode minimum useful parallelism, and a release taxonomy where `blocked_partial` is not closeable.

#### Risk Focus

- Whether provider request hook is now a hard prerequisite rather than an optional audit.
- Whether exact provider payload proof closes the fake projection artifact risk.
- Whether Phase 0 can no longer pass on field design alone.
- Whether 3x engineering partial is clearly non-closeable.
- Whether the new changes introduce new ambiguity or contradictions.

#### User-Perspective Review Focus

- Can a future engineer identify the first implementation step?
- Can a future release reviewer tell that `blocked_partial` is not closeable?
- Can a future agent understand that no real E3 should run until non-agent gates pass?

#### Assumptions To Attack

- The updated design fully closes Round 1 blocking findings.
- The new provider hook requirement is specific enough to prevent implementation drift.
- The release taxonomy prevents a repeat of the prior premature closeout.

#### Adversarial Lenses

- architecture
- validation
- release
- maintainability
- comprehension

#### Verification Status

- Documentation changes only.
- `git diff --check` passed after the changes.
- No real E3 / Agent run was performed.

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Focus only on whether Round 1 accepted blocking findings are closed or still blocked.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 15 minutes | bounded extension up to 10 minutes if alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| architecture-adversary | Same risk area as Round 1; closure must challenge whether architecture and release gates are now enforceable. | architecture, validation gates, release taxonomy |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| architecture-adversary | `multi_agent_v1.spawn_agent` | `019eddff-a938-77f3-a453-15afc734a849` | spawn_agent tool result nickname `Euler` | fork_context=false | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| architecture-adversary-r2 | architecture-adversary | 1 | `019eddff-a938-77f3-a453-15afc734a849` | 15 minutes | completed | reviewer found one remaining blocking finding | completed |

### Reviewer Outputs

#### architecture-adversary-r2

##### Summary

Round 1 blockers are mostly closed at design level, but not fully. Provider request instrumentation is now a hard prerequisite, Phase 1 is blocked behind Phase 0A/0B coverage, and `blocked_partial` is non-closeable. However, exact provider payload proof still has a blocking ambiguity: release can pass without requiring a searchable provider-visible payload artifact, while a privacy fallback allows hash plus structured size when raw capture is disabled.

##### Blocking Findings

- Exact provider payload proof is still ambiguous enough to fake active replacement.
  - Broken assumption: a payload hash plus structured size is enough to prove old TaskSpace history/raw output is absent.
  - Failure scenario: active projection emits normal artifacts, provider request has only `provider_payload_sha256`, and release checks `replacement_confirmed=true`, but the actual provider-visible prompt still contains stale TaskSpace history or raw output.
  - Trigger condition: raw payload capture is disabled for privacy and the design falls back to hash plus structured size while still allowing release gate evaluation.
  - Impact: Round 1 blocker #2 can recur: projection active passes while input/request remains inflated.
  - Proof needed: require an exact searchable provider payload artifact, or an in-process exact-payload scanner event that records matcher version, byte/token ranges checked, negative assertion results, and hash of the same scanned payload.

##### Non-blocking Risks

- Round 1 blocker #1 is closed at design level: provider request hook is now a hard prerequisite before runtime budget implementation.
- Round 1 blocker #3 is closed at design level: Phase 0B requires attribution coverage thresholds rather than field-design-only exit.
- Round 1 blocker #4 is closed at design level: `blocked_partial` is explicitly non-closeable.
- Provider hook exact file/function remains open, but acceptable because Phase 0A must resolve it before budget work.

##### User-Perspective Checks

- A future engineer can identify the first implementation step: locate and implement provider request dispatch hooks in Phase 0A.
- A future release reviewer can tell that `blocked_partial` is not closeable.
- A future agent can tell no real E3 should run until non-agent gates pass and the user approves real agent runs.
- Remaining risk: without exact-payload negative proof, a reviewer cannot distinguish real replacement from plausible metrics.

##### Required Fixes

- Add `provider_payload_artifact != empty` to release-pass requirements, or define a formal exact-payload scanner event that proves negative checks before redaction.
- Tie `legacy_taskspace_history_present=false`, `large_raw_output_tokens=0`, and `replacement_confirmed=true` to that exact scanned payload, not to projection artifacts or summaries.
- Clarify that hash-only evidence may support privacy-safe audit, but cannot by itself satisfy active replacement release proof.

##### Missing Tests

- Fixture where raw capture is disabled, hash exists, structured size exists, but legacy TaskSpace history is present: release must fail.
- Fixture proving `replacement_confirmed=true` cannot be emitted unless exact payload negative checks ran.
- Fixture where `provider_payload_artifact` is missing for an active TaskSpace request: release must fail unless the scanner event supplies equivalent exact-payload negative proof.

##### Missing Logs / Observability

- Exact-payload scan event with `request_id`, `provider_payload_sha256`, scanner/matcher version, checked byte/token ranges, negative checks performed, and pass/fail result.
- Explicit provenance field showing whether `legacy_taskspace_history_present` came from provider payload scan, redacted artifact scan, or projection artifact scan.
- Release decision evidence linking active replacement gate result back to the same `request_id` and payload hash.

##### Evidence

- `docs/v0.0.5/18-unfinished-work-engineering-design.md` - release gate listed payload hash but did not require payload artifact or exact scan event.
- `docs/v0.0.5/18-unfinished-work-engineering-design.md` - privacy fallback could still allow hash plus structured size without negative payload checks.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| architecture-adversary | Exact provider payload proof still ambiguous | Hash plus structured size does not prove stale history/raw output is absent from exact provider-visible payload. | blocking | accept | The design allowed privacy fallback without formal exact-payload scan proof. | Update design so release pass requires either searchable exact payload artifact or `exact-payload-scan-events.jsonl` tied to the same payload hash; hash-only evidence cannot satisfy active replacement. | Round 3 closure review |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: in progress
- Blocking re-review completed: yes
- Blocking re-review passed: no
- Blocking re-review round links:
  - Round 2
- Blocking re-review launch records:
  - pending Round 3
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Blocked reason: Round 2 accepted blocking finding requires design update and fresh closure review
- Allowed to proceed: no

## Round 3: Closure review for exact payload proof

### Review Input

#### Objective

Verify whether the Round 2 blocking finding about exact provider payload proof is closed.

#### Review Target

Closure review of active replacement proof requirements.

#### Target Locations

- `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- `vs_review/2026-06-19-v005-continuation-design-review.md`

#### Change Introduction

After Round 2 found hash-only fallback could still fake active replacement, the design was updated to require either a searchable exact provider payload artifact or `exact-payload-scan-events.jsonl` tied to the same `provider_payload_sha256`. Hash-only evidence is explicitly audit-only and cannot satisfy `release_pass`.

#### Risk Focus

- Whether hash-only evidence can still satisfy active replacement.
- Whether exact payload scan event is tied to request id and payload hash.
- Whether tests now cover missing payload artifact / missing scan event.

#### User-Perspective Review Focus

- Can a future release reviewer independently trust active replacement proof?

#### Assumptions To Attack

- The Round 2 blocking finding is fully closed.

#### Adversarial Lenses

- validation
- release
- observability

#### Verification Status

- Documentation changes only.
- No real E3 / Agent run was performed.

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Focus only on whether the Round 2 blocking finding is closed.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| normal | 10 minutes | bounded extension up to 5 minutes if alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| architecture-adversary | Same reviewer lens required to close the accepted architecture/release proof blocker. | exact payload proof, release evidence |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| architecture-adversary | `multi_agent_v1.spawn_agent` | `019ede02-472e-7132-9dc5-ee12d30056a5` | spawn_agent tool result nickname `Poincare` | fork_context=false | Round 3 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| architecture-adversary-r3 | architecture-adversary | 1 | `019ede02-472e-7132-9dc5-ee12d30056a5` | 10 minutes | completed | reviewer found no blocking findings | completed |

### Reviewer Outputs

#### architecture-adversary-r3

##### Summary

The Round 2 blocking finding is closed at the design-contract level. The updated design now requires active replacement release proof to include either a searchable exact provider payload artifact or an exact payload scan event, and explicitly says hash-only evidence is audit-only and cannot satisfy `release_pass`. The exact scan event is tied to `request_id` and `provider_payload_sha256`, and the replacement booleans must derive from exact payload artifact/scan provenance, not projection or summary artifacts.

##### Blocking Findings

- none

##### Non-blocking Risks

- Phase 0A wording was looser than the stricter release gate because it mentioned payload hash/artifact path support. This could be misread during implementation, even though release gate rules close the loophole.

##### User-Perspective Checks

- A future release reviewer can independently reject hash-only active replacement proof.
- The reviewer can trace exact scan proof through `request_id` and `provider_payload_sha256`.

##### Required Fixes

- none blocking
- optional cleanup: tighten Phase 0A wording so it consistently says exact payload artifact or exact pre-redaction scan event, not just payload hash/artifact path.

##### Missing Tests

- none blocking

##### Missing Logs / Observability

- none blocking

##### Evidence

- `docs/v0.0.5/18-unfinished-work-engineering-design.md` now requires exact payload artifact or exact scan event for release proof.
- `ExactPayloadScanEventV1` includes `request_id` and `provider_payload_sha256`.
- Replacement fields must point to exact payload artifact or exact scan event, not projection/summary artifacts.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| architecture-adversary | Phase 0A wording looser than release gate | Implementer might read payload hash support as enough despite release gate. | major | accept | Reviewer identified non-blocking but useful clarity issue. | Tightened Phase 0A tasks, deliverables, test, and exit criteria to require searchable payload artifact or exact pre-redaction scan event. | none |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 3
- Blocking re-review launch records:
  - `multi_agent_v1.spawn_agent` / `019ede02-472e-7132-9dc5-ee12d30056a5`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Blocked reason: n/a
- Allowed to proceed: yes

## Final Conclusion

Passed for design-review closure. The reviewed design may proceed to implementation planning, but v0.0.5 itself remains `NOT READY TO CLOSE` until the documented non-agent gates and later approved diagnostics pass.

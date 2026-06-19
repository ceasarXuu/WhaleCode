# Subagent VS Review: v0.0.5 unfinished engineering plan

- Created: 2026-06-19T15:07:27.7582235+08:00
- Updated: 2026-06-19T15:24:00+08:00
- Task: 对 v0.0.5 未完成项补充工程方案执行对抗性审查，避免再次把不完整实现、诊断运行或候选证据误判为版本收口依据。
- Report path: `vs_review/2026-06-19-v005-unfinished-plan-adversarial-review.md`
- Review mode: fresh internal subagents where available, plus main-agent local test-validity review because the third fresh subagent spawn hit the runtime concurrency limit.
- Source session policy: no inherited main-agent context for spawned reviewers.
- Status: blocked

## Round 1: Plan And Gate Challenge

### Review Input

#### Objective
Verify whether the current v0.0.5 continuation plan is sufficient to resume development safely. The plan must satisfy the v0.0.5 goal of real cost control without correctness regression, not just observability, and it must prevent premature release closure or misleading E3 claims.

#### Review Target
Design, implementation plan, experiment governance, release gates, and the current implementation direction for v0.0.5 unfinished work.

#### Target Locations
- `docs/v0.0.5/00-executive-summary.md`
- `docs/v0.0.5/08-observability-and-budget-metrics.md`
- `docs/v0.0.5/09-e3-validation-plan.md`
- `docs/v0.0.5/10-implementation-plan.md`
- `docs/v0.0.5/17-unfinished-work-inventory.md`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- `docs/experiments/README.md`
- `docs/experiments/taskspace-evidence-levels-and-samples.md`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1`
- `scripts/taskspace-benchmark/lib/e3-identity.ps1`
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1`
- `scripts/taskspace-benchmark/write-release-decision.ps1`
- `scripts/taskspace-benchmark/test-e3-start-gate.ps1`
- `scripts/taskspace-benchmark/test-release-decision.ps1`

#### Change Introduction
The current direction keeps v0.0.5 open and continues implementation around active TaskSpace prompt replacement, external E3 sample identity, suite manifest provenance, provider request attribution, budget/runtime cost gates, and release decision hardening. True E3 must remain blocked until code, gates, adversarial review, and non-agent validation are complete.

#### Risk Focus
- The plan may still confuse cost observability with actual runtime cost control.
- The release gate may rely on self-reported or synthetic artifacts rather than runtime-produced evidence.
- Diagnostic variants such as `terminal-bench_E3-P0_3_1` or `_3_2` may accidentally be used as release proof.
- The official comparable baseline may be unclear between `terminal-bench_E3-P0_3_5` and `terminal-bench_E3-v004-clean_3_5`.
- Active prompt replacement may remove old context but not enforce bounded future growth.
- Budget hard stop could improve cost by failing tasks, creating hidden correctness regression unless score eligibility is explicit.

#### Verification Status
- Real E3 is intentionally not run for this review.
- Recent non-agent checks were reported for active context replacement, start-gate sample derivation, and release-decision suite manifest fixtures.
- Runtime canonical `BudgetQualityImpactV1` production remains a known unfinished area under implementation.
- The third fresh reviewer for test-validity could not be spawned because the runtime reported `agent thread limit reached`; this is recorded as a degraded review path.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Do not run true E3 and do not call real agent benchmark execution.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| product-logic-adversary | v0.0.5 has already suffered from product conclusion drift and premature closure pressure. | Product goal, scope, release conclusion boundaries |
| architecture-adversary | The plan crosses Rust runtime, PowerShell harnesses, experiment governance, and release gates. | Runtime ownership, data contracts, maintainability |
| test-validity-adversary | Desired but not spawned due to runtime subagent concurrency limit. Main agent performs local test-validity challenge instead. | E3 naming, sample identity, repeats, fake pass prevention |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| product-logic-adversary | `multi_agent_v1.spawn_agent` explorer | `019edeb3-d8b9-7d31-9670-0af244656b23` / Herschel | spawn result in current Codex thread | no | Round 1 Review Input adapted to product focus | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| architecture-adversary | `multi_agent_v1.spawn_agent` explorer | `019edeb4-2d9f-7c30-981f-62a506efa3b7` / Linnaeus | spawn result in current Codex thread | no | Round 1 Review Input adapted to architecture focus | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| test-validity-adversary | unavailable | spawn failed: `agent thread limit reached` | failed spawn result in current Codex thread | no | Round 1 Review Input adapted to test focus | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### product-logic-adversary

##### Summary
The plan correctly recognizes that v0.0.5 cannot close yet and that the work must move from post-run observability into provider request lifecycle, runtime budget, active context replacement, state_commit displacement, spawn/node budget, and E3 start gates. The product direction is correct, but the version is only ready for implementation, not closure.

##### Blocking Findings
- Old document wording can still induce premature closure. `09-e3-validation-plan.md` and `10-implementation-plan.md` retain PASS/PARTIAL/product partial language even with supersession notes.
- Real cost control is not closed. The inventory records diagnostic evidence that TaskSpace remained at 3.66x walltime, 11.39x tokens, and 128.40x request count, far beyond v0.0.5 targets.
- The exact provider request hook location and ownership remain open in the plan, so budget enforcement could still stay design-only.
- Active context replacement remains a proof item until exact provider payload scan proves old TaskSpace history is absent.

##### Non-blocking Risks
- Budget hard stop can fake cost reduction by doing less work unless quality impact and score eligibility are strict.
- Map self-management is correctly downgraded to report-only foundation and must not count as P0 runtime cost-control success.
- Routing remains benchmark-profile controlled, not yet a product-grade runtime classifier.

##### Required Fixes
- Rewrite or archive old PASS/PARTIAL/old sample wording in `09` and `10` so it is impossible to use as current release criteria.
- Phase 0A must first locate and implement provider lifecycle producer evidence: request id, phase, payload hash/scan, token, latency, and status.
- Release decision must consume producer-owned typed artifacts, not script-inferred facts.
- Add machine-readable `blocked_partial.closeable=false`.
- Formal E3 must remain blocked until non-agent gates, code-complete marker, and user approval marker are all valid.

##### Missing Tests
- Provider request lifecycle hook fixture.
- Active context exact payload scan fixture, including a negative case where a projection artifact exists but legacy payload remains.
- Hash-only payload proof must fail release.
- Budget response synthetic tests for downgrade/hard stop and post-budget request/spawn restriction.
- Budget-induced quality impact tests for validation skip, early final, and blocked-by-budget score ineligibility.
- State_commit displacement fixture for low adoption, legacy fallback, and rejected retry pressure.
- Suite runner fixture proving `full_e3_allowed=false` schedules no samples.

##### Missing Logs / Observability
- Canonical provider request lifecycle events.
- Request phase attribution coverage and unknown phase rate.
- Active payload exact scan events bound to request id and payload hash.
- Budget response and budget quality impact events.
- State_commit adoption, displacement, rejection, retry metrics.
- Spawn/node/open-leaf budget counters and no-yield cooldown events.

##### Evidence
- `docs/v0.0.5/00-executive-summary.md:18` and `docs/v0.0.5/00-executive-summary.md:25` define the cost-control target.
- `docs/v0.0.5/08-observability-and-budget-metrics.md:100` and `docs/v0.0.5/08-observability-and-budget-metrics.md:113` define guardrails.
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:988` and `docs/v0.0.5/18-unfinished-work-engineering-design.md:991` state formal P0 release proof and diagnostic-only boundaries.
- `docs/experiments/taskspace-evidence-levels-and-samples.md:91` and `docs/experiments/taskspace-evidence-levels-and-samples.md:104` register P0 formal and diagnostic variants.

#### architecture-adversary

##### Summary
The architecture direction is broadly correct because it recognizes that cost control must move into active execution. The main risk is enforcement credibility: several gates prove artifacts exist, but runtime boundaries need stronger producer ownership to prove provider-visible requests were actually constrained.

##### Blocking Findings
- Provider request budget ownership is still split between `session/turn.rs` and `action_map/runtime.rs`. The wrapper around provider streaming is plausible, but the current proof does not cover retries, subagent turns, recovery turns, and non-main model calls.
- Active projection replacement is injected through developer history rather than an explicit provider-visible composition boundary. This may work, but it does not yet match the design requirement for a dedicated active provider-visible history builder plus exact payload proof.
- Runtime budget response remains too node/tool oriented unless provider-request budgets become primary. The observed failure is model request explosion, not only excessive tool outputs.
- Start gate marks missing v0.0.5 markers as `blocked`, not hard `fail`; this is not a direct release bypass, but it weakens formal-mode semantics and can allow expensive gate work before prerequisites are satisfied.

##### Non-blocking Risks
- State_commit displacement can be gamed if release checks only legacy action count and does not separate model-visible commits, runtime-synthesized commits, rejected retries, and fallback actions.
- Release decision validates artifact paths and hashes, but cannot prove canonical runtime production unless producer events are strongly bound.

##### Required Fixes
- Make provider request budget enforcement canonical at the provider dispatch boundary, with typed admitted/blocked/completed/failed/cancelled events.
- Replace projection injection with an explicit provider-visible composition function and exact provider payload scan.
- Treat missing or malformed v0.0.5 formal markers as hard blockers before formal scheduling.
- Add runtime-level request/spawn/node state machine tests proving budget violation changes behavior, not just reports it.

##### Missing Tests
- Provider request retry/subagent/recovery coverage.
- Active replacement negative payload-scan fixture.
- Start gate fixture where missing markers prevent formal scheduling.
- State_commit displacement fixture separating model-visible and synthesized commits.
- Budget quality impact fixture proving validation skip cannot count solved.

##### Missing Logs / Observability
- Provider lifecycle `request_phase` producer events.
- Budget quality-impact events for hard stop, thin downgrade, no-spawn, early final, and bounded recovery.
- Active replacement report bound to provider request id and payload hash.
- Spawn/node budget summary with post-budget spawn count and unreviewed subagent result count.

##### Evidence
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:105` and `docs/v0.0.5/18-unfinished-work-engineering-design.md:600` define provider lifecycle ownership.
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:2129` and `third_party/codex-cli/codex-rs/core/src/session/turn.rs:2145` show the current wrapper location.
- `docs/v0.0.5/18-unfinished-work-engineering-design.md:348` and `docs/v0.0.5/18-unfinished-work-engineering-design.md:352` define active replacement expectations.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1168` and `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1220` show node/tool budget barriers.

### Main Agent Local Test-Validity Review

##### Summary
The experiment governance document is materially improved and now defines E1-E5, diagnostic-only variants, formal sample identifiers, and comparison boundaries. However, test validity remains blocked until release artifacts and start gates make those distinctions machine-enforced rather than only documented.

##### Blocking Findings
- `docs/v0.0.5/09-e3-validation-plan.md` and `docs/v0.0.5/10-implementation-plan.md` still contain historical PASS/PARTIAL language that can be matched by humans or scripts despite supersession notes. Evidence: `docs/v0.0.5/09-e3-validation-plan.md:80`, `docs/v0.0.5/09-e3-validation-plan.md:115`, `docs/v0.0.5/10-implementation-plan.md:485`, `docs/v0.0.5/10-implementation-plan.md:492`, `docs/v0.0.5/10-implementation-plan.md:498`.
- `write-release-decision.ps1` has `blocked_partial` messaging, but the reviewed evidence only shows markdown notes and exit code, not a clear machine-readable `closeable=false` field. Evidence: `scripts/taskspace-benchmark/write-release-decision.ps1:541`, `scripts/taskspace-benchmark/write-release-decision.ps1:545`, `scripts/taskspace-benchmark/write-release-decision.ps1:652`, `scripts/taskspace-benchmark/write-release-decision.ps1:702`.
- `BudgetQualityImpactV1` is required by design and release gate, but the implementation state still appears consumer-heavy: release reads `budget-quality-impact-events.jsonl`, while a canonical runtime producer is not yet visible. Evidence: `docs/v0.0.5/18-unfinished-work-engineering-design.md:318`, `docs/v0.0.5/18-unfinished-work-engineering-design.md:327`, `scripts/taskspace-benchmark/write-release-decision.ps1:433`, `scripts/taskspace-benchmark/write-release-decision.ps1:435`.
- The start gate uses `next_allowed_command_category` and `full_e3_allowed`, but formal-mode preconditions need stronger fail/abort coverage so a diagnostic path cannot drift into formal sample scheduling. Evidence: `scripts/taskspace-benchmark/lib/e3-start-gate.ps1:49`, `scripts/taskspace-benchmark/lib/e3-start-gate.ps1:102`, `scripts/taskspace-benchmark/lib/e3-start-gate.ps1:110`, `docs/v0.0.5/18-unfinished-work-engineering-design.md:1142`, `docs/v0.0.5/18-unfinished-work-engineering-design.md:1143`.

##### Non-blocking Risks
- `cost-instrumentation.ps1` still emits PASS/PARTIAL terms for cost status; that may be acceptable as low-level cost-status vocabulary, but release reports must never equate PARTIAL with closeable. Evidence: `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1:755`, `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1:758`.
- `terminal-bench_E3-v004-clean_3_5` and `terminal-bench_E3-P0_3_5` are now documented as distinct, but future reports must table them separately. Evidence: `docs/v0.0.5/18-unfinished-work-engineering-design.md:30`, `docs/v0.0.5/18-unfinished-work-engineering-design.md:1202`, `docs/v0.0.5/18-unfinished-work-engineering-design.md:1203`.

##### Required Fixes
- Convert `09` and `10` historical release sections into explicit archive blocks or replace PASS/PARTIAL wording with non-current labels.
- Add `closeable=false` to machine-readable release decisions for `blocked_partial`.
- Ensure diagnostic-only variants write `not_release_proof=true`, `reported_evidence_level=diagnostic-only`, and cannot pass release even with 100% success.
- Require runtime-produced `BudgetQualityImpactV1` events for budget actions before release gate can pass.

##### Missing Tests
- Negative release fixture for diagnostic-only sample set with all successful pairs.
- Negative release fixture where `blocked_partial` lacks `closeable=false`.
- Start-gate/suite fixture proving missing markers abort before scheduling.
- Runtime producer fixture for `BudgetQualityImpactV1`.

##### Missing Logs / Observability
- Evidence binding sample-set derivation, suite manifest, run status, and release decision into one chain.
- Exact distinction in release markdown between diagnostic, formal P0, and v004-clean comparison tables.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| product-logic-adversary | Old `09`/`10` PASS/PARTIAL wording can still mislead closure. | blocking | accept | `09` and `10` still contain historical PASS/PARTIAL sections despite supersession notes. | No code/doc fix in this review turn; recorded as required remediation before closure. | Rewrite/archive historical sections and commit before next formal validation. |
| product-logic-adversary | Real cost control is not closed; previous diagnostic ratios still fail targets. | blocking | accept | v0.0.5 targets require <=2x/<=2.5x; diagnostics showed much higher ratios. | Keep v0.0.5 open; do not run formal E3 from this review. | Implement runtime provider budget and rerun only low-cost diagnostic after non-agent gates pass. |
| product-logic-adversary | Provider hook location/ownership remains open. | blocking | accept | `18` defines provider lifecycle as canonical producer but still requires Phase 0A hook closure. | Recorded as top implementation blocker. | Implement canonical provider lifecycle producer before budget-quality release claims. |
| product-logic-adversary | Active replacement remains unproven without exact payload scan. | blocking | accept | Current design requires exact provider payload scan, not projection artifact existence. | Recorded as blocker. | Add scan artifact and negative tests. |
| architecture-adversary | Provider request budget ownership split may miss retries/subagents/recovery. | blocking | accept | Current wrapper location is plausible but not proven across all request sources. | Recorded as architecture remediation. | Add dispatch-boundary producer and coverage fixtures. |
| architecture-adversary | Active projection replacement lacks explicit provider-visible composition boundary. | blocking | accept | Current implementation filters prompt items, but architecture should expose a named boundary and scan proof. | Recorded as design-to-code gap. | Refactor into explicit provider-visible builder or equivalent named boundary. |
| architecture-adversary | Runtime budget is too node/tool oriented unless provider budget is primary. | blocking | accept | Observed failure is provider request explosion. | Recorded as primary engineering sequence. | Complete provider request budget before relying on node/tool gates. |
| architecture-adversary | Start gate `blocked` semantics can still allow expensive gate work. | major | accept | Runner blocks full E3, but missing marker semantics are weaker than formal-mode fail/abort. | Recorded as test/gate hardening. | Add fixture and tighten formal-mode scheduling path. |
| main-agent local test-validity | `blocked_partial` lacks clear machine-readable `closeable=false`. | blocking | accept | Current script writes notes/exit code; explicit JSON closeability is needed. | Recorded as release artifact fix. | Add JSON field and negative fixture. |
| main-agent local test-validity | `BudgetQualityImpactV1` is consumer-gated but not yet runtime-produced. | blocking | accept | Release gate reads events; runtime producer remains unfinished. | Recorded as implementation blocker. | Implement producer and extraction tests before E3. |
| main-agent local test-validity | Diagnostic-only variants need machine-level release rejection. | blocking | accept | Governance docs define this, but release fixtures should prove it. | Recorded as required test. | Add negative fixture for all-success diagnostic release. |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: no
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - none yet; required after remediation
- Blocking re-review launch records:
  - none yet
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: no for formal E3/release closure; yes for implementation remediation.

## Final Conclusion

The current v0.0.5 unfinished-work plan is directionally sound and can guide the next implementation pass, but the review is blocked for release or formal E3 closure. Accepted blockers are: stale historical release wording, provider lifecycle producer ownership, runtime-produced budget quality impact, exact active payload replacement proof, stricter formal-mode start gate behavior, and machine-readable `blocked_partial.closeable=false`.

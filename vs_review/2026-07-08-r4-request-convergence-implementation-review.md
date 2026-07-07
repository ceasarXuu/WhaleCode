# Subagent VS Review: R4 request convergence implementation closure

- Created: 2026-07-08 05:27:50 +0800
- Updated: 2026-07-08 05:29:00 +0800
- Report schema: adversarial-v1
- Task: 审查 R4 request convergence 后续 Phase 的实际落地、runtime 边界收敛、验证证据与 no-go 结论是否完整可信。
- Report path: `vs_review/2026-07-08-r4-request-convergence-implementation-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context; reviewer receives only the review packet
- Status: open

## Round 1: implementation completeness and boundary review

### Review Input

#### Objective

对 R4 request convergence implementation closure 执行整体对抗性审查。重点确认 Phase 1-4 是否真正落地到生产路径，Phase 5/6 的 targeted diagnostics/no-go 是否证据充分，以及 runtime 是否仍残留越界的语义控制、动作纠正或策略注入。

#### Review Target

代码实现、测试矩阵、脚本/report gates、COE 记录、R4 工程文档和 Phase 5/6 决策。

#### Target Locations

- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs`
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1`
- `scripts/taskspace-benchmark/write-r4-public-10-tool-stress-report.ps1`
- `scripts/taskspace-benchmark/test-cost-instrumentation.ps1`
- `scripts/taskspace-benchmark/test-r4-public-10-usage-accounting-gate.ps1`
- `scripts/taskspace-benchmark/test-r4-public-10-tool-stress-plan.ps1`
- `docs/v0.0.5/build-R4/10-r4-request-convergence-engineering-plan.md`
- `docs/v0.0.5/build-R4/r4-public-10-tool-stress-plan.json`
- `docs/v0.0.5/build-R4/r4-public-10-tool-stress-report.snapshot.json`
- `coe/2026-07-03-05-03-r4-durable-evidence-gates.md`
- Targeted run artifacts:
  - `target/r4-phase5-stale-final-readiness-fix-20260708-heterogeneous/runs/terminal_bench__heterogeneous-dates/20260708-043442-428/pair-001`
  - `target/r4-phase5-request-convergence-org-20260708/runs/terminal_bench__organization-json-generator/20260708-044045-953/pair-001`
  - `target/r4-phase5-request-convergence-sqlite-20260708/runs/terminal_bench__sqlite-db-truncate/20260708-044047-534/pair-001`

#### Change Introduction

The implementation claims to close the current R4 request-convergence slice by:

- adding request reason ledger/report coverage and repeated no-delta detector fields;
- removing production runtime semantic controls such as inspect auto-transition and validation rework duplicate target read hard rejection;
- preserving output contract status and string evidence refs through state_commit;
- suppressing stale final-readiness recovery only when latest projection facts close the missing ledger ids;
- converting feedback/projection wording from strategy-like action suggestions to fact/source fields;
- running targeted diagnostics where `heterogeneous-dates` improves but `organization-json-generator` and `sqlite-db-truncate` remain TaskSpace no-go samples;
- recording Phase 6 no-go and deferring full public-10/E3.

#### Risk Focus

- Production path may still contain hidden hard rejects, auto-transitions, or strategy correction after the visible cleanup.
- Feedback/projection text may still coach Agent strategy through field names, next-action labels, or grammar snippets beyond strict tool-format requirements.
- Tests may encode implementation details rather than black-box boundary behavior.
- Phase 5 targeted sample evidence may be incomplete, stale, or overclaimed.
- Phase 6 no-go may be documented but report gates may still allow cherry-picked pass semantics.
- Request reason ledger may be logging-only and not sufficient evidence for convergence claims.
- Documentation may claim more completion than code/test/sample evidence supports.

#### User-Perspective Review Focus

- A future maintainer should be able to tell what is solved, what is no-go, and what must not be reintroduced.
- The runtime boundary should match the user principle: runtime/taskspace are tools and ledgers with hard baselines, not strategy controllers.
- Phase status should not imply E3 readiness or R4 utility success when targeted no-go samples remain.

#### Implementation Completeness Focus

- Verify every claimed production behavior has an actual production code path, not only test-only helpers.
- Check that duplicate validation rework target `read_file` is no longer rejected before ordinary tool execution in action-contract mode.
- Check that stale final-readiness recovery suppression depends on latest projection facts and does not force final_answer.
- Check that report scripts actually require request reason fields in measured rows and preserve legacy/missing rows honestly.
- Check that Phase 5/6 documentation matches the recorded run artifacts.

#### Target Benefit Focus

- Claimed benefit: reduce request amplification caused by stale/contradictory feedback and runtime overreach, while preserving Agent action authority.
- Baseline/evidence: earlier `heterogeneous-dates` loop hit 20 requests with repeated no-delta; targeted rerun solved at 11 requests with unknown=0 and repeated-no-delta=0.
- Counter-evidence: org/sqlite targeted paired samples still fail TaskSpace at 20 requests while standard solves.
- Review whether the implementation correctly treats this as diagnostic/no-go rather than benefit pass.

#### Assumptions To Attack

- Removing hard rejects is enough to close runtime boundary issues for this slice.
- Remaining action-contract instructions are purely transport/tool-format requirements.
- H-196 feedback wording cleanup removed strategy injection without reducing necessary tool feedback.
- Targeted sample artifacts are sufficient to justify Phase 6 no-go and defer full public-10/E3.
- All important changes are represented in COE/docs and tests.

#### Adversarial Lenses

- implementation-completeness
- runtime boundary
- state
- failure
- testing
- observability
- target-benefit
- documentation accuracy
- maintenance

#### Verification Status

- `cargo fmt --check`: passed
- `git diff --check`: passed
- `CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale --locked`: passed
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-cost-instrumentation.ps1`: passed
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-usage-accounting-gate.ps1`: passed
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-tool-stress-plan.ps1`: passed
- Focused Rust tests passed:
  - `implementation_recovery` 9 tests
  - `taskspace_action_contract` 77 tests
  - `action_contract_prompt` 31 tests
  - `final_readiness_recovery` 6 tests
  - `provider_request` 11 tests
  - `request_convergence` 1 test
  - `provider_response_actionability` 11 tests
  - `projection_` 22 tests
  - `validation_rework_duplicate_read` 7 tests
  - `state_commit_output_contract_status_reaches_projection` 1 test
  - `state_commit_string_evidence_refs_normalize_to_structured_refs` 1 test
- Known no-go:
  - `organization-json-generator`: standard solved, TaskSpace wrong, TaskSpace 20 requests, no `organization.json`
  - `sqlite-db-truncate`: standard solved, TaskSpace wrong, TaskSpace 20 requests, insufficient recovered rows
  - full public-10 and E3 were not run after targeted no-go.

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files and artifacts directly.
- Do not modify files.
- Cite evidence paths and line numbers where possible.
- Treat "benefit achieved" as false unless the evidence proves it; benefit gaps are non-blocking unless they hide correctness/release claims.
- Mark blocking findings when code/doc/test evidence would make the implementation closure dishonest.

### Internal Subagent Unavailable Fallback

- Internal subagent unavailable reason: n/a
- Local CLI discovery commands: n/a
- Discovered CLI candidates: n/a
- User-recommended alternative agent requested: n/a
- User approval requested: n/a
- Fallback outcome: n/a

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 20 min | 10 min if alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-completeness-adversary | The highest-value risk is whether claimed Phase completion actually landed in production code paths and whether no-go/benefit claims match evidence. | production wiring, runtime boundary completeness, validation evidence, benefit/no-go accuracy |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-completeness-adversary | `multi_agent_v1.spawn_agent` | `019f3e7b-ebf5-7593-82c0-bc53c3ef12d0` | spawn_agent tool result | `fork_context=false` | Round 1 Review Input | main-agent history, hidden reasoning, drafts, conclusions, full diff | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| reviewer-1 | implementation-completeness-adversary | 1 | `019f3e7b-ebf5-7593-82c0-bc53c3ef12d0` | <20 min | completed | blocking findings returned | accepted findings triaged below |

### Reviewer Outputs

#### reviewer-1: implementation-completeness-adversary

Summary:

- The broad Phase 6 no-go direction is correct.
- Closure was not fully honest as written because two direct evidence gaps remained:
  1. production-visible recovery/action-contract text still contained next-action coaching;
  2. targeted request-count evidence used multiple artifact counters without a declared canonical source.

Blocking finding BF-1: production-visible feedback still contains strategy/action coaching despite semantic-purity closure.

- Broken assumption: H-196 cleanup converted production feedback/projection wording from action suggestions into fact/source fields.
- Failure scenario: a model in action-contract mode receives recovery text like "Available next actions" or "Current request allowed actions are narrowed..." and follows runtime-authored strategy guidance instead of independently selecting the next semantic action.
- Evidence cited by reviewer:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs:1408` had validation action narrowing and "Do not call" wording.
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs:2171` and `2193` emitted `Available next actions`.
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs:15567-15587` inserted those recovery items in the production sampling loop.
  - `docs/v0.0.5/build-R4/10-r4-request-convergence-engineering-plan.md:267-271` claimed action-suggestion wording was converted to fact/source fields.
- Required proof: remove/reclassify production-visible phrases or narrow the closure claim.

Blocking finding BF-2: targeted sample request-count claims are not backed by a single unambiguous artifact definition.

- Broken assumption: Phase 5 targeted diagnostics have clean request-count evidence of 11/20/20 TaskSpace requests.
- Failure scenario: a maintainer reads documented "TaskSpace Requests" as measured model request evidence, but raw artifacts disagree depending on field.
- Evidence cited by reviewer:
  - docs claimed `heterogeneous-dates` TaskSpace requests = 11 and org/sqlite = 20.
  - cited right-side `request-summary.json` files expose top-level `model_request_count=1` and rollout trace `model_request_count=12/21/21`.
  - cited `request-reason-summary.json` files expose event counts `44/80/80`, not table values.
- Required proof: define canonical request-count source, explain top-level vs rollout-trace count semantics, and update the Phase 5 table/report.

Non-blocking risks:

- Phase 6 no-go is broadly honest because paired org/sqlite samples show standard solved and TaskSpace wrong, with E3 ineligible.
- `heterogeneous-dates` remains diagnostic-only because it is right-only and not utility-aggregate evidence.
- `documented_legacy_unavailable` request-reason rows are acceptable for historical snapshots but not a current measured public-10 pass.

Missing tests requested:

- Static/black-box coverage that production-visible recovery text does not reintroduce forbidden strategy phrases.
- Report test that request-count fields are derived from the canonical source and fail/flag ambiguous count source regressions.

### Main Agent Response

| Finding | Decision | Remediation |
|---|---|---|
| BF-1 | accepted | Rewrote remaining production-visible action-contract, projection, path-correction, bootstrap/routing/reborn, transition, and basemap prompt strings from strategy/action coaching into state facts, hard-baseline facts, rejected-baseline facts, action-space source facts, and transport/tool-format facts. Added COE H-197/E-379. Verified by static production scan, focused Rust tests, and Round 2 PASS. |
| BF-2 | accepted | Defined Phase 5 canonical TaskSpace request-count source as `request-phase-summary.json.provider_request_distinct_count`; updated report extractor source precedence to `request_phase_summary_provider_distinct`; added gate allow-list and usage-accounting fixture coverage; documented top-level summary vs rollout trace vs request-reason event count semantics. Added COE H-198/E-380. Verified by PowerShell report tests and Round 2 PASS. |
| right-only heterogeneous benefit warning | accepted | Kept `heterogeneous-dates` as diagnostic-only; no utility-benefit pass or E3/go claim. |
| legacy unavailable request-reason snapshot rows | accepted | Kept historical snapshot semantics; current measured public-10/E3 remains no-go and deferred. |

Post-fix verification:

```text
cargo fmt --check
  passed

git diff --check
  passed

CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale --locked
  passed

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib action_contract_prompt --locked
  passed: 31 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib projection_ --locked
  passed: 22 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib taskspace_boundary_feedback_items_do_not_emit_strategy_labels --locked
  passed: 1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib path_correction_recovery_item_is_advisory_feedback --locked
  passed: 1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib validation_rework_duplicate_read --locked
  passed: 7 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib implementation_recovery --locked
  passed: 9 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib final_readiness_recovery --locked
  passed: 6 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib provider_request --locked
  passed: 11 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib taskspace_action_contract --locked
  passed: 77 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib request_convergence --locked
  passed: 1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib provider_response_actionability --locked
  passed: 11 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib state_commit_output_contract_status_reaches_projection --locked
  passed: 1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib state_commit_string_evidence_refs_normalize_to_structured_refs --locked
  passed: 1 test

pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-cost-instrumentation.ps1
  passed

pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-usage-accounting-gate.ps1
  passed

pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-tool-stress-plan.ps1
  passed

production-segment static scan for exact BF-1 phrases in turn.rs/runtime.rs/basemap.rs
  passed: no production hits
```

## Round 2: blocking-finding closure review

### Review Input

Fresh read-only internal subagent review focused only on BF-1 and BF-2 closure. The reviewer was instructed to read current code/docs/scripts directly, reject overclaims, and return PASS or BLOCKING.

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| blocking-closure-adversary | `multi_agent_v1.spawn_agent` | `019f3e8e-7fe5-7c82-a945-ee2857d17faa` | spawn_agent tool result | `fork_context=false` | Round 2 focused closure input | main-agent history, hidden reasoning, drafts, conclusions, full diff | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| reviewer-2 | blocking-closure-adversary | 1 | `019f3e8e-7fe5-7c82-a945-ee2857d17faa` | <5 min | completed | closure review returned PASS | close accepted |

### Reviewer Outputs

#### reviewer-2: blocking-closure-adversary

Result: PASS.

- BF-1 closure: exact forbidden phrases have no production hits in `action_map/runtime.rs` or `action_map/basemap.rs`; `turn.rs` hits are in test/detector fixture areas, not production-visible runtime strings. Production path-correction text is fact/source phrasing.
- BF-2 closure: report extractor prefers `request-phase-summary.json.provider_request_distinct_count` when positive; gate allow-list includes `request_phase_summary_provider_distinct`; usage-accounting fixture asserts this precedence; docs distinguish canonical `11/20/20`, top-level summary `1`, rollout trace `12/21/21`, and request-reason event counts without claiming E3/utility benefit.
- Non-blocking residual reported: this vs_review file still needed closure status update, and one stale positive test assertion mentioned "Suggested recovery". Main agent resolved both after the review: this report now records closure, and the stale positive test assertion was replaced by fact/source assertions plus a negative assertion.

### Closure Status

- Blocking findings found: yes, BF-1 and BF-2
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2: reviewer-2 / `019f3e8e-7fe5-7c82-a945-ee2857d17faa`
- Blocking re-review launch records:
  - See Round 2 launch record above
- Rejected findings backed by evidence: none
- Deferred findings documented: yes; H-193 failed-edit actionability and H-194 long-inspect implementation efficiency remain next-slice blockers
- Implementation completeness gaps resolved or accepted by user: resolved for this closure slice
- Target benefit warnings recorded: yes; Phase 5 is diagnostic/no-go, not utility pass
- Blocked reason: none for this closure slice
- Allowed to proceed: yes, with no E3/public-10 go claim

## Final Conclusion

R4 request-convergence closure work for the current slice is complete and adversarially reviewed.

This is a closure/no-go state, not an E3 go state. The runtime-boundary blockers found in Round 1 are fixed and re-reviewed as passed. Phase 5 targeted diagnostics remain a benefit no-go because `organization-json-generator` and `sqlite-db-truncate` still fail in TaskSpace while standard solves; full public-10 and E3 stay deferred until the next failed-edit / long-inspect feedback slice is repaired and revalidated.

# Subagent VS Review: TaskSpace Typed Finish Contracts

- Created: 2026-06-12T15:10:48+08:00
- Updated: 2026-06-12T15:52:00+08:00
- Task: Continue TaskSpace v-0.0.4 Phase 4 by enforcing typed node finish contracts.
- Report path: `vs_review/2026-06-12-taskspace-typed-finish-contracts-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: passed

## Round 1: Post-implementation adversarial review

### Review Input

#### Objective
Review the completed TaskSpace v-0.0.4 Phase 4 implementation for correctness, architecture fit, and validation quality.

#### Review Target
Code implementation and tests for typed node finish contracts in TaskSpace action-map runtime.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/action_map/basemap.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Commit under review: `64631b08b Enforce TaskSpace typed node finish contracts`
- Validation commands:
  - `cargo fmt`
  - `cargo test -p codex-core action_map::basemap::tests -- --nocapture`
  - `cargo test -p codex-core action_map::runtime::tests -- --nocapture`
  - `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1`

#### Change Introduction
The change adds typed finish-contract enforcement for TaskSpace nodes: inspect/discover nodes need read/search evidence or node-tied problem-state effects; implementation nodes need edit evidence; validation nodes need successful test/build evidence plus a satisfied success criterion tied to the validation node; final synthesis nodes need satisfied or waived success criteria with evidence, at least one decision, and no open blocking question. The basemap prompt was updated to describe these contracts, and runtime tests were added or updated for the stricter contracts.

#### Risk Focus
- Whether the finish-contract checks correctly bind evidence to the node being completed.
- Whether final synthesis readiness is strong enough without blocking legitimate flows.
- Whether helpers such as problem-state detection or success-criterion matching create false positives.
- Whether test fixture changes weaken existing coverage or mask regressions.
- Whether the architecture keeps contract logic in appropriate runtime boundaries instead of prompt-only behavior.

#### Verification Status
- `cargo fmt` passed with existing nightly-only rustfmt warnings for `imports_granularity`.
- `cargo test -p codex-core action_map::basemap::tests -- --nocapture` passed.
- `cargo test -p codex-core action_map::runtime::tests -- --nocapture` passed, 152 tests.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1` passed.
- No full workspace-wide test suite was run.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Focus on findings that could invalidate Phase 4 closure or create long-term maintenance risk.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | Runtime state transitions and evidence binding changed in core code. | Correctness, false positives, edge cases |
| architecture-adversary | Typed contracts affect TaskSpace state-machine boundaries and long-term design. | Abstraction, ownership, maintainability |
| test-validity-adversary | Large fixture changes could make tests self-confirming. | Regression quality, missing black-box checks |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1.spawn_agent` explorer | `019ebaac-0124-75b0-9991-471beb537e2d` (`Euler`) | spawn_agent tool result | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless independently inspected | yes |
| architecture-adversary | `multi_agent_v1.spawn_agent` explorer | `019ebaac-5894-7180-a658-395be68e9d49` (`Popper`) | spawn_agent tool result | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless independently inspected | yes |
| test-validity-adversary | `multi_agent_v1.spawn_agent` explorer | `019ebaac-ab47-7b90-9bd3-264ab9f9e846` (`Darwin`) | spawn_agent tool result | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless independently inspected | yes |

### Reviewer Outputs

#### implementation-adversary

##### Summary
Phase 4 is close, but the typed validation-node finish contract is not fully closed. Main and child completion both route through `validate_completion_evidence`, and success-criterion evidence is bound to the same node by `result_id`, but the criterion is not bound to the successful validation/build result itself.

##### Blocking Findings
- [High] Validation nodes can complete with a satisfied criterion tied to the wrong result from the same node. The runtime checks for any successful Test/Build result on the node, then separately accepts any satisfied criterion referencing any result from that node. Evidence: `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:2657-2675`, `:7382-7393`, `:7414-7427`.

##### Non-blocking Risks
- [Medium] Final synthesis accepts success criteria with any non-empty evidence ref, including artifact-only refs. Evidence: `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:2686-2694`, `:7396-7404`, `:3154-3193`.
- [Low] Inspect-node problem-state effect is intentionally broad and can be satisfied by an open question tied to the node. Evidence: `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:2269-2288`, `:7354-7380`, `:12680-12715`.

##### Required Fixes
- Require validation nodes to satisfy criteria with at least one result from the same node where `tool_success == Some(true)` and `action_class == Some(Test | Build)`.
- Add same-node wrong-result regressions and child completion coverage.

##### Missing Tests
- Same-node successful Test plus criterion citing a failed Test result.
- Same-node successful Test plus criterion citing a successful Review/Read result.
- Child validation completion with successful Test plus bad criterion ref.

##### Missing Logs / Observability
- No dedicated structured event records typed finish-contract rejection reasons. Evidence: `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1988-1999`, `:3741-3748`.

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/action_map/basemap.rs:121-126`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:2619-2704`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:7341-7427`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:3730-3748`

#### architecture-adversary

##### Summary
The runtime boundary is broadly correct, and the implementation avoids trusting summary text by using structured `ActionClass` / `tool_success`. Phase 4 should not be treated as fully closed because final synthesis and validation-node criteria enforcement are weaker than the stated contract.

##### Blocking Findings
- [High] Final synthesis can finish with only one satisfied or waived criterion, even if other success criteria remain open. Evidence: `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:2686`, `:7396`.
- [High] Validation node success criteria only need to cite any result from the validation node, not the successful test/build result that proved the criterion. Evidence: `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:2657`, `:2670`, `:7382`.

##### Non-blocking Risks
- [Medium] Prompt/runtime drift remains manual. Evidence: `third_party/codex-cli/codex-rs/core/src/action_map/basemap.rs:121`, `:164`.
- [Medium] The implementation deepens the `runtime.rs` monolith with more helpers and fixtures.

##### Required Fixes
- Require final synthesis to prove every non-obsolete success criterion is either `satisfied` or `waived` with evidence, or explicitly define and test partial closure.
- Require validation criteria evidence to cite a successful Test/Build result from the validation node.

##### Missing Tests
- Final synthesis with two criteria where one is satisfied and one remains open must fail.
- Validation node with successful Test/Build but criterion citing a different non-test result from the same node must fail.
- Prompt/runtime drift should be tracked by a stronger test or follow-up.

##### Missing Logs / Observability
- No structured `node_finish_contract_rejected` trace/event appears to be emitted.

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:2626`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:2657`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:2670`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:2686`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:7396`
- `third_party/codex-cli/codex-rs/core/src/action_map/basemap.rs:121`

#### test-validity-adversary

##### Summary
Read-only review found one blocking test-validity/correctness gap: validation-node success criteria can be satisfied by the wrong result type/status as long as some successful Test/Build exists somewhere on the same node.

##### Blocking Findings
- [High] Validation success criteria are not bound to the successful validator result they claim to certify. Evidence: `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:2657`, `:2670`, `:7382`, `:7414`.

##### Non-blocking Risks
- [Medium] Final synthesis readiness is weakly evidenced because `task_success_criteria_satisfied` only requires `satisfied|waived` plus non-empty evidence refs. Evidence: `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:7396`.
- [Medium] Test fixtures seed readiness directly and can mirror permissive implementation. Evidence: `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:7874`.

##### Required Fixes
- Strengthen `node_satisfies_success_criterion` so the cited result is on the validation node and is a successful Test or Build tool result.
- Add regression tests for failed validator refs, review/read refs, and different-node refs.

##### Missing Tests
- Cross-node criterion evidence rejection.
- Same-node failed Test/Build criterion evidence rejection.
- Same-node non-validator result criterion evidence rejection.
- Child/subagent validation node with successful Test plus bad criterion ref.

##### Missing Logs / Observability
- Contract rejection paths return errors/blocker bodies but do not emit a specific typed contract-failure event.
- The benchmark harness smoke exercises harness/report plumbing, not typed finish-contract edge cases.

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/action_map/basemap.rs:121`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1988`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:3741`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:12718`

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| implementation-adversary / architecture-adversary / test-validity-adversary | Validation criteria can cite a wrong same-node result while a separate Test/Build passes. | blocking | accept | The runtime used separate predicates for successful validation action and satisfied criterion evidence. | Patch runtime to require the criterion evidence result itself to be successful Test/Build from the node; add main and child regressions. | Round 2 focused closure review required. |
| architecture-adversary | Final synthesis can finish with only one satisfied/waived criterion while other criteria remain open. | blocking | accept | `task_success_criteria_satisfied` used `.any(...)`, which is too weak for final synthesis closure. | Patch final synthesis readiness to require all criteria to be satisfied or waived with evidence; add multi-criterion negative regression. | Round 2 focused closure review required. |
| implementation-adversary / test-validity-adversary | Final synthesis accepts artifact-only evidence. | major | defer | Waiver/evidence strength policy is adjacent but not necessary to close the two concrete Phase 4 blockers; current Phase 4 contract says evidence refs, not accepted result evidence, for final synthesis. | No code change in this pass. | Track as future TaskSpace evidence-strength policy work. |
| implementation-adversary | Inspect-node problem-state effect is broad. | minor | reject | Phase 4 deliberately allows problem-state updates such as facts, open questions, or decisions to finish inspect work; nonblocking questions are not final closure and final synthesis blocks open blocking questions. | No code change. | None. |
| architecture-adversary | Prompt/runtime drift remains manual. | major | defer | Valid concern, but the immediate blocking runtime semantics are being fixed first. | Keep current substring prompt test; track stronger generated/golden contract mapping separately. | Future prompt/runtime contract mapping test. |
| architecture-adversary | `runtime.rs` monolith grows with helpers and fixtures. | major | defer | Valid maintainability risk; extracting a contract module is a larger refactor and should not be bundled with blocker closure. | No extraction in this pass. | Future module extraction once contract semantics stabilize. |
| all reviewers | Missing structured finish-contract rejection event. | major | defer | Valid observability gap, but current rejected main finishes return actionable errors and child finishes become blockers; event schema changes are out of scope for this closure patch. | No event schema change in this pass. | Future observability task: `node_finish_contract_rejected` trace/event. |

### Closure Status

- Blocking findings found: pending
- Accepted blocking findings fixed: yes
- Blocking re-review completed: pending
- Blocking re-review passed: pending
- Blocking re-review round links:
  - Round 2: Blocking closure review
- Blocking re-review launch records:
  - pending
- Rejected findings backed by evidence: pending
- Deferred findings documented: pending
- Allowed to proceed: pending

## Round 2: Blocking closure review

### Review Input

#### Objective
Verify closure of the accepted Round 1 blocking findings before Phase 4 is treated as passed.

#### Review Target
Closure patch for typed node finish contracts in `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `vs_review/2026-06-12-taskspace-typed-finish-contracts-review.md`
- Original commit under review: `64631b08b Enforce TaskSpace typed node finish contracts`
- Current uncommitted closure patch in working tree.

#### Change Introduction
The closure patch addresses accepted blocking findings from Round 1. Validation-node criteria now require the criterion evidence ref itself to cite a successful `ActionClass::Test` or `ActionClass::Build` `MainToolCall` result from the same validation node. Final synthesis readiness now requires all success criteria to be `satisfied` or `waived` with evidence, instead of accepting any one completed criterion. New regressions cover same-node failed validator refs, same-node non-validator refs, child validation completion with a bad criterion ref, and final synthesis with an open second criterion.

#### Risk Focus
- Whether validation-node criteria are now correctly bound to the successful validator result.
- Whether child/subagent completion uses the same stricter validation contract.
- Whether final synthesis now rejects unresolved criteria without blocking legitimate waived/satisfied criteria.
- Whether tests genuinely exercise the Round 1 counterexamples.

#### Verification Status
- `cargo test -p codex-core action_map::runtime::tests::finish_smoke_node_rejects -- --nocapture` passed, 2 tests.
- `cargo test -p codex-core action_map::runtime::tests::subagent_smoke_node_rejects -- --nocapture` passed, 1 test.
- `cargo test -p codex-core action_map::runtime::tests::finish_final_synthesis_requires_satisfied_criteria_and_decision -- --nocapture` passed, 1 test after fixing expected status/error text.
- `cargo test -p codex-core action_map::runtime::tests::finish_smoke_node_rejects_criterion_citing_other_validation_node_result -- --nocapture` passed, 1 test.
- `cargo test -p codex-core action_map::runtime::tests -- --nocapture` passed, 156 tests after adding explicit cross-node evidence rejection coverage.
- `cargo test -p codex-core action_map::basemap::tests -- --nocapture` passed.
- `cargo fmt` passed with existing nightly-only `imports_granularity` warnings.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1` passed.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Focus only on closure of accepted Round 1 blocking findings and any new blocker introduced by the closure patch.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | Validate runtime predicate correctness and same-result binding. | Correctness, state flow |
| architecture-adversary | Validate final synthesis all-criteria closure and boundary fit. | Contract semantics, maintainability |
| test-validity-adversary | Validate the new regressions actually cover reviewer counterexamples. | Test quality, self-deception risk |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1.spawn_agent` explorer | `019ebabf-ad1c-74e0-8896-8dc75476bff1` (`Herschel`) | spawn_agent tool result | no | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions | yes |
| architecture-adversary | `multi_agent_v1.spawn_agent` explorer | `019ebac0-085e-7de2-baec-1ad11a6c58c2` (`James`) | spawn_agent tool result | no | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions | yes |
| test-validity-adversary | `multi_agent_v1.spawn_agent` explorer | `019ebac0-658e-7e40-9f21-a3015a5a4fbd` (`Banach`) | spawn_agent tool result | no | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions | yes |

### Reviewer Outputs

#### implementation-adversary

##### Summary
Round 2 closure passes for the accepted blocking findings. Validation-node criteria now require the success criterion evidence ref itself to point at a same-node successful `Test` or `Build` `MainToolCall`, and child completion routes through the same `validate_completion_evidence_for` gate. Final synthesis now requires every criterion to be `satisfied` or `waived` with evidence.

##### Blocking Findings
- none

##### Non-blocking Risks
- none for the accepted Round 1 blocker closure. Previously deferred observability/evidence-strength concerns remain outside this closure patch.

##### Required Fixes
- none

##### Missing Tests
- none for the accepted blockers.

##### Missing Logs / Observability
- none newly introduced. The existing deferred gap remains: typed finish-contract rejections return errors or child blockers, but do not emit a dedicated structured rejection event.

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:2657-2677`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:2688-2691`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:7384-7408`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:7435-7451`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:3732-3750`

#### architecture-adversary

##### Summary
Closure patch satisfies the accepted Round 1 blocking architecture findings. Final synthesis now requires every success criterion to be `satisfied` or `waived` with evidence, and validation-node criteria now bind to the cited successful Test/Build `MainToolCall` result from the same node. No new blocker was identified.

##### Blocking Findings
- none

##### Non-blocking Risks
- [Medium] Final synthesis still treats any non-empty evidence ref as enough for `satisfied`/`waived`; it does not require accepted result validity. This was already deferred in Round 1 and is not required to close accepted blockers.
- [Medium] No dedicated structured finish-contract rejection event was added. Main finish returns errors and child finish records blockers, but observability remains a deferred follow-up.

##### Required Fixes
- none

##### Missing Tests
- none for the accepted Round 1 blockers.

##### Missing Logs / Observability
- No blocker. The structured `node_finish_contract_rejected` style event remains absent and should stay tracked as a follow-up.

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:2657-2677`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:2679-2692`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:7402-7408`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:7435-7454`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:3732-3750`

#### test-validity-adversary

##### Summary
Round 2 closure looks valid for the accepted Round 1 blockers. Validation success criteria now bind to the cited successful Test/Build `MainToolCall` from the same node, and final synthesis now requires every criterion to be satisfied or waived with evidence.

##### Blocking Findings
- none

##### Non-blocking Risks
- [Low] There was no explicit cross-node criterion-evidence regression in the initial closure patch. The code appeared protected by `node_has_result_id(node, result_id)` plus `result.node_id == node.id`, but a dedicated test would make the counterexample harder to regress.

##### Required Fixes
- none for blocker closure.

##### Missing Tests
- Non-blocking: explicit validation-node rejection when criterion cites a successful Test/Build result from a different node in the same active map.

##### Missing Logs / Observability
- none blocking. The previously deferred structured rejection event gap remains unchanged.

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:2657-2677`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:2679-2691`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:7435-7451`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:3732-3750`

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| implementation-adversary | Accepted Round 1 validation/final blockers are closed. | n/a | accept | Reviewer found no blocking findings after reading closure patch. | No further code action required. | none |
| architecture-adversary | Accepted Round 1 architecture blockers are closed. | n/a | accept | Reviewer found no blocking findings; final synthesis and validation evidence semantics now match accepted closure scope. | No further code action required. | none |
| test-validity-adversary | Missing explicit cross-node validation criterion regression. | minor | accept | The runtime predicate rejects cross-node refs, but explicit coverage improves regression protection. | Added `finish_smoke_node_rejects_criterion_citing_other_validation_node_result`; reran that test and full runtime tests. | none |
| architecture-adversary / test-validity-adversary | Structured finish-contract rejection event still missing. | major | defer | Same deferred observability gap from Round 1; no blocker for accepted closure. | No event schema change in this patch. | Future observability task. |
| architecture-adversary | Final synthesis evidence strength still accepts non-empty evidence refs without accepted result validity. | major | defer | Same deferred evidence-strength policy from Round 1; not part of accepted blocker closure. | No policy change in this patch. | Future evidence-strength policy task. |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2: Blocking closure review
- Blocking re-review launch records:
  - implementation-adversary: `019ebabf-ad1c-74e0-8896-8dc75476bff1`
  - architecture-adversary: `019ebac0-085e-7de2-baec-1ad11a6c58c2`
  - test-validity-adversary: `019ebac0-658e-7e40-9f21-a3015a5a4fbd`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Allowed to proceed: yes

## Final Conclusion

Passed. Round 1 accepted blocking findings were fixed, validated locally, and passed a fresh Round 2 closure review. Remaining risks are deferred non-blocking follow-ups: structured finish-contract rejection events, final-synthesis evidence-strength policy, prompt/runtime drift hardening, and possible future runtime contract module extraction.

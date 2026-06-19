# 10. Corrected Executable Implementation Plan

- Created: 2026-06-17
- Updated: 2026-06-17
- Version: v0.0.5-corrected
- Status: Corrected Draft
- Owner / Responsible: WhaleCode core engineering
- Related Systems: `action_map` runtime, `taskspace_control` handler, context history, protocol snapshot, E3 benchmark scripts

> Supersession note (2026-06-19): v0.0.5 continuation work now follows `18-unfinished-work-engineering-design.md` for unfinished P0 engineering, formal E3 sample selection, release taxonomy, and E3 start/release gates. The Phase 6 sample list in this document is retained as historical design context only. It must not be used to replace the current formal P0 release proof `terminal-bench_E3-P0_3_5`.
- Related Links: `00-executive-summary.md`, `03-protocol-compaction.md`, `04-context-projection-and-replay-control.md`, `13-design-corrections-and-engineering-contract.md`
- Risk Level: High
- Plan Type: Full

## 1. Problem Definition

v0.0.4 proved that TaskSpace can improve structure and evidence discipline, but it also amplified cost:

- too many model-visible protocol turns;
- too much repeated TaskSpace/history context;
- raw tool output can replay across later turns;
- map state records evidence, but does not yet manage active working memory;
- simple and format-sensitive tasks can be routed into heavier workflows than needed.

v0.0.5 keeps the original goal, but corrects the engineering route: first make cost and replay measurable, then ship profile-gated protocol compaction, output references, projection, map management, and routing. Cost gates apply only to the active compact profile, not to shadow instrumentation.

## 2. Current-State Facts

These facts should be rechecked at Phase 0 entry:

| Fact | Current Evidence | Engineering Impact |
|---|---|---|
| Runtime state lives in `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`. | `ActionMapRuntimeState` owns tasks, maps, trace events, sentinels, and gates. | v0.0.5 must extend this runtime instead of creating a parallel state store. |
| Tool entry is still fine-grained `taskspace_control`. | Handler accepts `start_task`, `finish_node`, `mark_result_validity`, `adopt_result`, etc. | `state_commit` must be added compatibly and legacy usage must be measured. |
| History handling truncates tool output, but does not referenceize it. | Context manager stores processed tool output in history. | Output referenceization must preserve provider tool-output invariants while hiding raw large output from later prompts. |
| Existing E3 scripts measure wall time, tool calls, and graph health. | Current reporting lacks token-summary, request-count ratio, projection tokens, and state_commit adoption rate. | Phase 0 is mandatory before claiming any v0.0.5 cost result. |

## 3. Corrected Goals

### Release Target

```text
TaskSpace v005-active direct input+output ratio <= 2.0x Standard
TaskSpace v005-active agent walltime ratio <= 2.0x Standard
TaskSpace solved >= Standard solved - 1
```

### Engineering Partial Target

Partial is acceptable only as an engineering milestone, not as a clean release claim:

```text
direct input+output ratio <= 3.0x
agent walltime ratio <= 3.0x
model_request_count_ratio <= 2.5x
root-cause outlier isolated and documented
quality gate not failed
```

### Diagnostic Metrics

`model_request_count_ratio`, `avg_input_per_request_ratio`, `state_commit_count`, `projection_tokens`, and `large_output_replay_count` explain the cost result. They do not replace the primary direct input+output and walltime gates.

## 4. Non-Goals

- Do not replace standard history in the shadow profile.
- Do not remove legacy `taskspace_control` actions in v0.0.5.
- Do not physically delete audit evidence during GC.
- Do not allow model-only salience to hide validator failures, user requirements, or unresolved blockers.
- Do not claim release success from the three-sample E3 alone; it is a focused engineering validation.

## 5. Assumptions And Verification

| Assumption | Verification Method | If Assumption Fails |
|---|---|---|
| Existing action_map tests can be extended around current fixtures. | Run targeted `codex-core` action_map tests after Phase 1. | Add fixture repair as a Phase 1 blocker before new behavior lands. |
| Responses/rollout logs contain enough usage data for request and token summaries. | Phase 0 parser audit against current E3 artifacts. | Add instrumentation in runtime/session before protocol changes. |
| Tool-output placeholders can satisfy provider tool-call response requirements. | Phase 2 integration test with large stdout and follow-up model turn. | Keep raw output truncated in prompt and mark output-ref active profile blocked. |
| Projection can be built from current task/map/result/ledger structures. | Phase 3 shadow projection diff against full developer context. | Add missing join keys or state fields before active projection. |

## 6. Phase Gate Overview

| Phase | Name | Gate Type | Can Ship Without Next Phase |
|---:|---|---|---|
| 0 | Instrumentation Baseline | Required | No |
| 1 | Transactional `state_commit` | Required | No |
| 2 | Output Referenceization | Required | No |
| 3 | Shadow Then Active Projection | Required | No |
| 4 | Map Self-Management | Required | Partial only |
| 5 | Routing / Thin / Verification-First | Required for compact profile | Partial only |
| 6 | Focused E3 Validation | Release decision | Yes |

## 7. Phased Execution Plan

### Phase 0: Instrumentation Baseline

#### Objective

Make v0.0.4 and v0.0.5 cost comparable before changing behavior.

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| E3 artifacts can be located. | Run existing E3 artifact discovery. | artifact index path | Unknown |
| Current graph health report still generates. | Run graph-health smoke. | `graph-health.json` | Unknown |

#### Implementation Tasks

- Add `token-summary.json` generation per side, pair, sample, and suite.
- Extract `model_request_count`, input tokens, output tokens, cached/uncached input when available, and parse-status fields when unavailable.
- Count `taskspace_control` actions by action name.
- Count `large_output_replay_count` from model-visible prompt/history reconstruction.
- Record `largest_tool_output_bytes` and raw-output-in-prompt violations.
- Fix thin warning semantics so "few nodes and no spawn" is not a violation.

#### Deliverables

- `token-summary.json`
- `request-summary.json`
- `taskspace-control-usage.json`
- corrected `graph-health.json`
- baseline report comparing Standard, v004 legacy TaskSpace, and empty v005 profile

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| Parser handles missing usage fields. | Unit tests with missing and malformed JSONL events. | Reports `unknown`, not zero. |
| Cost summary reconciles. | Aggregate pair totals to sample and suite. | Totals match within documented tolerance. |
| Thin warning correction. | Graph-health unit test. | Simple one-node/no-spawn run is not warned. |

#### Exit Criteria

- All v0.0.4 root-cause metrics are reproducible automatically.
- No cost gate uses missing data as a pass.
- Phase 1 is blocked if token/request summaries are unavailable.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| Usage events are incomplete. | Cost claims are invalid. | Missing token fields in >20% runs. | Mark unavailable fields explicitly. | Gate only on walltime/tool counts until instrumentation is added. |

#### Gate To Next Phase

`token-summary.json` and request-count summaries exist for at least one focused E3 smoke pair.

### Phase 1: Transactional `state_commit`

#### Objective

Reduce protocol turns without weakening evidence integrity.

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| Phase 0 metrics exist. | Inspect summary artifacts. | token/request summaries | Unknown |
| Current fine-grained action tests are understood. | Run targeted action_map tests or list known failures. | test output | Unknown |

#### Design Approach

`state_commit` is a compatible new action, not the only action. It must be transactional at section level:

- `schema_version`;
- optional `commit_id` for caller-controlled idempotency; when omitted, the handler derives an `auto-*` id from the submitted arguments so a missing id does not force a model retry;
- `active_node_id`;
- `sections` for nodes, result validity/adoption, success criteria, output contracts, fact sources, facts, decisions, blockers, and next action;
- per-section validation result;
- accepted sections mutate state;
- rejected sections return structured errors and never partially mutate that section.

#### Implementation Tasks

- Add Rust input types for `StateCommitV1`.
- Add `state_commit` branch to `taskspace_control`.
- Implement dry-run validation path.
- Add section-level validators for references, node status, result validity, criteria evidence, and decision dependencies.
- Emit `state_commit.accepted`, `state_commit.partial`, and `state_commit.rejected` trace events.
- Update prompt guidance to prefer `state_commit` at cognitive checkpoints.
- Keep legacy actions working and count their use.

#### Deliverables

- `state_commit` handler and runtime method
- transaction/idempotency tests
- legacy action compatibility tests
- `state_commit-usage.json`

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| Idempotent replay. | Submit same `commit_id` twice. | Second submission does not duplicate state. |
| Missing commit id recovery. | Submit `state_commit` without `commit_id`. | Handler derives an `auto-*` id and applies valid sections. |
| Partial rejection. | Commit valid fact and invalid result ref. | Fact accepted; invalid result section rejected with error. |
| Legacy compatibility. | Existing fine-grained action tests. | No unrelated behavior regression. |
| Cost smoke. | One focused pair. | `taskspace_control` call count decreases vs v004 baseline. |

#### Exit Criteria

- `state_commit` covers finish/validity/adoption/decision/criteria/output-contract/fact-source common path.
- Legacy actions still pass tests.
- Prompt and gate output reference `state_commit` as the preferred path.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| Large payload causes model errors. | More retries, not fewer. | `state_commit` rejection rate >20%. | Provide small templates and dry-run. | Keep legacy actions and downgrade profile. |

#### Gate To Next Phase

State updates in the smoke run primarily use `state_commit`, and invalid refs do not corrupt state.

### Phase 2: Output Referenceization

#### Objective

Stop large raw outputs from replaying into later model prompts while preserving auditability and provider protocol correctness.

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| Large-output baseline is measured. | Inspect Phase 0 summaries. | largest output and replay count | Unknown |
| Tool-output serialization path is identified. | Code inspection. | handler/context path notes | Unknown |

#### Design Approach

The provider-visible tool output remains present, but large bodies are replaced by a small placeholder:

```text
output_ref + sha256 + bytes + summary + head/tail + suggested slices
```

Full stdout/stderr is stored as an artifact. Slice-on-demand retrieves bounded content by line range, grep, head/tail, or structured summary.

#### Implementation Tasks

- Add artifact store location and retention policy for TaskSpace output refs.
- Add output referenceizer before history recording.
- Add bounded slice tool or `taskspace_control` slice action.
- Define thresholds: inline <=8KB, summarized 8-50KB, referenced >50KB.
- Add secret/sensitive-data scan before storing or exposing summaries.
- Emit output-ref trace events.

#### Deliverables

- `OutputReferenceV1` runtime type
- output artifact files
- slice-on-demand handler
- large-output policy tests

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| Provider invariant. | Large output followed by model turn. | Tool call/output pairing remains valid. |
| Prompt cleanliness. | Reconstruct next prompt. | Raw output >50KB absent. |
| Slice correctness. | Request head/tail/grep slice. | Returned slice is bounded and hash-linked. |

#### Exit Criteria

- `large_output_replay_count = 0` in focused smoke.
- Full raw output remains auditable through artifact refs.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| Summary hides key log lines. | Reduced solve quality. | Validator/log tasks regress. | Suggested slices and grep access. | Disable active output refs per profile while keeping artifacts. |

#### Gate To Next Phase

Large output no longer appears raw in subsequent prompts, and slice retrieval works.

### Phase 3: Shadow Then Active Context Projection

#### Objective

Build projection safely before using it to reduce model-visible context.

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| Output refs are active. | Phase 2 smoke. | `large_output_replay_count = 0` | Unknown |
| Projection source fields are available. | Snapshot audit. | field coverage report | Unknown |

#### Design Approach

Use two explicit profiles:

- `taskspace-v005-shadow`: builds projection and metrics, but does not replace context.
- `taskspace-v005-active`: injects projection as the TaskSpace context surface.

Cost gates apply only to `active`. Shadow is for safety, replacement potential, and debugging.

#### Implementation Tasks

- Implement `ContextProjectionV1` builder from active task, current node, criteria, blockers, accepted decisions/facts, relevant results, and next valid actions.
- Emit `projection-events.jsonl`.
- Add projection token estimator.
- Add active profile injection point.
- Preserve hidden refs available for expansion.
- Add rollback flag to return to legacy full developer context.

#### Deliverables

- projection builder
- shadow and active profile flags
- projection event artifacts
- projection budget tests

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| Shadow completeness. | Compare full context with projection. | Protected facts/criteria/blockers are present. |
| Active context budget. | Run active smoke. | Projection tokens within profile budget. |
| Rollback. | Switch active to legacy profile. | Next turn gets legacy context. |

#### Exit Criteria

- Every TaskSpace request has a projection event.
- Active profile passes smoke without missing protected evidence.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| Projection loses context. | Incorrect final answer or wrong patch. | Missing protected item in audit. | Protected-item invariant tests. | Revert to shadow/legacy context profile. |

#### Gate To Next Phase

Active projection passes focused smoke and produces bounded prompt context.

### Phase 4: Map Self-Management

#### Objective

Make map state manage active memory without deleting audit evidence.

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| Active projection works. | Phase 3 smoke. | projection reports | Unknown |
| Result lifecycle is measurable. | Phase 0/1 artifacts. | result/adoption metrics | Unknown |

#### Design Approach

Retention and salience are runtime-first:

- runtime assigns deterministic base retention;
- model may request salience boosts with evidence;
- protected evidence cannot be GC'd out of active context until resolved;
- GC means active-state transition to archived/audit-only, not physical deletion.

Protected items include user requirements, open blockers, active criteria, failed validators, current patch decisions, unresolved questions, and evidence cited by accepted decisions.

#### Implementation Tasks

- Add retention class to map items.
- Add deterministic salience score and protected-item rules.
- Add compaction operators: result->fact, node->phase summary, failure->hypothesis, subagent->decision yield/no-yield.
- Add audit-only archived state.
- Emit compaction and GC trace events.
- Add semantic replacement metrics.

#### Deliverables

- retention/salience fields
- compaction-events.jsonl
- map-management-summary.json
- protected evidence invariant tests

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| Retention coverage. | Snapshot audit. | 100% map items have retention class. |
| Protected evidence. | Unit tests. | Protected items remain in projection. |
| Compaction. | Synthetic map with stale results. | Active projection shrinks and audit refs remain. |

#### Exit Criteria

- Unreviewed active result count reduced by >=60% in focused run.
- Stale blocked nodes are absent from final projection unless still protected.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| GC hides negative evidence. | Wrong synthesis. | Failed validator missing from projection. | Protected-item rules. | Disable GC, keep retention labels only. |

#### Gate To Next Phase

Map management changes projection size without losing protected evidence.

### Phase 5: Routing / Thin / Verification-First

#### Objective

Avoid heavy TaskSpace paths for simple or format-sensitive tasks.

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| Active compact profile works. | Phase 3/4 smoke. | projection and map summary | Unknown |
| Baseline task shapes are known. | Review v0.0.4 samples. | task-shape notes | Unknown |

#### Design Approach

Start report-only, then enable active routing after evidence:

- `thin` for clear, bounded, validator-known tasks;
- `verification_first` for parser/output-format tasks;
- `default_compact` for normal work;
- `subagent_assisted` for independent evidence tracks;
- `deep` only after ambiguity or repeated failure.

#### Implementation Tasks

- Add `TaskShapeRouterV1` output artifact.
- Add route confidence and trigger reasons.
- Add escalation rules for validator failure, ambiguity, missing artifact, or cross-module dependency.
- Add stay-thin rule after a clear patch path is known.
- Keep no-spawn default for thin mode.

#### Deliverables

- `routing-decision.json`
- thin profile
- verification-first profile
- routing mistake report

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| Report-only router. | Existing v004 sample replay. | Every run has routing decision. |
| Thin no-spawn. | Thin smoke. | No default subagent spawn. |
| Verification-first. | `count-call-stack`. | Expected-format decision and local checker evidence exist. |

#### Exit Criteria

- Router is report-only clean before active.
- Active routing does not regress analyze-access-logs below Standard.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| Thin misroutes complex task. | Lower solve rate. | Validator failure with missing context. | Escalate to default/deep. | Disable active router, keep report-only. |

#### Gate To Next Phase

Focused samples show routing evidence and no severe quality regression.

### Phase 6: Focused E3 Validation And Release Decision

#### Objective

Decide whether v0.0.5 is a clean release, engineering partial, or failed experiment.

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| All previous phase gates pass. | Artifact checklist. | phase gate summary | Unknown |
| Standard and v005-active are comparable. | E3 start gate. | clean run status | Unknown |

#### Implementation Tasks

- Run focused E3 matrix on `analyze-access-logs`, `log-summary`, and `count-call-stack`.
- Generate pair, sample, and suite cost reports.
- Generate map-management summary.
- Generate routing mistake report.
- Write release decision note with PASS/PARTIAL/FAIL.

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| Engineering clean. | E3 harness. | `engineering_clean = true`. |
| Cost gate. | suite cost report. | PASS or documented PARTIAL. |
| Quality gate. | public/hidden oracle results. | TaskSpace solved >= Standard solved - 1. |
| Map gate. | map summary. | Retention/projection/GC metrics present. |

#### Exit Criteria

- PASS: primary cost and quality gates pass.
- PARTIAL: engineering partial target passes, root cause is isolated, and quality gate does not fail.
- FAIL: cost remains >5x, request ratio remains >5x, or quality gate fails.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| Three samples overfit. | False release confidence. | Success only on targeted samples. | Label as focused validation. | Require broader v0.0.6 validation before default rollout. |

#### Gate To Release

Do not call v0.0.5 release-ready unless the release decision note includes cost, quality, map, projection, output-ref, and routing evidence.

## 8. Dependencies

| Dependency | Type | Current Status | Blocking Risk | Handling Plan |
|---|---|---|---|---|
| Existing `action_map` runtime tests | system | Unknown | New gates may break fixtures. | Audit and update fixtures in Phase 1. |
| Responses/rollout usage data | data | Unknown | Token summaries may be incomplete. | Phase 0 marks unavailable fields and adds instrumentation if needed. |
| Provider tool-output ordering | third-party/API | Unknown | Output refs may violate tool protocol. | Phase 2 integration test before active profile. |
| E3 harness cleanliness | environment | Unknown | Cost results invalid. | E3 start gate remains mandatory. |

## 9. Release, Rollback, And Fallback Strategy

### Release Strategy

- Release method: profile-gated runtime behavior.
- Canary scope: focused E3 first, then local/manual TaskSpace runs.
- Expansion criteria: PASS cost and quality gates, no protected evidence loss.
- Pause criteria: missing token summaries, prompt raw output >50KB, quality regression, projection missing protected evidence.
- Owner: Unknown.
- Release window: Unknown.

### Rollback Strategy

- Rollbackable changes: profile switch from `taskspace-v005-active` to legacy/full context behavior.
- Non-directly rollbackable changes: persisted artifacts and new trace events remain as audit data.
- Rollback triggers: provider tool-output error, projection evidence loss, direct IO >5x, quality gate fail.
- Rollback validation: next run uses legacy developer context and fine-grained actions remain accepted.
- Owner: Unknown.

### Fallback / Degradation Strategy

- Degradable capability: active projection, output refs, active router, GC.
- Trigger: phase-specific gate failure.
- User-visible impact: TaskSpace remains more expensive but preserves correctness.
- System behavior while degraded: keep shadow metrics and legacy actions.
- Recovery steps: fix failing phase and rerun focused smoke.

## 10. Observability And Success Metrics

| Metric | Current Baseline | Target | Alert Threshold | Observation Window |
|---|---:|---:|---:|---|
| direct input+output ratio | v0.0.4 approx 20x | <=2x PASS, <=3x PARTIAL | >5x | focused E3 suite |
| agent walltime ratio | v0.0.4 approx 5x | <=2x PASS, <=3x PARTIAL | >5x | focused E3 suite |
| model_request_count_ratio | v0.0.4 approx 9.31x | <=2.0x PASS, <=2.5x PARTIAL | >5x | focused E3 suite |
| avg_input_per_request_ratio | v0.0.4 approx 2.16x | <=1.25x | >2x | focused E3 suite |
| large_output_replay_count | Unknown | 0 | >0 | every active run |
| projection protected-miss count | Unknown | 0 | >0 | every active run |
| state_commit rejection rate | Unknown | <=10% | >20% | focused E3 suite |
| unreviewed active result reduction | Unknown | >=60% | <30% | focused E3 suite |

## 11. Open Questions

1. Which exact rollout JSONL fields should be the source of truth for input/output/cached token counts?
2. Should output refs live under E3 artifacts only, or also under runtime session artifacts for normal local runs?
3. Should `state_commit` be a new `taskspace_control` action or a separate tool surface after v0.0.5?
4. What profile flag naming should be used in CLI/TUI config?
5. How broad must post-v0.0.5 validation be before compact profile can become default?

## 12. Plan Quality Checklist

- [x] Problem and current state are separated.
- [x] Cost metrics distinguish primary gates from diagnostics.
- [x] Shadow and active projection profiles are separated.
- [x] `state_commit` has transaction, idempotency, and partial-accept requirements.
- [x] Output referenceization preserves tool-output protocol requirements.
- [x] GC is archive/audit-only and protects negative evidence.
- [x] Each phase has deliverables, validation, fallback, and a next gate.

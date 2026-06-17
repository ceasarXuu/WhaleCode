# 13. Design Corrections And Engineering Contract

- Created: 2026-06-17
- Status: Authoritative correction for v0.0.5 implementation

This document corrects the v0.0.5 target design before implementation. Earlier v0.0.5 docs remain useful for motivation and concept detail, but this contract controls engineering decisions when there is a conflict.

## 1. Corrected Positioning

v0.0.5 is still:

```text
Protocol Compaction + Context Replay Control + Map Self-Management Foundation
```

The correction is that these capabilities must land behind explicit profiles and evidence gates. The release target remains 2x cost, but the work cannot claim cost success until the active compact profile is actually controlling model-visible context and output replay.

## 2. Cost Metric Contract

### Primary Gates

The primary cost gates are:

```text
direct input+output ratio
agent walltime ratio
```

These are the only metrics that can produce a clean cost PASS.

### Diagnostic Gates

The following explain why cost moved:

```text
model_request_count_ratio
avg_input_per_request_ratio
uncached_input_ratio
output_token_ratio
taskspace_control_call_count
state_commit_count
projection_tokens
large_output_replay_count
```

They are required in reports, but they must not be treated as mathematically independent proof of the primary 2x target.

### PASS / PARTIAL / FAIL

```text
PASS:
  direct input+output <= 2.0x
  walltime <= 2.0x
  quality gate passes

PARTIAL:
  direct input+output <= 3.0x
  walltime <= 3.0x
  model_request_count_ratio <= 2.5x
  outlier root cause isolated
  quality gate passes

FAIL:
  direct input+output > 5.0x
  or walltime > 5.0x
  or model_request_count_ratio > 5.0x
  or quality gate fails
```

## 3. Profile Contract

v0.0.5 must use two profiles:

| Profile | Behavior | Cost Gate Eligible |
|---|---|---|
| `taskspace-v005-shadow` | Builds metrics/projection/output-ref candidates without replacing context. | No |
| `taskspace-v005-active` | Uses output refs and compact projection as the model-visible TaskSpace surface. | Yes |

The shadow profile is for safety and measurement only. Cost gates apply only to active.

## 4. `state_commit` Contract

`state_commit` is a compatible new action. It must not remove old actions in v0.0.5.

Required fields:

```text
schema_version
commit_id (recommended; omitted values are recovered as handler-derived `auto-*` ids)
active_node_id
sections
```

Required Phase 1 sections:

```text
nodes
finished_nodes
blockers
result_validities
result_adoptions
success_criteria
output_contracts
fact_sources
facts
decisions
next_best_action
```

Required semantics:

- idempotent replay by caller-provided `commit_id`, with handler-derived `auto-*` ids used only to recover from missing model-formatted ids;
- section-level validation;
- section-level accept/reject result;
- no mutation from rejected sections;
- structured error for invalid refs;
- trace event for accepted, partial, and rejected commits;
- dry-run path for tests and gate suggestions.

This avoids turning protocol compaction into a larger retry loop.

## 5. Gate Response Contract

Gate failures must return structured recovery data:

```text
allowed
reason
blocking_items
next_valid_actions
missing_evidence
```

`next_valid_actions` are hints, not decisions. Runtime can describe what is structurally valid; the model must still make semantic choices.

## 6. Output Reference Contract

Large tool output must preserve provider tool-call protocol correctness.

The model-visible tool output for large bodies is:

```text
output_ref
sha256
bytes
summary
head
tail
suggested_slices
raw_output_elided = true
```

The raw output is stored as an artifact and may be accessed only through bounded slices. A next-turn prompt containing raw output >50KB is a hard failure for `taskspace-v005-active`.

## 7. Projection Contract

`ContextProjectionV1` must be generated for every TaskSpace request in both profiles.

Protected items must always be present while unresolved:

- user-stated requirements;
- active success criteria;
- current node and current blocker;
- failed validator evidence;
- open blocking questions;
- decisions that support active criteria;
- facts cited by accepted decisions;
- output refs needed by current validation.

Projection budget pressure must compact lower-priority items before removing protected items.

## 8. Map Self-Management Contract

Map management is not physical deletion.

Allowed transitions:

```text
active -> retained
active -> archived
active -> audit_only
superseded -> audit_only
```

Every map item must have:

```text
retention_class
base_salience
protected_reason optional
last_touched_turn
```

Salience is runtime-first. The model may request a salience boost or downgrade, but it cannot remove protected evidence.

## 9. Routing Contract

Routing starts report-only, then becomes active after validation.

Modes:

```text
thin
verification_first
default_compact
subagent_assisted
deep
```

Rules:

- thin tasks do not spawn subagents by default;
- verification-first requires expected-format decision and local checker evidence;
- low router confidence goes to `default_compact`;
- validator failure or ambiguity can escalate;
- once a clear patch path exists, stay thin unless new evidence expands scope.

## 10. Validation Contract

Focused E3 is an engineering validation, not a broad market-quality proof.

Required artifacts:

```text
token-summary.json
request-summary.json
taskspace-control-usage.json
projection-events.jsonl
output-ref-events.jsonl
compaction-events.jsonl
routing-decision.json
suite-cost-gate.json
suite-map-management-summary.json
release-decision.md
```

Clean release requires cost, quality, output-ref, projection, map, and routing evidence. Partial release must state exactly which gate missed and why.

## 11. 2026-06-18 Correction: Phase 4 Map Management Ownership

The v0.0.5 Phase 4 contract is snapshot-derived report management, not runtime-owned retention mutation.

Reason:

- Runtime state remains the replayable source of task, map, node, result, evidence, and ledger facts.
- `map-management-summary.json` and `compaction-events.jsonl` derive `retention_class`, `base_salience`, and `protected_reason` from that runtime observability snapshot.
- v0.0.5 must not widen the runtime protocol surface just to persist report-only salience fields.
- No physical deletion is permitted; archived and audit-only classifications are report/projection decisions, not destructive runtime GC.

Corrected acceptance:

- Every item reported in `map-management-summary.json` has retention and salience fields.
- `compaction-events.jsonl` records semantic replacement / archive-to-audit decisions.
- Missing observability source is `source_missing`, not pass.
- Runtime-owned retention state is deferred until a later version proves that active projection needs durable salience mutation rather than deterministic snapshot-derived classification.

## 12. 2026-06-18 Correction: Phase 5 Routing Ownership

The v0.0.5 routing contract is benchmark-profile controlled routing, not autonomous Rust runtime routing.

Reason:

- `TaskShapeRouterV1` depends on scenario manifest fields, visible validators, expected task budgets, and benchmark-level hidden-oracle strategy labels.
- Those inputs exist at benchmark/profile orchestration time, not inside the generic action-map runtime.
- The runtime should enforce the resulting constraints through TaskSpace gates and prompt-visible profile constraints, not infer benchmark-specific task shape on its own.

Corrected acceptance:

- `routing-decision.json` is written for the active profile.
- The TaskSpace prompt includes the selected routing constraints.
- `suite-routing-summary.json` reports route adherence and concrete routing mistakes.
- Thin/verification-first/subagent-assisted/deep modes are all representable by `TaskShapeRouterV1`.
- Runtime autonomous routing is deferred until a later version has a product-level task classifier that is independent of benchmark fixtures.

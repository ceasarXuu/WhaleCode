# v0.0.5 Completion Engineering Playbook

- Created: 2026-06-21
- Last reviewed: 2026-06-26
- Branch target: `whalecode-alpha`
- Status: split index for phased execution; not a release approval document
- Canonical design dependency: `18-unfinished-work-engineering-design.md`

This playbook has been split into phase-cohesive documents so implementation work can load only the relevant context.

## Current State

As of 2026-06-26:

```text
Phase A is implemented: profile is advisory-only, not a session/request/spawn/node hard cap.
Phase B is implemented: request phase attribution and phase summaries exist.
Phase C is implemented locally: exact_payload_scan is producer-owned and release fixtures reject synthetic/hash-only proof.
TaskSpace action-contract taskspace_control ABI repair is implemented after Phase B.
codex-core full library gate is green on the current follow-up fix.
Phase E/G plus post-ABI B-tier evidence remain blockers before Phase H or formal E3.
```

Read `00-overview-and-gates.md` and `09-module-checklist-and-closeout.md` for the current blocker list before executing any later phase.

## Read Order

1. `00-overview-and-gates.md` - purpose, closeout gates, current baseline, and phase order.
2. `01-phase-a-active-budget.md` - advisory active complexity profile and route-aware profile state.
3. `02-phase-b-request-phase-attribution.md` - request phase attribution and context propagation.
4. `03-phase-c-payload-scan-proof.md` - exact provider payload scan proof.
5. `04-phase-d-budget-quality-impact.md` - BudgetQualityImpactV1 and validator/quality semantics.
6. `05-phase-e-state-action-displacement.md` - legacy state action displacement denominator.
7. `06-phase-f-route-aware-spawn-budget.md` - route-aware spawn/node profile observability and subagent quality gates.
8. `07-phase-g-non-agent-gates-fixtures.md` - non-agent gates, release fixtures, and start-gate fixtures.
9. `08-phase-h-e3-readiness.md` - targeted diagnostic and formal E3 readiness.
10. `09-module-checklist-and-closeout.md` - module-by-module checklist, definition of done, and PR split.

## Execution Rule

Read `00-overview-and-gates.md` first, then read only the current phase file plus `09-module-checklist-and-closeout.md` when preparing closeout or PR boundaries.

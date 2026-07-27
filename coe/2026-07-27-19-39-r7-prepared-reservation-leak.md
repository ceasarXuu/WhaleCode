# Problem P-001: skipped prepared TaskSpace action leaks its reservation
- Status: confirmed
- Created: 2026-07-27 19:39
- Updated: 2026-07-27 19:39
- Objective: ensure every response-level TaskSpace reservation reaches a factual result or explicit release even when a prior sibling Tool fails
- Symptoms:
  - A later sibling Tool skipped after an earlier sibling failure retains its precommitted reservation.
- Expected behavior:
  - Every prepared action is dispatched and attributed, or is explicitly recorded as skipped/failed and released.
- Actual behavior:
  - The sequence layer returns a factual skipped Tool output but does not update the canonical Map reservation.
- Impact:
  - The declared node remains derived `InFlight`; later progress or `finish_map` can be blocked, and restart preserves the stale fact.
- Reproduction:
  - Prepare a TaskSpace response with at least two sequential segments; make the first segment fail; observe that later calls enter `prior_failure` while their reservations remain in `action_reservations`.
- Environment:
  - branch `whalecode-alpha`, production cutover commit `d64b598b8`
- Known facts:
  - All sibling reservations are committed before dispatch.
  - The `prior_failure` skip branch does not call bound result attribution or reservation release.
  - The only production release path is called from actual bound Tool dispatch.
- Ruled out:
  - Native Tool failure itself is not the leak: an actually dispatched failed Tool still runs bound result attribution with `success=false`.
- Fix criteria:
  - A focused multi-sibling failure test proves skipped prepared calls are factually recorded and released.
  - A restart/store assertion proves no skipped reservation survives canonical persistence.
  - Structured logs identify skipped-and-released prepared actions without Tool arguments or secrets.
- Current conclusion: H-001 is confirmed by independent reviewer evidence and direct production code-path tracing.
- Related hypotheses:
  - H-001
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: prior-failure branch bypasses canonical reservation release
- Status: confirmed
- Parent: P-001
- Claim: `execute_prepared_taskspace_siblings` precommits every declared reservation, but after an earlier Tool failure its `prior_failure` branch emits only skipped model outputs and never releases the corresponding later reservations.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - The response transaction is intentionally committed before native dispatch; therefore every non-dispatch exit requires a separate factual closure path.
- Falsifiable predictions:
  - If true: the preparation path commits all reservations, the skip branch has no Store mutation, and release is reachable only through actual bound dispatch.
  - If false: the skip branch or an enclosing response finalizer releases or attributes every undispatched prepared reservation.
- Diagnostic evidence plan:
  - Prediction or clause under test: trace each prepared reservation from response commit through failure-driven segment skipping to canonical release.
  - Signal: production call graph and exact mutation calls on prepare, dispatch, skip and result attribution branches.
  - Capture method: inspect `sequence.rs`, `parallel.rs` and `action_map/runtime/transactions.rs`, independently of the fresh reviewer.
  - Event name or marker:
    - `taskspace_action_reservation_committed`
    - `taskspace_native_tool_dispatched`
    - `taskspace_native_tool_result_attributed`
  - Correlation keys:
    - response control call id
    - sibling call id
    - reservation id
  - Differentiates from:
    - native Tool failures that are dispatched and attributed with `success=false`
  - Supports if:
    - the prior-failure branch generates only response output and no release transaction.
  - Refutes if:
    - a finalizer or skip-specific transaction releases every undispatched prepared reservation.
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - add a stable skipped-and-released event during repair
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: confirmed
- Repair design readiness: ready after user confirmation
- Next step: obtain repair authorization, add a failing focused test, implement factual skipped result release, and validate restart persistence.
- Blocker:
  - repair requires user confirmation under the Bug Killer evidence gate
- Close reason:
  - not closed

## Evidence E-001: fresh reviewer identifies deterministic reservation leak
- Related hypotheses:
  - H-001
- Direction: supports
- Type: external-review
- Source: internal reviewer `019fa35b-145a-7da1-911a-0d6f233725e4` in `vs_review/2026-07-27-r7-a2-b1x-review.md`
- Prediction or plan link:
  - H-001 prediction that prepared later siblings can be skipped without release
- Matched signal:
  - reviewer traced all-reservation commit, skipped response branch and actual-dispatch-only release
- Correlation keys:
  - reviewer finding `B-A2B1X-001`
- Raw content:
  ```text
  committed sibling reservations are leaked when a prior sibling fails
  Later skipped siblings only emit skipped responses; no record_taskspace_bound_tool_result call occurs in this branch.
  ```
- Interpretation: independent read-only review supports the exact mechanism and impact; it is not sufficient alone, so E-002 traces the production path independently.
- Time: 2026-07-27 19:37

## Evidence E-002: production call graph contains no skip release path
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `core/src/tools/sequence.rs`, `core/src/tools/parallel.rs`, `core/src/action_map/runtime/transactions.rs`
- Prediction or plan link:
  - H-001 diagnostic clause tracing commit, skip and release mutations
- Matched signal:
  - preparation commits every declared call; `prior_failure` only appends `skipped_responses`; release occurs only inside dispatched bound Tool result recording
- Correlation keys:
  - `ActionMapPreparedCall.call_id`
  - `ActionMapPreparedCall.reservation_id`
- Raw content:
  ```text
  prepare_taskspace_response(...) -> commits prepared.prepared_calls for all sibling calls
  if let Some(prior_call_id) = prior_failure { append skipped_responses(...); continue; }
  handle_taskspace_bound_tool_call_for_sequence(...) -> record_taskspace_bound_tool_result(...)
  release_main_action_result(...) -> rooted_dag::release_reservation(...)
  ```
- Interpretation: the skipped branch bypasses the sole release mutation. A dispatched failure is not an alternative explanation because it still enters result attribution with `success=false`.
- Time: 2026-07-27 19:39

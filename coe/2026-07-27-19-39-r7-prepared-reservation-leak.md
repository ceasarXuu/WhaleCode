# Problem P-001: skipped prepared TaskSpace action leaks its reservation
- Status: fixed
- Created: 2026-07-27 19:39
- Updated: 2026-07-27 20:12
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
  - branch `whalecode-alpha`, production cutover commit `d64b598b8`, repair commit `ebffaaa63`
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
- Current conclusion: H-001 已修复；skipped prepared action 现在以失败 ResultRef 写入 canonical Map 并释放 reservation。
- Related hypotheses:
  - H-001
- Resolution basis:
  - 修复前真实 sequence 回归稳定复现残留 reservation。
  - 修复后同用例证明 failed 与 skipped 两个 action 均闭环。
  - canonical Map restore 回归证明 skipped reservation 不会跨恢复保留。
  - 完整 `codex-core --lib` 与 `codex-tools --lib` 回归通过。
- Close reason:
  - root cause repaired and regression-protected

## Hypothesis H-001: prior-failure branch bypasses canonical reservation release
- Status: fixed
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
- Conclusion: confirmed root cause and fixed by canonical skipped-result closure
- Repair design readiness: implemented and independently reviewed
- Next step: cancellation/timeout closure 仍按原计划在 A2-B2 专项验证。
- Blocker:
  - none for H-001
- Close reason:
  - fixed by `ebffaaa63`

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

## Evidence E-003: focused production sequence test reproduces the stale reservation
- Related hypotheses:
  - H-001
- Direction: supports
- Type: failing-regression-test
- Source: `core/src/tools/sequence_taskspace_tests.rs`
- Prediction or plan link:
  - H-001 prediction that a later skipped sibling remains reserved
- Matched signal:
  - invalid `apply_patch` failed in the first barrier; later `exec_command` was skipped; canonical snapshot retained the `verify` reservation
- Correlation keys:
  - call id `verify`
  - node id `verify`
  - reservation suffix `control:1:verify`
- Raw content:
  ```text
  failed and skipped actions must both release their reservations:
  [ActionMapSnapshotReservation { node_id: "verify", tool_name: "exec_command", response_call_index: 1 }]
  ```
- Interpretation: the defect is reproduced through the real response preparation, Tool dispatch, failure and skip path rather than a synthetic Map-only fixture.
- Time: 2026-07-27 19:50

## Evidence E-004: skipped action now records a failed result and releases its reservation
- Related hypotheses:
  - H-001
- Direction: refutes-current-defect
- Type: passing-regression-test
- Source: `core/src/tools/sequence_taskspace_tests.rs`, repair commit `ebffaaa63`
- Prediction or plan link:
  - fix criterion requiring every prepared action to close
- Matched signal:
  - `prior_failure_releases_every_prepared_taskspace_reservation` passed
  - canonical reservations are empty
  - `tool-result://call/verify` exists with `is_error=true`
- Correlation keys:
  - call id `verify`
  - event `taskspace_prepared_tool_skipped_and_released`
- Raw content:
  ```text
  test tools::sequence::taskspace_tests::prior_failure_releases_every_prepared_taskspace_reservation ... ok
  ```
- Interpretation: the skip branch now uses the same canonical result/release transaction as dispatched actions without invoking the skipped ordinary Tool.
- Time: 2026-07-27 19:55

## Evidence E-005: canonical restore contains no skipped reservation
- Related hypotheses:
  - H-001
- Direction: refutes-current-defect
- Type: persistence-regression-test
- Source: `core/src/action_map/runtime/transactions.rs`, repair commit `ebffaaa63`
- Prediction or plan link:
  - fix criterion requiring restart/store safety
- Matched signal:
  - `skipped_action_release_survives_canonical_map_restore` passed
  - restored Map has zero reservations and both failed/skipped ResultRefs remain error facts
- Raw content:
  ```text
  test action_map::runtime::transactions::tests::skipped_action_release_survives_canonical_map_restore ... ok
  ```
- Interpretation: canonical serialization and restore preserve the closure rather than reconstructing an InFlight node.
- Time: 2026-07-27 19:57

## Evidence E-006: complete library regressions pass with local provider test configuration
- Related hypotheses:
  - H-001
- Direction: refutes-regression
- Type: regression-suite
- Source: local Rust test run with root `.env.local` loaded without printing credentials
- Prediction or plan link:
  - repair must not regress shared Tool or TaskSpace behavior
- Matched signal:
  - `codex-core`: 1896 passed, 0 failed, 3 ignored
  - `codex-tools`: 145 passed, 0 failed, 1 ignored
- Raw content:
  ```text
  test result: ok. 1896 passed; 0 failed; 3 ignored
  test result: ok. 145 passed; 0 failed; 1 ignored
  ```
- Interpretation: the repair is compatible with the complete compiled library suites. The separate `core/tests/suite`
  integration target still contains pre-existing removed-protocol references and is tracked as a test-harness drift, not
  evidence against this fix.
- Time: 2026-07-27 20:01

## Evidence E-007: fresh closure reviewer independently closes the blocker
- Related hypotheses:
  - H-001
- Direction: refutes-current-defect
- Type: external-review
- Source: internal reviewer `019fa378-8149-7073-a51b-e74ad57ec65d` in `vs_review/2026-07-27-r7-a2-b1x-review.md`
- Prediction or plan link:
  - repair must close skipped reservation without restoring current binding, ordinary Tool intrusion or a second Map source
- Matched signal:
  - reviewer traced skipped-result release through the canonical Session path
  - prior-failure and restore regressions cover the original failure
  - ordinary Tool registry invariant and production contract hashes pass
  - no new blocking finding
- Correlation keys:
  - reviewer round `R2-blocking-closure`
  - repair commit `ebffaaa63`
- Raw content:
  ```text
  A2-B1X may be restored to implementation-complete and proceed to A2-B2.
  ```
- Interpretation: a second fresh, read-only reviewer independently confirmed that the root cause and supporting test/log
  gaps are closed. Cancellation and timeout remain a separately bounded A2-B2 concern.
- Time: 2026-07-27 20:10

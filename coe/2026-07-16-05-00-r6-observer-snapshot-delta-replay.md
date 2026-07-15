# Problem P-001: R6 observer reports stale ActionMap state
- Status: open
- Created: 2026-07-16 05:00
- Updated: 2026-07-16 05:00
- Objective: Make production resume and offline observer reconstruct the same final ActionMap from one canonical checkpoint/delta replay implementation.
- Symptoms:
  - Phase E observer reports the initialization state while raw control results commit revisions through 7.
- Expected behavior:
  - Observer final revision, snapshot hash, nodes, edges, leases and results equal production resume for the same rollout.
- Actual behavior:
  - Observer final state remains at the last full checkpoint and does not include later snapshot deltas.
- Impact:
  - Benchmark Map conclusions are incorrect even though request, token and control counts remain readable.
- Reproduction:
  - Export ActionMap observability for the Phase E `subscription-billing-repair` TaskSpace rollout.
- Environment:
  - Linux, branch `whalecode-alpha`, runtime candidate `fa505477d`, current investigation HEAD `f700f3556`.
- Known facts:
  - The rollout contains 2 full checkpoints and 72 deltas.
  - Raw committed control revisions reach 7; observer reports the initialization state.
  - Production reconstruction already applies checkpoint and delta events.
- Ruled out:
  - Runtime failed to emit snapshot deltas.
- Fix criteria:
  - Production resume and offline export use one Rust replay implementation and produce the same restore verdict and final snapshot hash.
  - Observer full and forced-large modes build all final Map collections only from the replay proof.
  - Corrupt, missing or malformed replay input fails with a typed code and no stale fallback.
- Current conclusion: The observer owns a second, incomplete read model instead of consuming canonical Rust replay.
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: Observer checkpoint-only reconstruction omits canonical deltas
- Status: confirmed
- Parent: P-001
- Claim: The observer snapshot path recognizes full checkpoints but not snapshot deltas, while also mutating a parallel event-derived Map, so its final state can lag production replay.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - The stale observer revision matches the checkpoint boundary rather than the raw committed revision.
- Falsifiable predictions:
  - If true: raw rollout contains deltas after the checkpoint and production replay reaches a newer snapshot than observer output.
  - If false: observer consumes the same delta chain and final hash as production resume.
- Diagnostic evidence plan:
  - Prediction or clause under test: Compare rollout event types, raw committed revision and observer final revision.
  - Signal: checkpoint/delta counts and final revisions.
  - Capture method: Inspect the frozen Phase E rollout and observer output; trace both code paths.
  - Event name or marker:
    - `map_runtime_snapshot_delta`
  - Correlation keys:
    - rollout SHA256 `fe4aba73fd99632c2c96b35aca7f7bd0858e144cbbf532c38626b5c8266daddd`
  - Differentiates from:
    - H-002
  - Supports if:
    - Deltas exist and production code consumes them while observer does not.
  - Refutes if:
    - Both paths invoke the same reducer and still diverge.
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - Promote replay proof fields after repair.
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: confirmed
- Repair design readiness: ready; user authorized Phase E implementation
- Next step: Extract canonical replay and switch observer to its proof.
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: Runtime did not persist later ActionMap state
- Status: refuted
- Parent: P-001
- Claim: The stale observer state is caused by Runtime failing to emit snapshot changes after initialization.
- Layer: root-cause
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - A missing persistence event could also produce a stale offline report.
- Falsifiable predictions:
  - If true: the rollout contains no valid deltas after the initialization checkpoint.
  - If false: the rollout contains a valid delta chain and raw control revisions reach 7.
- Diagnostic evidence plan:
  - Prediction or clause under test: Count persisted checkpoint and delta events.
  - Signal: rollout event counts and raw control revisions.
  - Capture method: Mechanical rollout scan.
  - Event name or marker:
    - `map_runtime_snapshot_delta`
  - Correlation keys:
    - rollout SHA256 `fe4aba73fd99632c2c96b35aca7f7bd0858e144cbbf532c38626b5c8266daddd`
  - Differentiates from:
    - H-001
  - Supports if:
    - No later delta exists.
  - Refutes if:
    - Later deltas and committed revisions exist.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: refuted
- Repair design readiness: not applicable
- Next step: closed as alternative.
- Blocker:
  - none
- Close reason:
  - contradicted by persisted delta evidence

## Evidence E-001: Frozen Phase E rollout contains later committed state
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: reproduction
- Source: `benchmarks/taskspace/r6/phase-e-finish-boundary-result.json`
- Prediction or plan link:
  - H-001 and H-002 persisted-delta prediction.
- Matched signal:
  - 2 checkpoints, 72 deltas, committed revisions 2 through 7.
- Correlation keys:
  - rollout SHA256 `fe4aba73fd99632c2c96b35aca7f7bd0858e144cbbf532c38626b5c8266daddd`
- Raw content:
  ```text
  observer showed initialization state; raw control outputs committed revisions through 7
  ```
- Interpretation: Runtime persistence advanced beyond the observer state, supporting an observer replay defect and refuting missing persistence.
- Time: 2026-07-16 05:00

## Evidence E-002: Production and observer use different reconstruction code
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `core/src/session/rollout_reconstruction.rs`, `scripts/export-action-map-observability.ps1`, `scripts/action-map-observability-summary-lib.ps1`
- Prediction or plan link:
  - H-001 code-path divergence prediction.
- Matched signal:
  - Production calls `apply_snapshot_delta`; observer maintains separate PowerShell final-state collections.
- Correlation keys:
  - none
- Raw content:
  ```text
  reconstruct_map_runtime_state applies SnapshotUpdated and SnapshotDelta;
  observer snapshot replay recognizes SnapshotUpdated and separately mutates lifecycle state.
  ```
- Interpretation: Two read models explain the observed revision divergence.
- Time: 2026-07-16 05:00

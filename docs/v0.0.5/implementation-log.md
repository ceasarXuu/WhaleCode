# v0.0.5 Implementation Log

## 2026-06-17 Phase 0 instrumentation baseline start

Changed:

- Added benchmark cost instrumentation artifact generation.
- `metrics.json` now links to `token-summary.json`, `request-summary.json`, and `taskspace-control-usage.json`.
- Missing usage data is reported as unavailable or partial, never as zero.
- `taskspace_control` action counts are parsed from execution JSONL arguments.
- Thin TaskSpace runs with few nodes and no `spawn_agent` no longer emit `thin_mode_violation` by default.

Validation:

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-phase0-348b7a2c
TaskSpace benchmark harness self-test: PASS
```

Notes:

- The first failed run exposed a real artifact writer issue: missing output directories must be created before writing cost summaries.
- Reusing a self-test run root can trip old harness assumptions; use a fresh run root for deterministic local validation.
- The harness had one stale relative-vs-absolute path assertion for latest run discovery; the assertion now compares resolved paths.

Remaining Phase 0 work:

- Produce a focused E3 smoke pair with the new artifacts.
- Replace replay detection heuristics with model-visible prompt/history reconstruction once the active profile work starts.

## 2026-06-17 Phase 0 cost aggregate and gate

Changed:

- Added sample and suite level cost aggregation from side `metrics.json` files.
- Sample and suite roots now emit aggregate `token-summary.json`, `request-summary.json`, `taskspace-control-usage.json`, and `suite-cost-gate.json`.
- Cost gate status is fail-closed when request/token/walltime fields are missing.
- `suite-cost-gate.json` reports PASS, PARTIAL, or FAIL using the v0.0.5 corrected thresholds.
- Artifact-only finalization and E3 suite completion now regenerate cost aggregate artifacts.

Validation:

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-cost-813cef87
TaskSpace benchmark harness self-test: PASS

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-score-validity.ps1 -RunRoot target\e3-score-v005-cost-c8c45d87
E3 score-validity self-test: PASS
```

Notes:

- PowerShell `[ordered]` values are `OrderedDictionary`; passing them to a `[hashtable]` parameter copies the map and loses mutation. Cost aggregation uses `System.Collections.IDictionary` for in-place totals.

Remaining Phase 0 work:

- Run a real focused E3 smoke pair and confirm the generated aggregate artifacts from live Whale execution JSONL.
- Replace replay detection heuristics with model-visible prompt/history reconstruction once the active profile work starts.

## 2026-06-17 Phase 0 live smoke

Command:

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 -Scenario single-file-fast-fix -Repeats 1 -RunRoot target\v005-phase0-live-smoke-68ebc8a0 -TimeoutSeconds 600 -ValidationTimeoutSeconds 180 -ValidationPretestTimeoutSeconds 60 -ValidationTestTimeoutSeconds 180 -SandboxMode workspace-write -EnableAggregate -AllowNonE2Result
```

Result:

- Run exited 0.
- Run dir: `target\v005-phase0-live-smoke-68ebc8a0\single-file-fast-fix\20260617-012541-362`
- Side `token-summary.json`, `request-summary.json`, `taskspace-control-usage.json`, and sample `suite-cost-gate.json` were generated from live Whale JSONL.
- Usage parsing was measured for both sides.
- `suite-cost-gate.json` failed correctly, not from missing data:
  - direct input+output ratio: `5.1391`
  - walltime ratio: `2.0234`
  - model request count ratio: `1`
- TaskSpace runtime was active: observability showed `tasks=1`, `maps=1`, `nodes=4`, and `mapRuntimeEvents=121`.

Follow-up fix:

- Live exec JSONL does not expose `taskspace_control` as model-visible tool calls in this path, so `taskspace_control_count=0` is accurate but insufficient for Phase 0 overhead analysis.
- Added `taskspace_runtime_event_count` and `runtime_event_counts` from `action-map-observability.json`.
- Reprocessing the live smoke right side produced:
  - `taskspace_runtime_event_count=121`
  - `snapshot_updated=39`
  - `cognitive_state_updated=32`
  - `node_result_recorded=13`
  - `node_status_changed=12`

Validation:

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-runtime-f8f4c39c
TaskSpace benchmark harness self-test: PASS
```

Remaining Phase 0 work:

- Replace replay detection heuristics with model-visible prompt/history reconstruction once the active profile work starts.

## 2026-06-17 Phase 1 transactional state_commit start

Changed:

- Added compatible `taskspace_control(action=state_commit)` parsing with `schema_version=taskspace-state-commit-v1`.
- Added runtime `state_commit_for_main` with section-level transaction semantics for:
  - `success_criteria`
  - `fact_sources`
  - `facts`
  - `decisions`
  - `next_best_action`
- Each section is applied on a cloned runtime candidate and committed only if that whole section validates.
- Valid sections can land while invalid sections return structured section errors.
- Runtime emits `state_commit.section.<section>.accepted`, `state_commit.section.<section>.rejected`, and final `state_commit.accepted|partial|rejected` cognitive events.
- Cognitive protocol prompt now prefers `state_commit` for multi-record checkpoints while keeping legacy `record_*` actions available for targeted corrections.

Validation:

```text
cargo test -p codex-core state_commit --manifest-path third_party/codex-cli/codex-rs/Cargo.toml
2 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-statecommit-a9d2
TaskSpace benchmark harness self-test: PASS
```

Notes:

- The first implementation avoided adding a parallel TaskSpace state model. `state_commit` delegates to existing ledger/cognitive validation paths, so behavior stays compatible with legacy actions.
- Section-level rollback is implemented by applying each section to a candidate runtime clone, then replacing the main runtime only on success. This keeps successful sections durable while preventing partially written records inside a rejected section.
- Benchmark self-test had a stale long-path assumption: it asserted a legacy `.tmp.<guid>` path boundary while the writer used a shorter temp suffix. The writer now falls back to a short same-directory temp name when the naive temp path would exceed Windows path limits, and the self-test dynamically pads the fixture path to exercise that boundary.

Remaining Phase 1 work:

- Add live smoke evidence showing the model uses fewer `taskspace_control` calls after prompt guidance.

## 2026-06-17 Phase 1 validation evidence gate hardening

Changed:

- Relaxed SmokeTest/RegressionTest completion evidence matching to accept a non-empty `validator_ref` when the current validation node already has a successful test or build tool result.
- Kept the stricter `result_id` path intact for exact result binding.
- Added regression coverage for the live-smoke failure mode where the model records a satisfied test criterion with `validator_ref` text but not the exact runtime `result_id`.

Validation:

```text
cargo test -p codex-core finish_smoke_node --manifest-path third_party/codex-cli/codex-rs/Cargo.toml
5 passed

cargo test -p codex-core state_commit --manifest-path third_party/codex-cli/codex-rs/Cargo.toml
3 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-validationgate-6c1d
TaskSpace benchmark harness self-test: PASS
```

Operational lesson:

- Before any live TaskSpace benchmark, rebuild and reinstall the local `whale.exe`; otherwise the installed binary may not contain the latest prompt/runtime changes even when the source tree is current.
- A repeated validation-gate rejection can dominate token cost. Inspect `action-map-observability.json` for recurring completion errors before treating a cost-gate failure as only model behavior.

Remaining Phase 1 work:

- Rebuild/install Whale and rerun the Phase 1 live smoke against the gate fix.
- If the model still does not use `state_commit`, strengthen the immediate TaskSpace activation prompt instead of relying only on the BaseMap cognitive protocol section.

## 2026-06-17 Phase 1 lifecycle state_commit sections

Changed:

- Extended `state_commit` beyond cognitive ledger sections to lifecycle sections:
  - `nodes`
  - `finished_nodes`
  - `blockers`
  - `result_validities`
  - `result_adoptions`
- Lifecycle sections reuse existing runtime validation and event paths for `create_node`, `finish_node`, `block_node`, `mark_result_validity`, and `adopt_result`.
- Handler parser now accepts lifecycle state_commit payloads and validates node kinds / next node drafts before runtime execution.
- Cognitive protocol prompt now describes the full `state_commit` section set.

Validation:

```text
cargo test -p codex-core state_commit --manifest-path third_party/codex-cli/codex-rs/Cargo.toml
3 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-lifecycle-4e6b
TaskSpace benchmark harness self-test: PASS
```

Notes:

- Lifecycle transaction coverage uses the same candidate-runtime rollback strategy as cognitive sections. A failing `nodes` section no longer leaves earlier node creations behind, while unrelated successful sections still land.

Remaining Phase 1 work:

- Add live smoke evidence showing the model uses fewer `taskspace_control` calls after prompt guidance.

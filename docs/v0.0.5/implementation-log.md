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

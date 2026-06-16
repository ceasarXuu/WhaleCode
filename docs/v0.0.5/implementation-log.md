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

## 2026-06-17 Phase 1 live smoke gatefix and prompt guard

Live smoke:

```text
cargo build -p codex-cli --bin whale --locked --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
Finished dev profile

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-whale-local.ps1 -BinaryPath D:\BuildCache\whalecode\cargo-target\debug\whale.exe -PersistUserPath -BackupLegacyCopies
Installed Whale hash: 518B30485B3C7940817048EC7E98DD1FA243DE1ACBB07DFB63F65FC5F30E6C2D

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 -Scenario single-file-fast-fix -Repeats 1 -RunRoot target\v005-phase1-live-smoke-gatefix-518b -TimeoutSeconds 600 -ValidationTimeoutSeconds 180 -ValidationPretestTimeoutSeconds 60 -ValidationTestTimeoutSeconds 180 -SandboxMode workspace-write -EnableAggregate -AllowNonE2Result
RunDir: target\v005-phase1-live-smoke-gatefix-518b\single-file-fast-fix\20260617-023002-976
```

Observed:

- The previous repeated validation completion error did not dominate the run after the `validator_ref` gate fix.
- `taskspace_control_count=0`, `state_commit_count=0`, and `taskspace_runtime_event_count=274`.
- TaskSpace created 10 nodes and attempted 3 subagent spawns for a single-file fast fix.
- Suite cost gate still failed: direct input/output ratio `36.481`, wall-time ratio `8.9677`, model request ratio `1`.

Changed:

- Immediate TaskSpace activation now directs multi-record contract/fact/source/success-criteria updates to `taskspace_control(action=state_commit, schema_version=taskspace-state-commit-v1)`.
- Runtime developer context now states that small single-file fixes and one failing-test loops should stay on the main-agent path unless the user explicitly asks for multi-agent work or there are clearly independent evidence surfaces.
- `spawn_agent` missing-plan feedback now tells the model to continue on the main agent for small single-file fixes instead of only suggesting `record_subagent_plan`.
- Added prompt contract assertions so the small-task main-agent guard and state_commit guidance cannot silently disappear.

Validation:

```text
cargo test -p codex-core create_seed_map_sets_active_map_context --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

cargo test -p codex-core spawn_assignment_requires_recorded_subagent_plan --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-promptguard-9a3f
TaskSpace benchmark harness self-test: PASS
```

Remaining Phase 1 work:

- Rebuild/install Whale and rerun live smoke with the prompt guard.
- Pass criteria remain: nonzero `state_commit_count`, reduced runtime event expansion, no subagent warnings on `single-file-fast-fix`, and cost gate at least partial before moving to Phase 2.

## 2026-06-17 Phase 1 prompt guard live smoke and subagent yield gate

Live smoke:

```text
cargo build -p codex-cli --bin whale --locked --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
Finished dev profile

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-whale-local.ps1 -BinaryPath D:\BuildCache\whalecode\cargo-target\debug\whale.exe -PersistUserPath -BackupLegacyCopies
Installed Whale hash: F0FE137E32FB0AF2700F18492DEE920DDC4348092A73DC80FF61BCCF8461F34A

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 -Scenario single-file-fast-fix -Repeats 1 -RunRoot target\v005-phase1-live-smoke-promptguard-f0fe -TimeoutSeconds 600 -ValidationTimeoutSeconds 180 -ValidationPretestTimeoutSeconds 60 -ValidationTestTimeoutSeconds 180 -SandboxMode workspace-write -EnableAggregate -AllowNonE2Result
RunDir: target\v005-phase1-live-smoke-promptguard-f0fe\single-file-fast-fix\20260617-024816-297
```

Observed:

- Runtime expansion improved but did not pass the gate:
  - runtime events: `274 -> 189`
  - nodes: `10 -> 9`
  - spawn_agent calls: `3 -> 2`
  - direct input/output ratio: `36.481 -> 26.9127`
  - wall-time ratio: `8.9677 -> 7.2765`
- `state_commit_count` remained `0`.
- The remaining subagent plans had no `supports_questions`, `tests_hypotheses`, or `depends_on_results`, and graph health reported `subagent_no_decision_yield`.

Changed:

- `record_subagent_plan` now rejects yieldless plans. A subagent plan must include at least one `supports_questions`, `tests_hypotheses`, or `depends_on_results` reference so the result has a concrete adoption path.
- Test fixtures now attach a synthetic support question when preparing valid spawn assignments.
- Added a regression test that rejects an otherwise valid-looking subagent plan with no decision-yield reference.

Validation:

```text
cargo test -p codex-core subagent_plan --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
3 passed

cargo test -p codex-core spawn_assignment --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
9 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-yieldref-c72e
TaskSpace benchmark harness self-test: PASS
```

Operational lesson:

- Benchmark `RunRoot` names are part of the prompt isolation surface. Do not include treatment labels such as `subagent` in run directory names; the harness correctly rejects them as prompt leakage.

Remaining Phase 1 work:

- Rebuild/install Whale and rerun live smoke with the subagent yield gate.
- If `state_commit_count` remains zero, add a stronger compatibility path: return runtime hints from legacy multi-record actions or make the benchmark usage parser count runtime `state_commit.*` events separately from model-visible tool calls.

## 2026-06-17 Phase 1 yield gate live smoke

Live smoke:

```text
cargo build -p codex-cli --bin whale --locked --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
Finished dev profile

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-whale-local.ps1 -BinaryPath D:\BuildCache\whalecode\cargo-target\debug\whale.exe -PersistUserPath -BackupLegacyCopies
Installed Whale hash: 476D5A7DA6E110ECDA9BB8EBFA1AE2F5EB79BB625A7331701C7A50D94ED36E0F

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 -Scenario single-file-fast-fix -Repeats 1 -RunRoot target\v005-phase1-live-smoke-yieldref-476d -TimeoutSeconds 600 -ValidationTimeoutSeconds 180 -ValidationPretestTimeoutSeconds 60 -ValidationTestTimeoutSeconds 180 -SandboxMode workspace-write -EnableAggregate -AllowNonE2Result
RunDir: target\v005-phase1-live-smoke-yieldref-476d\single-file-fast-fix\20260617-030215-242
```

Observed:

- Scenario warnings cleared:
  - nodes: `4`
  - edges: `3`
  - spawn_agent calls: `0`
  - subagent results: `0`
- Runtime expansion improved again:
  - runtime events: `189 -> 112`
  - direct input/output ratio: `26.9127 -> 13.6204`
  - wall-time ratio: `7.2765 -> 4.1801`
  - input tokens: `2,936,618 -> 1,259,244`
- Cost gate still failed against Phase 0 thresholds:
  - direct input/output threshold for partial: `3`
  - wall-time threshold for partial: `3`
- `state_commit_count` remained `0` in model-visible usage, while runtime still produced cognitive state updates.

Conclusion:

- The yield gate fixed the major Phase 1 subagent over-expansion bug for `single-file-fast-fix`.
- Remaining overhead is no longer caused by subagent fan-out. The next bottleneck is TaskSpace prompt/context size and legacy multi-action update shape, which overlaps Phase 2 context projection/output referenceization work.
- Before claiming Phase 1 complete, either `state_commit` adoption must be proven through model-visible usage or the metric must be corrected to distinguish model-visible tool calls from runtime-synthesized cognitive updates.

Remaining Phase 1 work:

- Add an explicit runtime/usage metric for state-commit adoption vs legacy cognitive updates, because current `taskspace_control_count=0` is insufficient when runtime events are synthesized outside model-visible `whale-exec.jsonl` tool-call records.
- Decide whether to keep enforcing Phase 1 partial cost gate before Phase 2 or move context-size reduction into Phase 2 as the next necessary dependency.

## 2026-06-17 Phase 1 runtime state_commit metric

Changed:

- Added `runtime_state_commit_count` to taskspace control usage artifacts.
- The existing `state_commit_count` remains model-visible `taskspace_control(action=state_commit)` count from exec JSONL.
- The new field counts observability timeline events whose `details.updateKind` starts with `state_commit`, so runtime-synthesized state commits can be distinguished from legacy cognitive updates.
- Aggregate cost artifacts now sum `runtime_state_commit_count` for taskspace and standard modes.
- `metrics.json` now exposes `runtime_state_commit_count`.

Validation:

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-runtimecommit-3f8a
TaskSpace benchmark harness self-test: PASS

powershell -NoProfile -Command '. scripts\taskspace-benchmark\lib\cost-instrumentation.ps1; $out = Write-TaskspaceCostInstrumentationArtifacts ...; $out.taskspace_control_usage | ConvertTo-Json -Depth 10'
runtime_state_commit_count: 0
taskspace_runtime_event_count: 112
```

Conclusion:

- The latest live smoke did not merely miss model-visible tool-call counting; it also had no runtime `state_commit.*` events. Phase 1 still needs stronger model/tool routing if `state_commit` adoption remains a gate.

## 2026-06-17 Phase 1 preflight state_commit routing

Changed:

- Updated the cognitive preflight failure remediation to prefer one `taskspace_control(action=state_commit, schema_version=taskspace-state-commit-v1)` call with `sections.success_criteria`, `sections.fact_sources`, and related facts/decisions/next action.
- Kept legacy `record_success_criteria`, `record_output_contract`, and `record_fact_source` in the error text only as focused single-record correction paths.
- Updated the problem-ledger context hint for missing success criteria to point at `state_commit` instead of a legacy single-record action.

Validation:

```text
cargo test -p codex-core cognitive_preflight_requires_problem_success_criteria --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

cargo test -p codex-core create_seed_map_sets_active_map_context --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-preflightcommit-6b2d
TaskSpace benchmark harness self-test: PASS
```

Remaining Phase 1 work:

- Rebuild/install Whale and rerun live smoke to verify whether preflight routing produces nonzero `state_commit_count` or `runtime_state_commit_count`.

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

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

## 2026-06-17 Phase 2 routing fanout hardening

Changed:

- Promoted the pre-fix diagnostic routing rule from prompt guidance into runtime validation.
- `smoke_test` and `regression_test` node creation now rejects title/context markers for pre-fix diagnostics, baselines, and `emit_large_log` diagnostic commands.
- `finish_node` next-node drafts use the same validation, so the model cannot finish an inspect node into a misclassified diagnostic validation node.
- Detached `inspect_code_context` creation is now rejected after a completed narrow main-agent inspect pass. This prevents state_commit from polluting a single-track task with ready inspect fanout that later fails at spawn time.
- Added regression coverage for direct node creation, next-node draft creation, and lifecycle `state_commit` node-section rejection.

Validation:

```text
cargo test -p codex-core validation_node --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
4 passed

cargo test -p codex-core narrow_main_inspect --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
2 passed

cargo test -p codex-core state_commit_rejects_detached_inspect_after_narrow_main_inspect --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-routing-fanout
TaskSpace benchmark harness self-test: PASS

cargo build -p codex-cli --bin whale --locked --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
Finished dev build

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-whale-local.ps1 -BinaryPath D:\BuildCache\whalecode\cargo-target\debug\whale.exe -PersistUserPath -BackupLegacyCopies
Installed Whale: C:\Users\77585\.whale\bin\whale.exe
SHA256: F993C095F4C16E843354F4F5420A8ACE6FA7B7F9C3019F9860EC891524A52084

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 -Scenario large-output-ref-smoke -Repeats 1 -RunRoot target\v005-phase2-live-smoke-routing-fanout-f993 -TimeoutSeconds 600 -ValidationTimeoutSeconds 180 -ValidationPretestTimeoutSeconds 60 -ValidationTestTimeoutSeconds 180 -SandboxMode workspace-write -EnableAggregate -AllowNonE2Result
RunDir: target\v005-phase2-live-smoke-routing-fanout-f993\large-output-ref-smoke\20260617-070750-573
```

Live smoke evidence:

- `reported_evidence_level`: E2-candidate.
- right/taskspace: `business_success=True`, `exec_exit_code=0`, `public_validation_exit_code=0`, `hidden_oracle_exit_code=0`.
- Graph recovered from the prior fanout failure: `nodes=4`, `edges=3`, `open_leaf_nodes=0`, `spawn_agent_calls=0`, `edge_order_violations=0`, scenario warnings `none`.
- Observability nodes were the intended chain: `inspect_code_context -> implement_solution -> regression_test -> final_synthesis`, all completed.
- Remaining engineering-clean blockers are harness/environment level: `repeats_lt_3`, `cleanup_not_attempted_manifest_missing`, `no_tests_started_marker`.

Remaining Phase 2 work:

- This live smoke did not create a runtime output reference: `runtime_output_ref_created_count=0`. The model redirected the diagnostic output to `diagnostic_output.txt`, which kept `large_output_replay_count=0` and `raw_output_in_prompt_violation=false` but did not prove the output-reference path in this run. The next fix should prevent this bypass or add an oracle that requires direct large stdout capture.

## 2026-06-17 Phase 2 output-ref scenario contract

Changed:

- Added `expected.min_taskspace_runtime_output_ref_created_count` to the benchmark scenario contract.
- `large-output-ref-smoke` now requires at least one TaskSpace runtime output reference.
- Pair reports now show `taskspace_runtime_output_ref_created_count_below_expected` when a scenario requires output refs but the taskspace side creates too few.
- Resume/reclassification and live run paths both apply the scenario output-ref contract. Contract misses become metrics taints and therefore engineering-unclean evidence.
- Added harness self-test coverage for the output-ref contract miss path.

Validation:

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-outputref-contract-livepath-2
TaskSpace benchmark harness self-test: PASS

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 -Scenario large-output-ref-smoke -Repeats 1 -RunRoot target\v005-phase2-live-smoke-outputref-contract-livepath-2 -TimeoutSeconds 600 -ValidationTimeoutSeconds 180 -ValidationPretestTimeoutSeconds 60 -ValidationTestTimeoutSeconds 180 -SandboxMode workspace-write -EnableAggregate -AllowNonE2Result
RunDir: target\v005-phase2-live-smoke-outputref-contract-livepath-2\large-output-ref-smoke\20260617-072905-062
```

Live smoke evidence:

- `runtime_output_ref_created_count=1`.
- `large_output_replay_count=0`.
- `raw_output_in_prompt_violation=false`.
- `business_success=True`, `exec_exit_code=0`, `public_validation_exit_code=0`, `hidden_oracle_exit_code=0`.
- `open_leaf_nodes=0`, `spawn_agent_calls=0`.
- Remaining scenario warning: `taskspace_node_count_exceeds_expected: 9 > 4`.
- Remaining engineering-clean blockers: `repeats_lt_3`, `cleanup_not_attempted_manifest_missing`, `no_tests_started_marker`.

Remaining Phase 2 work:

- Route expansion still regressed under the output-ref-positive live smoke: the graph completed but used 9 nodes against the scenario target of 4. The next fix should inspect the observability graph for the redundant node pattern and close that path without weakening output-ref enforcement.

Follow-up note:

- Fixed the live runner's output-ref contract write-back path to locate the taskspace side through `pair.Left` / `pair.Right`; the pair object has no `Sides` collection.
- Latest live run after that fix:
  - RunDir: `target\v005-phase2-live-smoke-outputref-contract-livepath-2\large-output-ref-smoke\20260617-072905-062`
  - `runtime_output_ref_created_count=1`
  - `large_output_replay_count=0`
  - `raw_output_in_prompt_violation=false`
  - `nodes=9`, `open_leaf_nodes=0`, `spawn_agent_calls=0`
- Observed redundant route pattern: repeated diagnostic/inspect nodes (`node-4`, `node-5`) and blocked validation/final nodes after a completed implementation. The next runtime fix should prevent re-opening a diagnostic inspect after the diagnostic evidence already exists and require validation nodes with successful test evidence before final synthesis.

## 2026-06-17 Phase 2 shell output references

Changed:

- Extended shell command handling to write large aggregated output through the TaskSpace output-reference artifact path.
- The model-facing shell output now uses `OutputReferenceV1` for referenced large output instead of replaying the full raw payload.
- Added output-reference-aware formatting helpers for structured and freeform exec output.
- Added `large-output-ref-smoke` benchmark scenario with a 900-line diagnostic command and hidden oracle.
- Added benchmark instrumentation fallback that counts unique `OutputReferenceV1` artifact refs observed in action-map observability, so metrics do not miss shell-path output refs when no explicit `output_ref.created` timeline event is bound.

Validation:

```text
cargo fmt -p codex-core --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
ok

cargo test -p codex-core tools::handlers::shell::tests::shell_command_pre_tool_use_payload_uses_raw_command --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

cargo test -p codex-core exec_command_tool_output_referenceizes_large_response --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

cargo test -p codex-core experiment_output_ref_records_structured_trace_without_raw_output --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

cargo test -p codex-core read_output_ref --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
passed

cargo test -p codex-tools taskspace_control --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
passed

cargo build -p codex-cli --bin whale --locked --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-outputref-observability-fallback
TaskSpace benchmark harness self-test: PASS
```

Installed smoke binary:

```text
C:\Users\77585\.whale\bin\whale.exe
whale 0.1.0
SHA256 626BB07FB599557258D37B128FDB46EAE348AA8FC618D8CC7C2590A5D07A404C
```

Live smoke evidence:

```text
RunDir: target\v005-phase2-live-smoke-outputref-shellfix-626b\large-output-ref-smoke\20260617-053550-698
scenario: large-output-ref-smoke
left business_success: True
right business_success: True
right large_output_replay_count: 0
right raw_output_in_prompt_violation: False
right observability contains OutputReferenceV1 for shell command call_00_PfOG4fhgwHCfpgqIhpzM5446
right taskspace-control-usage.json after instrumentation fix: runtime_output_ref_created_count = 1
```

Failed / diagnostic smoke:

```text
RunDir: target\v005-phase2-live-smoke-outputref-metricsfix-626b\large-output-ref-smoke\20260617-054241-987
right exec_exit_code: 124
right business_success: False
cause: TaskSpace side timed out before completing the scenario, with repeated blocked implementation nodes.
use: negative reliability sample for routing/tool-budget behavior, not Phase 2 output-reference pass evidence.
```

Operational lesson:

- Phase 2 output-reference validation must exercise `whale exec --json` shell-command flow, not only unified exec unit tests. The first failed smoke showed `runtime_output_ref_created_count = 0` even after unified exec tests passed, because the live path was `command_execution` / shell handling.
- Metrics must use both explicit runtime events and observability-visible `OutputReferenceV1` refs. Otherwise shell-path referenceization can be model-visible but undercounted in `metrics.json`.

## 2026-06-17 Phase 2 single-track inspect budget recovery

Changed:

- Relaxed broad-inspect delegation gates so tool-result budget exhaustion alone no longer forces subagent delegation before implementation.
- Direct `inspect_code_context -> implement_solution` is now allowed when the inspect result is single-track, even if the inspect node hit the main-tool budget.
- Broad protection remains active when an accepted inspect result contains multiple implementation surfaces.
- Refined broad structural evidence detection to require at least two non-test implementation artifact refs, instead of treating any three claims as broad.

Reason:

- The previous live smoke `target\v005-phase2-live-smoke-outputref-metricsfix-626b\large-output-ref-smoke\20260617-054241-987` timed out because the runtime treated a single-file inspect as broad after ordinary read-tool budget pressure.
- The model then repeatedly created/blocked implementation and inspect nodes instead of applying the one-file fix.
- v0.0.5 design says budget is a guardrail, not the primary architecture. The runtime should require delegation only when the task shape actually has independent implementation surfaces.

Validation:

```text
cargo fmt -p codex-core --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
ok

cargo test -p codex-core budget_exhausted_single_track_inspect_can_enter_direct_implementation --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

cargo test -p codex-core broad_accepted_inspect_result_blocks_direct_implementation_without_subagents --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

cargo test -p codex-core inspect --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
16 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-singletrack-budget
TaskSpace benchmark harness self-test: PASS

cargo build -p codex-cli --bin whale --locked --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
passed
```

Installed smoke binary:

```text
C:\Users\77585\.whale\bin\whale.exe
whale 0.1.0
SHA256 52FF6A5882455D2933C9353C1DE6C8C08FA67126B6FC63F62BC6D0E6EFB194D2
```

Live smoke:

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 -Scenario large-output-ref-smoke -Repeats 1 -RunRoot target\v005-phase2-live-smoke-singletrack-budget-52ff -TimeoutSeconds 600 -ValidationTimeoutSeconds 180 -ValidationPretestTimeoutSeconds 60 -ValidationTestTimeoutSeconds 180 -SandboxMode workspace-write -EnableAggregate -AllowNonE2Result

RunDir: target\v005-phase2-live-smoke-singletrack-budget-52ff\large-output-ref-smoke\20260617-060336-086
right business_success: True
right exec_exit_code: 0
right exec_timed_out: False
right tool_call_count: 8
right nodes: 4
right spawn_agent_calls: 0
right open_leaf_nodes: 0
right runtime_output_ref_created_count: 1
right large_output_replay_count: 0
right raw_output_in_prompt_violation: False
```

Remaining:

- The same live smoke still includes a final_synthesis blocker message: "TaskSpace runtime does not accept success criteria updates or waive for final_synthesis completion." Metrics remain business-successful with no open leaf nodes, but final synthesis closure still needs a focused follow-up.

## 2026-06-17 Phase 2 final synthesis criteria tolerance

Changed:

- Added a final-synthesis-only success-criteria completion check.
- Existing `satisfied` / `waived` criteria still require evidence.
- Open `test` / `validator` criteria can be treated as complete for final synthesis only when the map has an accepted successful validation result.
- Open `artifact` / `behavior` criteria can be treated as complete for final synthesis only when the map has both:
  - an accepted implementation result with a successful edit action;
  - an accepted successful validation result.
- Open `performance` and other noncovered criterion kinds still block final synthesis.
- The guard that rejects newly added open criteria during final synthesis remains active.

Reason:

- The previous live smoke `target\v005-phase2-live-smoke-singletrack-budget-52ff\large-output-ref-smoke\20260617-060336-086` completed the business task but left final_synthesis blocked because initial `test` / `behavior` / `artifact` criteria were not precisely rewritten to `satisfied` even though implementation and validation evidence existed.
- The runtime should not require the model to re-state every initial criterion exactly at final time when typed accepted implementation and validation evidence already exists.

Validation:

```text
cargo fmt -p codex-core --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
ok

cargo test -p codex-core finish_final_synthesis --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
5 passed

cargo test -p codex-core final_synthesis_rejects_new_open_success_criterion --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-finalcriterion-2
TaskSpace benchmark harness self-test: PASS

cargo build -p codex-cli --bin whale --locked --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
passed
```

Diagnostic live smoke:

```text
RunDir: target\v005-phase2-live-smoke-finalcriterion-bfc9\large-output-ref-smoke\20260617-062058-221
right business_success: True
right exec_timed_out: False
right large_output_replay_count: 0
right raw_output_in_prompt_violation: False
right nodes: 6
right open_leaf_nodes: 0
right blocked_node_ratio: 0.3333
right runtime_output_ref_created_count: 0
```

Notes:

- This smoke used an earlier build of the final-criteria tolerance and is not proof of the latest runtime patch.
- It exposed two additional routing issues:
  - the model sometimes creates a `smoke_test` node for the required pre-edit diagnostic command, but runtime correctly blocks non-test shell work there;
  - the model redirected the large diagnostic output into `diagnostic_output.txt`, so no output-reference event was produced even though raw replay stayed clean.

Remaining:

- Rebuild/install and rerun a focused smoke after this patch if final_synthesis closure must be proven live.
- Add routing guidance or gate recovery so pre-edit diagnostic commands stay in `inspect_code_context` unless they are actual validation tests.

## 2026-06-17 Phase 2 output-ref trace events

Changed:

- Added TaskSpace runtime trace events for output-reference lifecycle:
  - `output_ref.created` when unified exec stores a large output artifact.
  - `output_ref.slice_read` when `taskspace_control(action=read_output_ref)` returns a bounded slice.
- Output-ref trace events store only `artifact_ref` plus metadata tags such as byte count and slice mode; they do not store raw stdout/stderr bodies.
- Extended benchmark cost instrumentation with:
  - `runtime_output_ref_created_count`
  - `runtime_output_ref_slice_read_count`
- Extended `metrics.json`, side summaries, and aggregate control summaries so output-ref event counts survive suite aggregation.

Validation:

```text
cargo fmt -p codex-core --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
passed

cargo test -p codex-core experiment_output_ref_records_structured_trace_without_raw_output --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

cargo test -p codex-core read_output_ref --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

cargo test -p codex-tools taskspace_control --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
2 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-outputref-events-final-neutral
TaskSpace benchmark harness self-test: PASS
```

Operational note:

- Running several `cargo test` commands in parallel can block on Cargo package/artifact locks. The commands eventually completed, but sequential package-scoped test runs are easier to read when diagnosing compile errors.
- PowerShell profile still emits conda `RequestsDependencyWarning`; using `powershell -NoProfile` keeps benchmark harness output focused.

Remaining Phase 2 work:

- Rebuild/install Whale and run a focused live smoke with large stdout followed by a model turn.
- Phase 2 exit is not proven until that run shows `large_output_replay_count = 0`, nonzero `runtime_output_ref_created_count`, and no provider/tool-output protocol regression.

## 2026-06-17 Phase 2 large exec output first reference slice

Changed:

- Identified the model-visible exec output replay path:
  - `ExecCommandToolOutput::to_response_item` builds the provider tool-output response item.
  - `Session::drain_in_flight` records that response item into conversation history through `record_conversation_items`.
- Added first-pass output referenceization for exec command outputs larger than 50KB.
- Large exec output now keeps the required tool-call output item, but replaces the model-visible body with `OutputReferenceV1` metadata:
  - `sha256`
  - byte count
  - line count
  - policy marker
  - bounded 2KB head and 2KB tail slices
- Small output behavior is unchanged and still uses the existing truncation path.

Validation:

```text
cargo test -p codex-core exec_command_tool_output_referenceizes_large_response --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

cargo test -p codex-core exec_command_tool_output_formats_truncated_response --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-outputref-neutral
TaskSpace benchmark harness self-test: PASS
```

Limits:

- This is not full Phase 2 completion.
- Raw output artifact storage and bounded slice retrieval are still missing.
- The 8KB-50KB summarized tier is still missing.
- Code-mode result serialization still follows the existing `truncated_output` behavior.
- Focused runtime evidence for `large_output_replay_count = 0` still needs an integration smoke with a large stdout followed by a model turn.

## 2026-06-17 Phase 2 OutputReferenceV1 policy module

Changed:

- Moved output reference policy out of `tools/context.rs` into `tools/output_reference.rs`.
- Added `OutputReferenceV1` as an explicit runtime type with:
  - policy
  - sha256
  - bytes
  - lines
  - bounded head/tail slices
- Added the planned threshold policy:
  - inline `<=8KB`
  - summarized `8KB-50KB`
  - referenced `>50KB`
- Updated exec command model-visible output to use the shared policy module.
- Added regression coverage for both medium summarized output and large referenced output.

Validation:

```text
cargo fmt -p codex-core --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
passed

cargo test -p codex-core output_reference --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
3 passed

cargo test -p codex-core exec_command_tool_output_ --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
3 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-outputpolicy-neutral
TaskSpace benchmark harness self-test: PASS
```

Operational note:

- Running multiple `cargo test` commands in parallel caused 120s timeouts while waiting on cargo locks. Re-running the same tests sequentially completed successfully.

Remaining Phase 2 work:

- Persist raw output artifacts and include an artifact ref in `OutputReferenceV1`.
- Add bounded slice retrieval.
- Add output-ref trace events and focused `large_output_replay_count = 0` integration smoke.

## 2026-06-17 Phase 2 exec output artifact store

Changed:

- Added `artifact_ref` to `OutputReferenceV1` model-visible metadata.
- Added raw exec output artifact writing for non-inline outputs.
- Artifact location is derived from the active rollout path:
  - rollout: `<rollout>.jsonl`
  - artifacts: `<rollout-stem>-artifacts/output-refs/<sha256>.stdout`
- Artifact refs use `output-ref://sha256/<sha256>`.
- `UnifiedExecHandler` attaches artifact refs after command execution and before response serialization.
- Artifact write failures are logged as warnings and do not break provider tool-output protocol.

Validation:

```text
cargo fmt -p codex-core --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
passed

cargo test -p codex-core output_reference --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
4 passed

cargo test -p codex-core exec_command_tool_output_ --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
3 passed

cargo test -p codex-core post_tool_use_payload --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
8 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-outputartifact-neutral
TaskSpace benchmark harness self-test: PASS
```

Remaining Phase 2 work:

- Add bounded slice retrieval for `output-ref://sha256/<sha256>`.
- Add output-ref trace events and focused next-turn replay smoke.
- Add sensitive-data scan before exposing summaries/slices.

## 2026-06-17 Phase 2 output-ref slice retrieval

Changed:

- Added bounded output artifact slice retrieval for `output-ref://sha256/<sha256>`.
- Added `taskspace_control(action=read_output_ref)` as the TaskSpace slice action.
- Supported slice modes:
  - `head`
  - `tail`
  - `line_range`
  - `grep`
- Slice output is capped by `max_bytes` and clamped to a 16KB hard maximum.
- Invalid output refs must use the `output-ref://sha256/<64 hex>` form.

Validation:

```text
cargo fmt -p codex-core --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
passed

cargo test -p codex-core output_reference --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
5 passed

cargo test -p codex-core read_output_ref --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-outputslice-neutral
TaskSpace benchmark harness self-test: PASS
```

Remaining Phase 2 work:

- Confirm model-visible tool schema exposes `read_output_ref` arguments in the active tool definition source.
- Add output-ref trace events and focused next-turn replay smoke.
- Add sensitive-data scan before exposing summaries/slices.

## 2026-06-17 Phase 2 read_output_ref tool schema exposure

Changed:

- Updated the model-visible `taskspace_control` tool schema source in `codex-rs/tools/src/taskspace_tool.rs`.
- Added `read_output_ref` to the exposed action enum.
- Added schema fields for:
  - `output_ref`
  - `mode`
  - `start_line`
  - `end_line`
  - `pattern`
  - `max_bytes`
- Updated the tool description so the model is told to retrieve bounded slices from `OutputReferenceV1` artifacts instead of asking for large raw stdout/stderr replay.
- Extended schema tests so the action and fields cannot silently disappear.

Validation:

```text
cargo test -p codex-tools taskspace_control --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
2 passed

cargo test -p codex-core read_output_ref --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

cargo fmt -p codex-tools -p codex-core --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-outputschema-neutral
TaskSpace benchmark harness self-test: PASS
```

Operational note:

- `cargo fmt --manifest-path third_party\codex-cli\codex-rs\Cargo.toml` fails with "Failed to find targets" on this workspace manifest. Use package-scoped formatting such as `cargo fmt -p codex-tools -p codex-core --manifest-path third_party\codex-cli\codex-rs\Cargo.toml`.

Remaining Phase 2 work:

- Add output-ref trace events and focused next-turn replay smoke.
- Add sensitive-data scan before exposing summaries/slices.

## 2026-06-17 Phase 2 replay summary evidence fields

Changed:

- Extended benchmark replay summary with explicit output-ref evidence fields:
  - `output_reference_count`
  - `output_slice_count`
  - `truncation_marker_count`
  - `raw_large_marker_count`
- Kept `large_output_replay_count` for compatibility, but now it is backed by counted marker classes instead of a single opaque heuristic.
- `raw_output_in_prompt_violation` is now true when a replay marker class is detected.
- Harness now verifies that `OutputReferenceV1` / `OutputSliceV1` artifacts are counted without being treated as raw replay.
- Harness now verifies that a focused raw middle marker repeated at large-output scale sets replay count and violation status.

Validation:

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-replaysummary-neutral
TaskSpace benchmark harness self-test: PASS
```

Limit:

- This is still a benchmark artifact heuristic, not a full provider request reconstruction. Full Phase 2 exit still needs a focused runtime smoke with a large stdout followed by a model turn.

## 2026-06-17 Phase 1 state_commit recovery path

Observed:

- Latest installed live smoke after output-contract support produced `runtime_state_commit_count=1`, but cost regressed:
  - direct input+output ratio: 8.7965
  - walltime ratio: 2.7188
  - right-side input tokens: 862932
  - nodes: 4
  - runtime events: 85
- The task solved, but stderr showed avoidable TaskSpace recovery failures:
  - early `state_commit` parse failure: missing `commit_id`
  - validation node completion still instructed the model to use legacy `record_success_criteria`
  - final_synthesis was created and repeatedly blocked because the validation result / success criterion closure was not batched cleanly.

Changed:

- Made `state_commit.commit_id` optional at the handler boundary.
- When omitted, the handler derives an `auto-*` id from the submitted arguments. Caller-provided ids still drive explicit idempotency.
- Updated validation-node completion gate text to prefer one `state_commit` with:
  - `result_validities`
  - `success_criteria` status `satisfied`
  - `finished_nodes`
- Updated `state_commit_v1.schema.json`, the implementation plan, and the engineering contract so the documented schema matches the runtime recovery behavior.

Validation:

```text
cargo fmt -p codex-core
ok

Get-Content -Raw docs\v0.0.5\schemas\state_commit_v1.schema.json | ConvertFrom-Json | Out-Null
schema json ok

cargo test -p codex-core state_commit --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
5 passed

cargo test -p codex-core finish_smoke_node_requires_successful_validation_evidence --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-statecommit-recovery-9b7c
TaskSpace benchmark harness self-test: PASS
```

Reason:

- Phase 1 should reduce protocol-turn count. Treating a missing `commit_id` as a hard parse failure and pointing validation recovery at legacy single-record actions both work against that goal.
- This does not remove explicit idempotency. It makes the common model-error recovery path cheap enough to keep the agent on the intended transactional protocol.

## 2026-06-17 Phase 1 narrow inspect spawn guard

Observed:

- Installed live smoke with Whale hash `7F9732372CDAEFA58216FF405E2939B35FB859865DBEE6DE2CD64431FFA5AB4C` solved the task but regressed badly:
  - RunDir: `target\v005-phase1-live-smoke-statecommit-recovery-7f97\single-file-fast-fix\20260617-034714-812`
  - direct input+output ratio: 20.3256
  - walltime ratio: 5.5428
  - nodes: 8
  - spawn_agent_calls: 2
  - runtime_state_commit_count: 2
- The missing-`commit_id` failure disappeared, so the previous recovery fix worked for that symptom.
- The next root cause was a model escape around the spawn gate:
  - runtime initially blocked `spawn_agent` because no subagent plan existed;
  - model then created extra inspect nodes and subagent plans;
  - once two ready inspect nodes existed, `validate_spawn_parallelism` allowed explorer spawn even though the task had already completed a narrow main-agent inspect pass.

Changed:

- Added `has_completed_narrow_inspect` as shared runtime graph-state detection.
- `record_subagent_plan_for_main` now rejects inspect subagent plans after the main agent has already completed a narrow inspect pass.
- Kept legitimate pre-planned subagent flow intact: the existing `subagent_plan_links_lease_child_result_and_snapshot` path still passes.

Validation:

```text
cargo fmt -p codex-core
ok

cargo test -p codex-core subagent_plan_rejects_followup_after_narrow_main_inspect --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

cargo test -p codex-core spawn_assignment_requires_recorded_subagent_plan --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

cargo test -p codex-core subagent_plan_links_lease_child_result_and_snapshot --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

cargo test -p codex-core state_commit --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
5 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-narrowguard-5c6d
TaskSpace benchmark harness self-test: PASS
```

Operational note:

- Do not put treatment labels such as `subagent` in benchmark `RunRoot`; the harness correctly failed a first attempt with `left cwd contains treatment label` / `right cwd contains treatment label`.

Remaining Phase 1 work:

- Rebuild/install Whale and rerun live smoke to verify the narrow-inspect guard removes the late subagent expansion.
- If final_synthesis still loops, the next likely fix is state_commit payload tolerance for `next_best_action.reason` and duplicate evidence-ref aliases.

## 2026-06-17 Phase 1 final_synthesis criterion guard

Observed:

- Installed live smoke with Whale hash `0E7254679E7130DAE50B5C6133158E099A00525537AA51E61A53541134B85AB8` showed the narrow-inspect guard worked:
  - RunDir: `target\v005-phase1-live-smoke-narrowguard-0e72\single-file-fast-fix\20260617-040427-282`
  - scenario warnings: none
  - spawn_agent_calls: 0
  - subagent_results: 0
  - nodes: 4
  - runtime events: 79
  - walltime ratio: 3.0415
  - direct input+output ratio: 9.2833
- Cost is still above the Phase 1/partial gate, but the previous subagent expansion root cause is closed for this smoke shape.
- Remaining final-stage failure:
  - smoke_test result was accepted;
  - model entered final_synthesis;
  - model then added/updated success criteria during final_synthesis, including a late `sc-2`, causing the final gate to keep reporting incomplete success criteria.

Changed:

- Added `validate_success_criteria_update_allowed`.
- While the current main node is `final_synthesis`, `record_success_criteria` now rejects new open/questioned criteria.
- Existing criteria can still be updated, and new criteria are allowed only if they are already `satisfied` or `waived` with evidence.
- Because `state_commit.success_criteria` reuses `record_success_criteria_for_main`, the same guard applies to transactional commits.

Validation:

```text
cargo fmt -p codex-core
ok

cargo test -p codex-core final_synthesis_rejects_new_open_success_criterion --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

cargo test -p codex-core finish_smoke_node_requires_successful_validation_evidence --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
1 passed

cargo test -p codex-core state_commit --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
5 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-finalgate-8d2e
TaskSpace benchmark harness self-test: PASS
```

Remaining Phase 1 work:

- Rebuild/install Whale and rerun live smoke to verify final_synthesis no longer loops on late open success criteria.
- If cost is still high with nodes=3/4 and spawn=0, the next dependency is likely Phase 2 prompt/context projection rather than more Phase 1 lifecycle gates.

## 2026-06-17 Phase 1 finalgate live smoke

Installed:

- Whale hash: `2F5F2AE947940FBED2375A703677AB220A9979E4E934FD189E4E9616EAFD0A4E`
- Commit under test: `07ade7df4`

Validation:

```text
cargo build -p codex-cli --bin whale --locked --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
Finished dev profile

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 -Scenario single-file-fast-fix -Repeats 1 -RunRoot target\v005-phase1-live-smoke-finalgate-2f5f -TimeoutSeconds 600 -ValidationTimeoutSeconds 180 -ValidationPretestTimeoutSeconds 60 -ValidationTestTimeoutSeconds 180 -SandboxMode workspace-write -EnableAggregate -AllowNonE2Result
RunDir: target\v005-phase1-live-smoke-finalgate-2f5f\single-file-fast-fix\20260617-041625-523
```

Result:

- TaskSpace business success: True
- public validation exit code: 0
- hidden oracle exit code: 0
- scenario warnings: none
- nodes: 4
- spawn_agent_calls: 0
- subagent_results: 0
- open_leaf_nodes: 0
- runtime_state_commit_count: 3
- taskspace runtime events: 86
- direct input+output ratio: 6.4041
- walltime ratio: 2.4078
- model_request_count_ratio: 1

Conclusion:

- The final_synthesis late-criterion guard did not prevent successful closure; final_synthesis completed in the live smoke.
- Phase 1 structural goals are now mostly true for this smoke shape: no late subagent expansion, no open leaf, final node closes, and runtime state_commit events are present.
- Phase 1 cost remains above both release and partial token gates. With spawn eliminated and request-count ratio at 1, the remaining dominant issue is prompt/context size, which belongs to Phase 2 output referenceization and context projection rather than more lifecycle gates.

Remaining:

- Treat Phase 1 as functionally ready for Phase 2 dependency work, but not release-ready by itself.
- Start Phase 2 with prompt/token source attribution and output referenceization.

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

## 2026-06-17 Phase 1 state_commit output contracts

Changed:

- Added `output_contracts` as a transactional `state_commit` section.
- `state_commit` can now satisfy the full problem preflight trio in one commit:
  - `success_criteria`
  - `output_contracts`
  - `fact_sources`
- Updated runtime prompt text, BaseMap guidance, JSON schema, implementation plan, correction contract, and the state commit example to include `output_contracts`.
- Added regression coverage that a single state_commit can satisfy `validate_cognitive_preflight`.

Validation:

```text
cargo test -p codex-core state_commit --manifest-path third_party\codex-cli\codex-rs\Cargo.toml
4 passed

Get-Content -Raw docs\v0.0.5\schemas\state_commit_v1.schema.json | ConvertFrom-Json | Out-Null
schema json ok

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\paired-bench-selftest-v005-outputcontracts-1d9a
TaskSpace benchmark harness self-test: PASS
```

Reason:

- The previous preflight remediation pointed at `state_commit`, but `state_commit` did not support output contracts. That made the recommended path incomplete and pushed the model back to legacy `record_output_contract`.

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

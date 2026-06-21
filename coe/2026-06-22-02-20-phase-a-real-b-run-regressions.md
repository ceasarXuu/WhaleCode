# Problem P-001: Phase A active budget fails real B acceptance

Status: investigating

Symptoms:
- A real B-tier `single-file-fast-fix` run completed as a harness run but TaskSpace failed the business task.
- TaskSpace spent fewer tool calls and less wall time than standard mode, but stopped before implementation.
- Runtime rollout contains active budget and budget-quality trace events, while exported active budget and budget summaries are empty or missing runtime.

Expected behavior:
- A single-file fast fix should complete under TaskSpace with active budget constraints.
- A prompt/router recommendation of `thin` should select the `thin` active budget, or route selection failure should be explicit.
- Budget pressure should force convergence into implementation/validation, not fail the turn after the model has identified the fix.
- Runtime budget events in `rollout.jsonl` should be extracted into active budget and budget summary artifacts for real runs.

Actual behavior:
- `target/phase-a-benefit-B/single-file-fast-fix/20260622-020940-812/pair-001/pair-report.md` reports standard solved and TaskSpace wrong.
- TaskSpace validation failed with `calculate_tax(19.99, "CA") == 1.4`, showing `src/tax_calc.py` was not fixed.
- `rollout.jsonl` shows `provider_node_request_budget_exhausted` at node request count `3/3` after the assistant identified the `round(..., 1)` bug.
- `rollout.jsonl` shows `active_budget route_mode:default_compact` despite the benchmark prompt carrying `recommended_mode: thin`.
- `active-budget-events.jsonl` is zero bytes and `spawn-node-budget-summary.json` reports `source_status: missing_runtime` even though runtime trace events exist in rollout.

Impact:
- Phase A budget enforcement can reduce cost by ending work prematurely, producing false-positive cost savings with failed business outcome.
- B-tier acceptance cannot pass until quality-preserving budget recovery and real artifact extraction are fixed.

Fix criteria:
- A fresh B-tier `single-file-fast-fix` run using the current built `whale.exe` completes with TaskSpace `business_success: True`.
- The run records active budget evidence in exported artifacts, not only embedded rollout snapshots.
- The run does not spawn subagents for the single-file task.
- If `thin` is supplied by the routing prompt, the active budget route is `thin` or an explicit route-selection failure is recorded.
- Budget-induced quality impact is visible in exported summaries when a budget block occurs.

## Hypothesis H-001: Node provider budget hard-stop blocks implementation after actionable inspect finding

Status: confirmed

Rationale:
- The model identified the exact tax rounding bug but the next provider request failed before edit/tool execution.

Predictions:
- The rollout should show an actionable assistant message naming `round(..., 1)` before the budget block.
- The next provider lifecycle event should be `status:blocked` with `budget_transition_reason:provider_node_request_budget_exhausted`.
- Public validation should still show the original rounding failure because no implementation edit occurred.

Diagnostic evidence plan:
- Inspect the B run rollout and validation artifacts for the actionable bug message, provider budget block, and unchanged validation failure.

## Evidence E-001: B run completed but TaskSpace failed business validation

Type: reproduction

Source:
- Command: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 -Scenario single-file-fast-fix -Repeats 1 -RunRoot target\phase-a-benefit-B -WhaleBin D:\BuildCache\whalecode\cargo-target\debug\whale.exe -Model deepseek-v4-flash -TimeoutSeconds 420 -ValidationTimeoutSeconds 180 -ValidationPretestTimeoutSeconds 60 -ValidationTestTimeoutSeconds 180 -SandboxMode full-auto -ConfigOverride 'model_reasoning_effort="low"' -AllowNonE2Result -AllowStaleWhaleBin`
- `target/phase-a-benefit-B/single-file-fast-fix/20260622-020940-812/sample-status.json`
- `target/phase-a-benefit-B/single-file-fast-fix/20260622-020940-812/pair-001/pair-report.md`

Observation:
- The harness exited `0`, `phase=completed`, and `run_validity=valid`.
- Pair report shows `outcome_standard: solved`, `outcome_taskspace: wrong`, and `right / taskspace business_success: False`.

Supports:
- Problem P-001 actual behavior.

## Evidence E-002: TaskSpace validation retained the original tax rounding failure

Type: reproduction

Source:
- `target/phase-a-benefit-B/single-file-fast-fix/20260622-020940-812/pair-001/right/artifacts/validation.stdout.log`

Observation:
- Public validation failed with `assert calculate_tax(19.99, "CA") == 1.45`, actual `1.4`.
- Hidden oracle also failed on `calculate_tax`.

Supports:
- H-001 prediction that no successful implementation edit occurred before validation.

## Evidence E-003: Runtime blocked provider request after actionable bug identification

Type: trace

Source:
- `target/phase-a-benefit-B/single-file-fast-fix/20260622-020940-812/pair-001/right/artifacts/rollout.jsonl`

Observation:
- The assistant message says the bug is `calculate_tax()` using `round(..., 1)` while README/tests require cents.
- The subsequent provider event has `kind:"provider_request_budget"`, `status:blocked`, `node_request_count:3`, `max_model_requests_per_node:3`, `budget_transition_reason:provider_node_request_budget_exhausted`, and `budget_response_action_taken:true`.
- The following quality event has `budget_action:hard_stop` and `final_classification:blocked_by_budget`.

Supports:
- H-001 is confirmed.

## Hypothesis H-002: Thin route constraints are present as prompt text but not consumed by runtime budget activation

Status: confirmed

Rationale:
- The benchmark prompt includes `recommended_mode: thin`, but the runtime activation event records `default_compact`.

Predictions:
- User prompt in rollout should contain `recommended_mode: thin`.
- The first `active_budget` runtime trace should record `route_mode:default_compact` rather than `thin`.

Diagnostic evidence plan:
- Inspect rollout user prompt and active budget trace.

## Evidence E-004: Prompt recommends thin while runtime activates default_compact

Type: trace

Source:
- `target/phase-a-benefit-B/single-file-fast-fix/20260622-020940-812/pair-001/right/artifacts/rollout.jsonl`

Observation:
- User prompt contains `TaskShapeRouterV1 active profile constraints` and `recommended_mode: thin`.
- The first runtime `active_budget` trace records `route_mode:default_compact`, `max_rollout_model_requests:10`, and `max_spawn_agent_calls:2`.

Supports:
- H-002 is confirmed.

## Hypothesis H-003: Cost instrumentation does not parse real rollout taskspace trace events

Status: confirmed

Rationale:
- Runtime events exist in real rollout, but extracted budget files are empty or report missing runtime.

Predictions:
- Real rollout contains `taskspace_trace_event_recorded` events for `active_budget`, `spawn_node_budget`, `provider_request_budget`, `projection_budget`, and `budget_quality_impact`.
- Extracted `active-budget-events.jsonl` should be empty or missing those events.
- Summary artifacts should report `missing_runtime` or zero budget events.

Diagnostic evidence plan:
- Compare file lengths and selected contents for rollout and exported budget artifacts from the same B run.

## Evidence E-005: Real budget traces are present but exported active-budget artifacts are empty

Type: artifact

Source:
- `target/phase-a-benefit-B/single-file-fast-fix/20260622-020940-812/pair-001/right/artifacts/rollout.jsonl`
- `target/phase-a-benefit-B/single-file-fast-fix/20260622-020940-812/pair-001/right/artifacts/active-budget-events.jsonl`
- `target/phase-a-benefit-B/single-file-fast-fix/20260622-020940-812/pair-001/right/artifacts/spawn-node-budget-summary.json`
- `target/phase-a-benefit-B/single-file-fast-fix/20260622-020940-812/pair-001/right/artifacts/budget_induced_quality_impact_summary.json`

Observation:
- `rollout.jsonl` includes `active_budget`, `spawn_node_budget`, `provider_request_budget`, `projection_budget`, and `budget_quality_impact` runtime trace events.
- `active-budget-events.jsonl` has length `0`.
- `spawn-node-budget-summary.json` reports `source_status: missing_runtime`.
- `budget_induced_quality_impact_summary.json` reports `budget_event_count: 0`, despite rollout containing `final_classification:blocked_by_budget`.

Supports:
- H-003 is confirmed.

## Hypothesis H-004: Thin route's four-request budget is under-calibrated for the real minimum single-file fix path

Status: confirmed

Rationale:
- The first repair allowed bounded node recovery and structured thin routing, but the rerun still reached hard budget before a file edit could execute.
- The real path needs at least task setup, inspection, inspect-to-implement convergence, implementation, and validation/finalization provider turns.

Predictions:
- A rerun should activate `route_mode:thin`.
- The run should no longer hard-block at the inspect node limit.
- If the four-request rollout budget is too low, the fourth request should be the first implement request and be at hard-stop pressure, causing no actual edit or no usable follow-up.

## Evidence E-006: B rerun activates thin and avoids spawn, but still fails business validation

Type: reproduction

Source:
- `target/phase-a-benefit-B-rerun/single-file-fast-fix/20260622-023744-831/pair-001/pair-report.md`
- `target/phase-a-benefit-B-rerun/single-file-fast-fix/20260622-023744-831/pair-001/right/artifacts/validation.stdout.log`
- `target/phase-a-benefit-B-rerun/single-file-fast-fix/20260622-023744-831/pair-001/right/artifacts/budget_induced_quality_impact_summary.json`

Observation:
- Pair report shows `outcome_taskspace: wrong`, `taskspace_tool_call_ratio: 0.83`, `taskspace_wall_time_ratio: 0.67`, `taskspace_nodes: 2`, and `taskspace_spawn_agent_calls: 0`.
- Validation still fails with `calculate_tax(19.99, "CA") == 1.4`.
- Budget summary records `active_budget_source: runtime`, `route_mode: thin`, `max_rollout_model_requests: 4`, and `max_model_requests_per_node: 2`.

Supports:
- H-004: structured routing and no-spawn constraints work, but the thin request ceiling is below the real minimum success path.

## Evidence E-007: The fourth request is the first implement request and already at hard stop

Type: trace

Source:
- `target/phase-a-benefit-B-rerun/single-file-fast-fix/20260622-023744-831/pair-001/right/artifacts/rollout.jsonl`

Observation:
- Request 3 contains an assistant message identifying the exact `round(..., 1)` bug and saying it will verify before fixing.
- Runtime then emits `forced_inspect_transition` with trigger `budget_pressure_follow_up_intent`, creating `node-2` as `implement_solution`.
- Request 4 records `request_count_before:3`, `request_count_after:4`, `runtime_budget_state:hard_stopped`, and `request_phase:budget_recovery`.
- The final actionability event previews pseudo `taskspace_control(state_commit)` text and no `src/tax_calc.py` edit appears in `git-diff.patch`.

Supports:
- H-004 is confirmed.

## Hypothesis H-005: Exact payload replacement scan reports false negatives for valid active projections

Status: confirmed

Rationale:
- The scan requires protected-item markers and treats any `taskspace_control` text as legacy history, but current active projections and budget guidance may legitimately mention `taskspace_control`, and simple thin tasks may have no protected items.

Predictions:
- Rerun payload scans should show `active_projection_present:true` but `replacement_confirmed:false`.
- The false result should be explainable by `legacy_taskspace_history_present:true` and `protected_items_present:false`, not by absence of the active projection.

## Evidence E-008: Payload scan marks active projection present but replacement unconfirmed

Type: artifact

Source:
- `target/phase-a-benefit-B-rerun/single-file-fast-fix/20260622-023744-831/pair-001/right/artifacts/active-context-replacement-report.json`
- `target/phase-a-benefit-B-rerun/single-file-fast-fix/20260622-023744-831/pair-001/right/artifacts/exact-payload-scan-events.jsonl`

Observation:
- `active-context-replacement-report.json` has `active_projection_present` evidence in scan events but reports `replacement_confirmed:false`, `legacy_taskspace_history_present:true`, and `protected_items_present:false`.
- The provider payloads still include current active projection markers. The failure is therefore diagnostic misclassification, not proof that the active projection was absent.

Supports:
- H-005 is confirmed.

## Hypothesis H-006: Implement nodes can fail after a correct edit because test follow-up intent is not converted into a validation node

Status: confirmed

Rationale:
- The runtime can force inspect convergence into implementation under provider budget pressure, but implementation convergence after a successful edit was still delegated to the next model turn.
- If the model says it will run tests and then emits no tool call, the implement node can consume its remaining node request budget and fail even though the working tree already contains the correct fix.

Predictions:
- A failed B run should show `git-diff.patch` with the correct application fix and public/hidden validation passing after the agent exits.
- The agent-side `exec_exit_code` should still be non-zero because the active implement node exhausts its provider request budget before binding a smoke-test node.

## Evidence E-009: B rerun8 has the correct fix and passing validators but TaskSpace exits 1

Type: reproduction

Source:
- `target/phase-a-benefit-B-rerun8/single-file-fast-fix/20260622-040659-055/pair-001/pair-report.md`
- `target/phase-a-benefit-B-rerun8/single-file-fast-fix/20260622-040659-055/pair-001/right/artifacts/git-diff.patch`
- `target/phase-a-benefit-B-rerun8/single-file-fast-fix/20260622-040659-055/pair-001/right/artifacts/validation.stdout.log`
- `target/phase-a-benefit-B-rerun8/single-file-fast-fix/20260622-040659-055/pair-001/right/artifacts/metrics.json`

Observation:
- `git-diff.patch` changes `round(..., 1)` to `round(..., 2)` in `src/tax_calc.py`.
- `validation.stdout.log` reports `3 passed in 0.03s`.
- `metrics.json` records `public_validation_exit_code:0` and `hidden_oracle_exit_code:0`, but `business_success:false` because `exec_exit_code:1`.

Supports:
- H-006 is confirmed: the business fix succeeded, while TaskSpace lifecycle convergence failed.

## Evidence E-010: The implement node hits node request exhaustion after saying it will run tests

Type: trace

Source:
- `target/phase-a-benefit-B-rerun8/single-file-fast-fix/20260622-040659-055/pair-001/right/artifacts/whale-exec.jsonl`

Observation:
- Request 5 applies the correct edit on `src/tax_calc.py`.
- Request 6 includes assistant text equivalent to "Now let me run the tests" and emits `TaskSpaceNoActionRecoveryV1`.
- Request 7 then fails with `TaskSpace blocked this provider request because the active node provider request budget is exhausted (3/3). Enter budget_recovery, final_synthesis, or bind a different node before requesting another model turn.`

Supports:
- H-006 explains why the B run failed despite correct edit and passing validators.

## Fix F-006: Force implementation convergence into focused validation after a successful edit

Implementation:
- Added `force_finish_implement_for_provider_budget` to complete an `implement_solution` node under provider pressure only when the node has a successful edit result.
- The forced transition binds a `smoke_test` node, records `forced_implement_transition`, and accepts the bridge result evidence package so validation tools are not blocked as unreviewed.
- Extended the turn loop so budget-pressure follow-up intent can force either `inspect_code_context -> implement_solution` or `implement_solution -> smoke_test`.

Verification:
- `cargo test -p codex-core provider_budget_follow_up_force_finishes --locked`
- `cargo test -p codex-core budget_pressure --locked`
- `cargo fmt --all -- --check`
- `cargo build -p codex-cli --bin whale --locked`

## Hypothesis H-007: DeepSeek implement turns can produce repeated reasoning-only responses after forced inspect transition

Status: confirmed

Rationale:
- The forced inspect transition correctly binds an `implement_solution` node and injects recovery guidance, but the next model responses can still contain no visible assistant text and no tool call.
- Generic recovery prompts are insufficient when the model repeatedly emits hidden/actionable reasoning without an actual `apply_patch` call.

Predictions:
- B reruns should show `TaskSpaceForcedInspectTransitionV1` followed by one or more `TaskSpaceNoActionRecoveryV1` events on `implement_solution`.
- The final failure should occur without any `apply_patch` event and without a `src/tax_calc.py` diff.

## Evidence E-011: B rerun12 spends both implement recovery attempts without an edit

Type: reproduction

Source:
- `target/phase-a-benefit-B-rerun12/single-file-fast-fix/20260622-043808-578/pair-001/pair-report.md`
- `target/phase-a-benefit-B-rerun12/single-file-fast-fix/20260622-043808-578/pair-001/right/artifacts/whale-exec.jsonl`
- `target/phase-a-benefit-B-rerun12/single-file-fast-fix/20260622-043808-578/pair-001/right/artifacts/git-diff.patch`

Observation:
- Pair report shows `outcome_taskspace: wrong`, `business_success:false`, `exec_exit_code:1`, `public_validation_exit_code:1`, and `hidden_oracle_exit_code:1`.
- The rollout records `TaskSpaceForcedInspectTransitionV1`, then `TaskSpaceNoActionRecoveryV1` recovery attempt `1/2` and `2/2`.
- The turn finally stops with `too many non-action assistant messages`, and `git-diff.patch` is empty.

Supports:
- H-007 is confirmed. The remaining blocker is not budget accounting or validation routing; it is the model/runtime contract for repeated hidden-action/no-tool responses in implementation nodes.

## Fix F-007: Earlier implement recovery and larger no-action allowance

Implementation:
- Implement budget pressure now starts at 50% instead of 75% in the turn-level follow-up detector.
- `implement_solution` no-action recovery allows two attempts, while other nodes remain capped at one.
- Recovery copy now explicitly instructs the model to call `apply_patch`.

Verification:
- `cargo test -p codex-core budget_pressure --locked`
- `cargo fmt --all -- --check`
- `cargo build -p codex-cli --bin whale --locked`
- B rerun12 verifies the new recovery telemetry, but B still fails because no `apply_patch` is emitted after both recovery attempts.

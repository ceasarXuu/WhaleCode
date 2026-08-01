# Phase H. Targeted diagnostic and formal E3 readiness

> Split from `22-v005-completion-engineering-playbook.md` to keep each execution context small and phase-cohesive.
>
> Canonical sequence: read `00-overview-and-gates.md` first, then only the phase file you are implementing.


## H.1 Diagnostic sequence

Only after Phases A-G are green:

Current status as of 2026-06-26: B-tier diagnostic executed; targeted E3 remains blocked.

Do not run targeted diagnostic or formal E3 yet. The old Phase E/G blockers are
closed, but the current blockers are:

```text
post-ABI B-tier smoke business/cache passed, but release/readiness gates did not pass
active context replacement proof failed on real B-tier payload
open_leaf_nodes = 1
taskspace_wall_time_ratio = 3.07
runtime timing is missing provider request duration attribution
current code-complete marker and explicit user approval marker are still missing
```

```powershell
# Provider and binary preflight.
cargo build -p codex-cli --bin whale --locked

'Reply exactly ok.' | D:\BuildCache\whalecode\cargo-target\debug\whale.exe exec `
  --json `
  -m deepseek-v4-flash `
  -c 'model_reasoning_effort="low"' `
  -s read-only `
  --skip-git-repo-check `
  --ephemeral `
  -

# B-tier smoke must pass business and cache gates before C/E3-style diagnostics.
pwsh -File scripts/taskspace-benchmark/run-taskspace-benchmark.ps1 `
  -Scenario single-file-fast-fix `
  -Repeats 1 `
  -RunRoot target\phase-a-benefit-B `
  -TimeoutSeconds 900 `
  -ValidationTimeoutSeconds 180 `
  -ValidationPretestTimeoutSeconds 60 `
  -ValidationTestTimeoutSeconds 180 `
  -SandboxMode workspace-write `
  -EnableAggregate `
  -AllowNonE2Result `
  -WhaleBin D:\BuildCache\whalecode\cargo-target\debug\whale.exe
```

```powershell
# Non-agent gates first. This command is blocked until Phase G implements the builder.
pwsh -File scripts/taskspace-benchmark/build-v005-non-agent-gates.ps1 `
  -RunRoot <run-root> `
  -TaskListHash <formal-task-list-hash> `
  -ProfileHash <profile-hash> `
  -SourceVersion <source-version>

# Then one targeted diagnostic, not release proof.
pwsh -File scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1 `
  -SampleSet terminal-bench_E3-P0_1_1 `
  -EvidenceTarget diagnostic-only `
  -Profile taskspace-v005-active
```

The targeted diagnostic must show:

```text
TaskSpace B-tier business_success=true before escalation
TaskSpace public and hidden validation exit codes are 0
TaskSpace provider-cache-trace-summary.json passes the hard cache gate
request count is no longer 30x-190x Standard
spawn/node profile trace is advisory-only and blocked_budget_event_count = 0
active payload scan passes
request phase summary has meaningful phase distribution
budget quality impact summary has no silent validation skip
open_leaf_nodes = 0 or graph hygiene follow-up is recorded before E3
agent_walltime_ratio <= 2.5x or runtime bottleneck report exists and formal E3 remains blocked
```

## H.1.1 2026-06-26 B-tier diagnostic result

Command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 `
  -Scenario single-file-fast-fix `
  -Repeats 1 `
  -RunRoot target\phase-h-btier-smoke `
  -TimeoutSeconds 900 `
  -ValidationTimeoutSeconds 180 `
  -ValidationPretestTimeoutSeconds 60 `
  -ValidationTestTimeoutSeconds 180 `
  -SandboxMode workspace-write `
  -EnableAggregate `
  -AllowNonE2Result `
  -WhaleBin D:\BuildCache\whalecode\cargo-target\debug\whale.exe
```

Run:

```text
target/phase-h-btier-smoke/single-file-fast-fix/20260626-221906-721
```

Passed evidence:

```text
pair outcome = both_success
TaskSpace business_success = true
TaskSpace public_validation_exit_code = 0
TaskSpace hidden_oracle_exit_code = 0
provider-cache-trace-summary:
  request_2_plus_hit_rate = 0.989783
  trace_coverage = 1
  cache_usage_missing_count = 0
  native_tools_schema_hot_path_count = 0
  tool_free_action_contract_count = 9
request-phase-summary:
  provider_request_hook_coverage = 100
  request_phase_attribution_coverage = 100
  phase_diversity_gate_pass = true
budget-quality-impact:
  blocked_by_budget_samples_count = 0
  budget_induced_validation_skip_count = 0
spawn-node-budget:
  status = pass
  over_budget_enforcement_status = advisory_only
  blocked_budget_event_count = 0
  unreviewed_subagent_result_count = 0
```

Blocking evidence:

```text
active-context-replacement-report:
  exact_payload_scan_passed = false
  replacement_confirmed = false
  legacy_taskspace_history_present = true
  raw_taskspace_control_history_tokens = 917
  protected_items_present = false
graph-health / metrics:
  open_leaf_nodes = 1
  nodes = 3
  spawn_agent_calls = 0
pair report:
  taskspace_wall_time_ratio = 3.07
sample-timing:
  bottleneck_classification = agent_bound
  runtime_optimization_status = blocked
  wait_attribution_status = missing
  wait_attribution_missing_fields includes model_request_duration_ms
cost-diagnostics:
  status = FAIL
  root_cause = fixed_taskspace_provider_context_surface_too_large
  provider_direct_input_output_ratio = 12.9726
  rollout_trace_model_request_count_ratio = 10
  projection_token_share_of_taskspace_input = 0.0022
  runtime_state_commit_per_rollout_request = 0
```

Conclusion:

```text
Do not run terminal-bench_E3-P0_1_1 or formal E3 yet.
Phase H benefit is diagnostic separation: business/cache/fanout are not the blocker;
the blocker is real provider-visible TaskSpace context replacement plus graph closeout/walltime.
```

## H.2 Formal E3 start gate

Only after targeted diagnostic is acceptable:

```powershell
pwsh -File scripts/taskspace-benchmark/lib/e3-start-gate.ps1 `
  -ExpectedSampleSetId terminal-bench_E3-P0_3_5 `
  -V005NonAgentGatesPath <run-root>\v005-non-agent-gates.json `
  -V005CodeCompletePath <run-root>\v005-code-complete.json `
  -V005UserApprovalPath <run-root>\v005-user-approval.json
```

Formal E3 may run only if:

```text
start-gate/gate-decision.json full_e3_allowed = true
start-gate/gate-decision.json v005_markers_passed = true
start-gate/gate-decision.json calibration_gate_passed = true
```

## H.3 Formal E3 command category

The formal command must produce:

```text
run-status.json evidence_target = E3
run-status.json sample_set_id = terminal-bench_E3-P0_3_5
run-status.json repeats_per_sample >= 5
pair_completed reported_evidence_level = E3 for every counted pair
formal pair ledger = exactly 3 samples x 5 repeats
```

`terminal-bench_E3-P0_1_1`, `_3_1`, and `_3_2` must never produce `release_pass`.

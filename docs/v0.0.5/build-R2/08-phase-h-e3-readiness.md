# Phase H. Targeted diagnostic and formal E3 readiness

> Split from `22-v005-completion-engineering-playbook.md` to keep each execution context small and phase-cohesive.
>
> Canonical sequence: read `00-overview-and-gates.md` first, then only the phase file you are implementing.


## H.1 Diagnostic sequence

Only after Phases A-G are green:

```powershell
# Non-agent gates first.
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
request count is no longer 30x-190x Standard
spawn count stays within route budget
active payload scan passes
request phase summary has meaningful phase distribution
budget quality impact summary has no silent validation skip
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

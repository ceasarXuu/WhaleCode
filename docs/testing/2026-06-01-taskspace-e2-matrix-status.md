# TaskSpace E2 Matrix Status

This note records the current E2 evidence boundary for TaskSpace benchmarks.

## Current Claim

WhaleCode has one successful TaskSpace E2 evidence matrix run across L1, L2, and L3 constructed scenarios.

This means:

- The paired benchmark harness can compare `standard` and `taskspace` modes under the same prompt, fixture, model, permissions, timeout, and oracle.
- Each scenario produced three valid utility pairs.
- The hidden oracle and public validation passed on both sides.
- Prompt guard did not detect internal TaskSpace concept leakage.
- The report can now distinguish evidence level from utility outcome.
- The clean utility matrix is not yet green because L1 still exposes TaskSpace overhead warnings.

This does not mean:

- TaskSpace has proven real-world product utility.
- TaskSpace is better for every task.
- E3 has been reached.
- Cost overhead is solved.

## Scenario Matrix

| scenario | level | purpose |
|---|---|---|
| `single-file-fast-fix` | L1 | Verify TaskSpace does not break or grossly distort a simple single-file fix. |
| `multi-file-order-pipeline` | L2 | Verify multi-file product-rule repair with one intentionally wrong public test expectation. |
| `subscription-billing-repair` | L3 | Verify a broader repair across parsing, plan pricing, tax, billing, and wrong test expectation. |

## Authoritative Run

Command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-e2-matrix.ps1 -Repeats 3 -Model deepseek-v4-flash -TimeoutSeconds 1200
```

Matrix report:

```text
C:\Users\77585\AppData\Local\Temp\whale-paired-matrix-runs\20260601-012138-943\e2-matrix-report.md
```

Observed top-level result:

```text
e2_evidence_readiness: True
e2_clean_readiness: False
scenario_count: 3
levels: L1, L2, L3
required_levels: L1, L2, L3
repeats_per_scenario: 3
evidence_blocking_gaps: none
utility_warning_gaps: single-file-fast-fix: warning_pairs_2
```

Per-scenario aggregate reports:

```text
C:\Users\77585\AppData\Local\Temp\whale-paired-matrix-runs\20260601-012138-943\single-file-fast-fix\20260601-012139-449\aggregate-report.md
C:\Users\77585\AppData\Local\Temp\whale-paired-matrix-runs\20260601-012138-943\multi-file-order-pipeline\20260601-012604-890\aggregate-report.md
C:\Users\77585\AppData\Local\Temp\whale-paired-matrix-runs\20260601-012138-943\subscription-billing-repair\20260601-013800-598\aggregate-report.md
```

Each aggregate reported:

```text
all_pairs: 3
valid_utility_pairs: 3
excluded_pairs: 0
reported_evidence_level: E2
evidence_gate_failures: none
```

## Utility Caveat

The E2 result proves paired comparability and constructed-scenario readiness. It does not automatically prove net product benefit.

Observed utility outcomes:

| scenario | utility outcomes | warning pairs |
|---|---|---:|
| `single-file-fast-fix` | `both_success_cost_within_budget=1`; `both_success_taskspace_cost_higher=2` | 2 |
| `multi-file-order-pipeline` | `both_success_cost_within_budget=3` | 0 |
| `subscription-billing-repair` | `both_success_cost_within_budget=3` | 0 |

This is expected for a first E2 matrix. L1 especially can show TaskSpace overhead because the task is simple. The correct interpretation is:

- L1: TaskSpace should not fail or become chaotic; current overhead warnings are product tuning input and prevent a clean utility claim.
- L2/L3: TaskSpace should show healthier structure and no missed oracle behavior; cost still needs continuous tracking.

## Next Evidence Step

To move beyond E2:

1. Add more scenario variants per level.
2. Add historical real failure samples.
3. Add external benchmark-style tasks.
4. Require manual review closure for ambiguous real-world outcomes.
5. Keep utility outcomes separate from evidence-level gates.

---
name: observe-taskspace-performance
description: Generate evidence-based TaskSpace benchmark performance reports from local run artifacts. Use when comparing Standard, TaskSpace, R4, or R5 runs; reporting results, actions, request amplification, time or token cost, DeepSeek cache behavior, map structure, result lifecycle, or semantic preservation; or auditing a benchmark run for skipped, incomplete, or incomparable sides.
---

# Observe TaskSpace Performance

Use the repository report generator instead of manually adding artifact values.

## Workflow

1. Locate a sample run root containing `pair-*` directories and side `artifacts/metrics.json` files.
2. Run:

```bash
pwsh -NoProfile -File scripts/taskspace-benchmark/write-performance-observation.ps1 \
  -RunRoot <run-root>
```

3. Read `<run-root>/performance-observation.md` for the comparison and `.json` for exact values.
4. Read `performance-observation-events.jsonl` before drawing conclusions from warnings or missing evidence.
5. State which rows are `complete`, `incomplete`, or `skipped`. Never treat placeholder zeroes as measurements.

## Reporting Rules

- Lead with result eligibility, then actions, cost/cache, and map.
- Use `logical-mode-map.json`; never infer treatment from left/right.
- Use request-2+ cache hit for warm-cache comparison and show strict-prefix evidence beside it.
- Keep cached and uncached input separate. Do not infer monetary cost without a frozen unit-price artifact.
- Include map nodes, edges, open leaves, root status, control actions, result validity, retention, and semantic replacement.
- Report TaskSpace control failures and cadence rejections separately from ordinary-tool failures.
- Treat map warnings as mechanical observations. Do not recommend Runtime semantic intervention solely because Agent planning is coarse.
- Mark historical R4 fields unavailable when final-wire or map artifacts do not exist; do not fabricate parity.

## Multiple Runs

Generate one report per sample run. For cross-version comparison, report each run's exact evidence and label non-contemporaneous baselines as historical; compare ratios only when scenarios, prompts, model/provider settings, validators, and artifact coverage are equivalent.

## Validation

After changing the report tool, run:

```bash
pwsh -NoProfile -File scripts/taskspace-benchmark/test-performance-observation.ps1
python /home/zhangxu/.codex/skills/.system/skill-creator/scripts/quick_validate.py \
  .agents/skills/observe-taskspace-performance
```

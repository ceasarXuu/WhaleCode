---
name: observe-taskspace-performance
description: Generate evidence-based TaskSpace benchmark performance reports from local run artifacts. Use when comparing Standard, TaskSpace, R4, R5, R6, or R7 runs; reporting results, actions, request amplification, time or token cost, DeepSeek cache behavior, map structure, result lifecycle, Tool binding sequences, or semantic preservation; or auditing a benchmark run for skipped, incomplete, or incomparable sides.
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
- Report zero-cache requests as `warmup candidate` only when the cache shape has not appeared earlier in the run. Report a zero hit on an already-seen shape separately as `same-shape zero`; do not merge both into a generic cache failure.
- Treat `tool_choice` as part of the provider cache shape. Show message-prefix preservation separately when named-to-auto changes leave messages intact but break the full request prefix.
- Keep cached and uncached input separate. Do not infer monetary cost without a frozen unit-price artifact.
- Include map nodes, edges, open leaves, root status, control actions, result validity, retention, and semantic replacement.
- Report TaskSpace failures as separate protocol, state-machine, and ordinary Tool counts. A Tool skipped after an earlier control failure is not an independently executed ordinary failure.
- For the current R7 contract, distinguish `taskspace_control`, the first ordinary Tool carrying an `initialize_map` object in `taskspace_binding`, ordinary Tools with `taskspace_binding.action=active`, ordinary Tools with `taskspace_binding.action=after_boundary`, and sequence-preflight rejected calls. Historical scalar bindings and full `taskspace_action` carrier fields remain historical evidence only.
- Report the `patch` section: total/max patch declarations per provider response, request-wide multi-patch attempts and preflight rejects, multi-file patches, prepare/commit/partial failures, and post-patch action/skipped counts. Treat missing rollout evidence as unavailable, not zero.
- Read observations cover only explicit `read_file` and `read_output_ref` identities. Do not infer reads or writes from shell command text, and never use these observations as a Runtime gate.
- Report `initialize_map`, `mutate_graph`, `bind_node`, `complete_then_continue`, direct block/unblock/rework actions, and `finish_map` counts. For the current branch-free terminal contract, split committed `finish_map` results by canonical `terminal_node_role` (`work` versus `finish`); for historical artifacts only, preserve the submitted `terminal_state` split.
- For current R7 runs, count one valid initialization carrier as an initialization pair and report it separately from bind pairs and complete-then-continue pairs. Also report forbidden direct `taskspace_control initialize_map`, standalone later boundaries, orphan `after_boundary` calls, active bindings, carrier commit/failure, and preflight-rejected call totals. Preserve provider order when deciding whether later calls are adjacent.
- For `TaskSpaceControlResultR6V1`, report committed delta present/missing counts, canonical graph revision batch counts, node-detail event counts, and any failed terminal result whose `state_commit` is true. A missing success delta or nonzero failure commit is a feedback-contract violation, not an Agent mistake. Current full Map state belongs only to the active projection and must not appear in control feedback.
- Treat a nonterminal boundary without its required adjacent ordinary Tool as a protocol failure with zero execution, not a state-machine failure. Runtime must not infer or insert the missing action. Report the actual and expected mechanical sequence when the artifact contains it.
- Report the provider-visible Tool section count, bytes, estimated tokens, and hash. Compare the current lightweight binding cost with both Standard and any historical full-carrier baseline only when the model, tool registry, and wire scanner are equivalent.
- Treat map warnings as mechanical observations. Do not recommend Runtime semantic intervention solely because Agent planning is coarse.
- Mark historical R4 fields unavailable when final-wire or map artifacts do not exist; do not fabricate parity.

## Multiple Runs

Generate one report per sample run. For cross-version comparison, report each run's exact evidence and label non-contemporaneous baselines as historical; compare ratios only when scenarios, prompts, model/provider settings, validators, and artifact coverage are equivalent.

## Map Budget Baselines

For R5-K map-scale, projection-budget, checkpoint, or replay analysis, run:

```bash
pwsh -NoProfile -File scripts/taskspace-benchmark/run-map-budget-k0.ps1
```

After a Docker pair exists, bind its TaskSpace rollout into the same baseline with
`-CapturedRolloutPath <right/artifacts/rollout.jsonl>`. This replays recorded lifecycle events; it
does not replace the synthetic 100/1k/10k scale probe or the session-native compaction fixture.

Read `k0-map-budget-report.json`, `k0-map-budget-report.md`, and
`k0-map-budget-events.jsonl` from the emitted run directory. Report node and edge profiles separately;
keep skeleton and node-detail bytes separate; state the first measured node count over every active
projection budget. Do not infer a compression algorithm from K0 size curves. Replay is valid only when
`replay_exact=true`, and corruption evidence must distinguish the current panic behavior from the selected
structured session-fatal contract. Report the session-native fixture independently from the synthetic delta
microbench: verify its resume, compaction, code-revision, exact-replay, and projection-outcome counts, and do
not combine its lifecycle cost with provider request cost.

## Validation

After changing the report tool, run:

```bash
pwsh -NoProfile -File scripts/taskspace-benchmark/test-performance-observation.ps1
pwsh -NoProfile -File scripts/taskspace-benchmark/test-map-budget-k0.ps1
python /home/zhangxu/.codex/skills/.system/skill-creator/scripts/quick_validate.py \
  .agents/skills/observe-taskspace-performance
```

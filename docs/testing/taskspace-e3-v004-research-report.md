# TaskSpace v0.0.4 E3 Run Research Report

Date: 2026-06-16

Run root:
`D:\whalecode-alpha\target\e3-v004-proof-20260615\serial-clean-v1\suite-20260616-020714`

## 1. Scope And Evidence

This report analyzes the clean v0.0.4 E3 serial run over Terminal-Bench calibration tasks:

- Tasks: `analyze-access-logs`, `log-summary`, `count-call-stack`
- Repeats: 5 per task
- Compared modes: `standard` vs `taskspace`
- Pair count: 15
- Engineering cleanliness: clean for this run
  - `invalid_harness_sample_count=0`
  - `signature_count=0`
  - `engineering_unclean_pairs=0`
  - `suite_score_valid=true`

Important caveat:

- The run still has E3 audit templates pending (`audit_required` at sample level). The raw solved/wrong values below are public-validator outcomes from `metrics.json`/`audit.json`, not a human-reviewed final benchmark claim.

## 2. Executive Summary

TaskSpace produced a small raw accuracy lift but at very high runtime and token cost.

| Metric | Standard | TaskSpace | TaskSpace / Standard |
|---|---:|---:|---:|
| Solved pairs | 7/15 | 8/15 | +1 solved pair |
| Agent time | 791.9s | 3952.2s | 4.99x |
| Public validation time | 1623.8s | 1437.8s | 0.89x |
| Docker run time | 1441.1s | 1286.4s | 0.89x |
| Direct input+output tokens | 2,564,355 | 51,073,287 | 19.92x |
| Uncached input tokens | 185,043 | 660,749 | 3.57x |
| Output tokens | 39,728 | 293,242 | 7.38x |
| Tool calls | 103 | 124 | 1.20x |

Main readout:

- TaskSpace is not broadly better yet. It is selectively better on some log-processing tasks, neutral on `log-summary`, and ineffective on `count-call-stack`.
- The cost profile is dominated by massive cached input reuse, not just visible subagent JSONL overhead.
- The runtime bottleneck is split between TaskSpace agent time and public validation. The suite-level timing classifies the run as `validator_bound`, but mode-level comparison shows TaskSpace agent execution is the largest differential.
- TaskSpace graph quality signals are weak: every TaskSpace run had `high_unreviewed_result_ratio`, and 13/15 had `high_blocked_node_ratio`.

## 3. Outcome Detail

| Task | Standard | TaskSpace | Interpretation |
|---|---:|---:|---|
| analyze-access-logs | 4/5 | 5/5 | TaskSpace gained one clear pair. |
| count-call-stack | 0/5 | 0/5 | Both modes failed consistently. TaskSpace spent more without solving. |
| log-summary | 3/5 | 3/5 | Same aggregate score, but different pair-level winners. |
| Total | 7/15 | 8/15 | Net +1 solved pair. |

Pair-level result categories:

| Category | Count |
|---|---:|
| Both solved | 5 |
| TaskSpace only | 3 |
| Standard only | 2 |
| Neither solved | 5 |

Pair-level details:

| Task | Pair | Result | Standard time | TaskSpace time | Standard tokens | TaskSpace tokens | TS token ratio |
|---|---:|---|---:|---:|---:|---:|---:|
| analyze-access-logs | 001 | TaskSpace only | 22.4s | 463.8s | 57,921 | 5,439,623 | 93.91x |
| analyze-access-logs | 002 | Both | 33.2s | 131.1s | 74,865 | 831,299 | 11.10x |
| analyze-access-logs | 003 | Both | 35.3s | 160.6s | 104,901 | 1,280,017 | 12.20x |
| analyze-access-logs | 004 | Both | 33.9s | 192.2s | 422,971 | 4,389,679 | 10.38x |
| analyze-access-logs | 005 | Both | 57.4s | 374.7s | 141,046 | 12,993,596 | 92.12x |
| count-call-stack | 001 | Neither | 57.2s | 151.6s | 144,994 | 1,047,578 | 7.22x |
| count-call-stack | 002 | Neither | 94.4s | 287.8s | 242,183 | 2,804,769 | 11.58x |
| count-call-stack | 003 | Neither | 72.4s | 251.9s | 230,269 | 2,240,984 | 9.73x |
| count-call-stack | 004 | Neither | 101.9s | 237.6s | 314,889 | 2,511,386 | 7.98x |
| count-call-stack | 005 | Neither | 134.7s | 254.8s | 431,759 | 2,552,022 | 5.91x |
| log-summary | 001 | TaskSpace only | 33.3s | 377.8s | 86,133 | 4,152,217 | 48.21x |
| log-summary | 002 | Both | 28.4s | 328.5s | 86,074 | 3,974,568 | 46.18x |
| log-summary | 003 | Standard only | 32.6s | 238.3s | 84,563 | 2,163,497 | 25.58x |
| log-summary | 004 | TaskSpace only | 26.6s | 263.8s | 69,243 | 2,652,195 | 38.30x |
| log-summary | 005 | Standard only | 28.2s | 237.7s | 72,544 | 2,039,857 | 28.12x |

## 4. Runtime Findings

Suite timing:

- Total pair duration: 8276212ms, about 137.9 minutes
- Agent execution: 4744306ms, about 79.1 minutes
- Public validation: 3061684ms, about 51.0 minutes
- Docker run: 2727821ms, about 45.5 minutes
- Suite bottleneck classification: `validator_bound`

Mode-level timing tells a sharper story:

- Standard agent time: 13.2 minutes total
- TaskSpace agent time: 65.9 minutes total
- TaskSpace added about 52.7 minutes of agent runtime while gaining one net solved pair.
- Public validation and Docker are large absolute costs, but they do not explain the TaskSpace vs Standard gap. Validation time is actually slightly lower for TaskSpace in this run.

Task-level average agent time:

| Task | Standard avg | TaskSpace avg | Ratio |
|---|---:|---:|---:|
| analyze-access-logs | 36.4s | 264.5s | 7.27x |
| count-call-stack | 92.1s | 236.7s | 2.57x |
| log-summary | 29.8s | 289.2s | 9.70x |

Interpretation:

- TaskSpace has a high fixed orchestration/context overhead. It is especially expensive on tasks that Standard can solve in under 60 seconds.
- `count-call-stack` shows that longer runtime does not guarantee better reasoning. TaskSpace spent 2.6x Standard time on average and still failed all five.

## 5. Token Cost Findings

Direct mode usage, excluding nested subagent JSONL files:

| Mode | Input | Cached input | Uncached input | Output | Reasoning output | Input+Output |
|---|---:|---:|---:|---:|---:|---:|
| Standard | 2,524,627 | 2,339,584 | 185,043 | 39,728 | 16,748 | 2,564,355 |
| TaskSpace | 50,780,045 | 50,119,296 | 660,749 | 293,242 | 82,420 | 51,073,287 |

All JSONL usage, including nested TaskSpace subagent JSONL files:

| Scope | Files | Input | Cached input | Uncached input | Output | Reasoning output | Input+Output |
|---|---:|---:|---:|---:|---:|---:|---:|
| Direct only | 30 | 53,304,672 | 52,458,880 | 845,792 | 332,970 | 99,168 | 53,637,642 |
| All JSONL | 45 | 53,982,666 | 52,920,192 | 1,062,474 | 341,964 | 100,764 | 54,324,630 |
| Nested extra | 15 | 677,994 | 461,312 | 216,682 | 8,994 | 1,596 | 686,988 |

Key observations:

- TaskSpace direct input is 20.1x Standard input.
- TaskSpace direct input+output is 19.9x Standard.
- TaskSpace uncached input is 3.6x Standard, much lower than the total-token ratio because the run benefits heavily from prompt caching.
- TaskSpace cached input rate is about 98.7%; Standard cached input rate is about 92.7%.
- The separate nested subagent JSONL files add only about 0.69M tokens. This means most TaskSpace token cost is not in standalone subagent files; it is in the main TaskSpace execution context, likely because graph state, subagent summaries, results, and accumulated context are repeatedly reintroduced into model calls.

Largest token outliers:

| Task | Pair | Mode | Outcome | Tokens | Agent time |
|---|---:|---|---|---:|---:|
| analyze-access-logs | 005 | TaskSpace | solved | 12,993,596 | 374.7s |
| analyze-access-logs | 001 | TaskSpace | solved | 5,439,623 | 463.8s |
| analyze-access-logs | 004 | TaskSpace | solved | 4,389,679 | 192.2s |
| log-summary | 001 | TaskSpace | solved | 4,152,217 | 377.8s |
| log-summary | 002 | TaskSpace | solved | 3,974,568 | 328.5s |

Outcome/cost relationship:

| Mode | Outcome | Runs | Avg agent time | Avg tokens | Avg tool calls |
|---|---|---:|---:|---:|---:|
| Standard | solved | 7 | 35.6s | 140,995 | 5.1 |
| Standard | wrong | 8 | 67.9s | 197,174 | 8.4 |
| TaskSpace | solved | 8 | 286.6s | 4,464,149 | 6.4 |
| TaskSpace | wrong | 7 | 237.1s | 2,194,299 | 10.4 |

This suggests TaskSpace success currently correlates with spending more context and time, not with a leaner or more efficient reasoning path.

## 6. TaskSpace Behavior Pattern

TaskSpace graph warnings:

| Warning | Count |
|---|---:|
| high_unreviewed_result_ratio | 15/15 |
| high_blocked_node_ratio | 13/15 |
| subagent_no_decision_yield | 7/15 |
| low_decision_density | 2/15 |
| synthesis_not_ready | 2/15 |

Subagent usage:

| Task | Spawn agent calls | Subagent results | TaskSpace solved |
|---|---:|---:|---:|
| analyze-access-logs | 4 | 12 | 5/5 |
| count-call-stack | 0 | 0 | 0/5 |
| log-summary | 11 | 42 | 3/5 |

Behavioral interpretation:

- TaskSpace uses substantial structure and context even when subagents are not spawned.
- On `count-call-stack`, TaskSpace did not spawn subagents at all, despite the task being consistently unsolved. It mainly behaved as a heavier single-agent workflow with more tool calls.
- On `log-summary`, TaskSpace spawned many subagents but only tied Standard. The graph repeatedly warned `subagent_no_decision_yield`, indicating subagent outputs were not consistently converted into adopted decisions.
- On `analyze-access-logs`, TaskSpace got a real benefit. It solved all five and beat Standard on one pair. However, the cost was extreme: 24.9M direct tokens for five TaskSpace runs versus 0.8M for Standard.

The strongest current TaskSpace pattern is "expensive breadth plus context accumulation." It can help on tasks where redundant inspection and synthesis improve reliability, but it does not yet reliably convert that breadth into decisions, nor does it detect when breadth is not helping.

## 7. Failure Pattern: count-call-stack

Both modes failed all five `count-call-stack` repeats. This is not an engineering-cleanliness issue:

- Public validators completed.
- Failures were normal validator assertion failures, primarily `FAILED ../tests/test_outputs.py::test_count_output`.
- `engineering_unclean=false` for all rows.

Observed behavior:

- Both modes wrote `output.txt` and often extracted or parsed `log.stack`.
- Standard changed parsing helper files in some repeats.
- TaskSpace created helper scripts in some repeats, but no subagents were spawned.
- TaskSpace used more tool calls than Standard on this task: 63 vs 55.
- TaskSpace used about 11.16M tokens versus Standard's 1.36M, with 0/5 solved for both.

Likely cause:

- This task appears format-sensitive and validator-specific. The agents generated plausible stack-analysis outputs but missed exact expected structure/content.
- TaskSpace did not introduce a distinct verification or format-diff strategy. It mostly amplified the same local exploration loop.
- Since no subagents were spawned, the multi-agent primitive did not engage on the task that most needed alternative hypotheses.

## 8. What This Run Says About v0.0.4

Positive signal:

- Engineering harness is clean for this run.
- TaskSpace can improve reliability on some log-processing tasks.
- Prompt caching makes the huge context cost less catastrophic than raw token totals suggest.

Negative signal:

- Net effectiveness lift is only +1/15.
- TaskSpace runtime is about 5x Standard agent runtime.
- TaskSpace direct token cost is about 20x Standard.
- Graph hygiene is poor: all TaskSpace runs have high unreviewed result ratio.
- Subagent output adoption is weak. In several runs, subagents exist but do not translate into decision yield.
- TaskSpace does not yet have a good "stop spending" mechanism when it is not improving the solution path.

The cleanest characterization is:

> v0.0.4 TaskSpace shows a correctness signal, but it is not cost-efficient. It behaves like a high-context orchestration system with occasional reliability gains rather than a consistently better coding agent mode.

## 9. Bottleneck Hypotheses

H1: Context bloat is the primary token bottleneck.

- Evidence: TaskSpace direct input+output is 51.1M tokens, 19.9x Standard.
- Evidence: Cached input dominates TaskSpace usage at about 98.7%.
- Inference: repeated graph/context/subagent state is being included in model calls even when the actual task is small.

H2: Decision adoption is the primary TaskSpace quality bottleneck.

- Evidence: `high_unreviewed_result_ratio` appears in 15/15 TaskSpace runs.
- Evidence: `subagent_no_decision_yield` appears in 7/15 runs.
- Inference: TaskSpace is generating intermediate results faster than it can validate/adopt them.

H3: Task routing is too coarse.

- Evidence: `count-call-stack` got no subagents and failed 0/5.
- Evidence: `log-summary` got many subagents but only tied Standard.
- Inference: TaskSpace is not yet deciding when to use subagents based on task shape and observed failure mode.

H4: Validation remains a suite-level speed bottleneck.

- Evidence: public validation consumed about 51 minutes of the run; Docker run consumed about 45.5 minutes.
- Evidence: validation time is large for both modes, so it limits total E3 iteration speed even after agent improvements.

## 10. Recommendations

1. Add first-class token summaries to suite artifacts.

- Persist `token-summary.json` at pair, sample, and suite levels.
- Split direct agent usage from nested subagent usage.
- Track input, cached input, uncached input, output, reasoning output, and cost estimate.
- Add top-token outliers to `suite-health.json` or a companion report.

2. Add TaskSpace budget guardrails.

- Abort or degrade TaskSpace when token ratio exceeds a threshold without new accepted decisions.
- Suggested initial thresholds:
  - `taskspace_total_tokens > 10x standard_tokens` and no new accepted decision in N steps.
  - `high_unreviewed_result_ratio` plus `subagent_no_decision_yield` after first synthesis checkpoint.
  - `synthesis_not_ready` near final answer should mark the run as diagnostically suspicious even if validator later passes.

3. Improve decision adoption.

- Require each subagent result to be accepted, rejected, or explicitly deferred.
- Track "adopted evidence per 1M tokens" and "accepted result per subagent result".
- Penalize graph growth that does not produce decisions.

4. Add task-shape routing.

- For small deterministic file tasks, start with a lightweight Standard-like path and escalate only on validator failure or ambiguity.
- For parser/format-sensitive tasks like `count-call-stack`, route to a verification-first workflow:
  - read expected output format,
  - generate small parser,
  - self-run output checks before final,
  - compare validator failure text against produced output.

5. Optimize validation runtime.

- Cache or prebuild validator Python/uv environments where possible.
- Continue Docker image cache work, but note that Docker build was only about 1 minute 24 seconds total; Docker run and test environment setup dominate.
- Keep validation timeout high enough to avoid invalid runs, but build preflight timing probes to identify tasks whose validator setup is intrinsically slow.

6. Add a TaskSpace value gate to E3 reporting.

- Report solved delta together with cost delta:
  - `extra_solved_pairs`
  - `extra_agent_minutes`
  - `extra_uncached_tokens`
  - `extra_total_tokens`
  - `extra_cost_per_additional_solved_pair`
- For this run, TaskSpace gained one extra solved pair at about +52.7 agent minutes and +48.5M direct input+output tokens.

## 11. Bottom Line

This v0.0.4 E3 run is useful because it is finally engineering-clean, but the product signal is mixed:

- Correctness: slight positive, +1/15.
- Runtime: negative, about 5x agent time.
- Token cost: strongly negative, about 20x direct tokens.
- Behavior quality: mixed to negative, with persistent unreviewed/blocked graph warnings.

The next phase should not only try to raise solved count. It should make TaskSpace prove value under cost controls: fewer unreviewed results, better decision adoption, adaptive escalation, and explicit token/runtime budgets.

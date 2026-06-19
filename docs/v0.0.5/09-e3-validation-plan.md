# 09. E3 Validation Plan

> 历史文档警告（2026-06-19）：本文已经被
> `18-unfinished-work-engineering-design.md` 和 `docs/experiments/`
> 的实验制度取代。本文中的旧 PASS/PARTIAL 口径、v004-clean 三样本安排、
> “product partial acceptable” 表述均不得作为 v0.0.5 当前收口、
> release decision 或正式 E3 依据。当前唯一 formal P0 release proof 是
> `terminal-bench_E3-P0_3_5`；`_1_1`、`_3_1`、`_3_2` 只能是 diagnostic-only。

## 1. 验证目标

v0.0.5 E3 不以“扩大样本”为目标。它验证两件事：

```text
1. TaskSpace 收敛成本是否进入 2x 阶段目标；
2. map 自我管理能力是否开始具备上下文替代前置条件。
```

## 2. 样本范围

继续使用 v0.0.4 的三个任务，保持可比性：

```text
analyze-access-logs
log-summary
count-call-stack
```

原因：

| 样本 | 用途 |
|---|---|
| analyze-access-logs | 验证保留 TaskSpace 可靠性收益，同时压低 outlier |
| log-summary | 验证 subagent/adoption/decision yield |
| count-call-stack | 验证 thin + verification-first 是否改变失败路径 |

## 3. 运行矩阵

最小矩阵：

```text
standard: 5 pairs/sample
v004-legacy-taskspace: optional replay or historical baseline
v005-compact-taskspace: 5 pairs/sample
```

如果运行成本有限，先跑：

```text
analyze-access-logs: pair 001, 005
log-summary: pair 001, 003, 004, 005
count-call-stack: all 5 pairs
```

## 4. 发布门槛

### Hard gates

```text
engineering_clean = true
suite_score_valid = true
large_output_replay_count = 0
state_commit enabled in 100% TaskSpace runs
context projection generated in 100% TaskSpace runs
token-summary.json present in 100% sides
```

### Cost gates

```text
TaskSpace / Standard direct input+output <= 2x target
TaskSpace / Standard agent walltime <= 2x target
model_request_count_ratio <= 2.5x
avg_input_per_request_ratio <= 1.25x
```

允许阶段性判定：

```text
historical strong-cost result: all cost gates pass
historical engineering-only result: main ratio <=3x and root cause outlier isolated
historical failed-cost result: main ratio remains >5x or model_request_ratio remains >5x
```

### Quality gates

```text
TaskSpace solved >= Standard solved - 1
TaskSpace does not regress analyze-access-logs below Standard
count-call-stack must show verification-first workflow evidence
log-summary subagent-heavy runs must show decision yield or stopped spawn
```

### Map management gates

```text
100% map items have retention class
projection size p95 within profile budget
semantic replacement rate measured
unreviewed active result count reduced >=60%
stale blocked nodes not active in final projection
```

## 5. Expected outcomes

### Strong success

```text
TaskSpace solved >= Standard solved
TaskSpace cost <=2x
high_unreviewed_result_ratio falls substantially
count-call-stack shows improved path or at least lower-cost failure
```

### Historical Engineering-Only Outcome

```text
cost <=2x
map management works
solved slightly regresses or remains tied
```

This historical outcome is not acceptable for current v0.0.5 release closure.
Any engineering-only or partial-cost result must be reported as
`blocked_partial` with `closeable=false`, and cannot replace
`terminal-bench_E3-P0_3_5` release proof.

### Failure

```text
model_request_count still >5x
full raw output still enters prompt
state_commit not adopted
projection grows linearly
map remains structured log rather than managed memory
```

## 6. Required artifact checklist

Each TaskSpace side must include:

```text
token-summary.json
context-projection-summary.json
state-management-summary.json
state-commit-events.jsonl
projection-events.jsonl
gc-events.jsonl
output-ref-events.jsonl
routing-decision.json
map-final.json
graph-health.json
```

Each pair must include:

```text
pair-cost-report.md
pair-routing-report.md
pair-value-report.md
standard/taskspace metrics.json
standard/taskspace validator stdout/stderr
standard/taskspace diffs
```

Suite must include:

```text
suite-cost-gate.json
suite-value-gate.md
suite-routing-summary.json
suite-map-management-summary.json
```

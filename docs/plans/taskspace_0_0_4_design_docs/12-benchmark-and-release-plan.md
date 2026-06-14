# 12. Benchmark 与 Release Plan

## 1. Benchmark 分层

0.0.4 不建议扩大 benchmark；应先清洗和分层现有 E3 样本。

| 层 | 用途 | 样本 |
|---|---|---|
| Low-friction regression | 确认 TaskSpace 不拖垮简单/中等直线任务 | recover-accuracy-log |
| Medium utility evidence | 主 utility 观察样本 | processing-pipeline 中 validator 稳定 pair |
| Stress/noisy validator | 压力测试，不作为主 utility 结论 | multi-source-data-merger |
| Fail-closed mechanism | 环境资产不可控验证 | query-optimize |

## 2. 0.0.4 最小复跑矩阵

```text
recover-accuracy-log: 5 pairs
processing-pipeline: 5 pairs
multi-source-data-merger: 2 diagnostic pairs
query-optimize: preflight only
```

## 3. 运行输出要求

每个 pair 必须输出：

```text
audit.yaml
graph-health.json
standard.diff.patch
taskspace.diff.patch
standard.validator stdout/stderr
taskspace.validator stdout/stderr
failure taxonomy
result adoption summary
```

## 4. Release gates

### Gate A：基础安全

```text
Docker cleanup 无残留；
remote asset fail-closed 生效；
validator artifact 可读取。
```

### Gate B：schema/gate 基础

```text
TaskSpace run 有 success criteria；
final synthesis gate 生效；
invalid result gate 生效。
```

### Gate C：audit 基础

```text
valid_utility_pairs > 0，或每个 pair 的 exclusion 原因机械可解释；
failure taxonomy 非 unknown。
```

### Gate D：行为质量

```text
recover-accuracy-log 不出现无解释 deep graph；
processing-pipeline 输出 subagent ROI；
graph health 能捕捉 node_overfragmentation 和 result_not_synthesized。
```

## 5. 成功判断

0.0.4 成功条件：

```text
不是 TaskSpace better > Standard better，
而是 TaskSpace 进入 clean audit + graph health + problem-state 可解释阶段。
```

## 6. 回归风险

| 风险 | 缓解 |
|---|---|
| Gate 过硬导致 agent 卡死 | 先 hard 少量关键 gate，其余 warning |
| Schema 过复杂导致模型不使用 | prompt 中给最小行动模板，viewer 提供状态缺口 |
| Thin mode 分类误导 | 0.0.4 只 report-only，不自动切换 |
| Audit manifest 过重 | 先用已有 artifact 聚合，避免引入人工作业负担 |
| 0.0.3 trace 不兼容 | versioned schema，legacy viewer mode |

## 7. 发布标准

```text
P0 issue 全部完成；
E3 focused rerun 完成；
release note 明确：0.0.4 是 observability/contract 版本，不宣称 utility win；
version registry 记录 clean audit 状态。
```
## 8. E3 runtime calibration and speed plan

This section is a hard execution plan for the observed problem: 15 E3 tasks can
take hours, and that cost is unacceptable unless the run is mechanically clean
and produces timing evidence. It is not enough to say "parallelize later"; the
harness must first identify where time is spent and must block unsupported speed
claims.

### Phase 8.1: required timing evidence before another full E3

Implementation:

```text
scripts/taskspace-benchmark/lib/timing.ps1
scripts/taskspace-benchmark/lib/runtime-bottleneck-report.ps1
scripts/taskspace-benchmark/lib/calibration-gate.ps1
```

Required artifacts:

```text
one-pair smoke:
  pair-timing.json
  sample-timing.json
  runtime-bottleneck.md

3-sample serial calibration:
  suite-timing.json
  runtime-calibration-report.md

parallel smoke:
  serial-vs-parallel-equivalence.json
```

Acceptance:

```text
calibration-gate.json status=pass
full_e3_allowed=true
speed_claim_allowed=true
no missing required timing fields
no parallel_smoke_score_drift
```

If this gate fails, a full E3 run is blocked. The run may only be used as harness
debugging evidence, not as TaskSpace score evidence.

### Phase 8.2: bottleneck classification

Each calibration report must classify the dominant time sink:

```text
agent_bound
validator_bound
docker_build_bound
docker_run_bound
cleanup_bound
engineering_unclean_slow
mixed_or_unclassified
```

Developer steps:

```text
1. Read pair-timing.json top_spans and subtotal_percentages.
2. Compare agent duration, public validation duration, Docker build/run, cleanup,
   model request, and resource wait totals.
3. If any engineering-unclean signature appears, classify as engineering_unclean_slow
   and invalidate score-bearing conclusions.
4. Write runtime-bottleneck.md with the concrete top span and optimization status.
```

Acceptance:

```text
runtime-bottleneck.md contains speedup_decision
suite aggregate renders Timing Summary
engineering_unclean_slow blocks score comparison
```

### Phase 8.3: safe speedup rollout

Speedup must be staged. The only supported parallelism in 0.0.4 is sample-level
parallelism after serial/parallel equivalence is proven.

Developer steps:

```text
1. Keep pair-level and validation-level parallelism fail-closed.
2. Enable MaxParallelSamples only after disk reservation and resource governor pass.
3. Run one serial smoke and one sample-parallel smoke on the same task list.
4. Compare suite-health.json through serial-vs-parallel-equivalence.json.
5. If any score-bearing field drifts, disable parallel full E3.
```

Acceptance:

```text
parallelism.json sample_parallel_supported
serial-vs-parallel-equivalence.json comparable=true
parallel_smoke_score_drift=false
calibration-gate.json status=pass
```

### Phase 8.4: full E3 execution rule

Before launching the 15-task E3:

```text
1. Run start gate.
2. Run disk-space preflight.
3. Run one-pair timing smoke.
4. Run 3-sample serial calibration.
5. Run sample-parallel smoke and equivalence diff.
6. Run calibration gate.
7. Only then start full E3.
```

Full E3 output must include:

```text
suite-health.json
aggregate.json
aggregate-report.md
suite-timing.json
runtime-calibration-report.md
calibration-gate.json
parallelism.json
```

No performance conclusion is valid without those files.

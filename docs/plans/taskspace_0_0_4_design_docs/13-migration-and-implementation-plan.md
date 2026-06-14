# 13. Migration 与实施计划

## 1. Schema versioning

新增：

```text
taskspace_schema_version = taskspace-v2
problem_state_ledger_version = problem-state-ledger-v1
graph_health_version = graph-health-v1
audit_manifest_version = taskspace-e3-audit-v1
```

0.0.3 trace 不迁移为 v2，只在 viewer 中作为 legacy 展示。

## 2. 数据迁移策略

| 旧字段 | 新字段 | 策略 |
|---|---|---|
| task.objective | ledger.objective | 新 run 直接写入；旧 run legacy display |
| outputContracts | success_criteria | 新 run 建议替换；旧 run可显示为 legacy output contract |
| facts | known_facts | 新 run 保留并加强 evidence refs |
| result validity | result adoption | 新增 adoption refs；旧 result adoption unknown |
| node kind | canonical node kind | 运行时映射 |

## 3. 实施阶段

### Stage 1：Audit / GraphHealth 先落地

目标：不改变 agent 行为，先把证据链补齐。

交付：

```text
audit.yaml
graph-health.json
failure taxonomy classifier
aggregate update
```

### Stage 2：ProblemStateLedger 与 schema v2

目标：改变 agent 必填状态，但只加入少量 hard gate。

交付：

```text
ProblemStateLedger
record_success_criteria
record_open_question
record_decision
record_next_best_action
start_task schema update
```

### Stage 3：ResultAdoption 与 typed node contract

目标：让 result/decision/node 进入引用链。

交付：

```text
adopt_result
result dependency refs
invalid/questioned gates
canonical node kinds
kind-specific finish requirements
```

### Stage 4：Subagent / Thin mode / Viewer v2

目标：可观测协作收益和低摩擦模式。

交付：

```text
record_subagent_plan
subagent ROI metrics
thin/standard/deep classifier report-only
viewer v2
```

## 4. 开发顺序

```text
1. CleanE3AuditManifest
2. FailureTaxonomyV1
3. GraphHealthReportOnly
4. ProblemStateLedgerV1
5. taskspace_control schema v2
6. ResultAdoptionV1
7. TypedNodeKindContractV1
8. SubagentContractV1
9. ThinModeClassifierReportOnly
10. ViewerV2
```

## 5. 回滚策略

| 改动 | 回滚方式 |
|---|---|
| taskspace_control v2 | 支持 schema_version，回退 v1 action |
| hard gate | feature flag 关闭为 warning |
| graph health | 纯 report-only，可安全保留 |
| audit manifest | 不影响 agent execution，可保留 |
| viewer v2 | 保留 legacy viewer |

## 6. Feature flags

```text
taskspace.problem_ledger.enabled
taskspace.schema_v2.enabled
taskspace.gate.final_synthesis.enabled
taskspace.gate.invalid_result.enabled
taskspace.graph_health.enabled
taskspace.audit_manifest.enabled
taskspace.thin_mode.report_only
```

## 7. 最小可交付 0.0.4

如果时间受限，保留：

```text
CleanE3AuditManifest
FailureTaxonomyV1
GraphHealthReportOnly
ProblemStateLedgerV1 minimal
ResultAdoptionV1 minimal final gate
```

推迟：

```text
SubagentContractV1
ThinModeClassifierReportOnly
ViewerV2 full drill-down
graph prune/merge hard actions
```
## 8. E3 runtime calibration implementation stages

The E3 harness must not launch another full 15-task run until runtime cost and
speedup safety are mechanically gated.

### Stage 5: timing instrumentation closure

Deliverables:

```text
pair-timing.json
sample-timing.json
suite-timing.json
runtime-bottleneck.md
runtime-calibration-report.md
```

Engineering details:

```text
1. Pair timing records agent, validation, Docker build/run, cleanup, model request,
   cache wait, and resource wait totals.
2. Sample timing aggregates pair timing and preserves bottleneck counts.
3. Suite timing aggregates sample timing and exposes sample_count.
4. Runtime reports render both markdown and JSON so CI can gate on fields.
```

Acceptance:

```text
test-e3-harness-guardrails.ps1 passes timing assertions
aggregate-report.md includes Timing Summary
speedup_decision is present in report JSON
```

### Stage 6: calibration gate before full E3

Deliverables:

```text
scripts/taskspace-benchmark/lib/calibration-gate.ps1
calibration-gate.json
```

Engineering details:

```text
1. Validate one-pair smoke artifacts before larger runs.
2. Validate 3-sample serial calibration artifacts before speed claims.
3. Validate serial-vs-parallel equivalence before sample-level parallel full E3.
4. Fail closed on missing timing fields, missing reports, low sample count, or
   parallel score drift.
```

Acceptance:

```text
full_e3_allowed=false when one-pair smoke evidence is missing
speed_claim_allowed=false when serial calibration evidence is missing
full_e3_allowed=false when parallel_smoke_score_drift=true
full_e3_allowed=true only when all calibration gates pass
```

### Stage 7: speedup rollout

Deliverables:

```text
MaxParallelSamples sample-level scheduler
parallelism.json
serial-vs-parallel-equivalence.json
```

Engineering details:

```text
1. Sample-level parallelism may run independent samples concurrently.
2. Pair-level, validation-level, Docker, and model concurrency remain fail-closed
   until separately proven.
3. Parallel smoke must compare score-bearing suite-health fields against a serial
   baseline before full E3.
4. Disk reservation and resource governor checks run before scheduling.
```

Acceptance:

```text
MaxParallelSamples=2 selftest completes
merged sample order is deterministic
serial-vs-parallel-equivalence.json comparable=true
parallel_smoke_score_drift=false
unsupported parallel fields still fail closed
```
